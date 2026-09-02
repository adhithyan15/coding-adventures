//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/vitamin-deficiency-symptom.adj`) driven
//! through the built CLI: a native `table` recording, for five of the
//! seven vitamins already tabled in `vitamins.adj`, the SYMPTOM described
//! in the same already-quoted NIH span that names the deficiency disease
//! -- a sibling decoding the symptom half of each already-verified quote.
//! Resolves forward and backward recall queries with the source's
//! citation, plus honest abstention on vitamin_c (whose cited span names
//! scurvy but states no symptom) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_vitamindeficiencysymptom_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("biology/vitamin-deficiency-symptom.adj");
    std::fs::copy(&src, dir.join("vitamin-deficiency-symptom.adj"))
        .expect("copy shipped vitamin-deficiency-symptom.adj");
}

#[test]
fn vitamin_deficiency_symptom_recalls_vitamin_d_bone_symptom_with_citation() {
    let dir = scratch("vitamind");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"\n\
         ? vitamin_deficiency_symptom(vitamin_d, $Symptom)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"vitamin_deficiency_symptom(vitamin_d, soft_weak_deformed_painful_bones)\""),
        "vitamin_d deficiency should recall the cited bone symptom: {out}"
    );
    assert!(
        out.contains("ods.od.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIH ODS citation: {out}"
    );
}

#[test]
fn vitamin_deficiency_symptom_backward_recalls_vitamin_b12_for_tired_and_weak() {
    let dir = scratch("b12");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"\n\
         ? vitamin_deficiency_symptom($V, tired_and_weak)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"vitamin_deficiency_symptom(vitamin_b12, tired_and_weak)\""),
        "vitamin_b12 should be the only recalled tired-and-weak vitamin: {out}"
    );
    assert!(
        !out.contains("vitamin_deficiency_symptom(vitamin_b9, tired_and_weak)"),
        "vitamin_b9's cited symptom is weakness_and_fatigue, not tired_and_weak: {out}"
    );
}

#[test]
fn vitamin_deficiency_symptom_abstains_on_vitamin_c() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"\n\
         ? vitamin_deficiency_symptom(vitamin_c, $Symptom)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "vitamin_c's cited span names scurvy but states no symptom -- honest abstention expected: {out}"
    );
}

const VITAMIN_DEFICIENCY_SYMPTOM_PIN: &str = r#""bindings":{"Symptom":"inability_to_see_in_low_light"},"citations":[{"source":"Xerophthalmia is the inability to see in low light, and it can lead to blindness if it isn’t treated.","locator":"https://ods.od.nih.gov/factsheets/VitaminA-Consumer/","trust":"authoritative""#;

#[test]
fn vitamin_deficiency_symptom_citation_matches_its_page_glyph_for_glyph() {
    let dir = scratch("glyph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vitamin-deficiency-symptom.adj\"
? vitamin_deficiency_symptom(vitamin_a, $Symptom)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THIS PIN QUERIES vitamin_a, NOT vitamin_d. The table ships ONE envelope
    // for five rows, and that envelope is the xerophthalmia/night-blindness
    // sentence -- which grounds vitamin_a and NOT vitamin_d. Pinning vitamin_d
    // would pair an answer about bone deformity with a citation about vision,
    // and freeze it in a test. That is #14124's defect class; the one-envelope
    // shape here is pre-existing and tracked there.
    //
    // This site was reported CLEAN by installment 3a's collector, which
    // could not complete TLS to its host and swallowed the error -- so an
    // UNCHECKED site was indistinguishable from a checked one. It came back
    // reachable AND flattened once fetch failures were printed instead.
    assert!(
        out.contains(VITAMIN_DEFICIENCY_SYMPTOM_PIN),
        "the vitamin deficiency symptom citation matches its page: {out}"
    );
}
