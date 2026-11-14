//! VER04j WX-A edge cases: the refusals of every layer that the main WX-A test
//! files do not reach -- checker pattern errors, projection chains in the ground
//! evaluator, contracts that mention record fields, binding-file limits, and the
//! generator's diagnostics for documents the checker would have refused.

use std::collections::{HashMap, HashSet};

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckerErrorKind as K, DiagnosticCode as D,
};
use coding_adventures_warrant_grammar::codegen::{
    find_operation_in_expr, CodegenLimits, EvalValue, Evaluator,
};
use coding_adventures_warrant_grammar::evidence::{generate_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, Pattern,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn with_file<R>(src: &str, f: impl FnOnce(&coding_adventures_warrant_grammar::parser::WarrantFile) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

fn check_code(src: &str) -> (D, K) {
    with_file(src, |f| {
        let e = check_warrant(f).expect_err(src);
        (e.code, e.kind)
    })
}

#[test]
fn checker_pattern_and_declaration_refusals() {
    // A duplicate variant name across declarations.
    let (c, k) = check_code("warrant t v1\n  variant V = A | B\n  variant V = C | D\n");
    assert_eq!((c, matches!(k, K::DuplicateSymbol(_))), (D::W0201, true));
    // `Err` against a non-Result, `Ok` against an Option, `Some` against a Result.
    let m = |pat: &str, ty: &str| {
        format!("warrant t v1\n  error E = A | B\n  model m(x: {ty}) -> Nat = match x with {pat} -> 1 | _ -> 2\n")
    };
    for (pat, ty) in [("Err(_)", "Option<U8>"), ("Some(_)", "Result<U8, E>"), ("Ok(_)", "Bool")] {
        assert_eq!(check_code(&m(pat, ty)), (D::W0301, K::IncompatibleMatchArms), "{pat}");
    }
    // `E.CASE` against a non-error scrutinee; a qualified pattern of three segments.
    assert_eq!(check_code(&m("E.A", "U8")), (D::W0301, K::IncompatibleMatchArms));
    let (c, k) = check_code(&m("a.b.c", "U8"));
    assert_eq!((c, matches!(k, K::UnboundConstructor(_))), (D::W0202, true));
    // Matching an error value (a binder from `Err(e)`) by its cases is exhaustive.
    let ok = "warrant t v1\n  error E = A | B\n  model m(r: Result<U8, E>) -> Nat = match r with Ok(x) -> 0 | Err(e) -> (match e with E.A -> 1 | E.B -> 2)\n";
    with_file(ok, |f| {
        check_warrant(f).expect("nested error match");
    });
    let bad = "warrant t v1\n  error E = A | B\n  model m(r: Result<U8, E>) -> Nat = match r with Ok(x) -> 0 | Err(e) -> (match e with E.A -> 1)\n";
    assert_eq!(check_code(bad), (D::W0301, K::NonExhaustiveMatch));
    // The legacy v0 check still handles `Err(binder)`.
    with_file(
        "warrant t\n  error E = A | B\n  model m(r: Result<U8, E>) -> U8 = match r with Ok(x) -> x | Err(e) -> 0u8\n",
        |f| {
            check_warrant(f).expect("v0 Result match");
        },
    );
}

#[test]
fn contracts_may_mention_record_fields() {
    let src = "warrant t v1
  error E = A | B
  record Limits(max_len: Nat, depth: U8)
  op parse(data: Bytes, limits: Limits) -> Result<Nat, E>
  labels parse:
    data secret
    limits.max_len public
    limits.depth public
  input parse.limits:
    hostile
    rejects limits.max_len > 4096 with E.A
    ensures no_panic
  bound parse:
    steps <= limits.max_len + 10
";
    with_file(src, |f| {
        check_warrant(f).expect("field paths in labels, rejects and bounds");
    });
    // A field that does not exist in a contract expression is W0301.
    let bad = src.replace("limits.max_len > 4096", "limits.nope > 4096");
    assert_eq!(check_code(&bad).0, D::W0301);
}

#[test]
fn the_ground_evaluator_projects_through_a_binder_chain() {
    let src = "warrant t v1\n  record In(a: U8, b: Bool)\n  record Out(inner: In, n: Nat)\n  example e:\n    (match Out(In(3u8, true), 4) with o -> (o.inner.a, o.inner.b, o.n)) == (3u8, true, 4)\n";
    with_file(src, |f| {
        let Declaration::Example(ex) = f
            .declarations
            .iter()
            .find(|d| matches!(d, Declaration::Example(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let mut ev = Evaluator::new(f, CodegenLimits::default());
        assert_eq!(ev.eval(&ex.body, &HashMap::new()).unwrap(), EvalValue::Bool(true));
        // `None` patterns and an Option scrutinee.
    });
    let src = "warrant t v1\n  example e:\n    (match None with None -> 1 | Some(x) -> 2) == 1\n";
    with_file(src, |f| {
        let Declaration::Example(ex) = &f.declarations[0] else {
            unreachable!()
        };
        let mut ev = Evaluator::new(f, CodegenLimits::default());
        assert_eq!(ev.eval(&ex.body, &HashMap::new()).unwrap(), EvalValue::Bool(true));
    });
}

#[test]
fn operations_are_found_inside_tuples_and_projections() {
    let src = "warrant t v1\n  op f(a: U8) -> (U8, U8)\n  example e1:\n    (f(1u8), 2) == (f(1u8), 2)\n  example e2:\n    f(1u8).0 == 1u8\n  example e3:\n    (1, (2, f(3u8))) == (1, (2, f(3u8)))\n";
    with_file(src, |f| {
        let names: HashSet<&str> = HashSet::from(["f"]);
        for d in &f.declarations {
            if let Declaration::Example(ex) = d {
                let Expression::Binary { left, .. } = &ex.body else {
                    panic!("eq")
                };
                assert_eq!(find_operation_in_expr(left, &names), Some("f"), "{}", ex.name);
            }
        }
        let empty: HashSet<&str> = HashSet::new();
        let Declaration::Example(ex) = &f.declarations[1] else {
            unreachable!()
        };
        let Expression::Binary { left, .. } = &ex.body else {
            panic!()
        };
        assert_eq!(find_operation_in_expr(left, &empty), None);
    });
}

#[test]
fn as_binder_only_sees_a_bare_name() {
    with_file(
        "warrant t v1\n  example e:\n    (match x with Some((a, b)) -> 1 | Some(c) -> 2 | Some(D(e)) -> 3 | _ -> 4) == 1\n",
        |f| {
            let Declaration::Example(ex) = &f.declarations[0] else {
                unreachable!()
            };
            let Expression::Binary { left, .. } = &ex.body else {
                panic!()
            };
            let Expression::Grouped { inner, .. } = &**left else {
                panic!()
            };
            let Expression::Match { arms, .. } = &**inner else {
                panic!()
            };
            let inner_of = |i: usize| match &arms[i].pattern {
                Pattern::Some(q) => q.as_binder(),
                _ => panic!(),
            };
            assert_eq!(inner_of(0), None);
            assert_eq!(inner_of(1), Some("c"));
            assert_eq!(inner_of(2), None);
            assert_eq!(arms[3].pattern.as_binder(), None);
        },
    );
}

// ---------------------------------------------------------------------------
// Binding-file limits
// ---------------------------------------------------------------------------

fn bindings(lines: &[String]) -> Result<(), String> {
    let mut s = String::from(
        "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/x/tests/generated/t.rs\nseed 1\nproperty_cases 1\nfuzz_iterations 1\nnegctl_fuzz_iterations 1\n",
    );
    for l in lines {
        s.push_str(l);
        s.push('\n');
    }
    parse_evidence_bindings(&s).map(|_| ())
}

#[test]
fn binding_file_limits_and_name_checks() {
    let op = "op f export=f faults=a".to_string();
    // 257 fields and 257 constructors are over the cap; 256 are fine.
    let fields = |n: usize| (0..n).map(|i| format!("f{i}:u8")).collect::<Vec<_>>().join(",");
    assert!(bindings(&[format!("struct T path=a::B fields={}", fields(256)), op.clone()]).is_ok());
    let e = bindings(&[format!("struct T path=a::B fields={}", fields(257)), op.clone()]).unwrap_err();
    assert!(e.contains("too many fields"), "{e}");
    let ctors = |n: usize| (0..n).map(|i| format!("C{i}")).collect::<Vec<_>>().join(",");
    assert!(bindings(&[format!("enum T path=a::B ctors={}", ctors(256)), op.clone()]).is_ok());
    let e = bindings(&[format!("enum T path=a::B ctors={}", ctors(257)), op.clone()]).unwrap_err();
    assert!(e.contains("too many constructors"), "{e}");
    // A type name must be an identifier.
    let e = bindings(&["struct 1x path=a::B fields=a:u8".to_string(), op.clone()]).unwrap_err();
    assert!(e.contains("not an identifier"), "{e}");
    // A declared type nested
    // inside `opt<..>` and `list<..>` must itself be declared.
    assert!(bindings(&[
        "struct U path=a::U fields=z:u8".to_string(),
        "struct W path=a::W fields=x:opt<U>,y:list<U>".to_string(),
        op.clone()
    ])
    .is_ok());
    let e = bindings(&[
        "struct W path=a::W fields=x:opt<Missing>".to_string(),
        op.clone(),
    ])
    .unwrap_err();
    assert!(e.contains("`Missing`"), "{e}");
    let e = bindings(&[
        "struct W path=a::W fields=y:list<(u8,Missing)>".to_string(),
        op,
    ])
    .unwrap_err();
    assert!(e.contains("`Missing`"), "{e}");
}

// ---------------------------------------------------------------------------
// Generator refusals on documents the checker would have refused
// ---------------------------------------------------------------------------

fn generate(src: &str, lines: &[&str]) -> Result<String, String> {
    let mut text = String::from(
        "package subject\nrevision 0\nspec code/specs/t.warrant\noutput code/packages/rust/x/tests/generated/t.rs\nseed 1\nproperty_cases 1\nfuzz_iterations 1\nnegctl_fuzz_iterations 1\n",
    );
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    let b = parse_evidence_bindings(&text).map_err(|e| format!("bindings: {e}"))?;
    with_file(src, |file| {
        let options = EvidenceOptions {
            package_crate: "subject".into(),
            spec_path: "code/specs/t.warrant".into(),
            spec_sha256: "0".repeat(64),
            seed: 1,
            property_cases: 1,
            fuzz_iterations: 1,
            negctl_fuzz_iterations: 1,
            exhaustive_budget: 400_000,
            arg_types: b.arg_types.clone(),
            hosts: b.hosts.clone(),
        };
        generate_evidence_harness(file, &b.manifest, &options)
            .map_err(|e| format!("{} {:?}", e.code, e.kind))
    })
}

const OPF: &[&str] = &["op f export=f faults=swap-components"];

fn claim_doc(extra: &str, body: &str) -> String {
    format!(
        "warrant t v1\n  record R(a: U8)\n  variant V = P | Q(a: U8)\n{extra}  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall a: U8. {body}\n"
    )
}

#[test]
fn the_translator_refuses_what_the_checker_would_have_refused() {
    // Wrong constructor arity, and an unknown constructor in a pattern, an
    // error case on a non-constructor, a pattern with the wrong field count.
    let cases = [
        ("f(a) == f(a) and R(a, a) == R(a, a)", "constructor `R` expects 1 arguments"),
        (
            "(match R(a) with Zzz(x) -> true | _ -> false) and f(a) == f(a)",
            "unknown constructor `Zzz`",
        ),
        (
            "(match Q(a) with Q(x, y) -> true | _ -> false) and f(a) == f(a)",
            "wrong number of fields",
        ),
    ];
    for (body, want) in cases {
        let e = generate(&claim_doc("", body), OPF).unwrap_err();
        assert!(e.contains("W0701") && e.contains(want.split('`').next().unwrap()), "{body}: {e}");
    }
    // A binder of a type the generator cannot enumerate, a symbolic width, a
    // recursive alias, and a v0 document with an Option binder.
    for (src, what) in [
        ("warrant t v1\n  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall z: Zzz. f(0u8) == f(0u8)\n", "unknown type"),
        ("warrant t v1\n  model L = List<L>\n  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall z: L. f(0u8) == f(0u8)\n", "recursive alias"),
    ] {
        let e = generate(src, OPF).unwrap_err();
        assert!(e.contains("W0701"), "{what}: {e}");
    }
    let v0 = "warrant t\n  op g(a: U8) -> U8\n  claim c:\n    forall o: Option<U8>. g(1u8) == g(1u8)\n";
    let e = generate(v0, &["op g export=g faults=or-for-xor"]).unwrap_err();
    assert!(e.contains("W0701"), "{e}");
    let sym = "warrant t v1\n  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall z: Bytes<N>. f(0u8) == f(0u8)\n";
    assert!(generate(sym, OPF).unwrap_err().contains("W0701"));
    // An error type that does not exist.
    let src = "warrant t v1\n  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall r: Result<Bool, Nope>. f(0u8) == f(0u8)\n";
    assert!(generate(src, OPF).unwrap_err().contains("W0701"));
}

#[test]
fn composite_adapter_refusals() {
    // A fixed-width Bytes parameter used to have no composite adapter; since
    // WX-N (VER04j section 14.4) it converts like `Bytes` (a `&[u8]`, the
    // default host type), and `bytes<2>` asks for `&[u8; 2]` (tests/wxn_*).
    let src = "warrant t v1\n  op g(a: Bytes<2>) -> (U8, U8)\n  claim c:\n    forall a: Bytes<2>. g(a) == g(a)\n";
    let text = generate(src, &["op g export=g faults=swap-components"]).unwrap();
    assert!(text.contains("args[0].bytes()"), "{text}");
    // A host array of the wrong width is still refused.
    let e = generate(src, &["op g export=g args=bytes<3> faults=swap-components"]).unwrap_err();
    assert!(e.contains("W0701"), "{e}");
    // A non-host `args=` entry on a composite operation.
    let src = "warrant t v1\n  op g(a: U8) -> (U8, U8)\n  claim c:\n    forall a: U8. g(a) == g(a)\n";
    let e = generate(src, &["op g export=g args=9z9 faults=swap-components"]).unwrap_err();
    assert!(e.contains("W0701"), "{e}");
    // A parameter a default host type cannot be derived for: Result.
    let src = "warrant t v1\n  error E = A\n  op g(r: Result<U8, E>) -> (U8, U8)\n  claim c:\n    forall a: U8. g(Ok(a)) == g(Ok(a))\n";
    let e = generate(src, &["op g export=g faults=swap-components"]).unwrap_err();
    assert!(e.contains("W0701"), "{e}");
    // Bytes and List parameters and results get their default host types
    // (`bytes`, `list<..>`) and convert.
    let src = "warrant t v1\n  op g(d: Bytes, l: List<Bool>) -> (List<Bool>, Nat)\n  claim c:\n    forall d: Bytes, l: List<Bool>. g(d, l) == g(d, l)\n";
    let text = generate(src, &["op g export=g faults=drop-consumed"]).unwrap();
    assert!(text.contains("args[0].bytes()"), "{text}");
    assert!(text.contains(".list().iter().map(|x"), "{text}");
    assert!(text.contains("V::List(("), "{text}");
}

// ---------------------------------------------------------------------------
// Security regressions: exponential type expansion and struct field binding
// ---------------------------------------------------------------------------

fn diamond(levels: usize) -> String {
    let mut s = String::from("  record L0(a: Bool, b: Bool)\n");
    for i in 1..=levels {
        s.push_str(&format!("  record L{i}(a: L{}, b: L{})\n", i - 1, i - 1));
    }
    s
}

#[test]
fn a_diamond_of_records_is_refused_quickly_by_the_generator() {
    let src = format!(
        "warrant t v1\n{}  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall x: L30. f(0u8) == f(0u8)\n",
        diamond(30)
    );
    let start = std::time::Instant::now();
    let e = generate(&src, OPF).unwrap_err();
    assert!(e.contains("W0706"), "{e}");
    assert!(start.elapsed().as_millis() < 1000, "{:?}", start.elapsed());
    // The same diamond as an operation result is refused too.
    let src = format!(
        "warrant t v1\n{}  op f(a: U8) -> L30\n  claim c:\n    forall a: U8. f(a) == f(a)\n",
        diamond(30)
    );
    let start = std::time::Instant::now();
    assert!(generate(&src, &["op f export=f faults=field-zero"]).is_err());
    assert!(start.elapsed().as_millis() < 1000, "{:?}", start.elapsed());
    // A modest diamond (L5 = 64 leaves) is expanded; its domain is over budget.
    let src = format!(
        "warrant t v1\n{}  op f(a: U8) -> (U8, U8)\n  claim c:\n    forall x: L5. f(0u8) == f(0u8)\n",
        diamond(5)
    );
    assert!(generate(&src, OPF).unwrap_err().contains("W0706"));
}

#[test]
fn a_wide_shape_is_refused_by_the_checker_without_deep_recursion() {
    let start = std::time::Instant::now();
    for levels in [17usize, 30] {
        let src = format!(
            "warrant t v1\n{}  model m(x: L{levels}) -> Nat = match x with _ -> 0\n",
            diamond(levels)
        );
        let (c, k) = check_code(&src);
        assert_eq!((c, k), (D::W0602, K::WorkBudgetExceeded), "L{levels}");
    }
    assert!(start.elapsed().as_millis() < 1000, "{:?}", start.elapsed());
    // A diamond that fits the cap is still matched.
    let src = format!(
        "warrant t v1\n{}  model m(x: L5) -> Nat = match x with _ -> 0\n",
        diamond(5)
    );
    with_file(&src, |f| {
        check_warrant(f).expect("small shape");
    });
}

#[test]
fn a_host_struct_must_name_the_records_fields() {
    let src = "warrant t v1\n  record R(a: U8, b: U8)\n  op h(a: U8) -> R\n  claim c:\n    forall a: U8. h(a) == h(a)\n";
    let swapped = generate(
        src,
        &[
            "struct R path=s::R fields=b:u8,a:u8",
            "op h export=h faults=field-zero",
        ],
    )
    .unwrap_err();
    assert!(swapped.contains("W0701"), "{swapped}");
    // The right names, and positional indexes of a tuple struct, are accepted.
    for fields in ["a:u8,b:u8", "0:u8,1:u8"] {
        let line = format!("struct R path=s::R fields={fields}");
        generate(src, &[&line, "op h export=h faults=field-zero"]).unwrap();
    }
}
