// flood.mjs — chargeur HTTP phase 6b (gitcard-api Rust) : >= 32 requêtes
// concurrentes en boucle pendant DURATION_S (défaut 180 s), mélange FIXE et
// documenté : /health 25 %, /api/themes 25 %, /stats/{u} + /api/data/{u}
// 50 % (8 pseudos tournants). Aucune sortie Internet voulue : le réseau est
// --internal, les appels GitHub échouent (5xx) — ce chemin erreur FAIT
// PARTIE de la charge (cache/erreurs/rate-limiter sollicités).
// Exécuté en conteneur `node:26.13.1-alpine` plafonné sur le même réseau
// --internal que le service testé. Sortie finale : UNE ligne JSON
// (codes HTTP, p50/p99, erreurs réseau) + 1 ligne de progression / 15 s.
const TARGET = process.env.TARGET || 'http://gitcard-p6:3000';
const DURATION_S = Number(process.env.DURATION_S || 180);
const CONCURRENCY = Number(process.env.CONCURRENCY || 32);
const USERS = [
  'probe-ada', 'probe-bob', 'probe-celine', 'probe-dmitri',
  'probe-eva', 'probe-farid', 'probe-gina', 'probe-hugo',
];

const codes = new Map();
let total = 0;
let errors = 0;
const latencies = [];

function pickPath() {
  const r = Math.random();
  if (r < 0.25) return '/health';
  if (r < 0.5) return '/api/themes';
  const u = USERS[(Math.random() * USERS.length) | 0];
  return r < 0.75 ? `/stats/${u}` : `/api/data/${u}`;
}

const deadline = Date.now() + DURATION_S * 1000;
let stopped = false;

async function worker() {
  while (!stopped) {
    const t0 = performance.now();
    try {
      const res = await fetch(TARGET + pickPath(), { signal: AbortSignal.timeout(30000) });
      total += 1;
      codes.set(res.status, (codes.get(res.status) || 0) + 1);
      latencies.push(performance.now() - t0);
    } catch {
      total += 1;
      errors += 1;
      codes.set('err', (codes.get('err') || 0) + 1);
    }
  }
}

const progress = setInterval(() => {
  const left = Math.max(0, Math.round((deadline - Date.now()) / 1000));
  console.log(`progress t-${left}s total=${total} codes=${JSON.stringify([...codes])}`);
}, 15000);

const workers = [];
for (let i = 0; i < CONCURRENCY; i += 1) workers.push(worker());
while (Date.now() < deadline) await new Promise((r) => setTimeout(r, 1000));
stopped = true;
clearInterval(progress);
await Promise.allSettled(workers);

latencies.sort((a, b) => a - b);
const pct = (p) => (latencies.length ? latencies[Math.min(latencies.length - 1, Math.floor((p / 100) * latencies.length))] : 0);
console.log(JSON.stringify({
  target: TARGET,
  duration_s: DURATION_S,
  concurrency: CONCURRENCY,
  total,
  rps: Number((total / DURATION_S).toFixed(1)),
  codes: Object.fromEntries(codes),
  errors,
  p50_ms: Number(pct(50).toFixed(1)),
  p99_ms: Number(pct(99).toFixed(1)),
}));
process.exit(0);
