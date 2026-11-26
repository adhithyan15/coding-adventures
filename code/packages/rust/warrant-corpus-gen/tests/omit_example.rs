//! `omit_example` (CV2-S10-1, generator half; VER04r 8.4, owner decisions [D22] and [D34],
//! refinement R4).
//!
//! An `omit_example <id>` profile line removes one example from `corpus.json` when the tool can
//! decide that it is inexpressible, and records the *computed* reason in the conditional key
//! `omitted_examples`.  The reason is `not-op-eq-pure` (the shape, tested first) or
//! `value-over-limit` (the normative five-step order of 8.4).
//!
//! This file mirrors `python/warrant-oracle/tests/test_omit_example.py` case by case, and pins
//! the **byte parity** with the oracle half: the committed `omit-example-v2` fixture is exactly
//! what this generator writes from its `omit.warrant` and `corpus.profile`.

mod support;

use coding_adventures_warrant_corpus_gen::lower::lower_document_omitting;
use coding_adventures_warrant_corpus_gen::{code, generate, CorpusV2, GenError, GenInput};
use support::minijson::parse;
use support::*;

const FIXTURE_WARRANT: &str =
    include_str!("../../../python/warrant-oracle/tests/fixtures/omit-example-v2/omit.warrant");
const FIXTURE_PROFILE: &str =
    include_str!("../../../python/warrant-oracle/tests/fixtures/omit-example-v2/corpus.profile");
const FIXTURE_PATH: &str =
    "code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/omit.warrant";

// ---- byte parity with the oracle half ---------------------------------------------------------

fn fixture() -> CorpusV2 {
    gen_at(FIXTURE_WARRANT, FIXTURE_PATH, FIXTURE_PROFILE).expect("the omit-example fixture generates")
}

fn gen_at(src: &str, path: &str, profile: &str) -> Result<CorpusV2, GenError> {
    with_file(src, |f| {
        generate(&GenInput {
            file: f,
            spec_path: path,
            spec_bytes: src.as_bytes(),
            profile_bytes: profile.as_bytes(),
            binding_faults: None,
        })
    })
}

/// The committed fixture is exactly what this generator writes: `corpus.json`, `cases.tfv` and
/// the one chunk file, byte for byte (the oracle's `make_fixture.py` wrote them from its own
/// `derive`; its BUILD checks them and `verify-v2`).
#[test]
fn the_committed_omit_example_fixture_is_what_the_generator_writes() {
    let out = fixture();
    assert_eq!(
        out.corpus_json,
        include_str!("../../../python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/corpus.json")
    );
    assert_eq!(
        out.cases_tfv,
        include_str!("../../../python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/cases.tfv")
    );
    let committed: [(&str, &str); 1] = [(
        "echo_is_identity.txt",
        include_str!("../../../python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/chunks/echo_is_identity.txt"),
    )];
    let have: Vec<(&str, &str)> = out.chunk_files.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    assert_eq!(have, committed);
}

#[test]
fn the_fixture_omits_three_examples_and_keeps_one() {
    let c = parse(&fixture().corpus_json).unwrap();
    let rows: Vec<(String, String)> = c
        .get("omitted_examples")
        .a()
        .iter()
        .map(|r| (r.get("id").s().to_string(), r.get("reason").s().to_string()))
        .collect();
    assert_eq!(
        rows,
        [
            (BIG.to_string(), "value-over-limit".to_string()),
            (SIZE.to_string(), "value-over-limit".to_string()),
            (TWO.to_string(), "not-op-eq-pure".to_string()),
        ]
    );
    let kept = c.get("examples").a();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].get("id").s(), "warrant/omitx/example/echo_small");
    assert_eq!(fixture().cases_tfv.matches('\n').count(), 2);
    assert!(!fixture().cases_tfv.contains("echo_big"));
}

/// A mutated profile or document does not reproduce the committed bytes: the comparison can fail.
#[test]
fn a_mutated_fixture_loses_parity() {
    let drop_last = FIXTURE_PROFILE.replace("omit_example warrant/omitx/example/echo_two_sided\n", "");
    assert_ne!(drop_last, FIXTURE_PROFILE);
    // the example is then refused (a shape that is not op == pure, without its line)
    let e = gen_at(FIXTURE_WARRANT, FIXTURE_PATH, &drop_last).unwrap_err();
    assert_eq!(e.code, code::W0701, "{e}");
    let one_less = FIXTURE_PROFILE.replace("omit_example warrant/omitx/example/echo_big_value\n", "");
    // CV2-F1: the 4,096-byte cap is enforced at emission, so an over-limit example without its
    // line is a refusal (code `limit`), no longer an emitted row.
    let e = gen_at(FIXTURE_WARRANT, FIXTURE_PATH, &one_less).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::LIMIT, "echo_big_value"), "{e}");
}

// ---- a small document builder (the same document the oracle's tests build) --------------------

const BIG: &str = "warrant/omitx/example/echo_big_value";
const SIZE: &str = "warrant/omitx/example/echo_size_error";
const TWO: &str = "warrant/omitx/example/echo_two_sided";

fn nest(n: usize, inner: &str) -> String {
    let mut s = inner.to_string();
    for _ in 0..n {
        s = format!("x2({s})");
    }
    s
}

/// 2 * 2^13 = 16,384 bytes: over 4,096, under `MAX_IR_VALUE`.
fn grow13() -> String {
    nest(13, "b\"aa\"")
}

/// 2 * 2^17 = 262,144 bytes: past `MAX_IR_VALUE` (a size error).
fn grow17() -> String {
    nest(17, "b\"aa\"")
}

const OPS: [(&str, &str, &str); 6] = [
    ("echo", "op echo(a: Bytes) -> Bytes", "a"),
    ("lo", "op lo(a: U8 | a < 10u8, b: Bytes) -> Bytes", "ab"),
    ("wr", "op wr(a: U8, b: Bytes) -> Bytes", "ab"),
    ("nn", "op nn(a: Bytes, b: Bytes) -> Bytes", "ab"),
    ("nb", "op nb(b: Bytes, a: Bytes) -> Bytes", "ab"),
    ("res", "op res(a: Bytes) -> Result<Bytes, E>", "a"),
];

fn doc_with_ops(examples: &str, version: &str, extra_ops: &[(&str, &str, &str)]) -> String {
    let ops: Vec<(&str, &str, &str)> = OPS.iter().chain(extra_ops).copied().collect();
    let mut s = format!(
        "warrant omitx{}\n  error E = BAD\n",
        if version == "v0" { "" } else { " v1" }
    );
    s += "  model x2(a: Bytes) -> Bytes = concat(a, a)\n";
    s += "  model same(a: Bytes, b: Bytes) -> Bool = a == b\n";
    s += "  model sq(a: Int) -> Int = a * a\n";
    // `Int` is model-internal (VER04r 2.1): an op signature may not name it, so the exact-window overflow is
    // raised inside a model whose result is Bytes (as in the oracle's test_omit_example.py).
    s += "  model ov(a: Bytes) -> Bytes = if sq(sq(sq(sq(10000000000)))) > 0 then a else a\n";
    for (_, line, _) in &ops {
        s += &format!("  {line}\n");
    }
    s += "  assume platform:\n    none\n  assume environment:\n    none\n";
    for (op, _, params) in &ops {
        s += &format!("  labels {op}:\n");
        for p in params.chars() {
            s += &format!("    {p} secret\n");
        }
    }
    for (op, _, params) in &ops {
        for p in params.chars() {
            s += &format!("  input {op}.{p}:\n    hostile\n    ensures no_panic, terminates\n");
        }
    }
    for (op, _, _) in &ops {
        s += &format!("  bound {op}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    }
    s += "  claim c:\n    forall a: Bytes<1>. echo(a) == a\n";
    s + examples
}

fn ex(name: &str, body: &str) -> String {
    format!("  example {name}:\n    {body}\n")
}

fn omit(name: &str) -> String {
    format!("omit_example warrant/omitx/example/{name}\n")
}

fn omits(names: &[&str]) -> String {
    names.iter().map(|n| omit(n)).collect()
}

fn try_gen(examples: &str, profile: &str, version: &str) -> Result<CorpusV2, GenError> {
    gen_at(&doc_with_ops(examples, version, &[]), "o.warrant", profile)
}

fn gen(examples: &str, profile: &str) -> CorpusV2 {
    try_gen(examples, profile, "v1").unwrap_or_else(|e| panic!("{e}"))
}

/// `{example name: reason}` of a generated corpus, in the order of the key.
fn reasons(out: &CorpusV2) -> Vec<(String, String)> {
    let c = parse(&out.corpus_json).unwrap();
    match c.has("omitted_examples") {
        false => Vec::new(),
        true => c
            .get("omitted_examples")
            .a()
            .iter()
            .map(|r| {
                (
                    r.get("id").s().rsplit('/').next().unwrap().to_string(),
                    r.get("reason").s().to_string(),
                )
            })
            .collect(),
    }
}

fn all(names: &[&str], reason: &str) -> Vec<(String, String)> {
    names.iter().map(|n| (n.to_string(), reason.to_string())).collect()
}

fn refused(examples: &str, names: &[&str], version: &str) -> GenError {
    try_gen(examples, &omits(names), version).expect_err("must be refused")
}

// ---- the profile line --------------------------------------------------------------------------

#[test]
fn an_unknown_id_is_a_profile_error_that_does_not_echo_it() {
    let e = refused(&ex("e", "echo(b\"aa\") == b\"aa\""), &["nope"], "v1");
    assert_eq!(e.code, code::PROFILE, "{e}");
    assert!(e.message.contains("names no example"), "{e}");
    assert!(!e.to_string().contains("nope"), "{e}");
    // the id of another module is not an example of this one either, nor is a bare name
    for line in ["omit_example warrant/other/example/e\n", "omit_example e\n"] {
        let e = try_gen(&ex("e", "echo(b\"aa\") == b\"aa\""), line, "v1").unwrap_err();
        assert_eq!(e.code, code::PROFILE, "{line}");
    }
}

#[test]
fn a_repeated_or_malformed_omit_example_line_is_a_profile_error() {
    let examples = ex("t", "echo(b\"aa\") != echo(b\"bb\")");
    for profile in [
        "omit_example warrant/omitx/example/t\nomit_example warrant/omitx/example/t\n",
        "omit_example\n",
        "omit_example a b\n",
        "omit_example  a\n",
        "omit_example a \n",
        "omit_examples a\n",
        "omit_example warrant/omitx/example/t\n \n",
    ] {
        let e = try_gen(&examples, profile, "v1").unwrap_err();
        assert_eq!(e.code, code::PROFILE, "{profile:?}: {e}");
    }
}

#[test]
fn an_emittable_example_is_a_stale_profile_and_the_message_names_only_the_id() {
    let e = refused(&ex("small", "echo(b\"ab\") == b\"ab\""), &["small"], "v1");
    assert_eq!(e.code, code::PROFILE, "{e}");
    assert!(e.message.contains("emittable"), "{e}");
    assert!(e.message.contains("warrant/omitx/example/small"), "{e}");
    assert!(!e.to_string().contains("6162") && !e.to_string().contains("b\"ab\""), "{e}");
}

#[test]
fn a_document_with_zero_examples_left_is_accepted() {
    let out = gen(&ex("t", "echo(b\"aa\") != echo(b\"bb\")"), &omit("t"));
    let c = parse(&out.corpus_json).unwrap();
    assert!(c.get("examples").a().is_empty());
    assert_eq!(out.cases_tfv, "tfv 1\n");
    assert_eq!(reasons(&out), all(&["t"], "not-op-eq-pure"));
}

// ---- the two reasons ---------------------------------------------------------------------------

#[test]
fn not_op_eq_pure_is_decided_from_the_shape() {
    let shapes = [
        ("ne", "echo(b\"aa\") != echo(b\"bb\")"),
        ("two", "echo(b\"aa\") == echo(b\"aa\")"), // a subject on both sides
        ("pure", "b\"aa\" == b\"aa\""),             // no subject at all
        ("nested", "x2(echo(b\"aa\")) == b\"aaaa\""), // the subject is not the top of its side
        ("inner", "echo(echo(b\"aa\")) == b\"aa\""), // a subject call inside an argument
        ("inner_right", "b\"aa\" == echo(echo(b\"aa\"))"),
        ("inner_two", "lo(1u8, echo(b\"aa\")) == b\"aa\""),
        ("lt", "x2(b\"aa\") != echo(b\"aa\")"), // not an equality
        ("boolean", "same(echo(b\"aa\"), b\"aa\")"), // a Bool call with a subject inside
    ];
    let examples: String = shapes.iter().map(|(n, b)| ex(n, b)).collect();
    let mut names: Vec<&str> = shapes.iter().map(|(n, _)| *n).collect();
    let out = gen(&examples, &omits(&names));
    names.sort_unstable(); // the key is sorted by id
    assert_eq!(reasons(&out), all(&names, "not-op-eq-pure"));
}

/// Impure guards (VER04r 4.4) and the like are the *lowering's* refusals for an example that is
/// not omitted, but the oracle's lowering never applies them to an example, so an omitted
/// example of such a shape is classified `not-op-eq-pure`, not refused.
#[test]
fn an_omitted_example_is_not_refused_for_its_guards() {
    let impure = ex("g", "echo(b\"aa\") == b\"aa\" and echo(b\"bb\") == b\"bb\"");
    let out = gen(&impure, &omit("g"));
    assert_eq!(reasons(&out), all(&["g"], "not-op-eq-pure"));
    // and without the line the same example is the generator's W0701
    let e = try_gen(&impure, "", "v1").unwrap_err();
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "g");
}

#[test]
fn a_shape_that_is_not_op_eq_pure_without_the_line_is_the_generators_w0701() {
    for body in [
        "echo(echo(b\"aa\")) == b\"aa\"",
        "echo(b\"aa\") != echo(b\"bb\")",
        "echo(b\"aa\") == echo(b\"aa\")",
    ] {
        let e = try_gen(&ex("s", body), "", "v1").unwrap_err();
        assert_eq!(e.code, code::W0701, "{e}");
        assert_eq!(e.at, "s");
        assert!(e.message.contains("one subject call"), "{e}");
    }
}

#[test]
fn value_over_limit_by_a_completed_evaluation_with_a_final_value_over_4096_bytes() {
    let g = grow13();
    let out = gen(
        &(ex("argbig", &format!("echo({g}) == b\"aa\"")) + &ex("expbig", &format!("echo(b\"aa\") == {g}"))),
        &omits(&["argbig", "expbig"]),
    );
    assert_eq!(reasons(&out), all(&["argbig", "expbig"], "value-over-limit"));
}

#[test]
fn the_limit_is_4096_bytes_inclusive() {
    let (at, over) = ("61".repeat(4096), "61".repeat(4097));
    let e = refused(&ex("at", &format!("echo(b\"{at}\") == b\"{at}\"")), &["at"], "v1");
    assert_eq!(e.code, code::PROFILE, "emittable: stale: {e}");
    let out = gen(&ex("over", &format!("echo(b\"{over}\") == b\"aa\"")), &omit("over"));
    assert_eq!(reasons(&out), all(&["over"], "value-over-limit"));
}

#[test]
fn a_bytes_value_under_ok_counts_toward_the_limit() {
    let out = gen(&ex("r", &format!("res(b\"aa\") == Ok({})", grow13())), &omit("r"));
    assert_eq!(reasons(&out), all(&["r"], "value-over-limit"));
    for body in ["res(b\"aa\") == Ok(b\"aa\")", "res(b\"aa\") == Err(E.BAD)"] {
        let e = refused(&ex("r", body), &["r"], "v1");
        assert_eq!(e.code, code::PROFILE, "{body}: {e}");
    }
}

#[test]
fn cv2_f1_a_non_omitted_example_over_4096_bytes_is_refused_at_emission() {
    let (at, over) = ("61".repeat(4096), "61".repeat(4097));
    // exactly 4,096 is emitted; 4,097 is not
    assert!(try_gen(&ex("at", &format!("echo(b\"{at}\") == b\"{at}\"")), "", "v1").is_ok());
    let g = grow13();
    for (what, body) in [
        ("argument", format!("echo({g}) == b\"aa\"")),
        ("pure value", format!("echo(b\"aa\") == {g}")),
        ("under ok", format!("res(b\"aa\") == Ok({g})")),
        ("literal", format!("echo(b\"{over}\") == b\"aa\"")),
    ] {
        let e = try_gen(&ex("big", &body), "", "v1").unwrap_err();
        assert_eq!((e.code, e.at.as_str()), (code::LIMIT, "big"), "{what}: {e}");
        // the omit decision is unchanged: with the line it is still value-over-limit
        let out = gen(&ex("big", &body), &omit("big"));
        assert_eq!(reasons(&out), all(&["big"], "value-over-limit"), "{what}");
    }
}

#[test]
fn cv2_f1_a_size_error_in_a_callees_admit_refuses_even_with_the_line() {
    // the admit itself grows a Bytes past MAX_IR_VALUE: a D19-check size error, not an argument's
    let big = (
        "bigadm",
        &*format!("op bigadm(a: Bytes | same({}, a)) -> Bytes", grow17()),
        "a",
    );
    let body = "bigadm(b\"aa\") == b\"aa\"";
    let src = |ex_| doc_with_ops(&ex(ex_, body), "v1", &[big]);
    for profile in [String::new(), omit("e")] {
        let e = gen_at(&src("e"), "o.warrant", &profile).unwrap_err();
        assert_eq!(e.code, code::W0701, "{e}");
        assert!(!e.size, "the flag does not survive admits_call: {e}");
        assert_ne!(e.at, "", "{e}");
    }
}

#[test]
fn both_reasons_apply_so_the_shape_wins() {
    // a size error *and* a shape that is not op == pure
    let body = format!("echo({}) != echo({})", grow17(), grow13());
    let out = gen(&ex("both", &body), &omit("both"));
    assert_eq!(reasons(&out), all(&["both"], "not-op-eq-pure"));
}

#[test]
fn an_omitted_example_is_not_admit_checked_wrap_checked_or_encoded() {
    // in a v0 document: outside the callee admit AND a U8 wrap, but the shape excuses both
    let body = "lo(200u8, b\"aa\") != wr(255u8 + 2u8, b\"aa\")";
    let out = try_gen(&ex("e", body), &omit("e"), "v0").unwrap();
    assert_eq!(reasons(&out), all(&["e"], "not-op-eq-pure"));
    assert!(parse(&out.corpus_json).unwrap().get("examples").a().is_empty());
    assert_eq!(out.cases_tfv, "tfv 1\n");
}

// ---- the normative order of the value-over-limit decision (8.4, R4) ---------------------------

/// 10^160: past the 128-bit exact window, which is not a size error.
const OVER: &str = "ov(b\"aa\")"; // evaluates 10^160: past the 128-bit exact window, not a size error

#[test]
fn step_2_an_error_that_is_not_a_size_error_refuses() {
    let e = try_gen(&ex("e", &format!("nn({OVER}, b\"aa\") == b\"aa\"")), &omit("e"), "v1").unwrap_err();
    assert!(!e.size, "{e}");
    assert_eq!(e.code, code::W0701, "{e}");
    assert!(e.message.contains("window"), "{e}");
    assert_eq!(e.at, "e");
}

#[test]
fn step_3_a_size_error_after_a_non_size_error_refuses() {
    // argument 1 fails with the window overflow, argument 2 would be a size error
    let e = try_gen(&ex("e", &format!("nn({OVER}, {}) == b\"aa\"", grow17())), &omit("e"), "v1").unwrap_err();
    assert!(!e.size && e.message.contains("window"), "{e}");
    // the pure side is evaluated after the arguments, so an argument's error comes first
    let e = try_gen(&ex("e", &format!("nn({OVER}, b\"aa\") == {}", grow17())), &omit("e"), "v1").unwrap_err();
    assert!(!e.size && e.message.contains("window"), "{e}");
}

#[test]
fn step_3_a_size_error_that_is_the_first_error_qualifies() {
    let g = grow17();
    let out = gen(&ex("e", &format!("echo({g}) == b\"aa\"")), &omit("e"));
    assert_eq!(reasons(&out), all(&["e"], "value-over-limit"));
    // on the pure side, after arguments that evaluated fine
    let out = gen(&ex("p", &format!("echo(b\"aa\") == {g}")), &omit("p"));
    assert_eq!(reasons(&out), all(&["p"], "value-over-limit"));
    // in argument 2, after argument 1 evaluated fine
    let out = gen(&ex("q", &format!("nn(b\"aa\", {g}) == b\"aa\"")), &omit("q"));
    assert_eq!(reasons(&out), all(&["q"], "value-over-limit"));
    // first means first: evaluation stops there, so a later non-size error is never met
    let out = gen(&ex("r", &format!("nb({g}, {OVER}) == b\"aa\"")), &omit("r"));
    assert_eq!(reasons(&out), all(&["r"], "value-over-limit"));
}

#[test]
fn a_size_error_in_a_literal_is_a_size_error_too() {
    let huge = "61".repeat(70_000);
    let out = gen(&ex("lit", &format!("echo(b\"{huge}\") == b\"aa\"")), &omit("lit"));
    assert_eq!(reasons(&out), all(&["lit"], "value-over-limit"));
}

#[test]
fn step_4_a_v0_wrap_plus_an_over_limit_value_still_refuses() {
    let body = format!("wr(255u8 + 2u8, {}) == b\"aa\"", grow13());
    let e = refused(&ex("w", &body), &["w"], "v0");
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "w");
    assert!(e.message.contains("wraps"), "{e}");
    // the same example in a v1 document does not wrap, so the line excuses it
    let out = gen(&ex("w", &body), &omit("w"));
    assert_eq!(reasons(&out), all(&["w"], "value-over-limit"));
}

#[test]
fn step_4_an_out_of_admit_call_plus_an_over_limit_value_still_refuses() {
    let body = format!("lo(20u8, {}) == b\"aa\"", grow13());
    let e = refused(&ex("a", &body), &["a"], "v1");
    assert_eq!(e.code, code::W0701, "{e}");
    assert!(e.message.contains("outside the op's admit"), "{e}");
    // inside the admit, the over-limit value qualifies
    let out = gen(&ex("a", &format!("lo(5u8, {}) == b\"aa\"", grow13())), &omit("a"));
    assert_eq!(reasons(&out), all(&["a"], "value-over-limit"));
}

#[test]
fn a_wrap_while_evaluating_the_admit_still_refuses() {
    // a callee whose own admit wraps in a v0 document (D18 applied to the admit)
    let wa = ("wa", "op wa(a: U8 | a + 250u8 > 3u8, b: Bytes) -> Bytes", "ab");
    let src = doc_with_ops(&ex("e", &format!("wa(10u8, {}) == b\"aa\"", grow13())), "v0", &[wa]);
    let e = gen_at(&src, "o.warrant", &omit("e")).unwrap_err();
    assert_eq!(e.code, code::W0701, "{e}");
    assert!(e.message.to_lowercase().contains("admit"), "{e}");
}

#[test]
fn any_other_check_failure_still_refuses() {
    // an example calling an op that does not exist is a type error: the line never rescues it
    let e = refused(&ex("s", &format!("nosuch({}) == b\"aa\"", grow17())), &["s"], "v1");
    // (the S2 checker refuses it first; the oracle's lowering says `type`)
    assert_eq!(e.code, code::CHECK, "{e}");
}

#[test]
fn an_omitted_example_is_still_lowered_so_a_level_refusal_refuses_the_document() {
    // a list literal is the reserved `ir.list`, which this generator does not lower: the line does not excuse it
    let e = refused(&ex("f", "len([1u8, 2u8]) == 2 and echo(b\"aa\") != echo(b\"bb\")"), &["f"], "v1");
    assert_eq!(e.code, code::LEVEL, "{e}");
    assert_eq!(e.level, Some("ir.list"), "{e}");
}

#[test]
fn an_ir_fixed_built_in_in_an_omitted_example_is_evaluated_since_s10c_2() {
    // `be_bytes` is `ir.fixed`, evaluated now: `echo(b"aa") == be_bytes(24929u16)` is an expressible
    // `op(args) == pure` example, so it is emitted, and a line that names it is a stale line (a
    // profile error), exactly as for any other expressible example
    let examples = ex("f", "echo(b\"aa\") == be_bytes(24929u16)");
    let ok = try_gen(&examples, "", "v1").expect("an ir.fixed example is emitted");
    assert!(!ok.cases_tfv.is_empty(), "the example is in cases.tfv");
    let e = refused(&examples, &["f"], "v1");
    assert_eq!(e.code, code::PROFILE, "{e}");
}

// ---- the evaluation order (arguments, pure side, D19, D18), for every example -------------------

#[test]
fn the_pure_side_is_evaluated_before_the_admit_check() {
    // argument outside the admit AND a pure side that fails: the pure side's error is the one
    let body = format!("lo(20u8, b\"aa\") == {}", grow17());
    let e = try_gen(&ex("e", &body), "", "v1").unwrap_err();
    assert!(e.size, "the pure side comes before D19: {e}");
    // not omitted, and not rescued either: the error is the generator's refusal
    assert_eq!(e.code, code::W0701, "{e}");
}

#[test]
fn the_admit_check_comes_before_the_v0_wrap_check() {
    // the pure side wraps (255u8 + 2u8) in a v0 document, and the argument is outside the admit
    let body = "lo(20u8, b\"aa\") == (if 255u8 + 2u8 > 0u8 then b\"aa\" else b\"\")";
    let e = try_gen(&ex("e", body), "", "v0").unwrap_err();
    assert_eq!(e.code, code::W0701, "{e}");
    assert!(e.message.contains("admit"), "D19 first: {e}");
    // inside the admit, the wrap is what refuses
    let body = "lo(5u8, b\"aa\") == (if 255u8 + 2u8 > 0u8 then b\"aa\" else b\"\")";
    let e = try_gen(&ex("e", body), "", "v0").unwrap_err();
    assert!(e.message.contains("wraps"), "D18: {e}");
}

#[test]
fn an_argument_error_comes_before_the_pure_sides() {
    let e = try_gen(&ex("e", &format!("nn({OVER}, b\"aa\") == {}", grow17())), "", "v1").unwrap_err();
    assert!(!e.size && e.message.contains("window"), "{e}");
    let e = try_gen(&ex("e", &format!("nn({OVER}, b\"aa\") == b\"aa\"")), "", "v1").unwrap_err();
    assert!(e.message.contains("window"), "{e}");
}

// ---- the key -----------------------------------------------------------------------------------

#[test]
fn the_key_is_sorted_by_id_and_written_only_when_non_empty() {
    let examples = ex("zz", "echo(b\"aa\") != echo(b\"bb\")") + &ex("aa", "echo(b\"cc\") != echo(b\"dd\")");
    let out = gen(&examples, &omits(&["zz", "aa"]));
    assert_eq!(
        reasons(&out),
        [
            ("aa".to_string(), "not-op-eq-pure".to_string()),
            ("zz".to_string(), "not-op-eq-pure".to_string())
        ]
    );
    let c = parse(&out.corpus_json).unwrap();
    for r in c.get("omitted_examples").a() {
        assert_eq!(r.keys(), ["id", "reason"]);
    }
    let plain = gen(&ex("keep", "echo(b\"aa\") == b\"aa\""), "");
    assert!(!parse(&plain.corpus_json).unwrap().has("omitted_examples"));
    assert!(!plain.corpus_json.contains("omitted_examples"));
    assert_eq!(parse(&plain.corpus_json).unwrap().get("examples").a().len(), 1);
}

#[test]
fn the_key_is_sorted_by_bytes_not_by_declaration() {
    // declaration order is `b`, `B`, `a`; byte order is `B`, `a`, `b`
    let examples = ex("b", "echo(b\"aa\") != echo(b\"bb\")")
        + &ex("B", "echo(b\"aa\") != echo(b\"cc\")")
        + &ex("a", "echo(b\"aa\") != echo(b\"dd\")");
    let out = gen(&examples, &omits(&["b", "B", "a"]));
    let names: Vec<String> = reasons(&out).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["B", "a", "b"]);
}

#[test]
fn without_the_lines_nothing_changes_for_an_existing_fixture() {
    for corpus in [
        include_str!("../../../python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/corpus.json"),
        include_str!("../../../../specs/fixtures/ct-compare-warrant-v2/corpus.json"),
    ] {
        assert!(!corpus.contains("omitted_examples"));
    }
}

#[test]
fn lowering_defers_the_shape_refusal_only_for_a_named_example() {
    let src = doc_with_ops(&ex("t", "echo(b\"aa\") != echo(b\"bb\")"), "v1", &[]);
    let id = "warrant/omitx/example/t".to_string();
    with_file(&src, |f| {
        let l = lower_document_omitting(f, std::slice::from_ref(&id)).unwrap();
        assert!(l.examples[0].reference.is_none());
        let e = lower_document_omitting(f, &[]).unwrap_err();
        assert_eq!((e.code, e.at.as_str()), (code::W0701, "t"));
        // an id that names another example excuses nothing here
        let e = lower_document_omitting(f, &["warrant/omitx/example/other".to_string()]).unwrap_err();
        assert_eq!(e.code, code::W0701);
    });
}

// ---- the binary and its exit status (CV2-F2: refusal 2, drift 1) -------------------------------

/// `--check` of the oracle's committed fixture passes through the real binary; a stale line and an
/// unknown id are refusals (exit 2, nothing written); a drifted `cases.tfv` is drift (exit 1).
#[test]
fn the_binary_checks_the_committed_fixture_and_refuses_a_stale_or_unknown_line() {
    use std::path::Path;
    use std::process::Command;
    let bin = env!("CARGO_BIN_EXE_warrant-corpus-gen");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let fixture = "code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2";
    let tmp = std::env::temp_dir().join(format!("wcg-omit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let run = |profile: &str, out: &str, check: bool| -> (i32, String) {
        let mut c = Command::new(bin);
        c.current_dir(&root).args([
            "--warrant",
            &format!("{fixture}/omit.warrant"),
            "--profile",
            profile,
            "--out",
            out,
        ]);
        if check {
            c.arg("--check");
        }
        let r = c.output().unwrap();
        (r.status.code().unwrap(), String::from_utf8_lossy(&r.stderr).into_owned())
    };
    let own = format!("{fixture}/corpus.profile");
    let committed = format!("{fixture}/corpus");
    let (c, e) = run(&own, &committed, true);
    assert_eq!(c, 0, "the committed fixture checks clean: {e}");
    // writing it elsewhere writes the same bytes
    let out = tmp.join("out");
    let (c, e) = run(&own, out.to_str().unwrap(), false);
    assert_eq!(c, 0, "{e}");
    for f in ["corpus.json", "cases.tfv", "chunks/echo_is_identity.txt"] {
        assert_eq!(
            std::fs::read(out.join(f)).unwrap(),
            std::fs::read(root.join(&committed).join(f)).unwrap(),
            "{f}"
        );
    }
    // a stale line (the kept example is emittable) and an unknown id: refusals, nothing written
    for (name, extra) in [
        ("stale", "omit_example warrant/omitx/example/echo_small\n"),
        ("unknown", "omit_example warrant/omitx/example/nope\n"),
    ] {
        let p = tmp.join(format!("{name}.profile"));
        std::fs::write(&p, format!("{FIXTURE_PROFILE}{extra}")).unwrap();
        let o = tmp.join(format!("o-{name}"));
        for check in [false, true] {
            let (c, e) = run(p.to_str().unwrap(), o.to_str().unwrap(), check);
            assert_eq!(c, 2, "{name} check={check}: {e}");
            assert!(e.contains("profile"), "{e}");
            assert!(!e.contains("nope"), "an unknown id is never echoed: {e}");
        }
        assert!(!o.exists(), "nothing is written for a refusal");
    }
    // the profile edited to drop a line: the example is over the cap and unomitted, so since CV2-F1
    // it is a refusal (exit 2), not a drift
    let p = tmp.join("less.profile");
    std::fs::write(
        &p,
        FIXTURE_PROFILE.replace("omit_example warrant/omitx/example/echo_big_value\n", ""),
    )
    .unwrap();
    let (c, _) = run(p.to_str().unwrap(), &committed, true);
    assert_eq!(c, 2, "the over-limit example is refused at emission");
    let _ = std::fs::remove_dir_all(tmp);
}
