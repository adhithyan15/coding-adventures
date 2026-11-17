//! The negative controls of VOS02 section 12.2 that gen-doc and check-doc own: NC1, NC2,
//! NC4, NC4b, NC5, NC17 and the document-level half of NC18, plus the R6 structural rules
//! (kind, cone) and the checks on the inputs.  A control that passes is a defect of the
//! gate (VER04h section 6), so each test asserts the specific finding, not just "failed".

mod common;
use common::*;

use vos_manifest::digest::sha256_hex;

const BITMAP: &str = "vos-fixture-bitmap";
const SCAN: &str = "vos-fixture-scan";
const ANY: &str = "vos-fixture-any";

fn generated(label: &str, fixture: &str) -> Repo {
    let r = Repo::with(label, fixture);
    let g = r.gen(fixture);
    assert!(g.findings.is_empty(), "{}", show(&g));
    r
}

fn has(o: &vos_link::Outcome, code: &str, needle: &str) -> bool {
    o.findings
        .iter()
        .any(|f| f.code == code && f.message.contains(needle))
}

fn doc_path(name: &str) -> String {
    format!("code/specs/{name}.warrant")
}

// ---------------------------------------------------------------------------
// NC1: a hand edit inside the copied contract block
// ---------------------------------------------------------------------------

#[test]
fn nc1_an_edit_inside_the_contract_block_fails_the_digest_the_copy_and_regeneration() {
    let r = generated("nc1a", SCAN);
    r.edit(
        &doc_path(SCAN),
        "if t >= 8u8 then (OutOfRange, m)",
        "if t >= 9u8 then (OutOfRange, m)",
    );
    let c = r.check(SCAN);
    let got = codes(&c);
    assert!(
        got.contains(&"CD02"),
        "the recorded digest no longer matches: {}",
        show(&c)
    );
    assert!(
        got.contains(&"CD03"),
        "the copy differs from the contract's part: {}",
        show(&c)
    );
    assert!(got.contains(&"CD08"), "regeneration differs: {}", show(&c));
}

#[test]
fn nc1_an_edit_with_the_digest_recomputed_still_fails_the_copy_and_regeneration() {
    // The attacker also fixes the marker's digest: CD02 is silent, the others are not.
    let r = generated("nc1b", SCAN);
    let doc = r.doc(SCAN);
    let begin = doc.find("  -- vos02:contract-begin").unwrap();
    let body_start = doc[begin..].find('\n').unwrap() + begin + 1;
    let body_end = doc.find("  -- vos02:contract-end\n").unwrap();
    let old_body = &doc[body_start..body_end];
    let new_body = old_body.replacen(
        "if t >= 8u8 then (OutOfRange, m)",
        "if t >= 9u8 then (OutOfRange, m)",
        1,
    );
    assert_ne!(old_body, new_body);
    let forged = format!(
        "{}  -- vos02:contract-begin sha256={}\n{}{}",
        &doc[..begin],
        sha256_hex(new_body.as_bytes()),
        new_body,
        &doc[body_end..]
    );
    r.write(&doc_path(SCAN), forged);
    let c = r.check(SCAN);
    let got = codes(&c);
    assert!(!got.contains(&"CD02"), "{}", show(&c));
    assert!(
        got.contains(&"CD03") && got.contains(&"CD08"),
        "{}",
        show(&c)
    );
}

#[test]
fn nc1_an_edit_inside_an_obligations_block_fails_too() {
    let r = generated("nc1c", SCAN);
    r.edit(
        &doc_path(SCAN),
        "  claim insert_safe:",
        "  claim insert_safe:  -- edited",
    );
    let c = r.check(SCAN);
    assert!(
        has(&c, "CD02", "obligations block `safety`"),
        "{}",
        show(&c)
    );
    assert!(
        has(&c, "CD03", "obligations block `safety`"),
        "{}",
        show(&c)
    );
}

// ---------------------------------------------------------------------------
// NC2: an obligation weakened with the same name
// ---------------------------------------------------------------------------

const WEAK: &str = "  claim insert_safe:\n    forall b: Bool. b == b\n";

#[test]
fn nc2_a_weaker_claim_with_an_obligations_name_has_a_different_claim_sha256() {
    // The implementation block declares its own `insert_safe` and does not embed the
    // safety tier, so there is no duplicate declaration for the Warrant checker to find:
    // only the digest comparison sees it.
    let r = Repo::with("nc2a", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        "kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"canonical\"]\n",
    );
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    r.write(p, format!("{}{WEAK}", r.read(p)));
    let g = r.gen(SCAN);
    assert!(has(&g, "CD05", "`insert_safe`"), "{}", show(&g));
    assert!(g
        .findings
        .iter()
        .any(|f| f.message.contains("different claim_sha256")));
    assert!(!r.path(&doc_path(SCAN)).exists(), "nothing is written");
}

#[test]
fn nc2_the_same_name_twice_is_a_duplicate_declaration() {
    // The safety tier IS embedded and the implementation block also declares insert_safe.
    let r = Repo::with("nc2b", SCAN);
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    r.write(p, format!("{}{WEAK}", r.read(p)));
    let g = r.gen(SCAN);
    assert!(has(&g, "CD04", "warrant-grammar refused"), "{}", show(&g));
    assert!(
        has(&g, "CD04", "DuplicateTestName"),
        "rejected for the duplicate declaration, not an unrelated slip: {}",
        show(&g)
    );
}

#[test]
fn nc2_a_weakened_claim_in_the_obligations_block_of_the_document_on_disk() {
    let r = generated("nc2c", SCAN);
    let doc = r.doc(SCAN);
    // Weaken the claim text but keep the digest marker and the name: the copy differs (CD03)
    // and the claim has the obligation's name with another claim_sha256 (CD05); the tier is
    // no longer fully carried (CD06).
    let weak = doc.replacen(
        "insert_out(build(ready_of(b0, b1, b2, b3, b4, b5, b6, b7)), t) == abs_insert(ready_of(b0, b1, b2, b3, b4, b5, b6, b7), t).0 and",
        "true and",
        1,
    );
    assert_ne!(doc, weak);
    r.write(&doc_path(SCAN), weak);
    let c = r.check(SCAN);
    let got = codes(&c);
    for want in ["CD02", "CD03", "CD05", "CD06", "CD08"] {
        assert!(got.contains(&want), "missing {want}: {}", show(&c));
    }
    assert!(has(&c, "CD06", "only partly carried"), "{}", show(&c));
    assert!(!carried(&c).contains(&"safety"));
}

// ---------------------------------------------------------------------------
// NC4, NC4b, NC5 and the structural rules of R6
// ---------------------------------------------------------------------------

fn copy_as(r: &Repo, from: &str, to: &str, toml: &str) {
    r.add_fixture(from);
    common::copy_dir(&r.pkg(from), &r.pkg(to));
    std::fs::remove_dir_all(r.pkg(from)).unwrap();
    r.write(&format!("code/packages/rust/{to}/vos.toml"), toml);
}

const TOML_HEAD: &str = "contract = \"vos-sched-contract\"\n";

#[test]
fn nc4_an_any_crate_declared_production_is_rejected_four_ways() {
    let r = Repo::with("nc4", ANY);
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"any\", \"safety\"]\n"),
    );
    let g = r.gen(ANY);
    assert!(
        has(&g, "CD07", "named `-any` declares kind = \"production\""),
        "{}",
        show(&g)
    );
    assert!(
        has(&g, "CD07", "carries an `any` obligation"),
        "{}",
        show(&g)
    );
    assert!(
        has(&g, "CD07", "does not carry the `canonical`"),
        "{}",
        show(&g)
    );
    assert!(has(&g, "CD07", "outside the cone"), "{}", show(&g));
    assert!(!r.path(&doc_path(ANY)).exists());
}

#[test]
fn nc4b_an_any_crate_whose_document_also_carries_canonical() {
    // It passes the canonical claims (VOS10 section 14), so only the rule stops it.
    let r = Repo::with("nc4b", ANY);
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        format!("kind = \"any\"\n{TOML_HEAD}obligations = [\"any\", \"canonical\", \"safety\"]\n"),
    );
    let g = r.gen(ANY);
    assert_eq!(codes(&g), ["CD07"], "{}", show(&g));
    assert!(
        has(
            &g,
            "CD07",
            "carries a `canonical` claim; an `any` crate may not"
        ),
        "{}",
        show(&g)
    );
    assert!(
        carried(&g).contains(&"canonical"),
        "the document really does carry it: {:?}",
        carried(&g)
    );
}

#[test]
fn nc5_a_production_crate_carrying_an_any_block() {
    // Not named -any, so the name lint is silent; the any obligations are carried and the
    // oracle field is outside the cone.
    let r = Repo::new("nc5");
    copy_as(
        &r,
        ANY,
        "vos-fixture-oracle",
        &format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"any\", \"canonical\", \"safety\"]\n"),
    );
    let g = r.gen("vos-fixture-oracle");
    assert!(
        has(&g, "CD07", "carries an `any` obligation"),
        "{}",
        show(&g)
    );
    assert!(has(&g, "CD07", "outside the cone"), "{}", show(&g));
    assert!(!has(&g, "CD07", "named `-any`"), "{}", show(&g));
}

#[test]
fn a_production_crate_with_an_oracle_field_outside_the_cone_is_rejected_by_structure() {
    // No any block, canonical carried, honest name: only the structural rule (R6 rule 2)
    // catches the register that `build` sets to a constant.
    let r = Repo::new("cone");
    copy_as(
        &r,
        ANY,
        "vos-fixture-oracle",
        &format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"canonical\", \"safety\"]\n"),
    );
    let g = r.gen("vos-fixture-oracle");
    assert_eq!(codes(&g), ["CD07"], "{}", show(&g));
    assert!(
        has(
            &g,
            "CD07",
            "field(s) 1 of `Impl` are not set from the abstract state"
        ),
        "{}",
        show(&g)
    );
    assert_eq!(carried(&g), ["canonical", "safety"]);
}

#[test]
fn the_same_register_set_from_the_abstract_state_is_inside_the_cone() {
    let r = Repo::new("cone-ok");
    copy_as(
        &r,
        ANY,
        "vos-fixture-oracle",
        &format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"canonical\", \"safety\"]\n"),
    );
    let p = "code/packages/rust/vos-fixture-oracle/warrant/impl.part";
    // choice = the same mask value: now a function of m.  (The claims still hold or fail
    // on their merits; the structural rule is what is under test.)
    r.edit(
        p,
        "b2m(m.p6, 6)), b2m(m.p7, 7)), 0u8))",
        "b2m(m.p6, 6)), b2m(m.p7, 7)), b2m(m.p0, 0)))",
    );
    let g = r.gen("vos-fixture-oracle");
    assert!(!has(&g, "CD07", "cone"), "{}", show(&g));
}

#[test]
fn a_build_that_is_not_a_constructor_application_fails_closed() {
    let r = Repo::with("build-helper", SCAN);
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    r.edit(
        p,
        "  model build(m: Ready) -> Impl = (\n    Impl(b2u(m.p0), b2u(m.p1), b2u(m.p2), b2u(m.p3), b2u(m.p4), b2u(m.p5), b2u(m.p6), b2u(m.p7)))\n",
        "  model mk(m: Ready) -> Impl = (\n    Impl(b2u(m.p0), b2u(m.p1), b2u(m.p2), b2u(m.p3), b2u(m.p4), b2u(m.p5), b2u(m.p6), b2u(m.p7)))\n  model build(m: Ready) -> Impl = mk(m)\n",
    );
    let g = r.gen(SCAN);
    assert!(
        has(&g, "CD07", "not an `Impl(..)` constructor application"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_production_crate_that_does_not_carry_canonical_is_rejected() {
    let r = Repo::with("no-canon", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen(SCAN);
    assert_eq!(codes(&g), ["CD07"], "{}", show(&g));
    assert!(has(
        &g,
        "CD07",
        "does not carry the `canonical` obligations"
    ));
}

#[test]
fn an_any_crate_without_the_any_block_or_the_name_is_rejected() {
    let r = Repo::with("any-no-block", ANY);
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        format!("kind = \"any\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen(ANY);
    assert!(
        has(&g, "CD07", "does not carry the `any` obligations"),
        "{}",
        show(&g)
    );
    let r = Repo::new("any-name");
    copy_as(
        &r,
        ANY,
        "vos-fixture-oracle",
        &format!("kind = \"any\"\n{TOML_HEAD}obligations = [\"any\", \"safety\"]\n"),
    );
    let g = r.gen("vos-fixture-oracle");
    assert!(has(&g, "CD07", "does not end in `-any`"), "{}", show(&g));
}

#[test]
fn an_any_crate_with_no_oracle_field_outside_the_cone_is_rejected() {
    // The scan Impl has eight fields, all set from the abstract state, so it has no oracle.
    let r = Repo::new("any-no-oracle");
    copy_as(
        &r,
        "vos-fixture-scan",
        "vos-fixture-x-any",
        &format!("kind = \"any\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen("vos-fixture-x-any");
    assert!(
        has(&g, "CD07", "no oracle field outside the cone"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_reference_model_has_no_bound_rows_and_no_any_name() {
    // The bitmap fixture has bound rows; declared a reference it is rejected.
    let r = Repo::with("ref-bound", BITMAP);
    r.write(
        "code/packages/rust/vos-fixture-bitmap/vos.toml",
        format!("kind = \"reference\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen(BITMAP);
    assert!(has(&g, "CD07", "`bound` declaration"), "{}", show(&g));
    let r = Repo::new("ref-any");
    copy_as(
        &r,
        "vos-fixture-ref",
        "vos-fixture-r-any",
        &format!("kind = \"reference\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen("vos-fixture-r-any");
    assert!(
        has(&g, "CD07", "named `-any` declares kind = \"reference\""),
        "{}",
        show(&g)
    );
}

#[test]
fn a_contract_without_the_needed_tier_cannot_satisfy_the_kind() {
    let r = Repo::with("no-tier", SCAN);
    let c = "code/packages/rust/vos-sched-contract";
    // Remove the canonical tier from contract.vos and its part.
    r.edit(&format!("{c}/contract.vos"), "tier canonical\n", "");
    std::fs::remove_file(r.path(&format!("{c}/warrant/obligations-canonical.part"))).unwrap();
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen(SCAN);
    assert!(has(&g, "CD07", "needs a `canonical` tier"), "{}", show(&g));
    let r = Repo::with("no-any-tier", ANY);
    let c = "code/packages/rust/vos-sched-contract";
    r.edit(&format!("{c}/contract.vos"), "tier any\n", "");
    std::fs::remove_file(r.path(&format!("{c}/warrant/obligations-any.part"))).unwrap();
    r.write(
        "code/packages/rust/vos-fixture-any/vos.toml",
        format!("kind = \"any\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let g = r.gen(ANY);
    assert!(has(&g, "CD07", "needs an `any` tier"), "{}", show(&g));
}

// ---------------------------------------------------------------------------
// NC17: markers, control characters
// ---------------------------------------------------------------------------

#[test]
fn nc17_a_forged_marker_in_impl_part_is_refused_by_gen_doc() {
    let h = "0".repeat(64);
    for (name, forged) in [
        ("exact end", "  -- vos02:impl-end\n".to_string()),
        (
            "exact begin",
            format!("  -- vos02:contract-begin sha256={h}\n"),
        ),
        (
            "indented",
            "          -- vos02:obligations-end\n".to_string(),
        ),
        (
            "after code",
            "  model zz(x: U8) -> U8 = x  -- vos02:impl-end\n".to_string(),
        ),
        (
            "in a string",
            "  example zz:\n    \"-- vos02:impl-end\" == \"\"\n".to_string(),
        ),
    ] {
        let r = Repo::with("nc17a", SCAN);
        let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
        r.write(p, format!("{}{forged}", r.read(p)));
        let g = r.gen(SCAN);
        assert!(has(&g, "GD01", "warrant/impl.part"), "{name}: {}", show(&g));
        assert!(
            has(&g, "GD01", "markers are written only by gen-doc"),
            "{name}: {}",
            show(&g)
        );
        assert!(!r.path(&doc_path(SCAN)).exists(), "{name}");
    }
}

#[test]
fn nc17_a_forged_marker_in_a_contract_package_part_is_refused() {
    for part in [
        "contract.part",
        "obligations-safety.part",
        "obligations-any.part",
    ] {
        let r = Repo::with("nc17b", ANY);
        let p = format!("code/packages/rust/vos-sched-contract/warrant/{part}");
        r.write(&p, format!("{}  -- vos02:impl-end\n", r.read(&p)));
        let g = r.gen(ANY);
        assert!(
            has(&g, "GD01", &format!("warrant/{part}")),
            "{part}: {}",
            show(&g)
        );
    }
}

#[test]
fn nc17_a_cr_a_tab_or_a_control_byte_in_a_part_is_refused() {
    for (bytes, what) in [
        (&b"  -- x\r\n"[..], "0x0d"),
        (b"  -- x\n\t-- y\n", "0x09"),
        (b"  -- x \x00\n", "0x00"),
        (b"  -- caf\xc3\xa9\n", "outside ASCII"),
    ] {
        let r = Repo::with("nc17c", SCAN);
        let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
        let mut text = r.read(p).into_bytes();
        text.extend_from_slice(bytes);
        r.write(p, text);
        let g = r.gen(SCAN);
        assert!(has(&g, "GD01", what), "{what}: {}", show(&g));
    }
    // And in a contract package part.
    let r = Repo::with("nc17d", SCAN);
    let p = "code/packages/rust/vos-sched-contract/warrant/obligations-policy.part";
    let mut text = r.read(p).into_bytes();
    text.extend_from_slice(b"  -- x\r\n");
    r.write(p, text);
    assert!(has(&r.gen(SCAN), "GD01", "obligations-policy.part"));
}

#[test]
fn nc17_a_part_with_no_final_lf_or_no_content_is_refused() {
    let r = Repo::with("nc17e", SCAN);
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    r.write(p, r.read(p).trim_end());
    assert!(has(&r.gen(SCAN), "GD01", "no LF"));
    r.write(p, "");
    assert!(has(&r.gen(SCAN), "GD01", "empty part"));
}

fn doc_lines(r: &Repo, name: &str) -> Vec<String> {
    r.doc(name)
        .split_inclusive('\n')
        .map(str::to_string)
        .collect()
}

fn write_lines(r: &Repo, name: &str, lines: &[String]) {
    r.write(&doc_path(name), lines.concat());
}

#[test]
fn nc17_a_duplicated_missing_or_reordered_marker_fails_check_doc() {
    let r = generated("nc17f", SCAN);
    let lines = doc_lines(&r, SCAN);
    let idx = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
    // duplicated
    for needle in ["vos02:impl-begin", "vos02:contract-end", "vos02:generated"] {
        let mut l = lines.clone();
        let i = idx(needle);
        l.insert(i, lines[i].clone());
        write_lines(&r, SCAN, &l);
        let c = r.check(SCAN);
        assert_eq!(codes(&c)[0], "CD01", "{needle} twice: {}", show(&c));
    }
    // missing
    for needle in [
        "vos02:impl-end",
        "vos02:contract-begin",
        "vos02:obligations-end",
    ] {
        let mut l = lines.clone();
        l.remove(idx(needle));
        write_lines(&r, SCAN, &l);
        let c = r.check(SCAN);
        assert_eq!(codes(&c)[0], "CD01", "{needle} missing: {}", show(&c));
    }
    // reordered: impl block before the contract block
    let mut l = lines.clone();
    let (cb, ce) = (idx("vos02:contract-begin"), idx("vos02:contract-end"));
    let (ib, ie) = (idx("vos02:impl-begin"), idx("vos02:impl-end"));
    let contract_block: Vec<String> = l.drain(cb..=ce).collect();
    let ib2 = ib - contract_block.len();
    let ie2 = ie - contract_block.len();
    for (k, line) in contract_block.into_iter().enumerate() {
        l.insert(ie2 + 1 + k, line);
    }
    assert!(ib2 < ie2);
    write_lines(&r, SCAN, &l);
    let c = r.check(SCAN);
    assert_eq!(codes(&c)[0], "CD01", "{}", show(&c));
    // tiers out of order
    let mut l = lines.clone();
    let (sb, se) = (
        idx("tier=safety"),
        lines
            .iter()
            .rposition(|x| x.contains("vos02:obligations-end"))
            .unwrap(),
    );
    let safety: Vec<String> = l.drain(sb..=se).collect();
    let cb = idx("tier=canonical");
    for (k, line) in safety.into_iter().enumerate() {
        l.insert(cb + k, line);
    }
    write_lines(&r, SCAN, &l);
    let c = r.check(SCAN);
    assert!(has(&c, "CD01", "tier order"), "{}", show(&c));
}

#[test]
fn nc17_marker_looking_text_anywhere_in_the_document_fails_check_doc() {
    let r = generated("nc17g", SCAN);
    for (from, to) in [
        (
            "  record AbsW(w: U8)\n",
            "  record AbsW(w: U8)  -- vos02:impl-end\n",
        ),
        (
            "  record AbsW(w: U8)\n",
            "  -- vos02:surprise\n  record AbsW(w: U8)\n",
        ),
    ] {
        let original = r.doc(SCAN);
        r.edit(&doc_path(SCAN), from, to);
        let c = r.check(SCAN);
        assert_eq!(codes(&c)[0], "CD01", "{}", show(&c));
        r.write(&doc_path(SCAN), original);
    }
}

#[test]
fn nc17_a_claim_moved_between_blocks_does_not_change_what_is_carried() {
    // Move `safety_alpha_build` out of the safety obligations block and into the
    // implementation block, leaving every marker as it was.  `carries` is computed from
    // the parsed claims and their claim_sha256, so the tier is still carried; the digest
    // and the copy checks are what flag the edit.
    let r = generated("nc17h", SCAN);
    let doc = r.doc(SCAN);
    let start = doc.find("  claim safety_alpha_build:").unwrap();
    let end = doc[start..].find("  -- vos02:obligations-end").unwrap() + start;
    let claim = doc[start..end].to_string();
    let moved = format!("{}{}", &doc[..start], &doc[end..]).replacen(
        "  -- vos02:impl-end\n",
        &format!("{claim}  -- vos02:impl-end\n"),
        1,
    );
    r.write(&doc_path(SCAN), moved);
    let c = r.check(SCAN);
    assert_eq!(carried(&c), ["canonical", "safety"], "{}", show(&c));
    let got = codes(&c);
    assert!(
        got.contains(&"CD02") && got.contains(&"CD03") && got.contains(&"CD08"),
        "{}",
        show(&c)
    );
    assert!(
        !got.contains(&"CD05") && !got.contains(&"CD06"),
        "carries is unchanged: {}",
        show(&c)
    );
}

#[test]
fn nc17_a_relabelled_block_does_not_change_what_is_carried() {
    let r = generated("nc17i", SCAN);
    // The canonical block labelled `tier=any` (still in sorted order, digest unchanged).
    r.edit(
        &doc_path(SCAN),
        "obligations-begin tier=canonical",
        "obligations-begin tier=any",
    );
    let c = r.check(SCAN);
    assert_eq!(carried(&c), ["canonical", "safety"], "{}", show(&c));
    assert!(has(&c, "CD03", "obligations block `any`"), "{}", show(&c));
}

// ---------------------------------------------------------------------------
// The document on disk: header, trailer, files that are not a document
// ---------------------------------------------------------------------------

#[test]
fn a_wrong_header_or_generated_marker_fails() {
    let r = generated("hdr", SCAN);
    r.edit(
        &doc_path(SCAN),
        "warrant vos_fixture_scan v1",
        "warrant other v1",
    );
    assert!(has(&r.check(SCAN), "CD01", "the header is"));
    r.edit(
        &doc_path(SCAN),
        "warrant other v1",
        "warrant vos_fixture_scan v1",
    );
    r.edit(
        &doc_path(SCAN),
        "contract-package=vos_sched_contract",
        "contract-package=vos_ready_contract",
    );
    assert!(has(&r.check(SCAN), "CD01", "names contract package"));
}

#[test]
fn an_assume_row_in_the_implementation_block_is_refused() {
    let r = Repo::with("assume", SCAN);
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    r.write(p, format!("{}  assume platform:\n    none\n", r.read(p)));
    let g = r.gen(SCAN);
    assert!(g.findings.iter().any(|f| f.code == "CD04"), "{}", show(&g));
}

#[test]
fn a_tampered_trailer_is_refused() {
    let r = generated("trailer", SCAN);
    r.edit(
        &doc_path(SCAN),
        "  assume platform:\n    none\n",
        "  assume platform:\n    contract.x.y\n",
    );
    let c = r.check(SCAN);
    let got = codes(&c);
    assert!(
        got.contains(&"CD04") || got.contains(&"CD08"),
        "{}",
        show(&c)
    );
    assert!(got.contains(&"CD08"));
}

#[test]
fn a_document_that_warrant_grammar_refuses_is_a_cd04() {
    let r = generated("badwarrant", SCAN);
    r.edit(
        &doc_path(SCAN),
        "  record CfgN(v: U8)\n",
        "  record CfgN(v: Nonsense)\n",
    );
    let c = r.check(SCAN);
    assert!(has(&c, "CD04", "warrant-grammar refused"), "{}", show(&c));
}

// ---------------------------------------------------------------------------
// The inputs (GD01..GD05)
// ---------------------------------------------------------------------------

#[test]
fn contract_vos_problems_are_gd02() {
    let c = "code/packages/rust/vos-sched-contract/contract.vos";
    for (from, to, why) in [
        (
            "config N 8",
            "config N 18446744073709551616",
            "does not fit",
        ),
        ("config N 8", "config N 100000000000000000000", "Expected"),
        (
            "tier canonical\n",
            "tier canonical\ntier canonical\n",
            "repeated tier",
        ),
        (
            "tier any\ntier canonical\n",
            "tier canonical\ntier any\n",
            "sorted",
        ),
        (
            "name vos-sched-contract",
            "name  vos-sched-contract",
            "canonical",
        ),
        ("vos-contract 1", "vos-contract 2", "version"),
        (
            "name vos-sched-contract",
            "name vos-other-contract",
            "lives in",
        ),
        (
            "ceiling dispatch depth 2",
            "ceiling dispatch  depth 2",
            "canonical",
        ),
    ] {
        let r = Repo::with("gd02", SCAN);
        r.edit(c, from, to);
        let g = r.gen(SCAN);
        assert!(has(&g, "GD02", why), "{from} -> {to}: {}", show(&g));
    }
    let r = Repo::with("gd02b", SCAN);
    std::fs::remove_file(r.path(c)).unwrap();
    assert!(has(&r.gen(SCAN), "GD02", "not found"));
}

#[test]
fn tier_files_must_agree_with_contract_vos_gd03() {
    let w = "code/packages/rust/vos-sched-contract/warrant";
    let r = Repo::with("gd03a", SCAN);
    std::fs::remove_file(r.path(&format!("{w}/obligations-policy.part"))).unwrap();
    assert!(has(
        &r.gen(SCAN),
        "GD03",
        "`tier policy` but warrant/obligations-policy.part does not exist"
    ));
    let r = Repo::with("gd03b", SCAN);
    r.write(
        &format!("{w}/obligations-fast.part"),
        "  claim x:\n    true\n",
    );
    assert!(has(
        &r.gen(SCAN),
        "GD03",
        "\"obligations-fast.part\" exists but contract.vos has no matching `tier` row"
    ));
    let r = Repo::with("gd03c", SCAN);
    r.edit(
        "code/packages/rust/vos-sched-contract/contract.vos",
        "tier policy\n",
        "",
    );
    assert!(has(
        &r.gen(SCAN),
        "GD03",
        "\"obligations-policy.part\" exists but"
    ));
    let r = Repo::with("gd03d", SCAN);
    std::fs::remove_file(r.path(&format!("{w}/contract.part"))).unwrap();
    assert!(has(&r.gen(SCAN), "GD03", "contract.part: not found"));
}

#[test]
fn obligations_parts_hold_only_unique_claims_gd04() {
    let w = "code/packages/rust/vos-sched-contract/warrant";
    let r = Repo::with("gd04a", SCAN);
    r.write(
        &format!("{w}/obligations-policy.part"),
        format!(
            "{}  model m() -> Bool = true\n",
            r.read(&format!("{w}/obligations-policy.part"))
        ),
    );
    assert!(has(&r.gen(SCAN), "GD04", "obligations-policy.part"));
    let r = Repo::with("gd04b", SCAN);
    // The same obligation name in two tiers.
    let safety = r.read(&format!("{w}/obligations-safety.part"));
    r.write(
        &format!("{w}/obligations-policy.part"),
        format!(
            "{}{}",
            r.read(&format!("{w}/obligations-policy.part")),
            safety
                .split("  claim remove_safe:")
                .next()
                .unwrap()
                .lines()
                .filter(|l| !l.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        ),
    );
    let g = r.gen(SCAN);
    assert!(has(&g, "GD04", "is declared in tier"), "{}", show(&g));
}

#[test]
fn vos_toml_problems_are_gd05() {
    let p = "code/packages/rust/vos-fixture-scan/vos.toml";
    let cases: Vec<(&str, &str)> = vec![
        ("kind = \"production\"\nkind = \"any\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\"]\n", "duplicate key"),
        ("kind = \"spine\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\"]\n", "not an implementation kind"),
        ("contract = \"vos-sched-contract\"\nobligations = [\"safety\"]\n", "no `kind`"),
        ("kind = \"production\"\nobligations = [\"safety\"]\n", "no `contract`"),
        ("kind = \"production\"\ncontract = \"../vos-sched-contract\"\nobligations = [\"safety\"]\n", "is not a package name"),
        ("kind = \"production\"\ncontract = \"vos-sched-contract\"\n", "no `obligations`"),
        ("kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"fast\"]\n", "is not a tier"),
        ("kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\", \"canonical\"]\n", "sorted and unique"),
        ("kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\", \"safety\"]\n", "sorted and unique"),
        ("kind = \"production\"\ncontract = \"vos-nothing\"\nobligations = [\"safety\"]\n", "does not exist"),
        ("kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\"]\nextra = 1\n", "unknown key"),
    ];
    for (text, why) in cases {
        let r = Repo::with("gd05", SCAN);
        r.write(p, text);
        let g = r.gen(SCAN);
        assert!(has(&g, "GD05", why), "{why}: {}", show(&g));
    }
    // Not UTF-8, and missing.
    let r = Repo::with("gd05b", SCAN);
    r.write(p, [0xff, 0xfe]);
    assert!(has(&r.gen(SCAN), "GD05", "not UTF-8"));
    let r = Repo::with("gd05c", SCAN);
    std::fs::remove_file(r.path(p)).unwrap();
    assert!(has(&r.gen(SCAN), "GD05", "vos.toml: not found"));
    // A contract package whose own vos.toml is not kind = "contract", or is missing.
    let r = Repo::with("gd05d", SCAN);
    r.write(
        "code/packages/rust/vos-sched-contract/vos.toml",
        "kind = \"production\"\n",
    );
    assert!(has(
        &r.gen(SCAN),
        "GD05",
        "does not say kind = \"contract\""
    ));
    let r = Repo::with("gd05e", SCAN);
    std::fs::remove_file(r.path("code/packages/rust/vos-sched-contract/vos.toml")).unwrap();
    assert!(has(
        &r.gen(SCAN),
        "GD05",
        "contract package: vos.toml: not found"
    ));
}

#[test]
fn a_missing_impl_part_is_gd01() {
    let r = Repo::with("noimpl", SCAN);
    std::fs::remove_file(r.path("code/packages/rust/vos-fixture-scan/warrant/impl.part")).unwrap();
    assert!(has(&r.gen(SCAN), "GD01", "warrant/impl.part: not found"));
}

#[test]
fn findings_are_collected_not_first_only() {
    // Two independent input problems are both reported.
    let r = Repo::with("many", SCAN);
    let w = "code/packages/rust/vos-sched-contract/warrant";
    std::fs::remove_file(r.path(&format!("{w}/obligations-policy.part"))).unwrap();
    r.write(
        &format!("{w}/obligations-fast.part"),
        "  claim x:\n    true\n",
    );
    let g = r.gen(SCAN);
    assert_eq!(
        g.findings.iter().filter(|f| f.code == "GD03").count(),
        2,
        "{}",
        show(&g)
    );
}

// ---------------------------------------------------------------------------
// Hostile Warrant input must not crash the process
// ---------------------------------------------------------------------------

#[test]
fn a_very_long_expression_chain_does_not_crash_the_process_no_verdict_asserted() {
    // NO-CRASH ONLY: what Warrant says about the chain is not asserted.
    // warrant-grammar's checker recurses over the expression tree; a long left-leaning
    // chain overflows a 2 MiB stack (VOS-D057).  gen-doc runs the analysis on a 256 MiB
    // stack, so this returns instead of aborting the test process.
    let r = Repo::with("deep", SCAN);
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    let chain = "      true and true and true and true and true and true and true and true and\n"
        .repeat(1_500);
    r.write(
        p,
        format!("{}  example deep:\n    ({chain}true)\n", r.read(p)),
    );
    let g = r.gen(SCAN);
    // Whatever Warrant says, we got here.
    let _ = show(&g);
}

// ---------------------------------------------------------------------------
// Corners of the document checks
// ---------------------------------------------------------------------------

#[test]
fn an_obligations_block_for_a_tier_the_contract_does_not_define_is_cd03() {
    let r = generated("tier-gone", SCAN);
    let c = "code/packages/rust/vos-sched-contract";
    r.edit(&format!("{c}/contract.vos"), "tier canonical\n", "");
    std::fs::remove_file(r.path(&format!("{c}/warrant/obligations-canonical.part"))).unwrap();
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        format!("kind = \"reference\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let o = r.check(SCAN);
    assert!(has(&o, "CD03", "is not a tier of contract"), "{}", show(&o));
}

#[test]
fn text_after_the_generated_trailer_is_cd04() {
    let r = generated("trailer2", SCAN);
    let doc = r.doc(SCAN);
    r.write(
        &doc_path(SCAN),
        format!("{doc}  -- a remark after the trailer\n"),
    );
    let o = r.check(SCAN);
    assert!(has(&o, "CD04", "not the generated one"), "{}", show(&o));
}

#[test]
fn a_missing_build_model_fails_closed_for_production_and_any() {
    // `build` declared as a bound operation (no body): the cone cannot be read.
    let from = "  model build(m: Ready) -> Impl = (";
    let r = Repo::with("nobuild1", "vos-fixture-scan");
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    let text = r.read(p);
    let a = text.find(from).unwrap();
    let b = text.find("  model inv_impl").unwrap();
    r.write(
        p,
        format!("{}  op build(m: Ready) -> Impl\n  labels build:\n    m public\n  input build.m:\n    hostile\n    ensures no_panic, terminates\n{}", &text[..a], &text[b..]),
    );
    let g = r.gen(SCAN);
    assert!(has(&g, "CD07", "is missing, so the cone"), "{}", show(&g));

    let r = Repo::with("nobuild2", ANY);
    let p = "code/packages/rust/vos-fixture-any/warrant/impl.part";
    let text = r.read(p);
    let a = text.find(from).unwrap();
    let b = text.find("  model inv_impl").unwrap();
    r.write(
        p,
        format!("{}  op build(m: Ready) -> Impl\n  labels build:\n    m public\n  input build.m:\n    hostile\n    ensures no_panic, terminates\n{}", &text[..a], &text[b..]),
    );
    let g = r.gen(ANY);
    assert!(
        has(&g, "CD07", "is missing, so the oracle field"),
        "{}",
        show(&g)
    );
}

#[test]
fn an_any_build_that_is_not_a_constructor_fails_closed() {
    let r = Repo::with("any-helper", ANY);
    let p = "code/packages/rust/vos-fixture-any/warrant/impl.part";
    let text = r.read(p);
    let a = text.find("  model build(m: Ready) -> Impl = (").unwrap();
    let b = text.find("  model inv_impl").unwrap();
    let old = &text[a..b];
    let helper =
        old.replacen("model build", "model mk", 1) + "  model build(m: Ready) -> Impl = mk(m)\n";
    r.write(p, format!("{}{helper}{}", &text[..a], &text[b..]));
    let g = r.gen(ANY);
    assert!(
        has(&g, "CD07", "oracle field cannot be found (fails closed)"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_document_over_the_cap_on_disk_is_no_verdict_but_in_memory_it_is_too() {
    // gen-doc: parts each under 1 MiB whose sum is over the document cap.
    let r = Repo::with("sum", SCAN);
    let p = "code/packages/rust/vos-fixture-scan/warrant/impl.part";
    let line = format!("  -- {}\n", "x".repeat(70));
    r.write(p, format!("{}{}", r.read(p), line.repeat(13_700)));
    let big = r.read(p).len();
    assert!(big < 1024 * 1024 && big > 1_000_000, "{big}");
    let e = vos_link::gen_doc(&r.pkg(SCAN), None).unwrap_err();
    assert!(
        e.to_string().contains("cannot assemble the document"),
        "{e}"
    );
}

// ---------------------------------------------------------------------------
// Security review round 1
// ---------------------------------------------------------------------------

const ANY_BUILD_ORACLE: &str = "b2m(m.p6, 6)), b2m(m.p7, 7)), 0u8))";

/// The `-any` implementation, relabelled as a production crate with an honest name, whose
/// oracle argument of `build` is replaced by `field`.
fn pretender(label: &str, field: &str, extra: &str) -> (Repo, vos_link::Outcome) {
    let r = Repo::new(label);
    copy_as(
        &r,
        ANY,
        "vos-fixture-pretend",
        &format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"canonical\", \"policy\", \"safety\"]\n"),
    );
    let p = "code/packages/rust/vos-fixture-pretend/warrant/impl.part";
    r.edit(
        p,
        ANY_BUILD_ORACLE,
        &format!("b2m(m.p6, 6)), b2m(m.p7, 7)), {field}))"),
    );
    if !extra.is_empty() {
        r.write(p, format!("{}{extra}", r.read(p)));
    }
    let g = r.gen("vos-fixture-pretend");
    (r, g)
}

#[test]
fn the_reviewers_pretender_is_now_rejected_by_the_taint_walk() {
    // S-M1: `bit_and(b2m(m.p0, 0), 0u8)` mentions m but is the constant 0.
    let (r, g) = pretender("taint1", "bit_and(b2m(m.p0, 0), 0u8)", "");
    assert!(
        has(
            &g,
            "CD07",
            "field(s) 1 of `Impl` are not set from the abstract state"
        ),
        "{}",
        show(&g)
    );
    assert!(!r.path(&doc_path("vos-fixture-pretend")).exists());
}

#[test]
fn variants_of_the_pretender_are_rejected_too() {
    for (name, field, extra) in [
        ("times zero", "b2m(m.p0, 0) * 0u8", ""),
        ("zero times", "0u8 * b2m(m.p0, 0)", ""),
        ("x minus x", "b2m(m.p0, 0) - b2m(m.p0, 0)", ""),
        ("x xor x", "bit_xor(b2m(m.p0, 0), b2m(m.p0, 0))", ""),
        (
            "ignored argument",
            "ign(m.p0)",
            "  model ign(x: Bool) -> U8 = 0u8\n",
        ),
        ("bit_and swapped", "bit_and(0u8, b2m(m.p0, 0))", ""),
    ] {
        let (_r, g) = pretender("taint2", field, extra);
        assert!(
            has(&g, "CD07", "not set from the abstract state"),
            "{name}: {}",
            show(&g)
        );
    }
}

#[test]
fn an_honest_derivation_of_the_register_is_still_inside_the_cone() {
    let (_r, g) = pretender("taint3", "bit_and(b2m(m.p0, 0), 255u8)", "");
    assert!(
        !has(&g, "CD07", "not set from the abstract state"),
        "{}",
        show(&g)
    );
}

#[test]
fn a_pass_says_the_cone_rule_is_only_a_structural_lint() {
    let r = Repo::with("note", SCAN);
    let g = r.gen(SCAN);
    assert!(
        g.notes
            .iter()
            .any(|n| n.contains("STRUCTURAL LINT") && n.contains("VOS-D060")),
        "{:?}",
        g.notes
    );
    assert!(g.notes.iter().any(|n| n.contains("author-declared")));
    let c = r.check(SCAN);
    assert!(!c.notes.is_empty());
    let r = Repo::with("note-ref", "vos-fixture-ref");
    assert!(
        r.gen("vos-fixture-ref").notes.is_empty(),
        "a reference model has no cone rule"
    );
}

#[test]
fn trailer_variants_are_each_a_cd04() {
    let tail = "  assume platform:\n    none\n  assume environment:\n    none\n";
    for (name, replacement) in [
        ("tampered environment row", "  assume platform:\n    none\n  assume environment:\n    HOME == HOME\n"),
        ("duplicated assume row", "  assume platform:\n    none\n  assume platform:\n    none\n  assume environment:\n    none\n"),
        ("reordered rows", "  assume environment:\n    none\n  assume platform:\n    none\n"),
        ("missing environment row", "  assume platform:\n    none\n"),
        ("missing platform row", "  assume environment:\n    none\n"),
        ("no rows at all", ""),
    ] {
        let r = generated("trailer-variants", SCAN);
        r.edit(&doc_path(SCAN), tail, replacement);
        let c = r.check(SCAN);
        assert!(codes(&c).contains(&"CD04"), "{name}: {}", show(&c));
    }
}

#[test]
fn an_extra_carried_tier_is_a_cd06() {
    // The document carries canonical and safety; vos.toml now says only safety.
    let r = generated("extra-tier", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        format!("kind = \"reference\"\n{TOML_HEAD}obligations = [\"safety\"]\n"),
    );
    let c = r.check(SCAN);
    assert!(
        has(
            &c,
            "CD06",
            "carries [canonical,safety] but vos.toml obligations is [safety]"
        ),
        "{}",
        show(&c)
    );
}

#[test]
fn forged_markers_in_the_canonical_and_policy_parts_are_refused() {
    for part in ["obligations-canonical.part", "obligations-policy.part"] {
        let r = Repo::with("nc17-more", SCAN);
        let p = format!("code/packages/rust/vos-sched-contract/warrant/{part}");
        r.write(&p, format!("{}  -- vos02:impl-end\n", r.read(&p)));
        let g = r.gen(SCAN);
        assert!(
            has(&g, "GD01", &format!("warrant/{part}")),
            "{part}: {}",
            show(&g)
        );
    }
}

#[test]
fn untrusted_strings_are_escaped_in_findings() {
    // S-L1: a value with ESC or a newline must not reach the report raw.
    // A file NAME with ESC or a newline can only be created on Unix: Windows
    // forbids control characters in file names, so there the sub-case cannot
    // arise and is skipped visibly. The value and header cases below run on
    // every platform.
    if cfg!(unix) {
        let r = Repo::with("esc1", SCAN);
        let w = "code/packages/rust/vos-sched-contract/warrant";
        r.write(
            &format!("{w}/obligations-x\u{1b}[31m\nFAKE.part"),
            "  claim x:\n    true\n",
        );
        let g = r.gen(SCAN);
        let text = show(&g);
        assert!(has(&g, "GD03", "obligations-x"), "{text}");
        assert!(
            !text.contains('\u{1b}'),
            "raw ESC reached the report: {text:?}"
        );
        assert!(
            !text.contains("\nFAKE"),
            "a file name forged a report line: {text:?}"
        );
        assert!(text.contains("\\u{1b}") && text.contains("\\n"), "{text}");
    } else {
        eprintln!("SKIPPED: control characters in file names are not creatable on this platform");
    }
    // A contract value that is legal TOML but not a package name.
    let r = Repo::with("esc2", SCAN);
    r.write(
        "code/packages/rust/vos-fixture-scan/vos.toml",
        "kind = \"production\"\ncontract = \"a b\\tc\"\nobligations = [\"safety\"]\n",
    );
    let g = r.gen(SCAN);
    assert!(
        g.findings
            .iter()
            .all(|f| !f.message.contains('\t') && !f.message.contains('\u{1b}')),
        "{}",
        show(&g)
    );
    // A hostile header in a document on disk: a control byte is refused at admission
    // (CD01 with the byte named in hex); printable oddities are quoted.
    let r = generated("esc3", SCAN);
    r.edit(
        &doc_path(SCAN),
        "warrant vos_fixture_scan v1",
        "warrant \u{1b}[2J v1",
    );
    let c = r.check(SCAN);
    assert!(has(&c, "CD01", "control byte 0x1b"), "{}", show(&c));
    assert!(!show(&c).contains('\u{1b}'));
    let r = generated("esc4", SCAN);
    r.edit(
        &doc_path(SCAN),
        "warrant vos_fixture_scan v1",
        "warrant a\"b\\c v1",
    );
    let c = r.check(SCAN);
    assert!(
        has(&c, "CD01", "the header is \"warrant a\\\"b\\\\c v1\""),
        "{}",
        show(&c)
    );
}

#[test]
fn a_cone_walk_that_runs_out_of_budget_fails_closed_for_production_and_any() {
    // N1: the walk gives up rather than answer; both kinds that need the answer fail.
    for (fixture, kind_word) in [
        (SCAN, "cone cannot be decided"),
        (ANY, "oracle field cannot be found"),
    ] {
        let r = generated("budget", fixture);
        let loaded = match vos_link::inputs::load(&r.pkg(fixture), None).unwrap() {
            vos_link::inputs::Load::Ready(l) => *l,
            vos_link::inputs::Load::Findings(f) => panic!("{f:?}"),
        };
        let doc = std::fs::read(r.path(&doc_path(fixture))).unwrap();
        let v = vos_link::check::verify_with_budget(&loaded, &doc, 10).unwrap();
        assert!(
            v.findings.iter().any(|f| f.code == "CD07"
                && f.message.contains("step budget")
                && f.message.contains(kind_word)),
            "{fixture}: {:?}",
            v.findings
        );
        // With the real budget the same document passes.
        assert!(vos_link::check::verify(&loaded, &doc)
            .unwrap()
            .findings
            .is_empty());
    }
}

#[test]
fn a_fanout_chain_in_an_implementation_is_checked_quickly() {
    // N1 end to end: the reviewer's chain (depth 18, fanout 6) as the oracle field.
    let r = Repo::new("fanout");
    copy_as(
        &r,
        ANY,
        "vos-fixture-pretend",
        &format!("kind = \"production\"\n{TOML_HEAD}obligations = [\"canonical\", \"safety\"]\n"),
    );
    let p = "code/packages/rust/vos-fixture-pretend/warrant/impl.part";
    let mut chain = String::new();
    for i in 1..18 {
        let call = format!("f{}(x)", i + 1);
        let mut e = call.clone();
        for _ in 1..6 {
            e = format!("bit_or({e}, {call})");
        }
        chain.push_str(&format!("  model f{i}(x: U8) -> U8 = {e}\n"));
    }
    chain.push_str("  model f18(x: U8) -> U8 = 0u8\n");
    r.write(p, format!("{}{chain}", r.read(p)));
    r.edit(
        p,
        ANY_BUILD_ORACLE,
        "b2m(m.p6, 6)), b2m(m.p7, 7)), f1(b2m(m.p0, 0))))",
    );
    let t = std::time::Instant::now();
    let g = r.gen("vos-fixture-pretend");
    assert!(
        has(&g, "CD07", "not set from the abstract state"),
        "{}",
        show(&g)
    );
    assert!(
        t.elapsed() < std::time::Duration::from_secs(20),
        "{:?}",
        t.elapsed()
    );
}

#[test]
fn a_model_with_more_than_63_parameters_fails_closed_end_to_end() {
    let params: Vec<String> = (0..70).map(|i| format!("p{i}: Bool")).collect();
    let args = vec!["b2m(m.p0, 0) == 0u8"; 70].join(", ");
    let extra = format!(
        "  model w({}) -> U8 = if p63 then 1u8 else 0u8\n",
        params.join(", ")
    );
    let (_r, g) = pretender("wide", &format!("w({args})"), &extra);
    assert!(has(&g, "CD07", "more than 63 parameters"), "{}", show(&g));
}
