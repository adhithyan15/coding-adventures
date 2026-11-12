//! VER04k sections 1.2, 2.2, 2.4, 2.11 and 2.15 ("Checker negatives"): the
//! checker side of WX-D slice D1.
//!
//! What is pinned here, one rule per test:
//!
//! * the **typing** of the seven D1 forms (section 2.4), including the two
//!   places where an expected type has to flow inward: `replicate([], n)`
//!   and `to_bytes([])` only type-check because the empty list literal is
//!   given its element type by its context, exactly as `[]` is in v0;
//! * the **negatives**: an ordinary type error inside a WX-D call is W0301;
//! * **version scoping** of the twelve names (section 1.2): reserved in v1
//!   (a v1 declaration so named is W0201), unbound in v0 (a call is W0202),
//!   and a v0 model *named* `take` keeps meaning its own definition;
//! * the five **D2 names** (`fold map count all any`) are reserved in v1 but
//!   not implemented by this tool version: a use is rejected with a
//!   "not yet implemented in this version" diagnostic (W0202) instead of being
//!   silently treated as a user model or an unbound name;
//! * the **list-literal cap** (section 2.11): a v1 list literal of more than
//!   4,096 elements is W0602; v0 is unchanged;
//! * a **mechanical scan** that no in-repository v1 document uses one of the
//!   twelve names as an identifier (section 1.2: "the WX-D1 slice re-checks
//!   mechanically").

use std::path::{Path, PathBuf};

use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckedWarrantModule, CheckerError, CheckerErrorKind, DiagnosticCode, Type,
    MAX_LIST_LITERAL, V1_FUNCTIONS,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits, LayoutTokenKind};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits, Version};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

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

fn all_twelve() -> impl Iterator<Item = &'static str> {
    D1.iter().chain(D2.iter()).copied()
}

fn check(src: &str) -> Result<CheckedWarrantModule, CheckerError> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .unwrap_or_else(|e| panic!("parse: {e}\n{src}"));
    check_warrant(&file)
}

/// `model m(<params>) -> <ret> = <body>` in a v1 document.
fn v1_model(params: &str, ret: &str, body: &str) -> String {
    format!("warrant t v1\n  model m({params}) -> {ret} = {body}\n")
}

/// The type the checker gives the body of `m`.
fn ty_of(params: &str, ret: &str, body: &str) -> Type {
    match check(&v1_model(params, ret, body)) {
        Ok(m) => m.models["m"].body_type.clone(),
        Err(e) => panic!("rejected ({params}) -> {ret} = {body}: {e:?}"),
    }
}

fn rejected(src: &str) -> CheckerError {
    match check(src) {
        Ok(_) => panic!("accepted:\n{src}"),
        Err(e) => e,
    }
}

fn list(t: Type) -> Type {
    Type::List(Box::new(t))
}

// ---------------------------------------------------------------------------
// Typing (section 2.4)
// ---------------------------------------------------------------------------

#[test]
fn take_drop_and_slice_keep_a_list_and_turn_any_bytes_into_dynamic_bytes() {
    // D(List<T>) = List<T>; D(Bytes<N>) = D(Bytes) = Bytes.
    for form in ["take(s, 2)", "drop(s, 2)", "slice(s, 1, 2)"] {
        assert_eq!(
            ty_of("s: List<U8>", "List<U8>", form),
            list(Type::U8),
            "{form}"
        );
        assert_eq!(
            ty_of("s: List<(U8, Bool)>", "List<(U8, Bool)>", form),
            list(Type::Tuple(vec![Type::U8, Type::Bool])),
            "{form}"
        );
        assert_eq!(ty_of("s: Bytes", "Bytes", form), Type::Bytes, "{form}");
        assert_eq!(ty_of("s: Bytes<4>", "Bytes", form), Type::Bytes, "{form}");
    }
    // A bytes literal is a Bytes<N>, so it is a sequence too.
    assert_eq!(ty_of("a: U8", "Bytes", "take(b\"0102\", 1)"), Type::Bytes);
}

#[test]
fn every_count_is_an_int_so_nat_widens_and_a_negative_count_type_checks() {
    // `len(xs) - 1` is an Int (Nat - Nat is Int, VER04j section 14 item 5):
    // the reason counts are Int at all (section 2.2).
    for (params, body) in [
        ("s: Bytes, n: Nat", "take(s, n)"),
        ("s: Bytes, n: Int", "take(s, n)"),
        ("s: Bytes", "drop(s, len(s) - 1)"),
        ("s: Bytes", "take(s, -1)"),
        ("s: Bytes, a: Nat, n: Int", "slice(s, a, n)"),
    ] {
        assert_eq!(ty_of(params, "Bytes", body), Type::Bytes, "{body}");
    }
    assert_eq!(ty_of("n: Int", "List<Nat>", "range(n)"), list(Type::Nat));
    assert_eq!(
        ty_of("n: Nat", "List<Nat>", "range(n - 3)"),
        list(Type::Nat)
    );
    assert_eq!(
        ty_of("x: U8, n: Int", "List<U8>", "replicate(x, n)"),
        list(Type::U8)
    );
}

#[test]
fn replicate_range_to_list_and_to_bytes_have_their_table_types() {
    assert_eq!(
        ty_of("x: Bool", "List<Bool>", "replicate(x, 3)"),
        list(Type::Bool)
    );
    assert_eq!(
        ty_of("x: (U8, U8)", "List<(U8, U8)>", "replicate(x, 3)"),
        list(Type::Tuple(vec![Type::U8, Type::U8]))
    );
    assert_eq!(ty_of("a: U8", "List<Nat>", "range(8)"), list(Type::Nat));
    assert_eq!(ty_of("b: Bytes", "List<U8>", "to_list(b)"), list(Type::U8));
    assert_eq!(
        ty_of("b: Bytes<3>", "List<U8>", "to_list(b)"),
        list(Type::U8)
    );
    assert_eq!(ty_of("xs: List<U8>", "Bytes", "to_bytes(xs)"), Type::Bytes);
    assert_eq!(
        ty_of("xs: List<U8>", "Bytes", "to_bytes(take(xs, 1))"),
        Type::Bytes
    );
    // Round trips compose.
    assert_eq!(
        ty_of("b: Bytes", "Bytes", "to_bytes(to_list(b))"),
        Type::Bytes
    );
}

#[test]
fn an_expected_type_flows_into_the_sequence_and_element_arguments() {
    // `[]` needs its element type from context, as in v0 (section 2.4).
    assert_eq!(ty_of("a: U8", "Bytes", "to_bytes([])"), Type::Bytes);
    assert_eq!(ty_of("a: U8", "Bytes", "to_bytes([0u8, 1u8])"), Type::Bytes);
    assert_eq!(ty_of("a: U8", "Bytes", "to_bytes([a, 255u8])"), Type::Bytes);
    assert_eq!(
        ty_of("n: Int", "List<List<U8>>", "replicate([], n)"),
        list(list(Type::U8))
    );
    assert_eq!(ty_of("n: Int", "List<U8>", "take([], n)"), list(Type::U8));
    assert_eq!(ty_of("n: Int", "List<U8>", "drop([], n)"), list(Type::U8));
    assert_eq!(
        ty_of("n: Int", "List<U8>", "slice([], n, n)"),
        list(Type::U8)
    );
    // A contextual Nat literal: `replicate(0, 3)` as a List<Nat>.
    assert_eq!(
        ty_of("a: U8", "List<Nat>", "replicate(0, 3)"),
        list(Type::Nat)
    );
    // In an equality the left side provides the context of the right.
    check("warrant t v1\n  example e:\n    to_list(b\"\") == []\n").expect("to_list(b\"\") == []");
    check("warrant t v1\n  example e:\n    range(3) == [0, 1, 2]\n")
        .expect("range(3) == [0, 1, 2]");
}

// ---------------------------------------------------------------------------
// Negatives: ordinary type errors inside a WX-D call are W0301
// ---------------------------------------------------------------------------

#[test]
fn wrong_types_inside_a_d1_call_are_w0301() {
    for (params, ret, body) in [
        ("s: List<U8>", "List<U8>", "take(s, true)"),
        ("s: Bytes", "Bytes", "drop(s, 1u8)"),
        ("s: Bytes", "Bytes", "slice(s, 1, b\"00\")"),
        ("s: Bool", "Bool", "take(s, 1)"),
        ("s: U8", "U8", "drop(s, 1)"),
        ("s: Option<Bytes>", "Bytes", "take(s, 1)"),
        ("x: U8", "List<U8>", "replicate(x, true)"),
        ("x: U8", "List<Nat>", "range(x)"),
        ("x: U8", "List<Nat>", "range(true)"),
        ("xs: List<U8>", "List<U8>", "to_list(xs)"),
        ("b: Bytes", "Bytes", "to_bytes(b)"),
        ("xs: List<Nat>", "Bytes", "to_bytes(xs)"),
        ("xs: List<U16>", "Bytes", "to_bytes(xs)"),
        // An unsuffixed literal is an Int (VER04 section 2): `[0, 1]` is a
        // List<Int>, not a List<U8>, even where a List<U8> is expected.
        ("a: U8", "Bytes", "to_bytes([0, 1])"),
        // `[]` with no context has no element type (as in v0).
        ("n: Int", "Nat", "len(replicate([], n))"),
        // The result of `take` is never a Bytes<N> (section 2.2).
        ("b: Bytes<4>", "Bytes<4>", "take(b, 4)"),
        // List<U8> and Bytes are different types (section 2.4).
        ("b: Bytes", "Bool", "to_list(b) == b"),
        ("b: Bytes", "Bool", "to_bytes(to_list(b)) == to_list(b)"),
    ] {
        let e = rejected(&v1_model(params, ret, body));
        assert_eq!(
            e.code,
            DiagnosticCode::W0301,
            "({params}) -> {ret} = {body}: {e:?}"
        );
    }
}

#[test]
fn wrong_arity_and_static_arguments_are_w0301() {
    for body in [
        "take(s)",
        "take(s, 1, 2)",
        "drop(s)",
        "slice(s, 1)",
        "slice(s, 1, 2, 3)",
        "replicate(1u8)",
        "range()",
        "range(1, 2)",
        "to_list()",
        "to_bytes(to_list(s), 1)",
        "take<U8>(s, 1)",
        "len(take)",
    ] {
        let e = rejected(&v1_model("s: Bytes", "Nat", &format!("len({body})")));
        assert_eq!(e.code, DiagnosticCode::W0301, "{body}: {e:?}");
    }
}

// ---------------------------------------------------------------------------
// Version scoping (section 1.2)
// ---------------------------------------------------------------------------

#[test]
fn the_twelve_names_are_reserved_v1_functions() {
    for name in all_twelve() {
        assert!(
            V1_FUNCTIONS.contains(&name),
            "{name} must be in V1_FUNCTIONS"
        );
    }
    assert_eq!(V1_FUNCTIONS.len(), 45);
}

#[test]
fn a_v1_declaration_named_like_a_wx_d_built_in_is_w0201() {
    for name in all_twelve() {
        let e = rejected(&format!(
            "warrant t v1\n  model {name}(a: Nat) -> Nat = a\n"
        ));
        assert_eq!(e.code, DiagnosticCode::W0201, "model {name}: {e:?}");
        assert_eq!(e.kind, CheckerErrorKind::DuplicateSymbol(name.to_string()));
        let e = rejected(&format!("warrant t v1\n  model {name} = Bytes\n"));
        assert_eq!(e.code, DiagnosticCode::W0201, "alias {name}: {e:?}");
        let e = rejected(&format!("warrant t v1\n  record {name}(a: U8)\n"));
        assert_eq!(e.code, DiagnosticCode::W0201, "record {name}: {e:?}");
    }
}

#[test]
fn a_wx_d_call_in_a_v0_document_is_an_unbound_name() {
    for (name, call) in [
        ("take", "take(s, 1)"),
        ("drop", "drop(s, 1)"),
        ("slice", "slice(s, 1, 1)"),
        ("replicate", "replicate(s, 1)"),
        ("range", "range(1)"),
        ("to_list", "to_list(s)"),
        ("to_bytes", "to_bytes(s)"),
        ("fold", "fold(s, 0, f)"),
        ("map", "map(s, f)"),
        ("count", "count(s, f)"),
        ("all", "all(s, f)"),
        ("any", "any(s, f)"),
    ] {
        let src = format!("warrant t\n  model m(s: Bytes) -> Bytes = {call}\n");
        let e = rejected(&src);
        assert_eq!(e.code, DiagnosticCode::W0202, "{call}: {e:?}");
        assert_eq!(
            e.kind,
            CheckerErrorKind::UnboundName(name.to_string()),
            "{call}"
        );
    }
}

#[test]
fn a_v0_model_named_like_a_wx_d_built_in_keeps_its_own_meaning() {
    for name in all_twelve() {
        let src = format!(
            "warrant t\n  model {name}(a: Nat) -> Int = a + 1\n  model m(a: Nat) -> Int = {name}(a)\n  example e:\n    {name}(4) == 5\n"
        );
        let checked = check(&src).unwrap_or_else(|e| panic!("v0 model {name}: {e:?}"));
        assert_eq!(
            checked.models[name].params,
            vec![("a".to_string(), Type::Nat)]
        );
        // `m`'s body has the user model's own result type (`Int`); the
        // built-in, had it applied, would have been a type error here.
        assert_eq!(checked.models["m"].body_type, Type::Int, "{name}");
    }
}

#[test]
fn the_d2_names_are_reserved_but_not_yet_implemented_in_v1() {
    for (name, body) in [
        ("fold", "fold(s, 0, f)"),
        ("map", "map(s, f)"),
        ("count", "count(s, f)"),
        ("all", "all(s, f)"),
        ("any", "any(s, f)"),
        // A bare reference is rejected the same way.
        ("count", "count"),
    ] {
        let src = format!(
            "warrant t v1\n  model f(x: U8) -> Bool = x == 0u8\n  model m(s: Bytes) -> Nat = len(to_list(s)) + 0 * len([{body}])\n"
        );
        let e = rejected(&src);
        assert_eq!(e.code, DiagnosticCode::W0202, "{body}: {e:?}");
        assert_eq!(
            e.kind,
            CheckerErrorKind::NotYetImplemented(name.to_string()),
            "{body}"
        );
        // VER04k section 9.1, E7: the message names the built-in and nothing
        // else of the document.
        assert_eq!(
            e.to_string(),
            format!("W0202: reserved built-in `{name}` is not yet implemented in this version"),
            "{body}"
        );
    }
}

#[test]
fn the_d1_names_are_implemented_and_compose() {
    let src = v1_model(
        "s: Bytes, n: Int",
        "Bytes",
        "to_bytes(concat(take(to_list(s), n), concat(drop(to_list(s), n), slice(replicate(0u8, n), 0, len(range(n))))))",
    );
    check(&src).unwrap_or_else(|e| panic!("{e:?}"));
}

// ---------------------------------------------------------------------------
// The list-literal cap (section 2.11)
// ---------------------------------------------------------------------------

fn long_list(header: &str, n: usize) -> String {
    let items = vec!["0u8"; n].join(", ");
    format!("warrant t{header}\n  model m(a: U8) -> Nat = len([{items}])\n")
}

#[test]
fn a_v1_list_literal_longer_than_4096_elements_is_w0602_and_v0_is_unchanged() {
    assert_eq!(MAX_LIST_LITERAL, 4096);
    check(&long_list(" v1", 4096)).expect("4,096 elements are allowed in v1");
    let src = long_list(" v1", 4097);
    let e = rejected(&src);
    assert_eq!(e.code, DiagnosticCode::W0602, "{e:?}");
    // At the literal's own span, `[` through `]`.
    let open = src.find('[').unwrap();
    let close = src.rfind(']').unwrap() + 1;
    assert_eq!(e.span, (open, close));
    assert_eq!(
        e.kind,
        CheckerErrorKind::ListLiteralTooLong {
            len: 4097,
            limit: 4096
        }
    );
    // v0 never had this cap (its evaluator's 65,536-element guard remains).
    check(&long_list("", 4097)).expect("v0 accepts 4,097 elements");
    check(&long_list("", 4096)).expect("v0 accepts 4,096 elements");
}

#[test]
fn the_list_cap_applies_wherever_a_literal_appears_in_v1() {
    let items = vec!["1u8"; 4097].join(", ");
    for src in [
        format!("warrant t v1\n  example e:\n    to_bytes([{items}]) == b\"\"\n"),
        format!("warrant t v1\n  model m(a: U8) -> Bytes = to_bytes([{items}])\n"),
        format!("warrant t v1\n  example e:\n    len([[{items}]]) == 1\n"),
    ] {
        assert_eq!(rejected(&src).code, DiagnosticCode::W0602);
    }
}

// ---------------------------------------------------------------------------
// The mechanical scan (section 1.2)
// ---------------------------------------------------------------------------

/// Every `*.warrant` file under `dir` (non-recursive).
fn warrant_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "warrant"))
        .collect();
    out.sort();
    out
}

#[test]
fn no_in_repository_v1_document_uses_one_of_the_twelve_names() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = warrant_files(&crate_dir.join("tests/fixtures"));
    files.extend(warrant_files(&crate_dir.join("../../../specs")));
    let mut v1_docs = 0;
    for path in &files {
        let text = std::fs::read_to_string(path).expect("read");
        let doc = SourceDocument::admit(text.as_bytes(), SourceLimits::default()).expect("admit");
        let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout");
        let file = parse_document(&tokens, &doc, ParserLimits::default()).expect("parse");
        if file.version != Version::V1 {
            continue;
        }
        v1_docs += 1;
        // Identifiers are `Word` tokens; comments never reach the token stream,
        // so prose such as "(count, byte) pairs" is not a use.
        for t in &tokens {
            if let LayoutTokenKind::Word(w) = t.kind {
                assert!(
                    !all_twelve().any(|n| n == w),
                    "{} uses the WX-D name `{w}`",
                    path.display()
                );
            }
        }
        // No wave-3 v1 document holds a list literal either (VER04k section
        // 2.15): `[` appears in no other construct.
        assert!(
            !tokens
                .iter()
                .any(|t| t.kind == LayoutTokenKind::Symbol("[")),
            "{} holds a list literal",
            path.display()
        );
        // And each still checks under the enlarged reservation.
        check_warrant(&file).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
    }
    // wx_fixed, wxa, wxh and wxv (section 1.2 names four).
    assert!(v1_docs >= 4, "only {v1_docs} v1 documents were scanned");
}

// ---------------------------------------------------------------------------
// Operation discovery into list literals: v1 only (S0 E13)
// ---------------------------------------------------------------------------

#[test]
fn operation_discovery_enters_list_literals_in_v1_and_v0_is_unchanged() {
    use coding_adventures_warrant_grammar::codegen::{
        find_operation_for_version, find_operation_in_expr,
    };
    use coding_adventures_warrant_grammar::parser::Declaration;
    use std::collections::HashSet;
    let find = |header: &str, body: &str| -> (Option<String>, Option<String>) {
        let src = format!("warrant t{header}\n  example e:\n    {body}\n");
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        let Declaration::Example(e) = &file.declarations[0] else {
            panic!()
        };
        let ops: HashSet<&str> = ["f", "g"].into_iter().collect();
        (
            find_operation_for_version(&e.body, &ops, file.version).map(String::from),
            find_operation_in_expr(&e.body, &ops).map(String::from),
        )
    };
    let some = |s: &str| Some(s.to_string());
    // v1: the elements are searched like call arguments ...
    assert_eq!(find(" v1", "to_bytes([f(a)]) == b").0, some("f"));
    assert_eq!(find(" v1", "[f(a)] == [b]").0, some("f"));
    assert_eq!(find(" v1", "[popcount(g(1))] == [1]").0, some("g"));
    // ... after the unchanged shallow search, first match wins.
    assert_eq!(find(" v1", "[f(1)] == g(1)").0, some("g"));
    assert_eq!(find(" v1", "[f(1)] == popcount(g(1))").0, some("f"));
    assert_eq!(find(" v1", "[1] == [popcount(1)]").0, None);
    // v0 is unchanged, and so is the version-blind search the evidence
    // generator still uses (S2a switches it).
    assert_eq!(find("", "[f(a)] == [b]").0, None);
    assert_eq!(find("", "f(1) == g(1)").0, some("f"));
    assert_eq!(find(" v1", "[f(a)] == [b]").1, None);
}

// ---------------------------------------------------------------------------
// Generation still refuses the new forms (fail closed until slice S2a)
// ---------------------------------------------------------------------------

mod generation {
    use std::collections::BTreeMap;

    use coding_adventures_warrant_grammar::binding::{BindingManifest, OpBinding};
    use coding_adventures_warrant_grammar::codegen::DiagnosticCode as CgCode;
    use coding_adventures_warrant_grammar::evidence::{generate_evidence_harness, EvidenceOptions};
    use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
    use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
    use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

    fn generate(src: &str) -> Result<String, (CgCode, String)> {
        let mut b = BTreeMap::new();
        b.insert(
            "f".to_string(),
            OpBinding {
                export: "f".into(),
                static_params: Vec::new(),
                fresh: false,
                error_normalization: None,
                negative_controls: vec!["narrow-width".into()],
            },
        );
        let manifest = BindingManifest {
            package: "subject".into(),
            revision: "0".into(),
            bindings: b,
        };
        let options = EvidenceOptions {
            package_crate: "subject".into(),
            spec_path: "code/specs/t.warrant".into(),
            spec_sha256: "0".repeat(64),
            seed: 1,
            property_cases: 10,
            fuzz_iterations: 10,
            negctl_fuzz_iterations: 10,
            exhaustive_budget: 400_000,
            arg_types: BTreeMap::new(),
            hosts: Default::default(),
        };
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
        let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
        let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
        super::check_warrant(&file).unwrap_or_else(|e| panic!("checker: {e:?}\n{src}"));
        generate_evidence_harness(&file, &manifest, &options).map_err(|e| (e.code, e.to_string()))
    }

    /// A checked v1 document: `op f(a: U8) -> U8`, one claim, and the form
    /// under test in the claim body (behind a deciding `true and`).
    fn doc(params: &str, body: &str) -> String {
        format!(
            "warrant t v1\n  op f(a: U8) -> U8\n  labels f:\n    a public\n  model g({params}) -> Bool = {body}\n  claim c:\n    forall a: U8. f(a) == a and g(a)\n"
        )
    }

    #[test]
    fn every_d1_form_is_still_w0701_at_generation() {
        // The control generates: the refusals below are the forms' own.
        generate(&doc("a: U8", "true")).expect("control document generates");
        for (name, body) in [
            ("take", "len(take(be_bytes(a), 1)) == 1"),
            ("drop", "len(drop(be_bytes(a), 1)) == 0"),
            ("slice", "len(slice(be_bytes(a), 0, 1)) == 1"),
            ("replicate", "len(replicate(a, 2)) == 2"),
            ("range", "len(range(3)) == 3"),
            ("to_list", "len(to_list(be_bytes(a))) == 1"),
        ] {
            let (code, text) = generate(&doc("a: U8", body)).expect_err(body);
            assert_eq!(code, CgCode::W0701, "{body}: {text}");
            assert!(text.contains(name), "{body}: {text}");
        }
        // `to_bytes` needs a List<U8>, and every List<U8> source (a literal or
        // a D1 form) is itself refused first: still W0701.
        let (code, text) = generate(&doc("a: U8", "len(to_bytes([a])) == 1")).unwrap_err();
        assert_eq!(code, CgCode::W0701, "{text}");
    }
}
