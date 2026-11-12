//! VER04j section 6 (WX-H) and section 14.2: the language half.
//!
//! * `output` is a contextual bound metric, W0101 in a v0 document and an
//!   unknown metric (W0401) when it is some other word;
//! * the type rule of an `output` row (byte strings, lists, a `Result` of one);
//! * `imin` / `imax`: typing, the ground evaluator, v1 scoping;
//! * the parameterised bounds the specification writes (`to_int(max_output)`,
//!   `unwrap_or(div(n, k), 0)`) type-check, and a secret-labelled parameter in
//!   a bound is still accepted (14.2 item 30).
//!
//! The evidence half (the observer, the generated output-bound suites and the
//! faults) is `tests/wxh_evidence.rs`.

use std::collections::HashMap;

use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckedWarrantModule, CheckerError, DiagnosticCode, Type, V1_FUNCTIONS,
};
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserError, ParserErrorKind, ParserLimits,
    WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn parse<R>(src: &str, f: impl FnOnce(Result<&WarrantFile, ParserError>) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let parsed = parse_document(&layout, &doc, ParserLimits::default());
    f(parsed.as_ref().map_err(Clone::clone))
}

fn check(src: &str) -> Result<CheckedWarrantModule, CheckerError> {
    parse(src, |p| check_warrant(p.expect("parse")))
}

/// A document with `op f(<params>) -> <ret>`, public labels and one `bound` row.
fn bounded(header: &str, params: &str, ret: &str, row: &str) -> String {
    let labels: String = params
        .split(", ")
        .filter(|p| !p.is_empty())
        .map(|p| format!("    {} public\n", p.split(':').next().unwrap()))
        .collect();
    format!(
        "warrant t{header}\n  error E = BAD\n  op f({params}) -> {ret}\n  labels f:\n{labels}  bound f:\n    {row}\n"
    )
}

fn code_of(src: &str) -> DiagnosticCode {
    check(src).expect_err("must be rejected").code
}

// ---------------------------------------------------------------------------
// Syntax and gating
// ---------------------------------------------------------------------------

#[test]
fn output_is_accepted_in_v1_and_is_w0101_in_v0() {
    let v1 = bounded(" v1", "d: Bytes", "Bytes", "output <= len(d)");
    parse(&v1, |p| {
        let file = p.expect("v1 parses");
        let Declaration::Bound(b) = file.declarations.last().unwrap() else {
            panic!("bound expected")
        };
        assert_eq!(b.rows[0].name, "output");
    });
    let v0 = bounded("", "d: Bytes", "Bytes", "output <= len(d)");
    parse(&v0, |p| {
        let e = p.expect_err("a v0 document may not bound `output`");
        assert_eq!(e.kind(), ParserErrorKind::V1ConstructInV0);
        assert_eq!(e.code(), "W0101");
    });
}

#[test]
fn an_unknown_metric_stays_w0401_in_both_versions() {
    for header in ["", " v1"] {
        let src = bounded(header, "d: Bytes", "Bytes", "outputs <= len(d)");
        assert_eq!(code_of(&src), DiagnosticCode::W0401, "header {header:?}");
    }
}

#[test]
fn a_v0_model_may_still_be_called_output() {
    // `output` is contextual: only the metric position is special.
    let src = "warrant t\n  model output(a: Nat) -> Int = a + 1\n  example e:\n    output(1) == 2\n";
    check(src).expect("a v0 model called output");
    let src = "warrant t v1\n  model output(a: Nat) -> Int = a + 1\n  example e:\n    output(1) == 2\n";
    check(src).expect("a v1 model called output");
}

// ---------------------------------------------------------------------------
// The type rule of an `output` row
// ---------------------------------------------------------------------------

#[test]
fn output_needs_a_sized_result() {
    for ret in [
        "Bytes",
        "Bytes<4>",
        "List<U8>",
        "Result<Bytes, E>",
        "Result<List<Bool>, E>",
        "Result<Bytes<2>, E>",
    ] {
        check(&bounded(" v1", "d: Bytes", ret, "output <= len(d)"))
            .unwrap_or_else(|e| panic!("{ret}: {e:?}"));
    }
    for ret in [
        "Nat",
        "Bool",
        "U8",
        "Option<Bytes>",
        "(Nat, Bytes)",
        "Result<Nat, E>",
        "Result<(Bytes, Nat), E>",
        "Result<Option<Bytes>, E>",
    ] {
        assert_eq!(
            code_of(&bounded(" v1", "d: Bytes", ret, "output <= len(d)")),
            DiagnosticCode::W0301,
            "{ret}"
        );
    }
}

#[test]
fn output_on_a_record_or_variant_result_is_w0301() {
    let src = "warrant t v1\n  record R(a: Bytes)\n  variant S = A(x: Bytes) | B\n  op f(d: Bytes) -> R\n  op g(d: Bytes) -> S\n  labels f:\n    d public\n  labels g:\n    d public\n  bound f:\n    output <= 1\n";
    assert_eq!(code_of(src), DiagnosticCode::W0301);
    let src = src.replace("bound f:", "bound g:");
    assert_eq!(code_of(&src), DiagnosticCode::W0301);
}

#[test]
fn bound_expressions_are_typed_as_int() {
    // An Option-typed bound expression is not an Int.
    let src = bounded(" v1", "d: Bytes", "Bytes", "output <= div(len(d), 2)");
    assert_eq!(code_of(&src), DiagnosticCode::W0301);
    // ... unless it is unwrapped (the specification's own spelling).
    let src = bounded(
        " v1",
        "d: Bytes",
        "Bytes",
        "output <= unwrap_or(div(len(d), 2), 0)",
    );
    check(&src).expect("unwrap_or(div(..), 0) is an Int");
    // A Bool is not a bound either.
    let src = bounded(" v1", "d: Bytes", "Bytes", "output <= (len(d) > 1)");
    assert_eq!(code_of(&src), DiagnosticCode::W0301);
}

#[test]
fn the_specification_example_type_checks() {
    let src = "warrant lz v1\n  error LzError = OUTPUT_LIMIT | CORRUPT\n  op decompress(data: Bytes, max_output: U32) -> Result<Bytes, LzError> fresh\n  labels decompress:\n    data public\n    max_output public\n  bound decompress:\n    output <= to_int(max_output)\n    steps <= 32 * (len(data) + to_int(max_output)) + 4096\n    heap <= 2 * to_int(max_output) + 8 * len(data) + 4096\n    depth <= 16\n";
    let m = check(src).expect("section 6.2 example");
    assert_eq!(m.operations["decompress"].params[1].1, Type::U32);
}

#[test]
fn a_secret_labelled_parameter_in_a_bound_is_unchanged() {
    // 14.2 item 30: the checker never rejected one (ct-compare bounds `len` of
    // secret parameters); WX-H does not start to.
    let src = "warrant t v1\n  op f(d: Bytes) -> Bytes\n  labels f:\n    d secret\n  bound f:\n    output <= len(d)\n    heap <= len(d)\n";
    check(src).expect("secret parameter in a bound");
}

// ---------------------------------------------------------------------------
// imin / imax
// ---------------------------------------------------------------------------

fn model_body(header: &str, body: &str) -> String {
    format!("warrant t{header}\n  model m(a: Nat, b: Int, c: U32) -> Int = {body}\n")
}

#[test]
fn imin_and_imax_type_as_int() {
    for body in [
        "imin(a, b)",
        "imax(b, a)",
        "imin(a, 3)",
        "imax(imin(a, b), -1)",
        "imin(to_int(c), 7)",
    ] {
        let m = check(&model_body(" v1", body)).unwrap_or_else(|e| panic!("{body}: {e:?}"));
        assert_eq!(m.models["m"].body_type, Type::Int, "{body}");
    }
}

#[test]
fn imin_and_imax_reject_other_operands_and_arities() {
    for body in [
        "imin(c, 3)",
        "imax(a, true)",
        "imin(a)",
        "imax(a, b, a)",
        "imin<U8>(a, b)",
        "imin",
    ] {
        let e = check(&model_body(" v1", body)).expect_err(body);
        assert_eq!(e.code, DiagnosticCode::W0301, "{body}: {e:?}");
    }
}

#[test]
fn imin_and_imax_are_reserved_in_v1_and_free_in_v0() {
    assert!(V1_FUNCTIONS.contains(&"imin") && V1_FUNCTIONS.contains(&"imax"));
    // 33 through WX-H; WX-D1 reserves its twelve sequence names (VER04k
    // section 1.2), see tests/wxd_checker.rs.
    assert_eq!(V1_FUNCTIONS.len(), 45);
    for name in ["imin", "imax"] {
        let v1 = format!("warrant t v1\n  model {name}(a: Nat) -> Nat = a\n");
        assert_eq!(code_of(&v1), DiagnosticCode::W0201, "{name} in v1");
        let v0 = format!("warrant t\n  model {name}(a: Nat) -> Nat = a\n  example e:\n    {name}(4) == 4\n");
        check(&v0).unwrap_or_else(|e| panic!("{name} in v0: {e:?}"));
    }
    // In a v0 document the call is the user's own model, not the built-in.
    let v0 = "warrant t\n  model m(a: Nat) -> Nat = imin(a, a)\n";
    assert_eq!(code_of(v0), DiagnosticCode::W0202);
}

fn eval_examples(src: &str) -> Vec<EvalValue> {
    parse(src, |p| {
        let file = p.expect("parse");
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        file.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Example(e) => {
                    let Expression::Binary { left, .. } = &e.body else {
                        panic!("== expected")
                    };
                    ev.reset_work();
                    Some(ev.eval(left, &HashMap::new()).expect("evaluates"))
                }
                _ => None,
            })
            .collect()
    })
}

#[test]
fn the_ground_evaluator_computes_imin_and_imax() {
    let src = "warrant t v1\n  example a:\n    imin(3, 5) == 3\n  example b:\n    imax(3, 5) == 5\n  example c:\n    imin(-4, 2) == -4\n  example d:\n    imax(-4, 2) == 2\n  example e:\n    imin(7, 7) == 7\n  example f:\n    imax(to_int(max_value<U32>()), 0) == 4294967295\n";
    let want = [3, 5, -4, 2, 7, 4_294_967_295i128];
    for (got, want) in eval_examples(src).into_iter().zip(want) {
        assert_eq!(got.as_i128(), Some(want));
    }
}
