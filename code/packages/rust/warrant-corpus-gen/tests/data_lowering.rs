//! Slice S11b by construct: records, variants, `Option`, the four `ir.data` tags, the data patterns,
//! `requires`, the typing rules R51 and R52, the refusal order of R35 and R53 (VER04r sections 16.14 and
//! 16.16, Appendix A.2).
//!
//! `tests/data_parity.rs` proves the lowering equals the oracle's on one big document and refuses the
//! oracle's 121 documents; this file states each rule once, small, so a failure names the rule.

mod support;

use coding_adventures_warrant_corpus_gen::cost::{ir_cost, ModelCosts};
use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::lower::{lower_document, Lowered, LoweredType};
use support::*;

const OPS: [&str; 1] = ["op f(a: Bytes, b: Bytes) -> Bool"];

/// The declarations every test shares: a record, two variants, an error type.
const DECLS: &str = concat!(
    "  error E = X | Y\n",
    "  record Point(x: U8, y: U8)\n",
    "  record Slot(o: Option<U8>, t: (U8,Bool), p: Point)\n",
    "  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Dot\n",
    "  variant Mode = Fast | Slow | Idle\n",
);

fn lower(extra: &str) -> Result<Lowered, GenError> {
    let src = doc_ops(" v1", &OPS, &format!("{DECLS}{extra}"));
    with_file(&src, lower_document)
}

fn ok(extra: &str) -> Lowered {
    lower(extra).unwrap_or_else(|e| panic!("{extra}: {e}"))
}

/// The compact JSON of the body of model `name`.
fn json(l: &Lowered, name: &str) -> String {
    l.model(name)
        .unwrap_or_else(|| panic!("no model {name}"))
        .body
        .to_json()
        .render()
        .unwrap()
        .split_whitespace()
        .collect()
}

fn requires(l: &Lowered, name: &str) -> Vec<&'static str> {
    l.model(name).unwrap().requires.clone()
}

// ---- declarations ---------------------------------------------------------------------------------

#[test]
fn records_and_variants_are_the_documents_types_in_declaration_order() {
    let l = ok("");
    let names: Vec<&str> = l.types.iter().map(LoweredType::name).collect();
    assert_eq!(names, ["Point", "Slot", "Shape", "Mode"]);
    let LoweredType::Record { fields, .. } = &l.types[1] else {
        panic!("a record")
    };
    let spelled: Vec<String> = fields.iter().map(|(n, t)| format!("{n}:{}", t.name())).collect();
    assert_eq!(spelled, ["o:Option<U8>", "t:(U8,Bool)", "p:Point"]);
    let LoweredType::Variant { ctors, .. } = &l.types[2] else {
        panic!("a variant")
    };
    assert_eq!(ctors.len(), 3);
    assert_eq!(ctors[2], ("Dot".to_string(), vec![]));
    // an error type is not one of them: it stays in `errors`
    assert_eq!(l.errors.len(), 1);
}

#[test]
fn a_document_with_no_record_or_variant_has_no_types() {
    let src = doc_ops(" v1", &OPS, "");
    assert!(with_file(&src, |f| lower_document(f).unwrap()).types.is_empty());
}

#[test]
fn a_list_in_a_declaration_is_refused_at_the_declaration_and_names_ir_list() {
    for decl in [
        "  record Bag(xs: List<U8>)\n",
        "  variant Many = One(xs: List<U8>) | Two\n",
        "  model Word = List<U8>\n",
        "  record Nest(o: Option<List<U8>>, t: (U8,List<Bool>))\n",
    ] {
        let e = lower(decl).unwrap_err();
        assert_eq!((e.code, e.level), (code::LEVEL, Some("ir.list")), "{decl}: {e}");
    }
}

/// A refused declaration (a list field, a field that does not resolve) is still known by name, so a
/// constructor of it, a pattern over it or a claim about it is refused with the declaration's `ir.list`
/// (the oracle's class and level) and never panics.
#[test]
fn a_use_of_a_list_refused_type_names_the_declarations_ir_list_and_never_panics() {
    let w = "  variant W = One(xs: List<U8>) | Two\n";
    let uses: [(&str, String); 9] = [
        ("a comparison", format!("{w}  model m(a: Bool) -> Bool = Two == Two\n")),
        ("a constructor result", format!("{w}  model m(a: U8) -> W = Two\n")),
        ("a constructor with fields", format!("{w}  model m(a: U8) -> Bool = One(a) == Two\n")),
        (
            "a constructor pattern",
            format!("{w}  model m(w: W) -> U8 = match w with One(_) -> 1 | Two -> 2\n"),
        ),
        ("a claim", format!("{w}  claim c:\n    forall a: Bytes. f(a, a) == (Two == Two)\n")),
        ("the model first", format!("  model m(a: Bool) -> Bool = Two == Two\n{w}")),
        (
            "an option of a list",
            "  variant W = One(xs: Option<List<U8>>) | Two\n  model m(a: Bool) -> Bool = Two == Two\n"
                .to_string(),
        ),
        (
            "a tuple with a list",
            "  variant W = One(xs: (U8, List<U8>)) | Two\n  model m(a: Bool) -> Bool = Two == Two\n"
                .to_string(),
        ),
        (
            "a record whose field does not resolve",
            "  record Q(xs: List<Zed>)\n  model m(a: Bool) -> Bool = Q(a) == Q(a)\n".to_string(),
        ),
    ];
    for (what, extra) in uses {
        let e = lower(&extra).unwrap_err();
        assert_eq!((e.code, e.level), (code::LEVEL, Some("ir.list")), "{what}: {e}");
    }
}

/// Appendix A.2: a `match` has at most 256 arms, and over that is the tools' `limit`, not the front-end
/// checker's `check` (the oracle's `a match has more than 256 arms`).
#[test]
fn a_match_of_more_than_256_arms_is_a_limit() {
    for arms in [257, 300] {
        let body = vec!["_ -> true"; arms].join(" | ");
        let e = lower(&format!("  model m(a: Bool) -> Bool = match a with {body}\n")).unwrap_err();
        assert_eq!(e.code, code::LIMIT, "{arms} arms: {e}");
    }
}

#[test]
fn the_corpus_limits_are_checked_before_anything_else() {
    // R34: 65 constructors and 9 fields are inside the language (256 and 32) and over the corpus limits;
    // the refusal is `limit` and comes before a level named earlier in the document
    let ctors: Vec<String> = (0..65).map(|i| format!("C{i}")).collect();
    let many = format!("  variant Many = {}\n", ctors.join(" | "));
    let e = lower(&format!("  model m(a: U8) -> Bool = [a] == [a]\n{many}")).unwrap_err();
    assert_eq!((e.code, e.message.as_str()), (code::LIMIT, "a variant has at most 64 constructors"), "{e}");
    let wide = "  variant Wide = Fat(a0: U8, a1: U8, a2: U8, a3: U8, a4: U8, a5: U8, a6: U8, a7: U8, a8: U8) | Thin\n";
    let e = lower(wide).unwrap_err();
    assert_eq!(e.message, "a constructor has at most 8 fields", "{e}");
    // 64 and 8 are the limits themselves: fine
    let ctors: Vec<String> = (0..64).map(|i| format!("C{i}")).collect();
    ok(&format!("  variant Many = {}\n", ctors.join(" | ")));
    let eight = "  variant Fat = Eight(a0: U8, a1: U8, a2: U8, a3: U8, a4: U8, a5: U8, a6: U8, a7: U8) | Thin\n";
    ok(eight);
    let cases: Vec<String> = (0..65).map(|i| format!("Z{i}")).collect();
    let e = lower(&format!("  error Huge = {}\n", cases.join(" | "))).unwrap_err();
    assert_eq!(e.message, "an error type has at most 64 cases", "{e}");
}

// ---- the four tags, exactly ------------------------------------------------------------------------

#[test]
fn some_none_ctor_and_field_lower_to_the_appendix_a2_shapes() {
    let l = ok(concat!(
        "  model s(a: U8) -> Option<U8> = Some(a)\n",
        "  model n(a: U8) -> Option<U8> = None\n",
        "  model p(a: U8, b: U8) -> Point = Point(a, b)\n",
        "  model c(a: U8) -> Shape = Circle(a)\n",
        "  model d(a: U8) -> Shape = Dot\n",
        "  model fx(p: Point) -> U8 = p.y\n",
        "  model fy(s: Slot) -> U8 = s.p.x\n",
        "  model ft(s: Slot) -> Bool = s.t.1\n",
        "  model pf(a: U8) -> U8 = Point(a, 1u8).x\n",
    ));
    assert_eq!(json(&l, "s"), r#"["some",["var","a"]]"#);
    assert_eq!(json(&l, "n"), r#"["none","U8"]"#);
    assert_eq!(json(&l, "p"), r#"["ctor","Point",[["var","a"],["var","b"]]]"#);
    assert_eq!(json(&l, "c"), r#"["ctor","Circle",[["var","a"]]]"#);
    assert_eq!(json(&l, "d"), r#"["ctor","Dot",[]]"#);
    // a field is its INDEX among the declared fields; the name stays in `types`
    assert_eq!(json(&l, "fx"), r#"["field",["var","p"],1]"#);
    assert_eq!(json(&l, "fy"), r#"["field",["field",["var","s"],2],0]"#);
    // a tuple component and a record field chain: `project` and `field`
    assert_eq!(json(&l, "ft"), r#"["project",["field",["var","s"],1],1]"#);
    assert_eq!(
        json(&l, "pf"),
        r#"["field",["ctor","Point",[["var","a"],["lit",{"type":"U8","value":"1"}]]],0]"#
    );
    // cost: some, ctor and field are 1 + operands, none is 1 (A.2)
    let costs = ModelCosts::compute(&l.models, 2000).unwrap();
    assert_eq!(costs.cost("s"), Some(2));
    assert_eq!(costs.cost("n"), Some(1));
    assert_eq!(costs.cost("p"), Some(3));
    assert_eq!(costs.cost("d"), Some(1));
    assert_eq!(costs.cost("fy"), Some(3));
    // field(ctor(a, 1u8)) = 1 + (1 + 1 + 1)
    assert_eq!(ir_cost(&l.model("pf").unwrap().body, &costs, 2000).unwrap(), 4);
}

#[test]
fn match_lowers_every_data_pattern_on_any_scrutinee() {
    let l = ok(concat!(
        "  model mo(x: Option<U8>) -> U8 = match x with Some(v) -> v | None -> 0u8\n",
        "  model mb(x: Bool) -> U8 = match x with true -> 1u8 | false -> 0u8\n",
        "  model ms(x: Shape) -> U8 = match x with Circle(r) -> r | Rect(w, _) -> w | Dot -> 0u8\n",
        "  model mp(x: Point) -> U8 = match x with Point(a, _) -> a\n",
        "  model mt(x: (Bool,Bool)) -> Bool = match x with (a, _) -> a\n",
        "  model mw(x: Mode) -> U8 = match x with Fast -> 1u8 | other -> 0u8\n",
    ));
    assert_eq!(
        json(&l, "mo"),
        concat!(
            r#"["match",["var","x"],[[["some",["bind","v"]],["var","v"]],"#,
            r#"[["none"],["lit",{"type":"U8","value":"0"}]]]]"#
        )
    );
    assert!(json(&l, "mb").contains(r#"[["bool",true],"#) && json(&l, "mb").contains(r#"[["bool",false],"#));
    assert!(json(&l, "ms").contains(r#"["ctor","Rect",[["bind","w"],["wild"]]]"#));
    assert!(json(&l, "ms").contains(r#"[["ctor","Dot",[]],"#));
    assert!(json(&l, "mp").contains(r#"["ctor","Point",[["bind","a"],["wild"]]]"#));
    // a match on a tuple is `ir.data` (by the scrutinee) and `ir.tuple` (by the type of the binder)
    assert_eq!(requires(&l, "mt"), ["ir.core", "ir.data", "ir.tuple"]);
    for name in ["mo", "mb", "ms", "mp", "mw"] {
        assert_eq!(requires(&l, name), ["ir.core", "ir.data"], "{name}");
    }
    // cost: 1 + the scrutinee + the worst arm
    let costs = ModelCosts::compute(&l.models, 2000).unwrap();
    assert_eq!(costs.cost("mo"), Some(3));
    assert_eq!(costs.cost("mb"), Some(3));
}

// ---- requires: by tag, pattern and type (R9, R29) ---------------------------------------------------

#[test]
fn requires_follow_the_tag_the_pattern_and_the_type() {
    let l = ok(concat!(
        "  model plain(a: U8) -> U8 = a\n",
        "  model typed(p: Point) -> Bool = true\n",
        "  model opt_param(o: Option<U8>) -> Bool = true\n",
        "  model tup_opt(t: (Option<U8>,Bool)) -> Bool = true\n",
        "  model res_opt(a: U8) -> Result<Option<U8>,E> = Ok(Some(a))\n",
        "  model builds(a: U8) -> Bool = Dot == Dot\n",
        "  model calls(a: U8) -> Bool = typed(Point(a, a))\n",
        "  model res_bool(r: Result<Bool,E>) -> U8 = match r with Ok(true) -> 1u8 | Ok(false) -> 2u8 | Err(_) -> 0u8\n",
        "  model res_plain(r: Result<Bool,E>) -> U8 = match r with Ok(_) -> 1u8 | Err(_) -> 0u8\n",
    ));
    assert_eq!(requires(&l, "plain"), ["ir.core"]);
    assert_eq!(requires(&l, "typed"), ["ir.core", "ir.data"]);
    assert_eq!(requires(&l, "opt_param"), ["ir.core", "ir.data"]);
    // an Option inside a tuple brings both
    assert_eq!(requires(&l, "tup_opt"), ["ir.core", "ir.data", "ir.tuple"]);
    assert_eq!(requires(&l, "res_opt"), ["ir.core", "ir.data"]);
    assert_eq!(requires(&l, "builds"), ["ir.core", "ir.data"]);
    // the call closure: `calls` needs what `typed` needs
    assert_eq!(requires(&l, "calls"), ["ir.core", "ir.data"]);
    // a data pattern on a Result adds ir.data to the ir.tuple the Result brings; without one it does not
    assert_eq!(requires(&l, "res_bool"), ["ir.core", "ir.data", "ir.tuple"]);
    assert_eq!(requires(&l, "res_plain"), ["ir.core", "ir.tuple"]);
}

#[test]
fn the_intermediate_use_of_a_producer_adds_no_ir_data_r29() {
    // `unwrap_or(producer(..), d)` is `ir.fixed` and no more, exactly as before the level existed: no
    // committed `requires` moves (D48).  Every other `Option` node is counted.
    let l = ok(concat!(
        "  model inter_at(b: Bytes, i: Nat) -> U8 = unwrap_or(at(b, i), 0u8)\n",
        "  model inter_add(a: U8, b: U8) -> U8 = unwrap_or(checked_add(a, b), 0u8)\n",
        "  model inter_div(a: Nat, b: Nat) -> Int = unwrap_or(div(a, b), 0)\n",
        "  model on_binder(o: Option<U8>) -> U8 = unwrap_or(o, 0u8)\n",
        "  model not_directly(b: Bytes, i: Nat, c: Bool) -> U8 = unwrap_or(if c then at(b, i) else at(b, 0), 0u8)\n",
        "  model compared(a: U8, b: U8, x: U8) -> Bool = checked_add(a, b) == Some(x)\n",
        "  model wrapped(a: U8, b: U8) -> Result<U8,E> = if true then Ok(unwrap_or(checked_add(a, b), 0u8)) else Err(E.X)\n",
        "  model in_ok(a: Nat, b: Nat) -> Result<Option<Int>,E> = Ok(div(a, b))\n",
    ));
    assert_eq!(requires(&l, "inter_at"), ["ir.core", "ir.fixed"]);
    assert_eq!(requires(&l, "inter_add"), ["ir.core", "ir.fixed"]);
    assert_eq!(requires(&l, "inter_div"), ["ir.core", "ir.fixed"]);
    assert_eq!(requires(&l, "on_binder"), ["ir.core", "ir.data", "ir.fixed"]);
    assert_eq!(requires(&l, "not_directly"), ["ir.core", "ir.data", "ir.fixed"]);
    assert_eq!(requires(&l, "compared"), ["ir.core", "ir.data", "ir.fixed"]);
    assert_eq!(requires(&l, "wrapped"), ["ir.core", "ir.fixed"]);
    assert_eq!(requires(&l, "in_ok"), ["ir.core", "ir.data", "ir.fixed"]);
}

// ---- typing: R51 (flows, unify, the width) and R52 (None) ---------------------------------------------

#[test]
fn an_option_flows_and_unifies_by_its_payload_but_nothing_lifts_through_a_tuple() {
    // R51, [D60]: `Option<Bytes<4>>` to `Option<Bytes>`, `Option<Nat>` to `Option<Int>`, recursively
    for good in [
        "  model m(a: Nat) -> Option<Int> = Some(a)\n",
        "  model m(a: Bytes<4>) -> Option<Bytes> = Some(a)\n",
        "  model m(a: Nat) -> Option<Option<Int>> = Some(Some(a))\n",
        "  model m(a: Nat, i: Int, c: Bool) -> Option<Int> = if c then Some(a) else Some(i)\n",
        "  model m(a: Nat) -> Bool = Some(a) == Some(1)\n",
    ] {
        ok(good);
    }
    // a tuple, a record and a variant are invariant, also under an Option; `Int` never flows to `Nat`
    for bad in [
        "  model m(a: Int) -> Option<Nat> = Some(a)\n",
        "  model m(a: Nat) -> Option<(Int,Int)> = Some((a, a))\n",
        "  model m(a: Bytes) -> Option<Bytes<2>> = Some(a)\n",
    ] {
        let e = lower(bad).unwrap_err();
        assert!(e.code == code::CHECK || e.code == code::TYPE, "{bad}: {e}");
    }
}

#[test]
fn a_none_is_typed_by_its_expected_type_and_by_nothing_else_r52() {
    // the right operand of `==` takes the left operand's type, left to right only
    let l = ok("  model m(o: Option<U8>) -> Bool = o == None\n");
    assert!(json(&l, "m").ends_with(r#"["none","U8"]]"#), "{}", json(&l, "m"));
    // the payload of Some, a constructor argument, an argument, the default of unwrap_or, a tuple component
    let l = ok(concat!(
        "  model nn() -> Option<Option<U8>> = Some(None)\n",
        "  model ca() -> Slot = Slot(None, (1u8, true), Point(1u8, 2u8))\n",
        "  model arg(o: Option<U8>) -> Bool = o == o\n",
        "  model call() -> Bool = arg(None)\n",
        "  model tc() -> (Option<U8>,Bool) = (None, true)\n",
        "  model uo(o: Option<Option<U8>>) -> Option<U8> = unwrap_or(o, None)\n",
    ));
    assert_eq!(json(&l, "nn"), r#"["some",["none","U8"]]"#);
    assert!(json(&l, "ca").contains(r#"["none","U8"]"#));
    assert!(json(&l, "call").contains(r#"[["none","U8"]]"#));
    assert!(json(&l, "tc").contains(r#"["none","U8"]"#));
    assert!(json(&l, "uo").ends_with(r#"["none","U8"]]"#));
    // a branch and an arm inherit the expected type of the expression they are in; under an expected
    // `Option<Int>` the `None` of `if c then Some(n) else None` is `["none","Int"]` (R52's example)
    let l = ok(concat!(
        "  model ifn(c: Bool, n: Nat) -> Option<Int> = if c then Some(n) else None\n",
        "  model mn(c: Bool) -> Option<U8> = match c with true -> Some(1u8) | false -> None\n",
    ));
    assert!(json(&l, "ifn").contains(r#"["none","Int"]"#), "{}", json(&l, "ifn"));
    assert!(json(&l, "mn").contains(r#"["none","U8"]"#));
    // but never from a sibling, a scrutinee, a left operand or a first operand of `unwrap_or`
    for bad in [
        "  model m(a: Bool) -> Bool = None == None\n",
        "  model m(o: Option<U8>) -> Bool = None == o\n",
        "  model m(a: Bool) -> Bool = match None with None -> true | _ -> false\n",
        "  model m(c: Bool, d: Bytes) -> U8 = unwrap_or(if c then at(d, 0) else None, 0u8)\n",
        "  model m(c: Bool, d: Bytes) -> U8 = unwrap_or(if c then None else at(d, 0), 0u8)\n",
    ] {
        let e = lower(bad).unwrap_err();
        assert!(e.code == code::CHECK || e.code == code::TYPE, "{bad}: {e}");
        assert!(!e.is_level(), "{bad}: {e}");
    }
}

#[test]
fn a_constructor_argument_is_checked_per_field_with_assignability() {
    // `Nat` to `Int` and `Bytes<N>` to `Bytes` per argument, an Option and a Result by their payload
    ok(concat!(
        "  record Wide(n: Int, b: Bytes, o: Option<Int>)\n",
        "  model m(a: Nat, c: Bytes<2>, o: Option<Nat>) -> Wide = Wide(a, c, o)\n",
    ));
    // a record argument of the wrong record, and a tuple that would lift, are refused
    for bad in [
        "  model m(a: U8) -> Point = Point(a, true)\n",
        "  record RT(p: (Int,Int))\n  model m(a: Nat) -> RT = RT((a, a))\n",
        "  model m(a: U8) -> Shape = Dot(a)\n",
        "  model m(a: U8) -> Point = Point(a)\n",
        "  model m(a: U8) -> Shape = Circle\n",
    ] {
        assert!(lower(bad).is_err(), "{bad}");
    }
}

#[test]
fn equality_is_structural_and_an_ordering_on_data_is_a_type_error() {
    ok("  model m(a: Point, b: Point) -> Bool = a == b and a != b\n");
    ok("  model m(a: Shape, b: Shape) -> Bool = a == b\n");
    ok("  model m(a: Option<U8>, b: Option<U8>) -> Bool = a == b\n");
    for bad in [
        "  model m(a: Point, b: Point) -> Bool = a < b\n",
        "  model m(a: Option<U8>, b: Option<U8>) -> Bool = a <= b\n",
        "  model m(a: Point, b: Shape) -> Bool = a == b\n",
    ] {
        assert!(lower(bad).is_err(), "{bad}");
    }
}

// ---- the corpus errors over the data patterns (R35 (3), (4)) --------------------------------------------

#[test]
fn a_match_that_falls_through_or_has_an_arm_no_value_reaches_is_a_corpus_error_over_every_type() {
    for (src, sentence) in [
        ("  model m(x: Bool) -> Bool = match x with true -> true\n", "a match that can fall through"),
        ("  model m(x: Option<U8>) -> Bool = match x with Some(_) -> true\n", "a match that can fall through"),
        ("  model m(x: Mode) -> Bool = match x with Fast -> true | Slow -> false\n", "a match that can fall through"),
        ("  model m(x: Point) -> Bool = match x with Point(_, _) -> true | Point(_, _) -> false\n", "a match arm that no value can reach"),
        ("  model m(x: Option<U8>) -> Bool = match x with _ -> true | Some(_) -> false\n", "a match arm that no value can reach"),
        // unreachable is found before fall-through (R12)
        ("  model m(x: Mode) -> Bool = match x with Fast -> true | Fast -> false\n", "a match arm that no value can reach"),
    ] {
        let e = lower(src).unwrap_err();
        assert_eq!((e.code, e.message.as_str()), (code::IR, sentence), "{src}: {e}");
    }
}

#[test]
fn the_bind_rules_are_corpus_errors_checked_after_the_structure_of_the_pattern() {
    for (src, sentence) in [
        (
            "  model m(x: Shape) -> Bool = match x with Rect(a, a) -> true | _ -> false\n",
            "the same bind name twice in one pattern",
        ),
        (
            "  model m(w: U8, x: Shape) -> Bool = match x with Rect(w, _) -> true | _ -> false\n",
            "a bind name shadows a binder, a parameter, result or an outer bind",
        ),
        (
            "  model m(o: Option<U8>) -> Bool = match o with Some(result) -> true | None -> false\n",
            "a bind name shadows a binder, a parameter, result or an outer bind",
        ),
        // R54: a pattern's own corpus error is named before a reserved construct in an arm body of an
        // earlier arm, though the checker's refusal of it is a different one (W0301)
        (
            "  model m(x: Shape) -> Bool = match x with Circle(r) -> range(1) == 1 | Rect(r, r) -> true | Dot -> false\n",
            "the same bind name twice in one pattern",
        ),
        // the oracle's own sentence for a bind named like a model, an op, an error type or a built-in
        (
            "  model helper(a: Bool) -> Bool = a\n  model m(o: Option<U8>) -> Bool = match o with Some(helper) -> true | None -> false\n",
            "a bind name shadows a model, an operation, an error type or a reserved built-in",
        ),
        (
            "  model m(o: Option<U8>) -> Bool = match o with Some(rotl) -> true | None -> false\n",
            "a bind name shadows a model, an operation, an error type or a reserved built-in",
        ),
    ] {
        let e = lower(src).unwrap_err();
        assert_eq!((e.code, e.message.as_str()), (code::IR, sentence), "{src}: {e}");
    }
}

// ---- projection errors are typing errors since S11b (R39 (c)) --------------------------------------------

#[test]
fn a_projection_that_does_not_fit_is_a_typing_error_never_a_level() {
    for bad in [
        "  model m(a: Bool) -> Bool = a.0\n",
        "  model m(a: Bool) -> Bool = a.x\n",
        "  model m(t: (Bool,Bool)) -> Bool = t.x\n",
        "  model m(s: Shape) -> U8 = s.r\n",
        "  model m(p: Point) -> U8 = p.0\n",
        "  model m(p: Point) -> U8 = p.z\n",
    ] {
        let e = lower(bad).unwrap_err();
        assert!(e.code == code::CHECK || e.code == code::TYPE, "{bad}: {e}");
        assert!(!e.is_level(), "{bad}: {e}");
    }
}

// ---- the refusal order of R35 and R53 -----------------------------------------------------------------------

#[test]
fn the_refusal_named_is_the_one_of_the_unit_declared_on_the_earliest_line() {
    // a claim is lowered after a model, but a refusal is named by position: the claim comes first here
    let e = lower(concat!(
        "  claim c:\n    forall a: Bytes. f(a, a) == (fixed<4>(a) == None)\n",
        "  model m(a: Nat) -> Bool = range(a) == a\n",
    ))
    .unwrap_err();
    assert_eq!((e.level, e.at.as_str()), (Some("fixed-cast"), "c"), "{e}");
    let e = lower(concat!(
        "  model m(a: Nat) -> Bool = range(a) == a\n",
        "  claim c:\n    forall a: Bytes. f(a, a) == (fixed<4>(a) == None)\n",
    ))
    .unwrap_err();
    assert_eq!((e.level, e.at.as_str()), (Some("ir.seq"), "m"), "{e}");
    // a list type in a declaration is a unit too
    let e = lower(concat!(
        "  model m(a: Nat) -> Bool = range(a) == a\n",
        "  record Bag(xs: List<U8>)\n",
    ))
    .unwrap_err();
    assert_eq!((e.level, e.at.as_str()), (Some("ir.seq"), "m"), "{e}");
    let e = lower(concat!(
        "  record Bag(xs: List<U8>)\n",
        "  model m(a: Nat) -> Bool = range(a) == a\n",
    ))
    .unwrap_err();
    assert_eq!((e.level, e.at.as_str()), (Some("ir.list"), "Bag"), "{e}");
}

#[test]
fn a_reserved_construct_wins_over_a_typing_error_of_the_same_unit_and_the_first_in_source_order() {
    // `None == None` is a typing error; the unit also holds a reserved construct, which is named
    for (body, level) in [
        ("None == None and fixed<4>(a) == None", "fixed-cast"),
        ("None == None and range(1) == 2", "ir.seq"),
        ("None == None and [a] == [a]", "ir.list"),
        ("None == None and slice(a, 0, 1) == a", "ir.list"),
        // the first of two reserved constructs, in source order
        ("to_list(a) == range(1)", "ir.list"),
        ("range(1) == to_list(a)", "ir.seq"),
    ] {
        let src = format!("  model m(a: Bytes) -> Bool = {body}\n");
        let e = lower(&src).unwrap_err();
        assert_eq!((e.code, e.level), (code::LEVEL, Some(level)), "{body}: {e}");
    }
    // a unit that holds no reserved construct keeps its typing error
    let e = lower("  model m(a: Bytes) -> Bool = None == None\n").unwrap_err();
    assert!(!e.is_level(), "{e}");
    // an earlier unit's refusal beats a later unit's typing error; a later one's does not beat an earlier error
    let e = lower("  model a1(a: U8) -> Bool = [a] == [a]\n  model a2(a: U8) -> Bool = a\n").unwrap_err();
    assert_eq!(e.level, Some("ir.list"), "{e}");
    let e = lower("  model a1(a: U8) -> Bool = a\n  model a2(a: U8) -> Bool = [a] == [a]\n").unwrap_err();
    assert!(!e.is_level(), "{e}");
}

#[test]
fn an_error_value_is_refused_only_when_it_is_used_and_is_err_any_otherwise() {
    let used = [
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(e) -> e == e\n",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(E.X) -> true | Err(e) -> e == E.Y\n",
    ];
    for src in used {
        let e = lower(src).unwrap_err();
        assert_eq!((e.code, e.level), (code::LEVEL, Some("err-value")), "{src}: {e}");
    }
    let l = ok("  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(e) -> false\n");
    assert!(json(&l, "m").contains(r#"["err_any"]"#));
    let l = ok("  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(_) -> false\n");
    assert!(json(&l, "m").contains(r#"["err_any"]"#));
}

#[test]
fn a_call_to_an_op_whose_signature_names_a_list_is_ir_list_wherever_it_is_declared_r57() {
    let ops = ["op f(a: Bytes, b: Bytes) -> Bool", "op h(a: Bytes, b: Bytes) -> List<U8>"];
    for extra in [
        "  model m(a: Bytes) -> Nat = len(h(a, a))\n",
        "  claim c:\n    forall a: Bytes. f(a, a) == (len(h(a, a)) == 0)\n",
    ] {
        let src = doc_ops(" v1", &ops, extra);
        let e = with_file(&src, |f| lower_document(f).unwrap_err());
        assert_eq!((e.code, e.level), (code::LEVEL, Some("ir.list")), "{extra}: {e}");
    }
    // a list operand of len, however it got its type (a field declared after its use)
    let e = lower("  model m(q: Late) -> Nat = len(q.xs)\n  record Late(xs: List<U8>)\n").unwrap_err();
    assert_eq!((e.code, e.level), (code::LEVEL, Some("ir.list")), "{e}");
}

// ---- since S11c-2 `analyse` and `generate` accept a document that needs ir.data (R37 lifted) -----------------

#[test]
fn analyse_accepts_a_document_that_declares_or_needs_ir_data_since_s11c_2() {
    use coding_adventures_warrant_corpus_gen::{analyse, profile::Profile};
    // a declared type, an op, a model, a claim and an example that need the level: S11b refused each of them
    // (naming the first), S11c-2 evaluates them and removes the guard
    let src = doc_ops(" v1", &OPS, DECLS);
    with_file(&src, |f| assert!(analyse(f, &Profile::default()).is_ok()));
    let src = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool", "op o(a: Bytes, b: Bytes) -> Option<U8>"],
        "",
    );
    with_file(&src, |f| assert!(analyse(f, &Profile::default()).is_ok()));
    let src = doc_ops(" v1", &OPS, "  model sm(a: U8) -> Option<U8> = Some(a)\n");
    with_file(&src, |f| assert!(analyse(f, &Profile::default()).is_ok()));
    let src = doc_ops(
        " v1",
        &OPS,
        "  claim cm:\n    forall o: Option<U8>. f(b\"\", b\"\") == (o == o)\n",
    );
    with_file(&src, |f| assert!(analyse(f, &Profile::default()).is_ok()));
    let src = doc_ops(
        " v1",
        &OPS,
        "  example ex:\n    f(b\"\", b\"\") == (Some(1u8) == Some(1u8))\n",
    );
    with_file(&src, |f| assert!(analyse(f, &Profile::default()).is_ok()));
}
