//! VER04j sections 4.5, 4.9 and 4.11: what WX-A programs *mean* in the ground
//! evaluator (construction, projection, matching, structural equality) and how
//! values are encoded for the cross-language corpus (tuples, records, variants
//! by constructor name, `Option`, `Result`) with the 64-container depth limit.

use std::collections::HashMap;

use coding_adventures_warrant_grammar::codegen::{
    CodegenLimits, DiagnosticCode, EvalValue, Evaluator, TFV_MAX_DEPTH,
};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const DECLS: &str = "  error E = A | B\n  record R(a: U8, b: Bool)\n  record Outer(inner: R, n: Nat)\n  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Empty\n";

/// Evaluate the left side of `<expr> == <expr>` in a v1 document with `DECLS`
/// and `extra` declarations, under `limits`.
fn eval_with(
    extra: &str,
    expr: &str,
    limits: CodegenLimits,
) -> Result<EvalValue, DiagnosticCode> {
    let src = format!("warrant t v1\n{DECLS}{extra}  example e:\n    ({expr}) == ({expr})\n");
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let ex = file
        .declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Example(e) => Some(e),
            _ => None,
        })
        .unwrap();
    let Expression::Binary {
        op: BinaryOp::Eq,
        left,
        ..
    } = &ex.body
    else {
        panic!("equality")
    };
    let mut ev = Evaluator::new(&file, limits);
    ev.eval(left, &HashMap::new()).map_err(|e| e.code)
}

fn eval(expr: &str) -> EvalValue {
    eval_with("", expr, CodegenLimits::default()).unwrap_or_else(|c| panic!("{expr}: {c}"))
}

fn eval_in(extra: &str, expr: &str) -> EvalValue {
    eval_with(extra, expr, CodegenLimits::default()).unwrap_or_else(|c| panic!("{expr}: {c}"))
}

fn truth(extra: &str, expr: &str) -> bool {
    matches!(eval_in(extra, expr), EvalValue::Bool(true))
}

fn int(v: i128) -> EvalValue {
    EvalValue::Int(v)
}
fn u8v(v: u8) -> EvalValue {
    EvalValue::U8(v)
}

#[test]
fn tuples_construct_project_and_nest() {
    assert_eq!(eval("(1, true)"), EvalValue::Tuple(vec![int(1), EvalValue::Bool(true)]));
    assert_eq!(eval("(1, 2).0"), int(1));
    assert_eq!(eval("(1, 2).1"), int(2));
    assert_eq!(eval("((1, 2), 3).0.1"), int(2));
    assert_eq!(
        eval("(1u8, 2u16)"),
        EvalValue::Tuple(vec![u8v(1), EvalValue::Fixed(FixedTy::U16, 2)])
    );
    // A projection of a call and of a model result.
    assert_eq!(eval_in("  model p(a: U8) -> (U8, U8) = (a, a + 1u8)\n", "p(7u8).1"), u8v(8));
}

#[test]
fn records_and_variants_construct_and_project() {
    assert_eq!(
        eval("R(5u8, true)"),
        EvalValue::Con {
            ty: "R".into(),
            index: 0,
            ctor: "R".into(),
            record: true,
            fields: vec![u8v(5), EvalValue::Bool(true)],
        }
    );
    assert_eq!(eval("R(5u8, true).a"), u8v(5));
    assert_eq!(eval("R(5u8, true).b"), EvalValue::Bool(true));
    assert_eq!(eval("Outer(R(1u8, false), 9).inner.a"), u8v(1));
    assert_eq!(eval("Outer(R(1u8, false), 9).n"), int(9));
    assert_eq!(
        eval("Rect(3u8, 4u8)"),
        EvalValue::Con {
            ty: "Shape".into(),
            index: 1,
            ctor: "Rect".into(),
            record: false,
            fields: vec![u8v(3), u8v(4)],
        }
    );
    assert!(matches!(eval("Empty"), EvalValue::Con { index: 2, ref fields, .. } if fields.is_empty()));
}

#[test]
fn equality_is_structural_and_numeric_by_value() {
    assert!(truth("", "R(1u8, true) == R(1u8, true)"));
    assert!(truth("", "R(1u8, true) != R(1u8, false)"));
    assert!(truth("", "Circle(3u8) != Rect(3u8, 3u8)"));
    assert!(truth("", "Circle(3u8) == Circle(3u8)"));
    assert!(truth("", "Empty == Empty"));
    assert!(truth("", "(1, (2, b\"0a\")) == (1, (2, b\"0a\"))"));
    assert!(truth("", "(1, b\"0a\") != (1, b\"0b\")"));
    // Int and Nat compare by value, also inside composites and Option.
    assert!(truth("  model n(a: Nat) -> Nat = a\n", "(n(2), 1) == (2, 1)"));
    assert!(truth("  model n(a: Nat) -> Nat = a\n", "Some((n(2), 1)) == Some((2, 1))"));
    // A tuple of different length or a different component is unequal.
    assert!(truth("", "(1, 2) != (1, 3)"));
}

#[test]
fn match_binds_nested_patterns_and_takes_the_first_arm() {
    assert_eq!(
        eval("match Rect(3u8, 4u8) with Circle(r) -> 1 | Rect(w, h) -> 2 | Empty -> 3"),
        int(2)
    );
    assert_eq!(
        eval("match Rect(3u8, 4u8) with Circle(r) -> r | Rect(w, h) -> w + h | Empty -> 0u8"),
        u8v(7)
    );
    assert_eq!(
        eval("match Some((1u8, true)) with Some((x, true)) -> x | Some((x, false)) -> 0u8 | None -> 9u8"),
        u8v(1)
    );
    assert_eq!(eval("match true with true -> 1 | false -> 2"), int(1));
    assert_eq!(eval("match (true, false) with (false, _) -> 1 | (_, true) -> 2 | (_, false) -> 3"), int(3));
    // Wildcards bind nothing and may repeat; a variable binds the whole value.
    assert_eq!(eval("match (1, 2) with (_, _) -> 5"), int(5));
    assert_eq!(eval("match (1, 2) with p -> p.1"), int(2));
    // Error cases in patterns.
    assert_eq!(eval("match Err(E.B) with Ok(x) -> 0 | Err(E.A) -> 1 | Err(E.B) -> 2"), int(2));
    assert_eq!(eval("match Err(E.A) with Ok(x) -> 0 | Err(e) -> 7"), int(7));
    assert_eq!(eval("match Ok((4u8, 5u8)) with Ok((v, n)) -> n | Err(e) -> 0u8"), u8v(5));
    // A match inside a model sees the model's parameters too.
    assert_eq!(
        eval_in(
            "  model area(s: Shape) -> Nat = match s with Circle(r) -> 3 | Rect(w, h) -> 4 | Empty -> 0\n",
            "area(Circle(1u8))"
        ),
        int(3)
    );
}

#[test]
fn v0_matches_still_evaluate() {
    let src = "warrant t\n  example e:\n    (match Some(2) with None -> 0 | Some(x) -> x + 1) == 3\n";
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
    let Declaration::Example(ex) = &file.declarations[0] else {
        panic!()
    };
    let mut ev = Evaluator::new(&file, CodegenLimits::default());
    assert_eq!(ev.eval(&ex.body, &HashMap::new()).unwrap(), EvalValue::Bool(true));
}

#[test]
fn evaluation_errors_are_reported_not_panicked() {
    // The checker would reject these; the evaluator reports them anyway.
    for expr in ["(1, 2).5", "5.0", "R(1u8, true).zzz", "Circle(1u8).r", "match 1 with Some(x) -> x"] {
        let r = eval_with("", expr, CodegenLimits::default());
        assert_eq!(r, Err(DiagnosticCode::W0702), "{expr}");
    }
}

#[test]
fn composite_size_and_work_are_bounded() {
    // Eleven leaves exceed a ten-leaf value budget: W0703.
    let tight = CodegenLimits {
        max_value_bytes: 10,
        ..CodegenLimits::default()
    };
    assert_eq!(
        eval_with("", "((1, 2, 3, 4, 5, 6, 7, 8), 1, 2, 3)", tight),
        Err(DiagnosticCode::W0703)
    );
    assert!(eval_with("", "((1, 2, 3, 4), 1)", tight).is_ok());
    assert_eq!(
        eval_with("", "R(1u8, true)", CodegenLimits { max_value_bytes: 2, ..CodegenLimits::default() }),
        Err(DiagnosticCode::W0703)
    );
    // The work budget bounds construction too.
    let starved = CodegenLimits {
        max_work: 3,
        ..CodegenLimits::default()
    };
    assert_eq!(eval_with("", "(1, 2, 3, 4, 5)", starved), Err(DiagnosticCode::W0703));
}

// ---------------------------------------------------------------------------
// Section 4.9: the portable corpus encoding
// ---------------------------------------------------------------------------

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn portable_encoding_of_tuples_records_and_variants() {
    // tuple and record: L<n> v1 ... vn, components in declaration order.
    assert_eq!(eval("(7u64, 3)").encode_portable(), "L2 I7 I3");
    assert_eq!(eval("R(5u8, true)").encode_portable(), "L2 I5 T");
    assert_eq!(eval("Outer(R(1u8, false), 9)").encode_portable(), "L2 L2 I1 F I9");
    // variant: L2 S<hex ctor name> L<k> f1 ... fk, by constructor *name*.
    assert_eq!(
        eval("Circle(5u8)").encode_portable(),
        format!("L2 S{} L1 I5", hex("Circle"))
    );
    assert_eq!(
        eval("Rect(3u8, 4u8)").encode_portable(),
        format!("L2 S{} L2 I3 I4", hex("Rect"))
    );
    assert_eq!(eval("Empty").encode_portable(), format!("L2 S{} L0", hex("Empty")));
    // Reordering the declarations cannot change a vector: the key is the name.
    let reordered = "  variant Shape2 = Empty | Rect(w: U8, h: U8) | Circle(r: U8)\n";
    assert_eq!(
        eval_in(reordered, "Circle(5u8)").encode_portable(),
        eval("Circle(5u8)").encode_portable()
    );
    // Option: L0 or L1 x (so Option<Option<T>> is not conflated).
    assert_eq!(EvalValue::Option(None).encode_portable(), "L0");
    assert_eq!(eval("Some((1, 2))").encode_portable(), "L1 L2 I1 I2");
    let nested = EvalValue::Option(Some(Box::new(EvalValue::Option(None))));
    assert_eq!(nested.encode_portable(), "L1 L0");
    // Result: the VER04i form, an M with one entry (ok -> value, err -> variant).
    assert_eq!(
        eval("Ok((1, 2))").encode_portable(),
        format!("M1 S{} L2 I1 I2", hex("ok"))
    );
    assert_eq!(
        eval("Err(E.B)").encode_portable(),
        format!("M1 S{} S{}", hex("err"), hex("B"))
    );
    // Bytes, Bool, lists and fixed-width integers inside a composite.
    assert_eq!(
        eval("(b\"0aff\", false, [1, 2], 255u8, -1i8)").encode_portable(),
        "L5 X0aff F L2 I1 I2 I255 I-1"
    );
    // The legacy encoder keeps its v0 shapes (an Option is its payload) and
    // delegates composites to the portable encoding.
    assert_eq!(EvalValue::Option(Some(Box::new(int(5)))).encode_tfv(), "I5");
    assert_eq!(EvalValue::Option(None).encode_tfv(), "N");
    assert_eq!(eval("(1, 2)").encode_tfv(), "L2 I1 I2");
    assert_eq!(eval("Empty").encode_tfv(), eval("Empty").encode_portable());
}

#[test]
fn the_encoding_agrees_with_the_test_bootstrap_tfv_parser() {
    // The bootstrap's own parser must read the strings back as the right value
    // shapes (the corpus is consumed by it).
    for expr in ["(7u64, 3)", "R(5u8, true)", "Circle(5u8)", "Empty", "Some((1, 2))", "Ok((1, 2))", "Err(E.A)"] {
        let text = eval(expr).encode_portable();
        let again = tfv_roundtrip(&text).unwrap_or_else(|e| panic!("{expr} -> {text}: {e}"));
        assert_eq!(again, text, "{expr}");
    }
}

/// Parse `text` as the value of a one-field TFV record and write it back.
fn tfv_roundtrip(text: &str) -> Result<String, String> {
    use coding_adventures_test_bootstrap::tfv;
    let file = format!("tfv 1\nr R1 Kv {text}\n");
    let records = tfv::parse_file(&file).map_err(|e| format!("{e:?}"))?;
    Ok(tfv::encode(&records[0].fields[0].1))
}

fn nest_tuples(depth: usize) -> EvalValue {
    let mut v = int(0);
    for _ in 0..depth {
        v = EvalValue::Tuple(vec![v, int(1)]);
    }
    v
}

#[test]
fn nesting_beyond_tfv_s_64_containers_is_w0709() {
    assert_eq!(TFV_MAX_DEPTH, 64);
    assert_eq!(nest_tuples(0).container_depth(), 0);
    assert_eq!(nest_tuples(64).container_depth(), 64);
    assert!(nest_tuples(64).encode_checked().is_ok());
    let e = nest_tuples(65).encode_checked().unwrap_err();
    assert_eq!(e.code, DiagnosticCode::W0709);
    assert_eq!(format!("{}", e.code), "W0709");
    // A variant costs two containers (the pair and the field list); a record,
    // Option and Result one each.
    let variant = |inner: EvalValue| EvalValue::Con {
        ty: "V".into(),
        index: 0,
        ctor: "C".into(),
        record: false,
        fields: vec![inner],
    };
    assert_eq!(variant(int(0)).container_depth(), 2);
    let mut v = int(0);
    for _ in 0..32 {
        v = variant(v);
    }
    assert_eq!(v.container_depth(), 64);
    assert!(v.encode_checked().is_ok());
    assert!(variant(v).encode_checked().is_err());
    let record = EvalValue::Con {
        ty: "R".into(),
        index: 0,
        ctor: "R".into(),
        record: true,
        fields: vec![int(1)],
    };
    assert_eq!(record.container_depth(), 1);
    assert_eq!(EvalValue::Option(Some(Box::new(int(1)))).container_depth(), 1);
    assert_eq!(EvalValue::Option(None).container_depth(), 1);
    assert_eq!(EvalValue::Result(Ok(Box::new(int(1)))).container_depth(), 1);
    assert_eq!(EvalValue::Result(Err(("E".into(), "A".into()))).container_depth(), 1);
    assert_eq!(EvalValue::List(vec![int(1), nest_tuples(2)]).container_depth(), 3);
    // The deepest value the TFV parser accepts is the deepest this accepts.
    assert!(tfv_roundtrip(&nest_tuples(64).encode_portable()).is_ok());
    assert!(tfv_roundtrip(&nest_tuples(65).encode_portable()).is_err());
}
