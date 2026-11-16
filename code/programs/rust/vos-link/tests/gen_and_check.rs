//! gen-doc and check-doc on real inputs: the real scheduler contract package and four
//! fixture implementations (a production bitmap, a production linear scan, the most-general
//! `-any`, and a reference model).  VOS-033, "run it for real".

mod common;
use common::*;

use vos_manifest::document::parse_layout;

const GOLDEN: &str = "tests/fixtures/expected/vos-fixture-bitmap.warrant";

fn golden_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(GOLDEN)
}

#[test]
fn gen_doc_writes_the_bitmap_document_and_check_doc_passes_it() {
    let r = Repo::with("bitmap", "vos-fixture-bitmap");
    let g = r.gen("vos-fixture-bitmap");
    assert!(g.findings.is_empty(), "{}", show(&g));
    assert_eq!(carried(&g), ["canonical", "policy", "safety"]);
    assert_eq!(
        g.action
            .as_deref()
            .map(|a| a.starts_with("wrote code/specs/vos-fixture-bitmap.warrant")),
        Some(true)
    );
    let c = r.check("vos-fixture-bitmap");
    assert!(c.findings.is_empty(), "{}", show(&c));
    assert_eq!(carried(&c), ["canonical", "policy", "safety"]);
    assert!(c.header.contains("kind production"));
    assert!(c.header.contains("contract vos-sched-contract"));
}

#[test]
fn the_document_is_the_golden_text() {
    // The golden file is the real output for the real contract: a change to the contract
    // package, to a part, or to the tool shows up here as a diff to review (VOS02 12.1
    // item 10: a contract change lists every document it regenerates).  To accept an
    // intended change: VOS_LINK_BLESS=1 cargo test.
    let r = Repo::with("golden", "vos-fixture-bitmap");
    r.gen("vos-fixture-bitmap");
    let doc = r.doc("vos-fixture-bitmap");
    if std::env::var_os("VOS_LINK_BLESS").is_some() {
        std::fs::create_dir_all(golden_path().parent().unwrap()).unwrap();
        std::fs::write(golden_path(), &doc).unwrap();
    }
    let golden = std::fs::read_to_string(golden_path()).expect("the golden file");
    assert_eq!(
        doc, golden,
        "run VOS_LINK_BLESS=1 cargo test to accept an intended change"
    );
}

#[test]
fn the_document_layout_is_the_one_of_the_spec() {
    let r = Repo::with("layout", "vos-fixture-bitmap");
    r.gen("vos-fixture-bitmap");
    let doc = r.doc("vos-fixture-bitmap");
    let l = parse_layout(doc.as_bytes()).unwrap();
    assert_eq!(l.header, "warrant vos_fixture_bitmap v1");
    assert_eq!(l.contract_package, "vos_sched_contract");
    // The contract block is the contract package's file, with its digest in the marker.
    assert_eq!(
        l.contract.body,
        r.read("code/packages/rust/vos-sched-contract/warrant/contract.part")
    );
    assert_eq!(
        l.contract.marker_sha256.as_deref(),
        Some(l.contract.computed_sha256().as_str())
    );
    assert_eq!(
        l.implementation.body,
        r.read("code/packages/rust/vos-fixture-bitmap/warrant/impl.part")
    );
    let tiers: Vec<_> = l.obligations.iter().map(|(t, _)| t.name()).collect();
    assert_eq!(tiers, ["canonical", "policy", "safety"]);
    for (tier, block) in &l.obligations {
        let part = r.read(&format!(
            "code/packages/rust/vos-sched-contract/warrant/obligations-{}.part",
            tier.name()
        ));
        assert_eq!(block.body, part);
    }
    assert!(doc.ends_with("  assume platform:\n    none\n  assume environment:\n    none\n"));
}

#[test]
fn every_fixture_implementation_generates_and_checks_clean() {
    for (name, kind, tiers) in [
        (
            "vos-fixture-scan",
            "production",
            vec!["canonical", "safety"],
        ),
        ("vos-fixture-any", "any", vec!["any", "safety"]),
        ("vos-fixture-ref", "reference", vec!["safety"]),
    ] {
        let r = Repo::with(name, name);
        let g = r.gen(name);
        assert!(g.findings.is_empty(), "{name}: {}", show(&g));
        assert_eq!(carried(&g), tiers, "{name}");
        assert!(g.header.contains(&format!("kind {kind}")));
        let c = r.check(name);
        assert!(c.findings.is_empty(), "{name}: {}", show(&c));
    }
}

#[test]
fn a_second_gen_doc_leaves_the_file_unchanged() {
    let r = Repo::with("idem", "vos-fixture-scan");
    r.gen("vos-fixture-scan");
    let first = r.doc("vos-fixture-scan");
    let second = r.gen("vos-fixture-scan");
    assert!(second.findings.is_empty());
    assert!(second.action.unwrap().starts_with("unchanged "));
    assert_eq!(first, r.doc("vos-fixture-scan"));
    assert!(
        !r.path("code/specs/.vos-fixture-scan.warrant.vos-link.tmp")
            .exists(),
        "no temporary file is left behind"
    );
}

#[test]
fn check_doc_with_no_document_says_to_generate_it() {
    let r = Repo::with("nodoc", "vos-fixture-scan");
    let c = r.check("vos-fixture-scan");
    assert_eq!(codes(&c), ["CD01"]);
    assert!(c.findings[0].message.contains("gen-doc"));
}

#[test]
fn an_implementation_edit_is_drift_until_regenerated() {
    let r = Repo::with("drift", "vos-fixture-scan");
    r.gen("vos-fixture-scan");
    assert!(r.check("vos-fixture-scan").findings.is_empty());
    // A harmless extra helper model in impl.part: the document on disk is now stale.
    r.edit(
        "code/packages/rust/vos-fixture-scan/warrant/impl.part",
        "  model b2u(b: Bool) -> U8 = if b then 1u8 else 0u8\n",
        "  model b2u(b: Bool) -> U8 = if b then 1u8 else 0u8\n  model unused_helper(x: U8) -> U8 = x\n",
    );
    let c = r.check("vos-fixture-scan");
    assert_eq!(codes(&c), ["CD08"], "{}", show(&c));
    assert!(c.findings[0].message.contains("first difference at line"));
    r.gen("vos-fixture-scan");
    assert!(r.check("vos-fixture-scan").findings.is_empty());
}

#[test]
fn a_contract_change_makes_every_document_stale() {
    let r = Repo::with("contract-change", "vos-fixture-bitmap");
    r.gen("vos-fixture-bitmap");
    // A comment edit in the contract block: no claim changes, but the copy is stale.
    r.edit(
        "code/packages/rust/vos-sched-contract/warrant/contract.part",
        "  -- Total abstract operations",
        "  -- Total abstract operations (edited)",
    );
    let c = r.check("vos-fixture-bitmap");
    assert_eq!(codes(&c), ["CD03", "CD08"], "{}", show(&c));
    assert!(r.gen("vos-fixture-bitmap").findings.is_empty());
    assert!(r.check("vos-fixture-bitmap").findings.is_empty());
}

#[test]
fn gen_doc_writes_nothing_when_the_document_would_be_rejected() {
    let r = Repo::with("no-write", "vos-fixture-bitmap");
    // The bitmap's one-field Impl cannot satisfy the any block: warrant-grammar refuses.
    r.write("code/packages/rust/vos-fixture-bitmap/vos.toml", "kind = \"production\"\ncontract = \"vos-sched-contract\"\nobligations = [\"any\", \"canonical\", \"safety\"]\n");
    let g = r.gen("vos-fixture-bitmap");
    assert!(g.findings.iter().any(|f| f.code == "CD04"), "{}", show(&g));
    assert!(!r.path("code/specs/vos-fixture-bitmap.warrant").exists());
    assert!(g.action.is_none());
}

#[test]
fn the_real_scheduler_contract_checks_out_as_a_contract_package() {
    // The inputs layer on the real files: tiers agree with the files, claims are unique.
    let r = Repo::with("inputs", "vos-fixture-scan");
    let o = r.gen("vos-fixture-scan");
    assert!(o.findings.is_empty());
}
