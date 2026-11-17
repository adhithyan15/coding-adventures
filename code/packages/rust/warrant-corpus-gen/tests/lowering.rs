//! End-to-end lowering and cost expectations, written by hand from the spec and from the
//! committed v1 corpus, not read back from the implementation.
//!
//! * ct-compare's lowered IR must equal what the v1 corpus compiler committed
//!   (`ct-compare-warrant-v1/corpus.json`): VER04r 10.3 check 1 against a committed artifact.
//! * the tag set emitted over every `ir.core` construct is exactly the 26 of Appendix A
//!   (VER04r 15.2 item 8).
//! * `ir_cost`: gf256's `gmul` is 2,011 and `ginv` 26,204 (VER04r 4.2, measured there with
//!   the oracle).  ct-compare's largest expression is **10**, not the 9 VER04r 4.2 says: see
//!   the test below and the S3a report (the oracle agrees with 10).

mod support;

use std::collections::BTreeSet;

use coding_adventures_warrant_corpus_gen::cost::{suite_cost, ModelCosts};
use coding_adventures_warrant_corpus_gen::ir::{Arith, Ir, ALL_TAGS};
use coding_adventures_warrant_corpus_gen::lower::{lower_document, Lowered, SuiteKind};
use coding_adventures_warrant_corpus_gen::profile::parse_profile;
use coding_adventures_warrant_corpus_gen::types::Ty;
use coding_adventures_warrant_corpus_gen::{analyse, code};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use support::*;

const V1_CORPUS: &str =
    include_str!("../../../../specs/fixtures/ct-compare-warrant-v1/corpus.json");

fn lowered(src: &str) -> Lowered {
    with_file(src, |f| lower_document(f).expect("lowers"))
}

/// The v1 corpus writes IR as compact JSON (no spaces); the canonical form is indented.
/// IR strings contain no whitespace, so dropping every whitespace character converts one
/// into the other.
fn compact(ir: &Ir) -> String {
    ir.to_json()
        .render()
        .unwrap()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[test]
fn ct_compare_models_ops_and_errors_equal_the_v1_corpus() {
    let l = lowered(CT_COMPARE);
    assert_eq!(l.module, "ct_compare");
    assert_eq!(l.document(), "v0");
    assert_eq!(l.errors.len(), 1);
    assert_eq!(l.errors[0].name, "SelectError");
    assert_eq!(l.errors[0].cases, ["CT_SELECT_LENGTH_MISMATCH"]);
    assert!(V1_CORPUS.contains(r#""types":{"SelectError":["CT_SELECT_LENGTH_MISMATCH"]}"#));

    // models: declaration order, bodies as v1 wrote them
    let order: Vec<&str> = l.models.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(order, ["same", "same_u64"]);
    for m in &l.models {
        let want = format!(
            r#""{}":{{"body":{},"params":["a","b"]}}"#,
            m.name,
            compact(&m.body)
        );
        assert!(V1_CORPUS.contains(&want), "{want}");
        assert_eq!(compact(&m.body), r#"["eq",["var","a"],["var","b"]]"#);
        assert_eq!(m.ret, Ty::Bool);
    }
    assert_eq!(
        l.models[0].params,
        [("a".to_string(), Ty::BYTES), ("b".to_string(), Ty::BYTES)]
    );
    assert_eq!(
        l.models[1].params,
        [
            ("a".to_string(), Ty::Fixed(FixedTy::U64)),
            ("b".to_string(), Ty::Fixed(FixedTy::U64))
        ]
    );

    // ops: v1 rows plus the v2 `requires`
    let rows: Vec<String> = l
        .ops
        .iter()
        .map(|o| {
            o.to_json()
                .render()
                .unwrap()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect()
        })
        .collect();
    assert_eq!(
        rows[0],
        r#"{"params":[["a","Bytes"],["b","Bytes"]],"requires":["ir.core"],"ret":"Bool"}"#
    );
    assert_eq!(
        rows[1],
        r#"{"params":[["a","Bytes<N>"],["b","Bytes<N>"]],"requires":["ir.core"],"ret":"Bool","width_params":["N"]}"#
    );
    assert_eq!(
        rows[2],
        r#"{"fresh":true,"params":[["a","Bytes"],["b","Bytes"],["choice","Bool"]],"requires":["ir.core"],"ret":"Result<Bytes,SelectError>"}"#
    );
    assert_eq!(
        rows[3],
        r#"{"params":[["a","U64"],["b","U64"]],"requires":["ir.core"],"ret":"Bool"}"#
    );
    // and the v1 corpus agrees on the shared fields
    for needle in [
        r#""eq":{"params":[["a","Bytes"],["b","Bytes"]],"ret":"Bool"}"#,
        r#""eq_fixed":{"params":[["a","Bytes<N>"],["b","Bytes<N>"]],"ret":"Bool","width_params":["N"]}"#,
        // (v1 is not key-sorted: `fresh` comes last)
        r#""select_bytes":{"params":[["a","Bytes"],["b","Bytes"],["choice","Bool"]],"ret":"Result<Bytes,SelectError>","fresh":true}"#,
        r#""eq_u64":{"params":[["a","U64"],["b","U64"]],"ret":"Bool"}"#,
    ] {
        assert!(V1_CORPUS.contains(needle), "{needle}");
    }
}

#[test]
fn ct_compare_suites_equal_the_v1_corpus_check_1() {
    let l = lowered(CT_COMPARE);
    assert_eq!(l.suites.len(), 9);
    for s in &l.suites {
        assert_eq!(s.id, format!("warrant/ct_compare/exhaustive/{}", s.claim));
        let SuiteKind::Reference {
            subject,
            expected,
            result,
        } = &s.kind
        else {
            panic!("{} must be a reference suite", s.claim)
        };
        // v1 wrote the three IR fields of a suite as admit / lhs / rhs, all compact.
        let at = V1_CORPUS
            .find(&format!(r#""claim":"{}""#, s.claim))
            .expect(&s.claim);
        // the suite object runs from its first key to the next suite's; "admit" is its
        // first key, and its keys are sorted, so the three fields sit around "claim".
        let start = V1_CORPUS[..at].rfind(r#"{"admit""#).expect("suite start");
        let end = start + V1_CORPUS[start..].find(r#""rhs""#).unwrap() + 400;
        let obj = &V1_CORPUS[start..end.min(V1_CORPUS.len())];
        assert!(
            obj.contains(&format!(r#""admit":{}"#, compact(&s.admit))),
            "{}: admit",
            s.claim
        );
        assert!(
            obj.contains(&format!(r#""lhs":{}"#, compact(subject))),
            "{}: lhs",
            s.claim
        );
        assert!(
            obj.contains(&format!(r#""rhs":{}"#, compact(expected))),
            "{}: rhs",
            s.claim
        );
        assert!(
            obj.contains(&format!(r#""result_type":"{}""#, result.name())),
            "{}: result",
            s.claim
        );
        assert!(
            obj.contains(&format!(r#""ops":["{}"]"#, s.ops[0])) && s.ops.len() == 1,
            "{}: ops",
            s.claim
        );
    }
}

#[test]
fn ct_compare_suites_by_hand() {
    let l = lowered(CT_COMPARE);
    let by = |claim: &str| l.suites.iter().find(|s| s.claim == claim).unwrap();

    let agrees = by("eq_agrees");
    assert_eq!(compact(&agrees.admit), r#"["lit",true]"#);
    assert_eq!(
        compact(&agrees.body),
        r#"["eq",["subject_op","eq",[["var","a"],["var","b"]]],["model_call","same",[["var","a"],["var","b"]]]]"#
    );
    assert_eq!(
        agrees.binders,
        [("a".to_string(), Ty::BYTES), ("b".to_string(), Ty::BYTES)]
    );

    let w4 = by("eq_fixed_agrees_w4");
    assert_eq!(w4.binders[0].1, Ty::bytes_n(4));
    let SuiteKind::Reference { subject, .. } = &w4.kind else {
        panic!()
    };
    assert_eq!(
        compact(subject),
        r#"["subject_op","eq_fixed",[["var","a"],["var","b"]],[4]]"#
    );
    assert_eq!(by("eq_fixed_agrees_w0").binders[0].1, Ty::bytes_n(0));

    let picks = by("select_picks");
    assert_eq!(
        compact(&picks.admit),
        r#"["eq",["len",["var","a"]],["len",["var","b"]]]"#
    );
    let SuiteKind::Reference {
        expected, result, ..
    } = &picks.kind
    else {
        panic!()
    };
    assert_eq!(
        compact(expected),
        r#"["ok",["if",["var","choice"],["var","a"],["var","b"]]]"#
    );
    assert_eq!(result.name(), "Result<Bytes,SelectError>");

    let rej = by("select_rejects_mismatch");
    assert_eq!(
        compact(&rej.admit),
        r#"["ne",["len",["var","a"]],["len",["var","b"]]]"#
    );
    let SuiteKind::Reference { expected, .. } = &rej.kind else {
        panic!()
    };
    assert_eq!(compact(expected), r#"["err","CT_SELECT_LENGTH_MISMATCH"]"#);

    let u = by("eq_u64_agrees");
    assert_eq!(u.binders[0].1, Ty::Fixed(FixedTy::U64));
    let SuiteKind::Reference { expected, .. } = &u.kind else {
        panic!()
    };
    assert_eq!(
        compact(expected),
        r#"["model_call","same_u64",[["var","a"],["var","b"]]]"#
    );
}

#[test]
fn ct_compare_examples_split_into_subject_and_expected() {
    let l = lowered(CT_COMPARE);
    assert_eq!(l.examples.len(), 12);
    let ok = l
        .examples
        .iter()
        .find(|x| x.name == "select_left_fresh")
        .unwrap();
    let r = ok.reference.as_ref().unwrap();
    assert_eq!(
        compact(&r.subject),
        r#"["subject_op","select_bytes",[["lit",{"type":"Bytes","value":"00"}],["lit",{"type":"Bytes","value":"ff"}],["lit",true]]]"#
    );
    assert_eq!(
        compact(&r.expected),
        r#"["ok",["lit",{"type":"Bytes","value":"00"}]]"#
    );
    assert_eq!(r.result.name(), "Result<Bytes,SelectError>");
    let u = l.examples.iter().find(|x| x.name == "u64_diff").unwrap();
    let r = u.reference.as_ref().unwrap();
    assert_eq!(compact(&r.expected), r#"["lit",false]"#);
    assert!(compact(&u.body).starts_with(
        r#"["eq",["subject_op","eq_u64",[["lit",{"type":"U64","value":"9223372036854775808"}]"#
    ));
    let w = l
        .examples
        .iter()
        .find(|x| x.name == "fixed_16_diff")
        .unwrap();
    assert!(compact(&w.reference.as_ref().unwrap().subject).ends_with("],[16]]"));
}

// ---- cost ------------------------------------------------------------------------

#[test]
fn ct_compare_costs_the_largest_expression_and_nothing_is_declared() {
    let a = with_file(CT_COMPARE, |f| analyse(f, &Default::default()).unwrap());
    let mut largest = 0;
    for (s, sc) in a.lowered.suites.iter().zip(&a.suite_costs) {
        largest = largest
            .max(sc.admit)
            .max(sc.body)
            .max(sc.expected.unwrap_or(0));
        let _ = s;
    }
    // VER04r 4.2 says "ct-compare's largest expression costs 9".  By the cost table the
    // `eq_agrees` body is eq(1) + subject_op(1+1+1) + model_call(1+1+1+body 3) = 10, and
    // 9 is `select_picks`' body (eq 1 + subject_op 4 + ok(if) 4).  The independent oracle
    // computes the same 10 (see the parity fixture), so the spec text is off by one.
    assert_eq!(largest, 10);
    let picks = a
        .lowered
        .suites
        .iter()
        .position(|s| s.claim == "select_picks")
        .unwrap();
    assert_eq!(a.suite_costs[picks].body, 9);
    assert_eq!(a.model_costs.cost("same"), Some(3));
    assert_eq!(a.model_costs.cost("same_u64"), Some(3));
    let totals: Vec<u64> = a.suite_costs.iter().map(|c| c.total).collect();
    // eq_agrees: admit 1 + body (1 + 3 + (1+2+3)) 10 + expected 6 = 17 ; w* the same ;
    // select_picks: admit 5 + body 9 + expected 4 = 18 ; select_rejects_mismatch:
    // admit 5 + body (1 + 4 + 1) 6 + expected 1 = 12
    assert_eq!(totals, [17, 17, 17, 17, 17, 17, 18, 12, 17]);
}

fn gf256() -> (Lowered, ModelCosts) {
    let l = lowered(GF256);
    let costs = ModelCosts::compute(&l.models, 32_768).expect("gf256 fits 32768");
    (l, costs)
}

#[test]
fn gf256_model_costs_match_the_worst_path_numbers_of_the_spec() {
    let (_, costs) = gf256();
    assert_eq!(costs.cost("gmul"), Some(2_011), "VER04r 4.2");
    assert_eq!(costs.cost("ginv"), Some(26_204), "VER04r 4.2");
    // xor1 = if a == b then 0u8 else 1u8 : 1 + 3 + max(1,1) = 5 ; xor2's heaviest path
    // calls xor1 on (a - 2u8, b - 2u8): 1 + 3 + (1 + 2*3 + 5) = ... pinned by the oracle
    assert_eq!(costs.cost("xor1"), Some(5));
    assert_eq!(costs.cost("xor8"), Some(110));
    assert_eq!(costs.depth("ginv"), Some(23));
}

#[test]
fn gf256_needs_the_declared_budget_and_four_suites_exceed_the_default() {
    let (l, costs) = gf256();
    let mut over_default = 0;
    let mut largest_expression = 0;
    for s in &l.suites {
        let c = suite_cost(s, &costs, 65_536).unwrap();
        if c.total > 2_000 {
            over_default += 1;
        }
        largest_expression = largest_expression
            .max(c.admit)
            .max(c.body)
            .max(c.expected.unwrap_or(0));
    }
    assert_eq!(l.suites.len(), 38);
    assert_eq!(
        over_default, 4,
        "VER04r 4.2: four of the 38 suites exceed 2,000"
    );
    assert_eq!(
        largest_expression, 26_210,
        "VER04r 4.2: the largest expression is 26,210"
    );

    // the default bound refuses it; the profile line is the only way through
    let e = with_file(GF256, |f| analyse(f, &Default::default()).unwrap_err());
    assert_eq!(e.code, code::LIMIT, "{e}");
    assert!(!e.at.is_empty(), "the refusal names the model");
    let profile = parse_profile(b"max_ir_cost 32768\n").unwrap();
    assert!(with_file(GF256, |f| analyse(f, &profile)).is_ok());
    // a ceiling-sized declared budget works too, but a smaller one than needed does not
    assert!(with_file(GF256, |f| analyse(
        f,
        &parse_profile(b"max_ir_cost 26000\n").unwrap()
    ))
    .is_err());
}

// ---- the tag set ---------------------------------------------------------------------

fn tags_of(l: &Lowered, out: &mut BTreeSet<&'static str>) {
    let mut visit = |ir: &Ir| {
        ir.walk(&mut |n| {
            out.insert(n.tag());
        })
    };
    for m in &l.models {
        visit(&m.body);
    }
    for s in &l.suites {
        visit(&s.admit);
        visit(&s.body);
    }
    for x in &l.examples {
        visit(&x.body);
    }
    for o in &l.ops {
        if let Some(a) = &o.admit {
            visit(a);
        }
    }
}

#[test]
fn the_tags_emitted_over_every_ir_core_construct_are_exactly_the_26_of_appendix_a() {
    let mut tags = BTreeSet::new();
    for src in [CT_COMPARE, CORE_V0, CORE_V1] {
        tags_of(&lowered(src), &mut tags);
    }
    // `is_err` is written by fault templates (S3c), never by a source construct.
    assert!(
        !tags.contains("is_err"),
        "no source construct lowers to is_err"
    );
    tags.insert("is_err");
    let want: BTreeSet<&str> = ALL_TAGS.iter().copied().collect();
    assert_eq!(tags, want);
    assert_eq!(want.len(), 26);
}

// ---- operand order and the desugarings -------------------------------------------------

fn model_body(l: &Lowered, name: &str) -> String {
    compact(&l.model(name).unwrap_or_else(|| panic!("{name}")).body)
}

#[test]
fn operand_order_and_desugarings_are_pinned() {
    let l = lowered(CORE_V0);
    // a - b + 2 * a : left-associative, source order, bare literal typed Nat next to a Nat
    assert_eq!(
        model_body(&l, "m_arith"),
        r#"["add",["sub",["var","a"],["var","b"]],["mul",["lit",{"type":"Nat","value":"2"}],["var","a"]]]"#
    );
    // a != b is `ne`; the chain of `or` is left-nested; `=>` is `implies`; groups vanish
    assert_eq!(
        model_body(&l, "m_logic"),
        r#"["or",["and",["var","a"],["var","b"]],["implies",["var","a"],["not",["var","b"]]]]"#
    );
    assert!(model_body(&l, "m_cmp").ends_with(r#"["ne",["var","a"],["var","b"]]]"#));
    assert_eq!(
        model_body(&l, "m_wide"),
        r#"["var","a"]"#,
        "((a)) is just `a`"
    );
    // unary minus is `neg`, and `-a - 1` is sub(neg(a), 1)
    assert_eq!(
        model_body(&l, "m_neg"),
        r#"["sub",["neg",["var","a"]],["lit",{"type":"Int","value":"1"}]]"#
    );
    // a bare literal outside a Nat context is Int
    assert_eq!(
        model_body(&l, "m_sign"),
        r#"["if",["gt",["var","a"],["lit",{"type":"Nat","value":"0"}]],["lit",{"type":"Int","value":"1"}],["neg",["lit",{"type":"Int","value":"1"}]]]"#
    );
    // a model call is not inlined
    assert_eq!(
        model_body(&l, "m_call"),
        r#"["sub",["model_call","m_arith",[["var","a"],["var","b"]]],["model_call","m_arith",[["var","b"],["var","a"]]]]"#
    );
    // a Bytes literal and concat nest in source order
    assert_eq!(
        model_body(&l, "m_cat"),
        r#"["concat",["var","a"],["concat",["var","b"],["lit",{"type":"Bytes","value":"00ff"}]]]"#
    );
    // Ok / Err take their types from the Result context
    assert_eq!(
        model_body(&l, "m_pick"),
        r#"["if",["var","c"],["ok",["var","a"]],["err","BAD"]]"#
    );
}

#[test]
fn nat_minus_nat_is_int_and_fixed_width_stays_fixed() {
    let l = lowered(CORE_V0);
    let Ir::Arith(_, ty, ..) = &l.model("m_arith").unwrap().body else {
        panic!()
    };
    assert_eq!(*ty, Ty::Int, "the outer `+` of Nat/Int operands is Int");
    let Ir::If(_, then, other) = &l.model("m_u8").unwrap().body else {
        panic!()
    };
    let Ir::Arith(Arith::Sub, t, ..) = &**then else {
        panic!()
    };
    assert_eq!(*t, Ty::Fixed(FixedTy::U8));
    let Ir::Arith(Arith::Add, t, ..) = &**other else {
        panic!()
    };
    assert_eq!(*t, Ty::Fixed(FixedTy::U8));
}

#[test]
fn refinements_fold_left_into_binary_and_and_a_single_one_is_not_wrapped() {
    let l = lowered(CORE_V0);
    let by = |c: &str| l.suites.iter().find(|s| s.claim == c).unwrap();
    let three = compact(&by("fold3").admit);
    assert_eq!(
        three,
        r#"["and",["and",["le",["var","a"],["lit",{"type":"U64","value":"100"}]],["le",["var","b"],["var","a"]]],["le",["var","c"],["var","b"]]]"#
    );
    // exactly one refinement: no `and`, no n-ary form
    assert_eq!(
        compact(&by("pick_ok").admit),
        r#"["eq",["len",["var","a"]],["len",["var","b"]]]"#
    );
    // none: `["lit", true]`
    assert_eq!(compact(&by("ref_left").admit), r#"["lit",true]"#);
    // and a claim with no quantifier at all has no binders
    let ground = by("ground");
    assert!(ground.binders.is_empty());
    assert_eq!(compact(&ground.admit), r#"["lit",true]"#);
}

#[test]
fn reference_and_predicate_follow_section_4_3() {
    let l = lowered(CORE_V0);
    let kind = |c: &str| l.suites.iter().find(|s| s.claim == c).unwrap().kind.clone();
    // subject on the left, on the right, and on both sides
    let SuiteKind::Reference {
        subject, expected, ..
    } = kind("ref_left")
    else {
        panic!()
    };
    assert!(subject.has_subject() && !expected.has_subject());
    let SuiteKind::Reference {
        subject,
        expected,
        result,
    } = kind("ref_right")
    else {
        panic!()
    };
    assert_eq!(
        compact(&subject),
        r#"["subject_op","f",[["var","a"],["var","b"]]]"#
    );
    assert_eq!(
        compact(&expected),
        r#"["model_call","m_lit",[["var","a"]]]"#
    );
    assert_eq!(result, Ty::Bool);
    for predicate in ["pred_two_calls", "pred_guard", "pred_if", "calls_models"] {
        assert!(
            matches!(kind(predicate), SuiteKind::Predicate),
            "{predicate} must be a predicate"
        );
    }
    // a reference suite's `ops` lists the subject operations by first occurrence
    let two = l
        .suites
        .iter()
        .find(|s| s.claim == "pred_two_calls")
        .unwrap();
    assert_eq!(two.ops, ["f"]);
    let nat = l.suites.iter().find(|s| s.claim == "nat_arith").unwrap();
    assert_eq!(nat.ops, ["e"]);
}

#[test]
fn v1_literals_keep_the_minimum_signed_value_as_a_neg_of_a_literal() {
    let l = lowered(CORE_V1);
    // VER04j 2.2: `-128i8` keeps both nodes; the literal holds -128 and `neg` wraps it to itself
    assert_eq!(
        model_body(&l, "w_min"),
        r#"["if",["eq",["var","a"],["neg",["lit",{"type":"I8","value":"-128"}]]],["neg",["lit",{"type":"I8","value":"-128"}]],["neg",["var","a"]]]"#
    );
    // hex literals lower like decimal ones, with the suffix's type
    assert_eq!(
        model_body(&l, "w_hex"),
        r#"["sub",["add",["var","a"],["lit",{"type":"U32","value":"255"}]],["lit",{"type":"U32","value":"1"}]]"#
    );
    // take and drop are ir.core in a v1 document
    assert_eq!(
        model_body(&l, "w_chain"),
        r#"["drop",["take",["var","a"],["lit",{"type":"Nat","value":"4"}]],["lit",{"type":"Nat","value":"1"}]]"#
    );
    assert_eq!(l.document(), "v1");
}

#[test]
fn op_refinements_become_the_ops_admit() {
    let l = lowered(CORE_V0);
    let h = l.op("h").unwrap();
    assert_eq!(
        compact(h.admit.as_ref().unwrap()),
        r#"["and",["le",["var","a"],["lit",{"type":"U64","value":"100"}]],["le",["var","b"],["var","a"]]]"#
    );
    assert!(l.op("f").unwrap().admit.is_none());
    let json = h.to_json().render().unwrap();
    assert!(json.contains("\"admit\""));
    let g = l.op("g").unwrap();
    assert_eq!(g.statics, ["N"]);
    assert_eq!(g.params[0].1.name(), "Bytes<N>");
    assert_eq!(g.ret.name(), "Bytes<N>");
    assert!(l.op("pick").unwrap().fresh);
    // gf256's `power` carries the refinement `exp <= 4294967295u64`
    let gf = lowered(GF256);
    assert_eq!(
        compact(gf.op("power").unwrap().admit.as_ref().unwrap()),
        r#"["le",["var","exp"],["lit",{"type":"U64","value":"4294967295"}]]"#
    );
}

#[test]
fn lowering_is_deterministic_and_the_view_is_stable() {
    let a = lowered(CT_COMPARE);
    let b = lowered(CT_COMPARE);
    assert_eq!(a, b);
}

#[test]
fn rendering_a_hand_built_tree_deeper_than_64_is_refused() {
    use coding_adventures_warrant_corpus_gen::view::oracle_view;
    let mut l = lowered(CT_COMPARE);
    let mut deep = Ir::Bool(true);
    for _ in 0..100 {
        deep = Ir::Not(Box::new(deep));
    }
    l.suites[0].admit = deep;
    // the cost walk and the renderer both refuse before recursing
    let e = oracle_view(&l, 65_536).unwrap_err();
    assert_eq!(e.code, code::LIMIT);
}

#[test]
fn the_example_that_was_a_predicate_is_gone_from_the_fixture() {
    let l = lowered(CORE_V0);
    assert!(l.examples.iter().all(|x| x.reference.is_some()));
}
