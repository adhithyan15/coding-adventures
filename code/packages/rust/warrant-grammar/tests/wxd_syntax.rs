//! VER04k section 2.5 and 2.15 ("Syntax"): the WX-D forms add **no** syntax.
//!
//! Every one of the twelve WX-D built-ins is spelled as an ordinary call
//! (`reference = qualified_name [ "<" widths ">" ] [ "(" args ")" ]`), and a
//! *section* such as `gmul_step(b)` in the function position of a D2 form is an
//! ordinary call too.  So this file proves three things over the real parser:
//!
//! 1. each form parses, in a v0 and in a v1 document, as an
//!    `Expression::Reference` whose name is the built-in and whose arguments
//!    are the written ones (the parser does not know the names are special:
//!    giving them meaning is the checker's job, see `tests/wxd_checker.rs`);
//! 2. the canonical grammar engine accepts the same text, from the loaded and
//!    from the embedded artifact alike;
//! 3. the canonical token and grammar files are byte-for-byte the files
//!    wave 3 shipped (their SHA-256 is pinned below).  A WX-D slice that needed
//!    a grammar change would fail here first.

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

#[allow(dead_code)]
mod support;

/// The twelve WX-D names: the seven of slice D1 and the five of slice D2.
const D1: [&str; 7] = [
    "take",
    "drop",
    "slice",
    "replicate",
    "range",
    "to_list",
    "to_bytes",
];
const D2: [&str; 5] = ["fold", "map", "count", "all", "any"];

/// One written call per form, with the arity VER04k section 2.2 gives it.
/// The D2 rows use a bare model name and a one-argument section (no tuple:
/// a tuple literal is a v1 construct, and every row must parse in v0 too).
fn calls() -> Vec<(&'static str, &'static str, usize)> {
    vec![
        ("take", "take(xs, 2)", 2),
        ("drop", "drop(xs, len(xs) - 1)", 2),
        ("slice", "slice(xs, 1, 2)", 3),
        ("replicate", "replicate(0u8, 3)", 2),
        ("range", "range(8)", 1),
        ("to_list", "to_list(b)", 1),
        ("to_bytes", "to_bytes([1u8, 2u8])", 1),
        ("fold", "fold(range(8), init, gmul_step(b))", 3),
        ("map", "map(xs, f)", 2),
        ("count", "count(b, is_zero)", 2),
        ("all", "all(range(len(xs)), ok_at(xs))", 2),
        ("any", "any(xs, p)", 2),
    ]
}

/// Parse `src` and hand the left operand of its single example's `==` to `f`.
fn with_lhs<R>(src: &str, f: impl FnOnce(&Expression) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"));
    let Declaration::Example(ex) = &file.declarations[0] else {
        panic!("not an example")
    };
    let Expression::Binary {
        op: BinaryOp::Eq,
        left,
        ..
    } = &ex.body
    else {
        panic!("not an equality")
    };
    f(left)
}

fn example(header: &str, expr: &str) -> String {
    format!("warrant t{header}\n  example e:\n    {expr} == {expr}\n")
}

#[test]
fn the_twelve_names_are_the_ones_the_specification_lists() {
    let mut all: Vec<&str> = D1.iter().chain(D2.iter()).copied().collect();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), 12);
    assert_eq!(calls().len(), 12);
    for (name, _, _) in calls() {
        assert!(all.contains(&name), "{name}");
    }
}

#[test]
fn every_form_parses_as_an_ordinary_call_in_v0_and_in_v1() {
    for header in ["", " v1"] {
        for (name, text, arity) in calls() {
            with_lhs(&example(header, text), |lhs| match lhs {
                Expression::Reference {
                    name: q,
                    widths,
                    args: Some(args),
                    ..
                } => {
                    assert_eq!(q.segments, vec![name], "{header:?} {text}");
                    assert!(widths.is_empty(), "{text}");
                    assert_eq!(args.len(), arity, "{header:?} {text}");
                }
                other => panic!("{header:?} {text}: not a call: {other:?}"),
            });
        }
    }
}

#[test]
fn a_section_is_a_call_in_argument_position_and_a_bare_model_name_is_a_reference() {
    // `fold(xs, init, gmul_step(b))`: the third argument is the call
    // `gmul_step(b)`; `count(b, is_zero)`: the second is a bare NAME.
    with_lhs(
        &example(" v1", "fold(range(8), (0u8, a), gmul_step(b))"),
        |lhs| {
            let Expression::Reference {
                args: Some(args), ..
            } = lhs
            else {
                panic!("{lhs:?}")
            };
            assert!(matches!(&args[1], Expression::Tuple { elements, .. } if elements.len() == 2));
            match &args[2] {
                Expression::Reference {
                    name,
                    args: Some(bound),
                    ..
                } => {
                    assert_eq!(name.segments, vec!["gmul_step"]);
                    assert_eq!(bound.len(), 1);
                }
                other => panic!("{other:?}"),
            }
        },
    );
    with_lhs(&example(" v1", "count(b, is_zero)"), |lhs| {
        let Expression::Reference {
            args: Some(args), ..
        } = lhs
        else {
            panic!("{lhs:?}")
        };
        assert!(matches!(
            &args[1],
            Expression::Reference { name, args: None, .. } if name.segments == vec!["is_zero"]
        ));
    });
}

#[test]
fn negative_counts_and_huge_counts_are_ordinary_expressions() {
    // `take(xs, -1)` is unary minus over an Int literal, and a 2^70 count is
    // spelled as a product (an integer literal is at most a u64).
    for text in [
        "take(xs, -1)",
        "range(-3)",
        "replicate(0u8, 4294967296 * 4294967296 * 64)",
    ] {
        with_lhs(&example(" v1", text), |lhs| {
            assert!(
                matches!(lhs, Expression::Reference { args: Some(_), .. }),
                "{text}"
            );
        });
    }
}

#[test]
fn the_canonical_grammar_accepts_every_form_from_both_artifacts() {
    use support::{parse, Fault};
    for header in ["", " v1"] {
        for (_, text, _) in calls() {
            let src = example(header, text);
            let loaded = parse(&src, true, Fault::None);
            let embedded = parse(&src, false, Fault::None);
            assert!(loaded.is_some(), "loaded grammar rejected {src:?}");
            assert_eq!(
                loaded, embedded,
                "the embedded artifact must match the canonical file: {src:?}"
            );
        }
    }
}

#[test]
fn the_canonical_token_and_grammar_files_are_unchanged_by_wx_d() {
    // Pinned on origin/main before WX-D1 (wave 3 plus WX-H/E).  VER04k section
    // 1.2: "no new reserved words, no new tokens, no new multi-character
    // operators".  A change here is a grammar change and needs the
    // specification first.
    let tokens = include_str!("../../../../grammars/warrant/warrant.tokens");
    let grammar = include_str!("../../../../grammars/warrant/warrant.grammar");
    assert_eq!(
        sha256_hex(tokens.as_bytes()),
        "8cb7036d0eb818cffc422605aa1da73a9a34a94d84dadc210d5afd38c83d61c1"
    );
    assert_eq!(
        sha256_hex(grammar.as_bytes()),
        "6217be01a8fa769054e17408024ebeaaea46d127986ab5c18c2f0c338f0168d7"
    );
}
