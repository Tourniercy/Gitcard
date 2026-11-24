// gen-redis-golden.mjs — P4b gitcard-api Rust : ORACLE REDIS du legacy Node v26.13.1.
// Exécuté UNIQUEMENT en conteneur `node:26.13.1-trixie-slim`, --network none.
// Un faux serveur RESP minimal (127.0.0.1) enregistre BYTE-EXACT chaque commande
// envoyée par le legacy exécuté (apps/api/src/index.ts, REDIS_URL=redis://127.0.0.1)
// — même mécanisme que gen-http-golden.mjs : hooks ESM (stub Sentry data-URL,
// desugar mécanique des parameter properties de github.ts) + @hono/node-server.
//
// Aucun appel réseau réel : tout est 127.0.0.1 (--network none). PAT_1 = fake-token-1.
// GitHub est stubbé EN PROCESS (fetch intercepté dans la copie desugared de
// github.ts) → réponse GraphQL déterministe, SVG reproductibles à l'octet.
//
// Scénarios (une seule exécution) :
//   A : boot (flush) + 1re requête carte (GET svg miss, GET data miss, SET data,
//       SET svg) + requête IDENTIQUE (aucune commande → preuve L1 devant L2)
//       + requête /api/data (GET data miss, SET data).
//   B : process FRAIS + store SEEDED après le boot-flush (écriture d'une autre
//       instance) → hit L2 : 1 GET, réponse = la chaîne stockée telle quelle,
//       requête suivante → hit L1 (aucune commande).
//   C : entrée svg EXPIRÉE dans le store (expires < now) → GET puis UNLINK puis
//       miss → régénération (GET data, SET data, SET svg).
//   D : Redis indisponible (serveur qui ACCEPTE puis FERME la connexion) →
//       observations boot/requête (redis-unavailable.json).
//   E : boot AVEC clés préexistantes dans le store → flush = SCAN + UNLINK batch.
//
// Sorties → tests/golden/redis/ : <segment>.hexlog (une commande par ligne,
// HEXA — normalisation unique : "expires":<digits> → "expires":N),
// <segment>.txt (lisible, arguments complets), manifest.json, redis-unavailable.json.
//
// Modes (env GEN_REDIS_MODE) : capture (défaut) | child (interne : sert le legacy).
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

const SENTRY_STUB_URL = `data:text/javascript,export function startSpan(_options, callback) { return callback(); }
export const metrics = { distribution() {}, count() {} };
export function captureException() {}
export function init() {}`;

// Réponse GraphQL déterministe (stub fetch) — stats card sans date → SVG stable.
const USER_STUB = {
  data: {
    user: {
      name: 'Probe User',
      login: 'probe-user',
      createdAt: '2020-01-15T00:00:00Z',
      contributionsCollection: {
        totalCommitContributions: 120,
        restrictedContributionsCount: 5,
        contributionCalendar: {
          totalContributions: 300,
          weeks: [
            {
              contributionDays: [
                { contributionCount: 4, date: '2026-01-01' },
                { contributionCount: 2, date: '2026-01-02' },
                { contributionCount: 0, date: '2026-01-03' },
              ],
            },
            {
              contributionDays: [
                { contributionCount: 6, date: '2026-01-08' },
                { contributionCount: 1, date: '2026-01-09' },
              ],
            },
          ],
        },
      },
      repositories: {
        totalCount: 7,
        nodes: [
          {
            stargazerCount: 10,
            forkCount: 2,
            languages: {
              edges: [
                { size: 500, node: { name: 'TypeScript', color: '#3178c6' } },
                { size: 200, node: { name: 'Rust', color: '#dea584' } },
              ],
            },
          },
        ],
      },
      pullRequests: { totalCount: 3 },
      issues: { totalCount: 1 },
      followers: { totalCount: 9 },
      repositoriesContributedTo: { totalCount: 2 },
    },
  },
};

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
    const desugarIfGithub = (urlStr) => {
      if (urlStr.endsWith('/apps/api/src/fetchers/github.ts')) {
        const real = fileURLToPath(urlStr);
        let desugared = desugarParameterProperties(readFileSync(real, 'utf8'));
        // Stub GitHub : `fetch` lexical (portée module) → réponse déterministe.
        // (JAMAIS de réseau réel — --network none de toute façon.)
        desugared = 'const fetch = globalThis.__GITCARD_FETCH ?? globalThis.fetch;\n' + desugared;
        const tmp = `/tmp/github-desugared-redis-${process.pid}.ts`;
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
const outDir = `${root}/tests/golden/redis`;
const md5hex = (buf) => createHash('md5').update(buf).digest('hex');

// ============================== mode child =====================================
if (process.env.GEN_REDIS_MODE === 'child') {
  globalThis.__GITCARD_FETCH = async () =>
    new Response(JSON.stringify(USER_STUB), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  const requireFromApi = createRequire(
    fileURLToPath(new URL('../apps/api/src/index.ts', import.meta.url)),
  );
  const nodeServerPath = requireFromApi.resolve('@hono/node-server');
  const { serve } = await import(pathToFileURL(nodeServerPath).href);
  const app = (await import(new URL('../apps/api/src/index.ts', import.meta.url).href)).default;
  const port = Number(process.env.PORT || 4101);
  serve({ fetch: app.fetch, port, hostname: '127.0.0.1' });
  console.log('CHILD_READY');
  setInterval(() => {}, 10_000); // rester vivant jusqu'au SIGKILL du parent
}

// ============================ faux serveur RESP ================================
// Parse RESP2 (tableaux de bulk strings), répond le minimum vital, CONSERVE un
// store clé→valeur et ENREGISTRE chaque commande (octets bruts + nom + args).
class FakeRespServer {
  constructor({ dead = false } = {}) {
    this.dead = dead; // dead : accepte puis FERME chaque connexion (scenario D)
    this.store = new Map();
    this.records = []; // { name, args, raw (Buffer), resp (Buffer) }
    this.server = null;
    this.port = null;
  }
  listen() {
    return new Promise((resolve) => {
      this.server = net.createServer((socket) => {
        if (this.dead) {
          socket.destroy();
          return;
        }
        let buf = Buffer.alloc(0);
        const replies = [];
        const tryHandle = () => {
          for (;;) {
            if (buf.length === 0) break;
            const parsed = this.#parseCommand(buf);
            if (!parsed) break;
            buf = buf.subarray(parsed.consumed);
            const resp = this.#handle(parsed.name, parsed.args);
            replies.push(resp);
            this.records.push({ name: parsed.name, args: parsed.args, raw: parsed.raw, resp });
          }
          if (replies.length) {
            socket.write(Buffer.concat(replies.map((r) => Buffer.from(r, 'latin1'))));
            replies.length = 0;
          }
        };
        socket.on('data', (d) => {
          buf = Buffer.concat([buf, d]);
          tryHandle();
        });
      });
      this.server.listen(0, '127.0.0.1', () => {
        this.port = this.server.address().port;
        resolve();
      });
    });
  }
  // parse une commande RESP (tableau de bulk strings) au début du buffer
  #parseCommand(buf) {
    if (buf[0] !== 0x2a) throw new Error(`RESP inattendu : ${buf.subarray(0, 8).toString('latin1')}`);
    let i = 1;
    const nl = buf.indexOf(0x0d, i);
    if (nl < 0) return null;
    const count = Number(buf.subarray(i, nl).toString('latin1'));
    i = nl + 2;
    const marks = [];
    for (let k = 0; k < count; k++) {
      if (buf[i] !== 0x24) throw new Error(`bulk attendu, buf=${buf.subarray(i, i + 8).toString('latin1')}`);
      const nl2 = buf.indexOf(0x0d, i);
      if (nl2 < 0) return null;
      const len = Number(buf.subarray(i + 1, nl2).toString('latin1'));
      const start = nl2 + 2;
      const end = start + len + 2; // + CRLF
      if (buf.length < end) return null;
      marks.push([start, start + len]);
      i = end;
    }
    const args = marks.map(([s, e]) => buf.subarray(s, e).toString('utf8'));
    return { name: args[0].toUpperCase(), args: args.slice(1), consumed: i, raw: buf.subarray(0, i) };
  }
  #scanReply(cursor, { match, count }) {
    const m = /^([^*]+)\*+$/.exec(match ?? '');
    const prefix = m ? m[1] : '';
    const keys = [...this.store.keys()].filter((k) => k.startsWith(prefix));
    const start = Number(cursor);
    const batch = keys.slice(start, start + (count || 10));
    const next = start + batch.length >= keys.length ? '0' : String(start + batch.length);
    const arr = `*${batch.length}\r\n` + batch.map((k) => this.#bulk(k)).join('');
    return `*2\r\n${this.#bulk(next)}${arr}`;
  }
  #bulk(s) {
    const b = Buffer.from(s, 'utf8');
    return `$${b.length}\r\n${b.toString('latin1')}\r\n`;
  }
  #handle(name, args) {
    switch (name) {
      case 'GET': {
        const v = this.store.get(args[0]);
        return v === undefined ? '$-1\r\n' : this.#bulk(v);
      }
      case 'SET': {
        this.store.set(args[0], args[1]); // PX ignoré : le faux serveur n'expire jamais
        return '+OK\r\n';
      }
      case 'UNLINK':
      case 'DEL': {
        let n = 0;
        for (const k of args) if (this.store.delete(k)) n++;
        return `:${n}\r\n`;
      }
      case 'SCAN': {
        // args : cursor [MATCH p] [COUNT n] [TYPE string]
        const cursor = args[0];
        const opts = {};
        for (let i = 1; i < args.length; i += 2) opts[args[i].toLowerCase()] = args[i + 1];
        return this.#scanReply(cursor, { match: opts.match, count: Number(opts.count) });
      }
      case 'MGET': {
        return `*${args.length}\r\n` + args.map((k) => (this.store.has(k) ? this.#bulk(this.store.get(k)) : '$-1\r\n')).join('');
      }
      case 'FLUSHDB':
      case 'FLUSHALL':
        this.store.clear();
        return '+OK\r\n';
      case 'CLIENT':
      case 'PING':
      case 'AUTH':
      case 'SELECT':
      case 'HELLO':
      default:
        return '+OK\r\n';
    }
  }
  close() {
    return new Promise((resolve) => this.server.close(() => resolve()));
  }
}

// ============================ helpers parent ==================================
const delay = (ms) => new Promise((r) => setTimeout(r, ms));

function spawnChild(port, extraEnv = {}) {
  const child = spawn(
    process.execPath,
    [fileURLToPath(import.meta.url)],
    {
      cwd: root,
      env: {
        PATH: process.env.PATH,
        HOME: '/tmp',
        PAT_1: 'fake-token-1',
        GEN_REDIS_MODE: 'child',
        PORT: String(port),
        ...extraEnv,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  child.stdout.on('data', () => {});
  child.stderr.on('data', () => {});
  return child;
}

async function waitReady(port, child) {
  for (let i = 0; i < 100; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/health`);
      if (r.status === 200) return true;
    } catch {
      /* pas prêt */
    }
    await delay(100);
  }
  throw new Error(`child ${port} jamais prêt${child ? ` (exit=${child.exitCode}, signal=${child.signalCode})` : ''}`);
}

function rawRequest(port, target, timeoutMs = 15000) {
  const reqText = `GET ${target} HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nConnection: close\r\n\r\n`;
  return new Promise((resolve, reject) => {
    const chunks = [];
    const socket = net.connect({ host: '127.0.0.1', port });
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error('timeout capture'));
    }, timeoutMs);
    socket.on('connect', () => socket.write(reqText));
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
  const headLines = text.slice(0, sep).split('\r\n');
  const bodyBytes = buf.subarray(Buffer.byteLength(text.slice(0, sep), 'latin1') + 4);
  return { status: Number(headLines[0].split(' ')[1]), headers: headLines.slice(1), bodyBytes };
}

async function getAndParse(port, target, timeoutMs) {
  const raw = await rawRequest(port, target, timeoutMs);
  return parseRawResponse(raw);
}

// ========================= normalisation hexlog/txt ===========================
// Normalisation UNIQUE, à LONGUEUR CONSERVÉE : "expires":<digits> →
// "expires":<zéros> (epoch Date.now() dynamique par construction ; remplacer
// chaque chiffre par '0' garde les longueurs bulk RESP valides → les hexlog
// restent re-parsables, et stables d'une régénération à l'autre).
function normalizeRaw(buf) {
  let s = buf.toString('latin1');
  s = s.replace(/"expires":(\d+)/g, (_m, digits) => `"expires":${'0'.repeat(digits.length)}`);
  return s;
}
function hexlog(records) {
  return records.map((r) => normalizeRaw(r.raw).toString('latin1')).map((s) => Buffer.from(s, 'latin1').toString('hex').toUpperCase()).join('\n') + '\n';
}
function txtlog(records) {
  return (
    records
      .map((r) => {
        const args = r.args.map((a) => JSON.stringify(a)).join(' ');
        return `${r.name} ${args}`;
      })
      .join('\n') + '\n'
  );
}

const LIB_VERSIONS = {};
for (const [name, path] of [
  ['cacheable', '/cacheable@2.3.4/node_modules/cacheable/package.json'],
  ['keyv', '/keyv@5.6.0/node_modules/keyv/package.json'],
  ['@keyv/redis', '/@keyv+redis@5.1.6_keyv@5.6.0/node_modules/@keyv/redis/package.json'],
]) {
  try {
    LIB_VERSIONS[name] = JSON.parse(readFileSync(`${root}/node_modules/.pnpm${path}`, 'utf8')).version;
  } catch {
    LIB_VERSIONS[name] = 'inconnu';
  }
}

// ============================== scénarios =====================================
if (!process.env.GEN_REDIS_MODE || process.env.GEN_REDIS_MODE === 'capture') {
  mkdirSync(outDir, { recursive: true });
  const segments = {}; // name → records[]
  const observations = {};

  function bucketByWindow(name, server, from, to) {
    segments[name] = server.records.slice(from, to);
  }
  function bucketByKey(server, from, to, kind, keySubstr) {
    const out = [];
    for (let i = from; i < to; i++) {
      const r = server.records[i];
      const targets = r.args.filter((a) => typeof a === 'string' && a.includes(keySubstr));
      const isTarget = r.name !== 'CLIENT' && targets.length > 0 && (r.name === 'GET' ? kind === 'get' : r.name === 'SET' ? kind === 'set' : r.name === 'UNLINK' || r.name === 'DEL' ? kind === 'del' : false);
      if (isTarget) out.push(r);
    }
    return out;
  }

  // ---------- Scénario A : boot + miss + sets + hit L1 ----------
  const A = new FakeRespServer();
  await A.listen();
  const childA = spawnChild(4101, { REDIS_URL: `redis://127.0.0.1:${A.port}` });
  await waitReady(4101, childA);
  await delay(400); // laisser le flush (SCAN) atterrir
  let mark = A.records.length;
  const respA1 = await getAndParse(4101, '/stats/probe-user');
  await delay(400);
  const end1 = A.records.length;
  segments.a_boot = A.records.slice(0, mark);
  segments.a_req1_get_svg = bucketByKey(A, mark, end1, 'get', ':svg:');
  segments.a_req1_get_data = bucketByKey(A, mark, end1, 'get', ':data:');
  segments.a_req1_set_data = bucketByKey(A, mark, end1, 'set', ':data:');
  segments.a_req1_set_svg = bucketByKey(A, mark, end1, 'set', ':svg:');
  segments.a_req1_other = A.records.slice(mark, end1).filter((r) => r.name !== 'GET' && r.name !== 'SET');
  observations.a_req1 = { status: respA1.status, bodyMd5: md5hex(respA1.bodyBytes), bodyBytes: respA1.bodyBytes.length, svgBody: respA1.bodyBytes.toString('utf8') };

  const before2 = A.records.length;
  const respA2 = await getAndParse(4101, '/stats/probe-user');
  await delay(400);
  segments.a_req2_all = A.records.slice(before2);
  observations.a_req2 = { status: respA2.status, bodyMd5: md5hex(respA2.bodyBytes), commands: A.records.length - before2 };

  const before3 = A.records.length;
  const respA3 = await getAndParse(4101, '/api/data/other-user');
  await delay(400);
  const end3 = A.records.length;
  segments.a_req3_get_data = bucketByKey(A, before3, end3, 'get', ':data:');
  segments.a_req3_set_data = bucketByKey(A, before3, end3, 'set', ':data:');
  segments.a_req3_other = A.records.slice(before3, end3).filter((r) => r.name !== 'GET' && r.name !== 'SET');
  observations.a_req3 = { status: respA3.status, bodyMd5: md5hex(respA3.bodyBytes) };
  childA.kill('SIGKILL');
  await A.close();

  // ---------- Scénario E : boot AVEC clés préexistantes → flush = SCAN+UNLINK ----
  const E = new FakeRespServer();
  E.store.set('keyv::keyv:svg:stats:olduser:12345678', JSON.stringify({ value: '<svg-old/>', expires: Date.now() + 3_600_000 }));
  E.store.set('keyv::keyv:data:olduser', JSON.stringify({ value: '{"stats":1}', expires: Date.now() + 3_600_000 }));
  await E.listen();
  const childE = spawnChild(4105, { REDIS_URL: `redis://127.0.0.1:${E.port}` });
  await waitReady(4105, childE);
  await delay(400);
  segments.e_boot_flush = E.records.slice();
  observations.e_storeAfterFlush = [...E.store.keys()];
  childE.kill('SIGKILL');
  await E.close();

  // ---------- Scénario B : process frais + store seedé APRÈS le flush → hit L2 --
  const B = new FakeRespServer();
  await B.listen();
  const childB = spawnChild(4102, { REDIS_URL: `redis://127.0.0.1:${B.port}` });
  await waitReady(4102, childB);
  await delay(400); // flush (SCAN sur store vide) terminé
  // store du serveur A : reconstruit depuis ses SET enregistrés (source byte-exacte)
  for (const r of segments.a_req1_set_data) B.store.set(r.args[0], r.args[1]);
  for (const r of segments.a_req1_set_svg) B.store.set(r.args[0], r.args[1]);
  for (const r of segments.a_req3_set_data) B.store.set(r.args[0], r.args[1]);
  segments.b_boot = B.records.slice();
  const bMark = B.records.length;
  const respB1 = await getAndParse(4102, '/stats/probe-user');
  await delay(400);
  const bEnd = B.records.length;
  segments.b_req1_get_svg = bucketByKey(B, bMark, bEnd, 'get', ':svg:');
  segments.b_req1_rest = B.records.slice(bMark, bEnd).filter((r) => !bucketByKey(B, bMark, bEnd, 'get', ':svg:').includes(r));
  observations.b_req1 = { status: respB1.status, bodyMd5: md5hex(respB1.bodyBytes), commands: bEnd - bMark };
  const b2Before = B.records.length;
  const respB2 = await getAndParse(4102, '/stats/probe-user');
  await delay(400);
  segments.b_req2_all = B.records.slice(b2Before);
  observations.b_req2 = { status: respB2.status, bodyMd5: md5hex(respB2.bodyBytes), commands: B.records.length - b2Before };
  childB.kill('SIGKILL');
  await B.close();

  // ---------- Scénario C : entrée svg EXPIRÉE → GET + UNLINK + miss -------------
  // (seed APRÈS le boot-flush : sinon le flush du boot UNLINKerait l'entrée
  // avant toute requête — cf. scénario E.)
  const svgEnvelope = segments.a_req1_set_svg[0].args[1];
  const svgValue = JSON.parse(svgEnvelope).value;
  const svgKey = segments.a_req1_set_svg[0].args[0];
  const C = new FakeRespServer();
  await C.listen();
  const childC = spawnChild(4103, { REDIS_URL: `redis://127.0.0.1:${C.port}` });
  await waitReady(4103, childC);
  await delay(400);
  C.store.set(svgKey, JSON.stringify({ value: svgValue, expires: Date.now() - 1000 }));
  const cBoot = C.records.length;
  const respC1 = await getAndParse(4103, '/stats/probe-user');
  await delay(400);
  const cEnd = C.records.length;
  segments.c_boot = C.records.slice(0, cBoot);
  segments.c_get_svg_expired = bucketByKey(C, cBoot, cEnd, 'get', ':svg:');
  segments.c_unlink_svg = bucketByKey(C, cBoot, cEnd, 'del', ':svg:');
  segments.c_get_data = bucketByKey(C, cBoot, cEnd, 'get', ':data:');
  segments.c_set_data = bucketByKey(C, cBoot, cEnd, 'set', ':data:');
  segments.c_set_svg = bucketByKey(C, cBoot, cEnd, 'set', ':svg:');
  segments.c_other = C.records.slice(cBoot, cEnd).filter((r) => r.name !== 'GET' && r.name !== 'SET' && r.name !== 'UNLINK' && r.name !== 'DEL');
  observations.c_req1 = { status: respC1.status, bodyMd5: md5hex(respC1.bodyBytes), storeAfter: [...C.store.keys()] };
  childC.kill('SIGKILL');
  await C.close();

  // ---------- Scénario D : Redis indisponible (accepte puis ferme) --------------
  const D = new FakeRespServer({ dead: true });
  await D.listen();
  const childD = spawnChild(4104, { REDIS_URL: `redis://127.0.0.1:${D.port}` });
  let dBoot = { ready: false };
  try {
    dBoot.ready = await waitReady(4104, childD);
  } catch (e) {
    dBoot.error = String(e);
  }
  let dRequest = { hung: null };
  const t0 = Date.now();
  try {
    const r = await getAndParse(4104, '/stats/probe-user', 4000);
    dRequest = { hung: false, status: r.status, ms: Date.now() - t0 };
  } catch (e) {
    dRequest = { hung: true, error: String(e), ms: Date.now() - t0 };
  }
  childD.kill('SIGKILL');
  await D.close();
  observations.d = { boot: dBoot, request: dRequest };

  // ------------------------------- écritures ----------------------------------
  const files = [];
  for (const [name, records] of Object.entries(segments)) {
    writeFileSync(`${outDir}/${name}.hexlog`, hexlog(records));
    writeFileSync(`${outDir}/${name}.txt`, txtlog(records));
    files.push({
      segment: name,
      commands: records.map((r) => r.name),
      count: records.length,
      hexlogMd5: md5hex(readFileSync(`${outDir}/${name}.hexlog`)),
      txtMd5: md5hex(readFileSync(`${outDir}/${name}.txt`)),
    });
  }
  writeFileSync(`${outDir}/redis-unavailable.json`, JSON.stringify({
    generator: { node: NODE_VERSION, scenario: 'D — serveur qui accepte puis ferme chaque connexion' },
    ...observations.d,
  }, null, 1));

  // faits dérivés (l'oracle CONSTATE, ne préjuge pas)
  const setSvg = segments.a_req1_set_svg[0];
  const envelope = JSON.parse(setSvg.args[1]);
  const facts = {
    boot: { commands: segments.a_boot.map((r) => r.name), flushdb_flushall_seen: segments.a_boot.some((r) => r.name === 'FLUSHDB' || r.name === 'FLUSHALL') },
    store_after_boot_flush_with_keys: observations.e_storeAfterFlush,
    first_get_miss: { command: segments.a_req1_get_svg.map((r) => `${r.name} ${JSON.stringify(r.args[0])}`)[0] ?? null, get_count_req1: segments.a_req1_get_svg.length + segments.a_req1_get_data.length },
    set: { command: `${setSvg.name} … PX ${setSvg.args[3] ?? null}`, key_example: setSvg.args[0], ttl_args: setSvg.args.slice(2) },
    envelope: { value_type: typeof envelope.value, expires_present: 'expires' in envelope, expires_type: typeof envelope.expires, expires_note: 'epoch ms (Date.now()+ttl) ; chaque chiffre masqué par 0 dans les hexlog (longueur conservée)' },
    expired_get: { commands: [...segments.c_get_svg_expired, ...segments.c_unlink_svg].map((r) => r.name), note: 'enveloppe expires < now → UNLINK puis miss (keyv 5.6.0 getRaw)' },
    prefix: { observed: setSvg.args[0].slice(0, setSvg.args[0].length - 'svg:stats:probe-user:289b4511'.length), full_key_example: setSvg.args[0], separator_note: 'namespace KeyvRedis (::) + préfixe Keyv (keyv:) → clé logique svg:/data: préfixée keyv::keyv:' },
    store_choice: { identical_requests: 2, get_count_req1: segments.a_req1_get_svg.length + segments.a_req1_get_data.length, get_count_req2: segments.a_req2_all.filter((r) => r.name === 'GET').length, conclusion: 'GETs uniquement sur la 1re requête → L1 mémoire DEVANT Redis (cacheable secondary) = l1_plus_l2' },
    hit_l2: { get_count: segments.b_req1_get_svg.length, rest_commands: segments.b_req1_rest.length, bodyMd5: observations.b_req1.bodyMd5, equals_fresh_render_md5: observations.b_req1.bodyMd5 === observations.a_req1.bodyMd5, second_request_commands: observations.b_req2.commands, note: 'chaîne stockée servie telle quelle (byte md5 identique au rendu frais)' },
  };

  const manifest = {
    generator: {
      node: NODE_VERSION,
      entry: 'node scripts/gen-redis-golden.mjs (jamais via pnpm) — faux serveur RESP 127.0.0.1 + legacy apps/api/src/index.ts servi par @hono/node-server',
      github: 'stub fetch en process (réponse GraphQL déterministe, /stats/probe-user)',
      pat: 'PAT_1=fake-token-1 (jeton fictif)',
      libs: LIB_VERSIONS,
      normalization: 'hexlog : octets de chaque commande (RESP), hexadécimal, une ligne par commande ; chaque CHIFFRE de "expires":<epoch> remplacé par 0 (longueur conservée → hexlog re-parsable comme RESP). Les commandes CLIENT SETINFO LIB-VER/LIB-NAME (transport du client, node-redis côté legacy, redis-rs côté port) sont hors sémantique cache et filtrées des deux côtés dans les tests',
      files,
    },
    facts,
  };
  writeFileSync(`${outDir}/manifest.json`, JSON.stringify(manifest, null, 1));

  for (const [k, v] of Object.entries(facts)) {
    console.log(`${k}: ${JSON.stringify(v).slice(0, 220)}`);
  }
  console.log(`GEN_REDIS_OK : ${files.length} segments → tests/golden/redis/`);
  process.exit(0);
}

console.log('GEN_REDIS_DONE');
