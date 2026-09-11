//! End-to-end test for the geography reference-lines FACTS library
//! (`adj-facts-stdlib/geography/reference-lines.adj`): a native `table` of
//! reference line → what it marks resolves forward AND reverse binding queries
//! with the NOAA citation, and abstains on a line the table does not ground —
//! 0 answer-time model calls.

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
fn geography_reference_lines_recall_binds_marks_with_citation_and_abstains() {
    let dir = scratch("referencelines");
    // Copy the shipped geography table beside the entry program and import it.
    let src = facts_stdlib().join("geography/reference-lines.adj");
    std::fs::copy(&src, dir.join("reference-lines.adj"))
        .expect("copy shipped reference-lines.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-lines.adj\"\n\
         ? reference_line(equator, $Marks)\n\
         ? reference_line($Line, zero_degrees_longitude)\n\
         ? reference_line(international_date_line, $Marks)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // (a) Forward: the equator marks 0 degrees latitude — the source's atom.
    assert!(
        out.contains("\"Marks\":\"zero_degrees_latitude\""),
        "equator → zero_degrees_latitude: {out}"
    );
    // Reverse: the line at 0 degrees longitude is the prime meridian.
    assert!(
        out.contains("\"Line\":\"prime_meridian\""),
        "0 deg longitude → prime_meridian: {out}"
    );
    // First, the welded value itself must not come back. This names the
    // DEFECT rather than a citation, so no duplicate elsewhere in the output
    // can satisfy it on the real one's behalf. It is asserted BEFORE the
    // citation object deliberately: with the order the other way round, a
    // RESTORE mutant tripped the citation assertion first and this one was
    // never once observed to fire.
    assert!(
        !out.contains(
            "usually December 21. Two other significant lines of latitude"
        ),
        "the two NESDIS spans are not welded back into one: {out}"
    );
    // (a cont.) The answer carries the NOAA citation as its proof.
    //
    // The pin here used to be a HOSTNAME and a TRUST TIER:
    // `contains("oceanservice.noaa.gov") && contains(trust)`. Both were
    // satisfied by the welded 392-character NESDIS `cites` that #13934
    // installment 4m split -- a string that occurs ZERO times on the page it
    // named -- exactly as happily as by the two real spans that replaced it.
    // 4l's oceans pin had the same shape and the same hole.
    //
    // The needle is the whole citation object as the serialiser actually
    // emits it, taken from a real run rather than from memory of the format,
    // and it CLOSES on the corroborations `]`. That bounds four things at
    // once: the source text, the locator, the trust tier, and the
    // corroboration SET -- so a fabricated `cites` cannot be appended without
    // reddening (#14735).
    //
    // Its string appears FOUR times in this test's output, at four distinct
    // JSON paths: `recall/[0]/answers/[0]/citations/[0]` and `.../steps/[0]`,
    // and the same pair under `recall/[1]` for the reverse query. That was
    // counted BEFORE trusting the needle, and six mutations of the `.adj`
    // each drove all four to zero together -- echoes of one field, not the
    // independently-driftable copies of #14745.
    assert!(
        out.contains(
            "\"source\":\"The equator is the most well known parallel. At 0 degrees latitude, it equally divides the Earth into the Northern and Southern hemispheres.\",\"locator\":\"https://oceanservice.noaa.gov/facts/latitude.html\",\"trust\":\"authoritative\",\"corroborations\":[{\"source\":\"The prime meridian, which runs through Greenwich, England, has a longitude of 0 degrees. It divides the Earth into the eastern and western hemispheres.\",\"locator\":\"https://oceanservice.noaa.gov/facts/longitude.html\"},{\"source\":\"In the Northern Hemisphere, the Summer Solstice occurs when the sun is directly above the Tropic of Cancer, usually June 21. In the Southern Hemisphere, the Summer Solstice occurs when the sun is directly above the Tropic of Capricorn, usually December 21.\",\"locator\":\"https://www.nesdis.noaa.gov/about/k-12-education/optical-phenomena/what-solstice\"},{\"source\":\"Two other significant lines of latitude are the Arctic Circle (around the North Pole) and the Antarctic Circle (around the South Pole).\",\"locator\":\"https://www.nesdis.noaa.gov/about/k-12-education/optical-phenomena/what-solstice\"}]"
        ),
        "carries the NOAA citation, its three corroborations, and nothing else: {out}"
    );
    // (b) The international date line is a real line but is NOT grounded in a row
    // here — honest abstention, never a fabricated `marks` atom.
    assert!(
        out.contains("\"abstained\":true"),
        "international_date_line abstains: {out}"
    );
}
