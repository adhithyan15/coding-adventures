//! End-to-end test for the earth-science CLOUD-TYPES facts library
//! (`adj-facts-stdlib/earth-science/cloud-types.adj`) driven through the built
//! CLI: a native `table` of cloud → altitude level resolves a binding-query
//! recall carrying the NOAA / National Weather Service citation, and abstains on
//! a word the page never assigns a level — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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

#[test]
fn earth_science_cloud_altitude_recall_binds_level_with_citation() {
    let dir = scratch("clouds");
    // Copy the shipped cloud-altitude table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/cloud-types.adj");
    std::fs::copy(&src, dir.join("cloud-types.adj")).expect("copy shipped cloud-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-types.adj\"\n\
         ? cloud_altitude(cirrus, $Level)\n\
         ? cloud_altitude(cumulus, $Level)\n\
         ? cloud_altitude(fog, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Cirrus is a high cloud; cumulus is a low cloud — the recalled levels.
    assert!(out.contains("\"Level\":\"high\""), "cirrus → high: {out}");
    assert!(out.contains("\"Level\":\"low\""), "cumulus → low: {out}");
    // The answer carries the NOAA / NWS (weather.gov) citation as its proof --
    // pinned as TEXT, not by locator and trust tier alone.
    //
    // This assertion used to be `contains(locator) && contains(trust)`, which
    // says nothing whatever about what the `source` field CONTAINS. It was
    // satisfied by the welded three-sentence string this installment removed --
    // a string that occurs nowhere on the page, because its three sentences sit
    // 1443 and 1446 characters apart -- exactly as happily as it is satisfied by
    // the three real spans that replaced it.
    //
    // The needle is the whole citation object as the serialiser actually emits
    // it, taken from a real run rather than from memory of the format, and it
    // CLOSES on the corroborations `]`. That bounds four things at once: the
    // source text, the locator, the trust tier, and the corroboration SET -- so
    // a fabricated `cites` cannot be appended without reddening (#14735).
    //
    // Its string appears FOUR times in this test's output. That was checked
    // BEFORE trusting the needle, not after: the serialiser echoes each citation
    // in `citations` and in `steps` -- two sections per answer -- and the program
    // ABOVE issues three queries of which two bind, so 2 x 2. None of the four is
    // an independent copy: mutating the `.adj` takes all four together.
    // Where a duplicate CAN drift -- a rule inlining a copy of a composed
    // library's citation -- a `contains` needle constrains the copy and not the
    // library, which is #14745.
    assert!(
        out.contains(
            "\"source\":\"The three main types of high clouds are cirrus, cirrostratus, and cirrocumulus.\",\"locator\":\"https://www.weather.gov/lmk/cloud_classification\",\"trust\":\"authoritative\",\"corroborations\":[{\"source\":\"The two main type of mid-level clouds are altostratus and altocumulus.\",\"locator\":\"https://www.weather.gov/lmk/cloud_classification\"},{\"source\":\"The two main types of low clouds include stratus, which develop horizontally, and cumulus, which develop vertically.\",\"locator\":\"https://www.weather.gov/lmk/cloud_classification\"}]"
        ),
        "carries the NOAA/NWS citation, its two corroborations, and nothing else: {out}"
    );
    // Fog is never assigned a level on the page — honest abstention, never a
    // fabricated level.
    assert!(out.contains("\"abstained\":true"), "fog abstains: {out}");
}

#[test]
fn earth_science_cloud_altitude_recall_binds_newly_added_rows() {
    let dir = scratch("clouds_new");
    let src = facts_stdlib().join("earth-science/cloud-types.adj");
    std::fs::copy(&src, dir.join("cloud-types.adj")).expect("copy shipped cloud-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-types.adj\"\n\
         ? cloud_altitude(cirrocumulus, $Level)\n\
         ? cloud_altitude(altocumulus, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The two rows THIS TEST BINDS -- both among the three added in #11015,
    // which also added cirrostratus -- are named by DIFFERENT SENTENCES, but
    // inside the ONE citation this table emits, not by different citations.
    // The grammar gives a `table` a single citation block, and the engine
    // attaches the whole object -- `source` plus both corroborations -- to
    // every row, so these two answers' `citations` sections are byte-identical.
    // Within it, `cirrocumulus` is named by the `source` (the high-deck
    // sentence) and `altocumulus` by the FIRST CORROBORATION (the mid-level
    // sentence). Which means a mid-level answer's PRIMARY source is a sentence
    // that does not name it; the sentence that does is a corroboration.
    //
    // This comment has been wrong twice. It first said both rows came from the
    // SAME already-quoted source sentence -- true only while one `source` field
    // held three welded sentences, and made false by installment 4k's split.
    // The correction then said they were grounded by DIFFERENT CITATIONS, which
    // is also not what the output shows. Both are recorded because the second
    // was introduced while fixing the first.
    assert!(out.contains("\"Level\":\"high\""), "cirrocumulus → high: {out}");
    assert!(out.contains("\"Level\":\"middle\""), "altocumulus → middle: {out}");
}
