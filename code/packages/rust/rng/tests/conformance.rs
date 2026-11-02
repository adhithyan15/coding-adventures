//! The shared rng corpus (spec rng.md, "Conformance corpus").
//!
//! `code/specs/fixtures/rng-v1/cases.json` pins exact outputs every
//! language's rng package must reproduce; its results come from the
//! independent reference `code/scripts/rng_conformance.py`. The corpus is
//! embedded at compile time, so the test reads no files.
//!
//! The crate has no dependencies, so the JSON is read by the small parser at
//! the bottom of this file. It accepts exactly what the corpus uses (objects,
//! arrays and ASCII strings without escapes) and panics on anything else.

use rng::{Lcg, Pcg32, Xorshift64};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

const CORPUS: &str = include_str!("../../../../specs/fixtures/rng-v1/cases.json");

/// The three generators behind one interface, for the corpus driver.
enum Gen {
    Lcg(Lcg),
    Xorshift64(Xorshift64),
    Pcg32(Pcg32),
}

impl Gen {
    fn new(name: &str, seed: u64) -> Gen {
        match name {
            "lcg" => Gen::Lcg(Lcg::new(seed)),
            "xorshift64" => Gen::Xorshift64(Xorshift64::new(seed)),
            "pcg32" => Gen::Pcg32(Pcg32::new(seed)),
            other => panic!("unknown generator {other}"),
        }
    }
    fn next_u32(&mut self) -> u32 {
        match self {
            Gen::Lcg(g) => g.next_u32(),
            Gen::Xorshift64(g) => g.next_u32(),
            Gen::Pcg32(g) => g.next_u32(),
        }
    }
    fn next_u64(&mut self) -> u64 {
        match self {
            Gen::Lcg(g) => g.next_u64(),
            Gen::Xorshift64(g) => g.next_u64(),
            Gen::Pcg32(g) => g.next_u64(),
        }
    }
    fn next_float(&mut self) -> f64 {
        match self {
            Gen::Lcg(g) => g.next_float(),
            Gen::Xorshift64(g) => g.next_float(),
            Gen::Pcg32(g) => g.next_float(),
        }
    }
    fn next_int_in_range(&mut self, min: i64, max: i64) -> i64 {
        match self {
            Gen::Lcg(g) => g.next_int_in_range(min, max),
            Gen::Xorshift64(g) => g.next_int_in_range(min, max),
            Gen::Pcg32(g) => g.next_int_in_range(min, max),
        }
    }
}

#[test]
fn every_corpus_case_matches() {
    let document = Json::parse(CORPUS);
    let cases = document.get("cases").array();
    assert!(
        cases.len() >= 27,
        "the corpus shrank to {} cases",
        cases.len()
    );
    for case in cases {
        let id = case.get("id").string();
        let seed: u64 = case.get("seed").string().parse().unwrap();
        let mut g = Gen::new(case.get("generator").string(), seed);
        for (index, call) in case.get("calls").array().iter().enumerate() {
            let op = call.get("op").string();
            let at = format!("{id} call {index} ({op})");
            match op {
                "next_u32" => assert_eq!(
                    g.next_u32().to_string(),
                    call.get("result").string(),
                    "{at}"
                ),
                "next_u64" => assert_eq!(
                    g.next_u64().to_string(),
                    call.get("result").string(),
                    "{at}"
                ),
                "next_float" => assert_eq!(
                    format!("{:016x}", g.next_float().to_bits()),
                    call.get("result_bits").string(),
                    "{at}"
                ),
                "next_int_in_range" => {
                    let min: i64 = call.get("min").string().parse().unwrap();
                    let max: i64 = call.get("max").string().parse().unwrap();
                    if call.has("error") {
                        // Rust reports an invalid range by panicking, before
                        // any draw: the sequence must carry on untouched.
                        let outcome =
                            catch_unwind(AssertUnwindSafe(|| g.next_int_in_range(min, max)));
                        assert!(outcome.is_err(), "{at}: expected a panic");
                    } else {
                        assert_eq!(
                            g.next_int_in_range(min, max).to_string(),
                            call.get("result").string(),
                            "{at}"
                        );
                    }
                }
                other => panic!("{at}: unknown op {other}"),
            }
        }
    }
}

// ── Regressions for the v1 range bug ─────────────────────────────────────────

#[test]
fn wide_ranges_reach_beyond_two_to_the_32() {
    // v1 drew 32 bits for every span. 64 draws over 2^40 values all landing
    // below 2^32 has probability 2^-512.
    let mut g = Pcg32::new(3);
    assert!((0..64).any(|_| g.next_int_in_range(0, 1 << 40) >= 1 << 32));
}

#[test]
fn the_whole_i64_domain_no_longer_divides_by_zero() {
    let mut g = Lcg::new(3);
    let values: Vec<i64> = (0..64)
        .map(|_| g.next_int_in_range(i64::MIN, i64::MAX))
        .collect();
    assert!(values.iter().any(|&v| v < 0) && values.iter().any(|&v| v > 0));
}

#[test]
fn ranges_ending_at_i64_max_do_not_overflow() {
    let mut g = Xorshift64::new(3);
    for _ in 0..1000 {
        assert!(g.next_int_in_range(i64::MAX - 5, i64::MAX) >= i64::MAX - 5);
        assert!(g.next_int_in_range(-1, i64::MAX) >= -1);
    }
}

// ── A minimal, strict JSON reader for the corpus ─────────────────────────────

enum Json {
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
    Num,
}

impl Json {
    fn parse(text: &str) -> Json {
        let bytes = text.as_bytes();
        let mut at = 0;
        let value = Json::value(bytes, &mut at);
        skip_ws(bytes, &mut at);
        assert_eq!(at, bytes.len(), "trailing data after the JSON document");
        value
    }

    fn value(b: &[u8], at: &mut usize) -> Json {
        skip_ws(b, at);
        match b[*at] {
            b'"' => {
                *at += 1;
                let start = *at;
                while b[*at] != b'"' {
                    assert!(
                        b[*at] != b'\\' && b[*at].is_ascii(),
                        "the corpus uses plain ASCII strings"
                    );
                    *at += 1;
                }
                *at += 1;
                Json::Str(String::from_utf8(b[start..*at - 1].to_vec()).unwrap())
            }
            b'[' => {
                *at += 1;
                let mut items = Vec::new();
                loop {
                    skip_ws(b, at);
                    if b[*at] == b']' {
                        *at += 1;
                        return Json::Arr(items);
                    }
                    if !items.is_empty() {
                        expect(b, at, b',');
                    }
                    items.push(Json::value(b, at));
                }
            }
            b'{' => {
                *at += 1;
                let mut fields = BTreeMap::new();
                loop {
                    skip_ws(b, at);
                    if b[*at] == b'}' {
                        *at += 1;
                        return Json::Obj(fields);
                    }
                    if !fields.is_empty() {
                        expect(b, at, b',');
                    }
                    let Json::Str(key) = Json::value(b, at) else {
                        panic!("object keys are strings")
                    };
                    skip_ws(b, at);
                    expect(b, at, b':');
                    let previous = fields.insert(key, Json::value(b, at));
                    assert!(previous.is_none(), "duplicate key");
                }
            }
            b'0'..=b'9' => {
                // Only schema_version is a bare number.
                while b[*at].is_ascii_digit() {
                    *at += 1;
                }
                Json::Num
            }
            other => panic!("unexpected byte {:?} in the corpus", other as char),
        }
    }

    fn get(&self, key: &str) -> &Json {
        match self {
            Json::Obj(fields) => fields.get(key).unwrap_or_else(|| panic!("missing {key}")),
            _ => panic!("not an object"),
        }
    }
    fn has(&self, key: &str) -> bool {
        matches!(self, Json::Obj(fields) if fields.contains_key(key))
    }
    fn array(&self) -> &[Json] {
        match self {
            Json::Arr(items) => items,
            _ => panic!("not an array"),
        }
    }
    fn string(&self) -> &str {
        match self {
            Json::Str(text) => text,
            _ => panic!("not a string"),
        }
    }
}

fn skip_ws(b: &[u8], at: &mut usize) {
    while *at < b.len() && b[*at].is_ascii_whitespace() {
        *at += 1;
    }
}

fn expect(b: &[u8], at: &mut usize, byte: u8) {
    skip_ws(b, at);
    assert_eq!(b[*at], byte, "expected {:?}", byte as char);
    *at += 1;
}
