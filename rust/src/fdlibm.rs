//! Port de `sin`/`cos` **tels qu'embarqués dans V8** (Node 26) — le port fdlibm de
//! `chromium/third_party/fdlibm/ieee754.cc` (repris dans `v8/src/base/numbers/ieee754.cc`).
//!
//! Motivation mesurée (P0) : la libm musl appelée par `f64::cos/sin` diverge de Node
//! 26.13.1 sur 86/4181 angles du sweep (±1 ULP, toujours sur `cos`) ; ce port reproduit
//! Node au bit près sur les 4181 angles du corpus (test `libm_cos_sin_parity_with_node`).
//! Trois pièges corrigés par rapport à un « fdlibm de mémoire » : le raffinement `qx`
//! de `__kernel_cos` (seuils 0.3 / 0.78125, terme `z*r - x*y`, C4 ≠ fdlibm originel),
//! le `iy=0` du wrapper `Sin` pour |x| ≤ π/4, et les itérations 2/3 de `rem_pio2`
//! sur les multiples (quasi) exacts de π/2.
//!
//! Périmètre assumé : réduction « large path » non portée (|x| ≥ 2^19·π/2 ≈ 405 000 →
//! panic debug) — aucun angle réel des cartes n'approche cette borne (donuts ≤ 2π+gaps).
//! NaN/±∞/0 suivent les wrappers V8 (`sin(NaN)=NaN`, `sin(±∞)=NaN`, `sin(±0)=±0`).

// Constantes exactes en bits (les décimaux du port V8 ; f64::from_bits évite tout
// souci d'arrondi littéral). S4/S5/S6/C4 diffèrent du fdlibm originel — c'est voulu,
// le fichier V8 fait foi.
const INV_PIO2: f64 = f64::from_bits(0x3FE4_5F30_6DC9_C883);
const PIO2_1: f64 = f64::from_bits(0x3FF9_21FB_5440_0000);
const PIO2_1T: f64 = f64::from_bits(0x3DD0_B461_1A62_6331);
const PIO2_2: f64 = f64::from_bits(0x3DD0_B461_1A60_0000);
const PIO2_2T: f64 = f64::from_bits(0x3BA3_198A_2E03_7073);
const PIO2_3: f64 = f64::from_bits(0x3BA3_198A_2E00_0000);
const PIO2_3T: f64 = f64::from_bits(0x397B_839A_2520_49C1);

/// High words des multiples impairs de π/2 (npio2_hw) — déclencheur des itérations
/// de réduction supplémentaires.
const NPIO2_HW: [u32; 32] = [
    0x3FF9_21FB,
    0x4009_21FB,
    0x4012_D97C,
    0x4019_21FB,
    0x401F_6A7A,
    0x4022_D97C,
    0x4025_FDBB,
    0x4029_21FB,
    0x402C_463A,
    0x402F_6A7A,
    0x4031_475C,
    0x4032_D97C,
    0x4034_6B9C,
    0x4035_FDBB,
    0x4037_8FDB,
    0x4039_21FB,
    0x403A_B41B,
    0x403C_463A,
    0x403D_D85A,
    0x403F_6A7A,
    0x4040_7E4C,
    0x4041_475C,
    0x4042_106C,
    0x4042_D97C,
    0x4043_A28C,
    0x4044_6B9C,
    0x4045_34AC,
    0x4045_FDBB,
    0x4046_C6CB,
    0x4047_8FDB,
    0x4048_58EB,
    0x4049_21FB,
];

const S1: f64 = f64::from_bits(0xBFC5_5555_5555_5549);
const S2: f64 = f64::from_bits(0x3F81_1111_1110_F8A6);
const S3: f64 = f64::from_bits(0xBF2A_01A0_19C1_61D5);
const S4: f64 = f64::from_bits(0x3EC7_1DE3_57B1_FE7D);
const S5: f64 = f64::from_bits(0xBE5A_E5E6_8A2B_9CEB);
const S6: f64 = f64::from_bits(0x3DE5_D93A_5ACF_D57C);

const C1: f64 = f64::from_bits(0x3FA5_5555_5555_554C);
const C2: f64 = f64::from_bits(0xBF56_C16C_16C1_5177);
const C3: f64 = f64::from_bits(0x3EFA_01A0_19CB_1590);
const C4: f64 = f64::from_bits(0xBE92_7E4F_809C_52AD);
const C5: f64 = f64::from_bits(0x3E21_EE9E_BDB4_B1C4);
const C6: f64 = f64::from_bits(0xBDA8_FAE9_BE88_38D4);

fn high_word(x: f64) -> u32 {
    ((x.to_bits() >> 32) & 0x7FFF_FFFF) as u32
}

/// `__ieee754_rem_pio2` : x = n·π/2 + y0 + y1, avec |y| ≤ ~π/4.
/// Retourne (n, y0, y1, sans_reduction) — `sans_reduction` vaut vrai quand |x| ≤ π/4
/// (le wrapper V8 passe alors iy=0 aux kernels).
fn rem_pio2(x: f64) -> (i32, f64, f64, bool) {
    let ix = high_word(x);
    if ix <= 0x3FE9_21FB {
        // |x| ~<= pi/4 : pas de réduction
        return (0, x, 0.0, true);
    }
    if ix < 0x4002_D97C {
        // |x| < 3pi/4 : cas spécial n = ±1
        if x > 0.0 {
            let mut z = x - PIO2_1;
            let (y0, y1) = if ix != 0x3FF9_21FB {
                let a = z - PIO2_1T;
                (a, (z - a) - PIO2_1T)
            } else {
                // près de pi/2 : pi en 33+33+53 bits
                z -= PIO2_2;
                let a = z - PIO2_2T;
                (a, (z - a) - PIO2_2T)
            };
            return (1, y0, y1, false);
        }
        let mut z = x + PIO2_1;
        let (y0, y1) = if ix != 0x3FF9_21FB {
            let a = z + PIO2_1T;
            (a, (z - a) + PIO2_1T)
        } else {
            z += PIO2_2;
            let a = z + PIO2_2T;
            (a, (z - a) + PIO2_2T)
        };
        return (-1, y0, y1, false);
    }
    // Large : |x| > 2^19·π/2 → réduction exacte par entiers (Payne–Hanek borné)
    if ix > 0x4139_21FB {
        let (n, y0, y1) = rem_pio2_large(x);
        return (n, y0, y1, false);
    }
    let t = x.abs();
    let n = (t * INV_PIO2 + 0.5) as i32;
    let f_n = f64::from(n);
    let mut r = t - f_n * PIO2_1;
    let mut w = f_n * PIO2_1T; // 1re passe, bonne à 85 bits
    let mut y0;
    if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
        y0 = r - w; // vérification rapide : pas d'annulation massive
    } else {
        // 2e (et 3e) itération, bonne à 118 (151) bits — multiples quasi exacts de pi/2
        let j = ix >> 20;
        y0 = r - w;
        let mut i = j - ((high_word(y0) >> 20) & 0x7FF);
        if i > 16 {
            let t2 = r;
            w = f_n * PIO2_2;
            r = t2 - w;
            w = f_n * PIO2_2T - ((t2 - r) - w);
            y0 = r - w;
            i = j - ((high_word(y0) >> 20) & 0x7FF);
            if i > 49 {
                let t3 = r;
                w = f_n * PIO2_3;
                r = t3 - w;
                w = f_n * PIO2_3T - ((t3 - r) - w);
                y0 = r - w;
            }
        }
    }
    let y1 = (r - y0) - w;
    if x < 0.0 {
        (-n, -y0, -y1, false)
    } else {
        (n, y0, y1, false)
    }
}

/// `__kernel_sin(x, y, iy)` — polynôme sur [−π/4, π/4].
fn kernel_sin(x: f64, y: f64, iy: i32) -> f64 {
    let z = x * x;
    let v = z * x;
    let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    if iy == 0 {
        x + v * (S1 + z * r)
    } else {
        x - ((z * (0.5 * y - v * r) - y) - v * S1)
    }
}

/// `__kernel_cos(x, y)` — polynôme sur [−π/4, π/4] avec raffinement qx (|x| > 0.3).
fn kernel_cos(x: f64, y: f64) -> f64 {
    let ix = high_word(x);
    let z = x * x;
    let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6))))); // z*r = C1·z² + …
    if ix < 0x3FD3_3333 {
        // |x| < 0.3
        return 1.0 - (0.5 * z - (z * r - x * y));
    }
    let qx: f64 = if ix > 0x3FE9_0000 {
        // |x| > 0.78125
        0.28125
    } else {
        // qx = |x|/4, mot bas masqué (exact)
        f64::from_bits(u64::from(ix - 0x0020_0000) << 32)
    };
    let iz = 0.5 * z - qx;
    let a = 1.0 - qx;
    a - (iz - (z * r - x * y))
}

/// Équivalent exact de `Math.sin(x)` pour |x| < 2^19·π/2.
pub fn sin(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return f64::NAN; // sin(±∞) = NaN, comme V8 (x - x)
    }
    if x == 0.0 {
        return x; // préserve ±0
    }
    let (n, y0, y1, no_reduction) = rem_pio2(x);
    let iy = i32::from(!no_reduction);
    match n & 3 {
        0 => kernel_sin(y0, y1, iy),
        1 => kernel_cos(y0, y1),
        2 => -kernel_sin(y0, y1, iy),
        _ => -kernel_cos(y0, y1),
    }
}

/// Équivalent exact de `Math.cos(x)` pour |x| < 2^19·π/2.
pub fn cos(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return f64::NAN; // cos(±∞) = NaN, comme V8 (x - x)
    }
    let (n, y0, y1, no_reduction) = rem_pio2(x);
    let iy = i32::from(!no_reduction);
    match n & 3 {
        0 => kernel_cos(y0, y1),
        1 => -kernel_sin(y0, y1, iy),
        2 => -kernel_cos(y0, y1),
        _ => kernel_sin(y0, y1, iy),
    }
}

// ----------------------------------------------------- réduction « large path »

/// Constantes exactes (générées par entiers, formule de Machin de π ; vérifiées
/// contre les décimales) : ⌊π/2·2^1240⌋ et ⌊2/π·2^1240⌋, little-endian u64.
const PI2_LIMBS: [u64; 20] = [
    0xac410aa5773daa52,
    0x47b9475b2c38c5e6,
    0x51fa499ebf06caba,
    0x6c38aba734d22c7f,
    0x7142c77e0b31b490,
    0xa3d9c8b67b8400f9,
    0x22f8963fcc9250cc,
    0xf6b5133f4b5d3e48,
    0x6de80d6fdbdc70d7,
    0xd34c6fdad617feb9,
    0xecc4bcfd8de89885,
    0xdadaa3848bc90b6a,
    0xdbe4be286e9fc26a,
    0x679a748636605614,
    0xf31c6809bbdf2a33,
    0x4c76273644a29410,
    0x1114cf98e804177d,
    0x1701b839a252049c,
    0x4442d18469898cc5,
    0x0000000001921fb5,
];

const TWO_OVER_PI_LIMBS: [u64; 20] = [
    0x46fc7b6babf0cfbc,
    0xb11f8d5d08560330,
    0xf78a5292ea6bfb5f,
    0xc7ebe5f17b3d0739,
    0x669e5fea2d7527ba,
    0xcf27cb09b74f463f,
    0x8b5a0a6d1f6d367e,
    0xffde05980fef2f11,
    0x8bbdf9283b1ff897,
    0x39835339f49c845f,
    0x26b45f7e413991d6,
    0xf52ebb4484e99c70,
    0x1cb129a73ee88235,
    0xea09d1921cfe1deb,
    0x3a424dd2e006492e,
    0xabdebbc561b7246e,
    0x993c439041fe5163,
    0xd1f534ddc0db6295,
    0x6e4e441529fc2757,
    0x0000000000a2f983,
];

const NL: usize = 36; // 2304 bits ≥ 2·e_max + 105 (e_max = 971 pour un f64 fini)

fn big_bitlen(a: &[u64; NL]) -> usize {
    for i in (0..NL).rev() {
        if a[i] != 0 {
            return 64 * i + (64 - a[i].leading_zeros() as usize);
        }
    }
    0
}

/// a + value·2^bit (value < 2^64 ; retenu propagé, buffers surdimensionnés).
fn big_add_value(a: &mut [u64; NL], bit: usize, value: u64) {
    let lo = bit / 64;
    let off = bit % 64;
    let mut carry: u128 = u128::from(value) << off;
    let mut i = lo;
    while carry != 0 && i < NL {
        let cur = u128::from(a[i]) + (carry & 0xFFFF_FFFF_FFFF_FFFF);
        a[i] = cur as u64;
        carry = (carry >> 64) + (cur >> 64);
        i += 1;
    }
}

/// a >> shift (shift < 1240) depuis un tableau 20 limbes.
fn big_shr19(a: &[u64; 20], shift: usize) -> [u64; NL] {
    let mut out = [0u64; NL];
    let limbs = shift / 64;
    let bits = shift % 64;
    for (i, item) in out.iter_mut().enumerate() {
        let src = i + limbs;
        if src >= 20 {
            break;
        }
        let mut v = a[src] >> bits;
        if bits > 0 && src + 1 < 20 {
            v |= a[src + 1] << (64 - bits);
        }
        *item = v;
    }
    out
}

fn big_shr19_full(a: &[u64; NL], shift: usize) -> [u64; NL] {
    let mut out = [0u64; NL];
    let limbs = shift / 64;
    let bits = shift % 64;
    for (i, item) in out.iter_mut().enumerate() {
        let src = i + limbs;
        if src >= NL {
            break;
        }
        let mut v = a[src] >> bits;
        if bits > 0 && src + 1 < NL {
            v |= a[src + 1] << (64 - bits);
        }
        *item = v;
    }
    out
}

fn big_mul(a: &[u64; NL], b: &[u64; NL]) -> [u64; NL] {
    let mut out = [0u64; NL];
    for i in 0..NL {
        if a[i] == 0 {
            continue;
        }
        let mut carry: u128 = 0;
        for j in 0..(NL - i) {
            let cur = u128::from(out[i + j]) + u128::from(a[i]) * u128::from(b[j]) + carry;
            out[i + j] = cur as u64;
            carry = cur >> 64;
        }
    }
    out
}

/// |a − b| et le signe (true = a < b).
fn big_sub(a: &[u64; NL], b: &[u64; NL]) -> ([u64; NL], bool) {
    let mut cmp = false;
    for i in (0..NL).rev() {
        if a[i] != b[i] {
            cmp = a[i] > b[i];
            break;
        }
    }
    let (x, y) = if cmp { (a, b) } else { (b, a) };
    let mut mag = [0u64; NL];
    let mut borrow: u64 = 0;
    for i in 0..NL {
        let (d1, b1) = x[i].overflowing_sub(y[i]);
        let (d2, b2) = d1.overflowing_sub(borrow);
        mag[i] = d2;
        borrow = u64::from(b1 || b2);
    }
    (mag, !cmp)
}

/// Décalage gauche générique.
fn big_shl(a: &[u64; NL], sh: usize) -> [u64; NL] {
    let mut out = [0u64; NL];
    let limbs = sh / 64;
    let bits = sh % 64;
    for i in (limbs..NL).rev() {
        let src = i - limbs;
        let mut v = a[src] << bits;
        if bits > 0 && src > 0 {
            v |= a[src - 1] >> (64 - bits);
        }
        out[i] = v;
    }
    out
}

/// a + 1 (retenue propagée).
fn big_add_one(a: &mut [u64; NL]) {
    for item in a.iter_mut() {
        let (v, ov) = item.overflowing_add(1);
        *item = v;
        if !ov {
            break;
        }
    }
}

/// V >> sh arrondi au plus proche, égalités → pair.
fn big_shr_round_even(v: &[u64; NL], sh: usize) -> [u64; NL] {
    if sh == 0 {
        return *v;
    }
    let mut q = big_shr19_full(v, sh);
    let back = big_shl(&q, sh);
    let (dropped, _) = big_sub(v, &back);
    let dl = big_bitlen(&dropped);
    let round_up = if dl > sh {
        true
    } else if dl == sh {
        // dropped == 2^(sh−1) exactement ?
        let hi = sh / 64;
        let exact_half = dropped[hi] == 1u64 << (sh % 64)
            && dropped.iter().enumerate().all(|(i, x)| i == hi || *x == 0);
        !exact_half || (q[0] & 1 == 1)
    } else {
        false
    };
    if round_up {
        big_add_one(&mut q);
    }
    q
}

/// Représentation double-double de V·2^scale façon fdlibm (rem_pio2 large) :
/// V est arrondi à 24 bits AU-DELÀ du double (77 bits significatifs), puis
/// y0 = arrondi double de cette valeur et y1 = résidu arrondi à 24 bits.
fn big_double_double(v: &[u64; NL], scale: i32) -> (f64, f64) {
    let l = big_bitlen(v);
    if l == 0 {
        return (0.0, 0.0);
    }
    if l <= 53 {
        return ((v[0] as f64) * 2f64.powi(scale), 0.0);
    }
    let v77 = big_shr_round_even(v, l - 77);
    let l77 = big_bitlen(&v77);
    let sh0 = l77 - 53;
    let q0 = big_shr_round_even(&v77, sh0);
    let l0 = big_bitlen(&q0);
    // y0 = q0·2^(scale + (l−77) + (l77−l0)) — (l−77) est la position du mot de
    // 77 bits dans V ; la retenue d'arrondi est couverte par l77−l0.
    let y0 = (q0[0] as f64) * 2f64.powi(scale + (l - 77) as i32 + (l77 - l0) as i32);
    let mut back = [0u64; NL];
    big_add_value(&mut back, sh0, q0[0]);
    let (res, _) = big_sub(v, &back);
    let lr = big_bitlen(&res);
    let y1 = if lr == 0 {
        0.0
    } else if lr <= 24 {
        (res[0] as f64) * 2f64.powi(scale)
    } else {
        let shr = lr - 24;
        let q1 = big_shr_round_even(&res, shr);
        let l1 = big_bitlen(&q1);
        (q1[0] as f64) * 2f64.powi(scale + (l1 - 24) as i32)
    };
    (y0, y1)
}

fn rem_pio2_large(x: f64) -> (i32, f64, f64) {
    let bits = x.to_bits();
    let neg = (bits >> 63) != 0;
    let e = (((bits >> 52) & 0x7ff) as i32) - 1075; // x = m·2^e, m ∈ [2^52, 2^53)
    let m = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);

    // n = round(x·2/π) = round((m·W) / 2^240) avec W = ⌊2/π·2^(e+240)⌋
    // (erreur < m·2^−240 = 2^−187 : l'arrondi est exact)
    let w = big_shr19(&TWO_OVER_PI_LIMBS, (1240 - (e + 240)) as usize);
    let mut q = big_mul_u64_small(m, &w);
    big_add_value(&mut q, 239, 1);
    let n_big = big_shr19_full(&q, 240);
    let n4 = (n_big[0] & 3) as i32;

    // f = y·2^1240 = m·2^(e+1240) − n·⌊π/2·2^1240⌋ — π/2 NON arrondi (l'erreur
    // de troncature, ≤ n·2^−1174, est négligeable ; un P arrondi à l'entier
    // près donnerait n·0,5 ≈ 8 ulp d'erreur sur y — mesuré avant correction).
    let pi2_big = big_shr19(&PI2_LIMBS, 0);
    let np = big_mul(&n_big, &pi2_big);
    let mut a = [0u64; NL];
    big_add_value(&mut a, (e + 1240) as usize, m);
    let (f_mag, f_neg) = big_sub(&a, &np);

    let (y0_mag, y1_mag) = big_double_double(&f_mag, -1240);
    let (mut y0, mut y1) = (y0_mag, y1_mag);
    if f_neg {
        y0 = -y0;
        y1 = -y1;
    }
    if neg {
        // x < 0 : n' = −n, y' = −y (mod 4 : (4 − n4) & 3)
        return ((4 - n4) & 3, -y0, -y1);
    }
    (n4, y0, y1)
}

fn big_mul_u64_small(m: u64, w: &[u64; NL]) -> [u64; NL] {
    let mut out = [0u64; NL];
    let mut carry: u128 = 0;
    for i in 0..NL {
        let cur = carry + u128::from(m) * u128::from(w[i]);
        out[i] = cur as u64;
        carry = cur >> 64;
    }
    debug_assert_eq!(carry, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_sensibles() {
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(sin(-0.0), -0.0);
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(sin(std::f64::consts::FRAC_PI_2), 1.0);
        assert_eq!(cos(std::f64::consts::PI), -1.0);
        assert_eq!(cos(std::f64::consts::FRAC_PI_2), 6.123233995736766e-17);
        // multiples exacts de pi/2 : itérations 2/3 de rem_pio2
        assert_eq!(sin(3.0 * std::f64::consts::FRAC_PI_2), -1.0);
        assert_eq!(cos(2.0 * std::f64::consts::PI), 1.0);
        assert!(sin(f64::NAN).is_nan());
        assert!(sin(f64::INFINITY).is_nan());
        assert!(cos(f64::NEG_INFINITY).is_nan());
    }
}
