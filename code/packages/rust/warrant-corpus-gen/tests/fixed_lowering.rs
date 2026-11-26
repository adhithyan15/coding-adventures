//! Slice S10b: lowering, typing, `requires` and cost of the `ir.fixed` built-ins, on small
//! documents (the 35 built-ins at every width are covered byte for byte by `fixed_parity.rs`).
//!
//! What is pinned here and not by the dump: the refusals (`Option` confined to `unwrap_or`,
//! the level a refused unit is named by), the single-pass static width of `from_be` / `from_le`
//! (with a deep-nesting regression: the oracle's first version re-lowered the operand at every
//! level and was exponential), the `requires` rules of R9, and that the ordinary path still
//! refuses every `ir.fixed` document (generation stays a fail-closed stub).

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use coding_adventures_warrant_corpus_gen::cost::{suite_cost, ModelCosts};
use coding_adventures_warrant_corpus_gen::lower::{
    census, lower_document, ItemKind, Lowered,
};
use coding_adventures_warrant_corpus_gen::{code, GenError};
use support::*;

const OPS: [&str; 1] = ["op f(a: Bytes, b: Bytes) -> Bool"];

/// Lower `extra` (models, claims, examples) in a v1 document, `ir.fixed` included.
fn lowered(extra: &str) -> Result<Lowered, GenError> {
    with_file(&doc_ops(" v1", &OPS, extra), lower_document)
}

/// As [`lowered`] with a parser deep enough for a 40-level operand (the default stops earlier;
/// the lowering's own limit is 64).
fn lowered_deep(src: &str) -> Result<Lowered, GenError> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let limits = ParserLimits {
        max_depth: 128,
        ..ParserLimits::default()
    };
    let file = parse_document(&layout, &doc, limits).unwrap();
    lower_document(&file)
}

fn model_json(l: &Lowered, name: &str) -> String {
    l.model(name)
        .unwrap()
        .body
        .to_json()
        .render()
        .unwrap()
        .split_whitespace()
        .collect()
}

// ---- shapes and types -------------------------------------------------------------------------

#[test]
fn a_built_in_lowers_to_one_node_with_the_static_type_in_position_one() {
    let l = lowered(concat!(
        "  model w(a: U8) -> U32 = widen<U32>(a)\n",
        "  model mx() -> I8 = max_value<I8>()\n",
        "  model rl(a: U16, n: Nat) -> U16 = rotl(a, n)\n",
        "  model be(a: U32) -> Bytes<4> = be_bytes(a)\n",
    ))
    .unwrap();
    assert_eq!(model_json(&l, "w"), r#"["widen","U32",["var","a"]]"#);
    assert_eq!(model_json(&l, "mx"), r#"["max_value","I8"]"#);
    assert_eq!(model_json(&l, "rl"), r#"["rotl",["var","a"],["var","n"]]"#);
    assert_eq!(model_json(&l, "be"), r#"["be_bytes",["var","a"]]"#);
}

#[test]
fn nat_div_and_rem_are_option_int_and_unwrap_or_gives_the_inner_type() {
    // R7: two Nat operands give Option<Int>, so the default is an Int literal.
    let l = lowered(concat!(
        "  model d(a: Nat, b: Nat) -> Int = unwrap_or(div(a, b), 0)\n",
        "  model r(a: U8, b: U8) -> U8 = unwrap_or(rem(a, b), 0u8)\n",
        "  model t(a: I8, b: I8) -> I8 = unwrap_or(div_trunc(a, b), 0i8)\n",
        "  model n(a: Nat) -> U8 = unwrap_or(try<U8>(a), 0u8)\n",
        "  model nth(a: Bytes, i: Nat) -> U8 = unwrap_or(at(a, i), 0u8)\n",
    ))
    .unwrap();
    assert_eq!(
        model_json(&l, "d"),
        r#"["unwrap_or",["div",["var","a"],["var","b"]],["lit",{"type":"Int","value":"0"}]]"#
    );
    assert_eq!(l.model("d").unwrap().ret.name(), "Int");
    assert_eq!(l.model("n").unwrap().ret.name(), "U8");
}

#[test]
fn a_bare_literal_takes_the_other_operands_type() {
    let l = lowered(concat!(
        "  model m(n: Nat) -> Int = unwrap_or(div(7, n), 0)\n",
        "  model s(a: U8) -> U8 = shl(a, 3)\n",
        "  model i(n: Nat) -> Int = imin(n, 3)\n",
    ))
    .unwrap();
    assert!(model_json(&l, "m").contains(r#"["lit",{"type":"Nat","value":"7"}]"#));
    assert!(model_json(&l, "s").contains(r#"["lit",{"type":"Nat","value":"3"}]"#));
    assert!(model_json(&l, "i").contains(r#"["lit",{"type":"Int","value":"3"}]"#));
}

// ---- Option is a real type of ir.data; the intermediate use adds nothing (D48, R29) ---------------

#[test]
fn an_option_lowers_everywhere_and_needs_ir_data_except_under_unwrap_or() {
    // S11b: a model result, an op result, a parameter and a binder may be an `Option` ([D48]); each
    // unit `requires` `ir.data` (R9, the type half).  Before S11b each was refused at `ir.data`.
    let cases: [(&[&str], &str); 4] = [
        (
            &OPS,
            "  model m(a: U8, b: U8) -> Option<U8> = checked_add(a, b)\n",
        ),
        (&["op f(a: Bytes, b: Bytes) -> Option<U8>"], ""),
        (&OPS, "  model p(x: Option<U8>) -> Bool = true\n"),
        (
            &OPS,
            "  claim c:\n    forall x: Option<U8>. f(b\"\", b\"\") == true\n",
        ),
    ];
    for (ops, extra) in cases {
        let src = doc_ops(" v1", ops, extra);
        let l = with_file(&src, lower_document).unwrap_or_else(|e| panic!("{extra}: {e}"));
        let all: Vec<&[&str]> = l
            .ops
            .iter()
            .map(|o| o.requires.as_slice())
            .chain(l.models.iter().map(|m| m.requires.as_slice()))
            .chain(l.suites.iter().map(|s| s.requires.as_slice()))
            .collect();
        assert!(all.iter().any(|r| r.contains(&"ir.data")), "{extra}: {all:?}");
    }
    // an Option operand of anything but unwrap_or's first operand, and an Option in a branch, lowers
    // (or is a typing error the checker names: `unwrap_or(0u8, ..)` has the wrong operand order)
    let operands = [
        "  model m(a: U8, b: U8) -> Bool = checked_add(a, b) == checked_add(b, a)\n",
        "  model m(a: U8, b: U8) -> U8 = unwrap_or(0u8, checked_add(a, b))\n",
        "  model m(a: U8, b: U8) -> U8 = unwrap_or(if true then checked_add(a, b) else checked_add(b, a), 0u8)\n",
    ];
    for extra in operands {
        let src = doc_ops(" v1", &OPS, extra);
        match with_file(&src, lower_document) {
            Ok(l) => assert!(l.models[0].requires.contains(&"ir.data"), "{extra}"),
            Err(e) => assert!(e.code == code::CHECK || e.code == code::TYPE, "{extra}: {e}"),
        }
    }
    // the intermediate use: `unwrap_or(producer(..), d)` is `ir.fixed` and no `ir.data` (R29 (b), (d))
    let src = doc_ops(
        " v1",
        &OPS,
        "  model m(a: U8, b: U8) -> U8 = unwrap_or(checked_add(a, b), 255u8)\n",
    );
    let l = with_file(&src, lower_document).unwrap();
    assert_eq!(l.models[0].requires, ["ir.core", "ir.fixed"]);
}

// ---- the static width of from_be / from_le: one pass, the checker's rule ---------------------------

#[test]
fn from_be_takes_the_static_width_of_a_literal_a_concat_and_an_if() {
    let l = lowered(concat!(
        "  model lit(c: Bool) -> U16 = from_be<U16>(b\"0102\")\n",
        "  model cat(c: Bool) -> U32 = from_le<U32>(concat(b\"0102\", b\"0304\"))\n",
        "  model iff(c: Bool) -> U16 = from_be<U16>(if c then b\"0102\" else b\"0304\")\n",
        "  model var(a: Bytes<8>) -> U64 = from_be<U64>(a)\n",
        "  model nested(c: Bool) -> U32 = from_be<U32>(concat(concat(b\"01\", b\"02\"), if c then b\"0304\" else b\"0506\"))\n",
    ));
    let l = l.unwrap();
    assert_eq!(l.models.len(), 5);
    assert_eq!(l.model("lit").unwrap().ret.name(), "U16");
}

#[test]
fn from_be_of_the_wrong_width_is_refused_as_a_malformed_document() {
    for body in [
        "from_be<U32>(b\"0102\")",
        "from_be<U16>(concat(b\"01\", b\"0203\"))",
        "from_le<U16>(if c then b\"0102\" else b\"01\")",
    ] {
        let src = doc_ops(
            " v1",
            &OPS,
            &format!("  model m(c: Bool) -> U16 = {body}\n"),
        );
        let e = with_file(&src, |f| lower_document(f).unwrap_err());
        assert!(e.code == code::CHECK || e.code == code::TYPE, "{body}: {e}");
    }
    // a dynamic Bytes parameter has no static width
    let src = doc_ops(
        " v1",
        &OPS,
        "  model m(a: Bytes) -> U16 = from_be<U16>(a)\n",
    );
    let e = with_file(&src, |f| lower_document(f).unwrap_err());
    assert!(e.code == code::CHECK || e.code == code::TYPE, "{e}");
}

/// Regression for the exponential lowering the oracle's first S10a version had: it lowered the
/// operand of `from_be` a second time to read its width, so a chain of nested operands cost
/// 2^depth.  The width is now read from the one pass that lowers the operand, so a deep chain is
/// linear.  (40 levels would be 2^40 lowerings, never finishing, under the old rule.)
#[test]
fn a_deeply_nested_from_be_operand_is_lowered_once() {
    let mut operand = String::from("b\"0102\"");
    for _ in 0..40 {
        operand = format!("concat({operand}, b\"\")");
    }
    let src = doc_ops(
        " v1",
        &OPS,
        &format!("  model deep(c: Bool) -> U16 = from_be<U16>({operand})\n"),
    );
    let start = Instant::now();
    let l = lowered_deep(&src).expect("lowers");
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(l.models.len(), 1);
    // and the same through nested `if`s of one width
    let mut operand = String::from("b\"0102\"");
    for _ in 0..30 {
        operand = format!("if c then {operand} else b\"0304\"");
        operand = format!("({operand})");
    }
    let src = doc_ops(
        " v1",
        &OPS,
        &format!("  model deepif(c: Bool) -> U16 = from_le<U16>({operand})\n"),
    );
    let start = Instant::now();
    assert!(lowered_deep(&src).is_ok());
    assert!(start.elapsed() < Duration::from_secs(10));
}

// ---- requires (R9) ---------------------------------------------------------------------------------

#[test]
fn requires_is_the_sorted_union_of_tag_levels_and_callee_requires() {
    let extra = concat!(
        "  model pure(a: U8) -> U8 = a\n",
        "  model fx(a: U8) -> U8 = bit_not(a)\n",
        "  model uses_fx(a: U8) -> U8 = pure(fx(a))\n",
        "  model uses_uses(a: U8) -> U8 = uses_fx(a)\n",
        "  claim cp:\n    forall a: U8. f(b\"\", b\"\") == (pure(a) == a)\n",
        "  claim cf:\n    forall a: U8. f(b\"\", b\"\") == (uses_uses(a) == 0u8)\n",
        "  claim cb:\n    forall a: U8 | popcount(a) < 4. f(b\"\", b\"\") == true\n",
        "  claim cg:\n    forall a: Bytes, b: Bytes. g(a, b) == true\n",
        "  example ep:\n    f(b\"\", b\"\") == true\n",
        "  example ef:\n    f(b\"\", b\"\") == (bit_not(1u8) == 0u8)\n",
    );
    let ops = [
        "op f(a: Bytes, b: Bytes) -> Bool",
        "op g(a: Bytes, b: Bytes | bit(1u8, 0)) -> Bool",
    ];
    let l = with_file(&doc_ops(" v1", &ops, extra), lower_document).unwrap();
    let core: Vec<&str> = vec!["ir.core"];
    let both: Vec<&str> = vec!["ir.core", "ir.fixed"];
    let req = |n: &str| l.model(n).unwrap().requires.clone();
    assert_eq!(req("pure"), core);
    assert_eq!(req("fx"), both);
    assert_eq!(
        req("uses_fx"),
        both,
        "a callee's own requires, not a re-walk"
    );
    assert_eq!(req("uses_uses"), both);
    let suite = |n: &str| {
        l.suites
            .iter()
            .find(|s| s.claim == n)
            .unwrap()
            .requires
            .clone()
    };
    assert_eq!(suite("cp"), core);
    assert_eq!(suite("cf"), both, "through the model closure");
    assert_eq!(
        suite("cb"),
        both,
        "a refinement is part of the unit (admit)"
    );
    // R9 reads the unit's own tags and models: an op's admit is the op's, not the claim's
    assert_eq!(suite("cg"), core);
    assert_eq!(l.op("f").unwrap().requires, core);
    assert_eq!(l.op("g").unwrap().requires, both);
    let ex = |n: &str| {
        l.examples
            .iter()
            .find(|e| e.name == n)
            .unwrap()
            .requires
            .clone()
    };
    assert_eq!(ex("ep"), core);
    assert_eq!(ex("ef"), both);
}

#[test]
fn an_ir_core_document_keeps_requires_ir_core_everywhere() {
    for src in [CT_COMPARE, GF256] {
        with_file(src, |f| {
            let l = lower_document(f).unwrap();
            let core = vec!["ir.core"];
            assert!(l.ops.iter().all(|o| o.requires == core));
            assert!(l.models.iter().all(|m| m.requires == core));
            assert!(l.suites.iter().all(|s| s.requires == core));
            assert!(l.examples.iter().all(|e| e.requires == core));
        });
    }
}

// ---- cost -------------------------------------------------------------------------------------------

#[test]
fn each_built_in_costs_one_plus_its_operands() {
    let l = lowered(concat!(
        "  model and2(a: U8, b: U8) -> U8 = bit_and(a, b)\n",
        "  model mx() -> U8 = max_value<U8>()\n",
        "  model wd(a: U8) -> U16 = widen<U16>(a)\n",
        "  model un(a: U8, b: U8) -> U8 = unwrap_or(div(a, b), 0u8)\n",
        "  model nested(a: U8, b: U8) -> U8 = bit_xor(and2(a, b), bit_not(a))\n",
        "  claim c:\n    forall a: U8, b: U8. f(b\"\", b\"\") == (bit_and(a, b) == 0u8)\n",
    ))
    .unwrap();
    let costs = ModelCosts::compute(&l.models, 2_000).unwrap();
    assert_eq!(costs.cost("and2"), Some(3)); // 1 + a + b
    assert_eq!(costs.cost("mx"), Some(1)); // the static type argument is not an operand
    assert_eq!(costs.cost("wd"), Some(2));
    assert_eq!(costs.cost("un"), Some(1 + (1 + 1 + 1) + 1)); // both operands, never a max
                                                             // bit_xor(1 + [and2: 1 + 1 + 1 + 3], bit_not: 1 + 1) = 1 + 6 + 2
    assert_eq!(costs.cost("nested"), Some(9));
    // admit 1 + body (eq: 1 + subject 1 + bit_and..) is counted per suite
    let sc = suite_cost(&l.suites[0], &costs, 2_000).unwrap();
    assert_eq!(sc.admit, 1);
    assert_eq!(sc.total, sc.admit + sc.body + sc.expected.unwrap_or(0));
}

// ---- the level a refused unit is named by -------------------------------------------------------------

#[test]
fn a_unit_is_named_by_its_first_unlowerable_construct_and_never_by_ir_fixed() {
    let extra = concat!(
        "  model only_fixed(a: U8) -> U8 = bit_not(a)\n",
        "  model fixed_then_tuple(a: U8) -> U8 = if bit_not(a) == 0u8 then (a, a).0 else a\n",
        "  model tuple_then_fixed(a: U8) -> U8 = if (a, a).0 == 0u8 then bit_not(a) else a\n",
        "  model fixed_then_ctor(a: U8) -> U8 = if bit_not(a) == 0u8 then unwrap_or(Some(a), 0u8) else a\n",
    );
    let src = doc_ops(" v1", &OPS, extra);
    let rows = with_file(&src, |f| census(f).unwrap());
    let level = |n: &str| {
        rows.iter()
            .find(|r| r.kind == ItemKind::Model && r.name == n)
            .and_then(|r| r.blocked.as_ref())
            .and_then(|b| b.level)
    };
    // S10c-2 evaluates `ir.fixed` (and S10t-2 `ir.tuple`): a unit that uses only those is not
    // blocked at all, and a constructor (`Some`, `ir.data`) lowers since S11b: nothing blocks it
    assert_eq!(level("only_fixed"), None);
    assert_eq!(level("fixed_then_tuple"), None);
    assert_eq!(level("tuple_then_fixed"), None);
    assert_eq!(level("fixed_then_ctor"), None);
    let blocked = |n: &str| {
        rows.iter()
            .find(|r| r.kind == ItemKind::Model && r.name == n)
            .map(|r| r.blocked.is_some())
    };
    assert_eq!(blocked("only_fixed"), Some(false));
}

// ---- generation takes an ir.fixed document -------------------------------------------------------------

#[test]
fn the_ordinary_path_lowers_an_ir_fixed_document_as_a_whole_since_s10c_2() {
    let extra = "  model fx(a: U8) -> U8 = bit_not(a)\n  claim c:\n    forall a: U8. f(b\"\", b\"\") == (fx(a) == 0u8)\n";
    let src = doc_ops(" v1", &OPS, extra);
    let l = with_file(&src, |f| lower_document(f).unwrap());
    assert_eq!(l.model("fx").unwrap().requires, ["ir.core", "ir.fixed"]);
    assert!(l.suites[0].requires.contains(&"ir.fixed"));
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

/// The real binary, on a document the generator refuses (`arithmetic.warrant`: `ir.data` lowers since S11b and
/// is generated since S11c-2, but its `ripple_carry_adder_with_carry_4_bit_exhaustive` claim has nine binders
/// and a suite has at most eight, VER04r 5.3): refused, nothing is written.  (`vos-sched-any.warrant`, which this
/// test used while `ir.data` was a lowering refusal, is refused by the `W0701` rule of an example now that it
/// lowers.)  (A refusal exits 2, the oracle's convention; 1 is reserved for a `--check` drift.)
#[test]
fn the_binary_refuses_a_document_beyond_the_evaluated_levels_and_writes_nothing() {
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let dir = std::env::temp_dir().join(format!("wcg-fixed-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let profile = dir.join("corpus.profile");
    fs::write(&profile, "").unwrap();
    let out = dir.join("out");
    for check in [false, true] {
        let mut cmd = Command::new(bin);
        cmd.current_dir(root()).args([
            "--warrant",
            "code/specs/arithmetic.warrant",
            "--profile",
            profile.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        if check {
            cmd.arg("--check");
        }
        let r = cmd.output().unwrap();
        assert_eq!(
            r.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&r.stderr)
        );
        let e = String::from_utf8_lossy(&r.stderr);
        // S11c-2 generates `ir.data`, so arithmetic is no longer refused for its level: it is refused for the
        // bound its nine-binder `ripple_carry_adder_with_carry_4_bit_exhaustive` claim breaks (VER04r 5.3)
        assert!(e.contains("more than 8 binders"), "{e}");
        assert!(!out.exists(), "nothing is written for a refused document");
    }
    let _ = fs::remove_dir_all(dir);
}
