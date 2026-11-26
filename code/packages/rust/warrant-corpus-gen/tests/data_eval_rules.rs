//! The rules of `ir.data` evaluation that S11c-2 owns, one at a time and small (the byte parity of the whole
//! fixture is `data_eval_parity.rs`): R95 (a nested composite is built), R96 to R98 (typing a fault row), R99
//! (TFV depth) and the stream and JSON forms of the three data kinds (VER04r 2.3 to 2.5, Appendix A.2).

mod support;

use coding_adventures_warrant_corpus_gen::error::code;
use coding_adventures_warrant_corpus_gen::fault_types::{fault_component, fault_unify};
use coding_adventures_warrant_corpus_gen::faults::{instantiate, instantiate_with, type_fault, Level};
use coding_adventures_warrant_corpus_gen::lower::{lower_document, LoweredType};
use coding_adventures_warrant_corpus_gen::stream::{encode_in, in_v1_set, suite_header, Header};
use coding_adventures_warrant_corpus_gen::tfv::{value_json, value_tfv, MAX_TFV_DEPTH};
use coding_adventures_warrant_corpus_gen::types::{Decls, Ty};
use coding_adventures_warrant_corpus_gen::value::Value;
use std::rc::Rc;
use support::*;

/// A refusal document with one operation `f(<params>) -> <ret>` and the declarations `decls`.
fn doc(decls: &str, params: &str, ret: &str) -> String {
    let names: Vec<&str> = params
        .split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.split(':').next().unwrap().trim())
        .collect();
    let mut s = format!("warrant r v1\n  error E = A | B\n{decls}  op f({params}) -> {ret}\n");
    s.push_str("  assume platform:\n    none\n  assume environment:\n    none\n");
    if !names.is_empty() {
        s.push_str("  labels f:\n");
        for n in &names {
            s.push_str(&format!("    {n} secret\n"));
        }
        for n in &names {
            s.push_str(&format!("  input f.{n}:\n    hostile\n    ensures no_panic, terminates\n"));
        }
    }
    s.push_str("  bound f:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    s
}

/// The compact JSON of `fault` on the operation of `source`, instantiated with the document's declarations.
fn fault_json(source: &str, fault: &str) -> (String, Vec<Level>) {
    with_file(source, |f| {
        let l = lower_document(f).expect("lowers");
        let op = l.op("f").expect("op f");
        let d = Decls::new(&l.types);
        let fault = instantiate_with(op, fault, &l.errors, &d).expect("instantiates");
        type_fault(&fault, op, &[], &d).expect("types");
        (
            fault.to_json().render().unwrap().split_whitespace().collect(),
            fault.requires.clone(),
        )
    })
}

#[test]
fn missing_error_builds_a_nested_tuple_and_takes_parameters_of_exactly_their_type_r95() {
    // R95 (e): the pre-S11c refusal of a nested component is gone; a leaf is the parameter of exactly its type
    let (json, requires) = fault_json(&doc("", "a: U8, n: Nat", "Result<((U8,Bool),Nat),E>"), "missing-error");
    assert!(
        json.contains(r#"["ok",["tuple",[["tuple",[["var","a"],["lit",false]]],["var","n"]]]]"#),
        "{json}"
    );
    assert_eq!(requires, [Level::Tuple]);
}

#[test]
fn missing_error_accepts_an_option_payload_with_the_typed_none_d71() {
    // D71: the oracle gave `Result<Option<U8>,E>` a template, so the shape is no longer refused
    let (json, requires) = fault_json(&doc("", "d: U8", "Result<Option<U8>,E>"), "missing-error");
    assert!(!json.contains(r#"["ok",["var","d"]]"#), "{json}");
    assert!(json.contains(r#"["ok",["none","U8"]]"#), "{json}");
    assert_eq!(requires, [Level::Data]);
    // an Option parameter of exactly the payload type is taken whole
    let (json, _) = fault_json(&doc("", "o: Option<U8>", "Result<Option<U8>,E>"), "missing-error");
    assert!(json.contains(r#"["ok",["var","o"]]"#), "{json}");
}

#[test]
fn missing_error_builds_a_record_from_its_fields_and_field_zero_zeroes_the_first_numeric_field() {
    let decls = "  record P(f: Bool, n: U16, m: Nat)\n";
    let (json, requires) = fault_json(&doc(decls, "n: U16", "Result<P,E>"), "missing-error");
    assert!(json.contains(r#"["ok",["ctor","P",[["lit",false],["var","n"],["lit",{"type":"Nat","value":"0"}]]]]"#), "{json}");
    assert_eq!(requires, [Level::Data]);
    // field-zero: the first numeric field (index 1, U16) becomes 0 and the others are read back (the Ok form)
    let (json, requires) = fault_json(&doc(decls, "a: U8", "Result<P,E>"), "field-zero");
    assert!(
        json.contains(r#"["ctor","P",[["field",["var","ok_payload"],0],["lit",{"type":"U16","value":"0"}],["field",["var","ok_payload"],2]]]"#),
        "{json}"
    );
    assert_eq!(requires, [Level::Data, Level::Tuple]);
    // bare record result: no `match`, `ir.data` alone
    let (json, requires) = fault_json(&doc(decls, "a: U8", "P"), "field-zero");
    assert!(json.starts_with(r#"{"expr":["ctor","P","#), "{json}");
    assert_eq!(requires, [Level::Data]);
}

#[test]
fn without_declarations_the_tables_are_empty_and_a_record_has_nothing_to_build() {
    // R95 (d): one rule; an empty table is not a second mode, `field-zero` has no record to name
    let source = doc("  record P(n: U8)\n", "a: U8", "P");
    let e = with_file(&source, |f| {
        let l = lower_document(f).unwrap();
        instantiate(l.op("f").unwrap(), "field-zero").unwrap_err()
    });
    assert_eq!(e.code, code::FAULT);
    assert!(e.message.ends_with("field-zero needs a record or Result<record, E> result"), "{e}");
}

// ---- R96 to R98: the common type of a fault row --------------------------------------------------------------

fn t(text: &str) -> Ty {
    match text {
        "Nat" => Ty::Nat,
        "Int" => Ty::Int,
        "Bool" => Ty::Bool,
        "Bytes" => Ty::BYTES,
        "Bytes<1>" => Ty::bytes_n(1),
        "Bytes<3>" => Ty::bytes_n(3),
        other => panic!("{other}"),
    }
}

#[test]
fn a_tuple_unifies_component_by_component_under_the_narrower_relation_r96() {
    // off-by-one-consumed: `(Bytes,Int)` against the declared `(Bytes,Nat)` has the common type `(Bytes,Int)`
    let got = Ty::Tuple(vec![t("Bytes"), t("Int")]);
    let declared = Ty::Tuple(vec![t("Bytes"), t("Nat")]);
    assert_eq!(fault_unify(&got, &declared), Some(Ty::Tuple(vec![t("Bytes"), t("Int")])));
    // nothing else lifts below a tuple: two `Bytes` of different widths have no common type there ...
    let narrow = Ty::Tuple(vec![t("Bytes<1>"), t("Bool")]);
    let wide = Ty::Tuple(vec![t("Bytes<3>"), t("Bool")]);
    assert_eq!(fault_unify(&narrow, &wide), None);
    // ... at any depth, so an `Option<Bytes<1>>` component does not meet an `Option<Bytes<3>>` one,
    let (a, b) = (Ty::Option(Box::new(t("Bytes<1>"))), Ty::Option(Box::new(t("Bytes<3>"))));
    assert_eq!(fault_component(&a, &b), None);
    // although the same two unify at the top level by R94
    assert_eq!(fault_unify(&a, &b), Some(Ty::Option(Box::new(Ty::BYTES))));
    // a Bool never meets a Bytes, and a record is nominal
    assert_eq!(fault_unify(&t("Bool"), &t("Bytes")), None);
    assert_eq!(fault_unify(&Ty::Named("A".into()), &Ty::Named("B".into())), None);
}

#[test]
fn an_undetermined_component_takes_the_determined_side_r97_r98() {
    // a bare `ok` has an undetermined error type: the common type takes the declared one
    let bare = Ty::Result(Box::new(t("Bytes")), "?".into());
    let declared = Ty::Result(Box::new(Ty::bytes_n(32)), "E".into());
    assert_eq!(
        fault_unify(&bare, &declared),
        Some(Ty::Result(Box::new(Ty::BYTES), "E".into())),
        "unification, not flows-to: plain Bytes meets a declared Bytes<32>"
    );
    // two different error types have none
    let other = Ty::Result(Box::new(t("Bytes")), "F".into());
    assert_eq!(fault_unify(&other, &declared), None);
}

// ---- R99: TFV depth ------------------------------------------------------------------------------------------

/// `links` records, each holding the previous one: `R1(a: R0)`, `R0(x: Bool)`.
fn record_chain(links: usize) -> (Decls, Ty, Value) {
    let mut types = vec![LoweredType::Record { name: "R0".into(), fields: vec![("x".into(), Ty::Bool)] }];
    let mut value = Value::Con(Rc::from("R0"), vec![Value::Bool(true)]);
    for k in 1..links {
        types.push(LoweredType::Record {
            name: format!("R{k}"),
            fields: vec![("a".into(), Ty::Named(format!("R{}", k - 1)))],
        });
        value = Value::Con(Rc::from(format!("R{k}").as_str()), vec![value]);
    }
    (Decls::new(&types), Ty::Named(format!("R{}", links - 1)), value)
}

/// `links` variants, each with a constructor holding the previous one and a payload-free `End`.
fn variant_chain(links: usize) -> (Decls, Ty, Value) {
    let mut types = vec![LoweredType::Variant {
        name: "V0".into(),
        ctors: vec![("End0".into(), vec![])],
    }];
    let mut value = Value::Con(Rc::from("End0"), vec![]);
    for k in 1..links {
        types.push(LoweredType::Variant {
            name: format!("V{k}"),
            ctors: vec![(format!("Up{k}"), vec![Ty::Named(format!("V{}", k - 1))]), (format!("End{k}"), vec![])],
        });
        value = Value::Con(Rc::from(format!("Up{k}").as_str()), vec![value]);
    }
    (Decls::new(&types), Ty::Named(format!("V{}", links - 1)), value)
}

#[test]
fn tfv_accepts_64_nested_containers_and_refuses_the_65th_r99() {
    assert_eq!(MAX_TFV_DEPTH, 64);
    // a chain of records: one container per link; the field's own `R<n>` of the line does not count
    let (d, ty, v) = record_chain(64);
    assert!(value_tfv(&d, &ty, &v).is_ok());
    let (d, ty, v) = record_chain(65);
    let e = value_tfv(&d, &ty, &v).unwrap_err();
    assert_eq!(e.code, code::LIMIT, "{e}");
    // a chain of variants: two containers per link, so 32 links are 64 and 33 are 65
    let (d, ty, v) = variant_chain(32);
    let line = value_tfv(&d, &ty, &v).expect("32 links");
    assert!(line.starts_with("L2 S5570"), "{line}");
    let (d, ty, v) = variant_chain(33);
    assert_eq!(value_tfv(&d, &ty, &v).unwrap_err().code, code::LIMIT);
    // the JSON of the deep value is not limited (the limit is TFV's own)
    assert!(value_json(&d, &ty, &v).is_ok());
}

/// `Option` nested `n` times around `Bool`, with the value `Some(..Some(true))`.
fn nested_options(n: usize) -> (Ty, Value) {
    let (mut ty, mut v) = (Ty::Bool, Value::Bool(true));
    for _ in 0..n {
        ty = Ty::Option(Box::new(ty));
        v = Value::Opt(Some(Box::new(v)));
    }
    (ty, v)
}

#[test]
fn an_option_and_a_result_each_open_one_container_r99() {
    let d = Decls::default();
    let (ty, v) = nested_options(64);
    assert!(value_tfv(&d, &ty, &v).is_ok());
    let (ty, v) = nested_options(65);
    assert_eq!(value_tfv(&d, &ty, &v).unwrap_err().code, code::LIMIT);
    // a `none` is `L0`: also a container (the 64th here)
    let (inner, _) = nested_options(63);
    assert_eq!(value_tfv(&d, &Ty::Option(Box::new(inner.clone())), &Value::Opt(None)).unwrap(), "L0");
    // a `Result` is `M1`, for `Ok` and for `Err`: 63 options inside an `Ok` are 64 containers, 64 are 65
    let result = |n: usize| {
        let (ty, v) = nested_options(n);
        (Ty::Result(Box::new(ty), "E".into()), Value::Ok(Box::new(v)))
    };
    let (ty, v) = result(63);
    assert!(value_tfv(&d, &ty, &v).is_ok());
    let (ty, v) = result(64);
    assert_eq!(value_tfv(&d, &ty, &v).unwrap_err().code, code::LIMIT);
    // an `Err` carries only its case: one container whatever the payload type
    assert_eq!(value_tfv(&d, &ty, &Value::Err(Rc::from("A"))).unwrap(), "M1 S657272 S41");
}

// ---- the stream and JSON forms ------------------------------------------------------------------------------

#[test]
fn an_option_keeps_the_v1_header_and_a_record_or_a_variant_makes_it_v2() {
    let opt = Ty::Option(Box::new(Ty::Fixed(coding_adventures_warrant_grammar::fixed::FixedTy::U8)));
    assert!(in_v1_set(&opt));
    assert_eq!(suite_header(std::slice::from_ref(&opt), &opt), Header::V1);
    // `Option<(Bool,Nat)>` holds a tuple, a record is a v2 type
    assert!(!in_v1_set(&Ty::Option(Box::new(Ty::Tuple(vec![Ty::Bool, Ty::Nat])))));
    assert!(!in_v1_set(&Ty::Named("P".into())));
}

#[test]
fn the_three_kinds_encode_with_their_tags() {
    let (d, ty, v) = record_chain(1);
    let mut out = Vec::new();
    encode_in(&d, &ty, &v, &mut out).unwrap();
    assert_eq!(out, [0x64, 1, 0, 0, 0, 1], "a record: 64, a u32 count, its fields");
    let (d, ty, v) = variant_chain(1);
    let mut out = Vec::new();
    encode_in(&d, &ty, &v, &mut out).unwrap();
    assert_eq!(out, [0x65, 4, 0, 0, 0, b'E', b'n', b'd', b'0', 0, 0, 0, 0], "a variant: 65, name, count");
    let some = Ty::Option(Box::new(Ty::Bool));
    let mut out = Vec::new();
    encode_in(&Decls::default(), &some, &Value::Opt(Some(Box::new(Value::Bool(true)))), &mut out).unwrap();
    assert_eq!(out, [0x41, 1]);
    let mut out = Vec::new();
    encode_in(&Decls::default(), &some, &Value::Opt(None), &mut out).unwrap();
    assert_eq!(out, [0x40]);
    // a constructor the type does not declare is refused, never encoded
    let (d, _, _) = record_chain(1);
    let bad = Value::Con(Rc::from("Nope"), vec![]);
    assert!(encode_in(&d, &Ty::Named("R0".into()), &bad, &mut Vec::new()).is_err());
}
