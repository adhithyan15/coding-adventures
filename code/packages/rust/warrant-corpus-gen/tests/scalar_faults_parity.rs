//! The S11f-2 acceptance test: the generator reproduces, byte for byte, the oracle's committed proof of
//! the faults `saturate-add`, `saturate-sub`, `shift-mask` and the scalar `missing-error` (VER04r 8.2,
//! 16.14 [D50], [D51], R31, R33).
//!
//! `python/warrant-oracle` committed, in slice S11f-1, a synthetic document, its `.profile` and two dumps in
//! `tests/fixtures/scalar-faults-v1/`:
//!
//! | dump | schema | what this test reproduces it from |
//! |---|---|---|
//! | `.oracle-faults.json` | `warrant-fault-parity-v1` | the fault rows, and the controls under the explicit closed gate (`GATE_CLOSED`) and the open gate (`GATE_OPEN`) |
//! | `.oracle-refusals.json` | `warrant-faults-refusals-v2` | each refused signature: the generator code `fault` and the oracle's reason (a top-level `faults` list, a `fault` per row) |
//!
//! The files are read from the oracle's directory (no copy).

mod support;

use coding_adventures_warrant_corpus_gen::error::code;
use coding_adventures_warrant_corpus_gen::faults::instantiate;
use coding_adventures_warrant_corpus_gen::json::Json;
use coding_adventures_warrant_corpus_gen::lower::lower_document;
use support::minijson::parse;
use support::parity::*;
use support::*;

const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.warrant"
);
const PROFILE: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.profile"
);
const FAULTS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.oracle-faults.json"
);
const REFUSALS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.oracle-refusals.json"
);

/// The specification's path as written into `corpus.json`.
const SPEC: &str = "scalar_faults_v1.warrant";

/// `warrant-fault-parity-v1`: the fault rows, the controls under the closed gate and under the open
/// gate (the oracle's `parity_dumps.faults_view`, the gates named explicitly).
fn faults_view() -> Json {
    let closed = generated(WARRANT, SPEC, PROFILE, &GATE_CLOSED).expect("generates (closed gate)");
    let open = generated(WARRANT, SPEC, PROFILE, &GATE_OPEN).expect("generates (open gate)");
    let s = parse(&closed.corpus_json).expect("parses");
    let g = parse(&open.corpus_json).expect("parses");
    assert_eq!(s.get("faults"), g.get("faults"), "the fault rows never depend on the gate");
    Json::obj([
        ("schema", Json::str("warrant-fault-parity-v1")),
        ("module", to_json(s.get("module"))),
        ("faults", to_json(s.get("faults"))),
        ("controls_shipped", to_json(s.get("controls"))),
        ("controls_gate_on", to_json(g.get("controls"))),
        ("accepted_blind_faults", to_json(s.get("accepted_blind_faults"))),
        ("levels_shipped", to_json(s.get("levels"))),
        ("levels_gate_on", to_json(g.get("levels"))),
        ("gate_shipped", Json::strs(GATE_CLOSED.iter().map(|l| l.name()))),
        ("gate_on", Json::strs(GATE_OPEN.iter().map(|l| l.name()))),
    ])
}

#[test]
fn the_fault_rows_and_controls_match_the_oracle_fault_dump_byte_for_byte() {
    let text = faults_view().render().expect("render");
    assert!(text == FAULTS, "{}", first_difference(&text, FAULTS));
}

#[test]
fn the_dump_is_not_vacuous() {
    assert!(FAULTS.contains("\"warrant-fault-parity-v1\""));
    let rows = parse(FAULTS).expect("parses");
    let faults = rows.get("faults").a();
    assert_eq!(faults.len(), 41);
    for name in ["saturate-add", "saturate-sub", "shift-mask", "missing-error"] {
        assert!(faults.iter().any(|r| r.get("fault").s() == name), "{name}");
    }
    assert_eq!(rows.get("controls_shipped").a().len(), 41);
    assert_eq!(rows.get("controls_gate_on").a().len(), 41);
}

/// The contract boilerplate for the operation `f` of a refusal document, so the S2 checker reaches the
/// lowering: `f` has one parameter, named after the first one of the signature (none: no `input`).
fn contracts(params: &[&str]) -> String {
    let mut c = String::from("  assume platform:\n    none\n  assume environment:\n    none\n");
    if !params.is_empty() {
        c.push_str("  labels f:\n");
        for p in params {
            c.push_str(&format!("    {p} secret\n"));
        }
        for p in params {
            c.push_str(&format!("  input f.{p}:\n    hostile\n    ensures no_panic, terminates\n"));
        }
    }
    c.push_str("  bound f:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    c
}

/// The parameter names of `op f(..)` in a refusal document (a generic header `f<N: Nat>` is skipped).
fn param_names(source: &str) -> Vec<String> {
    let sig = source.split("op f").nth(1).expect("an operation f");
    let sig = &sig[sig.find('(').expect("(") + 1..];
    let sig = &sig[..sig.find(')').expect(")")];
    sig.split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.split(':').next().expect("name").trim().to_string())
        .collect()
}

/// The operation `f` of a refusal document, lowered, or the lowering's refusal.
fn lowered_op(
    source: &str,
) -> Result<
    coding_adventures_warrant_corpus_gen::lower::LoweredOp,
    coding_adventures_warrant_corpus_gen::error::GenError,
> {
    let names = param_names(source);
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let src = format!("{source}{}", contracts(&refs));
    with_file(&src, |f| lower_document(f).map(|l| l.op("f").expect("op f").clone()))
}

/// The reason after the oracle's `fault: ` prefix.
fn why(message: &str) -> &str {
    message.strip_prefix("fault: ").expect("the oracle's message starts `fault: `")
}

#[test]
fn every_refused_signature_has_the_generator_code_fault_and_the_oracles_reason() {
    let dump = parse(REFUSALS).expect("the refusals dump parses");
    assert_eq!(dump.get("schema").s(), "warrant-faults-refusals-v2");
    let names: Vec<&str> = dump.get("faults").a().iter().map(|f| f.s()).collect();
    assert_eq!(names, ["saturate-add", "saturate-sub", "shift-mask", "missing-error"]);
    let rows = dump.get("refusals").a();
    assert_eq!(rows.len(), 53);
    let mut instantiated = 0;
    let mut by_lowering = Vec::new();
    for r in rows {
        let name = r.get("name").s();
        let fault = r.get("fault").s();
        assert!(names.contains(&fault), "{name}: {fault}");
        assert_eq!(r.get("oracle_code").s(), "generation", "{name}");
        let op = match lowered_op(r.get("source").s()) {
            Ok(op) => op,
            Err(e) => {
                // An `Option` or a variant never reaches the fault: the generator's lowering refuses the
                // signature itself (both are IR-only or `ir.data`), which is no weaker.
                assert!(
                    name.starts_with("an-option-") || name == "a-variant-payload",
                    "{fault}/{name}: {e}"
                );
                assert_eq!(e.code, code::LEVEL, "{e}");
                assert_eq!(e.level, Some("ir.data"), "{e}");
                by_lowering.push(format!("{fault}/{name}"));
                continue;
            }
        };
        let e = instantiate(&op, fault).expect_err(name);
        assert_eq!(e.code, code::FAULT, "{fault}/{name}: {e}");
        assert_eq!(
            e.message,
            format!(
                "the fault cannot be instantiated on this operation: {}",
                why(r.get("message").s())
            ),
            "{fault}/{name}: generator vs oracle `{}`",
            r.get("message").s()
        );
        instantiated += 1;
    }
    assert_eq!(instantiated + by_lowering.len(), 53);
    // Only the `Option` and variant signatures are stopped earlier, by the lowering.
    assert!(by_lowering.len() <= 6, "{by_lowering:?}");
}

// ---- unit tests of each new form (exact compact JSON, VER04r 8.2) -----------------------------

/// The compact JSON of `fault` on `op f(<params>) -> <ret>` (an error type `E` is declared).
fn form(params: &str, ret: &str, fault: &str) -> String {
    let src = format!("warrant r v1\n  error E = A | B\n  op f({params}) -> {ret}\n");
    let op = lowered_op(&src).expect("lowers");
    let f = instantiate(&op, fault).expect("instantiates");
    f.to_json().render().expect("render").split_whitespace().collect()
}

fn refusal(params: &str, ret: &str, fault: &str) -> String {
    let src = format!("warrant r v1\n  error E = A | B\n  op f({params}) -> {ret}\n");
    let op = lowered_op(&src).expect("lowers");
    let e = instantiate(&op, fault).expect_err("refused");
    assert_eq!(e.code, code::FAULT, "{e}");
    e.message
}

#[test]
fn saturate_add_and_sub_follow_the_four_rows_of_the_spec() {
    let a = r#"["var","a"],["var","b"]"#;
    assert_eq!(
        form("a: U8, b: U8", "U8", "saturate-add"),
        format!(r#"{{"expr":["unwrap_or",["checked_add",{a}],["max_value","U8"]],"fault":"saturate-add","kind":"replace","op":"f","requires":["ir.fixed"]}}"#)
    );
    assert_eq!(
        form("a: U8, b: U8", "Result<U8,E>", "saturate-sub"),
        format!(r#"{{"expr":["ok",["unwrap_or",["checked_sub",{a}],["min_value","U8"]]],"fault":"saturate-sub","kind":"replace","op":"f","requires":["ir.fixed"]}}"#)
    );
    let neg = r#"["lt",["var","b"],["lit",{"type":"I16","value":"0"}]]"#;
    assert_eq!(
        form("a: I16, b: I16", "I16", "saturate-add"),
        format!(r#"{{"expr":["unwrap_or",["checked_add",{a}],["if",{neg},["min_value","I16"],["max_value","I16"]]],"fault":"saturate-add","kind":"replace","op":"f","requires":["ir.fixed"]}}"#)
    );
    assert_eq!(
        form("a: I16, b: I16, c: Bool", "Result<I16,E>", "saturate-sub"),
        format!(r#"{{"expr":["ok",["unwrap_or",["checked_sub",{a}],["if",{neg},["max_value","I16"],["min_value","I16"]]]],"fault":"saturate-sub","kind":"replace","op":"f","requires":["ir.fixed"]}}"#)
    );
}

#[test]
fn shift_mask_masks_the_last_parameter_by_the_first_parameters_width() {
    // unsigned last parameter: `rem`, the literals in the last parameter's type, `w` = 32 from `U32`
    assert_eq!(
        form("a: U32, n: U8", "U8", "shift-mask"),
        r#"{"fault":"shift-mask","kind":"pre","op":"f","requires":["ir.fixed"],"rewrites":[{"expr":["unwrap_or",["rem",["var","n"],["lit",{"type":"U8","value":"32"}]],["lit",{"type":"U8","value":"0"}]],"param":"n"}]}"#
    );
    // signed last parameter: `rem_trunc`
    assert!(form("a: U8, n: I8", "I8", "shift-mask")
        .contains(r#"["rem_trunc",["var","n"],["lit",{"type":"I8","value":"8"}]]"#));
    // one parameter: it is both the first and the last
    assert!(form("a: U16", "U16", "shift-mask")
        .contains(r#"["rem",["var","a"],["lit",{"type":"U16","value":"16"}]]"#));
}

#[test]
fn scalar_missing_error_takes_the_first_parameter_of_exactly_the_type_else_zero() {
    let me = |payload: &str, params: &str| form(params, &format!("Result<{payload},E>"), "missing-error");
    let tail = r#"["var","result"]],"fault":"missing-error","kind":"replace","op":"f","requires":["ir.core"]}"#;
    assert_eq!(
        me("U8", "a: U8, b: U32"),
        format!(r#"{{"expr":["if",["is_err",["var","result"]],["ok",["var","a"]],{tail}"#)
    );
    // a U8 payload skips the U32 parameter: exactly the type, no widening
    assert!(me("U8", "b: U32").contains(r#"["ok",["lit",{"type":"U8","value":"0"}]]"#));
    assert!(me("Bool", "a: U8").contains(r#"["ok",["lit",false]]"#));
    assert!(me("Nat", "n: Nat").contains(r#"["ok",["var","n"]]"#));
    assert!(me("Nat", "a: Bool").contains(r#"["ok",["lit",{"type":"Nat","value":"0"}]]"#));
    // a Bytes payload with no Bytes parameter is the zero literal (refused before S11f-2)
    assert!(me("Bytes<4>", "a: U8").contains(r#"["ok",["lit",{"type":"Bytes","value":"00000000"}]]"#));
    assert!(me("Bytes", "a: U8").contains(r#"["ok",["lit",{"type":"Bytes","value":""}]]"#));
    // a Bytes payload with a Bytes parameter is unchanged: the first one, any width
    assert!(me("Bytes<4>", "a: Bytes<2>").contains(r#"["ok",["var","a"]]"#));
}

#[test]
fn the_new_templates_refuse_with_the_oracles_wording() {
    assert_eq!(
        refusal("a: Nat, b: U8", "U8", "saturate-sub"),
        "the fault cannot be instantiated on this operation: saturate-sub needs two leading parameters of one fixed-width type"
    );
    assert_eq!(
        refusal("a: U8, b: U8", "Result<U16,E>", "saturate-add"),
        "the fault cannot be instantiated on this operation: saturate-add needs a result of the parameters' type or a Result of it"
    );
    assert!(refusal("", "U8", "shift-mask").ends_with("shift-mask needs parameters"));
    assert!(refusal("a: U8, n: Nat", "U8", "shift-mask")
        .ends_with("shift-mask needs a fixed-width first and last parameter"));
    assert!(refusal("a: U8", "Bool", "missing-error").ends_with("missing-error needs a Result result"));
    assert!(refusal("a: U8, b: U8", "Result<(U8,U8),E>", "saturate-add").ends_with("or a Result of it"));
}
