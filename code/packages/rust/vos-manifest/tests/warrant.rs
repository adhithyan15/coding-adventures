//! What the Warrant tools say about a document: `claim_sha256`, the trailer, the
//! `bound` rows and the shape of `build` (the cone of VOS02 R6).

use vos_manifest::warrant::{
    analyze_document, claims_of_block, AssumeKind, BuildShape, WarrantStage,
};

const DOC: &str = "warrant t v1
  record M(a: Bool, b: Bool)
  record Impl(x: U8, y: U8)
  model build(m: M) -> Impl = (
    Impl(if m.a then 1u8 else 0u8,
         if m.b then 2u8 else 0u8))
  claim c1:
    forall v: Bool. v == v
  -- a comment
  claim c2:
    forall v: Bool, w: Bool. (
      v == v and
      w == w)
  assume platform:
    none
  assume environment:
    none
";

fn sha(tokens: &str) -> String {
    vos_manifest::digest::sha256_hex(tokens.as_bytes())
}

#[test]
fn claim_sha256_is_the_hash_of_the_space_joined_real_tokens() {
    let a = analyze_document(DOC).expect("type checks");
    assert_eq!(a.claims.len(), 2);
    assert_eq!(a.claims[0].name, "c1");
    // Independent definition: the tokens of the declaration, joined by single spaces.
    assert_eq!(
        a.claims[0].sha256,
        sha("claim c1 : forall v : Bool . v == v")
    );
    assert_eq!(
        a.claims[1].sha256,
        sha("claim c2 : forall v : Bool , w : Bool . ( v == v and w == w )")
    );
    assert!(vos_manifest::digest::is_hex64(&a.claims[0].sha256));
}

#[test]
fn comments_and_layout_do_not_change_a_digest_but_a_token_does() {
    let base = analyze_document(DOC).unwrap().claims;
    // A comment, re-wrapping, extra blank lines and different indentation inside the claim.
    let rewrapped = DOC
        .replace("  -- a comment\n", "  -- a different comment\n\n")
        .replace(
            "      v == v and\n      w == w)",
            "v == v\n   and\n          w == w)",
        );
    let again = analyze_document(&rewrapped).expect(&rewrapped).claims;
    assert_eq!(base, again);
    // A changed token: `==` to `!=`.
    let weaker = DOC.replace("forall v: Bool. v == v", "forall v: Bool. v != v");
    let w = analyze_document(&weaker).unwrap().claims;
    assert_ne!(base[0].sha256, w[0].sha256);
    assert_eq!(base[1], w[1], "the other claim is untouched");
    // A renamed claim.
    let renamed = analyze_document(&DOC.replace("claim c1", "claim c9"))
        .unwrap()
        .claims;
    assert_ne!(base[0].sha256, renamed[0].sha256);
}

#[test]
fn the_trailer_and_bound_rows_are_reported() {
    let a = analyze_document(DOC).unwrap();
    assert_eq!(a.assumptions.len(), 2);
    assert_eq!(a.assumptions[0].kind, AssumeKind::Platform);
    assert!(a.assumptions.iter().all(|r| r.is_none));
    assert_eq!(a.assumptions[0].start, DOC.find("assume platform").unwrap());
    assert_eq!(
        a.assumptions[1].start,
        DOC.find("assume environment").unwrap()
    );
    assert_eq!(a.bound_declarations, 0);
    assert_eq!(a.impl_fields, Some(2));
    // A non-`none` platform row is visible.
    let some = DOC.replace(
        "assume platform:\n    none",
        "assume platform:\n    contract.x.y",
    );
    // A platform row that is not `none` is reported as such (vos-link then refuses it).
    let a = analyze_document(&some).expect("the checker accepts a qualified platform row");
    assert!(!a.assumptions[0].is_none && a.assumptions[1].is_none);
}

#[test]
fn build_shape_reports_which_fields_are_in_the_cone() {
    let a = analyze_document(DOC).unwrap();
    assert_eq!(
        a.build,
        BuildShape::Constructor {
            in_cone: vec![true, true]
        }
    );
    // An oracle field set from a constant is outside the cone.
    let any = DOC.replace("if m.b then 2u8 else 0u8", "0u8");
    let a = analyze_document(&any).unwrap();
    assert_eq!(
        a.build,
        BuildShape::Constructor {
            in_cone: vec![true, false]
        }
    );
    // No build model.
    let none = DOC
        .replace("  model build(m: M) -> Impl = (\n    Impl(if m.a then 1u8 else 0u8,\n         if m.b then 2u8 else 0u8))\n", "");
    assert_eq!(analyze_document(&none).unwrap().build, BuildShape::Missing);
    // A build that is not a constructor application.
    let helper = DOC
        .replace(
            "    Impl(if m.a then 1u8 else 0u8,\n         if m.b then 2u8 else 0u8))",
            "    h(m))",
        )
        .replace(
            "  claim c1:",
            "  model h(m: M) -> Impl = Impl(1u8, 2u8)\n  claim c1:",
        );
    assert_eq!(
        analyze_document(&helper).unwrap().build,
        BuildShape::NotConstructor
    );
    // build with no parameter name to look for cannot occur (the parser requires one),
    // but a `build` model whose body mentions m only inside a nested call still counts.
    let nested = DOC.replace(
        "if m.a then 1u8 else 0u8",
        "bit_or(1u8, if m.a then 1u8 else 0u8)",
    );
    assert_eq!(
        analyze_document(&nested).unwrap().build,
        BuildShape::Constructor {
            in_cone: vec![true, true]
        }
    );
}

#[test]
fn the_tools_refusals_carry_their_stage_and_line() {
    let parse =
        analyze_document("warrant t v1\n  claim c:\n    forall v: Bool. (v ==\n").unwrap_err();
    assert_eq!(parse.stage, WarrantStage::Parse);
    // A type error: the claim compares a Bool with a U8.
    let check =
        analyze_document("warrant t v1\n  claim c:\n    forall v: Bool. v == 1u8\n").unwrap_err();
    assert_eq!(check.stage, WarrantStage::Check);
    // A duplicate declaration (NC2's other half).
    let dup = analyze_document("warrant t v1\n  claim c:\n    forall v: Bool. v == v\n  claim c:\n    forall v: Bool. v == v\n").unwrap_err();
    assert!(
        matches!(dup.stage, WarrantStage::Parse | WarrantStage::Check),
        "{dup}"
    );
    assert!(dup.to_string().len() > 4);
    // A parse refusal that knows its line says so.
    assert!(parse.line.is_some() || parse.stage == WarrantStage::Parse);
}

#[test]
fn a_block_of_claims_is_read_and_anything_else_is_refused() {
    let c = claims_of_block("  claim a:\n    forall v: Bool. v == v\n  -- remark\n  claim b:\n    forall v: Bool. v == v\n")
        .unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].sha256, sha("claim a : forall v : Bool . v == v"));
    assert_ne!(c[0].sha256, c[1].sha256, "the name is part of the claim");
    for other in [
        "  model m() -> Bool = true\n",
        "  op o(x: U8) -> U8\n",
        "  error E = A | B\n",
        "  variant V = A | B\n",
        "  labels o:\n    x public\n",
        "  input o.x:\n    hostile\n    ensures no_panic\n",
        "  bound o:\n    steps <= 1\n",
        "  assume environment:\n    none\n",
        "  record R(a: Bool)\n",
        "  assume platform:\n    none\n",
        "  claim a:\n    forall v: Bool. v == v\n  example e:\n    true\n",
    ] {
        let e = claims_of_block(other).unwrap_err();
        assert_eq!(e.stage, WarrantStage::NotClaims, "{other:?}");
        assert!(e.line.is_some());
    }
    assert_eq!(
        claims_of_block("  claim a:\n    forall v: (\n")
            .unwrap_err()
            .stage,
        WarrantStage::Parse
    );
    // A block with no declaration at all is not a document: the Warrant parser refuses
    // it, so an obligations part cannot be empty or all comments.
    assert_eq!(
        claims_of_block("  -- nothing\n").unwrap_err().stage,
        WarrantStage::Parse
    );
}

/// Every obligation of the real scheduler contract and its `claim_sha256`.  The digests were
/// produced by `tests/claim_sha256_reference.py` (an independent Python implementation of
/// D-GD7), not by this crate: if one changes, the definition or the obligation changed, and
/// every generated document and lock must be regenerated.
const PINS: [(&str, &str); 24] = [
    (
        "insert_safe",
        "ef3d7c1760812044ae6d87dfa9c58d03c123d61cdae97efe9e613ad1c3a39608",
    ),
    (
        "remove_safe",
        "43a0b03aad0cac17785b863d60be471bec79c92a052f07adb0cd70a545e31681",
    ),
    (
        "dispatch_safe",
        "e1fdce589522f3b94694ddf766e14d03eed74345c9f0aa11ad7933e218b1d0c4",
    ),
    (
        "safety_alpha_build",
        "a0df76dc929173876840307bf07e899349668e89f31e66fdc85c9987fe90d039",
    ),
    (
        "safety_init",
        "df7b6df1665d941d6c6b742162fdc69f8ca84c30ca0ddc968c04237e6638b931",
    ),
    (
        "dispatch_policy",
        "12e0986f42682a2996942b8ec7184497175bb8adcdf346b817f2b5f994c08012",
    ),
    (
        "canon_init",
        "1bd4fbf8b09c1f816bd67f9dab4ac31d38182cf909bd5dea4994273a7e169be5",
    ),
    (
        "canon_insert_post",
        "b9388ecf2d02e438c7bfe24f9d6a6b7a7d9f60cec9b692ec44befa99f0efc19d",
    ),
    (
        "canon_insert_img",
        "a575a09243e81f632031324505b5df3ab79539a168deea70a2632d58e0de29e4",
    ),
    (
        "canon_remove_post",
        "57758c20a5a1f56b0bb16094ab3a324679f2dc82c5a95fb494c4c3cb5108d2fc",
    ),
    (
        "canon_remove_img",
        "339afa8f826ab8d5f05584a5c62d88040ba15dd1b3c61cbd5a8171a1cf286d1d",
    ),
    (
        "canon_dispatch_post",
        "149af3ce7f315c0c4cc25db06947fe2636e0e2c2f7a51cf873c2b6639cf72393",
    ),
    (
        "canon_dispatch_img",
        "96413e700b67c2872d453da4d4ed988fda1db260dc9c6d230029f4a4a0821a42",
    ),
    (
        "dispatch_sound",
        "a7e01251153fa1dd77ea90a977193f9787fd052eb7949fef49197fda66214012",
    ),
    (
        "dispatch_covers_run",
        "fc7eb1b7a105aacc06fab5582206bca407535bd24fd7ec622a31b813670d9015",
    ),
    (
        "dispatch_covers_idle",
        "8ddfda72b86548a2c72eab4e22ab1d7b7021c5c2b32717ccbb0b1c2ce610ba5c",
    ),
    (
        "rep_init",
        "5a34bfc3544c12d40b65ffc6e8895bf99656166dd4ad69fa80e8eaf8ae42e672",
    ),
    (
        "rep_alpha",
        "bfeb535c6b8db440a32e55eee2fb675d8f5f50ef41c5d03d4fc984a276ee3d27",
    ),
    (
        "insert_rep_lo",
        "9e44b1d1eec14c115c6a5c2cb486ae492ba99adfa6702ff2a345c6dadbfc321e",
    ),
    (
        "insert_rep_hi",
        "86e0bd8dade4afd4125a95821ba62a4d99ef61d6379ff5192a7502d63b77cfc7",
    ),
    (
        "remove_rep_lo",
        "a67ac1ed645638879769104a688dfc16966f022bab4103b7c7dae8622a4ea731",
    ),
    (
        "remove_rep_hi",
        "d6a7b75ce631571f05f275369c02760e5f906a330bf687528a4bbcc9e9d16603",
    ),
    (
        "dispatch_rep",
        "2b3d7382d309cac6e96c74947b92ecb1bb09b7d1cfc95adbb95a7d8ff7a6399e",
    ),
    (
        "dispatch_rep_post",
        "e545ab30f35c853334f3113bba9995687b2b4ee3188aa7b920469b05589cbe43",
    ),
];

#[test]
fn the_real_obligation_blocks_have_the_expected_claims_and_pinned_digests() {
    let blocks = [
        (
            "safety",
            include_str!("../../vos-sched-contract/warrant/obligations-safety.part"),
            5usize,
        ),
        (
            "policy",
            include_str!("../../vos-sched-contract/warrant/obligations-policy.part"),
            1,
        ),
        (
            "canonical",
            include_str!("../../vos-sched-contract/warrant/obligations-canonical.part"),
            7,
        ),
        (
            "any",
            include_str!("../../vos-sched-contract/warrant/obligations-any.part"),
            3,
        ),
        (
            "representation",
            include_str!("../../vos-sched-contract/warrant/obligations-representation.part"),
            8,
        ),
    ];
    let mut seen = std::collections::BTreeMap::new();
    for (tier, text, n) in blocks {
        let claims = claims_of_block(text).unwrap_or_else(|e| panic!("{tier}: {e}"));
        assert_eq!(claims.len(), n, "{tier}");
        for c in claims {
            assert!(
                seen.insert(c.name.clone(), c.sha256).is_none(),
                "unique across tiers: {}",
                c.name
            );
        }
    }
    assert_eq!(seen.len(), PINS.len());
    for (name, pin) in PINS {
        let got = seen
            .get(name)
            .unwrap_or_else(|| panic!("no obligation {name}"));
        assert_eq!(got, pin, "{name}");
    }
}

#[test]
fn a_changed_literal_a_changed_name_or_a_quoted_word_changes_the_digest() {
    let d = |text: &str| {
        claims_of_block(text).unwrap_or_else(|e| panic!("{text}: {e}"))[0]
            .sha256
            .clone()
    };
    let base = d("  claim a:\n    forall v: U8. v == 1u8\n");
    assert_ne!(
        base,
        d("  claim a:\n    forall v: U8. v == 2u8\n"),
        "1u8 to 2u8"
    );
    assert_ne!(base, d("  claim a:\n    forall v: U16. v == 1u8\n"));
    assert_ne!(base, d("  claim b:\n    forall v: U8. v == 1u8\n"));
    // A bytes literal is hashed as written, `b"00"` with its prefix and quotes, not as the
    // text the lexer keeps inside them (S-I1: `"x"` and `x` must not hash alike).
    assert_eq!(
        d("  claim a:\n    b\"00\" == b\"00\"\n"),
        sha("claim a : b\"00\" == b\"00\"")
    );
    assert_ne!(
        d("  claim a:\n    b\"00\" == b\"00\"\n"),
        d("  claim a:\n    b\"01\" == b\"00\"\n")
    );
    // ... while whitespace and comments still do not matter.
    assert_eq!(
        base,
        d("  claim a:   -- c\n    forall   v: U8.   v == 1u8   -- d\n")
    );
}

#[test]
fn the_taint_walk_does_not_count_a_mention_that_cannot_change_the_value() {
    // The R6 cone rule (VOS02 10.1): a field of `Impl` is in the cone of `build` when its
    // value can depend on `m`.  Each variant below mentions `m` in the second field and
    // must still be reported OUTSIDE the cone; the honest forms must be inside.
    let doc = |field: &str, extra: &str| {
        DOC.replace("if m.b then 2u8 else 0u8", field)
            .replace("  claim c1:", &format!("{extra}  claim c1:"))
    };
    let cone = |field: &str, extra: &str| match analyze_document(&doc(field, extra))
        .unwrap_or_else(|e| panic!("{field}: {e}"))
        .build
    {
        BuildShape::Constructor { in_cone } => in_cone[1],
        other => panic!("{other:?}"),
    };
    // Honest.
    assert!(cone("if m.b then 2u8 else 0u8", ""));
    assert!(cone("bit_and(if m.b then 2u8 else 0u8, 255u8)", ""));
    assert!(
        cone("bit_or(if m.b then 2u8 else 0u8, 0u8)", ""),
        "`or` with zero keeps the value"
    );
    assert!(cone(
        "h(m.b)",
        "  model h(x: Bool) -> U8 = if x then 2u8 else 0u8\n"
    ));
    // Evasions the walk must now see through.
    assert!(
        !cone("bit_and(if m.b then 2u8 else 0u8, 0u8)", ""),
        "bit_and with 0"
    );
    assert!(
        !cone("bit_and(0u8, if m.b then 2u8 else 0u8)", ""),
        "bit_and with 0, swapped"
    );
    assert!(!cone("(if m.b then 2u8 else 0u8) * 0u8", ""), "times 0");
    assert!(!cone("0u8 * (if m.b then 2u8 else 0u8)", ""), "0 times");
    assert!(
        !cone(
            "(if m.b then 2u8 else 0u8) - (if m.b then 2u8 else 0u8)",
            ""
        ),
        "x - x"
    );
    assert!(
        !cone(
            "bit_xor(if m.b then 2u8 else 0u8, if m.b then 2u8 else 0u8)",
            ""
        ),
        "x xor x"
    );
    assert!(
        !cone("h(m.b)", "  model h(x: Bool) -> U8 = 0u8\n"),
        "an argument the model ignores"
    );
    assert!(
        !cone(
            "g(m.b)",
            "  model h(x: Bool) -> U8 = 0u8\n  model g(y: Bool) -> U8 = h(y)\n"
        ),
        "an argument ignored two calls down"
    );
    assert!(!cone("0u8", ""));
    // `and` with false is a Bool constant: use it in a Bool field of a small doc.
    let b = "warrant t v1\n  record M(a: Bool)\n  record Impl(x: Bool)\n  model build(m: M) -> Impl = (Impl(m.a and false))\n";
    assert_eq!(
        analyze_document(b).unwrap().build,
        BuildShape::Constructor {
            in_cone: vec![false]
        }
    );
    let b2 = b.replace("m.a and false", "m.a and true");
    assert_eq!(
        analyze_document(&b2).unwrap().build,
        BuildShape::Constructor {
            in_cone: vec![true]
        }
    );
}

/// `build` over a record with one field per expression form, to walk every arm of the
/// taint walk.  The abstract state is `M(a: Bool, b: U8)`.
fn cone_of(fields: &[(&str, &str)], extra: &str) -> Vec<bool> {
    let ty: Vec<String> = fields
        .iter()
        .enumerate()
        .map(|(i, (t, _))| format!("f{i}: {t}"))
        .collect();
    let args: Vec<&str> = fields.iter().map(|(_, e)| *e).collect();
    let doc = format!(
        "warrant t v1\n  record M(a: Bool, b: U8)\n  record Impl({})\n{extra}  model build(m: M) -> Impl = (Impl({}))\n",
        ty.join(", "),
        args.join(", ")
    );
    match analyze_document(&doc)
        .unwrap_or_else(|e| panic!("{e}\n{doc}"))
        .build
    {
        BuildShape::Constructor { in_cone } => in_cone,
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_expression_form_is_walked() {
    let got = cone_of(
        &[
            ("U8", "if m.a then m.b else 0u8"),
            ("U8", "if true then 1u8 else 0u8"),
            ("Bool", "not m.a"),
            ("Bool", "m.b == 1u8 or false"),
            ("Bool", "true"),
            ("U8", "match m.a with true -> m.b | false -> 0u8"),
            ("U8", "match true with true -> 1u8 | false -> 0u8"),
            ("(U8, Bool)", "(m.b, true)"),
            ("(U8, Bool)", "(1u8, true)"),
            ("U8", "(m.b, true).0"),
            // Projection is field-insensitive: a tuple with one tainted component taints
            // every projection of it (an accepted imprecision, listed in VOS02 10.1).
            ("U8", "(1u8, m.b).0"),
            ("Bool", "m.a and false"),
            ("Bool", "false and m.a"),
            ("U8", "m.b + 1u8"),
            ("U8", "(m.b)"),
        ],
        "",
    );
    assert_eq!(
        got,
        [
            true, false, true, true, false, true, false, true, false, true, true, false, false,
            true, true
        ]
    );
}

#[test]
fn a_model_call_chain_is_followed_to_its_end() {
    // m0 -> m1 -> ... : the argument reaches a body that uses it at the end.
    let chain = |n: usize| -> String {
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&format!("  model k{i}(x: U8) -> U8 = k{}(x)\n", i + 1));
        }
        s.push_str(&format!("  model k{n}(x: U8) -> U8 = x\n"));
        s
    };
    assert_eq!(cone_of(&[("U8", "k0(m.b)")], &chain(5)), [true]);
    // Per-model summaries have no depth limit: a long chain is followed all the way.
    assert_eq!(cone_of(&[("U8", "k0(m.b)")], &chain(40)), [true]);
    // A call with more arguments than the model has parameters, or a multi-segment call.
    assert_eq!(
        cone_of(&[("U8", "bit_and(m.b, 255u8)")], ""),
        [true],
        "a built-in with a non-zero operand keeps the dependence"
    );
}

#[test]
fn a_refusal_with_a_source_position_prints_its_line() {
    let e = analyze_document("warrant t v1\n\tclaim c:\n    true\n").unwrap_err();
    assert_eq!(e.line, Some(2));
    assert!(e.to_string().starts_with("line 2: "), "{e}");
    let c = claims_of_block("  model m() -> Bool = true\n").unwrap_err();
    assert!(c.to_string().starts_with("line 2: "), "{c}");
}

/// The round-2 reviewer's input: f_i(x) = bit_or(f_{i+1}(x) x FANOUT), f_D(x) = 0u8, with the
/// oracle field `f_1(m.b)`.  Following every call made this 3^depth.
fn fanout_chain(depth: usize, fanout: usize) -> String {
    let mut s = String::new();
    for i in 1..depth {
        let call = format!("f{}(x)", i + 1);
        let mut e = call.clone();
        for _ in 1..fanout {
            e = format!("bit_or({e}, {call})");
        }
        s.push_str(&format!("  model f{i}(x: U8) -> U8 = {e}\n"));
    }
    s.push_str(&format!("  model f{depth}(x: U8) -> U8 = 0u8\n"));
    s
}

#[test]
fn a_fanout_chain_is_polynomial_not_exponential() {
    // N1: depth 18, fanout 6 would be 6^18 calls without the memo.
    let t = std::time::Instant::now();
    let got = cone_of(&[("U8", "f1(m.b)")], &fanout_chain(18, 6));
    let took = t.elapsed();
    assert_eq!(got, [false], "the chain ends in the constant 0u8");
    assert!(took < std::time::Duration::from_secs(5), "took {took:?}");
    // Work grows linearly with depth: depth 4n against n, compared as a ratio.
    let time = |d: usize| {
        let t = std::time::Instant::now();
        let _ = cone_of(&[("U8", "f1(m.b)")], &fanout_chain(d, 3));
        t.elapsed().as_secs_f64()
    };
    let small = time(8).max(0.002);
    let big = time(32);
    assert!(
        big < 40.0 * small,
        "depth 32 took {big}s against {small}s at depth 8"
    );
}

#[test]
fn a_walk_that_runs_out_of_budget_gives_up_instead_of_answering() {
    let doc = format!(
        "warrant t v1\n  record M(a: Bool, b: U8)\n  record Impl(f0: U8)\n{}  model build(m: M) -> Impl = (Impl(f1(m.b)))\n",
        fanout_chain(10, 2)
    );
    let ok = vos_manifest::warrant::analyze_document(&doc).unwrap();
    assert_eq!(
        ok.build,
        BuildShape::Constructor {
            in_cone: vec![false]
        }
    );
    let tiny = vos_manifest::warrant::analyze_document_with_budget(&doc, 10).unwrap();
    assert_eq!(tiny.build, BuildShape::BudgetExhausted);
    assert_eq!(vos_manifest::warrant::TAINT_BUDGET, 1_000_000);
}

#[test]
fn a_recursive_model_summary_is_conservative() {
    // Warrant has no recursion, so the checker refuses this before the walk could see it;
    // the guard is for the day it does not.
    let doc = "warrant t v1\n  record M(a: Bool, b: U8)\n  record Impl(f0: U8)\n  model r(x: U8) -> U8 = r(x)\n  model build(m: M) -> Impl = (Impl(r(m.b)))\n";
    let e = analyze_document(doc).unwrap_err();
    assert_eq!(e.stage, WarrantStage::Check, "{e}");
}

#[test]
fn a_model_with_more_than_63_parameters_is_refused_not_approximated() {
    // The reviewer's input: w(p0..p69) depends only on p63, called with m.b at index 69.
    // With shared high bits that read as "p63 is the argument that matters" and hid the
    // dependence the other way round; the walk now gives up (fail closed).
    let wide = |n: usize| {
        let params: Vec<String> = (0..n).map(|i| format!("p{i}: Bool")).collect();
        let mut args = vec!["m.a".to_string(); n];
        args[n - 1] = "m.a".to_string();
        format!(
            "warrant t v1\n  record M(a: Bool, b: U8)\n  record Impl(f0: U8)\n  model w({}) -> U8 = if p63 then 1u8 else 0u8\n  model build(m: M) -> Impl = (Impl(w({})))\n",
            params.join(", "),
            args.join(", ")
        )
    };
    let a = analyze_document(&wide(70));
    match a {
        Ok(a) => assert_eq!(a.build, BuildShape::WideModel),
        Err(e) => panic!("{e}"),
    }
    // 63 parameters are still told apart (indices 0..=62): p62 is the one used here.
    let ok = wide(63).replace("if p63 then", "if p62 then");
    assert!(matches!(
        analyze_document(&ok).unwrap().build,
        BuildShape::Constructor { .. }
    ));
}

#[test]
fn declared_names_lists_claims_and_examples_in_document_order() {
    // VOS-039a: parsed only (no type check), claims and examples kept apart.
    let doc = "warrant t v1
  record Impl(x: U8)
  claim c2:
    forall v: Bool. v == v
  example e1:
    1u8 == 1u8
  claim c1:
    forall v: Bool. v == v
";
    let (claims, examples) = vos_manifest::warrant::declared_names(doc).unwrap();
    assert_eq!(claims, ["c2", "c1"]);
    assert_eq!(examples, ["e1"]);
    assert!(vos_manifest::warrant::declared_names("not a document").is_err());
}

#[test]
fn claim_calls_walks_the_parsed_body_so_strings_and_comments_hide_nothing() {
    // VOS-039a review M3: the call is found in the tree.  Warrant's only string-like literal is
    // the hex `b".."` (it cannot hold `--`), but the earlier text scan cut every line at the first
    // `--`; a tree has no comments and no literals to mislead it.
    let doc = "warrant t v1
  record Impl(x: U8)
  claim c1:
    forall v: Bool. v == v -- insert(1) in a comment
  claim c2:
    forall v: Bool. b\"00\" == b\"00\" and insert (v) == dispatch(v) and insertx(v) == v
  claim c3:
    forall v: Bool. if v then (b\"00\", unused(v)) == (b\"00\", v) else dispatch(insert(v)) == v
";
    let ops = ["insert".to_string(), "dispatch".to_string(), "unused".to_string()];
    let got = vos_manifest::warrant::claim_calls(doc, &ops).unwrap();
    assert_eq!(got[0], ("c1".to_string(), vec![]));
    assert_eq!(got[1], ("c2".to_string(), vec!["insert".to_string(), "dispatch".to_string()]));
    assert_eq!(got[2].1, ["unused", "dispatch", "insert"]);
}

// ---------------------------------------------------------------------------
// declarations_of_block (VOS-039b-1): kinds, names and spans IN THE BLOCK
// ---------------------------------------------------------------------------

#[test]
fn declarations_of_block_reports_kind_name_and_span_in_the_block() {
    use vos_manifest::warrant::{declarations_of_block, DeclKind};
    let block = "  -- a comment\n  model one(x: U8) -> U8 = x\n  claim c:\n    forall b: Bool. b == b\n  record R(a: Bool)\n";
    let d = declarations_of_block(block).expect("parses");
    assert_eq!(d.len(), 3);
    assert_eq!(
        (d[0].kind, d[0].name.as_str(), d[0].has_body),
        (DeclKind::Model, "one", true)
    );
    assert_eq!(&block[d[0].start..d[0].end], "model one(x: U8) -> U8 = x");
    assert_eq!((d[1].kind, d[1].name.as_str()), (DeclKind::Claim, "c"));
    assert!(block[d[1].start..d[1].end].starts_with("claim c:"));
    assert_eq!((d[2].kind, d[2].name.as_str()), (DeclKind::Other, ""));
    assert!(block[d[2].start..d[2].end].starts_with("record R"));
}

#[test]
fn declarations_of_block_refuses_text_the_parser_refuses_and_text_its_spans_would_not_index() {
    use vos_manifest::warrant::declarations_of_block;
    let e = declarations_of_block("  model broken(\n").unwrap_err();
    assert_eq!(e.stage, WarrantStage::Parse);
    // A CRLF line end is normalised away, so a span would not index the bytes: refused.
    let e = declarations_of_block("  model a() -> U8 = 1u8\r\n").unwrap_err();
    assert_eq!(e.stage, WarrantStage::Parse);
    assert!(e.message.contains("normalised"));
}

#[test]
fn claim_calls_finds_a_config_model_called_with_an_empty_argument_list() {
    // `cfg_N()` is a call with no arguments: check-contract's "each config key is used by a
    // lemma" rests on this.
    let doc = "warrant t v1\n  model cfg_N() -> U8 = 8u8\n  claim uses:\n    forall t: U8. (t < cfg_N())\n  claim ignores:\n    forall t: U8. (t == t)\n";
    let got = vos_manifest::warrant::claim_calls(doc, &["cfg_N".to_string()]).unwrap();
    assert_eq!(got[0], ("uses".to_string(), vec!["cfg_N".to_string()]));
    assert_eq!(got[1], ("ignores".to_string(), vec![]));
}
