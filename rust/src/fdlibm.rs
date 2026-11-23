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
    // Medium : |x| ~<= 2^19*(pi/2)
    debug_assert!(
        ix <= 0x4139_21FB,
        "réduction « large path » non portée (|x| ≥ 2^19·π/2)"
    );
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
