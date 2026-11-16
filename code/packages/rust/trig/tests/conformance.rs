// ============================================================================
// Bit-exact conformance against the shared TRIG-REF-1 vectors
// ============================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as a row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// with x and y as IEEE binary64 bit patterns (16 lowercase hex digits).  We decode
// x by a bit cast, call the public function, encode the result and compare the
// two integers (PHY00a 6.5).  A NaN result matches any NaN.  `error` means sqrt
// of a negative, which in this crate panics.

use std::fs;
use std::panic;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../specs/fixtures/trig-reference-v1")
}

fn bits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

fn call(op: &str, x: &str) -> f64 {
    if op == "atan2" {
        let (ys, xs) = x.split_once(',').unwrap();
        return trig::atan2(bits(ys), bits(xs));
    }
    let v = bits(x);
    match op {
        "sin" => trig::sin(v),
        "cos" => trig::cos(v),
        "tan" => trig::tan(v),
        "radians" => trig::radians(v),
        "degrees" => trig::degrees(v),
        "sqrt" => trig::sqrt(v),
        "atan" => trig::atan(v),
        _ => panic!("unknown op {op}"),
    }
}

fn check_group(group: &str) {
    let text = fs::read_to_string(fixture_dir().join(format!("{group}.tsv"))).unwrap();
    assert!(text.ends_with('\n'));
    let mut rows = 0;
    let mut failures = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 5, "{line}");
        let (op, x, y) = (f[0], f[1], f[2]);
        rows += 1;
        if y == "error" {
            let r = panic::catch_unwind(|| call(op, x));
            if r.is_ok() {
                failures.push(format!("{op} {x}: no error"));
            }
            continue;
        }
        let want = u64::from_str_radix(y, 16).unwrap();
        let got = call(op, x);
        let ok = if want == 0x7FF8000000000000 {
            got.is_nan()
        } else {
            got.to_bits() == want
        };
        if !ok {
            failures.push(format!("{op} {x}: want {y}, got {:016x}", got.to_bits()));
        }
    }
    assert!(rows > 0);
    assert!(
        failures.is_empty(),
        "{} failures, first: {:?}",
        failures.len(),
        &failures[..failures.len().min(8)]
    );
}

#[test]
fn edge_vectors_match_bit_for_bit() {
    check_group("edge");
}
#[test]
fn sweep_binade_vectors_match_bit_for_bit() {
    check_group("sweep-binade");
}
#[test]
fn sweep_cw_vectors_match_bit_for_bit() {
    check_group("sweep-cw");
}
#[test]
fn sweep_prng_vectors_match_bit_for_bit() {
    check_group("sweep-prng");
}

#[test]
fn contract_sin_negative_zero_domain_and_atan2_nan() {
    assert_eq!(trig::sin(-0.0).to_bits(), 0x8000000000000000);
    assert_eq!(trig::cos(0.0), 1.0);
    for x in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e20,
        536870913.0,
    ] {
        assert!(trig::sin(x).is_nan() && trig::cos(x).is_nan() && trig::tan(x).is_nan());
    }
    assert_eq!(trig::sin(536870912.0).to_bits(), 0x3FD4E67C0E2622DF);
    assert!(trig::atan2(f64::NAN, 1.0).is_nan());
    assert!(trig::atan2(1.0, f64::NAN).is_nan());
    assert_eq!(trig::atan2(-0.0, -1.0), trig::PI);
    assert_eq!(trig::radians(180.0), trig::PI);
    assert_eq!(trig::degrees(trig::PI), 180.0);
}
