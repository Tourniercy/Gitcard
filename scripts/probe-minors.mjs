// probe-minors.mjs — oracles des mineurs m-2/m-3 de la revue P4 (gitcard-api).
// Exécuté UNIQUEMENT en conteneur `node:26.13.1-trixie-slim`, --network none.
// Le legacy EXÉCUTÉ tranche :
//   m-2 — PORT non entier (zod l'accepte) : que fait le LISTEN legacy ?
//         (`serve({ fetch, port: 3000.5 })`, entrée prod @hono/vite-build/node)
//   m-3 — bind sur un port occupé : stderr sémantique du legacy.
// Sorties → tests/golden/config-minors/ (stderr byte-exact + manifeste avec
// code de sortie, signal et bloc sémantique sans la pile V8).
import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const NODE_VERSION = process.version;
if (NODE_VERSION !== 'v26.13.1') {
  throw new Error(`Node ${NODE_VERSION} != v26.13.1 (prod) : régénérer avec la bonne image`);
}

const rootURL = new URL('..', import.meta.url);
const root = fileURLToPath(rootURL);
const outDir = `${root}/tests/golden/config-minors`;
mkdirSync(outDir, { recursive: true });

const BASE = { PATH: process.env.PATH, HOME: '/tmp', PAT_1: 'fake-token-1' };
const CHILD = fileURLToPath(new URL('gen-http-golden.mjs', import.meta.url));

function semanticBlock(stderr) {
  // bloc sémantique = tout sauf la pile V8 (« at … », « throw », Node.js vX)
  return stderr
    .split('\n')
    .filter((l) => !/^\s+at /.test(l))
    .join('\n')
    .replace(/^\s*throw .*$/gm, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

// ------------------------- m-2 : PORT = 3000.5 --------------------------------
const m2 = spawnSync(process.execPath, [CHILD], {
  env: { ...BASE, GEN_HTTP_MODE: 'serve-child', PROBE_PORT: '3000.5' },
  encoding: 'buffer',
  timeout: 30000,
});
const m2Stderr = m2.stderr?.toString('utf8') ?? '';
writeFileSync(`${outDir}/port-3000-5.stderr.txt`, m2Stderr);

// ------------------------- m-3 : port occupé ----------------------------------
// enfant 1 : TIENT le port 4199 (restera vivant pendant la tentative 2)
const holder = spawn(process.execPath, [CHILD], {
  env: { ...BASE, GEN_HTTP_MODE: 'serve-child', PROBE_PORT: '4199' },
  stdio: ['ignore', 'pipe', 'ignore'],
});
let holderReady = false;
holder.stdout.on('data', (d) => {
  if (d.toString('utf8').includes('SERVE_OK')) holderReady = true;
});
for (let i = 0; i < 50 && !holderReady; i++) {
  await new Promise((r) => setTimeout(r, 100));
}
if (!holderReady) throw new Error('holder jamais prêt sur 4199');
const m3 = spawnSync(process.execPath, [CHILD], {
  env: { ...BASE, GEN_HTTP_MODE: 'serve-child', PROBE_PORT: '4199' },
  encoding: 'buffer',
  timeout: 30000,
});
const m3Stderr = m3.stderr?.toString('utf8') ?? '';
writeFileSync(`${outDir}/eaddrinuse.stderr.txt`, m3Stderr);
holder.kill('SIGKILL');

const manifest = {
  generator: {
    node: NODE_VERSION,
    entry: 'node scripts/probe-minors.mjs (jamais via pnpm) — enfants GEN_HTTP_MODE=serve-child (serve({fetch, port}) = entrée prod @hono/vite-build/node)',
    pat: 'PAT_1=fake-token-1 (jeton fictif)',
    note: 'stderr byte-exact ; le bloc sémantique exclut la pile V8 (lignes « at … »)',
  },
  m2_port_non_entier: {
    probe_port: '3000.5',
    exitCode: m2.status,
    signal: m2.signal,
    semanticBlock: semanticBlock(m2Stderr),
  },
  m3_bind_occupe: {
    probe_port: '4199',
    exitCode: m3.status,
    signal: m3.signal,
    semanticBlock: semanticBlock(m3Stderr),
  },
};
writeFileSync(`${outDir}/manifest.json`, JSON.stringify(manifest, null, 1));
console.log(JSON.stringify(manifest, null, 1));
console.log('PROBE_MINORS_OK');
