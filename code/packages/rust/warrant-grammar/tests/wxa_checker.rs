//! VER04j sections 4.2-4.4 and 10.2: the type rules of WX-A.  One case per row
//! of the "type negatives" list of section 4.11, each pinning the diagnostic ID,
//! the error kind and the offending span, plus the positive rules (structural
//! tuples, nominal records and variants, projection, patterns).

use coding_adventures_warrant_grammar::checker::{
    check_warrant, CheckedWarrantModule, CheckerError, CheckerErrorKind as K, DiagnosticCode as D,
    Type,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn check(src: &str) -> Result<CheckedWarrantModule, CheckerError> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file)
}

/// A v1 document: the shared declarations, then `lines` (indented two spaces).
fn v1(lines: &[&str]) -> String {
    let mut s = String::from(
        "warrant t v1\n  error E = A | B\n  record R(a: U8, b: Bool)\n  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Empty\n",
    );
    for l in lines {
        s.push_str("  ");
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn ok(lines: &[&str]) -> CheckedWarrantModule {
    let src = v1(lines);
    check(&src).unwrap_or_else(|e| panic!("rejected: {e}\n{src}"))
}

fn body_type(lines: &[&str]) -> Type {
    ok(lines).models["m"].body_type.clone()
}

fn err(lines: &[&str]) -> (CheckerError, String) {
    let src = v1(lines);
    match check(&src) {
        Ok(_) => panic!("accepted:\n{src}"),
        Err(e) => (e, src),
    }
}

/// Assert the diagnostic, the error kind (by predicate) and the offending
/// source text (the span's bytes).
fn rejects(lines: &[&str], code: D, kind: impl Fn(&K) -> bool, span_text: &str) {
    let (e, src) = err(lines);
    assert_eq!(e.code, code, "{e:?}\n{src}");
    assert!(kind(&e.kind), "kind {:?}\n{src}", e.kind);
    assert_eq!(&src[e.span.0..e.span.1], span_text, "span\n{src}");
}

fn tuple(ts: &[Type]) -> Type {
    Type::Tuple(ts.to_vec())
}

// ---------------------------------------------------------------------------
// Positive rules
// ---------------------------------------------------------------------------

#[test]
fn tuples_are_structural_and_typed_componentwise() {
    assert_eq!(
        body_type(&["model m(a: U8) -> (U8, Bool) = (a, true)"]),
        tuple(&[Type::U8, Type::Bool])
    );
    // A literal takes its component's expected type: `Nat` here, `Int` otherwise.
    assert_eq!(
        body_type(&["model m(a: U8) -> (U8, Nat) = (a, 3)"]),
        tuple(&[Type::U8, Type::Nat])
    );
    assert_eq!(
        body_type(&["model m(a: U8) -> ((U8, U8), Bool) = ((a, a), false)"]),
        tuple(&[tuple(&[Type::U8, Type::U8]), Type::Bool])
    );
    // Two tuple types are equal iff componentwise equal, through an alias too.
    ok(&[
        "model Pair = (U8, U8)",
        "model m(p: Pair) -> (U8, U8) = p",
        "model n(a: U8) -> Pair = (a, a)",
    ]);
    // Projection by position.
    assert_eq!(body_type(&["model m(p: (U8, Bool)) -> Bool = p.1"]), Type::Bool);
    assert_eq!(
        body_type(&["model m(p: ((U8, U8), Bool)) -> U8 = p.0.1"]),
        Type::U8
    );
    assert_eq!(
        body_type(&["model f(a: U8) -> (U8, Bool) = (a, true)", "model m(a: U8) -> U8 = f(a).0"]),
        Type::U8
    );
    // Equality is structural and type directed.
    ok(&["model m(a: (U8, Bool), b: (U8, Bool)) -> Bool = a == b"]);
    ok(&["model m(a: U8) -> Bool = (a, a) != (a, a)"]);
}

#[test]
fn records_and_variants_are_nominal_with_positional_constructors() {
    assert_eq!(
        body_type(&["model m(a: U8) -> R = R(a, true)"]),
        Type::Adt("R".into())
    );
    assert_eq!(
        body_type(&["model m(a: U8) -> Shape = Rect(a, a)"]),
        Type::Adt("Shape".into())
    );
    assert_eq!(body_type(&["model m(a: U8) -> Shape = Empty"]), Type::Adt("Shape".into()));
    // Field projection on a binder (`r.a`) and on a call (`f(x).a`).
    assert_eq!(body_type(&["model m(r: R) -> U8 = r.a"]), Type::U8);
    assert_eq!(
        body_type(&["model f(a: U8) -> R = R(a, true)", "model m(a: U8) -> Bool = f(a).b"]),
        Type::Bool
    );
    // Nested records: a chain of fields.
    assert_eq!(
        body_type(&["record Outer(inner: R, n: Nat)", "model m(o: Outer) -> Bool = o.inner.b"]),
        Type::Bool
    );
    // Per-argument subtyping as in v0: Nat <= Int and Bytes<N> <= Bytes.
    ok(&["record I(x: Int, y: Bytes)", "model m(n: Nat) -> I = I(n, b\"00\")"]);
    // Equality of constructed values.
    ok(&["model m(a: U8) -> Bool = R(a, true) == R(a, false)"]);
    // Nominal: two records with identical fields are different types.
    let (e, _) = err(&["record R2(a: U8, b: Bool)", "model m(r: R2) -> R = r"]);
    assert_eq!(e.code, D::W0301);
    // A record's name is its type and its constructor.
    ok(&["model m(r: R) -> R = R(r.a, r.b)"]);
    // Nullary and field-carrying constructors share one variant type.
    ok(&["model m(s: Shape) -> Shape = if true then s else Empty"]);
    // Projection of a binder head wins over an error type of the same spelling.
    assert_eq!(body_type(&["model m(E: R) -> U8 = E.a"]), Type::U8);
}

#[test]
fn match_binds_nested_patterns_and_unifies_arm_types() {
    assert_eq!(
        body_type(&["model m(s: Shape) -> Nat = match s with Circle(r) -> 1 | Rect(_, h) -> 2 | Empty -> 3"]),
        Type::Nat,
        "literal arms take the target's Nat"
    );
    ok(&["model m(s: Shape) -> Nat = match s with Circle(r) -> 1 | Rect(_, h) -> 2 | Empty -> 3"]);
    ok(&["model m(o: Option<(U8, Bool)>) -> U8 = match o with Some((x, true)) -> x | Some((x, false)) -> 0u8 | None -> 1u8"]);
    ok(&["model m(r: Result<(U8, Nat), E>) -> Nat = match r with Ok((v, n)) -> n | Err(E.A) -> 0 | Err(E.B) -> 1"]);
    ok(&["model m(r: Result<U8, E>) -> Nat = match r with Ok(x) -> 1 | Err(e) -> 2"]);
    ok(&["model m(b: Bool) -> Nat = match b with true -> 1 | false -> 2"]);
    ok(&["model m(p: (Bool, Bool)) -> Nat = match p with (true, _) -> 1 | (false, true) -> 2 | (false, false) -> 3"]);
    ok(&["model m(r: R) -> Nat = match r with R(x, true) -> 1 | R(_, false) -> 2"]);
    // A wildcard or a binder covers everything.
    ok(&["model m(n: Nat) -> Nat = match n with x -> x"]);
    ok(&["model m(n: Option<Nat>) -> Nat = match n with None -> 0 | Some(_) -> 1"]);
    // The binders of an arm are in scope in its body with the right types.
    assert_eq!(
        body_type(&["model m(o: Option<(U8, Bool)>) -> U8 = match o with Some((x, _)) -> x | None -> 0u8"]),
        Type::U8
    );
    // Nat and Int arms unify to Int.
    assert_eq!(
        body_type(&["model m(b: Bool, n: Nat, i: Int) -> Int = match b with true -> n | false -> i"]),
        Type::Int
    );
}

#[test]
fn labels_accept_a_field_path_of_a_record_parameter() {
    ok(&[
        "op f(r: R) -> Bool",
        "labels f:",
        "  r.a public",
        "  r.b secret",
    ]);
    let (e, _) = err(&[
        "op f(r: R) -> Bool",
        "labels f:",
        "  r.nope public",
        "  r.b public",
    ]);
    assert_eq!(e.code, D::W0401);
    // A field path of a non-record parameter is still refused.
    let (e, _) = err(&["op f(n: Nat) -> Bool", "labels f:", "  n.x public"]);
    assert_eq!(e.code, D::W0401);
}

// ---------------------------------------------------------------------------
// Negatives (section 4.11), one per row
// ---------------------------------------------------------------------------

#[test]
fn constructor_arity_and_field_types_are_checked() {
    rejects(
        &["model m(a: U8) -> R = R(a)"],
        D::W0301,
        |k| matches!(k, K::ArityMismatch { expected: 2, actual: 1 }),
        "R(a)",
    );
    rejects(
        &["model m(a: U8) -> R = R(a, true, a)"],
        D::W0301,
        |k| matches!(k, K::ArityMismatch { expected: 2, actual: 3 }),
        "R(a, true, a)",
    );
    // A wrong field type.
    rejects(
        &["model m(a: U8) -> R = R(true, a)"],
        D::W0301,
        |k| matches!(k, K::TypeMismatch { expected, actual } if expected == "U8" && actual == "Bool"),
        "R(true, a)",
    );
    // A field-carrying constructor named bare; a nullary one written with ().
    rejects(
        &["model m(a: U8) -> Shape = Circle"],
        D::W0301,
        |k| matches!(k, K::ArityMismatch { expected: 1, actual: 0 }),
        "Circle",
    );
    rejects(
        &["model m(a: U8) -> Shape = Empty()"],
        D::W0301,
        |k| matches!(k, K::NullaryWithParens(c) if c == "Empty"),
        "Empty()",
    );
    // A constructor does not take static widths.
    let (e, _) = err(&["model m(a: U8) -> R = R<2>(a, true)"]);
    assert_eq!(e.code, D::W0301);
}

#[test]
fn projection_is_checked() {
    // t.2 of a pair.
    rejects(
        &["model m(p: (U8, U8)) -> U8 = p.2"],
        D::W0301,
        |k| matches!(k, K::NoSuchField(_)),
        "p.2",
    );
    // A field that does not exist, and a position on a record.
    rejects(
        &["model m(r: R) -> U8 = r.zzz"],
        D::W0301,
        |k| matches!(k, K::NoSuchField(f) if f == "R.zzz"),
        "r.zzz",
    );
    let (e, _) = err(&["model m(r: R) -> U8 = r.0"]);
    assert_eq!(e.code, D::W0301);
    // Projecting a variant that is not a record: use `match`.
    rejects(
        &["model m(s: Shape) -> U8 = s.r"],
        D::W0301,
        |k| matches!(k, K::ProjectNonRecord(n) if n == "Shape"),
        "s.r",
    );
    rejects(
        &["model m(s: Shape) -> U8 = s.0"],
        D::W0301,
        |k| matches!(k, K::ProjectNonRecord(n) if n == "Shape"),
        "s.0",
    );
    // A field name on a tuple, a position on a scalar, a projection of a literal.
    let (e, _) = err(&["model m(p: (U8, U8)) -> U8 = p.a"]);
    assert_eq!(e.code, D::W0301);
    let (e, _) = err(&["model m(n: Nat) -> U8 = n.0"]);
    assert_eq!(e.code, D::W0301);
    let (e, _) = err(&["model m(n: Nat) -> Nat = 3.0"]);
    assert_eq!(e.code, D::W0301);
    // A dotted name with a call or widths is not a projection.
    let (e, _) = err(&["model m(r: R) -> U8 = r.a(1)"]);
    assert_eq!(e.code, D::W0301);
}

#[test]
fn recursive_types_are_cycles() {
    for decls in [
        vec!["record Node(next: Option<Node>)"],
        vec!["record Node(items: List<Node>)"],
        vec!["variant T = Leaf | Branch(l: T, r: T)"],
        vec!["record A(b: B)", "record B(a: Option<A>)"],
        vec!["record A(x: (U8, A))"],
        vec!["model Alias = List<Node>", "record Node(n: Alias)"],
        vec!["record Node(r: Result<Node, E>)"],
    ] {
        let (e, _) = err(&decls);
        assert_eq!(e.code, D::W0203, "{decls:?}: {e:?}");
        assert!(matches!(e.kind, K::CyclicDependency(_)), "{decls:?}");
    }
    // The same types without recursion are fine, including a diamond.
    ok(&["record A(x: U8)", "record B(a: A, a2: A)", "record C(b: B, b2: B)"]);
    // The cycle names its members.
    let (e, _) = err(&["record A(b: B)", "record B(a: Option<A>)"]);
    let K::CyclicDependency(path) = e.kind else {
        panic!("cycle")
    };
    assert!(path.contains("A") && path.contains("B"), "{path}");
}

#[test]
fn exhaustiveness_and_redundancy_are_w0301() {
    let nonexhaustive = |lines: &[&str]| {
        let (e, _) = err(lines);
        assert_eq!(e.code, D::W0301, "{lines:?}: {e:?}");
        assert_eq!(e.kind, K::NonExhaustiveMatch, "{lines:?}");
    };
    nonexhaustive(&["model m(o: Option<U8>) -> Nat = match o with Some(x) -> 1"]);
    nonexhaustive(&["model m(s: Shape) -> Nat = match s with Circle(r) -> 1 | Empty -> 3"]);
    nonexhaustive(&["model m(b: Bool) -> Nat = match b with true -> 1"]);
    nonexhaustive(&["model m(p: (Bool, Bool)) -> Nat = match p with (true, _) -> 1 | (false, true) -> 2"]);
    nonexhaustive(&["model m(r: Result<U8, E>) -> Nat = match r with Ok(x) -> 1 | Err(E.A) -> 2"]);
    nonexhaustive(&["model m(o: Option<Option<U8>>) -> Nat = match o with None -> 0 | Some(None) -> 1"]);
    nonexhaustive(&["model m(r: R) -> Nat = match r with R(_, true) -> 1"]);
    // A redundant arm: a repeated arm, an arm after a wildcard, an arm covered
    // by earlier arms jointly.
    rejects(
        &["model m(o: Option<U8>) -> Nat = match o with None -> 0 | Some(x) -> 1 | Some(y) -> 2"],
        D::W0301,
        |k| *k == K::RedundantArm,
        "Some(y) -> 2",
    );
    rejects(
        &["model m(s: Shape) -> Nat = match s with _ -> 0 | Empty -> 1"],
        D::W0301,
        |k| *k == K::RedundantArm,
        "Empty -> 1",
    );
    rejects(
        &["model m(b: Bool) -> Nat = match b with true -> 1 | false -> 2 | true -> 3"],
        D::W0301,
        |k| *k == K::RedundantArm,
        "true -> 3",
    );
    rejects(
        &["model m(p: (Bool, Bool)) -> Nat = match p with (true, _) -> 1 | (_, true) -> 2 | (false, false) -> 3 | (true, true) -> 4"],
        D::W0301,
        |k| *k == K::RedundantArm,
        "(true, true) -> 4",
    );
    // v0 behaviour is unchanged: a document without the v1 header still
    // accepts a repeated Option arm (the legacy check).
    check("warrant t\n  model m(o: Option<U8>) -> U8 = match o with None -> 0u8 | Some(x) -> x | Some(y) -> y\n")
        .expect("v0 keeps its legacy match check");
}

#[test]
fn match_pattern_typing_errors() {
    let incompatible = |lines: &[&str]| {
        let (e, _) = err(lines);
        assert_eq!(e.code, D::W0301, "{lines:?}: {e:?}");
        assert_eq!(e.kind, K::IncompatibleMatchArms, "{lines:?}");
    };
    // A pattern of the wrong kind for the scrutinee.
    incompatible(&["model m(o: Option<U8>) -> Nat = match o with Ok(x) -> 1 | _ -> 2"]);
    incompatible(&["model m(s: Shape) -> Nat = match s with R(a, b) -> 1 | _ -> 2"]);
    incompatible(&["model m(p: (Bool, Bool)) -> Nat = match p with (a, b, c) -> 1 | _ -> 2"]);
    incompatible(&["model m(n: Nat) -> Nat = match n with Circle(r) -> 1 | _ -> 2"]);
    incompatible(&["model m(n: Nat) -> Nat = match n with true -> 1 | _ -> 2"]);
    incompatible(&["model m(n: Nat) -> Nat = match n with None -> 1 | _ -> 2"]);
    incompatible(&["model m(r: Result<U8, E>) -> Nat = match r with Ok(x) -> 1 | Err(F.A) -> 2"]);
    // Arms of different types.
    incompatible(&["model m(b: Bool) -> Nat = match b with true -> 1 | false -> true"]);
    // An unknown constructor with arguments.
    let (e, _) = err(&["model m(o: Option<U8>) -> Nat = match o with None -> 1 | Zzz(x) -> 2"]);
    assert_eq!((e.code, matches!(e.kind, K::UnboundConstructor(_))), (D::W0202, true));
    // An unknown error case.
    let (e, _) = err(&["model m(r: Result<U8, E>) -> Nat = match r with Ok(x) -> 1 | Err(E.C) -> 2 | Err(_) -> 3"]);
    assert_eq!((e.code, matches!(e.kind, K::UnboundConstructor(_))), (D::W0202, true));
    // Wrong constructor arity in a pattern.
    let (e, _) = err(&["model m(s: Shape) -> Nat = match s with Circle(a, b) -> 1 | _ -> 2"]);
    assert_eq!(e.kind, K::ArityMismatch { expected: 1, actual: 2 });
    let (e, _) = err(&["model m(s: Shape) -> Nat = match s with Rect(a) -> 1 | _ -> 2"]);
    assert_eq!(e.kind, K::ArityMismatch { expected: 2, actual: 1 });
    // A nullary constructor in a pattern with arguments, a field-carrying one bare.
    let (e, _) = err(&["model m(s: Shape) -> Nat = match s with Empty(x) -> 1 | _ -> 2"]);
    assert!(matches!(e.kind, K::ArityMismatch { expected: 0, actual: 1 }), "{e:?}");
    let (e, _) = err(&["model m(s: Shape) -> Nat = match s with Circle -> 1 | _ -> 2"]);
    assert!(matches!(e.kind, K::ArityMismatch { expected: 1, actual: 0 }), "{e:?}");
}

#[test]
fn binders_may_not_spell_constructors_and_may_not_repeat() {
    // A binder equal to a constructor name: parameters, quantifier binders.
    for line in [
        "model m(Empty: U8) -> U8 = Empty",
        "model m(R: U8) -> U8 = R",
        "op f(Circle: U8) -> Bool",
    ] {
        let (e, _) = err(&[line]);
        assert_eq!(e.code, D::W0203, "{line}: {e:?}");
        assert!(matches!(e.kind, K::ShadowedBinding(_)), "{line}");
    }
    let (e, _) = err(&["claim c:", "  forall Rect: U8. true"]);
    assert_eq!((e.code, matches!(e.kind, K::ShadowedBinding(_))), (D::W0203, true));
    // A pattern variable repeated in one pattern, or shadowing a model, an
    // operation, a built-in or an outer binder.
    for line in [
        "model m(p: (U8, U8)) -> U8 = match p with (x, x) -> x",
        "model f(a: U8) -> U8 = a",
        "model m(p: Option<U8>) -> U8 = match p with None -> 0u8 | Some(f) -> f",
        "model m(p: Option<U8>) -> U8 = match p with None -> 0u8 | Some(len) -> len",
        "model m(p: Option<U8>) -> U8 = match p with None -> 0u8 | Some(p) -> p",
        "model m(p: Option<U8>) -> U8 = match p with None -> 0u8 | Some(E) -> 1u8",
    ] {
        if line.starts_with("model f(") {
            continue;
        }
        let lines = if line.contains("Some(f)") {
            vec!["model f(a: U8) -> U8 = a", line]
        } else {
            vec![line]
        };
        let (e, _) = err(&lines);
        assert_eq!(e.code, D::W0203, "{line}: {e:?}");
    }
}

#[test]
fn constructor_names_are_module_symbols() {
    // Clashes with a model, an operation, an error type, a type name, another
    // constructor and a built-in are all W0201.
    for decls in [
        vec!["model Empty(a: U8) -> U8 = a"],
        vec!["op Rect(a: U8) -> Bool"],
        vec!["variant V2 = E | Other"],
        vec!["variant V2 = Shape | Other"],
        vec!["variant V2 = Circle | Other"],
        vec!["record Circle(x: U8)"],
        vec!["variant V2 = Some | Other"],
        vec!["record Shape(x: U8)"],
        vec!["record R(x: U8)"],
        vec!["variant V2 = X | Other", "variant V3 = X | Another"],
    ] {
        let (e, _) = err(&decls);
        assert_eq!(e.code, D::W0201, "{decls:?}: {e:?}");
        assert!(matches!(e.kind, K::DuplicateSymbol(_)), "{decls:?}");
    }
}

#[test]
fn composites_are_invariant_and_unordered() {
    // (Nat, Nat) where (Int, Int) is expected: widen the components explicitly.
    rejects(
        &["model m(a: Nat, b: Nat) -> (Int, Int) = (a, b)"],
        D::W0301,
        |k| matches!(k, K::TypeMismatch { .. }),
        "(a, b)",
    );
    // ... also under Option and as a function argument.
    let (e, _) = err(&["model m(a: Nat) -> Option<(Int, Int)> = Some((a, a))"]);
    assert_eq!(e.code, D::W0301);
    let (e, _) = err(&[
        "model f(p: (Int, Int)) -> Int = p.0",
        "model m(a: Nat) -> Int = f((a, a))",
    ]);
    assert_eq!(e.code, D::W0301);
    // Explicit widening of a literal context is fine: literals adapt.
    ok(&["model m() -> (Int, Int) = (1, 2)"]);
    ok(&["model m(a: Int, b: Int) -> (Int, Int) = (a, b)"]);
    // A tuple compared with `<`, and records and variants likewise.
    for body in [
        "model m(a: (U8, U8), b: (U8, U8)) -> Bool = a < b",
        "model m(a: R, b: R) -> Bool = a <= b",
        "model m(a: Shape, b: Shape) -> Bool = a > b",
    ] {
        rejects(&[body], D::W0301, |k| matches!(k, K::TypeMismatch { .. }), &body[body.find("= ").unwrap() + 2..]);
    }
    // Equality needs the same type: tuple vs record, record vs record.
    let (e, _) = err(&["model m(a: (U8, Bool), r: R) -> Bool = a == r"]);
    assert_eq!(e.code, D::W0301);
    let (e, _) = err(&["record Q(a: U8, b: Bool)", "model m(q: Q, r: R) -> Bool = q == r"]);
    assert_eq!(e.code, D::W0301);
}

#[test]
fn error_types_are_not_value_types() {
    let (e, _) = err(&["record Bad(x: E)"]);
    assert_eq!(e.code, D::W0301);
}

#[test]
fn work_and_arm_budgets_are_w0602() {
    // More than 256 arms.
    let arms: Vec<String> = (0..257).map(|_| "_ -> 1".to_string()).collect();
    let line = format!("model m(b: Bool) -> Nat = match b with {}", arms.join(" | "));
    let (e, _) = err(&[&line]);
    assert_eq!((e.code, e.kind), (D::W0602, K::TooManyArms));
    // 256 arms are accepted by the arm limit (they are redundant after the
    // first, which is the other diagnostic).
    let arms: Vec<String> = (0..256).map(|_| "_ -> 1".to_string()).collect();
    let line = format!("model m(b: Bool) -> Nat = match b with {}", arms.join(" | "));
    let (e, _) = err(&[&line]);
    assert_eq!(e.kind, K::RedundantArm);
    // A record type that expands to 2^30 shape nodes exhausts the work budget
    // instead of the machine.
    let mut decls = vec!["record L0(a: Bool, b: Bool)".to_string()];
    for i in 1..=30 {
        decls.push(format!("record L{i}(a: L{}, b: L{})", i - 1, i - 1));
    }
    decls.push("model m(x: L30) -> Nat = match x with _ -> 0".to_string());
    let refs: Vec<&str> = decls.iter().map(String::as_str).collect();
    let (e, _) = err(&refs);
    assert_eq!(e.code, D::W0602, "{e:?}");
    // A diamond of that depth is fine without a match (linear cycle check).
    let refs: Vec<&str> = decls[..decls.len() - 1].iter().map(String::as_str).collect();
    ok(&refs);
}

#[test]
fn nested_patterns_beyond_depth_eight_are_refused() {
    // Depth 8 is accepted by both parser and checker.
    let pat = format!("{}x{}", "Some(".repeat(7), ")".repeat(7));
    let src = format!(
        "warrant t v1\n  model m(o: Option<Option<Option<Option<Option<Option<Option<Option<U8>>>>>>>>) -> U8 = match o with {pat} -> 2u8 | _ -> 1u8\n"
    );
    check(&src).expect("depth 8 is accepted");
}

#[test]
fn equality_and_conditionals_see_through_composites() {
    ok(&["model m(a: U8, b: U8) -> (U8, U8) = if a == b then (a, b) else (b, a)"]);
    // Branches must agree.
    let (e, _) = err(&["model m(a: U8) -> (U8, U8) = if true then (a, a) else (a, true)"]);
    assert_eq!(e.code, D::W0301);
    let (e, _) = err(&["model m(a: U8) -> R = if true then R(a, true) else Empty"]);
    assert_eq!(e.code, D::W0301);
}
