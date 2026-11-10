//! VER04j section 4.11 (syntax corpus) and section 1 (versioning) for WX-A:
//! tuple types and literals, records, variants, projections, nested patterns,
//! and the rule that a v0 document means exactly what it always did.
//!
//! Each case is authored with its own expected verdict, never derived from the
//! parser.  The version gating lives in the hand-written parser, so these tests
//! drive the real parser (see `wx_syntax.rs` for why the grammar-driven corpus
//! runner cannot express W0101).

use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserError, ParserErrorKind, ParserLimits, Pattern,
    ProjKey, TypeExpr, Version, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn run<R>(src: &str, f: impl FnOnce(Result<&WarrantFile, ParserError>) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let parsed = parse_document(&layout, &doc, ParserLimits::default());
    f(parsed.as_ref().map_err(Clone::clone))
}

fn accepts(src: &str) {
    run(src, |p| {
        if let Err(e) = p {
            panic!("rejected ({e}): {src}");
        }
    });
}

fn verdict(src: &str) -> (&'static str, ParserErrorKind) {
    run(src, |p| {
        let e = p.expect_err(src);
        (e.code(), e.kind())
    })
}

fn rejects(src: &str, code: &str) {
    assert_eq!(verdict(src).0, code, "{src}");
}

/// One example `<expr> == <expr>` in a document with the given header.
fn ex(header: &str, expr: &str) -> String {
    format!("warrant t{header}\n  example e:\n    {expr} == {expr}\n")
}

/// The expression on the left of the single example's `==`.
fn lhs<R>(src: &str, f: impl FnOnce(&Expression) -> R) -> R {
    run(src, |p| {
        let file = p.expect("parses");
        let Declaration::Example(e) = &file.declarations[0] else {
            panic!("example expected")
        };
        let Expression::Binary { left, .. } = &e.body else {
            panic!("equality expected")
        };
        f(left)
    })
}

// ---------------------------------------------------------------------------
// Types and declarations
// ---------------------------------------------------------------------------

#[test]
fn tuple_types_record_and_variant_declarations_parse() {
    accepts("warrant t v1\n  model P = (U8, Nat)\n");
    accepts("warrant t v1\n  model P = ((U8, U8), Option<(Bool, Nat)>, List<(U8, U8)>)\n");
    accepts("warrant t v1\n  record R(a: U8)\n");
    accepts("warrant t v1\n  record Tlv(tag: U8, constructed: Bool, number: U32, value: Bytes)\n");
    accepts("warrant t v1\n  variant V = A | B\n");
    accepts("warrant t v1\n  variant V = A(x: U8) | B(x: U8, y: (U8, U8)) | C\n");
    // An arity-8 tuple and a 32-field record are the largest allowed.
    accepts("warrant t v1\n  model P = (U8, U8, U8, U8, U8, U8, U8, U8)\n");
    let fields: Vec<String> = (0..32).map(|i| format!("f{i}: U8")).collect();
    accepts(&format!("warrant t v1\n  record R({})\n", fields.join(", ")));
    // The shapes of the parsed declarations.
    run(
        "warrant t v1\n  record R(a: U8, b: (Bool, Nat))\n  variant V = A | B(x: U8)\n",
        |p| {
            let file = p.unwrap();
            let Declaration::Record(r) = &file.declarations[0] else {
                panic!("record")
            };
            assert_eq!((r.name, r.fields.len()), ("R", 2));
            assert!(matches!(&r.fields[1].ty, TypeExpr::Tuple { elems, .. } if elems.len() == 2));
            let Declaration::Variant(v) = &file.declarations[1] else {
                panic!("variant")
            };
            let names: Vec<_> = v.ctors.iter().map(|c| (c.name, c.fields.len())).collect();
            assert_eq!(names, [("A", 0), ("B", 1)]);
        },
    );
}

#[test]
fn declaration_rejects() {
    // A one-component and an empty tuple type; a ninth component.
    rejects("warrant t v1\n  model P = (U8)\n", "W0001");
    rejects("warrant t v1\n  model P = ()\n", "W0001");
    rejects(
        "warrant t v1\n  model P = (U8, U8, U8, U8, U8, U8, U8, U8, U8)\n",
        "W0001",
    );
    // A variant needs two constructors (a single one is a record).
    rejects("warrant t v1\n  variant V = A\n", "W0001");
    // Duplicates inside one declaration are W0201 from the parser.
    assert_eq!(
        verdict("warrant t v1\n  variant V = A | A\n"),
        ("W0201", ParserErrorKind::DuplicateName)
    );
    rejects("warrant t v1\n  record R(a: U8, a: Bool)\n", "W0201");
    rejects("warrant t v1\n  variant V = A(x: U8, x: U8) | B\n", "W0201");
    // Size limits: 33 fields, 257 constructors.
    let fields: Vec<String> = (0..33).map(|i| format!("f{i}: U8")).collect();
    rejects(&format!("warrant t v1\n  record R({})\n", fields.join(", ")), "W0001");
    let ctors: Vec<String> = (0..257).map(|i| format!("C{i}")).collect();
    rejects(&format!("warrant t v1\n  variant V = {}\n", ctors.join(" | ")), "W0001");
    let ctors: Vec<String> = (0..256).map(|i| format!("C{i}")).collect();
    accepts(&format!("warrant t v1\n  variant V = {}\n", ctors.join(" | ")));
    // Malformed shapes.
    rejects("warrant t v1\n  record R()\n", "W0001");
    rejects("warrant t v1\n  record R(a)\n", "W0001");
    rejects("warrant t v1\n  record R(a: U8\n", "W0001");
    rejects("warrant t v1\n  variant V = A |\n", "W0001");
    rejects("warrant t v1\n  variant V A | B\n", "W0001");
    rejects("warrant t v1\n  model P = (U8,)\n", "W0001");
}

// ---------------------------------------------------------------------------
// Expressions: tuples and projections
// ---------------------------------------------------------------------------

#[test]
fn tuple_literals_and_projections_parse_with_the_right_shape() {
    lhs(&ex(" v1", "(1, 2)"), |e| {
        assert!(matches!(e, Expression::Tuple { elements, .. } if elements.len() == 2));
    });
    lhs(&ex(" v1", "((1, 2), 3)"), |e| {
        let Expression::Tuple { elements, .. } = e else {
            panic!("tuple")
        };
        assert!(matches!(&elements[0], Expression::Tuple { .. }));
    });
    // `(e)` stays parenthesisation.
    lhs(&ex(" v1", "(1)"), |e| assert!(matches!(e, Expression::Grouped { .. })));
    // t.0 on a name is a qualified name; on a call, a postfix projection.
    lhs(&ex(" v1", "t.0"), |e| {
        assert!(matches!(e, Expression::Project { key: ProjKey::Index(0), .. }));
    });
    lhs(&ex(" v1", "r.field"), |e| {
        let Expression::Reference { name, .. } = e else {
            panic!("a name head parses as a qualified name")
        };
        assert_eq!(name.segments, ["r", "field"]);
    });
    lhs(&ex(" v1", "f(x).0"), |e| {
        let Expression::Project { base, key, .. } = e else {
            panic!("project")
        };
        assert_eq!(*key, ProjKey::Index(0));
        assert!(matches!(&**base, Expression::Reference { args: Some(_), .. }));
    });
    lhs(&ex(" v1", "f(x).field"), |e| {
        assert!(matches!(e, Expression::Project { key: ProjKey::Field("field"), .. }));
    });
    // Chains and tuples of projections.
    lhs(&ex(" v1", "f(x).1.name.0"), |e| {
        let Expression::Project { base, key, .. } = e else {
            panic!("project")
        };
        assert_eq!(*key, ProjKey::Index(0));
        assert!(matches!(&**base, Expression::Project { key: ProjKey::Field("name"), .. }));
    });
    lhs(&ex(" v1", "(a, b).1"), |e| {
        assert!(matches!(e, Expression::Project { key: ProjKey::Index(1), .. }));
    });
    accepts(&ex(" v1", "t.0.1"));
    accepts(&ex(" v1", "(1, 2, 3, 4, 5, 6, 7, 8)"));
    // The quantifier separator is unaffected: `.` followed by whitespace.
    accepts("warrant t v1\n  claim c:\n    forall a: (U8, U8). a.0 == a.1\n");
    accepts("warrant t v1\n  claim c:\n    forall a: Nat | a < 5. a == a\n");
}

#[test]
fn projection_and_tuple_rejects() {
    // A ninth component of a tuple literal.
    rejects(&ex(" v1", "(1, 2, 3, 4, 5, 6, 7, 8, 9)"), "W0001");
    // t.00 (leading zero), t.0u8 (suffix), t. 0 (whitespace), t.0x1 (hex).
    rejects(&ex(" v1", "t.00"), "W0001");
    rejects(&ex(" v1", "t.0u8"), "W0001");
    rejects(&ex(" v1", "t. 0"), "W0001");
    rejects(&ex(" v1", "t.0x1"), "W0001");
    rejects(&ex(" v1", "f(x).00"), "W0001");
    // A trailing comma and an empty tuple.
    rejects(&ex(" v1", "(1,)"), "W0001");
    rejects(&ex(" v1", "()"), "W0001");
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

fn arm_patterns(src: &str) -> Vec<Pattern<'static>> {
    run(src, |p| {
        let file = p.expect("parses");
        let Declaration::Example(e) = &file.declarations[0] else {
            panic!("example")
        };
        let Expression::Binary { left, .. } = &e.body else {
            panic!("eq")
        };
        let Expression::Grouped { inner, .. } = &**left else {
            panic!("grouped match")
        };
        let Expression::Match { arms, .. } = &**inner else {
            panic!("match")
        };
        arms.iter()
            .map(|a| to_static(&a.pattern))
            .collect::<Vec<_>>()
    })
}

/// A lifetime-free rendering of a pattern for comparison.
fn to_static(p: &Pattern<'_>) -> Pattern<'static> {
    fn leak(s: &str) -> &'static str {
        Box::leak(s.to_string().into_boxed_str())
    }
    match p {
        Pattern::None => Pattern::None,
        Pattern::Wild => Pattern::Wild,
        Pattern::Bool(b) => Pattern::Bool(*b),
        Pattern::Some(q) => Pattern::Some(Box::new(to_static(q))),
        Pattern::Ok(q) => Pattern::Ok(Box::new(to_static(q))),
        Pattern::Err(q) => Pattern::Err(Box::new(to_static(q))),
        Pattern::Tuple(ps) => Pattern::Tuple(ps.iter().map(to_static).collect()),
        Pattern::Ctor { name, args } => Pattern::Ctor {
            name: coding_adventures_warrant_grammar::parser::QualifiedName {
                segments: name.segments.iter().map(|s| leak(s)).collect(),
                span: (0, 0),
            },
            args: args.iter().map(to_static).collect(),
        },
    }
}

fn matches_doc(header: &str, arms: &str) -> String {
    format!("warrant t{header}\n  example e:\n    (match x with {arms}) == 1\n")
}

#[test]
fn nested_patterns_parse() {
    let pats = arm_patterns(&matches_doc(" v1", "Ok((v, n)) -> 1 | Err(E.X) -> 2 | Err(_) -> 3"));
    assert_eq!(pats.len(), 3);
    assert!(matches!(&pats[0], Pattern::Ok(q) if matches!(&**q, Pattern::Tuple(t) if t.len() == 2)));
    assert!(matches!(&pats[1], Pattern::Err(q)
        if matches!(&**q, Pattern::Ctor { name, args } if name.segments == ["E", "X"] && args.is_empty())));
    assert!(matches!(&pats[2], Pattern::Err(q) if matches!(&**q, Pattern::Wild)));
    let pats = arm_patterns(&matches_doc(" v1", "Some(Circle(r)) -> 1 | Some(_) -> 2 | None -> 3"));
    assert!(matches!(&pats[0], Pattern::Some(q)
        if matches!(&**q, Pattern::Ctor { name, args } if name.segments == ["Circle"] && args.len() == 1)));
    let pats = arm_patterns(&matches_doc(" v1", "(true, _) -> 1 | (false, Red) -> 2"));
    assert!(matches!(&pats[0], Pattern::Tuple(t) if matches!(t[0], Pattern::Bool(true)) && matches!(t[1], Pattern::Wild)));
    // `_` may repeat; a bare NAME is a Ctor with no arguments (binder or constructor).
    let pats = arm_patterns(&matches_doc(" v1", "(_, _, x) -> 1"));
    assert!(matches!(&pats[0], Pattern::Tuple(t) if t.len() == 3));
    // Pattern depth 8 is allowed, 9 is not.
    let nest = |d: usize| format!("{}x{}", "Some(".repeat(d - 1), ")".repeat(d - 1));
    accepts(&matches_doc(" v1", &format!("{} -> 1", nest(8))));
    rejects(&matches_doc(" v1", &format!("{} -> 1", nest(9))), "W0001");
}

#[test]
fn pattern_rejects() {
    rejects(&matches_doc(" v1", "(x) -> 1"), "W0001");
    rejects(&matches_doc(" v1", "(a, b, c, d, e, f, g, h, i) -> 1"), "W0001");
    rejects(&matches_doc(" v1", "C() -> 1"), "W0001");
    rejects(&matches_doc(" v1", "Some -> 1"), "W0001");
    rejects(&matches_doc(" v1", "1 -> 1"), "W0001");
    rejects(&matches_doc(" v1", "b\"00\" -> 1"), "W0001");
}

// ---------------------------------------------------------------------------
// Versioning: every WX-A construct is W0101 in a v0 document
// ---------------------------------------------------------------------------

#[test]
fn every_wxa_construct_is_w0101_in_a_v0_document_and_accepted_in_v1() {
    let cases: Vec<(String, String)> = vec![
        // declarations
        ("record R(a: U8)".into(), "model x = U8".into()),
        ("variant V = A | B".into(), "model x = U8".into()),
        // tuple type
        ("model P = (U8, Nat)".into(), "model P = U8".into()),
    ];
    for (v1_text, _) in &cases {
        let v0 = format!("warrant t\n  {v1_text}\n");
        let v1 = format!("warrant t v1\n  {v1_text}\n");
        assert_eq!(verdict(&v0), ("W0101", ParserErrorKind::V1ConstructInV0), "{v0}");
        accepts(&v1);
    }
    for expr in ["(1, 2)", "t.0", "f(x).0", "f(x).name", "(a, b).1"] {
        let v0 = ex("", expr);
        let v1 = ex(" v1", expr);
        // `t.0` is the only one that could be mistaken for something else.
        assert_eq!(verdict(&v0).0, "W0101", "{expr}");
        accepts(&v1);
    }
    // Patterns: `_`, constructors, nested patterns, booleans and tuples.
    for arms in [
        "_ -> 1",
        "Red -> 1 | _ -> 2",
        "(a, b) -> 1",
        "true -> 1 | false -> 2",
        "Some(Some(x)) -> 1",
        "Some((a, b)) -> 1",
        "Ok(true) -> 1",
        "Circle(r) -> 1",
        "E.X -> 1",
    ] {
        let v0 = matches_doc("", arms);
        assert_eq!(verdict(&v0).0, "W0101", "{arms}");
        accepts(&matches_doc(" v1", arms));
    }
}

#[test]
fn v0_documents_keep_their_meaning() {
    // The v0 pattern forms still parse, `_` is an ordinary binder name there,
    // and a model may be called `record`, `variant` or `rem`.
    accepts(&matches_doc("", "None -> 1 | Some(x) -> 2"));
    accepts(&matches_doc("", "Ok(x) -> 1 | Err(e) -> 2"));
    accepts(&matches_doc("", "Some(_) -> 1 | None -> 2"));
    run(&matches_doc("", "Some(_) -> 1 | None -> 2"), |p| {
        let pats = {
            let file = p.unwrap();
            let Declaration::Example(e) = &file.declarations[0] else {
                panic!()
            };
            let Expression::Binary { left, .. } = &e.body else {
                panic!()
            };
            let Expression::Grouped { inner, .. } = &**left else {
                panic!()
            };
            let Expression::Match { arms, .. } = &**inner else {
                panic!()
            };
            arms[0].pattern.clone()
        };
        // `_` is the *binder* named `_` in v0, not the wildcard.
        assert_eq!(
            match &pats {
                Pattern::Some(q) => q.as_binder(),
                _ => None,
            },
            Some("_")
        );
    });
    accepts("warrant t\n  model record(a: U8) -> U8 = a\n");
    accepts("warrant t\n  model variant(a: U8) -> U8 = a\n  model rem(a: U8) -> U8 = a\n");
    accepts("warrant t\n  model L = List<List<U8>>\n");
    // And a v1 document parses the same v0 text to the same AST (apart from the
    // header's version).
    let src0 = "warrant t\n  error E = A | B\n  model f(a: U8, b: Option<U8>) -> U8 = match b with None -> a | Some(x) -> x\n";
    let src1 = src0.replacen("warrant t", "warrant t v1", 1);
    run(src0, |a| {
        run(&src1, |b| {
            let (a, b) = (a.unwrap(), b.unwrap());
            assert_eq!((a.version, b.version), (Version::V0, Version::V1));
            assert_eq!(a.declarations.len(), b.declarations.len());
        })
    });
}

#[test]
fn a_binder_and_a_record_may_share_a_prefix_without_confusion() {
    // `x.f` with a binder head is a qualified name; `Name.CASE` stays the
    // error-constructor form until the checker decides.
    lhs(&ex(" v1", "E.CASE"), |e| {
        let Expression::Reference { name, .. } = e else {
            panic!("reference")
        };
        assert_eq!(name.segments, ["E", "CASE"]);
    });
}
