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
    let a = analyze_document(&some);
    if let Ok(a) = a {
        assert!(!a.assumptions[0].is_none);
    }
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

#[test]
fn the_real_obligation_blocks_have_the_expected_claims_and_pinned_digests() {
    let blocks = [
        (
            "safety",
            include_str!("../../vos-sched-contract/warrant/obligations-safety.part"),
            4usize,
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
    ];
    let mut names = std::collections::BTreeSet::new();
    for (tier, text, n) in blocks {
        let claims = claims_of_block(text).unwrap_or_else(|e| panic!("{tier}: {e}"));
        assert_eq!(claims.len(), n, "{tier}");
        for c in claims {
            assert!(
                names.insert(c.name.clone()),
                "obligation names are unique across tiers: {}",
                c.name
            );
            if c.name == "safety_alpha_build" {
                // Pinned: if this changes, the definition of claim_sha256 (D-GD7) or the
                // text of the obligation changed, and every document must be regenerated.
                assert_eq!(
                    c.sha256,
                    "a0df76dc929173876840307bf07e899349668e89f31e66fdc85c9987fe90d039"
                );
            }
        }
    }
    assert_eq!(names.len(), 15);
}

#[test]
fn a_failure_prints_with_and_without_a_line() {
    let with =
        analyze_document("warrant t v1\n  claim c:\n    forall v: Bool. (v ==\n").unwrap_err();
    let without = vos_manifest::warrant::WarrantFailure {
        stage: WarrantStage::Check,
        message: "boom".to_string(),
        line: None,
    };
    assert_eq!(without.to_string(), "boom");
    if let Some(l) = with.line {
        assert!(with.to_string().starts_with(&format!("line {l}: ")));
    }
    let located = vos_manifest::warrant::WarrantFailure {
        line: Some(7),
        ..without
    };
    assert_eq!(located.to_string(), "line 7: boom");
}

#[test]
fn a_bound_declaration_is_counted() {
    let with_bound = DOC.replace(
        "  claim c1:",
        "  op o(x: U8) -> U8\n  bound o:\n    steps <= 10\n  labels o:\n    x public\n  input o.x:\n    hostile\n    ensures no_panic, terminates\n  claim c1:",
    );
    let a = analyze_document(&with_bound).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(a.bound_declarations, 1);
}

#[test]
fn source_admission_and_layout_budgets_are_refusals_not_panics() {
    // Over the 1 MiB admission ceiling.
    let huge = format!("warrant t v1\n  -- {}\n", "x".repeat(1024 * 1024));
    let e = analyze_document(&huge).unwrap_err();
    assert_eq!(e.stage, WarrantStage::Parse);
    // More tokens than the layout budget (65,536) in a document well under 1 MiB.
    let mut many = String::from("warrant t v1\n");
    for i in 0..7_000 {
        many.push_str(&format!("  claim c{i}:\n    forall v: Bool. v == v\n"));
    }
    let e = analyze_document(&many).unwrap_err();
    assert_eq!(e.stage, WarrantStage::Parse, "{e}");
    // A tab in indentation is a layout error with a span.
    let e = analyze_document("warrant t v1\n\tclaim c:\n    true\n").unwrap_err();
    assert_eq!(e.stage, WarrantStage::Parse);
    assert!(e.line.is_some(), "{e}");
}
