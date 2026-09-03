//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/landform-extent.adj`) driven through the
//! built CLI: a native `table` naming the "extent" (size) descriptor the
//! SAME USGS Feature Type Thesaurus spans already state for two landforms
//! -- a sibling to the already-shipped `landforms.adj` (which only carries
//! each landform's single descriptor), decoding the extent-clause half of
//! spans already sitting unused inside that table's own header. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a landform (canyon) the cited spans give no extent
//! descriptor for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_landformextent_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/landform-extent.adj");
    std::fs::copy(&src, dir.join("landform-extent.adj"))
        .expect("copy shipped landform-extent.adj");
}

#[test]
fn landform_extent_recalls_plateau_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-extent.adj\"\n\
         ? landform_extent(plateau, $Extent)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"landform_extent(plateau, great_extent)\""),
        "plateau is described as covering great extent: {out}"
    );
    assert!(
        out.contains("apps.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn landform_extent_recalls_backward_from_considerable_extent() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-extent.adj\"\n\
         ? landform_extent($Landform, considerable_extent)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"landform_extent(plain, considerable_extent)\""),
        "considerable_extent names plain: {out}"
    );
}

#[test]
fn landform_extent_abstains_honestly_on_canyon() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-extent.adj\"\n\
         ? landform_extent(canyon, $Extent)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "canyon's own cited span states no extent descriptor -- honest abstention: {out}"
    );
}

const LANDFORM_EXTENT_PIN: &str = r#""bindings":{"Extent":"great_extent"},"citations":[{"source":"Comparatively flat areas of great extent and elevation; specif. extensive land regions considerably above the adjacent country or above sea level; commonly limited on at least one side by an abrupt descent, have flat or nearly smooth surfaces but are often dissected by deep valleys and surmounted by high hills or mountains, and have a large part of their total surface at or near the summit level.","locator":"https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3","trust":"authoritative""#;

#[test]
fn landform_extent_citation_is_the_pages_whole_sentence() {
    let dir = scratch("reground");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-extent.adj\"\n? landform_extent(plateau, $Extent)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The value carried a MARKED trailing ellipsis on a 209-character prefix,
    // and the same 209 characters are shipped by two libraries.
    //
    // DROPPING THE ELLIPSIS ALONE WAS THE FIRST REPAIR AND IT WAS WORSE. It
    // removed a declared elision and left an undeclared one: a mid-sentence
    // truncation with no terminal punctuation and no signal that the sentence
    // continues. An ellipsis ADMITS its omission; a bare prefix HIDES one.
    // The value is now the page's complete 399-character sentence.
    assert!(
        out.contains(LANDFORM_EXTENT_PIN),
        "the plateau citation is a contiguous page span: {out}"
    );
}
