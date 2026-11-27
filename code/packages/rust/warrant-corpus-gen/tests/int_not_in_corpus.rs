//! CV2-F17 / VER04r 2.1 and 2.3: `Int` never reaches a corpus, and an example or expected value must be a
//! value of its declared type.
//!
//! This file mirrors `python/warrant-oracle/tests/test_int_not_in_corpus.py` case by case.  `Int` is
//! model-internal: the type of a `lit` node and of the intermediate of `Nat` arithmetic.  The **lowering**
//! accepts a document that names it elsewhere (the committed `data-forms-v1` is a lowering-only fixture of
//! such documents); **generation** refuses it, `W0701`, when it would be written to a `types` row, an op
//! signature or a binder.  A model signature, a `lit` and a `none` are not looked at.
//!
//! Every message below is copied from the oracle's source (`corpus_v2.refuse_int_in_corpus`,
//! `corpus_v2._check_value`, `stream.make_encoder`, `tfv.value`, `corpus_v2.value_json`), and the BUILD-time
//! evidence that the two tools say the same thing on the same documents is the table in
//! `CHANGELOG.d/00233-cv2-f17-gen-int-and-range-parity.md`.

mod support;

use coding_adventures_warrant_corpus_gen::enumerate::enumerate_document;
use coding_adventures_warrant_corpus_gen::lower::{lower_document, LoweredType};
use coding_adventures_warrant_corpus_gen::profile::Profile;
use coding_adventures_warrant_corpus_gen::stream::encode_in;
use coding_adventures_warrant_corpus_gen::tfv::{value_json, value_tfv};
use coding_adventures_warrant_corpus_gen::types::{Decls, Ty};
use coding_adventures_warrant_corpus_gen::value::Value;
use coding_adventures_warrant_corpus_gen::{code, generate, CorpusV2, GenError, GenInput};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use support::*;

type Op<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a str);

/// The contract boilerplate (`assume`, `labels`, `input`, `bound`) the S2 checker wants for scalar parameters.
fn contracts(ops: &[Op<'_>]) -> String {
    let mut s = String::from("  assume platform:\n    none\n  assume environment:\n    none\n");
    for (name, params, _) in ops {
        if params.is_empty() {
            continue;
        }
        s += &format!("  labels {name}:\n");
        for (p, _) in *params {
            s += &format!("    {p} secret\n");
        }
    }
    for (name, params, _) in ops {
        for (p, _) in *params {
            s += &format!("  input {name}.{p}:\n    hostile\n    ensures no_panic, terminates\n");
        }
    }
    for (name, _, _) in ops {
        s += &format!("  bound {name}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    }
    s
}

/// `warrant t v1`, an error type, `decls`, the ops, their contracts and `rest` (claims, examples).
fn full(ops: &[Op<'_>], rest: &str, decls: &str) -> String {
    let mut s = format!("warrant t v1\n  error E = A | B\n{decls}");
    for (name, params, ret) in ops {
        let ps: Vec<String> = params.iter().map(|(p, t)| format!("{p}: {t}")).collect();
        s += &format!("  op {name}({}) -> {ret}\n", ps.join(", "));
    }
    s + &contracts(ops) + rest
}

const ECHO: &[Op<'static>] = &[("echo", &[("a", "Bool")], "Bool")];
const CLAIM: &str = "  claim c:\n    forall a: Bool. echo(a) == a\n";

fn doc(decls: &str) -> String {
    full(ECHO, CLAIM, decls)
}

fn gen(src: &str) -> Result<CorpusV2, GenError> {
    with_file(src, |f| {
        generate(&GenInput {
            file: f,
            spec_path: "t.warrant",
            spec_bytes: src.as_bytes(),
            profile_bytes: b"",
            binding_faults: None,
        })
    })
}

/// The refusal of `src` by generation: `W0701`, and the message (which `Display` writes after `W0701: `).
/// The same document lowers: only generation refuses.
fn refused(src: &str) -> String {
    with_file(src, |f| lower_document(f).expect("the document still lowers"));
    let e = gen(src).expect_err("generation refuses");
    assert_eq!(e.code, code::W0701, "{e}");
    e.message
}

fn int_message(place: &str) -> String {
    format!("`Int` is model-internal and may not be {place} of a corpus (VER04r 2.1)")
}

fn range_message(head: &str) -> String {
    format!("a value is outside the range of {head} (VER04r 2.3)")
}

// ---- Int in a corpus position ----------------------------------------------------------------

#[test]
fn a_document_without_int_in_a_corpus_position_derives() {
    let c = gen(&doc("")).expect("generates");
    assert!(c.corpus_json.contains(r#""kind":"error""#) || c.corpus_json.contains("\"E\""));
}

#[test]
fn a_record_field_of_int_at_any_depth_is_refused() {
    for field in ["Int", "Option<Int>", "(Int,Bool)", "Result<Int,E>", "Option<Option<Int>>", "(Bool,(Nat,Option<Int>))"] {
        assert_eq!(refused(&doc(&format!("  record R(i: {field})\n"))), int_message("a record field"), "{field}");
    }
}

#[test]
fn a_variant_field_of_int_at_any_depth_is_refused() {
    for field in ["Int", "Option<Int>", "(Int,Bool)"] {
        assert_eq!(refused(&doc(&format!("  variant V = A(i: {field}) | B\n"))), int_message("a variant field"), "{field}");
    }
}

#[test]
fn a_record_field_reached_through_an_alias_is_refused() {
    assert_eq!(refused(&doc("  model Z = Int\n  record R(i: Z)\n")), int_message("a record field"));
}

#[test]
fn an_op_parameter_or_result_of_int_is_refused() {
    let both = "  claim c:\n    forall a: Bool. echo(a) == echo(a)\n";
    type Case<'a> = (&'a [(&'a str, &'a str)], &'a str, &'a str);
    let cases: [Case<'_>; 5] = [
        (&[("a", "Bool"), ("n", "Int")], "Bool", "  claim c:\n    forall a: Bool, n: Int. echo(a, n) == echo(a, n)\n"),
        (&[("a", "Bool")], "Int", both),
        (&[("a", "Bool"), ("o", "Option<Int>")], "Bool", "  claim c:\n    forall a: Bool, o: Option<Int>. echo(a, o) == echo(a, o)\n"),
        (&[("a", "Bool")], "Option<Int>", both),
        (&[("a", "Bool")], "Result<Int,E>", both),
    ];
    for (params, ret, claim) in cases {
        let src = full(&[("echo", params, ret)], claim, "");
        assert_eq!(refused(&src), int_message("an op signature"), "{ret}");
    }
}

#[test]
fn an_op_with_no_binder_of_int_still_refuses_on_its_signature() {
    let src = full(&[("f", &[("n", "Int")], "Bool"), ("echo", &[("a", "Bool")], "Bool")], CLAIM, "");
    assert_eq!(refused(&src), int_message("an op signature"));
}

#[test]
fn a_binder_of_int_is_refused() {
    // no op takes an Int, so the binder is the only position (an unused one: it is still enumerated)
    let src = full(ECHO, "  claim c:\n    forall a: Bool, n: Int. echo(a) == a\n", "");
    assert_eq!(refused(&src), int_message("a binder"));
}

#[test]
fn the_first_kind_in_the_oracles_order_names_the_refusal() {
    // a record field outranks a variant field, an op signature and a binder, whatever the declaration order
    let all = full(
        &[("echo", &[("a", "Bool"), ("n", "Int")], "Bool")],
        "  claim c:\n    forall a: Bool, n: Int. echo(a, n) == echo(a, n)\n",
        "  variant V = A(i: Int) | B\n  record R(i: Int)\n",
    );
    assert_eq!(refused(&all), int_message("a record field"));
    let no_record = full(
        &[("echo", &[("a", "Bool"), ("n", "Int")], "Bool")],
        "  claim c:\n    forall a: Bool, n: Int. echo(a, n) == echo(a, n)\n",
        "  variant V = A(i: Int) | B\n",
    );
    assert_eq!(refused(&no_record), int_message("a variant field"));
}

#[test]
fn the_declared_types_still_lower_with_their_int_fields() {
    let src = doc("  record R(i: Option<Int>)\n");
    with_file(&src, |f| {
        let l = lower_document(f).unwrap();
        let LoweredType::Record { fields, .. } = &l.types[0] else { panic!("a record") };
        assert_eq!(fields[0].1.name(), "Option<Int>");
    });
}

#[test]
fn enumeration_still_reports_the_document_it_is_not_a_corpus_refusal() {
    // the generator's counterpart of the oracle's `emit-v2`, which reports per suite and is not a document
    // refusal: only `generate` (the oracle's `derive`) refuses
    let src = doc("  record R(i: Int)\n");
    let e = with_file(&src, |f| enumerate_document(f, &Profile::default()).map(|e| e.suites.len()));
    assert_eq!(e.expect("enumerates"), 1);
}

#[test]
fn a_model_signature_lit_and_none_of_int_are_not_refused() {
    let models = "  model sq(a: Int) -> Int = a * a\n\
                  \x20 model big(a: Bool) -> Bool = sq(3) > 8 and a\n\
                  \x20 model none_int(a: Bool) -> Option<Int> = None\n\
                  \x20 model dflt(a: Bool) -> Bool = unwrap_or(none_int(a), 7) > 0\n";
    let src = full(ECHO, "  claim c:\n    forall a: Bool. echo(a) == big(a)\n", models);
    let c = gen(&src).expect("generates");
    assert!(c.corpus_json.contains("\"sq\""));
}

// ---- an expected value must be a value of its type -----------------------------------------------------

const NAT_CLAIM: &str = "  claim c:\n    forall a: Nat. f(a) == a\n";

fn nat_doc(example: &str, claim: &str) -> String {
    let body = if example.is_empty() { String::new() } else { format!("  example e:\n    {example}\n") };
    full(&[("f", &[("a", "Nat")], "Nat")], &format!("{body}{claim}"), "")
}

#[test]
fn an_example_whose_expected_value_is_outside_its_type_is_refused() {
    let ok = gen(&nat_doc("f(3) == 2 + 1", NAT_CLAIM)).expect("2 + 1 is 3");
    assert!(ok.corpus_json.contains(r#""expected": "3""#));
    // -3 was a value no Nat has: the row would have said "expected": "-3"
    assert_eq!(refused(&nat_doc("f(3) == 2 - 5", NAT_CLAIM)), range_message("Nat"));
    // 2^64
    assert_eq!(refused(&nat_doc("f(3) == 18446744073709551615 + 1", NAT_CLAIM)), range_message("Nat"));
}

#[test]
fn an_example_value_inside_an_option_is_range_checked_at_depth() {
    let src = full(
        &[("f", &[("a", "U8")], "Option<Nat>")],
        "  example e:\n    f(1u8) == Some(2 - 5)\n  claim c:\n    forall a: U8. f(a) == Some(1)\n",
        "",
    );
    assert_eq!(refused(&src), range_message("Nat"));
}

#[test]
fn a_claim_whose_expected_value_leaves_the_result_type_is_refused_not_an_overflow() {
    for expr in ["a - 1", "a * 18446744073709551615", "a + 18446744073709551615"] {
        let claim = format!("  claim c:\n    forall a: Nat. f(a) == {expr}\n");
        assert_eq!(refused(&nat_doc("", &claim)), range_message("Nat"), "{expr}");
    }
}

#[test]
fn an_exact_intermediate_past_the_2_127_window_is_refused_not_a_panic() {
    let claim = "  claim c:\n    forall a: Nat. f(a) == a * 18446744073709551615 * 18446744073709551615 * 18446744073709551615\n";
    with_file(&nat_doc("", claim), |f| {
        let e = generate(&GenInput {
            file: f,
            spec_path: "t.warrant",
            spec_bytes: b"",
            profile_bytes: b"",
            binding_faults: None,
        })
        .expect_err("refused");
        assert_eq!(e.code, code::W0701);
    });
}

#[test]
fn a_fixed_width_result_that_wraps_is_not_out_of_range() {
    // `a - 1u8` on a U8 wraps (VER04r 3.2): the value is a U8, so nothing leaves the type
    let src = full(&[("f", &[("a", "U8")], "U8")], "  claim c:\n    forall a: U8. f(a) == a - 1u8\n", "");
    gen(&src).expect("generates");
}

#[test]
fn the_bytes_length_of_a_value_is_refused_by_the_generator_only_pin() {
    // PARITY GAP, reported, not closed here: the oracle's `_check_value` and encoder do not check a `Bytes<N>`
    // length, so it DERIVES these two documents (an example whose value has 3 bytes for a `Bytes<2>` result,
    // and a claim whose expected value does), while this generator has always refused them (`stream::encode_in`).
    // The pin keeps the generator's behaviour visible until the owner decides which side moves.
    let ex = full(
        &[("f", &[("a", "U8")], "Bytes<2>")],
        "  example e:\n    f(1u8) == b\"aabbcc\"\n  claim c:\n    forall a: U8. f(a) == b\"aabb\"\n",
        "",
    );
    assert_eq!(refused(&ex), "a stream value does not have the declared Bytes<N> length");
    let cl = full(&[("f", &[("a", "U8")], "Bytes<2>")], "  claim c:\n    forall a: U8. f(a) == b\"aabbcc\"\n", "");
    assert_eq!(refused(&cl), "a stream value does not have the declared Bytes<N> length");
}

// ---- the encoders, one type at a time (the oracle's parametrised cases) -------------------------------------

fn fixed(name: &str) -> Ty {
    Ty::Fixed(match name {
        "U8" => FixedTy::U8,
        "U16" => FixedTy::U16,
        "U32" => FixedTy::U32,
        "U64" => FixedTy::U64,
        "I8" => FixedTy::I8,
        "I16" => FixedTy::I16,
        "I32" => FixedTy::I32,
        _ => FixedTy::I64,
    })
}

#[test]
fn the_stream_encoder_checks_the_range_of_every_scalar() {
    let cases: [(&str, Ty, i128, i128); 7] = [
        ("U8", fixed("U8"), 255, -1),
        ("U16", fixed("U16"), 65535, i128::from(u64::MAX) + 1),
        ("U64", fixed("U64"), i128::from(u64::MAX), i128::from(u64::MAX) + 1),
        ("Nat", Ty::Nat, i128::from(u64::MAX), -1),
        ("I8", fixed("I8"), -128, i128::from(i64::MAX) + 1),
        ("I8", fixed("I8"), 127, i128::from(i64::MIN) - 1),
        ("I64", fixed("I64"), i128::from(i64::MIN), i128::from(i64::MAX) + 1),
    ];
    for (head, ty, good, bad) in cases {
        let mut out = Vec::new();
        encode_in(&Decls::default(), &ty, &Value::Int(good), &mut out).unwrap();
        assert_eq!(out.len(), 9, "{head}");
        let e = encode_in(&Decls::default(), &ty, &Value::Int(bad), &mut Vec::new()).unwrap_err();
        assert_eq!((e.code, e.message), (code::W0701, range_message(head)));
    }
}

#[test]
fn an_i64_value_is_never_masked() {
    // the oracle used to write `v & (2^64-1)` for a signed value; a value past i64 is now refused
    let e = encode_in(&Decls::default(), &fixed("I64"), &Value::Int(1 << 63), &mut Vec::new()).unwrap_err();
    assert_eq!(e.message, range_message("I64"));
}

#[test]
fn an_example_value_is_range_checked_by_its_own_type_at_any_depth() {
    // the declarations come from a real document, so the record is in `Decls`
    let doc_src = full(&[("f", &[("a", "Bool")], "Bool")], "  claim c:\n    forall a: Bool. f(a) == a\n", "  record P(x: U8, y: I8)\n");
    with_file(&doc_src, |f| {
        let l = lower_document(f).unwrap();
        let d = Decls::new(&l.types);
        let p = Ty::Named("P".into());
        let con = |x: i128, y: i128| Value::Con("P".into(), vec![Value::Int(x), Value::Int(y)]);
        encode_in(&d, &p, &con(255, -128), &mut Vec::new()).unwrap();
        let bad: [(Ty, Value, &str); 4] = [
            (p.clone(), con(256, 0), "U8"),
            (p.clone(), con(0, 128), "I8"),
            (Ty::Option(Box::new(fixed("U8"))), Value::Opt(Some(Box::new(Value::Int(256)))), "U8"),
            (
                Ty::Result(Box::new(Ty::Tuple(vec![fixed("U8"), Ty::Nat])), "E".into()),
                Value::Ok(Box::new(Value::Tuple(vec![Value::Int(1), Value::Int(-1)]))),
                "Nat",
            ),
        ];
        for (ty, v, head) in bad {
            let e = encode_in(&d, &ty, &v, &mut Vec::new()).unwrap_err();
            assert_eq!((e.code, e.message), (code::W0701, range_message(head)), "{head}");
        }
        encode_in(&d, &Ty::Option(Box::new(fixed("U8"))), &Value::Opt(None), &mut Vec::new()).unwrap();
        encode_in(&d, &Ty::Result(Box::new(fixed("U8")), "E".into()), &Value::Err("B".into()), &mut Vec::new()).unwrap();
    });
}

#[test]
fn an_int_value_has_no_json_tfv_or_stream_form_and_says_w0701() {
    let d = Decls::default();
    let v = Value::Int(5);
    let enc = encode_in(&d, &Ty::Int, &v, &mut Vec::new()).unwrap_err();
    assert_eq!((enc.code, enc.message.as_str()), (code::W0701, "an `Int` value has no stream encoding (VER04r 2.1)"));
    let json = value_json(&d, &Ty::Int, &v).unwrap_err();
    assert_eq!((json.code, json.message.as_str()), (code::W0701, "an `Int` value is never written to a corpus (VER04r 2.1)"));
    let tfv = value_tfv(&d, &Ty::Int, &v).unwrap_err();
    assert_eq!((tfv.code, tfv.message.as_str()), (code::W0701, "an `Int` value is never written to a corpus (VER04r 2.1)"));
}
