// gen-graphql.mjs — P3 gitcard-api Rust : valeurs attendues = legacy EXÉCUTÉ.
// Aucun appel GitHub : `fetch` est remplacé par un stub qui renvoie les fixtures
// de tests/golden/graphql/fixtures/*.json (entrées écrites à la main) et
// `@sentry/node` est résolu vers un stub sans état (comme dans les tests legacy,
// qui ne l'initialisent jamais).
// Produit (tout en AJOUT sous tests/golden/graphql/) :
//   user-query.txt          — la requête GraphQL VERBATIM (extraite du corps capturé)
//   <fixture>.expected.json — JSON.stringify(FetchResult) legacy, octets exacts
//   <fixture>.error.json    — pour les levées : nom, message, status, remaining, resetAt
//   requests.json           — corps + en-têtes capturés par fixture (jeton factice)
//   svg-md5.json            — md5 + uid du rendu SVG legacy de chaque carte (bonus)
//   number-sweep.json       — 2000 valeurs : littéral d'entrée → JSON.stringify attendu
import { registerHooks } from 'node:module';
import { readdirSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const NODE_VERSION = process.version;
if (NODE_VERSION !== 'v26.13.1') {
  throw new Error(`Node ${NODE_VERSION} != v26.13.1 (prod)`);
}

// Stub Sentry : data URL (l'espace de noms ESM du vrai module n'est pas mutable).
const SENTRY_STUB_URL = `data:text/javascript,export function startSpan(_options, callback) { return callback(); }
export const metrics = { distribution() {} };
export function captureException() {}`;

// Node 26 = type-stripping SANS transform : les parameter properties
// (`public status?: number`) de github.ts ne sont pas supportées. Desugaring
// MÉCANIQUE vérifié (4 remplacements exacts, sémantique identique — la classe
// déclare le champ et l'assigne dans le constructeur), aucune autre ligne
// touchée ; chaque remplacement DOIT trouver sa cible (sinon on lève).
function desugarParameterProperties(src) {
  const hits = [
    ['    public status?: number,\n', 1],
    ['    public remaining: number,\n', 1],
    ['    public resetAt?: Date,\n', 1],
    ['  constructor(public username: string) {', 1],
  ];
  let out = src;
  for (const [needle, n] of hits) {
    const count = out.split(needle).length - 1;
    if (count !== n) {
      throw new Error(`desugar : cible trouvée ${count}x (attendu ${n}) : ${JSON.stringify(needle.slice(0, 40))}`);
    }
    out = out.split(needle).join(needle === hits[3][0] ? '  constructor(username: string) {' : '');
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
    try {
      const resolved = nextResolve(specifier, context);
      if (resolved.url.endsWith('/apps/api/src/fetchers/github.ts')) {
        const real = fileURLToPath(resolved.url);
        const desugared = desugarParameterProperties(readFileSync(real, 'utf8'));
        const tmp = `/tmp/github-desugared-${process.pid}.ts`;
        writeFileSync(tmp, desugared);
        return { url: `file://${tmp}`, shortCircuit: true };
      }
      return resolved;
    } catch (err) {
      if (specifier.startsWith('.')) {
        const url = new URL(specifier, context.parentURL);
        const p = url.pathname;
        for (const cand of [`${p}.ts`, `${p}/index.ts`]) {
          if (existsSync(cand)) {
            return { url: `file://${cand}`, shortCircuit: true };
          }
        }
      }
      throw err;
    }
  },
});

const root = fileURLToPath(new URL('..', import.meta.url));
const outDir = `${root}/tests/golden/graphql`;
mkdirSync(outDir, { recursive: true });

const {
  renderStatsCard,
  renderStreakCard,
  renderLangsCard,
  renderProfileCard,
} = await import(new URL('../packages/svg-renderer/src/index.ts', import.meta.url).href);
const { parseCardOptions } = await import(
  new URL('../apps/api/src/utils/query-params.ts', import.meta.url).href
);
const { fetchGitHubData } = await import(
  new URL('../apps/api/src/fetchers/github.ts', import.meta.url).href
);

const md5hex = (s) => createHash('md5').update(s, 'utf8').digest('hex');

// --- stub fetch : capture la requête, renvoie la fixture -------------------
let captured = null;
let respondWith = null;
globalThis.fetch = async (url, init) => {
  captured = { url, method: init.method, headers: { ...init.headers }, body: init.body };
  return {
    ok: respondWith.ok,
    status: respondWith.status ?? 200,
    statusText: respondWith.statusText ?? 'OK',
    headers: new Headers(respondWith.headers ?? {}),
    json: async () => respondWith.body,
  };
};

const fixtureDir = `${outDir}/fixtures`;
const fixtures = existsSync(fixtureDir)
  ? readdirSync(fixtureDir)
      .filter((f) => f.endsWith('.json'))
      .map((f) => f.replace(/\.json$/, ''))
      .sort()
  : [];
const FAKE_TOKEN = 'fake-token-1';
const RENDER = {
  stats: { fn: renderStatsCard, pick: (r) => r.stats },
  streak: { fn: renderStreakCard, pick: (r) => r.streak },
  langs: { fn: renderLangsCard, pick: (r) => r.languages },
  profile: { fn: renderProfileCard, pick: (r) => r.profile },
};
const defaultOptions = parseCardOptions({});
let uid = 0;
const requests = {};
const svgMd5 = {};
let okCount = 0;
let errCount = 0;

for (const id of fixtures) {
  const fixture = JSON.parse(readFileSync(`${fixtureDir}/${id}.json`, 'utf8'));
  respondWith = { ok: true, status: 200, statusText: 'OK', headers: {}, body: fixture };
  captured = null;
  try {
    const result = await fetchGitHubData(fixture.data.user.login, FAKE_TOKEN);
    okCount++;
    writeFileSync(`${outDir}/${id}.expected.json`, JSON.stringify(result));
    const perCard = {};
    for (const card of ['stats', 'streak', 'langs', 'profile']) {
      const { fn, pick } = RENDER[card];
      const svgUid = uid;
      const svg = fn(pick(result), defaultOptions);
      uid++;
      perCard[card] = { uid: svgUid, md5: md5hex(svg) };
    }
    svgMd5[id] = perCard;
  } catch (err) {
    errCount++;
    writeFileSync(
      `${outDir}/${id}.error.json`,
      JSON.stringify({
        name: err.constructor.name,
        message: err.message,
        status: err.status ?? null,
        remaining: err.remaining ?? null,
        resetAt: err.resetAt instanceof Date ? err.resetAt.toISOString() : null,
      }),
    );
  }
  requests[id] = { ...captured };
}

// Requête VERBATIM : extraite du corps capturé (jamais retapée).
const firstReq = requests[fixtures[0]];
const queryVerbatim = JSON.parse(firstReq.body).query;
writeFileSync(`${outDir}/user-query.txt`, queryVerbatim);

// --- sweep sérialiseur de nombres : 2000 valeurs (mélange fixe) ------------
function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const sweep = [];
{
  const rnd = mulberry32(0x20261124);
  const special = [-0, 0, 1, -1, 123, -123, 2 ** 31, 2 ** 53 - 1, -(2 ** 53) + 1, 1e15, 1e21, -1e21, 1.5e22, 5e-7, 1e-7, 123.456, -12.5, 0.1, 1e300, -1e300, 2 ** 70, 987654321.123, 1 / 3, 2 / 3, 1e-5];
  for (const v of special) {
    sweep.push({ input: Object.is(v, -0) ? '-0' : JSON.stringify(v), expected: JSON.stringify(v) });
  }
  while (sweep.length < 2000) {
    const cat = Math.floor(rnd() * 6);
    let v;
    if (cat === 0) v = Math.floor(rnd() * 4294967296) * (rnd() < 0.5 ? 1 : -1);
    else if (cat === 1) v = Math.floor(rnd() * 2 ** 53) * (rnd() < 0.5 ? 1 : -1);
    else if (cat === 2) v = Math.round((rnd() * 1000 - 500) * 1000) / 1000;
    else if (cat === 3) v = rnd() * 10 ** Math.floor(rnd() * 24 - 6);
    else if (cat === 4) v = (rnd() * 2 - 1) * 10 ** Math.floor(rnd() * 22);
    else v = Math.round(rnd() * 1e6) / 10 ** Math.floor(rnd() * 8);
    if (!Number.isFinite(v)) continue;
    sweep.push({ input: JSON.stringify(v), expected: JSON.stringify(v) });
  }
}
writeFileSync(`${outDir}/number-sweep.json`, JSON.stringify(sweep));

writeFileSync(`${outDir}/svg-md5.json`, JSON.stringify(svgMd5));
writeFileSync(
  `${outDir}/requests.json`,
  JSON.stringify({ tokenUsed: FAKE_TOKEN, requests }),
);
console.log(
  `gen-graphql : ${fixtures.length} fixtures, ${okCount} attendus, ${errCount} erreurs, sweep ${sweep.length}, uid 0..${uid - 1}`,
);
console.log('GEN_GRAPHQL_OK');
