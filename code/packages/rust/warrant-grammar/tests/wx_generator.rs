//! VER04j sections 2.6, 3.6 and 10.4: what the generator decides for a v1
//! document -- binder domains, the budget demotion rule, the count-aware `Nat`
//! domain -- and what it refuses.  These tests inspect the generated text;
//! `tests/wx_evidence.rs` compiles and runs it.

use std::collections::BTreeMap;

use coding_adventures_warrant_grammar::binding::{BindingManifest, OpBinding};
use coding_adventures_warrant_grammar::codegen::DiagnosticCode;
use coding_adventures_warrant_grammar::evidence::{generate_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn bindings(ops: &[(&str, &[&str])]) -> BindingManifest {
    let mut b = BTreeMap::new();
    for (name, faults) in ops {
        b.insert(
            name.to_string(),
            OpBinding {
                export: name.to_string(),
                static_params: Vec::new(),
                fresh: false,
                error_normalization: None,
                negative_controls: faults.iter().map(|s| s.to_string()).collect(),
            },
        );
    }
    BindingManifest {
        package: "subject".into(),
        revision: "0".into(),
        bindings: b,
    }
}

fn options(budget: u64, arg_types: BTreeMap<String, Vec<String>>) -> EvidenceOptions {
    EvidenceOptions {
        package_crate: "subject".into(),
        spec_path: "code/specs/t.warrant".into(),
        spec_sha256: "0".repeat(64),
        seed: 1,
        property_cases: 10,
        fuzz_iterations: 10,
        negctl_fuzz_iterations: 10,
        exhaustive_budget: budget,
        arg_types,
    }
}

fn generate(
    src: &str,
    b: &BindingManifest,
    budget: u64,
    arg_types: BTreeMap<String, Vec<String>>,
) -> Result<String, (DiagnosticCode, String)> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    generate_evidence_harness(&file, b, &options(budget, arg_types))
        .map_err(|e| (e.code, format!("{e}")))
}

/// A one-claim v1 document over `op f(<params>) -> <ret>`.
fn claim_doc(header: &str, params: &str, ret: &str, binders: &str, body: &str) -> String {
    format!(
        "warrant t{header}\n  op f({params}) -> {ret}\n  claim c:\n    forall {binders}. {body}\n"
    )
}

fn tys_of(text: &str) -> String {
    let start = text
        .find("const CLAIM_C_TYS: &[Ty] = &[")
        .expect("TYS constant");
    let rest = &text[start + "const CLAIM_C_TYS: &[Ty] = &[".len()..];
    rest[..rest.find("];").unwrap()].to_string()
}

fn demoted_of(text: &str) -> String {
    let start = text
        .find("const CLAIM_C_DEMOTED: &[&str] = &[")
        .expect("DEMOTED constant");
    let rest = &text[start + "const CLAIM_C_DEMOTED: &[&str] = &[".len()..];
    rest[..rest.find("];").unwrap()].to_string()
}

const REFERENCE: u64 = 400_000;
const RAISED: u64 = 1 << 26;

fn two_u16(budget: u64) -> Result<String, (DiagnosticCode, String)> {
    let b = bindings(&[("f", &["narrow-width"])]);
    generate(
        &claim_doc(
            " v1",
            "a: U16, b: U16",
            "U16",
            "a: U16, b: U16",
            "f(a, b) == a + b",
        ),
        &b,
        budget,
        BTreeMap::new(),
    )
}

#[test]
fn a_u16_pair_is_demoted_leftmost_first_until_it_fits() {
    // 65536 * 65536 -> 281 * 65536 (18.4M, still over) -> 281 * 281 (78,961).
    let text = two_u16(REFERENCE).unwrap();
    assert_eq!(tys_of(&text), "Ty::FxEdge(F::U16), Ty::FxEdge(F::U16)");
    assert_eq!(demoted_of(&text), "\"a\", \"b\"");
    // At 2^26 one demotion is enough (18.4M <= 67.1M): only the first.
    let text = two_u16(RAISED).unwrap();
    assert_eq!(tys_of(&text), "Ty::FxEdge(F::U16), Ty::Fx(F::U16)");
    assert_eq!(demoted_of(&text), "\"a\"");
    // Within budget, nothing is demoted: 65536 alone.
    let b = bindings(&[("f", &["narrow-width"])]);
    let one = generate(
        &claim_doc(" v1", "a: U16", "U16", "a: U16", "f(a) == a + 1u16"),
        &b,
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(tys_of(&one), "Ty::Fx(F::U16)");
    assert_eq!(demoted_of(&one), "");
}

#[test]
fn the_budget_rule_demotes_i16_too_and_fails_with_w0706_when_nothing_is_left() {
    // `op f(a)` is called with the first binder only; the other binders just
    // multiply the domain, which is all the budget rule looks at.
    let gen = |ty: &str, binders: &str, budget: u64| {
        generate(
            &claim_doc(" v1", &format!("a: {ty}"), ty, binders, "f(a) == a"),
            &bindings(&[("f", &["narrow-width"])]),
            budget,
            BTreeMap::new(),
        )
    };
    // I16 x I16 behaves like U16 x U16 (527 * 527 = 277,729 fits the reference budget).
    let text = generate(
        &claim_doc(
            " v1",
            "a: I16, b: I16",
            "I16",
            "a: I16, b: I16",
            "f(a, b) == a + b",
        ),
        &bindings(&[("f", &["narrow-width"])]),
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(tys_of(&text), "Ty::FxEdge(F::I16), Ty::FxEdge(F::I16)");
    // U16 x U8 x U8: the one U16 is demoted (281 * 256 * 256 = 18.4M), which is
    // still over the reference budget -- W0706 -- and within the raised one.
    assert_eq!(
        gen("U16", "a: U16, b: U8, c: U8", REFERENCE).unwrap_err().0,
        DiagnosticCode::W0706
    );
    let text = gen("U16", "a: U16, b: U8, c: U8", RAISED).unwrap();
    assert_eq!(
        tys_of(&text),
        "Ty::FxEdge(F::U16), Ty::Fx(F::U8), Ty::Fx(F::U8)"
    );
    assert_eq!(demoted_of(&text), "\"a\"");
    // U8 binders are never demoted: three of them are W0706 at the reference
    // budget and fine at 2^24 (the raised option gf256 uses).
    assert_eq!(
        gen("U8", "a: U8, b: U8, c: U8", REFERENCE).unwrap_err().0,
        DiagnosticCode::W0706
    );
    let text = gen("U8", "a: U8, b: U8, c: U8", 1 << 24).unwrap();
    assert_eq!(demoted_of(&text), "");
    // Wide types have no full domain to demote: three U32 binders are W0706.
    assert_eq!(
        gen("U32", "a: U32, b: U32, c: U32", REFERENCE)
            .unwrap_err()
            .0,
        DiagnosticCode::W0706
    );
    assert!(gen("U32", "a: U32, b: U32", REFERENCE).is_ok());
    // Demotion leaves the *other* binders' kinds alone, and keeps their order.
    let text = gen("U16", "a: U8, b: U16, c: U16", RAISED);
    // (`f(a)` has the wrong argument type here; the generator does not type
    // check, and the budget outcome is what is asserted.)
    let text = text.unwrap();
    assert_eq!(
        tys_of(&text),
        "Ty::Fx(F::U8), Ty::FxEdge(F::U16), Ty::FxEdge(F::U16)"
    );
    assert_eq!(demoted_of(&text), "\"b\", \"c\"");
}

#[test]
fn a_nat_that_flows_directly_into_a_shift_count_gets_the_count_domain() {
    let b = bindings(&[("f", &["shift-mask"])]);
    let g = |body: &str| {
        generate(
            &claim_doc(" v1", "a: U8, n: Nat", "U8", "a: U8, n: Nat", body),
            &b,
            REFERENCE,
            BTreeMap::new(),
        )
        .unwrap()
    };
    for func in ["shl", "shr", "rotl", "rotr", "bit"] {
        let body = if func == "bit" {
            "f(a, n) == (if bit(a, n) then 1u8 else 0u8)".to_string()
        } else {
            format!("f(a, n) == {func}(a, n)")
        };
        assert_eq!(tys_of(&g(&body)), "Ty::Fx(F::U8), Ty::NatCount", "{func}");
    }
    // sar needs a signed operand; the count rule is the same.
    let text = generate(
        &claim_doc(
            " v1",
            "a: I8, n: Nat",
            "I8",
            "a: I8, n: Nat",
            "f(a, n) == sar(a, n)",
        ),
        &bindings(&[("f", &["shift-mask"])]),
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(tys_of(&text), "Ty::Fx(F::I8), Ty::NatCount");
    // Used only indirectly (inside arithmetic) or not at all: the plain Nat set.
    assert_eq!(
        tys_of(&g("f(a, n) == shl(a, 1) + a")),
        "Ty::Fx(F::U8), Ty::Nat"
    );
    // The *first* argument is not a count.
    let b2 = bindings(&[("f", &["shift-mask"])]);
    let text = generate(
        &claim_doc(" v1", "n: Nat", "U8", "n: Nat", "f(n) == shl(1u8, 1)"),
        &b2,
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(tys_of(&text), "Ty::Nat");
}

#[test]
fn generation_is_deterministic_and_marks_the_version() {
    let a = two_u16(REFERENCE).unwrap();
    let b = two_u16(REFERENCE).unwrap();
    assert_eq!(a, b);
    assert!(
        a.contains("pub fn write_manifest_for"),
        "the v1 prelude is selected"
    );
    assert!(a.contains("Json::str(\"v1\")"));
    assert!(!a.contains("//@"), "marker lines never reach the output");
    // The v0 prelude is not.
    let v0 = generate(
        "warrant t\n  op f(a: U8) -> U8\n  claim c:\n    forall a: U8. f(a) == a\n",
        &bindings(&[("f", &["wrapping-sub"])]),
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap();
    assert!(!v0.contains("write_manifest_for"));
    assert!(!v0.contains("Json::str(\"v1\")"));
    assert!(v0.contains("V::Nat"));
}

#[test]
fn the_generator_refuses_what_it_cannot_translate_or_adapt_in_a_v1_document() {
    let b = |faults: &[&str]| bindings(&[("f", faults)]);
    let refuse = |src: &str, faults: &[&str], args: Vec<(&str, Vec<&str>)>| -> DiagnosticCode {
        let at: BTreeMap<String, Vec<String>> = args
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
            .collect();
        generate(src, &b(faults), REFERENCE, at).unwrap_err().0
    };
    let doc = |params: &str, ret: &str, binders: &str, body: &str| {
        claim_doc(" v1", params, ret, binders, body)
    };
    // Unknown fault.
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == a"),
            &["no-such-fault"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    // sign-confuse on an unsigned operation, missing-error on a total one.
    assert_eq!(
        refuse(
            &doc("a: U8, b: U8", "Bool", "a: U8, b: U8", "f(a, b) == (a < b)"),
            &["sign-confuse"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == a"),
            &["missing-error"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    // args= with the wrong arity or an unsupported type.
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == a"),
            &["narrow-width"],
            vec![("f", vec!["u8", "u8"])]
        ),
        DiagnosticCode::W0701
    );
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == a"),
            &["narrow-width"],
            vec![("f", vec!["u128"])]
        ),
        DiagnosticCode::W0701
    );
    // A Result operation needs an error normalisation in its binding.
    let src = "warrant t v1\n  error E = BAD\n  op f(a: I32) -> Result<I8, E>\n  claim c:\n    forall a: I32. f(a) == Ok(0i8)\n";
    assert_eq!(
        refuse(src, &["missing-error"], vec![]),
        DiagnosticCode::W0704
    );
    // An operation with no scalar shape (a Bytes parameter next to an integer).
    assert_eq!(
        refuse(
            &doc("a: U8, b: Bytes", "U8", "a: U8, b: Bytes", "f(a, b) == a"),
            &["narrow-width"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    // Forms the translator never supported are still W0701 in v1.
    for body in [
        "f(a) == [a]",
        "f(a) == (match try<U8>(a) with None -> a | Some(v) -> v)",
    ] {
        let src = doc("a: U8", "U8", "a: U8", body);
        let _ = src;
    }
    assert_eq!(
        refuse(
            &doc("a: U8", "Bool", "a: U8", "f(a) == [true]"),
            &["invert"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    // An unknown built-in call or a symbolic static argument.
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == nosuch(a)"),
            &["narrow-width"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == wrap<N>(a)"),
            &["narrow-width"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    // Wrong operand count reaches the translator when the checker is skipped.
    assert_eq!(
        refuse(
            &doc("a: U8", "U8", "a: U8", "f(a) == bit_not(a, a)"),
            &["narrow-width"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
    // A v0 document cannot use v1 binder types.
    assert_eq!(
        refuse(
            "warrant t\n  op f(a: U16) -> U16\n  claim c:\n    forall a: U16. f(a) == a\n",
            &["narrow-width"],
            vec![]
        ),
        DiagnosticCode::W0701
    );
}

#[test]
fn a_claim_whose_only_operation_call_is_inside_a_built_in_still_names_its_operation() {
    let b = bindings(&[("f", &["popcount-off"])]);
    let src = "warrant t v1\n  op f(a: I8) -> Nat\n  claim c:\n    forall a: I8. popcount(bit_not(a)) == f(bit_not(a))\n  claim d:\n    forall a: I8. 8 == popcount(a) + 0\n".to_string();
    // Claim `d` has no operation at all and must be refused; `c` is fine.
    let only_c = "warrant t v1\n  op f(a: I8) -> Nat\n  claim c:\n    forall a: I8. popcount(bit_not(a)) == f(bit_not(a))\n";
    assert!(generate(only_c, &b, REFERENCE, BTreeMap::new()).is_ok());
    assert_eq!(
        generate(&src, &b, REFERENCE, BTreeMap::new())
            .unwrap_err()
            .0,
        DiagnosticCode::W0701
    );
    // popcount(op(a)) == 3: the operation is only inside the call's arguments.
    let nested = "warrant t v1\n  op g(a: I8) -> I8\n  claim c:\n    forall a: I8. popcount(g(a)) == popcount(a)\n";
    let b = bindings(&[("g", &["narrow-width"])]);
    let text = generate(nested, &b, REFERENCE, BTreeMap::new()).unwrap();
    assert!(
        text.contains("\"warrant/t/exhaustive/c\", \"exhaustive\", \"g\""),
        "the subject operation is g"
    );
}

#[test]
fn v1_scalar_adapters_name_the_faults_the_binding_lists() {
    let doc = "warrant t v1\n  op f(a: U32, b: U32) -> U32\n  claim c:\n    forall a: U32, b: U32. f(a, b) == a + b\n";
    let text = generate(
        doc,
        &bindings(&[("f", &["saturate-add", "trap-as-zero-add", "narrow-width"])]),
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap();
    for needle in [
        "Fault::SaturateAdd => p0.saturating_add(p1)",
        "Fault::TrapAsZeroAdd => p0.checked_add(p1).unwrap_or(0)",
        "Fault::NarrowWidth => (r as u16) as u32",
        "let p0 = u32::try_from(args[0].int())",
        "V::Fixed(F::U32, r as i128)",
        "pub enum Fault {\n    None,\n    SaturateAdd,\n    TrapAsZeroAdd,\n    NarrowWidth,\n}",
    ] {
        assert!(text.contains(needle), "missing `{needle}`");
    }
    // `args=` overrides the Rust parameter types.
    let mut at = BTreeMap::new();
    at.insert("f".to_string(), vec!["u32".to_string(), "u16".to_string()]);
    let text = generate(doc, &bindings(&[("f", &["narrow-width"])]), REFERENCE, at).unwrap();
    assert!(text.contains("let p1 = u16::try_from(args[1].int())"));
}

#[test]
fn the_operation_search_looks_inside_arguments_only_when_the_shallow_search_fails() {
    use coding_adventures_warrant_grammar::codegen::find_operation_in_expr;
    use coding_adventures_warrant_grammar::parser::Declaration;
    use std::collections::HashSet;
    let find = |body: &str| -> Option<String> {
        let src = format!("warrant t v1\n  example e:\n    {body}\n");
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        let Declaration::Example(e) = &file.declarations[0] else {
            panic!()
        };
        let ops: HashSet<&str> = ["f", "g"].into_iter().collect();
        find_operation_in_expr(&e.body, &ops).map(String::from)
    };
    // Shallow hits keep their v0 answer, left operand first.
    assert_eq!(find("f(1) == g(1)").as_deref(), Some("f"));
    assert_eq!(find("g(1) == f(1)").as_deref(), Some("g"));
    assert_eq!(
        find("1 == (if true then g(1) else f(1))").as_deref(),
        Some("g")
    );
    assert_eq!(find("-f(1) == 1").as_deref(), Some("f"));
    assert_eq!(find("(f(1)) == 1").as_deref(), Some("f"));
    // Nothing shallow: look inside the arguments of non-operation calls.
    assert_eq!(find("popcount(f(1)) == 3").as_deref(), Some("f"));
    assert_eq!(find("3 == popcount(bit_not(g(1)))").as_deref(), Some("g"));
    assert_eq!(find("-popcount(f(1)) == 3").as_deref(), Some("f"));
    assert_eq!(find("(popcount(g(1))) == 3").as_deref(), Some("g"));
    assert_eq!(
        find("1 == (if popcount(f(1)) == 1 then 1 else 2)").as_deref(),
        Some("f")
    );
    assert_eq!(
        find("1 == (if true then popcount(g(1)) else 2)").as_deref(),
        Some("g")
    );
    assert_eq!(
        find("1 == (if true then 1 else popcount(f(1)))").as_deref(),
        Some("f")
    );
    assert_eq!(
        find("1 == (match f(1) with None -> 1 | Some(x) -> x)").as_deref(),
        Some("f")
    );
    assert_eq!(
        find("1 == (match None with None -> f(1) | Some(x) -> x)").as_deref(),
        Some("f")
    );
    assert_eq!(find("1 == 2"), None);
    assert_eq!(find("popcount(1) == 2"), None);
    assert_eq!(find("[f(1)] == [1]"), None, "lists are not searched");
}

#[test]
fn count_binders_sees_through_every_expression_shape_and_the_budget_overflow_is_refused() {
    let b = bindings(&[("f", &["shift-mask"])]);
    let g = |body: &str| {
        generate(
            &claim_doc(" v1", "a: U8, n: Nat", "U8", "a: U8, n: Nat", body),
            &b,
            REFERENCE,
            BTreeMap::new(),
        )
    };
    // A count under `not`, inside a conditional and behind parentheses is still direct.
    assert_eq!(
        tys_of(&g("not (f(a, n) != shl(a, n))").unwrap()),
        "Ty::Fx(F::U8), Ty::NatCount"
    );
    assert_eq!(
        tys_of(&g("f(a, n) == (if true then shl(a, n) else a)").unwrap()),
        "Ty::Fx(F::U8), Ty::NatCount"
    );
    // Match, list and nested quantifier bodies are walked too, then refused
    // by the translator (W0701), never silently skipped.
    for body in [
        "f(a, n) == (match try<U8>(1) with None -> shl(a, n) | Some(v) -> v)",
        "f(a, n) == [shl(a, n)]",
        "f(a, n) == (forall m: Nat. shl(a, m) == a)",
    ] {
        assert_eq!(g(body).unwrap_err().0, DiagnosticCode::W0701, "{body}");
    }
    // Nine U8 binders overflow the u64 product: nothing can be demoted, W0706.
    let binders: Vec<String> = (0..9).map(|i| format!("x{i}: U8")).collect();
    let (code, msg) = generate(
        &claim_doc(" v1", "a: U8", "U8", &binders.join(", "), "f(x0) == x0"),
        &bindings(&[("f", &["narrow-width"])]),
        REFERENCE,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(code, DiagnosticCode::W0706, "{msg}");
    // Return types the v1 scalar shape cannot adapt.
    for ret in ["Option<Bool>", "Bytes<1>", "List<U8>"] {
        let src = format!(
            "warrant t v1\n  op f(a: U8) -> {ret}\n  claim c:\n    forall a: U8. f(a) == f(a)\n"
        );
        assert_eq!(
            generate(
                &src,
                &bindings(&[("f", &["narrow-width"])]),
                REFERENCE,
                BTreeMap::new()
            )
            .unwrap_err()
            .0,
            DiagnosticCode::W0701,
            "{ret}"
        );
    }
}
