//! Independent exact-integer reference for the named Q16.16 fixture.
//!
//! Fixture expectations are checked by an independent BigInt oracle, then the
//! same vectors and seeded typed inputs are evaluated through the generated
//! Warrant model and the real public Rust exports.
mod warrant {
    #![allow(dead_code, unused_variables, unused_mut, unused_parens, clippy::all)]
    include!("generated/warrant_evidence.rs");
    include!("support/warrant_model_bridge.rs");
}

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use serde_json::{json, Value};

const FIXTURE: &str = include_str!("../../../../specs/fixtures/fixed-point-q16-v1/cases.json");
const WARRANT_SPEC: &str = include_str!("../../../../specs/fixed-point.warrant");
const SCALE: i64 = 65_536;
const MIN: i32 = i32::MIN;
const MAX: i32 = i32::MAX;
const MODES: [&str; 4] = ["TowardZero", "Floor", "Ceil", "NearestEven"];

fn rounded(n: BigInt, d: BigInt, mode: &str) -> BigInt {
    assert!(
        d > BigInt::from(0),
        "oracle denominator must be normalized positive"
    );
    let toward_zero = &n / &d;
    let remainder = &n % &d;
    let floor = if remainder.is_negative() {
        &toward_zero - 1
    } else {
        toward_zero.clone()
    };
    let ceil = if remainder.is_positive() {
        &toward_zero + 1
    } else {
        toward_zero
    };
    match mode {
        "TowardZero" => {
            if n.is_negative() {
                ceil
            } else {
                floor
            }
        }
        "Floor" => floor,
        "Ceil" => ceil,
        "NearestEven" => {
            // Compare exact distances to the two integer brackets. This is an
            // independent derivation from signed quotient/remainder adjustment.
            let from_floor = &n - (&floor * &d);
            let to_ceil = (&ceil * &d) - &n;
            match from_floor.cmp(&to_ceil) {
                std::cmp::Ordering::Less => floor,
                std::cmp::Ordering::Greater => ceil,
                std::cmp::Ordering::Equal if (&floor % 2) == BigInt::from(0) => floor,
                std::cmp::Ordering::Equal => ceil,
            }
        }
        _ => panic!("unknown rounding mode {mode}"),
    }
}

fn checked(raw: BigInt) -> Value {
    if raw >= BigInt::from(MIN) && raw <= BigInt::from(MAX) {
        json!({"ok_raw": raw.to_i32().expect("range checked")})
    } else {
        json!({"error": "Overflow"})
    }
}

fn saturated(raw: BigInt) -> Value {
    let in_range = raw >= BigInt::from(MIN) && raw <= BigInt::from(MAX);
    let value = if in_range {
        raw.to_i32().expect("range checked")
    } else if raw < BigInt::from(MIN) {
        MIN
    } else {
        MAX
    };
    json!({"raw": value, "overflowed": !in_range})
}

fn checked_saturated(raw: BigInt) -> Value {
    json!({"checked": checked(raw.clone()), "saturated": saturated(raw)})
}

fn operation_expected(vector: &Value, mode: Option<&str>) -> Value {
    let op = vector["operation"].as_str().unwrap();
    let input = &vector["input"];
    let a = input["a_raw"].as_i64().map(BigInt::from);
    let b = input["b_raw"].as_i64().map(BigInt::from);
    match op {
        "from_integer" => checked(BigInt::from(input["integer"].as_i64().unwrap()) * SCALE),
        "to_integer" => json!(rounded(
            BigInt::from(input["raw"].as_i64().unwrap()),
            BigInt::from(SCALE),
            mode.unwrap()
        )
        .to_i32()
        .unwrap()),
        "add" => checked_saturated(a.unwrap() + b.unwrap()),
        "sub" => checked_saturated(a.unwrap() - b.unwrap()),
        "mul" => checked_saturated(rounded(
            a.unwrap() * b.unwrap(),
            BigInt::from(SCALE),
            mode.unwrap(),
        )),
        "div" => {
            let denominator = b.unwrap();
            if denominator == BigInt::from(0) {
                json!({"checked":{"error":"DivisionByZero"},"saturated":{"error":"DivisionByZero"}})
            } else {
                let numerator = a.unwrap() * SCALE * denominator.signum();
                checked_saturated(rounded(numerator, denominator.abs(), mode.unwrap()))
            }
        }
        other => panic!("unknown fixture operation {other}"),
    }
}

#[test]
fn fixture_schema_and_count_are_pinned() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    assert_eq!(fixture["schema"], "coding-adventures/fixed-point-q16-v1");
    assert_eq!(fixture["scale"], SCALE);
    assert_eq!(fixture["raw_min"], i32::MIN);
    assert_eq!(fixture["raw_max"], i32::MAX);
    assert_eq!(fixture["rounding_modes"], json!(MODES));
    assert_eq!(
        fixture["vector_count"].as_u64(),
        fixture["vectors"].as_array().map(|v| v.len() as u64)
    );
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 39, "fixture vector count is versioned");
    let mut ids = std::collections::BTreeSet::new();
    let mut operations = std::collections::BTreeMap::<&str, usize>::new();
    for vector in vectors {
        assert!(
            ids.insert(vector["id"].as_str().unwrap().to_owned()),
            "duplicate id"
        );
        let operation = vector["operation"].as_str().unwrap();
        *operations.entry(operation).or_default() += 1;
        let input = &vector["input"];
        for field in ["raw", "a_raw", "b_raw", "integer"] {
            if let Some(value) = input.get(field) {
                let value = value
                    .as_i64()
                    .expect("fixture inputs are integer JSON numbers");
                assert!(
                    (i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&value),
                    "{field} out of i32 range"
                );
            }
        }
        if matches!(operation, "mul" | "div" | "to_integer") {
            let keys: std::collections::BTreeSet<_> = vector["expected_by_mode"]
                .as_object()
                .expect("mode result object")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(keys, MODES.into_iter().collect(), "{operation} mode set");
        }
    }
    assert_eq!(
        operations,
        [
            ("add", 5),
            ("div", 10),
            ("from_integer", 7),
            ("mul", 6),
            ("sub", 4),
            ("to_integer", 7)
        ]
        .into()
    );
    for required in [
        "from-first-overflow-positive",
        "to-negative-half",
        "add-max-plus-one",
        "sub-min-minus-one",
        "mul-positive-half",
        "mul-negative-half",
        "mul-post-round-boundary",
        "div-zero-positive",
        "div-odd-tie",
        "div-negative-denominator",
        "div-min-denominator",
    ] {
        assert!(ids.contains(required), "missing named vector {required}");
    }
}

#[test]
fn every_fixture_expectation_matches_the_independent_exact_oracle() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    for vector in fixture["vectors"].as_array().unwrap() {
        let id = vector["id"].as_str().unwrap();
        match vector["operation"].as_str().unwrap() {
            "mul" | "div" | "to_integer" => {
                for mode in MODES {
                    assert_eq!(
                        vector["expected_by_mode"][mode],
                        operation_expected(vector, Some(mode)),
                        "{id} / {mode}"
                    );
                }
            }
            _ => assert_eq!(vector["expected"], operation_expected(vector, None), "{id}"),
        }
    }
}

#[test]
fn three_rounding_boundaries_discriminate_post_rounding_and_tie_policy() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    let vectors: std::collections::BTreeMap<_, _> = fixture["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v["id"].as_str().unwrap(), v))
        .collect();
    assert_eq!(
        vectors["mul-positive-half"]["expected_by_mode"]["NearestEven"]["checked"],
        json!({"error":"Overflow"})
    );
    assert_eq!(
        vectors["mul-positive-half"]["expected_by_mode"]["Floor"]["checked"],
        json!({"ok_raw":i32::MAX})
    );
    assert_eq!(
        vectors["mul-negative-half"]["expected_by_mode"]["Floor"]["checked"],
        json!({"error":"Overflow"})
    );
    assert_eq!(
        vectors["mul-negative-half"]["expected_by_mode"]["NearestEven"]["checked"],
        json!({"ok_raw":i32::MIN})
    );
    assert_eq!(
        vectors["mul-post-round-boundary"]["expected_by_mode"]["Ceil"]["saturated"],
        json!({"raw":i32::MAX,"overflowed":true})
    );
    assert_eq!(
        vectors["mul-post-round-boundary"]["expected_by_mode"]["NearestEven"]["saturated"],
        json!({"raw":i32::MAX,"overflowed":false})
    );
}

use coding_adventures_fixed_point::{
    checked_add, checked_div, checked_mul, checked_sub, from_integer, saturating_add,
    saturating_div, saturating_mul, saturating_sub, to_integer, Q16Error, RoundingMode,
    SaturatingResult, Q16,
};
use coding_adventures_test_bootstrap::fuzz::{fuzz_target, FuzzOptions, FuzzOutcome};
use std::cell::RefCell;

const PROPERTY_SEED: u64 = 0x5146_3136_5052_4f50;
const FUZZ_SEED: u64 = 0x5146_3136_4655_5a5a;
const PROPERTY_CASES: usize = 4096;
const FUZZ_CASES: usize = 4096;

fn mode(name: &str) -> RoundingMode {
    match name {
        "TowardZero" => RoundingMode::TowardZero,
        "Floor" => RoundingMode::Floor,
        "Ceil" => RoundingMode::Ceil,
        "NearestEven" => RoundingMode::NearestEven,
        _ => panic!("unknown mode {name}"),
    }
}

fn warrant_q16(raw: i32) -> warrant::V {
    warrant::V::Con(
        "Q16",
        0,
        "Q16",
        vec![warrant::V::Fixed(warrant::F::I32, i128::from(raw))],
    )
}

fn warrant_mode(name: &str) -> warrant::V {
    let (index, label) = match name {
        "TowardZero" => (0, "TowardZero"),
        "Floor" => (1, "Floor"),
        "Ceil" => (2, "Ceil"),
        "NearestEven" => (3, "NearestEven"),
        _ => panic!("unknown mode {name}"),
    };
    warrant::V::Con("RoundingMode", index, label, vec![])
}

fn warrant_result(raw: Result<i32, Q16Error>) -> warrant::V {
    match raw {
        Ok(value) => warrant::V::Ok(Box::new(warrant_q16(value))),
        Err(Q16Error::Overflow) => warrant::V::Err("Overflow".into()),
        Err(Q16Error::DivisionByZero) => warrant::V::Err("DivisionByZero".into()),
    }
}

fn warrant_api_result(raw: Result<Q16, Q16Error>) -> warrant::V {
    match raw {
        Ok(value) => warrant::V::Ok(Box::new(warrant_q16(value.raw))),
        Err(Q16Error::Overflow) => warrant::V::Err("Overflow".into()),
        Err(Q16Error::DivisionByZero) => warrant::V::Err("DivisionByZero".into()),
    }
}

fn assert_generated_warrant_is_current() {
    assert_eq!(
        coding_adventures_sha256::sha256_hex(WARRANT_SPEC.as_bytes()),
        warrant::SPEC_SHA256,
        "generated Warrant evaluators must match the current source before use"
    );
}

fn warrant_sat(value: SaturatingResult) -> warrant::V {
    warrant::V::Con(
        "SaturatingResult",
        0,
        "SaturatingResult",
        vec![
            warrant_q16(value.value.raw),
            warrant::V::Bool(value.overflowed),
        ],
    )
}

fn warrant_api_sat_div(value: Result<SaturatingResult, Q16Error>) -> warrant::V {
    match value {
        Ok(saturated) => warrant::V::Ok(Box::new(warrant_sat(saturated))),
        Err(Q16Error::Overflow) => warrant::V::Err("Overflow".into()),
        Err(Q16Error::DivisionByZero) => warrant::V::Err("DivisionByZero".into()),
    }
}

fn triple_result(
    actual: Result<Q16, Q16Error>,
    oracle: Result<i32, Q16Error>,
    model: warrant::V,
) -> bool {
    let actual = warrant_api_result(actual);
    actual == warrant_result(oracle) && actual == model
}

fn triple_saturated(actual: SaturatingResult, oracle: SaturatingResult, model: warrant::V) -> bool {
    let actual = warrant_sat(actual);
    actual == warrant_sat(oracle) && actual == model
}

fn oracle_div(a: i32, b: i32, rounding: &str) -> Result<i32, Q16Error> {
    if b == 0 {
        return Err(Q16Error::DivisionByZero);
    }
    let numerator = BigInt::from(a) * SCALE * i64::from(b.signum());
    let exact = rounded(numerator, BigInt::from(b).abs(), rounding);
    if exact < BigInt::from(MIN) || exact > BigInt::from(MAX) {
        Err(Q16Error::Overflow)
    } else {
        Ok(exact.to_i32().expect("bounded exact result"))
    }
}

fn oracle_saturating_add(a: i32, b: i32) -> SaturatingResult {
    oracle_saturating(BigInt::from(a) + BigInt::from(b))
}

fn oracle_checked(raw: BigInt) -> Result<i32, Q16Error> {
    if raw < BigInt::from(i32::MIN) || raw > BigInt::from(i32::MAX) {
        Err(Q16Error::Overflow)
    } else {
        Ok(raw.to_i32().expect("bounded exact result"))
    }
}

fn oracle_saturating(raw: BigInt) -> SaturatingResult {
    let bounded = raw
        .clone()
        .max(BigInt::from(i32::MIN))
        .min(BigInt::from(i32::MAX));
    SaturatingResult {
        value: Q16::from_raw(bounded.to_i32().expect("clamped result")),
        overflowed: raw < BigInt::from(i32::MIN) || raw > BigInt::from(i32::MAX),
    }
}

fn oracle_saturating_div(a: i32, b: i32, rounding: &str) -> Result<SaturatingResult, Q16Error> {
    if b == 0 {
        return Err(Q16Error::DivisionByZero);
    }
    let numerator = BigInt::from(a) * SCALE * i64::from(b.signum());
    Ok(oracle_saturating(rounded(
        numerator,
        BigInt::from(b).abs(),
        rounding,
    )))
}

fn oracle_from_integer(value: i32) -> Result<i32, Q16Error> {
    let raw = BigInt::from(value) * SCALE;
    if raw < BigInt::from(MIN) || raw > BigInt::from(MAX) {
        Err(Q16Error::Overflow)
    } else {
        Ok(raw.to_i32().expect("bounded exact result"))
    }
}

fn check_api(actual: Result<Q16, Q16Error>, expected: Result<i32, Q16Error>, context: &str) {
    match (actual, expected) {
        (Ok(actual), Ok(expected)) => assert_eq!(actual.raw, expected, "{context}"),
        (Err(actual), Err(expected)) => assert_eq!(actual, expected, "{context}"),
        (actual, expected) => panic!("{context}: actual {actual:?}, expected {expected:?}"),
    }
}

fn check_saturated(actual: SaturatingResult, expected: SaturatingResult) -> bool {
    actual.value.raw == expected.value.raw && actual.overflowed == expected.overflowed
}

fn encode_pair(a: i32, b: i32) -> Vec<u8> {
    [a.to_le_bytes(), b.to_le_bytes()].concat()
}

fn decode_i32(bytes: &[u8], offset: usize) -> i32 {
    let mut word = [0_u8; 4];
    let available = bytes.get(offset..).unwrap_or_default();
    let n = available.len().min(4);
    word[..n].copy_from_slice(&available[..n]);
    i32::from_le_bytes(word)
}

#[test]
fn named_vectors_match_the_real_public_exports() {
    assert_generated_warrant_is_current();
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    let mut compared = 0;
    for vector in fixture["vectors"].as_array().unwrap() {
        let op = vector["operation"].as_str().unwrap();
        let input = &vector["input"];
        match op {
            "from_integer" => {
                let value = input["integer"].as_i64().unwrap() as i32;
                assert!(
                    triple_result(
                        from_integer(value),
                        oracle_from_integer(value),
                        warrant::q16_from_integer_model(warrant::V::Fixed(
                            warrant::F::I32,
                            i128::from(value)
                        )),
                    ),
                    "Warrant/API/oracle from_integer {}",
                    vector["id"]
                );
                check_api(
                    from_integer(value),
                    oracle_from_integer(value),
                    vector["id"].as_str().unwrap(),
                );
                compared += 1;
            }
            "div" => {
                let a = input["a_raw"].as_i64().unwrap() as i32;
                let b = input["b_raw"].as_i64().unwrap() as i32;
                for rounding in MODES {
                    let args_a = warrant_q16(a);
                    let args_b = warrant_q16(b);
                    let args_mode = warrant_mode(rounding);
                    let expected = oracle_div(a, b, rounding);
                    let checked = checked_div(Q16::from_raw(a), Q16::from_raw(b), mode(rounding));
                    assert!(
                        triple_result(
                            checked,
                            expected,
                            warrant::q16_checked_div_model(
                                args_a.clone(),
                                args_b.clone(),
                                args_mode.clone()
                            )
                        ),
                        "Warrant/API/oracle checked_div {}/ {rounding}",
                        vector["id"]
                    );
                    check_api(
                        checked,
                        expected,
                        &format!("{} / {rounding}", vector["id"].as_str().unwrap()),
                    );
                    let actual_sat =
                        saturating_div(Q16::from_raw(a), Q16::from_raw(b), mode(rounding));
                    assert_eq!(
                        actual_sat,
                        oracle_saturating_div(a, b, rounding),
                        "saturating_div {} / {rounding}",
                        vector["id"]
                    );
                    assert_eq!(
                        warrant_api_sat_div(actual_sat),
                        warrant::q16_saturating_div_model(args_a, args_b, args_mode),
                        "Warrant/API saturating_div {}/{rounding}",
                        vector["id"]
                    );
                    compared += 2;
                }
            }
            "add" => {
                let a = input["a_raw"].as_i64().unwrap() as i32;
                let b = input["b_raw"].as_i64().unwrap() as i32;
                let expected = oracle_saturating_add(a, b);
                let actual = saturating_add(Q16::from_raw(a), Q16::from_raw(b));
                assert!(
                    triple_saturated(
                        actual,
                        expected,
                        warrant::q16_saturating_add_model(warrant_q16(a), warrant_q16(b))
                    ),
                    "Warrant/API/oracle saturating_add {}",
                    vector["id"]
                );
                let checked = checked_add(Q16::from_raw(a), Q16::from_raw(b));
                let checked_expected = oracle_checked(BigInt::from(a) + BigInt::from(b));
                assert!(
                    triple_result(
                        checked,
                        checked_expected,
                        warrant::q16_checked_add_model(warrant_q16(a), warrant_q16(b))
                    ),
                    "Warrant/API/oracle checked_add {}",
                    vector["id"]
                );
                check_api(checked, checked_expected, "checked_add vector");
                assert_eq!(actual, expected, "{}", vector["id"]);
                assert_eq!(
                    json!({"raw": actual.value.raw, "overflowed": actual.overflowed}),
                    vector["expected"]["saturated"],
                    "fixture result {}",
                    vector["id"]
                );
                compared += 2;
            }
            "sub" => {
                let a = input["a_raw"].as_i64().unwrap() as i32;
                let b = input["b_raw"].as_i64().unwrap() as i32;
                let checked = checked_sub(Q16::from_raw(a), Q16::from_raw(b));
                let checked_expected = oracle_checked(BigInt::from(a) - BigInt::from(b));
                assert!(
                    triple_result(
                        checked,
                        checked_expected,
                        warrant::q16_checked_sub_model(warrant_q16(a), warrant_q16(b))
                    ),
                    "Warrant/API/oracle checked_sub {}",
                    vector["id"]
                );
                check_api(checked, checked_expected, "checked_sub vector");
                let saturated = saturating_sub(Q16::from_raw(a), Q16::from_raw(b));
                let sat_expected = oracle_saturating(BigInt::from(a) - BigInt::from(b));
                assert!(
                    triple_saturated(
                        saturated,
                        sat_expected,
                        warrant::q16_saturating_sub_model(warrant_q16(a), warrant_q16(b))
                    ),
                    "Warrant/API/oracle saturating_sub {}",
                    vector["id"]
                );
                assert_eq!(saturated, sat_expected, "saturating_sub vector");
                compared += 2;
            }
            "mul" => {
                let a = input["a_raw"].as_i64().unwrap() as i32;
                let b = input["b_raw"].as_i64().unwrap() as i32;
                for rounding in MODES {
                    let exact = rounded(
                        BigInt::from(a) * BigInt::from(b),
                        BigInt::from(SCALE),
                        rounding,
                    );
                    let args_a = warrant_q16(a);
                    let args_b = warrant_q16(b);
                    let args_mode = warrant_mode(rounding);
                    let checked = checked_mul(Q16::from_raw(a), Q16::from_raw(b), mode(rounding));
                    let checked_expected = oracle_checked(exact.clone());
                    assert!(
                        triple_result(
                            checked,
                            checked_expected,
                            warrant::q16_checked_mul_model(
                                args_a.clone(),
                                args_b.clone(),
                                args_mode.clone()
                            )
                        ),
                        "Warrant/API/oracle checked_mul {} / {rounding}",
                        vector["id"]
                    );
                    check_api(checked, checked_expected, "checked_mul vector");
                    let saturated =
                        saturating_mul(Q16::from_raw(a), Q16::from_raw(b), mode(rounding));
                    let sat_expected = oracle_saturating(exact);
                    assert!(
                        triple_saturated(
                            saturated,
                            sat_expected,
                            warrant::q16_saturating_mul_model(args_a, args_b, args_mode)
                        ),
                        "Warrant/API/oracle saturating_mul {} / {rounding}",
                        vector["id"]
                    );
                    assert_eq!(saturated, sat_expected, "saturating_mul vector");
                    compared += 2;
                }
            }
            "to_integer" => {
                let raw = input["raw"].as_i64().unwrap() as i32;
                for rounding in MODES {
                    let actual = to_integer(Q16::from_raw(raw), mode(rounding));
                    let expected = rounded(BigInt::from(raw), BigInt::from(SCALE), rounding)
                        .to_i32()
                        .unwrap();
                    assert_eq!(
                        warrant::V::Fixed(warrant::F::I32, i128::from(actual)),
                        warrant::q16_to_integer_model(warrant_q16(raw), warrant_mode(rounding)),
                        "Warrant/API to_integer vector"
                    );
                    assert_eq!(actual, expected, "to_integer vector");
                    compared += 1;
                }
            }
            other => panic!("unrecognized fixture operation {other}"),
        }
    }
    assert_eq!(
        compared,
        7 + 10 * MODES.len() * 2 + 5 * 2 + 4 * 2 + 6 * MODES.len() * 2 + 7 * MODES.len()
    );
}

#[derive(Clone, Debug)]
struct BinaryCase {
    a: i32,
    b: i32,
}

fn shrink_binary(case: BinaryCase, fails: &dyn Fn(&BinaryCase) -> bool) -> (BinaryCase, usize) {
    let (a, a_steps) = coding_adventures_test_bootstrap::pbt::shrink_int(
        i128::from(case.a),
        0,
        i128::from(i32::MIN),
        i128::from(i32::MAX),
        |candidate| {
            fails(&BinaryCase {
                a: candidate as i32,
                b: case.b,
            })
        },
        128,
    );
    let with_a = BinaryCase {
        a: a as i32,
        b: case.b,
    };
    let (b, b_steps) = coding_adventures_test_bootstrap::pbt::shrink_int(
        i128::from(with_a.b),
        0,
        i128::from(i32::MIN),
        i128::from(i32::MAX),
        |candidate| {
            fails(&BinaryCase {
                a: with_a.a,
                b: candidate as i32,
            })
        },
        128,
    );
    (
        BinaryCase {
            a: with_a.a,
            b: b as i32,
        },
        a_steps + b_steps,
    )
}

fn pbt_options() -> coding_adventures_test_bootstrap::pbt::PbtOptions {
    coding_adventures_test_bootstrap::pbt::PbtOptions {
        num_tests: PROPERTY_CASES,
        max_retries: 100,
        max_shrink_steps: 256,
        is_ci: false,
    }
}

#[test]
fn canonical_seeded_pbt_checks_three_existing_exports() {
    assert_generated_warrant_is_current();
    use coding_adventures_test_bootstrap::pbt::{check_property, gen_bounded_int};
    use coding_adventures_test_bootstrap::prng::SplitMix64;
    use coding_adventures_test_bootstrap::Verdict;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let from_calls = AtomicUsize::new(0);
    let from_verdict = check_property(
        PROPERTY_SEED,
        &pbt_options(),
        |rng| Ok(gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32),
        |value| {
            from_calls.fetch_add(1, Ordering::Relaxed);
            triple_result(
                from_integer(*value),
                oracle_from_integer(*value),
                warrant::q16_from_integer_model(warrant::V::Fixed(
                    warrant::F::I32,
                    i128::from(*value),
                )),
            )
        },
        |initial, fails| {
            let (shrunk, steps) = coding_adventures_test_bootstrap::pbt::shrink_int(
                i128::from(initial),
                0,
                i128::from(i32::MIN),
                i128::from(i32::MAX),
                |candidate| fails(&(candidate as i32)),
                256,
            );
            (shrunk as i32, steps)
        },
    );
    assert_eq!(from_verdict, Verdict::Pass);
    assert_eq!(from_calls.load(Ordering::Relaxed), PROPERTY_CASES);

    for (mode_index, rounding) in MODES.into_iter().enumerate() {
        let calls = AtomicUsize::new(0);
        let verdict = check_property(
            PROPERTY_SEED ^ (mode_index as u64 + 1),
            &pbt_options(),
            |rng: &mut SplitMix64| {
                Ok(BinaryCase {
                    a: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                    b: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                })
            },
            |case| {
                calls.fetch_add(1, Ordering::Relaxed);
                triple_result(
                    checked_div(Q16::from_raw(case.a), Q16::from_raw(case.b), mode(rounding)),
                    oracle_div(case.a, case.b, rounding),
                    warrant::q16_checked_div_model(
                        warrant_q16(case.a),
                        warrant_q16(case.b),
                        warrant_mode(rounding),
                    ),
                )
            },
            shrink_binary,
        );
        assert_eq!(verdict, Verdict::Pass, "checked_div {rounding}");
        assert_eq!(calls.load(Ordering::Relaxed), PROPERTY_CASES);
        // These deterministic controls complement the uniform random draws,
        // which need not hit rare denominator/error strata in 4096 samples.
        assert_eq!(
            checked_div(Q16::from_raw(0), Q16::from_raw(0), mode(rounding)),
            Err(Q16Error::DivisionByZero)
        );
        check_api(
            checked_div(Q16::from_raw(i32::MAX), Q16::from_raw(1), mode(rounding)),
            oracle_div(i32::MAX, 1, rounding),
            "checked_div overflow boundary",
        );
    }

    let add_calls = AtomicUsize::new(0);
    let add_verdict = check_property(
        PROPERTY_SEED ^ 0xADD,
        &pbt_options(),
        |rng: &mut SplitMix64| {
            Ok(BinaryCase {
                a: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                b: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
            })
        },
        |case| {
            add_calls.fetch_add(1, Ordering::Relaxed);
            let checked = checked_add(Q16::from_raw(case.a), Q16::from_raw(case.b));
            let checked_ok = triple_result(
                checked,
                oracle_checked(BigInt::from(case.a) + BigInt::from(case.b)),
                warrant::q16_checked_add_model(warrant_q16(case.a), warrant_q16(case.b)),
            );
            let actual = saturating_add(Q16::from_raw(case.a), Q16::from_raw(case.b));
            checked_ok
                && triple_saturated(
                    actual,
                    oracle_saturating_add(case.a, case.b),
                    warrant::q16_saturating_add_model(warrant_q16(case.a), warrant_q16(case.b)),
                )
        },
        shrink_binary,
    );
    assert_eq!(add_verdict, Verdict::Pass);
    assert_eq!(add_calls.load(Ordering::Relaxed), PROPERTY_CASES);
}

#[test]
fn canonical_pbt_shrinker_has_a_preserving_negative_control() {
    assert_generated_warrant_is_current();
    use coding_adventures_test_bootstrap::pbt::check_property;
    use coding_adventures_test_bootstrap::prng::SplitMix64;
    use coding_adventures_test_bootstrap::Verdict;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let initial = BinaryCase { a: 100, b: -25 };
    let actual_calls = AtomicUsize::new(0);
    let verdict = check_property(
        PROPERTY_SEED,
        &coding_adventures_test_bootstrap::pbt::PbtOptions {
            num_tests: 1,
            ..pbt_options()
        },
        move |_rng: &mut SplitMix64| Ok(initial.clone()),
        |case| {
            actual_calls.fetch_add(1, Ordering::Relaxed);
            let expected = oracle_saturating_add(case.a, case.b);
            let mut faulty_actual = saturating_add(Q16::from_raw(case.a), Q16::from_raw(case.b));
            faulty_actual.value.raw ^= 1;
            check_saturated(faulty_actual, expected)
        },
        shrink_binary,
    );
    assert!(
        matches!(verdict, Verdict::Fail { ref message, .. }
        if message.contains("minimal=BinaryCase { a: 0, b: 0 }") && message.contains("after 2 shrink steps")),
        "the known wrong-result mutation must shrink while retaining failure: {verdict:?}"
    );
    assert_eq!(
        actual_calls.load(Ordering::Relaxed),
        6,
        "initial plus all preserving shrink probes execute the public API"
    );
    let original_expected = oracle_saturating_add(100, -25);
    let minimized_expected = oracle_saturating_add(0, 0);
    assert_eq!(
        original_expected.overflowed, minimized_expected.overflowed,
        "shrunk failure retains the same public result class"
    );
    assert!(!check_saturated(
        {
            let mut actual = saturating_add(Q16::from_raw(100), Q16::from_raw(-25));
            actual.value.raw ^= 1;
            actual
        },
        original_expected
    ));
    assert!(!check_saturated(
        {
            let mut actual = saturating_add(Q16::from_raw(0), Q16::from_raw(0));
            actual.value.raw ^= 1;
            actual
        },
        minimized_expected
    ));
}

#[test]
fn repository_fuzz_engine_runs_typed_export_campaigns() {
    assert_generated_warrant_is_current();
    let mut total_calls = 0_usize;
    let from_classes = RefCell::new([0_usize; 2]);
    let from_seeds = [
        0_i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        32767,
        32768,
        -32768,
        -32769,
    ]
    .into_iter()
    .map(|x| x.to_le_bytes().to_vec())
    .collect::<Vec<_>>();
    let from_calls = std::cell::Cell::new(0_usize);
    let report = fuzz_target(
        &from_seeds,
        &FuzzOptions {
            max_iterations: FUZZ_CASES,
            seed: FUZZ_SEED,
            max_input_len: 4,
            base_energy: 10,
        },
        |bytes, coverage| {
            from_calls.set(from_calls.get() + 1);
            let value = decode_i32(bytes, 0);
            let expected = oracle_from_integer(value);
            from_classes.borrow_mut()[usize::from(expected.is_err())] += 1;
            assert!(
                triple_result(
                    from_integer(value),
                    expected,
                    warrant::q16_from_integer_model(warrant::V::Fixed(
                        warrant::F::I32,
                        i128::from(value)
                    ))
                ),
                "typed from_integer Warrant/API/oracle fuzz"
            );
            coverage.record_edge(usize::from(expected.is_err()));
            FuzzOutcome::Pass
        },
    );
    assert_eq!(report.iterations, FUZZ_CASES);
    assert!(report.is_clean());
    assert_eq!(from_calls.get(), report.iterations + from_seeds.len());
    total_calls += from_calls.get();
    assert!(from_classes.borrow().iter().all(|count| *count > 0));

    for (mode_index, rounding) in MODES.into_iter().enumerate() {
        let classes = RefCell::new([0_usize; 3]);
        let seeds = [
            encode_pair(0, 0),
            encode_pair(1, 1),
            encode_pair(i32::MAX, 1),
            encode_pair(i32::MIN, 1),
            encode_pair(i32::MIN, i32::MIN),
        ];
        let calls = std::cell::Cell::new(0_usize);
        let report = fuzz_target(
            &seeds,
            &FuzzOptions {
                max_iterations: FUZZ_CASES,
                seed: FUZZ_SEED ^ mode_index as u64,
                max_input_len: 8,
                base_energy: 10,
            },
            |bytes, coverage| {
                calls.set(calls.get() + 1);
                let a = decode_i32(bytes, 0);
                let b = decode_i32(bytes, 4);
                let expected = oracle_div(a, b, rounding);
                let class = match expected {
                    Ok(_) => 0,
                    Err(Q16Error::Overflow) => 1,
                    Err(Q16Error::DivisionByZero) => 2,
                };
                classes.borrow_mut()[class] += 1;
                assert!(
                    triple_result(
                        checked_div(Q16::from_raw(a), Q16::from_raw(b), mode(rounding)),
                        expected,
                        warrant::q16_checked_div_model(
                            warrant_q16(a),
                            warrant_q16(b),
                            warrant_mode(rounding)
                        )
                    ),
                    "typed checked_div Warrant/API/oracle fuzz"
                );
                let actual_sat = warrant_api_sat_div(saturating_div(
                    Q16::from_raw(a),
                    Q16::from_raw(b),
                    mode(rounding),
                ));
                let expected_sat = match oracle_saturating_div(a, b, rounding) {
                    Ok(value) => warrant::V::Ok(Box::new(warrant_sat(value))),
                    Err(Q16Error::Overflow) => warrant::V::Err("Overflow".into()),
                    Err(Q16Error::DivisionByZero) => warrant::V::Err("DivisionByZero".into()),
                };
                assert_eq!(
                    actual_sat, expected_sat,
                    "typed saturating_div API/oracle fuzz"
                );
                assert_eq!(
                    actual_sat,
                    warrant::q16_saturating_div_model(
                        warrant_q16(a),
                        warrant_q16(b),
                        warrant_mode(rounding)
                    ),
                    "typed saturating_div Warrant fuzz"
                );
                coverage.record_edge(10 * mode_index + class);
                FuzzOutcome::Pass
            },
        );
        assert_eq!(report.iterations, FUZZ_CASES);
        assert!(report.is_clean());
        assert_eq!(calls.get(), report.iterations + seeds.len());
        total_calls += calls.get();
        assert!(
            classes.borrow()[0] > 0 && classes.borrow()[1] > 0 && classes.borrow()[2] > 0,
            "fuzz div {rounding} strata {:?}",
            *classes.borrow()
        );
    }

    let classes = RefCell::new([0_usize; 2]);
    let seeds = [
        encode_pair(0, 0),
        encode_pair(i32::MAX, 1),
        encode_pair(i32::MIN, -1),
        encode_pair(i32::MAX, i32::MAX),
    ];
    let add_calls = std::cell::Cell::new(0_usize);
    let report = fuzz_target(
        &seeds,
        &FuzzOptions {
            max_iterations: FUZZ_CASES,
            seed: FUZZ_SEED ^ 0xADD,
            max_input_len: 8,
            base_energy: 10,
        },
        |bytes, coverage| {
            add_calls.set(add_calls.get() + 1);
            let a = decode_i32(bytes, 0);
            let b = decode_i32(bytes, 4);
            let expected = oracle_saturating_add(a, b);
            classes.borrow_mut()[usize::from(expected.overflowed)] += 1;
            assert!(
                triple_saturated(
                    saturating_add(Q16::from_raw(a), Q16::from_raw(b)),
                    expected,
                    warrant::q16_saturating_add_model(warrant_q16(a), warrant_q16(b))
                ),
                "typed saturating_add Warrant/API/oracle fuzz"
            );
            assert!(
                triple_result(
                    checked_add(Q16::from_raw(a), Q16::from_raw(b)),
                    oracle_checked(BigInt::from(a) + BigInt::from(b)),
                    warrant::q16_checked_add_model(warrant_q16(a), warrant_q16(b))
                ),
                "typed checked_add Warrant/API/oracle fuzz"
            );
            coverage.record_edge(30 + usize::from(expected.overflowed));
            FuzzOutcome::Pass
        },
    );
    assert_eq!(report.iterations, FUZZ_CASES);
    assert!(report.is_clean());
    assert_eq!(add_calls.get(), report.iterations + seeds.len());
    total_calls += add_calls.get();
    assert!(classes.borrow().iter().all(|count| *count > 0));
    assert_eq!(
        total_calls,
        (FUZZ_CASES + from_seeds.len()) + 4 * (FUZZ_CASES + 5) + (FUZZ_CASES + seeds.len()),
        "actual callbacks include initial-corpus triage",
    );
}

#[test]
fn canonical_pbt_checks_every_operation_and_rounding_mode() {
    assert_generated_warrant_is_current();
    use coding_adventures_test_bootstrap::pbt::{check_property, gen_bounded_int};
    use coding_adventures_test_bootstrap::prng::SplitMix64;
    use coding_adventures_test_bootstrap::Verdict;
    use std::sync::atomic::{AtomicUsize, Ordering};

    for (op_index, op) in ["add", "sub"].into_iter().enumerate() {
        let calls = AtomicUsize::new(0);
        let verdict = check_property(
            PROPERTY_SEED ^ (0x100 + op_index as u64),
            &pbt_options(),
            |rng: &mut SplitMix64| {
                Ok(BinaryCase {
                    a: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                    b: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                })
            },
            |case| {
                calls.fetch_add(1, Ordering::Relaxed);
                let (exact, checked, saturated, checked_model, saturated_model) = if op == "add" {
                    (
                        BigInt::from(case.a) + BigInt::from(case.b),
                        checked_add(Q16::from_raw(case.a), Q16::from_raw(case.b)),
                        saturating_add(Q16::from_raw(case.a), Q16::from_raw(case.b)),
                        warrant::q16_checked_add_model(warrant_q16(case.a), warrant_q16(case.b)),
                        warrant::q16_saturating_add_model(warrant_q16(case.a), warrant_q16(case.b)),
                    )
                } else {
                    (
                        BigInt::from(case.a) - BigInt::from(case.b),
                        checked_sub(Q16::from_raw(case.a), Q16::from_raw(case.b)),
                        saturating_sub(Q16::from_raw(case.a), Q16::from_raw(case.b)),
                        warrant::q16_checked_sub_model(warrant_q16(case.a), warrant_q16(case.b)),
                        warrant::q16_saturating_sub_model(warrant_q16(case.a), warrant_q16(case.b)),
                    )
                };
                triple_result(checked, oracle_checked(exact.clone()), checked_model)
                    && triple_saturated(saturated, oracle_saturating(exact), saturated_model)
            },
            shrink_binary,
        );
        assert_eq!(verdict, Verdict::Pass, "{op}");
        assert_eq!(
            calls.load(Ordering::Relaxed),
            PROPERTY_CASES,
            "{op} callbacks"
        );
    }

    for (mode_index, rounding) in MODES.into_iter().enumerate() {
        let mul_calls = AtomicUsize::new(0);
        let mul_verdict = check_property(
            PROPERTY_SEED ^ (0x200 + mode_index as u64),
            &pbt_options(),
            |rng: &mut SplitMix64| {
                Ok(BinaryCase {
                    a: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                    b: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                })
            },
            |case| {
                mul_calls.fetch_add(1, Ordering::Relaxed);
                let exact = rounded(
                    BigInt::from(case.a) * BigInt::from(case.b),
                    BigInt::from(SCALE),
                    rounding,
                );
                triple_result(
                    checked_mul(Q16::from_raw(case.a), Q16::from_raw(case.b), mode(rounding)),
                    oracle_checked(exact.clone()),
                    warrant::q16_checked_mul_model(
                        warrant_q16(case.a),
                        warrant_q16(case.b),
                        warrant_mode(rounding),
                    ),
                ) && triple_saturated(
                    saturating_mul(Q16::from_raw(case.a), Q16::from_raw(case.b), mode(rounding)),
                    oracle_saturating(exact),
                    warrant::q16_saturating_mul_model(
                        warrant_q16(case.a),
                        warrant_q16(case.b),
                        warrant_mode(rounding),
                    ),
                )
            },
            shrink_binary,
        );
        assert_eq!(
            mul_verdict,
            Verdict::Pass,
            "checked/saturating_mul {rounding}"
        );
        assert_eq!(mul_calls.load(Ordering::Relaxed), PROPERTY_CASES);

        let div_calls = AtomicUsize::new(0);
        let div_verdict = check_property(
            PROPERTY_SEED ^ (0x300 + mode_index as u64),
            &pbt_options(),
            |rng: &mut SplitMix64| {
                Ok(BinaryCase {
                    a: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                    b: gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
                })
            },
            |case| {
                div_calls.fetch_add(1, Ordering::Relaxed);
                let checked =
                    checked_div(Q16::from_raw(case.a), Q16::from_raw(case.b), mode(rounding));
                let expected = oracle_div(case.a, case.b, rounding);
                triple_result(
                    checked,
                    expected,
                    warrant::q16_checked_div_model(
                        warrant_q16(case.a),
                        warrant_q16(case.b),
                        warrant_mode(rounding),
                    ),
                ) && {
                    let actual = saturating_div(
                        Q16::from_raw(case.a),
                        Q16::from_raw(case.b),
                        mode(rounding),
                    );
                    let oracle = oracle_saturating_div(case.a, case.b, rounding);
                    let model = warrant::q16_saturating_div_model(
                        warrant_q16(case.a),
                        warrant_q16(case.b),
                        warrant_mode(rounding),
                    );
                    let actual_v = warrant_api_sat_div(actual);
                    actual_v == model
                        && actual_v
                            == match oracle {
                                Ok(value) => warrant::V::Ok(Box::new(warrant_sat(value))),
                                Err(Q16Error::Overflow) => warrant::V::Err("Overflow".into()),
                                Err(Q16Error::DivisionByZero) => {
                                    warrant::V::Err("DivisionByZero".into())
                                }
                            }
                }
            },
            shrink_binary,
        );
        assert_eq!(
            div_verdict,
            Verdict::Pass,
            "checked/saturating_div {rounding}"
        );
        assert_eq!(div_calls.load(Ordering::Relaxed), PROPERTY_CASES);

        let to_calls = AtomicUsize::new(0);
        let to_verdict = check_property(
            PROPERTY_SEED ^ (0x400 + mode_index as u64),
            &pbt_options(),
            |rng| Ok(gen_bounded_int(rng, i128::from(i32::MIN), i128::from(i32::MAX)) as i32),
            |raw| {
                to_calls.fetch_add(1, Ordering::Relaxed);
                let actual = to_integer(Q16::from_raw(*raw), mode(rounding));
                let oracle = rounded(BigInt::from(*raw), BigInt::from(SCALE), rounding)
                    .to_i32()
                    .unwrap();
                actual == oracle
                    && warrant::V::Fixed(warrant::F::I32, i128::from(actual))
                        == warrant::q16_to_integer_model(warrant_q16(*raw), warrant_mode(rounding))
            },
            |initial, fails| {
                let (value, steps) = coding_adventures_test_bootstrap::pbt::shrink_int(
                    i128::from(initial),
                    0,
                    i128::from(i32::MIN),
                    i128::from(i32::MAX),
                    |candidate| fails(&(candidate as i32)),
                    256,
                );
                (value as i32, steps)
            },
        );
        assert_eq!(to_verdict, Verdict::Pass, "to_integer {rounding}");
        assert_eq!(to_calls.load(Ordering::Relaxed), PROPERTY_CASES);
    }
}

#[test]
fn repository_fuzz_engine_covers_sub_mul_and_conversion_exports() {
    assert_generated_warrant_is_current();
    let mut callbacks = 0_usize;
    let pair_seeds = [
        encode_pair(0, 0),
        encode_pair(i32::MAX, 1),
        encode_pair(i32::MIN, 1),
        encode_pair(i32::MIN, -1),
        encode_pair(i32::MAX, i32::MAX),
        encode_pair(-1, 131_072),
        encode_pair(3, 131_072),
    ];
    for op_index in 0..1 {
        let calls = std::cell::Cell::new(0_usize);
        let classes = RefCell::new([0_usize; 2]);
        let report = fuzz_target(
            &pair_seeds,
            &FuzzOptions {
                max_iterations: FUZZ_CASES,
                seed: FUZZ_SEED ^ (0x500 + op_index as u64),
                max_input_len: 8,
                base_energy: 10,
            },
            |bytes, coverage| {
                calls.set(calls.get() + 1);
                let a = decode_i32(bytes, 0);
                let b = decode_i32(bytes, 4);
                let exact = BigInt::from(a) - BigInt::from(b);
                let checked = oracle_checked(exact.clone());
                let saturated = oracle_saturating(exact);
                let matches = triple_result(
                    checked_sub(Q16::from_raw(a), Q16::from_raw(b)),
                    checked,
                    warrant::q16_checked_sub_model(warrant_q16(a), warrant_q16(b)),
                ) && triple_saturated(
                    saturating_sub(Q16::from_raw(a), Q16::from_raw(b)),
                    saturated,
                    warrant::q16_saturating_sub_model(warrant_q16(a), warrant_q16(b)),
                );
                assert!(matches, "typed sub fuzz: {a}, {b}");
                classes.borrow_mut()[usize::from(saturated.overflowed)] += 1;
                coverage.record_edge(40 + usize::from(saturated.overflowed));
                FuzzOutcome::Pass
            },
        );
        assert_eq!(report.iterations, FUZZ_CASES);
        assert!(report.is_clean());
        assert_eq!(calls.get(), FUZZ_CASES + pair_seeds.len());
        assert!(classes.borrow()[0] > 0 && classes.borrow()[1] > 0);
        callbacks += calls.get();
    }

    for (mode_index, rounding) in MODES.into_iter().enumerate() {
        let mul_calls = std::cell::Cell::new(0_usize);
        let mul_classes = RefCell::new([0_usize; 2]);
        let report = fuzz_target(
            &pair_seeds,
            &FuzzOptions {
                max_iterations: FUZZ_CASES,
                seed: FUZZ_SEED ^ (0x600 + mode_index as u64),
                max_input_len: 8,
                base_energy: 10,
            },
            |bytes, coverage| {
                mul_calls.set(mul_calls.get() + 1);
                let a = decode_i32(bytes, 0);
                let b = decode_i32(bytes, 4);
                let exact = rounded(
                    BigInt::from(a) * BigInt::from(b),
                    BigInt::from(SCALE),
                    rounding,
                );
                let checked = oracle_checked(exact.clone());
                let sat = oracle_saturating(exact);
                assert!(
                    triple_result(
                        checked_mul(Q16::from_raw(a), Q16::from_raw(b), mode(rounding)),
                        checked,
                        warrant::q16_checked_mul_model(
                            warrant_q16(a),
                            warrant_q16(b),
                            warrant_mode(rounding)
                        ),
                    ) && triple_saturated(
                        saturating_mul(Q16::from_raw(a), Q16::from_raw(b), mode(rounding)),
                        sat,
                        warrant::q16_saturating_mul_model(
                            warrant_q16(a),
                            warrant_q16(b),
                            warrant_mode(rounding)
                        ),
                    ),
                    "typed mul {rounding} fuzz: {a}, {b}"
                );
                mul_classes.borrow_mut()[usize::from(sat.overflowed)] += 1;
                coverage.record_edge(50 + 2 * mode_index + usize::from(sat.overflowed));
                FuzzOutcome::Pass
            },
        );
        assert_eq!(report.iterations, FUZZ_CASES);
        assert!(report.is_clean());
        assert_eq!(mul_calls.get(), FUZZ_CASES + pair_seeds.len());
        assert!(mul_classes.borrow()[0] > 0 && mul_classes.borrow()[1] > 0);
        callbacks += mul_calls.get();

        let div_calls = std::cell::Cell::new(0_usize);
        let div_classes = RefCell::new([0_usize; 3]);
        let report = fuzz_target(
            &pair_seeds,
            &FuzzOptions {
                max_iterations: FUZZ_CASES,
                seed: FUZZ_SEED ^ (0x700 + mode_index as u64),
                max_input_len: 8,
                base_energy: 10,
            },
            |bytes, coverage| {
                div_calls.set(div_calls.get() + 1);
                let a = decode_i32(bytes, 0);
                let b = decode_i32(bytes, 4);
                let checked = oracle_div(a, b, rounding);
                let sat = oracle_saturating_div(a, b, rounding);
                let actual_checked =
                    checked_div(Q16::from_raw(a), Q16::from_raw(b), mode(rounding));
                let actual_sat = saturating_div(Q16::from_raw(a), Q16::from_raw(b), mode(rounding));
                let checked_model = warrant::q16_checked_div_model(
                    warrant_q16(a),
                    warrant_q16(b),
                    warrant_mode(rounding),
                );
                let sat_model = warrant::q16_saturating_div_model(
                    warrant_q16(a),
                    warrant_q16(b),
                    warrant_mode(rounding),
                );
                let matches = triple_result(actual_checked, checked, checked_model)
                    && warrant_api_sat_div(actual_sat) == sat_model
                    && warrant_api_sat_div(actual_sat)
                        == match sat {
                            Ok(x) => warrant::V::Ok(Box::new(warrant_sat(x))),
                            Err(Q16Error::Overflow) => warrant::V::Err("Overflow".into()),
                            Err(Q16Error::DivisionByZero) => {
                                warrant::V::Err("DivisionByZero".into())
                            }
                        };
                assert!(matches, "typed div {rounding} fuzz: {a}, {b}");
                let class = match checked {
                    Ok(_) => 0,
                    Err(Q16Error::Overflow) => 1,
                    Err(Q16Error::DivisionByZero) => 2,
                };
                div_classes.borrow_mut()[class] += 1;
                coverage.record_edge(60 + 3 * mode_index + class);
                FuzzOutcome::Pass
            },
        );
        assert_eq!(report.iterations, FUZZ_CASES);
        assert!(report.is_clean());
        assert_eq!(div_calls.get(), FUZZ_CASES + pair_seeds.len());
        assert!(
            div_classes.borrow()[0] > 0
                && div_classes.borrow()[1] > 0
                && div_classes.borrow()[2] > 0
        );
        callbacks += div_calls.get();

        let to_seeds = [0_i32, 1, -1, 32_767, 32_768, i32::MAX, i32::MIN]
            .into_iter()
            .map(|x| x.to_le_bytes().to_vec())
            .collect::<Vec<_>>();
        let to_calls = std::cell::Cell::new(0_usize);
        let report = fuzz_target(
            &to_seeds,
            &FuzzOptions {
                max_iterations: FUZZ_CASES,
                seed: FUZZ_SEED ^ (0x800 + mode_index as u64),
                max_input_len: 4,
                base_energy: 10,
            },
            |bytes, coverage| {
                to_calls.set(to_calls.get() + 1);
                let raw = decode_i32(bytes, 0);
                let expected = rounded(BigInt::from(raw), BigInt::from(SCALE), rounding)
                    .to_i32()
                    .unwrap();
                let actual = to_integer(Q16::from_raw(raw), mode(rounding));
                assert_eq!(actual, expected, "typed to_integer {rounding} fuzz: {raw}");
                assert_eq!(
                    warrant::V::Fixed(warrant::F::I32, i128::from(actual)),
                    warrant::q16_to_integer_model(warrant_q16(raw), warrant_mode(rounding)),
                    "Warrant/API to_integer fuzz"
                );
                coverage.record_edge(80 + mode_index);
                FuzzOutcome::Pass
            },
        );
        assert_eq!(report.iterations, FUZZ_CASES);
        assert!(report.is_clean());
        assert_eq!(to_calls.get(), FUZZ_CASES + to_seeds.len());
        callbacks += to_calls.get();
    }
    assert_eq!(
        callbacks,
        9 * (FUZZ_CASES + pair_seeds.len()) + 4 * (FUZZ_CASES + 7)
    );
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        let actual: std::collections::BTreeSet<_> = object.keys().map(String::as_str).collect();
        let expected: std::collections::BTreeSet<_> = expected.iter().copied().collect();
        actual == expected
    })
}

/// Strict test-side decoder for fixture-shaped adapter bytes. A refusal is not
/// an arithmetic outcome; accepted documents must still match the oracle.
fn fixture_bytes_admitted(bytes: &[u8]) -> bool {
    let Ok(fixture) = serde_json::from_slice::<Value>(bytes) else {
        return false;
    };
    if !exact_keys(
        &fixture,
        &[
            "schema",
            "scale",
            "raw_min",
            "raw_max",
            "rounding_modes",
            "vector_count",
            "vectors",
        ],
    ) || fixture["schema"] != "coding-adventures/fixed-point-q16-v1"
        || fixture["scale"].as_i64() != Some(SCALE)
        || fixture["raw_min"].as_i64() != Some(i64::from(i32::MIN))
        || fixture["raw_max"].as_i64() != Some(i64::from(i32::MAX))
        || fixture["rounding_modes"] != json!(MODES)
    {
        return false;
    }
    let Some(vectors) = fixture["vectors"].as_array() else {
        return false;
    };
    if vectors.is_empty() || fixture["vector_count"].as_u64() != Some(vectors.len() as u64) {
        return false;
    }
    let mut ids = std::collections::BTreeSet::new();
    for vector in vectors {
        let Some(id) = vector["id"].as_str() else {
            return false;
        };
        if id.is_empty() || !ids.insert(id) {
            return false;
        }
        let Some(operation) = vector["operation"].as_str() else {
            return false;
        };
        let input = &vector["input"];
        let expected_fields: &[&str] = match operation {
            "from_integer" => &["integer"],
            "to_integer" => &["raw"],
            "add" | "sub" | "mul" | "div" => &["a_raw", "b_raw"],
            _ => return false,
        };
        if !exact_keys(input, expected_fields) {
            return false;
        }
        for field in expected_fields {
            let Some(value) = input[*field].as_i64() else {
                return false;
            };
            if !(i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&value) {
                return false;
            }
        }
        let by_mode = matches!(operation, "to_integer" | "mul" | "div");
        if by_mode {
            if !exact_keys(vector, &["id", "operation", "input", "expected_by_mode"])
                || !exact_keys(&vector["expected_by_mode"], &MODES)
            {
                return false;
            }
            for rounding in MODES {
                if vector["expected_by_mode"][rounding]
                    != operation_expected(vector, Some(rounding))
                {
                    return false;
                }
            }
        } else {
            if !exact_keys(vector, &["id", "operation", "input", "expected"])
                || vector["expected"] != operation_expected(vector, None)
            {
                return false;
            }
        }
    }
    true
}

#[test]
fn malformed_serialized_fixture_campaign_is_separate_from_arithmetic_fuzz() {
    use coding_adventures_test_bootstrap::fuzz::fuzz_target;
    const SEED: u64 = 0x5146_3136_4d41_4c46;
    const ITERATIONS: usize = 4096;
    let valid_minimal = br#"{"schema":"coding-adventures/fixed-point-q16-v1","scale":65536,"raw_min":-2147483648,"raw_max":2147483647,"rounding_modes":["TowardZero","Floor","Ceil","NearestEven"],"vector_count":1,"vectors":[{"id":"zero","operation":"from_integer","input":{"integer":0},"expected":{"ok_raw":0}}]}"#.to_vec();
    let seeds = vec![
        valid_minimal,
        b"".to_vec(),
        b"{".to_vec(),
        b"[]".to_vec(),
        br#"{"schema":"wrong"}"#.to_vec(),
        br#"{"schema":"coding-adventures/fixed-point-q16-v1","scale":65536,"raw_min":-2147483648,"raw_max":2147483647,"rounding_modes":["TowardZero","Floor","Ceil","NearestEven"],"vector_count":1,"vectors":[{"id":"bad","operation":"from_integer","input":{"integer":1.0},"expected":{"ok_raw":65536}}]}"#.to_vec(),
    ];
    let calls = std::cell::Cell::new(0_usize);
    let outcomes = RefCell::new([0_usize; 2]); // accepted, refused
    let report = fuzz_target(
        &seeds,
        &FuzzOptions {
            max_iterations: ITERATIONS,
            seed: SEED,
            max_input_len: 512,
            base_energy: 10,
        },
        |input, coverage| {
            calls.set(calls.get() + 1);
            let admitted = fixture_bytes_admitted(input);
            outcomes.borrow_mut()[usize::from(!admitted)] += 1;
            coverage.record_edge(usize::from(!admitted));
            FuzzOutcome::Pass
        },
    );
    assert_eq!(report.iterations, ITERATIONS);
    assert!(report.is_clean());
    assert_eq!(calls.get(), ITERATIONS + seeds.len());
    assert!(
        outcomes.borrow()[0] > 0 && outcomes.borrow()[1] > 0,
        "fixture byte outcomes {:?}",
        *outcomes.borrow()
    );
    assert!(
        fixture_bytes_admitted(FIXTURE.as_bytes()),
        "canonical full fixture must pass strict adapter gate"
    );
}
