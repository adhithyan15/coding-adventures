//! VER04j sections 1 and 2.2/2.4: the version header, the new literal forms and
//! the rule that a v0 document means exactly what it always did.
//!
//! These are the cases the specification asks of its `warrant-syntax-v2`
//! corpus.  The version gating lives in the hand-written parser (the canonical
//! grammar cannot see a header word), so the corpus runner of `self_corpus.rs`,
//! which drives the grammar engine, cannot express "W0101"; these tests drive
//! the real parser instead.  The canonical artifacts are checked separately at
//! the end of the file.

use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::layout::{
    capture_layout, LayoutLimits, LayoutTokenKind as K,
};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, Literal, ParserError, ParserErrorKind,
    ParserLimits, UnaryOp, Version,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

#[allow(dead_code)]
mod support;

fn run<R>(
    src: &str,
    f: impl FnOnce(Result<&coding_adventures_warrant_grammar::parser::WarrantFile, ParserError>) -> R,
) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let parsed = parse_document(&layout, &doc, ParserLimits::default());
    f(parsed.as_ref().map_err(Clone::clone))
}

/// `warrant t [v1]` with one example whose body is `<expr> == <expr>`.
fn doc(header: &str, expr: &str) -> String {
    format!("warrant t{header}\n  example e:\n    {expr} == {expr}\n")
}

/// The literal on the left of the example's `==`, or the parse error.
fn literal(header: &str, expr: &str) -> Result<Literal<'static>, ParserError> {
    run(&doc(header, expr), |parsed| {
        let file = parsed?;
        let Declaration::Example(ex) = &file.declarations[0] else {
            panic!("example expected")
        };
        let Expression::Binary {
            op: BinaryOp::Eq,
            left,
            ..
        } = &ex.body
        else {
            panic!("equality expected")
        };
        match &**left {
            Expression::Literal { lit, .. } => Ok(match lit {
                Literal::Bool(b) => Literal::Bool(*b),
                Literal::U64(n) => Literal::U64(*n),
                Literal::U8(n) => Literal::U8(*n),
                Literal::Int(n) => Literal::Int(*n),
                Literal::Fixed(t, v) => Literal::Fixed(*t, *v),
                Literal::Bytes(_) => panic!("bytes literal"),
            }),
            Expression::Unary {
                op: UnaryOp::Neg, ..
            } => Ok(Literal::Bool(false)),
            other => panic!("not a literal: {other:?}"),
        }
    })
}

fn code_of(header: &str, expr: &str) -> (&'static str, ParserErrorKind) {
    let e = literal(header, expr).expect_err(expr);
    (e.code(), e.kind())
}

#[test]
fn header_accepts_v0_and_v1_and_nothing_else() {
    run("warrant x\n  error E = A\n", |p| {
        assert_eq!(p.unwrap().version, Version::V0);
    });
    run("warrant x v1\n  error E = A\n", |p| {
        assert_eq!(p.unwrap().version, Version::V1);
    });
    for bad in [
        "warrant x v2\n  error E = A\n",
        "warrant x V1\n  error E = A\n",
        "warrant x v10\n  error E = A\n",
        "warrant x v0\n  error E = A\n",
    ] {
        run(bad, |p| {
            let e = p.unwrap_err();
            assert_eq!(e.code(), "W0101", "{bad}");
            assert_eq!(e.kind(), ParserErrorKind::InvalidVersionHeader);
            // The span names the offending word, not the file.
            let (a, b) = e.span().unwrap();
            assert_eq!(&bad[a..b], bad.split_whitespace().nth(2).unwrap());
        });
    }
    // A third word after `v1` is an ordinary syntax error.
    run("warrant x v1 extra\n  error E = A\n", |p| {
        let e = p.unwrap_err();
        assert_eq!(
            (e.code(), e.kind()),
            ("W0001", ParserErrorKind::UnexpectedToken)
        );
    });
    // A keyword is not a version.
    run("warrant x model\n  error E = A\n", |p| {
        assert_eq!(p.unwrap_err().code(), "W0101");
    });
}

#[test]
fn every_suffix_and_hex_form_is_accepted_in_a_v1_document() {
    let v1 = " v1";
    let cases: Vec<(&str, Literal)> = vec![
        ("0u8", Literal::U8(0)),
        ("255u8", Literal::U8(255)),
        ("18446744073709551615u64", Literal::U64(u64::MAX)),
        ("0u16", Literal::Fixed(FixedTy::U16, 0)),
        ("65535u16", Literal::Fixed(FixedTy::U16, 65535)),
        (
            "4294967295u32",
            Literal::Fixed(FixedTy::U32, u32::MAX as i128),
        ),
        ("0i8", Literal::Fixed(FixedTy::I8, 0)),
        ("127i8", Literal::Fixed(FixedTy::I8, 127)),
        ("32767i16", Literal::Fixed(FixedTy::I16, 32767)),
        (
            "2147483647i32",
            Literal::Fixed(FixedTy::I32, i32::MAX as i128),
        ),
        (
            "9223372036854775807i64",
            Literal::Fixed(FixedTy::I64, i64::MAX as i128),
        ),
        // Hex: no suffix is an Int literal, a suffix is range checked.
        ("0x1d", Literal::Int(0x1d)),
        ("0x0", Literal::Int(0)),
        ("0xffffffffffffffff", Literal::Int(u64::MAX)),
        ("0x1du8", Literal::U8(0x1d)),
        ("0x0001u16", Literal::Fixed(FixedTy::U16, 1)),
        ("0xffu8", Literal::U8(255)),
        (
            "0xffffffffu32",
            Literal::Fixed(FixedTy::U32, u32::MAX as i128),
        ),
        ("0xffffffffffffffffu64", Literal::U64(u64::MAX)),
        ("0x7fi8", Literal::Fixed(FixedTy::I8, 127)),
        ("0x7fffi16", Literal::Fixed(FixedTy::I16, 32767)),
        (
            "0x7fffffffi32",
            Literal::Fixed(FixedTy::I32, i32::MAX as i128),
        ),
        (
            "0x7fffffffffffffffi64",
            Literal::Fixed(FixedTy::I64, i64::MAX as i128),
        ),
    ];
    for (text, want) in cases {
        assert_eq!(literal(v1, text).unwrap(), want, "{text}");
    }
    // The v0 spellings still mean what they meant, in either document.
    assert_eq!(literal("", "5u8").unwrap(), Literal::U8(5));
    assert_eq!(literal("", "5u64").unwrap(), Literal::U64(5));
    assert_eq!(literal("", "5").unwrap(), Literal::Int(5));
    assert_eq!(literal(v1, "5").unwrap(), Literal::Int(5));
}

#[test]
fn the_signed_minimum_is_a_literal_only_as_the_operand_of_unary_minus() {
    // Folded into one literal; the span starts at the minus sign.
    for (text, ty, min) in [
        ("-128i8", FixedTy::I8, -128i128),
        ("-32768i16", FixedTy::I16, -32768),
        ("-2147483648i32", FixedTy::I32, i32::MIN as i128),
        ("-9223372036854775808i64", FixedTy::I64, i64::MIN as i128),
        ("-0x80i8", FixedTy::I8, -128),
    ] {
        let src = doc(" v1", text);
        run(&src, |p| {
            let file = p.unwrap();
            let Declaration::Example(ex) = &file.declarations[0] else {
                panic!()
            };
            let Expression::Binary { left, .. } = &ex.body else {
                panic!()
            };
            let Expression::Literal { lit, span } = &**left else {
                panic!("{text} was not folded to a literal: {left:?}")
            };
            assert_eq!(*lit, Literal::Fixed(ty, min), "{text}");
            assert_eq!(&src[span.0..span.1], text);
        });
    }
    // An ordinary negation stays a unary minus over a literal.
    run(&doc(" v1", "-5i8"), |p| {
        let file = p.unwrap();
        let Declaration::Example(ex) = &file.declarations[0] else {
            panic!()
        };
        let Expression::Binary { left, .. } = &ex.body else {
            panic!()
        };
        assert!(matches!(
            &**left,
            Expression::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));
    });
    // The magnitude alone, or behind parentheses, is out of range.
    for bad in [
        "128i8",
        "-(128i8)",
        "32768i16",
        "2147483648i32",
        "9223372036854775808i64",
        "0x80i8",
        "-129i8",
        "-32769i16",
    ] {
        assert_eq!(
            code_of(" v1", bad),
            ("W0101", ParserErrorKind::LiteralOutOfRange),
            "{bad}"
        );
    }
}

#[test]
fn malformed_and_out_of_range_literals_are_rejected_in_a_v1_document() {
    // Out of range of the suffix type: W0101.
    for bad in [
        "256u8",
        "65536u16",
        "4294967296u32",
        "18446744073709551616u64",
        "256i8",
        "4294967296i32",
        "0x100u8",
        "0x10000u16",
        "0x1ffffffffffffffff",
        "0x1ffffffffffffffffu64",
        "340282366920938463463374607431768211456u16",
    ] {
        assert_eq!(
            code_of(" v1", bad),
            ("W0101", ParserErrorKind::LiteralOutOfRange),
            "{bad}"
        );
    }
    // Hex digit rules: lowercase, at least one digit, only the listed suffixes.
    for bad in [
        "0xU8", "0xFF", "0x1D", "0xffU8", "0x", "0xg", "0x1u128", "0x1i128",
    ] {
        let (code, kind) = code_of(" v1", bad);
        assert_eq!(kind, ParserErrorKind::UnexpectedToken, "{bad}");
        assert_eq!(code, "W0001", "{bad}");
    }
    // Plain syntax errors: leading zeros, digit separators, unknown suffixes.
    for bad in [
        "01u16", "00u8", "01u32", "1_000", "1u128", "1i128", "1u", "1usize",
    ] {
        assert_eq!(
            code_of(" v1", bad).1,
            ParserErrorKind::UnexpectedToken,
            "{bad}"
        );
    }
}

#[test]
fn every_new_construct_is_rejected_in_a_v0_document() {
    for text in [
        "5u16", "5u32", "5i8", "5i16", "5i32", "5i64", "0x1d", "0x1du8", "0xffu64", "-5i8",
        "-128i8",
    ] {
        let (code, kind) = code_of("", text);
        assert_eq!(
            (code, kind),
            ("W0101", ParserErrorKind::V1ConstructInV0),
            "{text}"
        );
    }
    // ... and the identical text with the `v1` header is accepted.
    for text in [
        "5u16", "5u32", "5i8", "5i16", "5i32", "5i64", "0x1d", "0x1du8", "0xffu64", "-5i8",
        "-128i8",
    ] {
        assert!(literal(" v1", text).is_ok(), "{text}");
    }
    // The v0 rejection of an out-of-range u8/u64 keeps its v0 code.
    assert_eq!(
        code_of("", "256u8"),
        ("W0001", ParserErrorKind::UnexpectedToken)
    );
    assert_eq!(
        code_of("", "18446744073709551616u64"),
        ("W0001", ParserErrorKind::UnexpectedToken)
    );
    assert_eq!(code_of("", "01u8").0, "W0001");
}

#[test]
fn no_new_reserved_words_and_no_new_operator_tokens() {
    // A v0 document may keep models called `record`, `rem`, `shl`, ...
    let v0 = "warrant t\n  model record(a: U8) -> U8 = a\n  model rem(a: U8) -> U8 = a\n  model shl(a: U8) -> U8 = a\n  model widen(a: U8) -> U8 = a\n";
    run(v0, |p| assert_eq!(p.unwrap().declarations.len(), 4));
    // `>>` is two tokens, so nested generics still parse in both versions.
    for header in ["", " v1"] {
        let src = format!("warrant t{header}\n  model m(a: List<List<U8>>) -> Bool = true\n");
        run(&src, |p| assert!(p.is_ok()));
    }
    // `_` is an ordinary binder name to the v0 parser.
    run(
        "warrant t\n  model m(o: Option<U8>) -> U8 = match o with None -> 0u8 | Some(_) -> 1u8\n",
        |p| {
            assert!(p.is_ok());
        },
    );
    // The layout lexer: no `>>`, `&`, `^`, `|` operator tokens were added.
    let src = SourceDocument::admit(
        b"warrant t v1\n  model m() -> Bool = a >> b\n",
        SourceLimits::default(),
    )
    .unwrap();
    let tokens = capture_layout(&src, LayoutLimits::default()).unwrap();
    let symbols: Vec<&str> = tokens
        .iter()
        .filter_map(|t| match t.kind {
            K::Symbol(s) => Some(s),
            _ => None,
        })
        .collect();
    assert!(symbols.windows(2).any(|w| w == [">", ">"]), "{symbols:?}");
    assert!(!symbols.contains(&">>"));
}

#[test]
fn the_layout_lexer_splits_numbers_exactly_as_the_token_regexes_do() {
    let lex = |text: &str| -> Vec<String> {
        let text = format!("x {text}\n");
        let src = SourceDocument::admit(text.as_bytes(), SourceLimits::default()).unwrap();
        capture_layout(&src, LayoutLimits::default())
            .unwrap()
            .iter()
            .filter_map(|t| match t.kind {
                K::Number(n) => Some(format!("N:{n}")),
                K::Word(w) => Some(format!("W:{w}")),
                _ => None,
            })
            .skip(1)
            .collect()
    };
    assert_eq!(lex("5u16"), ["N:5u16"]);
    assert_eq!(lex("5u1"), ["N:5", "W:u1"]);
    assert_eq!(lex("5u128"), ["N:5", "W:u128"]);
    assert_eq!(lex("0x1du8"), ["N:0x1du8"]);
    assert_eq!(lex("0xFF"), ["N:0xFF"]);
    assert_eq!(lex("1_000"), ["N:1", "W:_000"]);
    assert_eq!(lex("12i64 3i8"), ["N:12i64", "N:3i8"]);
    assert_eq!(lex("0x"), ["N:0x"]);
}

#[test]
fn a_v1_call_with_a_static_type_argument_parses_as_a_static_application() {
    run(
        "warrant t v1\n  example e:\n    widen<U32>(a) == max_value<I8>()\n",
        |p| {
            let file = p.unwrap();
            let Declaration::Example(ex) = &file.declarations[0] else {
                panic!()
            };
            let Expression::Binary { left, right, .. } = &ex.body else {
                panic!()
            };
            for side in [left, right] {
                let Expression::Reference { widths, args, .. } = &**side else {
                    panic!()
                };
                assert_eq!(widths.len(), 1);
                assert!(args.is_some());
            }
        },
    );
}

// ---------------------------------------------------------------------------
// The canonical grammar artifacts (warrant.tokens / warrant.grammar)
// ---------------------------------------------------------------------------

#[test]
fn the_canonical_grammar_accepts_the_new_literals_and_the_optional_header_word() {
    use support::{parse, Fault};
    let accept = |src: &str| {
        assert!(
            parse(src, true, Fault::None).is_some(),
            "loaded grammar rejected {src:?}"
        );
        assert!(
            parse(src, false, Fault::None).is_some(),
            "embedded grammar rejected {src:?}"
        );
        assert_eq!(
            parse(src, true, Fault::None),
            parse(src, false, Fault::None),
            "the embedded artifact must match the canonical file"
        );
    };
    let reject = |src: &str| {
        assert!(
            parse(src, true, Fault::None).is_none(),
            "loaded grammar accepted {src:?}"
        );
        assert!(
            parse(src, false, Fault::None).is_none(),
            "embedded grammar accepted {src:?}"
        );
    };
    for lit in [
        "5u16",
        "5u32",
        "5i8",
        "5i16",
        "5i32",
        "5i64",
        "0x1d",
        "0x1du8",
        "0xffffu16",
        "0x0001i32",
        "255u8",
        "5u64",
    ] {
        accept(&format!("warrant t v1\n  example e:\n    {lit} == {lit}\n"));
    }
    accept("warrant t v1\n  error E = A\n");
    accept("warrant t\n  error E = A\n");
    // A third header word, and the lexemes the token regexes never matched.
    reject("warrant t v1 extra\n  error E = A\n");
    for lit in ["0xU8", "0xFF", "01u16", "1_000", "1u128", "0x"] {
        reject(&format!("warrant t v1\n  example e:\n    {lit} == {lit}\n"));
    }
}

#[test]
fn the_new_diagnostics_have_fixed_safe_messages_and_oversized_decimals_are_syntax_errors() {
    let message = |header: &str, expr: &str| literal(header, expr).expect_err(expr).to_string();
    assert_eq!(
        message(" v1", "256u8"),
        "W0101: literal out of range for its type"
    );
    assert_eq!(message("", "5u16"), "W0101: v1 construct in a v0 document");
    run("warrant x v2\n  error E = A\n", |p| {
        assert_eq!(p.unwrap_err().to_string(), "W0101: invalid version header");
    });
    // No source text leaks into any of them.
    for text in [message(" v1", "256u8"), message("", "5u16")] {
        assert!(!text.contains("256") && !text.contains("5u16"), "{text}");
    }
    // A plain decimal beyond u64 is the same W0001 syntax error as ever.
    for header in ["", " v1"] {
        assert_eq!(
            code_of(header, "99999999999999999999"),
            ("W0001", ParserErrorKind::UnexpectedToken)
        );
    }
}
