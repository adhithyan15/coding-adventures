//! Targeted tests of the `ir.tuple` slice (S10t-2) on small synthetic documents: typing, the
//! static-width rule, cost, requires, corpus errors, fault templates and the gate.  The oracle
//! parity of the big document is `tuple_parity.rs`; these pin the rules one at a time.

mod support;

use coding_adventures_warrant_corpus_gen::cost::{ir_cost, ModelCosts};
use coding_adventures_warrant_corpus_gen::faults::{
    first_unimplemented, instantiate, instantiate_in, Level, SHIPPED_GATE,
};
use coding_adventures_warrant_corpus_gen::lower::{lower_document, Lowered};
use coding_adventures_warrant_corpus_gen::stream::{encode, in_v1_set};
use coding_adventures_warrant_corpus_gen::types::Ty;
use coding_adventures_warrant_corpus_gen::value::Value;
use coding_adventures_warrant_corpus_gen::{code, GenError};
use support::*;

const OPS: [&str; 1] = ["op f(a: Bytes, b: Bytes) -> Bool"];

fn lower(extra: &str) -> Result<Lowered, GenError> {
    let src = doc_ops(" v1", &OPS, &format!("  error E = X | Y\n{extra}"));
    with_file(&src, |f| lower_document(f))
}

fn json(l: &Lowered, model: &str) -> String {
    l.model(model).unwrap().body.to_json().render().unwrap().split_whitespace().collect()
}

#[test]
fn concat_literals_and_if_of_sized_operands_flow_to_bytes_n_everywhere() {
    let ok = "  model w1(a: Bytes<2>) -> Bytes<4> = concat(a, b\"0102\")\n  model w2(c: Bool) -> Bytes<2> = if c then concat(b\"00\", b\"01\") else concat(b\"02\", b\"03\")\n  model take4(x: Bytes<4>) -> Bool = true\n  model w4(a: Bytes<2>) -> Bool = take4(concat(a, b\"0102\"))\n";
    let l = lower(ok).expect("static widths flow");
    assert_eq!(l.models.len(), 4);
    // The oracle's rule is on the `Bytes` expression itself, not through a `Result`: the Ok payload
    // flows, but the `Result<Bytes,E>` it builds is not assignable to `Result<Bytes<3>,E>`.  The
    // generator mirrors that (the oracle is the reference).
    assert!(lower("  model w3(a: Bytes<1>) -> Result<Bytes<3>, E> = Ok(concat(a, b\"0102\"))\n").is_err());
    // a wrong width does not
    for bad in [
        "  model b1(a: Bytes<2>) -> Bytes<5> = concat(a, b\"0102\")\n",
        "  model b2(a: Bytes) -> Bytes<4> = concat(a, b\"0102\")\n",
        "  model b3(c: Bool) -> Bytes<2> = if c then concat(b\"00\", b\"01\") else b\"0203ff\"\n",
    ] {
        let e = lower(bad).expect_err(bad);
        assert!(e.code == code::TYPE || e.code == code::CHECK, "{bad}: {e}");
    }
}

#[test]
fn tuples_lower_with_a_typed_literal_context_and_projection_chains() {
    let l = lower("  model p(t: ((Bool,Nat),Bool)) -> Nat = t.0.1\n  model q() -> (Nat,Nat) = (1, 2)\n  model r(a: Bool) -> Bool = (a, (a, 1)).1.0\n").unwrap();
    assert_eq!(json(&l, "p"), r#"["project",["project",["var","t"],0],1]"#);
    assert_eq!(
        json(&l, "q"),
        r#"["tuple",[["lit",{"type":"Nat","value":"1"}],["lit",{"type":"Nat","value":"2"}]]]"#
    );
    assert_eq!(l.model("q").unwrap().ret.name(), "(Nat,Nat)");
    assert_eq!(json(&l, "r"), r#"["project",["project",["tuple",[["var","a"],["tuple",[["var","a"],["lit",{"type":"Int","value":"1"}]]]]],1],0]"#);
    // a bad index is refused (the checker names it)
    assert!(lower("  model z(t: (Bool,Bool)) -> Bool = t.2\n").is_err());
}

#[test]
fn match_lowers_every_pattern_and_err_e_is_err_any() {
    let l = lower("  model m(r: Result<(Bool,Bool),E>) -> Nat = match r with Ok((a, _)) -> (if a then 1 else 0) | Err(E.X) -> 2 | Err(e) -> 3\n").unwrap();
    assert_eq!(
        json(&l, "m"),
        concat!(
            r#"["match",["var","r"],[[["ok",["tuple",[["bind","a"],["wild"]]]],"#,
            r#"["if",["var","a"],["lit",{"type":"Nat","value":"1"}],["lit",{"type":"Nat","value":"0"}]]],"#,
            r#"[["err","X"],["lit",{"type":"Nat","value":"2"}]],[["err_any"],["lit",{"type":"Nat","value":"3"}]]]]"#
        )
    );
    // an arm that USES the error value has no IR form: ir.data
    let e = lower("  model u(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(e) -> e == e\n")
        .expect_err("uses e");
    assert!(e.code == code::LEVEL || e.code == code::CHECK, "{e}");
    // a match on a non-Result is ir.data
    let e = lower("  model d(o: Bool) -> Nat = match o with true -> 1 | false -> 0\n").unwrap_err();
    assert_eq!(e.level, Some("ir.data"));
}

#[test]
fn the_arms_of_a_match_unify_and_a_mismatch_is_refused() {
    // Nat and Int arms unify to Int
    let l = lower("  model n(r: Result<Bool,E>) -> Int = match r with Ok(_) -> 1 | Err(_) -> 0 - 1\n").unwrap();
    assert!(l.model("n").is_some());
    let e = lower("  model bad(r: Result<Bool,E>) -> Bool = match r with Ok(_) -> true | Err(_) -> 1\n").unwrap_err();
    assert!(e.code == code::TYPE || e.code == code::CHECK, "{e}");
}

#[test]
fn match_cost_is_one_plus_the_scrutinee_plus_the_maximum_over_the_arms() {
    let l = lower("  model m(r: Result<U8,E>) -> U8 = match r with Ok(x) -> x + 1u8 + 1u8 | Err(_) -> 0u8\n  model t(a: Bool, b: Bool, c: Bool) -> (Bool,Bool,Bool) = (a, b, c)\n  model pr(a: (Bool,Bool)) -> Bool = a.0\n").unwrap();
    let costs = ModelCosts::compute(&l.models, 2000).unwrap();
    // scrutinee 1; Ok arm: add(add(x,1),1) = 1 + (1+1+1) + 1 = 5; Err arm 1; so 1 + 1 + 5
    assert_eq!(costs.cost("m"), Some(7));
    assert_eq!(costs.cost("t"), Some(4));
    assert_eq!(costs.cost("pr"), Some(2));
    assert_eq!(ir_cost(&l.model("m").unwrap().body, &costs, 2000).unwrap(), 7);
}

#[test]
fn requires_counts_a_tuple_type_even_without_a_tuple_tag() {
    let l = lower("  model through(t: (U8,U8)) -> (U8,U8) = t\n  model plain(a: U8) -> U8 = a\n").unwrap();
    assert_eq!(l.model("through").unwrap().requires, ["ir.core", "ir.tuple"]);
    assert_eq!(l.model("plain").unwrap().requires, ["ir.core"]);
}

#[test]
fn a_tuple_value_streams_with_tag_64_and_is_not_in_the_v1_set() {
    let ty = Ty::Tuple(vec![Ty::Bool, Ty::Nat]);
    let mut out = Vec::new();
    encode(&ty, &Value::Tuple(vec![Value::Bool(true), Value::Int(3)]), &mut out).unwrap();
    assert_eq!(out, [&[0x64u8, 2, 0, 0, 0, 1, 0x10][..], &3u64.to_le_bytes()[..]].concat());
    assert!(!in_v1_set(&ty));
    assert!(!in_v1_set(&Ty::Result(Box::new(ty.clone()), "E".into())));
    // the shape must agree with the type
    assert!(encode(&ty, &Value::Tuple(vec![Value::Bool(true)]), &mut Vec::new()).is_err());
}

fn instantiated(ret: &str, params: &str, fault: &str) -> Result<String, GenError> {
    let lop = hand_op(ret, params);
    let errs = vec![coding_adventures_warrant_corpus_gen::lower::ErrorType {
        name: "E".into(),
        cases: vec!["X".into(), "Y".into()],
    }];
    instantiate_in(&lop, fault, &errs).map(|f| f.to_json().render().unwrap().split_whitespace().collect())
}

fn ty(s: &str) -> Ty {
    match s {
        "Bool" => Ty::Bool,
        "Nat" => Ty::Nat,
        "U8" => Ty::Fixed(coding_adventures_warrant_grammar::fixed::FixedTy::U8),
        "Bytes" => Ty::BYTES,
        "Bytes<2>" => Ty::bytes_n(2),
        "(Bool,Bool)" => Ty::Tuple(vec![Ty::Bool, Ty::Bool]),
        "(Bool,Nat)" => Ty::Tuple(vec![Ty::Bool, Ty::Nat]),
        "(Bool,Bool,Bool)" => Ty::Tuple(vec![Ty::Bool; 3]),
        "(Nat,Bool)" => Ty::Tuple(vec![Ty::Nat, Ty::Bool]),
        s if s.starts_with("Result<") => {
            let inner = &s["Result<".len()..s.len() - ",E>".len()];
            Ty::Result(Box::new(ty(inner)), "E".into())
        }
        other => panic!("{other}"),
    }
}

fn hand_op(ret: &str, params: &str) -> coding_adventures_warrant_corpus_gen::lower::LoweredOp {
    let params: Vec<(String, Ty)> = params
        .split(", ")
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (n, t) = p.split_once(": ").unwrap();
            (n.to_string(), ty(t))
        })
        .collect();
    coding_adventures_warrant_corpus_gen::lower::LoweredOp {
        name: "o".into(),
        statics: vec![],
        params,
        ret: ty(ret),
        fresh: false,
        admit: None,
        requires: vec!["ir.core"],
    }
}

#[test]
fn the_tuple_fault_templates_instantiate_by_signature_with_exact_requires() {
    let j = |ret, params, fault| instantiated(ret, params, fault).unwrap();
    let swap = j("(Bool,Bool)", "a: Bool", "swap-components");
    assert!(swap.contains(r#""requires":["ir.tuple"]"#), "{swap}");
    assert!(swap.contains(r#"["tuple",[["project",["var","result"],1],["project",["var","result"],0]]]"#));
    let ok_swap = j("Result<(Bool,Bool),E>", "a: Bool", "swap-components");
    assert!(ok_swap.contains(r#"["ok",["bind","ok_payload"]]"#) && ok_swap.contains(r#"["err_any"]"#));
    let drop = j("(Bool,Nat)", "a: Bool", "drop-consumed");
    assert!(drop.contains(r#"["lit",{"type":"Nat","value":"0"}]"#));
    let ok_as_err = j("Result<(Bool,Bool),E>", "a: Bool", "ok-as-err");
    assert!(ok_as_err.contains(r#"["ok",["wild"]],["err","X"]"#), "{ok_as_err}");
    // missing-error with a tuple payload: the first parameter of exactly the component's type, else zero
    let miss = j("Result<(Nat,Bool),E>", "b: Bool, n: Nat", "missing-error");
    assert!(miss.contains(r#"["ok",["tuple",[["var","n"],["var","b"]]]]"#), "{miss}");
    let miss0 = j("Result<(Bool,Nat),E>", "d: Bytes", "missing-error");
    assert!(miss0.contains(r#"["lit",false]"#) && miss0.contains(r#""value":"0""#));
    // a template on an op whose type has a tuple adds ir.tuple to a core template
    let inv = j("Bool", "t: (Bool,Bool)", "invert");
    assert!(inv.contains(r#""requires":["ir.core","ir.tuple"]"#), "{inv}");
    // flip / drop-last: bare and Ok forms, levels exactly as the spec says
    assert!(j("Bytes<2>", "d: Bytes<2>", "flip-result-bit").contains(r#""requires":["ir.fixed"]"#));
    assert!(j("Result<Bytes<2>,E>", "d: Bytes<2>", "flip-result-bit").contains(r#""requires":["ir.fixed","ir.tuple"]"#));
    assert!(j("Bytes", "d: Bytes", "drop-last-result-byte").contains(r#""requires":["ir.core"]"#));
    assert!(j("Result<Bytes,E>", "d: Bytes", "drop-last-result-byte").contains(r#""requires":["ir.core","ir.tuple"]"#));
    assert!(j("U8", "a: U8", "flip-result-bit").contains(r#"["bit_xor",["var","result"],"#));
    assert!(j("Result<U8,E>", "a: U8", "flip-result-bit").contains(r#"["bit_xor",["var","ok_payload"]"#));
}

#[test]
fn a_template_the_signature_cannot_take_is_a_fault_error_and_nothing_is_skipped() {
    for (ret, params, fault) in [
        ("Bool", "a: Bool", "ok-as-err"),
        ("Result<Bool,E>", "a: Bool", "ok-as-err"),
        ("(Bool,Bool)", "a: Bool", "drop-consumed"),
        ("(Nat,Bool)", "a: Bool", "drop-consumed"),
        ("(Bool,Nat)", "a: Bool", "swap-components"),
        ("U8", "a: U8", "drop-last-result-byte"),
        ("Bool", "a: Bool", "flip-result-bit"),
        ("Result<(Bool,Bool),E>", "a: Bool", "flip-result-bit"),
        ("Result<Bytes,E>", "ok_payload: Bool", "drop-last-result-byte"),
        ("Result<Bytes,E>", "ok_payload: Bool", "flip-result-bit"),
        ("Result<(Bool,Bool),E>", "ok_payload: Bool", "swap-components"),
    ] {
        let e = instantiated(ret, params, fault).expect_err(fault);
        assert_eq!(e.code, code::FAULT, "{ret} {fault}: {e}");
    }
    // ok-as-err needs the error type's cases (none known)
    let lop = hand_op("Result<(Bool,Bool),E>", "a: Bool");
    assert_eq!(instantiate(&lop, "ok-as-err").unwrap_err().code, code::FAULT);
}

#[test]
fn the_gate_defers_to_the_first_unimplemented_level_in_sorted_order() {
    use Level::{Core, Fixed, Tuple};
    assert_eq!(first_unimplemented(&[Fixed, Tuple], &SHIPPED_GATE), Some(Fixed));
    assert_eq!(first_unimplemented(&[Core, Tuple], &SHIPPED_GATE), Some(Tuple));
    assert_eq!(first_unimplemented(&[Fixed, Tuple], &[Core, Tuple]), Some(Fixed));
    assert_eq!(first_unimplemented(&[Core, Tuple], &[Core, Tuple]), None);
    assert_eq!(first_unimplemented(&[Core], &SHIPPED_GATE), None);
    assert!(Core < Fixed && Fixed < Tuple, "the derived order is the sorted order of the names");
}
