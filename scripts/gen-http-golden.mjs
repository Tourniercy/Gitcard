// gen-http-golden.mjs — P4 gitcard-api Rust : oracles HTTP du legacy Node v26.13.1.
// Exécuté UNIQUEMENT en conteneur `node:26.13.1-trixie-slim`, --network none
// (serveur legacy sur 127.0.0.1, captures par socket brute). JAMAIS via pnpm.
//
// Modes (env GEN_HTTP_MODE) :
//   capture (défaut) : SVG d'erreur (errorSvg du legacy) + paramsHash
//                      (parseCardOptions du legacy) + serveur legacy (@hono/node-server)
//                      + matrice HTTP capturée en BRUT (socket) → tests/golden/http/base/
//   capture-cors     : idem capture avec CORS_ORIGIN (boot séparé) → tests/golden/http/cors-origin/
//   cfg              : sondes de config (spawn d'enfants, env invalides) → tests/golden/http/config/
//   cfg-child        : (interne) importe apps/api/src/index.ts ; CONFIG_OK si l'env est valide.
//
// Aucun appel réseau réel : tout est 127.0.0.1. PAT_1 = fake-token-1 (jeton fictif).
import { registerHooks, createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import net from 'node:net';

const NODE_VERSION = process.version;
if (NODE_VERSION !== 'v26.13.1') {
  throw new Error(`Node ${NODE_VERSION} != v26.13.1 (prod) : régénérer avec la bonne image`);
}

// Stub Sentry : data URL (l'espace de noms ESM du vrai module n'est pas mutable)
// — mécanisme identique à scripts/gen-graphql.mjs (jamais initialisé sans DSN).
const SENTRY_STUB_URL = `data:text/javascript,export function startSpan(_options, callback) { return callback(); }
export const metrics = { distribution() {}, count() {} };
export function captureException() {}
export function init() {}`;

// Node 26 = type-stripping SANS transform : les parameter properties
// (`public status?: number`) de github.ts ne sont pas supportées. Desugaring
// MÉCANIQUE repris de scripts/gen-graphql.mjs (4 remplacements exacts +
// 3 assignations de champs, chaque cible DOIT être trouvée).
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

// --- Hook de résolution : extensions .ts + stub Sentry + desugar github.ts ---
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === '@sentry/node') {
      return { url: SENTRY_STUB_URL, shortCircuit: true };
    }
    const desugarIfGithub = (urlStr) => {
      if (urlStr.endsWith('/apps/api/src/fetchers/github.ts')) {
        const real = fileURLToPath(urlStr);
        const desugared = desugarParameterProperties(readFileSync(real, 'utf8'));
        const tmp = `/tmp/github-desugared-${process.pid}.ts`;
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
const httpDir = `${root}/tests/golden/http`;
const PORT = 3000;
const md5hex = (s) => createHash('md5').update(s, 'utf8').digest('hex');

// ================================ MODE cfg-child ================================
if (process.env.GEN_HTTP_MODE === 'cfg-child') {
  await import(new URL('../apps/api/src/index.ts', import.meta.url).href);
  console.log('CONFIG_OK');
  process.exit(0);
}

const { errorSvg } = await import(
  new URL('../apps/api/src/routes/card-response.ts', import.meta.url).href
);
const { parseCardOptions } = await import(
  new URL('../apps/api/src/utils/query-params.ts', import.meta.url).href
);

// ================= 1. SVG d'erreur (gabarit card-response.ts, à l'octet) =========
// errorSvg ne consomme PAS le compteur cardIdCounter (pas de createCardWrapper)
// → uids 512.. « suivants » du corpus (0..511), invariant uid == index.
const errorSvgDir = `${root}/tests/golden/errors`;
mkdirSync(errorSvgDir, { recursive: true });
const ERROR_SVGS = [
  { name: '404-user-not-found', message: 'User not found: probe-user', status: 404, cacheSeconds: 300 },
  { name: '404-user-not-found-escape', message: `User not found: a<b&c"d'e`, status: 404, cacheSeconds: 300 },
  { name: '429-rate-limit', message: 'GitHub API rate limit exceeded. Please try again later.', status: 429, cacheSeconds: 60 },
  { name: '429-all-pat', message: 'All API tokens are rate-limited. Please try again later.', status: 429, cacheSeconds: 60 },
  { name: '502-generic', message: 'An error occurred while generating the card.', status: 502, cacheSeconds: 60 },
];
const errorsManifest = {
  generator: {
    node: NODE_VERSION,
    entry: 'node scripts/gen-http-golden.mjs (jamais via pnpm) — errorSvg de apps/api/src/routes/card-response.ts',
    uidRule: 'uid = suite du corpus global (manifest.json : 0..511) ; errorSvg ne consomme pas le compteur cardIdCounter',
  },
  entries: [],
};
for (const { name, message, status, cacheSeconds } of ERROR_SVGS) {
  const svg = errorSvg(message);
  writeFileSync(`${errorSvgDir}/${name}.svg`, svg);
  errorsManifest.entries.push({
    file: `errors/${name}.svg`,
    uid: 512 + errorsManifest.entries.length,
    md5: md5hex(svg),
    message,
    status,
    cacheSeconds,
    contentType: 'image/svg+xml; charset=utf-8',
  });
}
writeFileSync(`${errorSvgDir}/manifest.json`, JSON.stringify(errorsManifest, null, 1));
console.log(`error-svgs : ${errorsManifest.entries.length} SVG (uid 512..${512 + errorsManifest.entries.length - 1})`);

// ================= 2. paramsHash (md5(JSON.stringify(CardOptions)).slice(0,8)) ====
// parseCardOptions du legacy (zod). Clés répétées : Hono c.req.query() = PREMIER
// gagne (`results[name] ??= value`, hono/dist/utils/url.js) — cf. sondes HTTP
// q_dupe1/q_dupe2 qui l'épinglent côté serveur.
const PARAM_SETS = [
  { label: 'no-query', query: {} },
  { label: 'theme', query: { theme: 'dracula' } },
  { label: 'theme-empty', query: { theme: '' } },
  { label: 'hide-two', query: { hide: 'stars,commits' } },
  { label: 'hide-empty', query: { hide: '' } },
  { label: 'hide-holes', query: { hide: 'a,,b' } },
  { label: 'show-icons-empty', query: { show_icons: '' } },
  { label: 'show-icons-0', query: { show_icons: '0' } },
  { label: 'show-icons-1', query: { show_icons: '1' } },
  { label: 'cache-1800', query: { cache_seconds: '1800' } },
  { label: 'cache-empty', query: { cache_seconds: '' } },
  { label: 'cache-abc', query: { cache_seconds: 'abc' } },
  { label: 'cache-0x10', query: { cache_seconds: '0x10' } },
  { label: 'cache-neg', query: { cache_seconds: '-5' } },
  { label: 'cache-inf', query: { cache_seconds: 'Infinity' } },
  { label: 'bg-empty', query: { bg_color: '' } },
  { label: 'colors', query: { bg_color: '1a1b27', title_color: 'ff79c6' } },
  { label: 'locale-fr', query: { locale: 'fr' } },
  { label: 'locale-unknown', query: { locale: 'xx' } },
  { label: 'locale-pt-BR', query: { locale: 'pt-BR' } },
  { label: 'layout-compact', query: { layout: 'compact' } },
  { label: 'unknown-stripped', query: { unknown: 'x' } },
  { label: 'dupe-first-wins', query: { show_icons: 'true', __second__: 'false' } },
];
mkdirSync(httpDir, { recursive: true });
const paramsOut = [];
for (const { label, query } of PARAM_SETS) {
  const q = { ...query };
  const second = q.__second__;
  delete q.__second__;
  // dupe-first-wins : la REQUÊTE HTTP porte la clé deux fois ; le record vu par
  // parseCardOptions (c.req.query()) ne garde que la PREMIÈRE valeur.
  const record = second !== undefined ? q : q;
  const options = parseCardOptions(record);
  const stringified = JSON.stringify(options);
  paramsOut.push({
    label,
    query: q,
    stringified,
    paramsHash: md5hex(stringified).slice(0, 8),
  });
}
writeFileSync(`${httpDir}/params-hash.json`, JSON.stringify({ node: NODE_VERSION, params: paramsOut }, null, 1));
console.log(`params-hash : ${paramsOut.length} jeux (dont sans query)`);

// ================= 3. Serveur legacy + captures HTTP brutes =======================
const PROBES = [
  { name: 'get_health', method: 'GET', path: '/health' },
  { name: 'post_health', method: 'POST', path: '/health' },
  { name: 'get_healthz', method: 'GET', path: '/healthz' },
  { name: 'post_healthz', method: 'POST', path: '/healthz' },
  { name: 'get_themes', method: 'GET', path: '/api/themes' },
  { name: 'post_themes', method: 'POST', path: '/api/themes' },
  { name: 'get_stats', method: 'GET', path: '/stats/probe-user' },
  { name: 'get_stats_streak', method: 'GET', path: '/stats/probe-user/streak' },
  { name: 'get_stats_langs', method: 'GET', path: '/stats/probe-user/top-langs' },
  { name: 'get_stats_profile', method: 'GET', path: '/stats/probe-user/profile' },
  { name: 'get_data', method: 'GET', path: '/api/data/probe-user' },
  { name: 'get_stats_bad_q', method: 'GET', path: '/stats/probe-user?show_icons=peut-etre' },
  { name: 'get_stats_q_dupe1', method: 'GET', path: '/stats/probe-user?show_icons=peut-etre&show_icons=true' },
  { name: 'get_stats_q_dupe2', method: 'GET', path: '/stats/probe-user?show_icons=true&show_icons=peut-etre' },
  { name: 'get_stats_cs_empty', method: 'GET', path: '/stats/probe-user?cache_seconds=' },
  { name: 'get_stats_cs_inf', method: 'GET', path: '/stats/probe-user?cache_seconds=Infinity' },
  { name: 'get_data_bad_q', method: 'GET', path: '/api/data/probe-user?show_icons=peut-etre' },
  { name: 'get_nope', method: 'GET', path: '/nope' },
  { name: 'post_nope', method: 'POST', path: '/nope' },
  { name: 'get_root', method: 'GET', path: '/' },
  { name: 'get_origin', method: 'GET', path: '/health', extraHeaders: ['Origin: https://example.com'] },
  {
    name: 'options_preflight',
    method: 'OPTIONS',
    path: '/stats/probe-user',
    extraHeaders: ['Origin: https://example.com', 'Access-Control-Request-Method: GET'],
  },
  {
    name: 'options_preflight_ach',
    method: 'OPTIONS',
    path: '/stats/probe-user',
    extraHeaders: ['Origin: https://example.com', 'Access-Control-Request-Method: GET', 'Access-Control-Request-Headers: x-custom, content-type'],
  },
  {
    name: 'options_preflight_nope',
    method: 'OPTIONS',
    path: '/nope',
    extraHeaders: ['Origin: https://example.com', 'Access-Control-Request-Method: GET'],
  },
  { name: 'options_bare', method: 'OPTIONS', path: '/health' },
  { name: 'head_health', method: 'HEAD', path: '/health' },
  { name: 'head_nope', method: 'HEAD', path: '/nope' },
];

function rawRequest({ method, path, extraHeaders = [] }) {
  const lines = [
    `${method} ${path} HTTP/1.1`,
    `Host: 127.0.0.1:${PORT}`,
    ...extraHeaders,
    'Connection: close',
  ];
  if (method === 'POST') lines.splice(2, 0, 'Content-Length: 0');
  return lines.join('\r\n') + '\r\n\r\n';
}

function captureRaw(port, requestText) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    const socket = net.connect({ host: '127.0.0.1', port });
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
  // corps : déchunker si besoin (le legacy Node devrait mettre Content-Length,
  // mais on neutralise tout framing pour comparer les OCTETS du corps)
  let bodyBytes = buf.subarray(Buffer.byteLength(head, 'latin1') + 4);
  const te = headers.find((h) => /^transfer-encoding:/i.test(h));
  if (te && /chunked/i.test(te)) {
    const out = [];
    let rest = bodyBytes;
    for (;;) {
      const nl = rest.indexOf('\r\n');
      const size = parseInt(rest.subarray(0, nl).toString('latin1').split(';')[0], 16);
      if (!size) break;
      out.push(rest.subarray(nl + 2, nl + 2 + size));
      rest = rest.subarray(nl + 2 + size + 2);
    }
    bodyBytes = Buffer.concat(out);
  }
  return { statusLine, headers, bodyBytes };
}

const MODE = process.env.GEN_HTTP_MODE || 'capture';
if (MODE === 'capture' || MODE === 'capture-cors') {
  const outDir = MODE === 'capture' ? `${httpDir}/base` : `${httpDir}/cors-origin`;
  const corsOrigin = process.env.CORS_ORIGIN; // absent en boot A, défini en boot B

  // '@hono/node-server' : résolu depuis apps/api (les bare specifiers depuis
  // scripts/ ne voient pas apps/api/node_modules) — même patron que gen-graphql.
  const requireFromApi = createRequire(
    fileURLToPath(new URL('../apps/api/src/index.ts', import.meta.url))
  );
  const nodeServerPath = requireFromApi.resolve('@hono/node-server');
  const { serve } = await import(pathToFileURL(nodeServerPath).href);
  const app = (await import(new URL('../apps/api/src/index.ts', import.meta.url).href)).default;
  const server = serve({ fetch: app.fetch, port: PORT, hostname: '127.0.0.1' });

  // attendre la readiness
  let ready = false;
  for (let i = 0; i < 100 && !ready; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/health`);
      ready = r.status === 200;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  if (!ready) throw new Error('serveur legacy jamais prêt');

  const manifest = {
    generator: {
      node: NODE_VERSION,
      entry: 'node scripts/gen-http-golden.mjs (jamais via pnpm) — apps/api/src/index.ts servi par @hono/node-server',
      hono: JSON.parse(readFileSync(`${root}/apps/api/node_modules/hono/package.json`, 'utf8')).version,
      nodeServer: '2.1.6',
      corsOrigin: corsOrigin === undefined ? null : corsOrigin,
      note: 'captures par socket brute (HTTP/1.1, Connection: close) ; en-tête Date et timestamp /health : dynamiques (format vérifié par le test, valeur non)',
      pat: 'PAT_1=fake-token-1 (jeton fictif) ; REDIS_URL/SENTRY_DSN absents',
    },
    probes: [],
  };
  mkdirSync(outDir, { recursive: true });
  for (const probe of PROBES) {
    const reqText = rawRequest(probe);
    const raw = await captureRaw(PORT, reqText);
    const { statusLine, headers, bodyBytes } = parseRawResponse(raw);
    writeFileSync(`${outDir}/${probe.name}.body`, bodyBytes);
    manifest.probes.push({
      name: probe.name,
      method: probe.method,
      path: probe.path,
      request: reqText,
      statusLine,
      status: Number(statusLine.split(' ')[1]),
      headers, // ordre du legacy, Date exclue côté test (dynamique)
      bodyFile: `${probe.name}.body`,
      bodyBytes: bodyBytes.length,
      bodyMd5: createHash('md5').update(bodyBytes).digest('hex'),
    });
    console.log(`${probe.name}: ${statusLine} (${bodyBytes.length} o)`);
  }
  writeFileSync(`${outDir}/manifest.json`, JSON.stringify(manifest, null, 1));
  server.close();
  console.log(`CAPTURE_OK ${MODE} : ${manifest.probes.length} sondes → ${outDir}`);
  process.exit(0);
}

// ================= 4. Sondes de config (env invalides → crash zod) =================
if (MODE === 'cfg') {
  const BASE = { PATH: process.env.PATH, HOME: '/tmp' };
  const CASES = [
    { name: 'ok-default', env: { PAT_1: 'fake-token-1' }, expectOk: true },
    { name: 'no-pat1', env: {}, expectOk: false },
    { name: 'pat1-empty', env: { PAT_1: '' }, expectOk: false },
    { name: 'port-abc', env: { PAT_1: 'fake-token-1', PORT: 'abc' }, expectOk: false },
    { name: 'port-0', env: { PAT_1: 'fake-token-1', PORT: '0' }, expectOk: false },
    { name: 'port-70000', env: { PAT_1: 'fake-token-1', PORT: '70000' }, expectOk: false },
    { name: 'port-empty', env: { PAT_1: 'fake-token-1', PORT: '' }, expectOk: true },
    { name: 'loglevel-bad', env: { PAT_1: 'fake-token-1', LOG_LEVEL: 'trace' }, expectOk: false },
    { name: 'loglevel-empty', env: { PAT_1: 'fake-token-1', LOG_LEVEL: '' }, expectOk: true },
    { name: 'cache_ttl-low', env: { PAT_1: 'fake-token-1', CACHE_TTL: '30' }, expectOk: false },
    { name: 'cache_ttl-empty', env: { PAT_1: 'fake-token-1', CACHE_TTL: '' }, expectOk: true },
    { name: 'redis-bad', env: { PAT_1: 'fake-token-1', REDIS_URL: 'notaurl' }, expectOk: false },
    { name: 'redis-empty', env: { PAT_1: 'fake-token-1', REDIS_URL: '' }, expectOk: true },
    { name: 'cors-empty', env: { PAT_1: 'fake-token-1', CORS_ORIGIN: '' }, expectOk: true },
    { name: 'sentry-dsn-bad', env: { PAT_1: 'fake-token-1', SENTRY_DSN: 'bad-dsn' }, expectOk: false },
    { name: 'sentry-rate-high', env: { PAT_1: 'fake-token-1', SENTRY_TRACES_SAMPLE_RATE: '2' }, expectOk: false },
    { name: 'sentry-rate-nan', env: { PAT_1: 'fake-token-1', SENTRY_TRACES_SAMPLE_RATE: 'abc' }, expectOk: false },
    { name: 'sentry-rate-empty', env: { PAT_1: 'fake-token-1', SENTRY_TRACES_SAMPLE_RATE: '' }, expectOk: true },
    { name: 'sentry-env-empty', env: { PAT_1: 'fake-token-1', SENTRY_ENVIRONMENT: '' }, expectOk: true },
  ];
  const cfgDir = `${httpDir}/config`;
  mkdirSync(cfgDir, { recursive: true });
  const results = [];
  for (const { name, env, expectOk } of CASES) {
    const child = spawnSync(process.execPath, [fileURLToPath(import.meta.url)], {
      env: { ...BASE, ...env, GEN_HTTP_MODE: 'cfg-child' },
      encoding: 'buffer',
      timeout: 60000,
    });
    const stderr = child.stderr?.toString('utf8') ?? '';
    const stdout = child.stdout?.toString('utf8') ?? '';
    const ok = child.status === 0 && stdout.includes('CONFIG_OK');
    writeFileSync(`${cfgDir}/${name}.stderr.txt`, stderr);
    // bloc sémantique : tout ce qui précède la pile V8 (lignes « at ... »)
    const messageBlock = stderr
      .split('\n')
      .filter((l) => !/^\s+at /.test(l))
      .join('\n')
      .replace(/^\s*throw .*$/gm, '')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
    results.push({
      name,
      env,
      exitCode: child.status,
      signal: child.signal,
      ok,
      expectOk,
      match: ok === expectOk,
      messageBlock,
    });
    console.log(`${name}: exit=${child.status} ok=${ok} attendu=${expectOk}`);
  }
  writeFileSync(`${cfgDir}/cases.json`, JSON.stringify({ node: NODE_VERSION, results }, null, 1));
  const bad = results.filter((r) => !r.match);
  if (bad.length) {
    console.log(`CFG_MISMATCH: ${bad.map((b) => b.name).join(',')}`);
    process.exit(1);
  }
  console.log('CFG_OK');
  process.exit(0);
}

// ================= 5. mode serve-child (sondes mineurs m-2/m-3) ================
// Interne à scripts/probe-minors.mjs : sert l'app SUR UN PORT DONNÉ
// (PROBE_PORT, peut être non entier) exactement comme l'entrée de prod
// @hono/vite-build/node (`serve({ fetch: app.fetch, port })`) — pour capturer
// le stderr du LISTEN (ERR_SOCKET_BAD_PORT / EADDRINUSE). N'écrit aucun
// fichier, ne touche aucune capture existante.
if (MODE === 'serve-child') {
  const requireFromApi = createRequire(
    fileURLToPath(new URL('../apps/api/src/index.ts', import.meta.url))
  );
  const nodeServerPath = requireFromApi.resolve('@hono/node-server');
  const { serve } = await import(pathToFileURL(nodeServerPath).href);
  const app = (await import(new URL('../apps/api/src/index.ts', import.meta.url).href)).default;
  const port = Number(process.env.PROBE_PORT ?? PORT);
  serve({ fetch: app.fetch, port });
  console.log('SERVE_OK');
  setInterval(() => {}, 10_000); // rester vivant jusqu'au kill du parent
  process.exit(0); // inaccessible (setInterval maintient la boucle) — garde
}

console.log('GEN_HTTP_OK');
