//! VER33 section 4 / ADL-001a: the `profile algorithm` header, its refusals,
//! and the `realizes` binding schema.
//!
//! The corpus is `code/specs/fixtures/warrant-profile-algorithm-v1/`: one file
//! per case, the expected outcome on the first line (`-- expect: ok|W0801`
//! in a document, `# expect: ...` in a binding file).  These tests drive the
//! real guarded parser, checker and binding validator over every file, and
//! then pin the properties the corpus cannot say: spans, the closed
//! embedding set, the panic-template rules, and that a pure document is
//! untouched.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use coding_adventures_warrant_grammar::binding::{
    check_panic_template, parse_evidence_bindings, validate_realizations, ModelModule, Realization,
    EMBEDDINGS, MAX_PANIC_TEMPLATES,
};
use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckedWarrantModule, CheckerErrorKind, DiagnosticCode,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, OpDecl, Parameter, ParserErrorKind, ParserLimits, Profile,
    TypeExpr, Version, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .join("code/specs/fixtures/warrant-profile-algorithm-v1")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(corpus_dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Parse and check `src`: `Ok((profile, checked))`, or the code of the first
/// refusal (parser or checker) with its byte span.
type Refusal = (String, Option<(usize, usize)>);

fn load(src: &str) -> Result<(Profile, CheckedWarrantModule), Refusal> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .map_err(|e| (e.code().to_string(), e.span()))?;
    let profile = file.profile;
    let checked = check_warrant(&file).map_err(|e| (e.code.to_string(), Some(e.span)))?;
    Ok((profile, checked))
}

fn expectation(text: &str, marker: &str) -> String {
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix(marker))
        .unwrap_or_else(|| panic!("first line must start with `{marker}`"))
        .trim()
        .to_string()
}

fn corpus_files(ext: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus_dir())
        .expect("corpus directory")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(ext))
        .collect();
    names.sort();
    names
}

#[test]
fn every_document_case_has_its_authored_outcome() {
    let mut codes = BTreeSet::new();
    let mut seen = 0;
    for name in corpus_files(".warrant") {
        if name.starts_with("bind-") {
            continue;
        }
        let src = read(&name);
        let want = expectation(&src, "-- expect:");
        let got = load(&src).map(|_| "ok".to_string()).map_err(|(c, _)| c);
        match (&want[..], &got) {
            ("ok", Ok(_)) => {}
            (w, Err(g)) if w == g => {}
            _ => panic!("{name}: expected {want}, got {got:?}"),
        }
        codes.insert(want);
        seen += 1;
    }
    assert!(seen >= 20, "the corpus shrank: {seen} document cases");
    // At least one positive and one negative per diagnostic this slice adds.
    for code in ["ok", "W0101", "W0801", "W0802", "W0803", "W0804", "W0805"] {
        assert!(codes.contains(code), "no corpus case expects {code}");
    }
}

#[test]
fn every_binding_case_has_its_authored_outcome() {
    let mut codes = BTreeSet::new();
    let mut seen = 0;
    for name in corpus_files(".bindings") {
        let text = read(&name);
        let want = expectation(&text, "# expect:");
        let directive = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("# {key}:")))
                .unwrap_or_else(|| panic!("{name}: missing `# {key}:`"))
                .trim()
                .to_string()
        };
        let ops = load(&read(&directive("ops"))).expect("ops document checks");
        let model_src = read(&directive("model"));
        let model_doc =
            SourceDocument::admit(model_src.as_bytes(), SourceLimits::default()).unwrap();
        let model_layout = capture_layout(&model_doc, LayoutLimits::default()).unwrap();
        let model_file =
            parse_document(&model_layout, &model_doc, ParserLimits::default()).unwrap();
        let model_checked = check_warrant(&model_file).expect("model document checks");
        let model = ModelModule::new(&model_file, &model_checked);

        let got: Result<(), String> = match parse_evidence_bindings(&text) {
            // A parse error reads `line N: W0xxx: reason`; the code is its second field.
            Err(message) => Err(message
                .split(": ")
                .find(|part| part.len() == 5 && part.starts_with('W'))
                .unwrap_or(&message)
                .to_string()),
            Ok(file) => validate_realizations(&ops.1, &file.realizations, &[model])
                .map_err(|e| e.code.to_string()),
        };
        match (&want[..], &got) {
            ("ok", Ok(())) => {}
            (w, Err(g)) if w == g => {}
            _ => panic!("{name}: expected {want}, got {got:?}"),
        }
        codes.insert(want);
        seen += 1;
    }
    assert!(seen >= 20, "the corpus shrank: {seen} binding cases");
    for code in ["ok", "W0806", "W0807"] {
        assert!(codes.contains(code), "no binding case expects {code}");
    }
}

// ── The header ──────────────────────────────────────────────────────────────

fn parse_ok<R>(src: &str, f: impl FnOnce(&WarrantFile<'_>) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

fn parse_err(src: &str) -> (ParserErrorKind, &'static str, Option<(usize, usize)>) {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let e = parse_document(&layout, &doc, ParserLimits::default()).expect_err(src);
    (e.kind(), e.code(), e.span())
}

#[test]
fn the_header_selects_the_profile_and_the_version_independently() {
    parse_ok("warrant m\n  error E = A\n", |f| {
        assert_eq!((f.version, f.profile), (Version::V0, Profile::Pure));
    });
    parse_ok("warrant m v1\n  error E = A\n", |f| {
        assert_eq!((f.version, f.profile), (Version::V1, Profile::Pure));
    });
    parse_ok("warrant m v1 profile algorithm\n  error E = A\n", |f| {
        assert_eq!((f.version, f.profile), (Version::V1, Profile::Algorithm));
    });
}

#[test]
fn a_refused_word_is_reported_at_that_word() {
    let src = "warrant m v1 profile algorithm\n  model f(x: Int) -> Int = while x\n";
    let (kind, code, span) = parse_err(src);
    assert_eq!(
        (kind, code),
        (ParserErrorKind::ProfileUnboundedLoop, "W0803")
    );
    let (a, b) = span.expect("span");
    assert_eq!(&src[a..b], "while");

    // The measure form is named as `measure` even though it starts with `rec`.
    let src = "warrant m v1 profile algorithm\n  rec model f(n: Nat) -> Nat measure n = n\n";
    let (kind, code, span) = parse_err(src);
    assert_eq!((kind, code), (ParserErrorKind::ProfileMeasure, "W0802"));
    let (a, b) = span.expect("span");
    assert_eq!(&src[a..b], "measure");

    // A bad header points at the offending word.
    let src = "warrant m v1 profile fast\n  error E = A\n";
    let (kind, code, span) = parse_err(src);
    assert_eq!(
        (kind, code),
        (ParserErrorKind::InvalidProfileHeader, "W0801")
    );
    let (a, b) = span.expect("span");
    assert_eq!(&src[a..b], "fast");
}

#[test]
fn the_first_refused_word_in_source_order_wins() {
    let src = "warrant m v1 profile algorithm\n  model f(x: F64) -> Int = loop x\n";
    assert_eq!(parse_err(src).1, "W0804");
}

#[test]
fn a_pure_document_is_never_refused_for_the_profile_words() {
    for src in [
        "warrant m\n  model rec(measure: Int) -> Int = measure\n",
        "warrant m v1\n  model f(state: Int, io: Int, loop: Int) -> Int = state + io + loop\n",
        "warrant m v1\n  op make(n: U8) -> Bytes fresh\n",
    ] {
        parse_ok(src, |f| assert_eq!(f.profile, Profile::Pure));
    }
}

#[test]
fn the_algorithm_profile_does_not_refuse_what_it_admits() {
    // `fold`/`range` are the bounded loops of VER33 section 4.1: not in the table.
    let src = "warrant m v1 profile algorithm\n  model fold(range: Int) -> Int = range\n";
    parse_ok(src, |f| assert_eq!(f.profile, Profile::Algorithm));
}

// ── The checker's own guard, over an AST built without the parser ──────────

fn algorithm_file<'a>(declarations: Vec<Declaration<'a>>) -> WarrantFile<'a> {
    WarrantFile {
        name: "hand_built",
        version: Version::V1,
        profile: Profile::Algorithm,
        declarations,
        span: (0, 0),
    }
}

fn named(name: &str) -> TypeExpr<'_> {
    TypeExpr::Named { name, span: (3, 4) }
}

fn op<'a>(is_fresh: bool, params: Vec<Parameter<'a>>, ret: TypeExpr<'a>) -> Declaration<'a> {
    Declaration::Op(OpDecl {
        name: "o",
        static_params: vec![],
        params,
        return_type: ret,
        is_fresh,
        span: (1, 2),
    })
}

fn param<'a>(inout: bool, ty: TypeExpr<'a>) -> Parameter<'a> {
    Parameter {
        name: "x",
        param_type: ty,
        refinement: None,
        inout,
        span: (5, 6),
    }
}

#[test]
fn the_checker_refuses_effects_and_floats_in_a_hand_built_profile_ast() {
    let cases = [
        (
            op(true, vec![], named("Int")),
            DiagnosticCode::W0805,
            "fresh",
        ),
        (
            op(false, vec![param(true, named("Int"))], named("Int")),
            DiagnosticCode::W0805,
            "inout",
        ),
        (
            op(false, vec![], named("F64")),
            DiagnosticCode::W0804,
            "float result",
        ),
        (
            op(false, vec![param(false, named("F32"))], named("Int")),
            DiagnosticCode::W0804,
            "float param",
        ),
        (
            op(
                false,
                vec![],
                TypeExpr::Option {
                    inner: Box::new(named("F64")),
                    span: (7, 8),
                },
            ),
            DiagnosticCode::W0804,
            "float under Option",
        ),
    ];
    for (decl, code, what) in cases {
        let file = algorithm_file(vec![decl]);
        let err = check_warrant(&file).expect_err(what);
        assert_eq!(err.code, code, "{what}");
        assert!(
            matches!(err.kind, CheckerErrorKind::ProfileRefusal(_)),
            "{what}"
        );
    }
    // The same ops are not refused in a pure document (the guard is the profile's).
    let mut pure = algorithm_file(vec![op(true, vec![], named("Int"))]);
    pure.profile = Profile::Pure;
    assert_ne!(
        check_warrant(&pure).err().map(|e| e.code),
        Some(DiagnosticCode::W0805)
    );
}

// ── Panic templates and the embedding set ──────────────────────────────────

#[test]
fn panic_templates_must_be_exact_and_authored() {
    for ok in [
        "a must be 0 or 1",
        "a must be 0 or 1, got {}",
        "input value {} is not a bit",
        "braces {{ and }} escaped",
        "exactly8 {}",
        "gate input out of domain",
    ] {
        assert_eq!(check_panic_template(ok), Ok(()), "{ok}");
    }
    for bad in [
        "",
        "   ",
        "{}",
        "{} {}",
        "a must be *",
        "?",
        ".*",
        " leading",
        "trailing ",
        "tab\there",
        "bare { brace",
        "bare } brace",
        "{name}",
        "attempt to add with overflow",
        "Attempt To Subtract With Overflow",
        "called `Option::unwrap()` on a `None` value",
        "called `Result::unwrap()` on an `Err` value: {}",
        "index out of bounds: the len is {} but the index is {}",
        "attempt to divide by zero",
        "runtime error: index out of range [5] with length 3",
        "ZeroDivisionError: division by zero",
        &"x".repeat(257),
        // Security review, finding 1: near-wildcards (the anchor rule).
        "{}.",
        "a{}",
        "{}: {}",
        "{} !",
        "{} is not a bit",
        "x",
        "short {}",
        "a b c d {} efghijkl",
        // Finding 2: non-printable-ASCII, and the longer denylist.
        "a must be\u{a0}0 or 1 here",
        "must be a bit\u{200b}ok",
        "must be a bit\u{202e}ok",
        "must be a bit\u{2028}ok",
        "caf\u{e9} must be a bit",
        "index  out  of  bounds",
        "attempt  to add",
        "index {} out of bounds for the buffer",
        "Index 5 out of bounds for length 3",
        "java.lang.ArithmeticException: / by zero",
        "runtime error: slice bounds out of range [:5] with capacity 3",
        "panic: assignment to entry in nil map",
        "assertion failed: left == right",
        "IndexError: pop from empty list",
        "ZeroDivisionError: integer division or modulo by zero",
        "IndexError: list index out of range",
        "TypeError: value is not a function",
        "TypeError: Cannot read properties of undefined",
        "memory allocation: out of memory",
        "called Result::unwrap on an Err value",
        "called Option::unwrap on a None value",
        "thread main has overflowed its stack overflow",
        "capacity overflow while growing",
    ] {
        assert!(
            check_panic_template(bad).is_err(),
            "{bad:?} must be refused"
        );
    }
}

#[test]
fn the_embedding_set_is_closed() {
    assert_eq!(EMBEDDINGS, ["int-by-value", "bytes-identity"]);
    let mut r = Realization {
        module: "m".into(),
        model: "f".into(),
        embedding: "int-by-value".into(),
        panic_messages: vec![],
    };
    assert_eq!(r.check_shape("op"), Ok(()));
    for unknown in [
        "",
        "Int-By-Value",
        "int-by-value ",
        "custom",
        "option-componentwise",
    ] {
        r.embedding = unknown.into();
        let (code, _) = r.check_shape("op").unwrap_err();
        assert_eq!(code, DiagnosticCode::W0806, "{unknown:?}");
    }
    r.embedding = "bytes-identity".into();
    assert_eq!(r.check_shape("op"), Ok(()));
}

#[test]
fn the_panic_set_is_finite_and_distinct() {
    let many: Vec<String> = (0..=MAX_PANIC_TEMPLATES)
        .map(|i| format!("bad value {i}"))
        .collect();
    let r = Realization {
        module: "m".into(),
        model: "f".into(),
        embedding: "int-by-value".into(),
        panic_messages: many,
    };
    let (code, _) = r.check_shape("op").unwrap_err();
    assert_eq!(code, DiagnosticCode::W0807);
}

#[test]
fn a_binding_file_without_realizes_parses_exactly_as_before() {
    let text = read("bind-pos-no-realizes-unchanged.bindings");
    let file = parse_evidence_bindings(&text).expect("parses");
    assert!(file.realizations.is_empty());
    let with = parse_evidence_bindings(&read("bind-pos-realizes.bindings")).expect("parses");
    let r: &BTreeMap<_, _> = &with.realizations;
    assert_eq!(r.len(), 1);
    let nand = &r["nand_gate"];
    assert_eq!(
        (
            nand.module.as_str(),
            nand.model.as_str(),
            nand.embedding.as_str()
        ),
        ("logic_gates_model", "nand_gate", "int-by-value")
    );
    assert_eq!(
        nand.panic_messages,
        vec![
            "a must be 0 or 1".to_string(),
            "b must be 0 or 1".to_string()
        ]
    );
}

#[test]
fn realization_model_modules_come_from_a_checked_file_and_are_unique() {
    use coding_adventures_warrant_grammar::binding::Realization;
    let model_src = read("bind-model.warrant");
    let pure_src = read("bind-model-pure.warrant");
    let ops_src = read("bind-ops.warrant");
    let r: BTreeMap<String, Realization> = [(
        "nand_gate".to_string(),
        Realization {
            module: "logic_gates_model".into(),
            model: "nand_gate".into(),
            embedding: "int-by-value".into(),
            panic_messages: vec![],
        },
    )]
    .into();
    let (_, ops) = load(&ops_src).unwrap();
    let with = |src: &str, f: &mut dyn FnMut(ModelModule<'_>)| {
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        let checked = check_warrant(&file).unwrap();
        f(ModelModule::new(&file, &checked));
    };
    with(&model_src, &mut |m| {
        assert!(validate_realizations(&ops, &r, &[m]).is_ok());
        // The same module twice is refused (a shadowed duplicate).
        let e = validate_realizations(&ops, &r, &[m, m]).unwrap_err();
        assert_eq!(e.code, DiagnosticCode::W0806);
    });
    // A pure document's profile is read from its own file: refused.
    with(&pure_src, &mut |m| {
        let e = validate_realizations(&ops, &r, &[m]).unwrap_err();
        assert_eq!(e.code, DiagnosticCode::W0806);
    });
}

#[test]
fn the_profile_words_are_reserved_in_the_module_name_too() {
    for name in ["measure", "state", "rec", "F64"] {
        let src = format!("warrant {name} v1 profile algorithm\n  error E = A\n");
        let (_, code, _) = parse_err(&src);
        assert!(code.starts_with("W08"), "{name}: {code}");
    }
}
