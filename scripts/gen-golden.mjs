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
const {
    renderStatsCard,
    renderStreakCard,
    renderLangsCard,
    renderProfileCard,
    LOCALE_CODES,
    THEME_NAMES,
    THEME_GROUPS,
    getTheme,
} = await import(new URL('../packages/svg-renderer/src/index.ts', import.meta.url).href);
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

// ================= 10bis. Extension fix3 : MODE ENFANT (invoqué par la section 10) =================
// Généré par un processus node FRAÎS : le compteur legacy `cardIdCounter`
// (module-global du renderer) démarre à 0 → 459 rendus à jeter l'amènent à 459,
// puis les 53 SVG edge-fix3 sont produits avec uid 459..511 = LEUR INDEX dans le
// manifeste (invariant `uid == index` épingle par rust/tests/parity.rs). Le
// compteur du processus PARENT n'est pas déplacé : les méta date/counters-extract
// (su/pu/u) et les 459 SVG d'origine restent inchangés d'un octet (diff
// tests/golden = ajouts + manifeste uniquement).
if (process.env.GEN_FIX3_CHILD === '1') {
  const fix3 = JSON.parse(readFileSync(`${goldenDir}/fixtures/edge-fix3.json`, 'utf8'));
  const defaults = parseCardOptions({});
  for (let i = 0; i < 459; i++) renderStatsCard(fix3.stats, defaults); // compteur legacy : 0 → 459
  let childUid = 459;
  let written = 0;
  for (const card of CARDS) {
    for (const opt of OPTION_SETS) {
      if (opt.langsOnly && card !== 'langs') continue;
      const options = parseCardOptions(opt.query);
      const label = `x-edge-fix3-${opt.label}`;
      const svg = RENDER[card](fix3[card === 'langs' ? 'langs' : card] ?? fix3[card], options);
      mkdirSync(`${goldenDir}/${card}/edge-fix3`, { recursive: true });
      writeFileSync(`${goldenDir}/${card}/edge-fix3/${label}.svg`, svg);
      written++;
      childUid++;
    }
  }
  console.log(`fix3-enfant : ${written} SVG, uid 459..${childUid - 1}`);
  process.exit(0);
}

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
// ================= 2bis. Extension P2 : 1 fixture × carte stats × 35 thèmes =================
// APPENDUE À LA FIN du manifeste (les uid des entrées v1 restent inchangés).
// Ordre = THEME_NAMES (ordre d'insertion de themes/index.ts, oracle Node).
const THEME_FIXTURE = fixtures.find((f) => f.id === 'typical');
for (const themeName of THEME_NAMES) {
  const options = parseCardOptions({ theme: themeName });
  // préfixe x-theme- : évite toute collision avec les labels v1 (ex. 'theme-dark'
  // = variante dracula de la matrice d'options v1)
  const label = `x-theme-${themeName}`;
  const svg = renderStatsCard(THEME_FIXTURE.data.stats, options);
  const rel = `stats/${THEME_FIXTURE.id}/${label}.svg`;
  mkdirSync(`${goldenDir}/stats/${THEME_FIXTURE.id}`, { recursive: true });
  writeFileSync(`${goldenDir}/${rel}`, svg);
  totalBytes += Buffer.byteLength(svg, 'utf8');
  manifest.entries.push({
    file: rel,
    card: 'stats',
    fixture: THEME_FIXTURE.id,
    label,
    uid: uid++,
    md5: md5hex(svg),
    paramsHash: md5hex(JSON.stringify(options)).slice(0, 8),
    options: JSON.parse(JSON.stringify(options)),
    query: { theme: themeName },
  });
}
console.log(`themes: ${THEME_NAMES.length} rendus stats × fixture ${THEME_FIXTURE.id}, uid ${THEME_NAMES.length} finis à ${uid - 1}`);

// ================= 2ter. Extension fix2 (R6) : fixture non ASCII + email non null =================
// APPENDUE après l'extension thèmes : les uid des 406 entrées précédentes restent
// inchangés → les SVG existants sont réécrits à l'identique (diff = ajouts + manifeste).
// Étiquettes préfixées x- : aucune collision avec les labels v1 ni x-theme-*.
const FIX2_FIXTURES = ['unicode'].map((id) => ({
  id,
  data: JSON.parse(readFileSync(`${goldenDir}/fixtures/${id}.json`, 'utf8')),
}));
for (const { id, data } of FIX2_FIXTURES) {
  for (const card of CARDS) {
    for (const opt of OPTION_SETS) {
      if (opt.langsOnly && card !== 'langs') continue;
      const options = parseCardOptions(opt.query);
      const label = `x-${id}-${opt.label}`;
      const svg = RENDER[card](data[card === 'langs' ? 'langs' : card] ?? data[card], options);
      const rel = `${card}/${id}/${label}.svg`;
      mkdirSync(`${goldenDir}/${card}/${id}`, { recursive: true });
      writeFileSync(`${goldenDir}/${rel}`, svg);
      totalBytes += Buffer.byteLength(svg, 'utf8');
      manifest.entries.push({
        file: rel,
        card,
        fixture: id,
        label,
        uid: uid++,
        md5: md5hex(svg),
        paramsHash: md5hex(JSON.stringify(options)).slice(0, 8),
        options: JSON.parse(JSON.stringify(options)),
        query: opt.query,
      });
    }
  }
}
console.log(`fix2 R6 : fixture(s) ${FIX2_FIXTURES.map((f) => f.id).join(', ')} rendus, uid finis à ${uid - 1}`);

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
  // séparateur décimal (fix2 R4/R2 : nécessaire au format Intl des nombres fractionnaires)
  const decimalPart = nf.formatToParts(1.5).find((p) => p.type === 'decimal');
  const number = {};
  for (const n of intSamples) number[String(n)] = nf.format(n);
  locales[code] = {
    separatorRightToLeft: groupParts.slice().reverse(),
    groupSizesRightToLeft: collapsed,
    digits,
    decimal: decimalPart ? decimalPart.value : '',
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
// fix2 R4 : oracle du repli de locale — `Intl.NumberFormat(tag_inconnu)` résout vers
// la locale par défaut du runtime (regroupement « root ») ; tags invalides jettent
// une RangeError côté Intl, mais parseCardOptions (zod) mappe déjà tout hors
// LOCALE_CODES vers 'en', donc la crate ne voit que des tags valides-sans-données.
const localeFallback = {};
for (const bad of ['xx', 'zz', 'qq']) {
  localeFallback[bad] = new Intl.NumberFormat(bad).format(1234567);
}
intlTables.localeFallback = localeFallback;
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

// ================= 7. Extraits legacy pour tests unitaires Rust (P2) =================
// Oracle thèmes : ordre, groupes, getTheme (repli nom inconnu + casse).
mkdirSync(`${goldenDir}/meta`, { recursive: true });
const themesOracle = {
  node: NODE_VERSION,
  themeNames: THEME_NAMES,
  groups: THEME_GROUPS,
  defaultTheme: getTheme('default'),
  unknownTheme: getTheme('pas-un-theme'),
  caseSensitiveUnknown: getTheme('Dracula'),
};
writeFileSync(`${goldenDir}/meta/themes-extract.json`, JSON.stringify(themesOracle, null, 1));

// Oracle createArcPath (exporté mais inutilisé par les cartes — porté quand même).
const { createArcPath } = await import(
  new URL('../packages/svg-renderer/src/cards/base-card.ts', import.meta.url).href
);
const ARC_ANGLES = [
  0, 0.02, 0.1, 0.3, 0.55, 1.0, 1.25, 2.0, 2.75, 3.0, 3.5, 4.0, 4.75, 5.0,
  Math.PI / 6, Math.PI / 4, Math.PI / 2, Math.PI, 2 * Math.PI, 6.283185307179586,
];
const arcSamples = [];
for (const a1 of ARC_ANGLES) {
  for (const a2 of [a1 + 0.1, a1 + Math.PI / 3, a1 + 2.5]) {
    arcSamples.push({ startAngle: a1, endAngle: a2, out: createArcPath(90, 110, 55, a1, a2) });
  }
}
writeFileSync(
  `${goldenDir}/meta/arc-extract.json`,
  JSON.stringify({ node: NODE_VERSION, cx: 90, cy: 110, radius: 55, samples: arcSamples }, null, 1)
);
console.log(`p2-meta: thèmes ${THEME_NAMES.length} / ${THEME_GROUPS.length} groupes, ${arcSamples.length} arcs createArcPath`);

// ================= 8. Oracle fix2 R1 : parsing `new Date(x)` (V8) =================
// Pour chaque entrée : md5 du SVG legacy de (i) la carte streak (les 4 champs date
// = input) et (ii) la carte profile (createdAt = input ET toutes les dates du
// contributionCalendar = input). Quand le legacy LÈVE (RangeError « Invalid time
// value » sur date invalide), on enregistre `err:<nom>` — le port Rust doit échouer
// de même (aucun panic). uid = valeur du compteur global cardIdCounter au moment
// du rendu (+1 par rendu réussi ; un rendu en échec ne consomme rien).
const ABSENT = Symbol('absent');
const serInput = (v) => (v === ABSENT ? { $absent: true } : v);

const dateInputsRepresentative = [
  // ISO date seule
  '2024-01-05', '2026-02-14', '1970-01-01', '2000-02-29',
  // ISO date-heure, avec/sans Z, fraction, séparateur espace
  '2024-01-05T12:00:00Z', '2024-01-05T12:00:00.500Z', '2024-01-05T12:00:00.123456Z',
  '2024-01-05T12:00:00.999Z', '2024-01-05T12:00:00.9999Z', '2024-01-05T12:00:00',
  '2024-01-05T12:00', '2024-01-05 12:00:00',
  // décalages
  '2024-01-05T12:00:00+02:00', '2024-01-05T12:00:00-05:30', '2024-01-05T12:00:00+0200',
  '2024-01-05T00:00:00+23:59', '2024-01-05T00:30:00-01:00',
  // année seule / année-mois
  '2024', '1970', '2000', '2024-06', '2024-12', '1999-12',
  // jours hors bornes (rollover V8)
  '2026-02-30', '2024-02-30', '2024-02-31', '2024-04-31', '2024-01-00', '2024-01-32',
  '2023-12-00', '2024-02-00',
  // mois 00 / 13+ / composantes massives
  '2024-00-15', '2024-13-01', '2024-13-32', '2024-20-20', '2024-99-99',
  // heure hors bornes
  '2024-01-05T24:00:00Z', '2024-01-05T12:60:00Z',
  // mois en lettres (parseur legacy V8)
  'January 5, 2024', 'Jan 5, 2024', '5 January 2024', 'JANUARY 5, 2024', 'jan 5 2024',
  'February 30, 2026', 'Feb 29, 2024', 'Feb 29, 2025', 'Dec 25, 1999',
  'september 21, 2023', 'January 2024', 'Jan 5', 'May 2024',
  // formats type RFC 2822
  'Mon, 05 Jan 2024 12:00:00 GMT', 'Mon, 05 Jan 2024 12:00:00 +0200',
  'Fri, 05 Jan 2024 12:00:00 UTC', '05 Jan 2024 12:00:00 GMT',
  'Tue, 01 Apr 2025 00:00:00 GMT', 'Sun, 03 Mar 2024 23:59:59 PST',
  // numériques non ISO (parseur legacy)
  '2026-1-5', '01/05/2024', '2024/01/05', '2024/1/5', '12/25/2023', '2/3/24', '1/5',
  // formes diverses acceptées/refusées
  '0024-05-06', '0099-12-31', '10000', '  2024-01-05  ', '2024-01-05z', '+002024-01-05',
  '1735689600000', '20240105',
  // refus attendus
  '', ' ', 'garbage', 'not a date', 'x', '2026-01-01T12Z', 'null', 'undefined',
  // non-chaînes : null, nombre (epoch), champ absent
  null, 1735689600000, 0, -86400000, 951782400000, 12.5, ABSENT,
];

// liste pseudo-aléatoire : graine fixe (mulberry32), alphabet mixte, longueur 0..40
function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const RAND_N = 2000;
const RAND_ALPHA = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -:,./+';
const dateInputsRandom = [];
{
  const rnd = mulberry32(0x20261124);
  for (let i = 0; i < RAND_N; i++) {
    const len = Math.floor(rnd() * 41);
    let s = '';
    for (let j = 0; j < len; j++) s += RAND_ALPHA[Math.floor(rnd() * RAND_ALPHA.length)];
    dateInputsRandom.push(s);
  }
}

const typicalData = fixtures.find((f) => f.id === 'typical').data;
const defaultOptions = parseCardOptions({});

function renderDateEntry(input) {
  const streak = JSON.parse(JSON.stringify(typicalData.streak));
  for (const k of ['currentStreakStart', 'currentStreakEnd', 'longestStreakStart', 'longestStreakEnd']) {
    if (input === ABSENT) delete streak[k]; else streak[k] = input;
  }
  const profile = JSON.parse(JSON.stringify(typicalData.profile));
  if (input === ABSENT) delete profile.createdAt; else profile.createdAt = input;
  for (const d of profile.contributionCalendar) {
    if (input === ABSENT) delete d.date; else d.date = input;
  }
  const entry = { in: serInput(input) };
  try {
    entry.su = uid;
    entry.s = md5hex(renderStreakCard(streak, defaultOptions));
    uid++;
  } catch (e) {
    entry.s = `err:${e.constructor.name}`;
  }
  try {
    entry.pu = uid;
    entry.p = md5hex(renderProfileCard(profile, defaultOptions));
    uid++;
  } catch (e) {
    entry.p = `err:${e.constructor.name}`;
  }
  return entry;
}

mkdirSync(`${goldenDir}/meta`, { recursive: true });
const dateExtract = {
  node: NODE_VERSION,
  kind: 'date-extract',
  now: new Date().toISOString().slice(0, 7),
  fixture: 'typical',
  options: 'parseCardOptions({}) — variante « default » du manifeste',
  uidRule: 'su/pu = valeur du compteur global cardIdCounter au moment du rendu streak/profile ; +1 par rendu réussi (createCardWrapper) ; un rendu en échec (err:*) ne consomme rien ; numérotation globale du script',
  injectionRule: 'streak : currentStreakStart/End + longestStreakStart/End = input ; profile : createdAt = input ET contributionCalendar[*].date = input ; {"$absent":true} = champ supprimé (undefined)',
  representative: dateInputsRepresentative.map(renderDateEntry),
  random: dateInputsRandom.map(renderDateEntry),
};
writeFileSync(`${goldenDir}/meta/date-extract.json`, JSON.stringify(dateExtract));
console.log(`r1: ${dateExtract.representative.length} entrées représentatives + ${dateExtract.random.length} aléatoires (dates), uid fini à ${uid - 1}`);

// ================= 9. Oracle fix2 R2 : compteurs et champs texte =================
// Pour chaque carte et chaque champ numérique/texte déclaré dans types.ts :
// valeurs hors/aux bornes ; md5 du SVG legacy ou err:<nom> quand le legacy lève.
const NUM_VALUES = [ABSENT, null, 0, -5, 12.5, 9007199254740993, 1e21, '42', true];
const TEXT_VALUES = [ABSENT, null, '', 0, 'José García', '日本語のテスト', 'Émoji 🚀 ici'];
const NUMERIC_FIELDS = {
  stats: ['totalStars', 'totalForks', 'totalCommits', 'totalPRs', 'totalIssues', 'contributedTo'],
  streak: ['totalContributions', 'currentStreak', 'longestStreak'],
  langs: ['languages[0].percentage', 'languages[0].size'],
  profile: ['contributionsThisYear', 'publicRepos', 'contributionCalendar[*].count'],
};
const TEXT_FIELDS = {
  stats: ['username', 'name'],
  streak: ['username', 'currentStreakStart', 'currentStreakEnd', 'longestStreakStart', 'longestStreakEnd'],
  langs: ['username', 'languages[0].name', 'languages[0].color'],
  profile: ['username', 'name', 'createdAt', 'email', 'contributionCalendar[*].date'],
};

function setPath(root, path, value) {
  const star = path.indexOf('[*]');
  if (star !== -1) {
    const head = path.slice(0, star);
    const rest = path.slice(star + 3).replace(/^\./, '');
    const arr = head === '' ? root : getPath(root, head);
    for (const el of arr) setPath(el, rest, value);
    return;
  }
  const dot = path.indexOf('.');
  if (dot === -1) {
    if (value === ABSENT) delete root[path]; else root[path] = value;
    return;
  }
  const head = path.slice(0, dot);
  const m = head.match(/^(.+?)\[(\d+)\]$/);
  const child = m ? getPath(root, m[1])[Number(m[2])] : root[head];
  setPath(child, path.slice(dot + 1), value);
}
function getPath(root, path) {
  const m = path.match(/^(.+?)\[(\d+)\]$/);
  return m ? getPath(root, m[1])[Number(m[2])] : root[path];
}

const counterEntries = [];
for (const card of CARDS) {
  for (const kind of ['number', 'text']) {
    const fields = kind === 'number' ? NUMERIC_FIELDS[card] : TEXT_FIELDS[card];
    const values = kind === 'number' ? NUM_VALUES : TEXT_VALUES;
    for (const field of fields) {
      for (const value of values) {
        const data = JSON.parse(JSON.stringify(typicalData[card === 'langs' ? 'langs' : card]));
        setPath(data, field, value);
        const entry = { card, kind, field, value: serInput(value) };
        try {
          entry.u = uid;
          entry.md5 = md5hex(RENDER[card](data, defaultOptions));
          uid++;
        } catch (e) {
          entry.err = e.constructor.name;
        }
        counterEntries.push(entry);
      }
    }
  }
}
const countersExtract = {
  node: NODE_VERSION,
  kind: 'counters-extract',
  now: new Date().toISOString().slice(0, 7),
  fixture: 'typical',
  options: 'parseCardOptions({}) — variante « default » du manifeste',
  uidRule: 'u = valeur du compteur global cardIdCounter au moment du rendu ; +1 par rendu réussi ; un rendu en échec (err) ne consomme rien ; numérotation globale du script',
  numericValues: NUM_VALUES.map(serInput),
  textValues: TEXT_VALUES.map(serInput),
  numericFields: NUMERIC_FIELDS,
  textFields: TEXT_FIELDS,
  entries: counterEntries,
};
writeFileSync(`${goldenDir}/meta/counters-extract.json`, JSON.stringify(countersExtract));
console.log(`r2: ${counterEntries.length} entrées compteurs/texte, uid fini à ${uid - 1}`);

// ================= 10. Extension fix3 : fixture edge-fix3 (I3 ∞, M1 -0, M2 ère BC) =================
// Les 53 SVG sont rendus par le PROCESSUS ENFANT (cf. section 10bis : compteur
// legacy frais amené à 459 → uid 459..511, contigus aux 459 entrées précédentes).
// Le parent ne fait que fusionner : il relit les octets écrits, recalcule md5 /
// paramsHash / bytes et pousse les entrées aux indices 459..511. Les méta
// date/counters-extract et les 459 SVG d'origine ne sont pas retouchés.
{
  const { spawnSync } = await import('node:child_process');
  const child = spawnSync(process.execPath, [fileURLToPath(import.meta.url)], {
    env: { ...process.env, GEN_FIX3_CHILD: '1' },
    stdio: 'inherit',
  });
  if (child.status !== 0) {
    throw new Error(`gen fix3 (processus enfant) a échoué : status=${child.status}`);
  }
  for (const card of CARDS) {
    for (const opt of OPTION_SETS) {
      if (opt.langsOnly && card !== 'langs') continue;
      const options = parseCardOptions(opt.query);
      const label = `x-edge-fix3-${opt.label}`;
      const rel = `${card}/edge-fix3/${label}.svg`;
      const svg = readFileSync(`${goldenDir}/${rel}`, 'utf8');
      totalBytes += Buffer.byteLength(svg, 'utf8');
      manifest.entries.push({
        file: rel,
        card,
        fixture: 'edge-fix3',
        label,
        uid: manifest.entries.length, // = index courant : contiguïté uid == index
        md5: md5hex(svg),
        paramsHash: md5hex(JSON.stringify(options)).slice(0, 8),
        options: JSON.parse(JSON.stringify(options)),
        query: opt.query,
      });
    }
  }
}
manifest.generator.fix3 =
  'section 10 : fixture edge-fix3 (I3 totalStars=1e400 -> infini, M1 totalForks=-0.0 -> -0, M2 dates d ere BC sur streak) rendue par un processus enfant (compteur legacy frais amene a 459) — uid 459..511 contigus (uid == index), 459 SVG d origine et metas date/counters-extract inchangés';
manifest.counts = { files: manifest.entries.length, bytes: totalBytes, fixtures: fixtures.length };
writeFileSync(`${goldenDir}/manifest.json`, JSON.stringify(manifest, null, 2));
console.log(
  `fix3 : edge-fix3 rendu (processus enfant), corpus ${manifest.entries.length} SVG, ${totalBytes} octets, uid parent 0..${uid - 1} + enfants 459..511`,
);
console.log('GEN_OK');
