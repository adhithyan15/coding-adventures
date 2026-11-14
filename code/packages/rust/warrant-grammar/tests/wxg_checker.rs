//! VER04k sections 4.2, 4.3 and 4.11 (checker negatives): the post-state slot
//! rules S1 to S4, the admissible `inout` types, the caps, and the rules for
//! `ensures` blocks.  Each case is authored with its own expected code.

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

/// The checker's verdict on a v1 document: `None` when it checks, else the code.
fn verdict(src: &str) -> Option<String> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    check_warrant(&file).err().map(|e| e.code.to_string())
}

fn ok(src: &str) {
    assert_eq!(verdict(src), None, "{src}");
}

fn bad(src: &str, code: &str) {
    assert_eq!(verdict(src).as_deref(), Some(code), "{src}");
}

/// A v1 document with `Cause = X`, the given declaration lines, and a public
/// `labels` block for every parameter of the one operation `f`.
fn doc(op: &str, params: &[&str], rest: &str) -> String {
    let labels: String = params.iter().map(|p| format!("    {p} public\n")).collect();
    format!("warrant t v1\n  error Cause = X\n  record Rec(a: U8, b: Bool)\n  {op}\n  labels f:\n{labels}{rest}")
}

#[test]
fn every_slot_rule_accepts_its_shape() {
    // S1: k = 1 and R = P1.
    ok(&doc("op f(buf: inout Bytes) -> Bytes", &["buf"], ""));
    // S2: R = Result<P1, E>.
    ok(&doc("op f(buf: inout Bytes) -> Result<Bytes, Cause>", &["buf"], ""));
    // S3: the last k components are the post-states.
    ok(&doc("op f(v: Nat, out: inout Bytes) -> (Nat, Bytes)", &["v", "out"], ""));
    ok(&doc("op f(a: inout Bytes, n: inout Nat) -> (Bool, Bytes, Nat)", &["a", "n"], ""));
    ok(&doc("op f(a: inout Bytes, n: inout Nat) -> (Bytes, Nat)", &["a", "n"], ""));
    // S4: Result of a tuple with the same tail condition.
    ok(&doc(
        "op f(v: Nat, out: inout Bytes) -> Result<(Nat, Bytes), Cause>",
        &["v", "out"],
        "",
    ));
    // S1 with a record: the whole result is the post-state.  (A tuple cannot be an
    // inout type, so S1 and S3 never both match: VER04k section 4.13 item 1.)
    ok(&doc("op f(p: inout Rec) -> Rec", &["p"], ""));
}

#[test]
fn slot_near_misses_are_w0713() {
    // Result<P, E> with k = 2.
    bad(&doc("op f(a: inout Bytes, b: inout Bytes) -> Result<Bytes, Cause>", &["a", "b"], ""), "W0713");
    // The tuple's tail is in the wrong order.
    bad(&doc("op f(a: inout Bytes, b: inout Nat) -> (Nat, Bytes)", &["a", "b"], ""), "W0713");
    // n < k.
    bad(&doc("op f(a: inout Bytes, b: inout Nat) -> (Nat, Bytes, Bool)", &["a", "b"], ""), "W0713");
    // The post-state is inside an Option, a List, a user variant.
    bad(&doc("op f(a: inout Bytes) -> Option<Bytes>", &["a"], ""), "W0713");
    bad(&doc("op f(a: inout Bytes) -> List<Bytes>", &["a"], ""), "W0713");
    // A result that has nothing to do with the parameter.
    bad(&doc("op f(a: inout Bytes) -> Nat", &["a"], ""), "W0713");
    // Exact type equality: Bytes<4> is not Bytes.
    bad(&doc("op f(a: inout Bytes<4>) -> Bytes", &["a"], ""), "W0713");
}

#[test]
fn inout_types_that_cannot_be_written_back_are_w0713() {
    bad(&doc("op f(a: inout Option<U8>) -> Option<U8>", &["a"], ""), "W0713");
    // A tuple is not admissible, as an inout type or as a record field.
    bad(&doc("op f(a: inout (Nat, Nat)) -> (Nat, Nat)", &["a"], ""), "W0713");
    let src = "warrant t v1\n  record Bad(p: (U8, U8))\n  op f(a: inout Bad) -> Bad\n  labels f:\n    a public\n";
    bad(src, "W0713");
    // A symbolic width is a static parameter, never an inout type.
    bad(
        "warrant t v1\n  op f<N: Nat>(a: inout Bytes<N>) -> Bytes<N>\n  labels f:\n    a public\n",
        "W0713",
    );
    // The admissible ones.
    for ty in ["Bool", "U8", "U64", "Nat", "Bytes", "Bytes<4>", "List<U8>", "Rec", "List<Rec>"] {
        ok(&doc(&format!("op f(a: inout {ty}) -> {ty}"), &["a"], ""));
    }
}

#[test]
fn inout_caps_and_fresh_are_w0713() {
    // Four inout parameters are the most.
    let four = "op f(a: inout U8, b: inout U8, c: inout U8, d: inout U8) -> (U8, U8, U8, U8)";
    ok(&doc(four, &["a", "b", "c", "d"], ""));
    let five = "op f(a: inout U8, b: inout U8, c: inout U8, d: inout U8, e: inout U8) -> (U8, U8, U8, U8, U8)";
    bad(&doc(five, &["a", "b", "c", "d", "e"], ""), "W0713");
    // A fresh operation returns a buffer that is by definition not the caller's.
    bad(&doc("op f(a: inout Bytes) -> Bytes fresh", &["a"], ""), "W0713");
}

#[test]
fn ensures_rows_are_checked_against_the_parameters_and_result() {
    let op = "op f(buf: inout Bytes) -> Bytes";
    ok(&doc(op, &["buf"], "  ensures f:\n    len(result) == len(buf)\n    result == buf or true\n"));
    // A row is a Bool.
    bad(&doc(op, &["buf"], "  ensures f:\n    len(result)\n"), "W0301");
    // `result` has the operation's declared type: Nat is not Bytes.
    bad(&doc(op, &["buf"], "  ensures f:\n    result == 1\n"), "W0301");
    // An unbound name.
    bad(&doc(op, &["buf"], "  ensures f:\n    len(nope) == 0\n"), "W0202");
    // `result` is not bound outside an ensures row.
    bad(&doc(op, &["buf"], "  claim c:\n    forall b: Bytes. len(result) == 0\n"), "W0202");
    // An ensures block may be attached to an operation with no inout parameter.
    ok(&doc("op f(x: U8) -> U8", &["x"], "  ensures f:\n    result == x or true\n"));
}

#[test]
fn ensures_structure_rules() {
    let op = "op f(buf: inout Bytes) -> Bytes";
    // 32 rows are the most; 33 is W0602.
    let rows = |n: usize| -> String {
        let body: String = (0..n).map(|_| "    len(result) == len(buf)\n").collect();
        format!("  ensures f:\n{body}")
    };
    ok(&doc(op, &["buf"], &rows(32)));
    bad(&doc(op, &["buf"], &rows(33)), "W0602");
    // The block names a declared operation.
    bad(&doc(op, &["buf"], "  ensures g:\n    true\n"), "W0401");
    // One block per operation.
    bad(
        &doc(op, &["buf"], "  ensures f:\n    true\n  ensures f:\n    true\n"),
        "W0201",
    );
    // A parameter named `result` is shadowed by the contextual binder.
    bad(
        &doc("op f(result: inout Bytes) -> Bytes", &["result"], "  ensures f:\n    true\n"),
        "W0203",
    );
    // ... but legal when the operation has no ensures block.
    ok(&doc("op f(result: inout Bytes) -> Bytes", &["result"], ""));
}

#[test]
fn ensures_rows_may_not_call_operations_or_quantify() {
    let two = "op f(buf: inout Bytes) -> Bytes\n  op g(x: Bytes) -> Bytes\n  labels g:\n    x public";
    bad(&doc(two, &["buf"], "  ensures f:\n    result == g(buf)\n"), "W0713");
    bad(&doc(two, &["buf"], "  ensures f:\n    result == f(buf)\n"), "W0713");
    bad(&doc(two, &["buf"], "  ensures f:\n    forall x: U8. x == x\n"), "W0713");
    bad(&doc(two, &["buf"], "  ensures f:\n    exists x: U8. x == x\n"), "W0713");
    // A model call is fine.
    ok(&doc(
        "model same(a: Bytes, b: Bytes) -> Bool = a == b\n  op f(buf: inout Bytes) -> Bytes",
        &["buf"],
        "  ensures f:\n    same(result, buf)\n",
    ));
}

#[test]
fn a_v0_document_cannot_reach_any_of_it() {
    // v0 text never gets to the checker: the parser refuses it (W0101), which is
    // what keeps every v0 document meaning what it did.
    let src = "warrant t\n  op f(buf: inout Bytes) -> Bytes\n  labels f:\n    buf public\n";
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let e = parse_document(&layout, &doc, ParserLimits::default()).expect_err("v0");
    assert_eq!(e.code(), "W0101");
}
