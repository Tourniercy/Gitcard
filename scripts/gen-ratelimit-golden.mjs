// gen-ratelimit-golden.mjs — P5 gitcard-api Rust : oracles du rate-limiter
// `hono-rate-limiter` (apps/api/src/routes/data.ts:20-27) exécutés sur le
// legacy Node v26.13.1 — UNIQUEMENT en conteneur `node:26.13.1-trixie-slim`,
// --network none, dépôt copié dans le conteneur, sorties sous tests/golden/ratelimit/.
// JAMAIS via pnpm. Aucun appel réseau réel : serveur 127.0.0.1, GitHub stubbé
// en process (globalThis.fetch AVANT l'import de apps/api/src/index.ts —
// mécanisme identique à gen-http-golden.mjs pour Sentry/desugar).
//
// Ce que chaque boot prouve (brief §3.A) :
//   window30_31   : (1) la 30e et la 31e requête /api/data — statut, en-têtes
//                   bruts ORDONNÉS, corps (200 comptent, 429 texte).
//   headers_200_404_500 : (2) les en-têtes émis AVANT la limite sur 200/404/500
//                   (RateLimit-Remaining décroît : 404 et 500 COMPTENT).
//   count_404     : (6) 30 × 404 puis la 31e → 429 (un 404 compte).
//   keys          : (3) cf-connecting-ip absent → clé = adresse remote ;
//                   deux clés → compteurs indépendants.
//   reset_61s     : (4) fenêtre FIXE ancrée au 1er hit : reset=60 → 59 à
//                   t0+1,25 s (Math.ceil) → réinitialisé à t0+61,5 s (attente
//                   réelle unique).
//   (5) autres routes : 40 cartes + themes + health → AUCUN en-tête
//       ratelimit-*, aucun 429 (dans le boot window30_31).
//
// Modes : défaut = capture (5 boots enfants) ; rl-child = interne (sert
// l'app avec un stub fetch, RL_STUB = json, PROBE_PORT).
import { registerHooks, createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import net from 'node:net';

const NODE_VERSION = process.version;
if (NODE_VERSION !== 'v26.13.1') {
  throw new Error(`Node ${NODE_VERSION} != v26.13.1 (prod) : régénérer avec la bonne image`);
}

// --- Stub Sentry (identique à gen-http-golden.mjs) ---
const SENTRY_STUB_URL = `data:text/javascript,export function startSpan(_options, callback) { return callback(); }\nexport const metrics = { distribution() {}, count() {} };\nexport function captureException() {}\nexport function init() {}`;

// --- Desugaring mécanique de github.ts (identique à gen-http-golden.mjs) ---
function desugarParameterProperties(src) {
  let out = src;
  // FIX P5 (vs gen-http-golden.mjs) : les parameter properties de la SIGNATURE
  // du constructeur deviennent des paramètres PLAIN (l'annotation `public X` ne
  // survit pas au type-stripping Node), pas des suppressions — gen-http-golden
  // supprimait `status` puis référençait `this.status = status` →
  // ReferenceError à la construction de GitHubApiError (jamais exposé là :
  // ses captures ne construisent jamais GitHubApiError — « fetch failed » est
  // une Error nue). Ici les chemins 404/429 construisent ces classes.
  for (const [needle, plain] of [
    ['    public status?: number,\n', '    status?: number,\n'],
    ['    public remaining: number,\n', '    remaining: number,\n'],
    ['    public resetAt?: Date,\n', '    resetAt?: Date,\n'],
    ['  constructor(public username: string) {', '  constructor(username: string) {'],
  ]) {
    const count = out.split(needle).length - 1;
    if (count !== 1) {
      throw new Error(`desugar : cible trouvée ${count}x (attendu 1) : ${JSON.stringify(needle.slice(0, 40))}`);
    }
    out = out.split(needle).join(plain);
  }
  out = out.replace(
    "    super(message);\n    this.name = 'GitHubApiError';",
    "    super(message);\n    this.status = status;\n    this.name = 'GitHubApiError';",
  );
  out = out.replace(
    '    super(`User not found: ${username}`, 404);\n    this.name =',
    '    super(`User not found: ${username}`, 404);\n    this.username = username;\n    this.name =',
  );
  out = out.replace(
    "    super('GitHub API rate limit exceeded', 429);\n    this.name =",
    "    super('GitHub API rate limit exceeded', 429);\n    this.remaining = remaining;\n    this.resetAt = resetAt;\n    this.name =",
  );
  for (const mark of ['this.status = status;', 'this.username = username;', 'this.remaining = remaining;', 'this.resetAt = resetAt;']) {
    if (!out.includes(mark)) throw new Error(`desugar : marque absente : ${mark}`);
  }
  return out;
}

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === '@sentry/node') {
      return { url: SENTRY_STUB_URL, shortCircuit: true };
    }
    const desugarIfGithub = (urlStr) => {
      if (urlStr.endsWith('/apps/api/src/fetchers/github.ts')) {
        const real = fileURLToPath(urlStr);
        const desugared = desugarParameterProperties(readFileSync(real, 'utf8'));
        const tmp = `/tmp/github-desugared-rl-${process.pid}.ts`;
        writeFileSync(tmp, desugared);
        return `file://${tmp}`;
      }
      return urlStr;
    };
    try {
      const resolved = nextResolve(specifier, context);
      const mapped = desugarIfGithub(resolved.url);
      return mapped === resolved.url ? resolved : { url: mapped, shortCircuit: true };
    } catch (err) {
      if (specifier.startsWith('.')) {
        const url = new URL(specifier, context.parentURL);
        const p = url.pathname;
        for (const cand of [`${p}.ts`, `${p}/index.ts`]) {
          if (existsSync(cand)) {
            return { url: desugarIfGithub(`file://${cand}`), shortCircuit: true };
          }
        }
      }
      throw err;
    }
  },
});

const rootURL = new URL('..', import.meta.url);
const root = fileURLToPath(rootURL);
const outDir = `${root}/tests/golden/ratelimit`;
const PORT = 3000;
const md5hex = (s) => createHash('md5').update(s).digest('hex');

// ============================== MODE rl-child (interne) ==============================
if (process.env.RL_CHILD === '1') {
  const stub = JSON.parse(process.env.RL_STUB);
  const fixture = readFileSync(`${root}/tests/golden/graphql/fixtures/${stub.fixture}.json`, 'utf8');
  globalThis.fetch = async (_url, opts) => {
    const body = JSON.parse(opts.body);
    const login = body.variables.login;
    if (stub.mode === 'fail' || (stub.mode === 'by-user' && login === stub.failUser)) {
      throw new Error('fetch failed (stub)');
    }
    if (stub.mode === 'by-user' && login === stub.notFoundUser) {
      return new Response(
        JSON.stringify({
          errors: [{ message: `Could not resolve to a User with the login of '${stub.notFoundUser}'.` }],
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    }
    return new Response(fixture, { status: 200, headers: { 'content-type': 'application/json' } });
  };
  const requireFromApi = createRequire(
    fileURLToPath(new URL('../apps/api/src/index.ts', import.meta.url)),
  );
  const nodeServerPath = requireFromApi.resolve('@hono/node-server');
  const { serve } = await import(pathToFileURL(nodeServerPath).href);
  const app = (await import(new URL('../apps/api/src/index.ts', import.meta.url).href)).default;
  serve({ fetch: app.fetch, port: Number(process.env.PROBE_PORT ?? PORT), hostname: '127.0.0.1' });
  console.log('RL_SERVE_OK');
  setInterval(() => {}, 10_000); // rester vivant jusqu'au kill du parent
} else {
  await captureAll();
}

// ============================== MODE capture (parent) ================================
async function captureAll() {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  function rawRequest({ method, path, extraHeaders = [] }) {
  const lines = [`${method} ${path} HTTP/1.1`, `Host: 127.0.0.1:${PORT}`, ...extraHeaders, 'Connection: close'];
  if (method === 'POST') lines.splice(2, 0, 'Content-Length: 0');
  return lines.join('\r\n') + '\r\n\r\n';
}

function captureRaw(requestText) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    const socket = net.connect({ host: '127.0.0.1', port: PORT });
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error('timeout capture'));
    }, 15000);
    socket.on('connect', () => socket.write(requestText));
    socket.on('data', (d) => chunks.push(d));
    socket.on('close', () => {
      clearTimeout(timer);
      resolve(Buffer.concat(chunks));
    });
    socket.on('error', (err) => {
      clearTimeout(timer);
      reject(err);
    });
  });
}

function parseRawResponse(buf) {
  const text = buf.toString('latin1');
  const sep = text.indexOf('\r\n\r\n');
  const head = text.slice(0, sep);
  const headLines = head.split('\r\n');
  const statusLine = headLines[0];
  const headers = headLines.slice(1);
  const bodyBytes = buf.subarray(Buffer.byteLength(head, 'latin1') + 4);
  return { statusLine, headers, bodyBytes };
}

// Définition des boots. keepBody : indices (0-based) dont le CORPS est stocké.
const BOOTS = [
  {
    name: 'window30_31',
    stub: { mode: 'ok', fixture: 'name-null' },
    note: '(1)+(2)+(5) 31 × /api/data/window-user → 30e=200 remaining 0, 31e=429 texte ; 40 cartes + themes + health : aucun ratelimit-*',
    probes: [
      ...Array.from({ length: 31 }, (_, i) => ({
        name: `hit_${i + 1}`,
        method: 'GET',
        path: '/api/data/window-user',
        keepBody: i === 28 || i === 29 || i === 30,
      })),
      ...Array.from({ length: 40 }, (_, i) => ({
        name: `card_flood_${i + 1}`,
        method: 'GET',
        path: '/stats/flood-user',
        capture: 'head',
      })),
      { name: 'themes_apres_flood', method: 'GET', path: '/api/themes', keepBody: true },
      { name: 'health_apres_flood', method: 'GET', path: '/health', capture: 'head' },
    ],
  },
  {
    name: 'headers_200_404_500',
    stub: { mode: 'by-user', fixture: 'name-null', notFoundUser: 'ghost-user', failUser: 'fail-user' },
    note: '(2) 200 puis 404 puis 500 — remaining 29→28→27 : erreurs COMPTÉES',
    probes: [
      { name: 'r1_200', method: 'GET', path: '/api/data/ok-user', keepBody: true },
      { name: 'r2_404', method: 'GET', path: '/api/data/ghost-user', keepBody: true },
      { name: 'r3_500', method: 'GET', path: '/api/data/fail-user', keepBody: true },
    ],
  },
  {
    name: 'count_404',
    stub: { mode: 'by-user', fixture: 'name-null', notFoundUser: 'ghost-user', failUser: 'fail-user' },
    note: '(6) 30 × 404 comptent → la 31e = 429',
    probes: [
      { name: `hit_1`, method: 'GET', path: '/api/data/ghost-user', keepBody: true },
      ...Array.from({ length: 28 }, (_, i) => ({
        name: `hit_${i + 2}`,
        method: 'GET',
        path: '/api/data/ghost-user',
        capture: 'head',
      })),
      { name: `hit_30`, method: 'GET', path: '/api/data/ghost-user', capture: 'head' },
      { name: `hit_31`, method: 'GET', path: '/api/data/ghost-user', keepBody: true },
    ],
  },
  {
    name: 'keys',
    stub: { mode: 'ok', fixture: 'name-null' },
    note: '(3) sans cf-connecting-ip : clé = remote (127.0.0.1) ; cf-connecting-ip: 9.9.9.9 → compteur INDÉPENDANT (429 sur la 31e du header, loopback intact)',
    probes: [
      { name: `loop_1`, method: 'GET', path: '/api/data/keys-user', capture: 'head' },
      { name: `loop_2`, method: 'GET', path: '/api/data/keys-user', capture: 'head' },
      { name: `loop_3`, method: 'GET', path: '/api/data/keys-user', keepBody: true },
      ...Array.from({ length: 31 }, (_, i) => ({
        name: `cfip_${i + 1}`,
        method: 'GET',
        path: '/api/data/keys-user',
        extraHeaders: ['cf-connecting-ip: 9.9.9.9'],
        capture: i === 30 ? 'full' : 'head',
        keepBody: i === 30,
      })),
      { name: `loop_4`, method: 'GET', path: '/api/data/keys-user', keepBody: true },
    ],
  },
  {
    name: 'reset_61s',
    stub: { mode: 'ok', fixture: 'name-null' },
    note: '(4) fenêtre FIXE ancrée au 1er hit : reset 60 → 59 à t0+1,25 s → reset total à t0+61,5 s (remaining 29)',
    probes: [
      { name: 'r1_t0', method: 'GET', path: '/api/data/reset-user', keepBody: true },
      { name: 'r2_t0_1250ms', method: 'GET', path: '/api/data/reset-user', delayMs: 1250, keepBody: true },
      { name: 'r3_t0_61500ms', method: 'GET', path: '/api/data/reset-user', delayMs: 60250, keepBody: true },
    ],
  },
  {
    name: 'paths',
    stub: { mode: 'ok', fixture: 'name-null' },
    note: 'sémantique du motif use(\'/api/data/*\') : /api/data nu, /api/data/ (segment vide), /api/data/deep/a/b (profond), POST /api/data/posted — chaque sonde éclaire si le limiter S\'APPLIQUE (en-têtes ratelimit-*) et le statut (route vs fallback notFound)',
    probes: [
      { name: 'bare', method: 'GET', path: '/api/data', keepBody: true },
      { name: 'slash', method: 'GET', path: '/api/data/', keepBody: true },
      { name: 'deep', method: 'GET', path: '/api/data/deep/a/b', keepBody: true },
      { name: 'posted', method: 'POST', path: '/api/data/posted', keepBody: true },
      { name: 'themes_control', method: 'GET', path: '/api/themes', capture: 'head' },
    ],
  },
];

function spawnChild(stub) {
  const child = spawn(process.execPath, [fileURLToPath(import.meta.url)], {
    env: {
      PATH: process.env.PATH,
      HOME: '/tmp',
      PAT_1: 'fake-token-1',
      RL_CHILD: '1',
      RL_STUB: JSON.stringify(stub),
      PROBE_PORT: String(PORT),
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return child;
}

async function waitReady() {
  for (let i = 0; i < 100; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/health`);
      if (r.status === 200) return true;
    } catch {
      await sleep(100);
    }
  }
  return false;
}

const manifest = {
  generator: {
    node: NODE_VERSION,
    entry: 'node scripts/gen-ratelimit-golden.mjs (jamais via pnpm) — rateLimiter hono-rate-limiter de apps/api/src/routes/data.ts:20-27',
    hono: JSON.parse(readFileSync(`${root}/apps/api/node_modules/hono/package.json`, 'utf8')).version,
    honoRateLimiter: JSON.parse(
      readFileSync(
        `${root}/node_modules/.pnpm/hono-rate-limiter@0.5.3_hono@4.13.16/node_modules/hono-rate-limiter/package.json`,
        'utf8',
      ),
    ).version,
    note: 'captures par socket brute (HTTP/1.1, Connection: close), en-têtes bruts ORDONNÉS ; Date dynamique (format vérifié côté port) ; clé sans cf-connecting-ip = remote 127.0.0.1 ; PAT_1=fake-token-1 ; REDIS_URL/SENTRY_DSN absents',
    github: 'stub en process (globalThis.fetch avant import) : ok / 404 GraphQL / throw — jamais api.github.com',
    window: { windowMs: 60000, limit: 30, standardHeaders: 'draft-6', store: 'MemoryStore' },
  },
  boots: [],
};

mkdirSync(outDir, { recursive: true });

for (const boot of BOOTS) {
  const child = spawnChild(boot.stub);
  let childStderr = '';
  child.stderr.on('data', (d) => {
    childStderr += d.toString();
  });
  if (!(await waitReady())) {
    console.error(`boot ${boot.name} jamais prêt. stderr enfant:\n${childStderr}`);
    child.kill('SIGKILL');
    process.exit(1);
  }
  const bootOut = { name: boot.name, stub: boot.stub, note: boot.note, probes: [] };
  for (const probe of boot.probes) {
    if (probe.delayMs) await sleep(probe.delayMs);
    const reqText = rawRequest(probe);
    const raw = await captureRaw(reqText);
    const { statusLine, headers, bodyBytes } = parseRawResponse(raw);
    const entry = {
      name: probe.name,
      method: probe.method,
      path: probe.path,
      request: reqText,
      statusLine,
      status: Number(statusLine.split(' ')[1]),
      headers, // ordre BRUT du legacy (hono via @hono/node-server : alphabétique, puis content-length, Date, Connection)
      bodyBytes: bodyBytes.length,
      bodyMd5: createHash('md5').update(bodyBytes).digest('hex'),
    };
    if (probe.keepBody) {
      entry.bodyFile = `${boot.name}_${probe.name}.body`;
      writeFileSync(`${outDir}/${entry.bodyFile}`, bodyBytes);
    }
    // avance d'horloge du parent (boot reset_61s) : lue par le port pour
    // avancer son horloge VIRTUELLE du même pas (aucune attente réelle).
    if (probe.delayMs) entry.delayMs = probe.delayMs;
    if (probe.extraHeaders) entry.extraHeaders = probe.extraHeaders;
    bootOut.probes.push(entry);
    console.log(`${boot.name}/${probe.name}: ${statusLine}`);
  }
  manifest.boots.push(bootOut);
  bootOut.diagnostics = {
    // stderr complet (aucun secret : PAT fictif, stub local) — le message
    // `Error fetching data for <user>: <err>` de data.ts y est épinglé.
    stderr: childStderr,
  };
  child.kill('SIGKILL');
  await sleep(200);
}

writeFileSync(`${outDir}/manifest.json`, JSON.stringify(manifest, null, 1));
console.log(`RATELIMIT_OK : ${manifest.boots.length} boots, ${manifest.boots.reduce((a, b) => a + b.probes.length, 0)} sondes`);
}
