// gen-golden.mjs — P0 gitcard-api Rust : corpus golden v1 + tables Intl + sweep libm + octicons.
// À exécuter UNIQUEMENT via le binaire node de l'image (node scripts/gen-golden.mjs),
// JAMAIS via pnpm (`.npmrc` force use-node-version=24.13.1 ≠ prod 26.13.1).
// Aucun appel réseau. Fixtures 100 % fictives (tests/golden/fixtures/).
import { registerHooks } from 'node:module';
import { createRequire } from 'node:module';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const NODE_VERSION = process.version;
console.log(`node ${NODE_VERSION}`);
if (NODE_VERSION !== 'v26.13.1') {
  throw new Error(`Node ${NODE_VERSION} != v26.13.1 (prod) : régénérer le corpus avec la bonne image`);
}

// --- Hook de résolution : les sources legacy importent sans extension ('./types') ---
registerHooks({
  resolve(specifier, context, nextResolve) {
    try {
      return nextResolve(specifier, context);
    } catch (err) {
      if (specifier.startsWith('.')) {
        const base = context.parentURL;
        const url = new URL(specifier, base);
        const p = url.pathname;
        for (const cand of [`${p}.ts`, `${p}/index.ts`]) {
          const candUrl = `file://${cand}`;
          if (existsSync(fileURLToPath(candUrl))) {
            return { url: candUrl, shortCircuit: true };
          }
        }
      }
      throw err;
    }
  },
});

const rootURL = new URL('..', import.meta.url);
const root = fileURLToPath(rootURL);
const goldenDir = `${root}/tests/golden`;

// --- Imports du legacy (type-stripping natif, pas de build) ---
const { renderStatsCard, renderStreakCard, renderLangsCard, renderProfileCard, LOCALE_CODES } =
  await import(new URL('../packages/svg-renderer/src/index.ts', import.meta.url).href);
const { parseCardOptions } = await import(
  new URL('../apps/api/src/utils/query-params.ts', import.meta.url).href
);
const requireFrom = createRequire(
  fileURLToPath(new URL('../packages/svg-renderer/src/index.ts', import.meta.url))
);
const octicons = requireFrom('@primer/octicons');

const md5hex = (s) => createHash('md5').update(s, 'utf8').digest('hex');

// ================= 1. Fixtures fictives =================
const fixtures = ['big-numbers', 'empty', 'escape-user', 'newcomer', 'single-lang', 'ties', 'typical'].map(
  (id) => ({ id, data: JSON.parse(readFileSync(`${goldenDir}/fixtures/${id}.json`, 'utf8')) })
);

// ================= 2. Matrice d'options (bornée, cf. brief §5) =================
const OPTION_SETS = [
  { label: 'default', query: {} },
  { label: 'hide', query: { hide: 'stars,commits' } },
  { label: 'colors', query: { bg_color: '1a1b27', title_color: 'ff79c6', text_color: 'e6e6e6', icon_color: '8be9fd', border_color: '6272a4' } },
  { label: 'no-icons', query: { show_icons: 'false' } },
  { label: 'no-border', query: { hide_border: 'true' } },
  { label: 'no-title', query: { hide_title: 'true' } },
  { label: 'fr', query: { locale: 'fr' } },
  { label: 'de', query: { locale: 'de' } },
  { label: 'ar', query: { locale: 'ar' } },
  { label: 'hi', query: { locale: 'hi' } },
  { label: 'ja', query: { locale: 'ja' } },
  { label: 'cache-min', query: { cache_seconds: '1800' } },
  { label: 'theme-dark', query: { theme: 'dracula' } },
  { label: 'compact', query: { layout: 'compact', hide_border: 'true' }, langsOnly: true },
];
const CARDS = ['stats', 'streak', 'langs', 'profile'];
const RENDER = { stats: renderStatsCard, streak: renderStreakCard, langs: renderLangsCard, profile: renderProfileCard };

// Ordre de rendu FIGÉ (documenté) : fixtures → cartes → options ; uid = index global depuis 0.
for (const card of CARDS) rmSync(`${goldenDir}/${card}`, { recursive: true, force: true });

const manifest = {
  generator: {
    node: NODE_VERSION,
    entry: 'node scripts/gen-golden.mjs (jamais via pnpm)',
    uidRule: 'uid = index global des rendus dans l ordre: fixtures[] x cartes[] x options[]; commence a 0; un createCardWrapper par rendu',
    note: 'cache_seconds n entre jamais dans les octets du SVG (headers/ETag uniquement) : variantes cache-* = mêmes octets, hash différent',
  },
  counts: {},
  entries: [],
};

let uid = 0;
let totalBytes = 0;
for (const { id, data } of fixtures) {
  for (const card of CARDS) {
    for (const opt of OPTION_SETS) {
      if (opt.langsOnly && card !== 'langs') continue;
      const options = parseCardOptions(opt.query);
      const svg = RENDER[card](data[card === 'langs' ? 'langs' : card] ?? data[card], options);
      const rel = `${card}/${id}/${opt.label}.svg`;
      mkdirSync(`${goldenDir}/${card}/${id}`, { recursive: true });
      writeFileSync(`${goldenDir}/${rel}`, svg);
      const bytes = Buffer.byteLength(svg, 'utf8');
      totalBytes += bytes;
      manifest.entries.push({
        file: rel,
        card,
        fixture: id,
        label: opt.label,
        uid: uid++,
        md5: md5hex(svg),
        paramsHash: md5hex(JSON.stringify(options)).slice(0, 8),
        options: JSON.parse(JSON.stringify(options)),
        query: opt.query,
      });
    }
  }
}
manifest.counts = { files: manifest.entries.length, bytes: totalBytes, fixtures: fixtures.length };
writeFileSync(`${goldenDir}/manifest.json`, JSON.stringify(manifest, null, 2));
console.log(`corpus: ${manifest.entries.length} SVG, ${totalBytes} octets, uid 0..${uid - 1}`);

// ================= 3. Tables Intl (NumberFormat + DisplayNames + dates) =================
const intSamples = [0, 999, 1234567, 1500000, 123456789];
const locales = {};
for (const code of LOCALE_CODES) {
  const nf = new Intl.NumberFormat(code);
  const parts = nf.formatToParts(123456789);
  const groupParts = parts.filter((p) => p.type === 'group').map((p) => p.value);
  const intDigits = parts.filter((p) => p.type === 'integer').map((p) => p.value).join('');
  // tailles de groupes de droite à gauche, dédupliquées en [primaire, secondaire?]
  const chunkLens = parts.filter((p) => p.type === 'integer').map((p) => p.value.length);
  const sizes = chunkLens.slice().reverse();
  const collapsed = [sizes[0]];
  for (let i = 1; i < sizes.length; i++) if (sizes[i] !== collapsed[collapsed.length - 1]) collapsed.push(sizes[i]);
  const zeroPart = nf.formatToParts(0).find((p) => p.type === 'integer').value;
  const digits = [zeroPart[0]];
  for (let i = 0; i < 9; i++) digits.push(intDigits[i]);
  const number = {};
  for (const n of intSamples) number[String(n)] = nf.format(n);
  locales[code] = {
    separatorRightToLeft: groupParts.slice().reverse(),
    groupSizesRightToLeft: collapsed,
    digits,
    label: new Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code,
    number,
  };
}
// dates : toutes les dates présentes dans les fixtures (streak + calendrier profile)
const dateStrs = new Set();
for (const { data } of fixtures) {
  const s = data.streak;
  for (const k of ['currentStreakStart', 'currentStreakEnd', 'longestStreakStart', 'longestStreakEnd'])
    if (s[k]) dateStrs.add(s[k]);
  for (const d of data.profile.contributionCalendar) dateStrs.add(d.date);
}
const dateTimeFmt = new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric', year: 'numeric' });
const dates = {};
const months = {};
for (const d of [...dateStrs].sort()) {
  dates[d] = dateTimeFmt.format(new Date(d));
  months[d] = new Date(d).toLocaleString('en', { month: 'short' });
}
const intlTables = { node: NODE_VERSION, localeCount: LOCALE_CODES.length, locales, dates, months };
writeFileSync(`${goldenDir}/intl-tables.json`, JSON.stringify(intlTables, null, 1));
console.log(`intl: ${LOCALE_CODES.length} locales (${LOCALE_CODES.join(',')}), ${dateStrs.size} dates`);

// ================= 4. Échantillons numériques (toFixed + String) =================
const dv = new DataView(new ArrayBuffer(8));
const bitsOf = (x) => (dv.setFloat64(0, x), dv.getBigUint64(0));
const ofBits = (b) => (dv.setBigUint64(0, b), dv.getFloat64(0));

const fixedExplicit = [
  [42.25, 1], [1.005, 2], [2.675, 2], [-0.04, 1], [1e21, 0], [1e21, 2], [1e-7, 2], [1e-7, 7],
  [123456789123456780000, 0], [2.5, 0], [-2.5, 0], [0.5, 0], [-0.5, 0], [0.125, 2], [-0.125, 2],
  [0.15, 1], [-0.15, 1], [12.25, 1], [12.35, 1], [42.5, 1], [27.375, 2], [17.875, 2], [100.25, 2],
  [0, 1], [-0, 1], [1745, 0], [1e-6, 2], [1e-6, 7], [5e-324, 0], [5e-324, 2],
  [NaN, 2], [Infinity, 2], [-Infinity, 2], [1.7976931348623157e308, 2], [99.99999, 2], [-99.99999, 2],
];
const stringExplicit = [
  0, -0, 1745, 0.1, 0.2, 0.30000000000000004, 1e21, 1e-7, 1e-6, 123456789123456780000,
  5e-324, 1.7976931348623157e308, 1 / 3, -1 / 3, 2 ** 53, 2 ** 53 + 2, 123456789.345,
  12.25, 41.1764705882353, 99.99999999999999, -0.0000001, 1e20, 9007199254740993,
];
const MASK64 = (1n << 64n) - 1n;
let lcg = 0x9e3779b97f4a7c15n;
function nextU64() {
  lcg ^= (lcg << 13n) & MASK64;
  lcg ^= lcg >> 7n;
  lcg ^= (lcg << 17n) & MASK64;
  return (lcg * 0x2545f4914f6cdd1dn) & MASK64;
}
const randDoubles = [];
for (let i = 0; i < 6000; i++) {
  const bits = nextU64();
  let b = bits;
  if (i % 2 === 0) {
    // exponent "réaliste" : ~2^-23 .. ~2^56, mantisse aléatoire
    const exp = 1000n + BigInt(i % 80);
    b = (bits & 1n << 63n) | (exp << 52n) | (bits & (1n << 52n) - 1n);
  }
  const raw = b >> 52n & 0x7ffn;
  if (raw === 0x7ffn) continue; // pas de NaN/Inf aléatoires dans le sweep toFixed (cas dédiés ci-dessus)
  randDoubles.push(ofBits(b));
}
const fixedSamples = [];
for (const [x, d] of fixedExplicit) fixedSamples.push([bitsOf(x).toString(16), d, x.toFixed(d)]);
for (const x of randDoubles) for (const d of [0, 1, 2]) fixedSamples.push([bitsOf(x).toString(16), d, x.toFixed(d)]);
const stringSamples = [];
for (const x of stringExplicit) stringSamples.push([bitsOf(x).toString(16), String(x)]);
for (const x of randDoubles) stringSamples.push([bitsOf(x).toString(16), String(x)]);
writeFileSync(
  `${goldenDir}/num-samples.json`,
  JSON.stringify({ node: NODE_VERSION, fixed: fixedSamples, string: stringSamples })
);
console.log(`num: ${fixedSamples.length} toFixed, ${stringSamples.length} String`);

// ================= 5. Sweep libm cos/sin =================
const angleSet = [];
const pushAngle = (a) => angleSet.push(a);
for (const { data } of fixtures) {
  let cur = 0;
  for (const lang of data.langs.languages) {
    const sweep = (lang.percentage / 100) * Math.PI * 2;
    const gap = 0.02;
    pushAngle(cur + gap);
    pushAngle(cur + sweep - gap);
    cur += sweep;
  }
}
for (const a of [...angleSet]) pushAngle(a - Math.PI / 2);
for (let i = 0; i <= 4096; i++) pushAngle((i / 4096) * Math.PI * 2);
const sweep = angleSet.map((a) => [bitsOf(a).toString(16), bitsOf(Math.cos(a)).toString(16), bitsOf(Math.sin(a)).toString(16)]);
writeFileSync(`${goldenDir}/libm-sweep.json`, JSON.stringify({ node: NODE_VERSION, angles: sweep }));
console.log(`libm: ${sweep.length} angles`);

// ================= 6. octicons 19.23.1 =================
const ICONS = ['star', 'repo-forked', 'git-commit', 'git-pull-request', 'issue-opened', 'repo', 'calendar', 'flame', 'trophy', 'history', 'mail'];
const out = {};
for (const name of ICONS) {
  const v16 = octicons[name].heights[16];
  if (!v16 || !v16.path) throw new Error(`octicon manquant: ${name}`);
  out[name] = { width: v16.width, path: v16.path };
}
const octiconsPkg = JSON.parse(readFileSync(`${root}/packages/svg-renderer/node_modules/@primer/octicons/package.json`, 'utf8'));
const octJson = JSON.stringify({ source: `@primer/octicons@${octiconsPkg.version}`, icons: out }, null, 1);
writeFileSync(`${root}/rust/src/octicons.json`, octJson);
mkdirSync(`${goldenDir}/meta`, { recursive: true });
writeFileSync(`${goldenDir}/meta/octicons-extract.json`, octJson);
console.log(`octicons: ${Object.keys(out).length} icônes 16px, version ${octiconsPkg.version}`);
console.log('GEN_OK');
