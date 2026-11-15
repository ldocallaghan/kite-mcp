//! Exact transport of IEEE 754 doubles.
//!
//! Between the server and the worker a double travels as the 16 hexadecimal
//! digits of its big-endian bit pattern. Towards clients a double is written
//! as the shortest decimal that parses back to the same bits (serde_json uses
//! the `ryu` algorithm), and a value that JSON cannot hold as a number
//! (NaN, R's NA, infinities) is written as a string.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The low 32 bits of R's `NA_real_` (a NaN with payload 1954).
const R_NA_LOW_WORD: u64 = 1954;

pub fn f64_to_hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

pub fn hex_to_f64(s: &str) -> Result<f64, String> {
    if s.len() != 16
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!(
            "`{s}` is not a double: expected 16 lower-case hexadecimal digits"
        ));
    }
    u64::from_str_radix(s, 16)
        .map(f64::from_bits)
        .map_err(|e| e.to_string())
}

/// True when `x` carries the bit pattern R uses for `NA_real_`.
pub fn is_r_na(x: f64) -> bool {
    x.is_nan() && (x.to_bits() & 0xffff_ffff) == R_NA_LOW_WORD
}

/// A double on the worker protocol: hexadecimal bit pattern.
#[derive(Clone, Copy, PartialEq)]
pub struct HexF64(pub f64);

impl fmt::Debug for HexF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({})", self.0, f64_to_hex(self.0))
    }
}

impl Serialize for HexF64 {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&f64_to_hex(self.0))
    }
}

impl<'de> Deserialize<'de> for HexF64 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        hex_to_f64(&s).map(HexF64).map_err(serde::de::Error::custom)
    }
}

/// A double as shown to clients. Finite values are JSON numbers that parse
/// back to the same bits; other values are the strings "NA", "NaN", "Inf"
/// and "-Inf".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Num(pub f64);

impl Num {
    pub fn label(&self) -> Option<&'static str> {
        let x = self.0;
        if x.is_finite() {
            None
        } else if is_r_na(x) {
            Some("NA")
        } else if x.is_nan() {
            Some("NaN")
        } else if x > 0.0 {
            Some("Inf")
        } else {
            Some("-Inf")
        }
    }
}

impl Serialize for Num {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.label() {
            None => s.serialize_f64(self.0),
            Some(l) => s.serialize_str(l),
        }
    }
}

impl JsonSchema for Num {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Number".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "description": "A double exactly as KITE computed it. Finite values are JSON numbers that parse back to the same IEEE 754 value; NA, NaN, Inf and -Inf are strings.",
            "oneOf": [
                {"type": "number"},
                {"type": "string", "enum": ["NA", "NaN", "Inf", "-Inf"]}
            ]
        })
    }
}

/// `x` to 4 significant figures, for display. Values from 0.001 up to
/// 10,000 are written as decimals, others in e-notation (`9.757e-5`).
/// Non-finite values keep their labels.
pub fn display4(x: f64) -> String {
    if let Some(l) = Num(x).label() {
        return l.to_string();
    }
    if x == 0.0 {
        return "0".into();
    }
    // the exponent after rounding to 4 significant figures, so that 9.9996 counts as 10
    let sci = format!("{x:.3e}");
    let e: i32 = sci
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    if !(-3..4).contains(&e) {
        sci
    } else {
        let decimals = (3 - e).max(0) as usize;
        format!("{x:.decimals$}")
    }
}

/// The percentage change a ratio stands for, `(ratio - 1) * 100`, to 4
/// significant figures, with a percent sign.
pub fn percent_display(ratio: f64) -> String {
    format!("{} %", display4((ratio - 1.0) * 100.0))
}

/// Shortest decimal that parses back to `x` in Rust, written so that R also
/// reads it as a numeric literal (for example `1.2`, `200.0`, `1e-10`).
/// Whether R's parser returns the same bits is checked by the conformance
/// tests (`tests/conformance`).
pub fn shortest_decimal(x: f64) -> String {
    format!("{x:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip_keeps_every_bit() {
        let mut values = vec![
            0.0,
            -0.0,
            1.0,
            1.2,
            0.1,
            1.0 / 3.0,
            f64::MIN_POSITIVE,
            5e-324,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            f64::from_bits(0x7ff0_0000_0000_07a2),
        ];
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..10_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            values.push(f64::from_bits(state));
        }
        for x in values {
            let h = f64_to_hex(x);
            assert_eq!(h.len(), 16);
            assert_eq!(hex_to_f64(&h).unwrap().to_bits(), x.to_bits());
        }
        assert_eq!(f64_to_hex(1.0), "3ff0000000000000");
        assert_eq!(f64_to_hex(-0.0), "8000000000000000");
    }

    #[test]
    fn malformed_hex_is_rejected() {
        assert!(hex_to_f64("3FF0000000000000").is_err());
        assert!(hex_to_f64("1.2").is_err());
        assert!(hex_to_f64("3ff000000000000").is_err());
    }

    #[test]
    fn client_numbers_parse_back_to_the_same_bits() {
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        for _ in 0..10_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = f64::from_bits(state);
            if !x.is_finite() {
                continue;
            }
            let json = serde_json::to_string(&Num(x)).unwrap();
            let back: f64 = serde_json::from_str(&json).unwrap();
            assert_eq!(back.to_bits(), x.to_bits(), "{json}");
        }
    }

    #[test]
    fn display_values_have_four_significant_figures() {
        assert_eq!(display4(0.8516332754471132), "0.8516");
        assert_eq!(display4(1.1449932563292378), "1.145");
        assert_eq!(display4(490.26744994553644), "490.3");
        assert_eq!(display4(1234.56), "1235");
        assert_eq!(display4(12345.6), "1.235e4");
        assert_eq!(display4(9.757295662195745e-5), "9.757e-5");
        assert_eq!(display4(0.0012345), "0.001234");
        assert_eq!(display4(9.99996), "10.00");
        assert_eq!(display4(-1.0233699636502482), "-1.023");
        assert_eq!(display4(0.0), "0");
        assert_eq!(display4(f64::NAN), "NaN");
        assert_eq!(percent_display(0.8516332754471132), "-14.84 %");
        assert_eq!(percent_display(1.1449932563292378), "14.50 %");
        assert_eq!(percent_display(1.0), "0 %");
    }

    #[test]
    fn display_values_round_correctly() {
        // every display value is within half a unit in its last place of the exact value
        let mut state: u64 = 0x0123_4567_89ab_cdef;
        for _ in 0..10_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = (state % 2_000_000) as f64 / 1_000_000.0 + 1e-9;
            let s = display4(x);
            let shown: f64 = s.parse().unwrap();
            let (mantissa, exp) = match s.split_once('e') {
                Some((m, e)) => (m.to_string(), e.parse::<i32>().unwrap()),
                None => (s.clone(), 0),
            };
            let decimals = mantissa.split_once('.').map(|(_, d)| d.len()).unwrap_or(0) as i32;
            let digits = mantissa
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>();
            assert_eq!(digits.trim_start_matches('0').len(), 4, "{s}");
            assert!(
                (shown - x).abs() <= 0.5 * 10f64.powi(exp - decimals) * (1.0 + 1e-9),
                "{x} {s}"
            );
        }
    }

    #[test]
    fn non_finite_values_are_labelled() {
        let na = f64::from_bits(0x7ff0_0000_0000_07a2);
        assert_eq!(serde_json::to_string(&Num(na)).unwrap(), "\"NA\"");
        assert_eq!(serde_json::to_string(&Num(f64::NAN)).unwrap(), "\"NaN\"");
        assert_eq!(
            serde_json::to_string(&Num(f64::INFINITY)).unwrap(),
            "\"Inf\""
        );
        assert_eq!(
            serde_json::to_string(&Num(f64::NEG_INFINITY)).unwrap(),
            "\"-Inf\""
        );
    }
}
