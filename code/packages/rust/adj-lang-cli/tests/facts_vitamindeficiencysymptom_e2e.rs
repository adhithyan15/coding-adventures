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

// WIDENED WITH THE SPAN IT PINS. The old span — "Xerophthalmia is
// the inability to see in low light…" — states the SYMPTOM and
// never names vitamin A, so on its own it did not warrant
// `(vitamin_a, inability_to_see_in_low_light)`. The page's
// preceding sentence supplies the link and is contiguous. The pin
// moves with the span; keeping the span to satisfy the pin would
// be the tail wagging the dog.
const VITAMIN_DEFICIENCY_SYMPTOM_PIN: &str = r#""bindings":{"Symptom":"inability_to_see_in_low_light"},"citations":[{"source":"The most common sign of vitamin A deficiency is an eye condition called xerophthalmia. Xerophthalmia is the inability to see in low light, and it can lead to blindness if it isn’t treated.","locator":"https://ods.od.nih.gov/factsheets/VitaminA-Consumer/","trust":"authoritative""#;

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
    // THIS PIN QUERIES vitamin_a, and the reason it had to is now gone.
    //
    // It used to read: "The table ships ONE envelope for five rows, and that
    // envelope is the xerophthalmia/night-blindness sentence -- which grounds
    // vitamin_a and NOT vitamin_d. Pinning vitamin_d would pair an answer
    // about bone deformity with a citation about vision, and freeze it in a
    // test. That is #14124's defect class; the one-envelope shape here is
    // pre-existing and tracked there."
    //
    // Since #14986 every row carries the fact sheet that states it, so
    // `vitamin_d` can be pinned to its own rickets sentence — and is, below.
    // The test that could only safely query one row is the shape this
    // conversion removes.
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

const ODS: &str = "https://ods.od.nih.gov/factsheets/";

/// Assert one row's warrant, binding the VITAMIN so exactly one row answers.
fn assert_vitamin(tag: &str, vitamin: &str, symptom: &str, span: &str, sheet: &str) {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!(
            "import \"vitamin-deficiency-symptom.adj\"\n? vitamin_deficiency_symptom({vitamin}, $S)\n"
        ),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {vitamin}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"S\":\"{symptom}\"")),
        "{vitamin} binds {symptom}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{ODS}{sheet}\",\"trust\":\"authoritative\""
        )),
        "{vitamin} is warranted by its own fact sheet's sentence: {out}"
    );
}

/// #14986, and the fix for the #14124 note this file already carried. The
/// VITAMIN-A span was this table's `source` for all five rows, so a recall of
/// `vitamin_b12` came back proved by a sentence about xerophthalmia.
///
/// Five rows, five NIH ODS fact sheets, one vitamin each — the other four
/// spans were already in the file as untiered `cites`, in row order.
#[test]
fn every_vitamin_carries_its_own_fact_sheet() {
    assert_vitamin(
        "vdva", "vitamin_a", "inability_to_see_in_low_light",
        "The most common sign of vitamin A deficiency is an eye condition called xerophthalmia. Xerophthalmia is the inability to see in low light, and it can lead to blindness if it isn\u{2019}t treated.",
        "VitaminA-Consumer/",
    );
    assert_vitamin(
        "vdvd", "vitamin_d", "soft_weak_deformed_painful_bones",
        "In children, vitamin D deficiency causes rickets, a disease in which the bones become soft, weak, deformed, and painful.",
        "VitaminD-Consumer/",
    );
    assert_vitamin(
        "vdb1", "vitamin_b1", "tingling_and_numbness_in_feet_and_hands",
        "Severe thiamin deficiency leads to a disease called beriberi with the added symptoms of tingling and numbness in the feet and hands, loss of muscle, and poor reflexes.",
        "Thiamin-Consumer/",
    );
    assert_vitamin(
        "vdb9", "vitamin_b9", "weakness_and_fatigue",
        "Getting too little folate can result in megaloblastic anemia, a blood disorder that causes weakness, fatigue, trouble concentrating, irritability, headache, heart palpitations, and shortness of breath.",
        "Folate-Consumer/",
    );
    assert_vitamin(
        "vdb12", "vitamin_b12", "tired_and_weak",
        "Vitamin B12 also helps prevent megaloblastic anemia, a blood condition that makes people tired and weak.",
        "VitaminB12-Consumer/",
    );
}

/// THE WIDENING, and why the pairing check is worth running.
///
/// The shipped vitamin-A span stated the SYMPTOM and never named vitamin A, so
/// on its own it did not warrant `(vitamin_a, inability_to_see_in_low_light)` —
/// the link came from the page it sat on, not from the quoted run. The page's
/// preceding sentence supplies it and is contiguous.
///
/// The span also carries a CURLY apostrophe (U+2019) in "isn't". An ASCII
/// normalisation would make it stop being the page's bytes.
#[test]
fn the_vitamin_a_span_names_the_vitamin_and_keeps_its_curly_apostrophe() {
    let adj = std::fs::read_to_string(
        facts_stdlib().join("biology/vitamin-deficiency-symptom.adj"),
    )
    .expect("read shipped vitamin-deficiency-symptom.adj");
    assert!(
        adj.contains("The most common sign of vitamin A deficiency"),
        "the vitamin-A span names the vitamin, not only the disease"
    );
    assert!(
        adj.contains("if it isn\u{2019}t treated."),
        "and keeps the page's curly apostrophe"
    );
    // SCOPED TO THE ROW'S source LINE, not the whole file. The first form
    // forbade the ASCII spelling ANYWHERE in the .adj -- and then fired on the
    // header comment that quotes the old span while explaining what changed.
    // It was right to fire: that quote had been typed with an ASCII
    // apostrophe the original never had. Both were fixed -- the quote, and the
    // assertion that could not tell a machine value from prose about it.
    assert!(
        !adj.contains("        source \"Xerophthalmia"),
        "no row still carries the narrow symptom-only span"
    );
    for line in adj.lines() {
        if line.starts_with("        source \"") {
            assert!(
                !line.contains("isn't"),
                "no row source uses the ASCII apostrophe: {line}"
            );
        }
    }
}

/// #15193. Five rows, five distinct spans, five distinct fact sheets — no row
/// shares a warrant with another. Asserted against the SHIPPED FILE.
#[test]
fn no_two_rows_share_a_span_or_a_page() {
    let adj = std::fs::read_to_string(
        facts_stdlib().join("biology/vitamin-deficiency-symptom.adj"),
    )
    .expect("read shipped vitamin-deficiency-symptom.adj");
    // EACH ROW'S *EFFECTIVE* PAGE, not the count of locator lines. This used
    // to assert `locators.len() == 5` -- a PROXY for the property this test
    // is named after. It measured how the file is WRITTEN rather than what
    // page each row RESOLVES to, so it went red when the vitamin-A row
    // stopped restating the envelope's own URL: a change that altered no
    // row's page at all, and produced byte-identical CLI output.
    //
    // A row's effective page is its own `locator` when it has one and the
    // envelope's otherwise, which is exactly what `row_provenance` does.
    let envelope_locator = adj
        .lines()
        .find_map(|l| l.strip_prefix(r#"    locator ""#))
        .expect("envelope locator")
        .trim_end_matches(0x22 as char)
        .to_string();

    let mut spans: Vec<String> = Vec::new();
    let mut pages: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            if pending.is_some() {
                pages.push(pending.take().expect("previous row's page"));
            }
            spans.push(rest.trim_end_matches(0x22 as char).to_string());
            pending = Some(envelope_locator.clone());
        } else if let Some(rest) = line.strip_prefix(r#"        locator ""#) {
            assert!(pending.is_some(), "a row locator precedes no row source");
            pending = Some(rest.trim_end_matches(0x22 as char).to_string());
        }
    }
    if let Some(last) = pending.take() {
        pages.push(last);
    }

    assert_eq!(spans.len(), 5, "five row sources: {spans:?}");
    assert_eq!(pages.len(), 5, "one effective page per row: {pages:?}");
    let mut us = spans.clone();
    us.sort();
    us.dedup();
    assert_eq!(us.len(), 5, "no two rows share a span: {spans:?}");
    let mut up = pages.clone();
    up.sort();
    up.dedup();
    assert_eq!(up.len(), 5, "no two rows share a fact sheet: {pages:?}");
    // AND THE CONVENTION IS OBSERVED: a row restates `locator` only when its
    // page differs from the envelope's (ADJ-TABLES.md §4). Four of these
    // five do differ; the fifth is the vitamin-A row, whose page IS the
    // envelope's, so it inherits.
    assert_eq!(
        adj.matches("\n        locator \"").count(),
        4,
        "four rows restate a locator; the one on the envelope's own page inherits"
    );
    assert!(
        pages.contains(&envelope_locator),
        "and the inheriting row still resolves to the envelope's page: {pages:?}"
    );
    assert!(
        !adj.contains("\n    cites \""),
        "no corroboration survives at table level: each is now a row's own source"
    );
    assert!(
        adj.contains("    columns vitamin, symptom"),
        "the shipped column names are unchanged"
    );
}
