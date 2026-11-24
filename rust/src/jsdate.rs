//! Port du parsing et du formatage de dates du legacy (fix2 R1) :
//! `new Date(x)` de V8 (Node v26.13.1) puis
//! `Intl.DateTimeFormat('en', { month:'short', day:'numeric', year:'numeric' }).format(d)`
//! et `d.toLocaleString('en', { month:'short' })`.
//!
//! Sémantique REJOUÉE par l'oracle `tests/golden/meta/date-extract.json`
//! (87 entrées représentatives + 2 000 pseudo-aléatoires, node:26.13.1-alpine,
//! `--network none`) — l'oracle prime sur toute hypothèse :
//! - ISO : année 4 chiffres (ou ±6), mois 1..=12, jour 1..=31 (REJET de 00 et 32),
//!   heure 0..=24, min/sec 0..=59 (rejet de 60) ; jour 29-31 sur mois court →
//!   report (« 2026-02-30 » → 2 mars) ; date seule ou sans fuseau → UTC ;
//!   violation de forme OU de plage → Invalid (pas de repli).
//! - Repli (parseur legacy) : noms de mois (≥ 3 lettres, insensible à la casse),
//!   jours/années/mois numériques, heure `H:MM(:SS(.f)?)?` + am/pm, fuseaux nommés
//!   (GMT/UTC/UT/Z/EST/…) et `±HHMM`/`±HH:MM` ; mots inconnus ignorés ; nombre nu
//!   1..=12 → mois (jour 1, année 2001), sinon année (règle deux chiffres :
//!   < 50 → +2000, sinon +1900) ; défauts : année 2001, jour 1 ; ≥ 4 nombres → Invalid.

use crate::intl::MONTHS_EN;
use crate::jsval::JsVal;

/// Limite V8 : `TimeClip` → NaN au-delà de ±8.64e15 ms (±100 000 000 jours).
const MS_RANGE: f64 = 8.64e15;

/// Résultat de `new Date(x)` : ms depuis l'époque (peut être négatif/fractionnaire)
/// ou date invalide (`NaN`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsDate {
    Valid(f64),
    Invalid,
}

/// `new Date(x)` à un argument : `Number(x)` pour tout sauf les strings.
pub fn new_date(v: &JsVal) -> JsDate {
    match v {
        // ToNumber(undefined) = NaN → Invalid ; ToNumber(null) = 0 → epoch.
        JsVal::Undefined => JsDate::Invalid,
        JsVal::Null => JsDate::Valid(0.0),
        JsVal::Bool(b) => JsDate::Valid(if *b { 1.0 } else { 0.0 }),
        JsVal::Number(n) if n.is_nan() => JsDate::Invalid,
        JsVal::Number(n) => JsDate::Valid(*n),
        JsVal::Str(s) => match parse_string(s) {
            Some(ms) => JsDate::Valid(ms),
            None => JsDate::Invalid,
        },
        // Objet/tableau JSON : ToPrimitive → NaN (hors oracles, jamais rendu).
        JsVal::Other => JsDate::Invalid,
    }
}

/// `new Date(s)` pour une string : `Some(ms)` ou `None` (Invalid Date).
pub fn parse_string(s: &str) -> Option<f64> {
    let t = s.trim_matches(|c: char| {
        matches!(
            c,
            ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{a0}' | '\u{feff}'
        )
    });
    match try_iso(t) {
        IsoVerdict::Ms(ms) => Some(ms),
        IsoVerdict::Invalid => None, // forme ISO mais plage hors bornes : pas de repli
        IsoVerdict::NotIso => try_fallback(t),
    }
}

fn clip(ms: f64) -> Option<f64> {
    if ms.is_finite() && ms.abs() <= MS_RANGE {
        Some(ms)
    } else {
        None
    }
}

// ------------------------------------------------------------------ parseur ISO

enum IsoVerdict {
    Ms(f64),
    /// La string n'a pas la forme ISO → parseur legacy.
    NotIso,
    /// Forme ISO mais composante hors plage → Invalid, sans repli (oracle).
    Invalid,
}

fn read_exact(b: &[u8], i: &mut usize, n: usize) -> Option<i64> {
    if *i + n > b.len() {
        return None;
    }
    let mut v: i64 = 0;
    for k in 0..n {
        if !b[*i + k].is_ascii_digit() {
            return None;
        }
        v = v * 10 + i64::from(b[*i + k] - b'0');
    }
    *i += n;
    Some(v)
}

fn try_iso(s: &str) -> IsoVerdict {
    let b = s.as_bytes();
    let mut i = 0usize;
    let year = if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        let neg = b[i] == b'-';
        i += 1;
        match read_exact(b, &mut i, 6) {
            Some(y) if neg => -y,
            Some(y) => y,
            None => return IsoVerdict::NotIso,
        }
    } else {
        match read_exact(b, &mut i, 4) {
            Some(y) => y,
            None => return IsoVerdict::NotIso,
        }
    };
    let (mut month, mut day) = (1i64, 1i64);
    if i < b.len() && b[i] == b'-' {
        i += 1;
        month = match read_exact(b, &mut i, 2) {
            Some(m) => m,
            None => return IsoVerdict::NotIso,
        };
        if i < b.len() && b[i] == b'-' {
            i += 1;
            day = match read_exact(b, &mut i, 2) {
                Some(d) => d,
                None => return IsoVerdict::NotIso,
            };
        }
    }
    // Plages ISO : rejette 00/13 (mois) et 00/32 (jour) — l'oracle V8 renvoie alors
    // Invalid (pas de report mensuel).
    if !(1..=12).contains(&month) {
        return IsoVerdict::Invalid;
    }
    if !(1..=31).contains(&day) {
        return IsoVerdict::Invalid;
    }
    let (mut hh, mut mm, mut ss, mut msf) = (0i64, 0i64, 0i64, 0i64);
    let mut tz_min: Option<i64> = None;
    if i < b.len() && matches!(b[i], b'T' | b't' | b' ') {
        i += 1;
        hh = match read_exact(b, &mut i, 2) {
            Some(h) => h,
            None => return IsoVerdict::NotIso,
        };
        if i >= b.len() || b[i] != b':' {
            return IsoVerdict::NotIso;
        }
        i += 1;
        mm = match read_exact(b, &mut i, 2) {
            Some(m) => m,
            None => return IsoVerdict::NotIso,
        };
        if i < b.len() && b[i] == b':' {
            i += 1;
            ss = match read_exact(b, &mut i, 2) {
                Some(sec) => sec,
                None => return IsoVerdict::NotIso,
            };
            if i < b.len() && b[i] == b'.' {
                i += 1;
                let start = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                if i == start {
                    return IsoVerdict::NotIso;
                }
                // ≥ 4 chiffres : troncature aux 3 premiers (oracle .9999 → 999)
                let n = (i - start).min(3);
                let mut v: i64 = s[start..start + n].parse().expect("chiffres");
                for _ in n..3 {
                    v *= 10;
                }
                msf = v;
            }
        }
        // Plages ISO de l'heure (oracle : T24:00:00Z ok, T12:60 → Invalid).
        if !(0..=24).contains(&hh) || !(0..=59).contains(&mm) || !(0..=59).contains(&ss) {
            return IsoVerdict::Invalid;
        }
        if i < b.len() {
            match b[i] {
                b'Z' => {
                    i += 1;
                    tz_min = Some(0);
                }
                b'+' | b'-' => {
                    let sign = if b[i] == b'-' { -1 } else { 1 };
                    i += 1;
                    let oh = match read_exact(b, &mut i, 2) {
                        Some(h) => h,
                        None => return IsoVerdict::NotIso,
                    };
                    let om = if i < b.len() && b[i] == b':' {
                        i += 1;
                        match read_exact(b, &mut i, 2) {
                            Some(m) => m,
                            None => return IsoVerdict::NotIso,
                        }
                    } else if i + 1 < b.len() && b[i].is_ascii_digit() && b[i + 1].is_ascii_digit()
                    {
                        let m = i64::from(b[i] - b'0') * 10 + i64::from(b[i + 1] - b'0');
                        i += 2;
                        m
                    } else {
                        0
                    };
                    tz_min = Some(sign * (oh * 60 + om));
                }
                _ => return IsoVerdict::NotIso,
            }
        }
    }
    if i != b.len() {
        return IsoVerdict::NotIso;
    }
    // Report du débordement de jour (Feb 30 → Mar 2) : MakeDay normalise.
    let days = days_from_civil(year, month - 1, day);
    let ms = days as f64 * 86_400_000.0 + ((hh * 3600 + mm * 60 + ss) as f64) * 1000.0 + msf as f64
        - tz_min.unwrap_or(0) as f64 * 60_000.0;
    match clip(ms) {
        Some(ms) => IsoVerdict::Ms(ms),
        None => IsoVerdict::Invalid,
    }
}

// -------------------------------------------------------------- parseur legacy

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(i64),
    Month(usize),
    Time { h: i64, mi: i64, s: i64, ms: i64 },
    AmPm(bool),
    Tz(i64),
}

const MONTH_PREFIXES: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

fn try_fallback(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0usize;
    let mut toks: Vec<Tok> = Vec::new();
    let mut lone_sign = false; // signe orphelin en attente d'un composant reconnu
    while i < b.len() {
        let c = b[i];
        // NB : 'T' n'est PAS un délimiteur ici (oracle : « T5 », « 5T » → Invalid) ;
        // les strings ISO qui en ont besoin sont déjà traitées par try_iso.
        if c.is_ascii_whitespace() || matches!(c, b',' | b'/' | b'.' | b'(' | b')') {
            i += 1;
            continue;
        }
        if c == b'+' || c == b'-' {
            // Offset ±HH[:MM] / ±HHMM : uniquement après une heure (« +0200 » RFC) ;
            // sinon signe + chiffres = NOMBRE SIGNÉ dont on ignore le signe — toléré
            // seulement attaché (ou en tout début) : oracle « 5+6 », « 3432+11 »,
            // « 5 -6 », « 5 -06 », « 5 + » → Invalid ; « -6 » → juin, « +66 » → 1966,
            // « 5-06 » → 6 mai 2001, « x-6 » → juin, « 2024-01-05z » → 5 janvier.
            let attached = i > 0 && (b[i - 1].is_ascii_alphanumeric());
            let two_digits =
                i + 2 < b.len() && b[i + 1].is_ascii_digit() && b[i + 2].is_ascii_digit();
            let after_time = toks
                .last()
                .is_some_and(|t| matches!(t, Tok::Time { .. } | Tok::Tz(_)));
            if two_digits && after_time {
                let hh = i64::from(b[i + 1] - b'0') * 10 + i64::from(b[i + 2] - b'0');
                let mut j = i + 3;
                let mm = if j + 2 < b.len() && b[j] == b':' {
                    let v = i64::from(b[j + 1] - b'0') * 10 + i64::from(b[j + 2] - b'0');
                    j += 3;
                    v
                } else if j + 1 < b.len() && b[j].is_ascii_digit() && b[j + 1].is_ascii_digit() {
                    let v = i64::from(b[j] - b'0') * 10 + i64::from(b[j + 1] - b'0');
                    j += 2;
                    v
                } else {
                    0
                };
                let min = hh * 60 + mm;
                lone_sign = false;
                toks.push(Tok::Tz(if c == b'-' { -min } else { min }));
                i = j;
                continue;
            }
            let next_digit = i + 1 < b.len() && b[i + 1].is_ascii_digit();
            if next_digit {
                if c == b'+' && !toks.is_empty() {
                    return None; // « 5+6 », « 3432+11 », « 5 +06 » → Invalid
                }
                if c == b'-' && !attached && !toks.is_empty() {
                    return None; // « 5 -6 », « 5 -06 », « 5 -0600 » → Invalid
                }
                i += 1; // signe absorbé : la valeur absolue sera lue comme nombre
                continue;
            }
            // signe orphelin : détaché → invalide sauf composant reconnu ensuite
            // ; attaché → inoffensif (« 6- », « 6-z » → juin ; « 5 + » → Invalid)
            lone_sign = !attached;
            i += 1;
            continue;
        }
        if c == b':' {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            let run = &s[start..i];
            // Heure `H:MM(:SS(.fff)?)?` — H ≤ 2 chiffres ; l'oracle exige MM sur
            // 2 chiffres exactement (« 6:2 », « 6:m » → Invalid) et interdit toute
            // lettre attachée à la suite (« 12:00x » → Invalid).
            if run.len() <= 2
                && i < b.len()
                && b[i] == b':'
                && i + 2 < b.len()
                && b[i + 1].is_ascii_digit()
                && b[i + 2].is_ascii_digit()
            {
                let h: i64 = run.parse().expect("≤ 2 chiffres");
                i += 1;
                let mi = i64::from(b[i] - b'0') * 10 + i64::from(b[i + 1] - b'0');
                i += 2;
                let (mut sec, mut msf) = (0i64, 0i64);
                if i < b.len() && b[i] == b':' && i + 2 < b.len() {
                    sec = i64::from(b[i + 1] - b'0') * 10 + i64::from(b[i + 2] - b'0');
                    i += 3; // i est sur ':' : deux chiffres ensuite
                    if i < b.len() && b[i] == b'.' {
                        i += 1;
                        let fs = i;
                        while i < b.len() && b[i].is_ascii_digit() {
                            i += 1;
                        }
                        if i == fs {
                            return None; // « 6:00:00.z » → Invalid
                        }
                        let n = (i - fs).min(3);
                        let mut v: i64 = s[fs..fs + n].parse().expect("chiffres");
                        for _ in n..3 {
                            v *= 10;
                        }
                        msf = v;
                    }
                }
                if i < b.len() && (b[i].is_ascii_alphabetic()) {
                    return None; // lettre attachée au temps → poison
                }
                lone_sign = false;
                toks.push(Tok::Time {
                    h,
                    mi,
                    s: sec,
                    ms: msf,
                });
                continue;
            }
            if run.len() <= 2 && i < b.len() && b[i] == b':' {
                return None; // forme temporelle inaboutie : « 6:2 », « 1:2z » → Invalid
            }
            if i < b.len() && b[i].is_ascii_alphabetic() {
                // run chiffres+lettres : la suite doit être un suffixe reconnu
                // (mois ou fuseau) — sinon poison (« 5j », « 3My », « 12x » → Invalid)
                let ls = i;
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let w = s[ls..i].to_ascii_lowercase();
                // I1 (fix3) : run de chiffres NON borné — V8 rejette toute valeur
                // > 2^53 ; au-delà de i64 → Invalid, jamais de panic
                // (oracle : « 99999999999999999999june » → Invalid).
                let n: i64 = match run.parse() {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                if w.len() >= 3 {
                    if let Some(mi) = MONTH_PREFIXES.iter().position(|p| w.starts_with(p)) {
                        lone_sign = false;
                        toks.push(Tok::Num(n));
                        toks.push(Tok::Month(mi));
                        continue;
                    }
                }
                if let Some(off) = tz_name_offset(&w) {
                    lone_sign = false;
                    toks.push(Tok::Num(n));
                    toks.push(Tok::Tz(off));
                    continue;
                }
                return None;
            }
            lone_sign = false;
            // I1 (fix3) : cf. ci-dessus — run de chiffres non borné → checked,
            // débordement i64 → Invalid (jamais de panic).
            let n: i64 = match run.parse() {
                Ok(v) => v,
                Err(_) => return None,
            };
            toks.push(Tok::Num(n));
            continue;
        }
        if c.is_ascii_alphabetic() {
            let start = i;
            while i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
            let w = s[start..i].to_ascii_lowercase();
            // lettres + chiffres attachés : uniquement un nom de mois (« jun6 »,
            // « jan6z » → mois puis jour) — sinon poison (« x2 », « m6 », « T5 »)
            let attached_digit = i < b.len() && b[i].is_ascii_digit();
            if attached_digit {
                let mi = if w.len() >= 3 {
                    MONTH_PREFIXES.iter().position(|p| w.starts_with(p))
                } else {
                    None
                };
                lone_sign = false;
                toks.push(Tok::Month(mi?));
                continue; // les chiffres attachés sont relus par la branche chiffres
            }
            let tok = match w.as_str() {
                "am" => Some(Tok::AmPm(false)),
                "pm" => Some(Tok::AmPm(true)),
                _ => tz_name_offset(&w).map(Tok::Tz).or_else(|| {
                    if w.len() >= 3 {
                        MONTH_PREFIXES
                            .iter()
                            .position(|p| w.starts_with(p))
                            .map(Tok::Month)
                    } else {
                        None // mot trop court : ignoré s'il précède tout composant
                    }
                }),
            };
            match tok {
                Some(t) => {
                    lone_sign = false;
                    toks.push(t);
                }
                None => {
                    // mot inconnu : ignoré SEULEMENT avant tout composant reconnu
                    // (oracle : « 5 j » → Invalid, « x-6 » → juin, « mCMDR…6z » → juin)
                    if !toks.is_empty() {
                        return None;
                    }
                }
            }
            continue;
        }
        i += 1;
    }
    if lone_sign {
        return None;
    }
    assemble(toks)
}

/// Offset fuseau d'un nom (minutes, UTC−locale ; EST = −300 etc.).
fn tz_name_offset(w: &str) -> Option<i64> {
    match w {
        "gmt" | "utc" | "ut" | "z" => Some(0),
        "est" => Some(-300),
        "edt" => Some(-240),
        "cst" => Some(-360),
        "cdt" => Some(-300),
        "mst" => Some(-420),
        "mdt" => Some(-360),
        "pst" => Some(-480),
        "pdt" => Some(-420),
        _ => None,
    }
}

/// Assemble année/mois/jour + heure + fuseau en ms (défauts V8 : année 2001, jour 1).
fn assemble(toks: Vec<Tok>) -> Option<f64> {
    let mut month: Option<usize> = None;
    let mut time: Option<(i64, i64, i64, i64)> = None;
    let mut ampm: Option<bool> = None;
    let mut tz: Option<i64> = None;
    let mut nums: Vec<i64> = Vec::new();
    for t in toks {
        match t {
            Tok::Month(m) => {
                if month.is_some() {
                    return None;
                }
                month = Some(m);
            }
            Tok::Time { h, mi, s, ms } => {
                if time.is_some() {
                    return None;
                }
                time = Some((h, mi, s, ms));
            }
            Tok::AmPm(p) => ampm = Some(p),
            Tok::Tz(off) => tz = Some(off),
            Tok::Num(n) => nums.push(n),
        }
    }
    if ampm.is_some() && time.is_none() {
        return None; // « 5am », « 5 am » → Invalid (oracle)
    }
    let (year, m_idx, day) = assemble_ymd(month, &nums)?;
    let (mut h, mi, sec, msf) = time.unwrap_or((0, 0, 0, 0));
    if let Some(pm) = ampm {
        if pm && h < 12 {
            h += 12;
        }
        if !pm && h == 12 {
            h = 0;
        }
    }
    let days = days_from_civil(year, i64::try_from(m_idx).expect("mois 0..=11"), day);
    let ms = days as f64 * 86_400_000.0 + ((h * 3600 + mi * 60 + sec) as f64) * 1000.0 + msf as f64
        - tz.unwrap_or(0) as f64 * 60_000.0;
    clip(ms)
}

/// Année/mois(0-based)/jour depuis le mois nommé éventuel et les nombres restants.
fn assemble_ymd(month: Option<usize>, nums: &[i64]) -> Option<(i64, usize, i64)> {
    let mut raw_year: Option<i64> = None;
    let (m_idx, day);
    if let Some(m) = month {
        if nums.len() > 2 {
            return None;
        }
        // premier nombre ≤ 31 → jour, sinon (unique) → année ; défauts : jour 1
        day = match nums.first() {
            Some(&n0) if n0 <= 31 => n0,
            Some(&n0) => {
                if nums.len() > 1 {
                    return None; // deux candidats « année »
                }
                raw_year = Some(n0);
                1
            }
            None => 1,
        };
        if raw_year.is_none() && nums.len() > 1 {
            raw_year = Some(nums[1]);
        }
        m_idx = m;
    } else {
        match nums.len() {
            1 => {
                let n = nums[0];
                if (1..=12).contains(&n) {
                    m_idx = n as usize - 1;
                    day = 1;
                    raw_year = None;
                } else {
                    m_idx = 0;
                    day = 1;
                    raw_year = Some(n);
                }
            }
            2 => {
                // oracle : deux nombres = mois/jour (année par défaut 2001) ou
                // année en PREMIER (« 2024-6z » → 1er juin 2024) ; jamais une
                // année en deuxième position (« 5 66 », « 5 0666 » → Invalid)
                let (a, b) = (nums[0], nums[1]);
                if (1..=12).contains(&a) && (1..=31).contains(&b) {
                    m_idx = a as usize - 1;
                    day = b;
                    raw_year = None;
                } else if a >= 100 && (1..=12).contains(&b) {
                    raw_year = Some(a);
                    m_idx = b as usize - 1;
                    day = 1;
                } else if (1..=31).contains(&a) && (1..=12).contains(&b) {
                    day = a;
                    m_idx = b as usize - 1;
                    raw_year = None;
                } else {
                    return None;
                }
            }
            3 => {
                let (a, bb, c) = (nums[0], nums[1], nums[2]);
                let (y, mut m, mut d) = if a >= 100 { (a, bb, c) } else { (c, a, bb) };
                if m > 12 && d <= 12 {
                    std::mem::swap(&mut m, &mut d);
                }
                if !(1..=12).contains(&m) {
                    return None;
                }
                raw_year = Some(y);
                m_idx = m as usize - 1;
                day = d;
            }
            _ => return None, // 0 nombre sans nom de mois, ou ≥ 4 nombres
        }
    }
    let mut year = raw_year.unwrap_or(2001);
    if (0..=99).contains(&year) {
        year = if year < 50 { year + 2000 } else { year + 1900 };
    }
    // I2 (fix3) : fenêtre TimeClip ±8,64e15 ms ≈ ±275 760 ans — au-delà le
    // legacy rendrait Invalid de toute façon (oracle : « 9223372036854775807 »,
    // « 100000000000000000 » → Invalid). On rejette AVANT `days_from_civil`,
    // dont `era * 146_097` déborde d'i64 dès |année| ≳ 2,5e16 (panic en debug,
    // wrap silencieux en release → date erronée). Valable debug ET release :
    // aucun calcul entier n'a lieu sur une année hors fenêtre.
    if year.abs() > 275_760 {
        return None;
    }
    Some((year, m_idx, day))
}

// ------------------------------------------------------------------ calendrier

/// Jour epoch ← (année, mois 0-based quelconque, jour quelconque) : MakeDay reporte
/// les débordements (Feb 30 → Mar 2, mois 12 → janvier N+1).
fn days_from_civil(y: i64, m_idx: i64, d: i64) -> i64 {
    let y2 = y + m_idx.div_euclid(12);
    let m = m_idx.rem_euclid(12); // 0..=11
    let y3 = y2 - if m < 2 { 1 } else { 0 }; // ère « mars = mois 0 »
    let era = y3.div_euclid(400);
    let yoe = y3 - era * 400;
    let mp = if m < 2 { m + 10 } else { m - 2 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Jour epoch → (année, mois 1-based, jour) — `getFullYear/getMonth/getDate`.
pub fn civil_from_days_full(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

// ------------------------------------------------------------------- formatage

/// `Intl.DateTimeFormat('en', { month:'short', day:'numeric', year:'numeric' }).format(d)`.
pub fn format_date_en(ms: f64) -> String {
    let days = (ms / 86_400_000.0).floor() as i64;
    let (y, m, d) = civil_from_days_full(days);
    // M2 (fix3) : ICU affiche l'année d'ÈRE — année astronomique ≤ 0 → 1 − y
    // (oracle : « -000001-01-01 » → « Jan 1, 2 », « 0000-01-01 » → « Jan 1, 1 »,
    // « -002026-03-02 » → « Mar 2, 2027 »).
    let year_era = if y <= 0 { 1 - y } else { y };
    format!(
        "{} {}, {year_era}",
        MONTHS_EN[usize::try_from(m).expect("mois 1..=12") - 1],
        d
    )
}

/// `d.toLocaleString('en', { month: 'short' })` (ticks X du profile).
pub fn format_month_en(ms: f64) -> String {
    let days = (ms / 86_400_000.0).floor() as i64;
    let (_, m, _) = civil_from_days_full(days);
    MONTHS_EN[usize::try_from(m).expect("mois 1..=12") - 1].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Option<String> {
        parse_string(s).map(format_date_en)
    }

    #[test]
    fn iso_ancres_oracle() {
        assert_eq!(d("2024-01-05").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("2024").as_deref(), Some("Jan 1, 2024"));
        assert_eq!(d("2024-06").as_deref(), Some("Jun 1, 2024"));
        assert_eq!(
            d("2024-01-05T12:00:00.9999Z").as_deref(),
            Some("Jan 5, 2024")
        );
        assert_eq!(
            d("2024-01-05T00:00:00+23:59").as_deref(),
            Some("Jan 4, 2024")
        );
        assert_eq!(d("2026-02-30").as_deref(), Some("Mar 2, 2026")); // report jour
        assert_eq!(d("2024-02-30").as_deref(), Some("Mar 1, 2024"));
        assert_eq!(d("2024-04-31").as_deref(), Some("May 1, 2024"));
        assert_eq!(d("2024-01-05T24:00:00Z").as_deref(), Some("Jan 6, 2024"));
        assert_eq!(d("0024-05-06").as_deref(), Some("May 6, 24"));
        assert_eq!(d("0099-12-31").as_deref(), Some("Dec 31, 99"));
        assert_eq!(d("+002024-01-05").as_deref(), Some("Jan 5, 2024"));
        // Rejets ISO (oracle : pas de repli → Invalid)
        for bad in [
            "2024-01-00",
            "2024-01-32",
            "2024-00-15",
            "2024-13-01",
            "2024-99-99",
            "2024-01-05T12:60:00Z",
        ] {
            assert_eq!(parse_string(bad), None, "{bad} doit être invalide");
        }
    }

    #[test]
    fn repli_ancres_oracle() {
        assert_eq!(d("January 5, 2024").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("jan 5 2024").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("5 January 2024").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("February 30, 2026").as_deref(), Some("Mar 2, 2026"));
        assert_eq!(d("Feb 29, 2025").as_deref(), Some("Mar 1, 2025")); // 2025 non bissextile
        assert_eq!(d("January 2024").as_deref(), Some("Jan 1, 2024"));
        assert_eq!(d("Jan 5").as_deref(), Some("Jan 5, 2001"));
        assert_eq!(d("May 2024").as_deref(), Some("May 1, 2024"));
        assert_eq!(
            d("Mon, 05 Jan 2024 12:00:00 GMT").as_deref(),
            Some("Jan 5, 2024")
        );
        assert_eq!(
            d("Sun, 03 Mar 2024 23:59:59 PST").as_deref(),
            Some("Mar 4, 2024")
        );
        assert_eq!(d("2026-1-5").as_deref(), Some("Jan 5, 2026"));
        assert_eq!(d("01/05/2024").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("2024/01/05").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("12/25/2023").as_deref(), Some("Dec 25, 2023"));
        assert_eq!(d("2/3/24").as_deref(), Some("Feb 3, 2024"));
        assert_eq!(d("1/5").as_deref(), Some("Jan 5, 2001"));
        assert_eq!(d("10000").as_deref(), Some("Jan 1, 10000"));
        assert_eq!(d("2024-01-05z").as_deref(), Some("Jan 5, 2024"));
        assert_eq!(d("  2024-01-05  ").as_deref(), Some("Jan 5, 2024"));
        // nombres nus (liste aléatoire de l'oracle)
        assert_eq!(d("3").as_deref(), Some("Mar 1, 2001"));
        assert_eq!(d("0").as_deref(), Some("Jan 1, 2000"));
        assert_eq!(d("32").as_deref(), Some("Jan 1, 2032"));
        assert_eq!(d("84").as_deref(), Some("Jan 1, 1984"));
        assert_eq!(d("mCMDRnPwtkTGzV.-cai:6z").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("aXFzu+1").as_deref(), Some("Jan 1, 2001"));
        assert_eq!(d("+-4").as_deref(), Some("Apr 1, 2001"));
        assert_eq!(
            d(" EoPMmhlhnSXWPnbsaILJTfY,7").as_deref(),
            Some("Jul 1, 2001")
        );
        // micro-cas du tokenizer (sonde Node v26.13.1, fix2)
        assert_eq!(d("5z").as_deref(), Some("May 1, 2001"));
        assert_eq!(d("6,z").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("6.z").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("6-z").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("+6z").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("x-6").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("-6").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("6-").as_deref(), Some("Jun 1, 2001"));
        assert_eq!(d("666").as_deref(), Some("Jan 1, 666"));
        assert_eq!(d("66666").as_deref(), Some("Jan 1, 66666"));
        assert_eq!(d("6jun").as_deref(), Some("Jun 6, 2001"));
        assert_eq!(d("jun6").as_deref(), Some("Jun 6, 2001"));
        assert_eq!(d("6 jun").as_deref(), Some("Jun 6, 2001"));
        assert_eq!(d("jan6z").as_deref(), Some("Jan 6, 2001"));
        assert_eq!(d("5.").as_deref(), Some("May 1, 2001"));
        assert_eq!(d(".5").as_deref(), Some("May 1, 2001"));
        assert_eq!(d("5/").as_deref(), Some("May 1, 2001"));
        assert_eq!(d("Jan 5,").as_deref(), Some("Jan 5, 2001"));
        assert_eq!(d(",5 Jan").as_deref(), Some("Jan 5, 2001"));
        assert_eq!(d("Jan 5 EST").as_deref(), Some("Jan 5, 2001"));
        assert_eq!(d("Jan 5 (Est)").as_deref(), Some("Jan 5, 2001"));
        // refus supplémentaires (sonde)
        for bad in [
            "5j",
            "5 j",
            "6k",
            "3My",
            "2G",
            "x2",
            "L2g",
            "12x",
            "12:x",
            "12:00x",
            "Jan 5x",
            "Jan 5 x",
            "1:2z",
            "6:m",
            "6:m5",
            "m6",
            "m6z",
            "6 m",
            "6:2",
            "2:6",
            "5T",
            "T5",
            "5am",
            "5 am",
            "ampm",
            "666666",
            "6666666",
            "6:00:00.z",
        ] {
            assert_eq!(parse_string(bad), None, "{bad} doit être invalide");
        }
    }

    #[test]
    fn non_chaines_et_formatage_mois() {
        assert_eq!(new_date(&JsVal::Null), JsDate::Valid(0.0));
        assert_eq!(new_date(&JsVal::Undefined), JsDate::Invalid);
        assert_eq!(new_date(&JsVal::Bool(true)), JsDate::Valid(1.0));
        assert_eq!(
            new_date(&JsVal::Number(-86_400_000.0)),
            JsDate::Valid(-86_400_000.0)
        );
        assert_eq!(format_date_en(-86_400_000.0), "Dec 31, 1969");
        assert_eq!(format_date_en(12.5), "Jan 1, 1970");
        assert_eq!(format_month_en(0.0), "Jan");
        assert_eq!(format_month_en(951782400000.0), "Feb");
    }
}
