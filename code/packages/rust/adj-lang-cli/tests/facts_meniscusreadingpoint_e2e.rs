//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/meniscus-reading-point.adj`) driven through
//! the built CLI: a native `table` naming the two basic shapes a liquid's
//! meniscus can take in a laboratory measuring vessel (concave or convex)
//! and which point of its curve (lowest or highest) is actually used to
//! take the reading, grounding NIST's "Good Measurement Practice for Method
//! of Reading a Meniscus" (GMP 3, NIST Interagency Report NIST.IR.7383-2019).
//! Runs the relation BACKWARD as a genuine recall in both directions, and
//! abstains honestly on `flat` -- a meniscus shape the cited document never
//! names. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_meniscusreadingpoint_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("chemistry/meniscus-reading-point.adj");
    std::fs::copy(&src, dir.join("meniscus-reading-point.adj"))
        .expect("copy shipped meniscus-reading-point.adj");
}

#[test]
fn meniscus_reading_point_recall_binds_point_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"meniscus-reading-point.adj\"\n\
         ? meniscus_reading_point(concave, $Point)\n\
         ? meniscus_reading_point(convex, $Point)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("meniscus_reading_point(concave, lowest_point)"),
        "a concave meniscus is read at its lowest point: {out}"
    );
    assert!(
        out.contains("meniscus_reading_point(convex, highest_point)"),
        "a convex meniscus is read at its highest point: {out}"
    );
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST citation at authoritative trust: {out}"
    );
}

#[test]
fn meniscus_reading_point_reverse_binds_shape_from_point() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"meniscus-reading-point.adj\"\n\
         ? meniscus_reading_point($Shape, lowest_point)\n\
         ? meniscus_reading_point($Shape, highest_point)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The relation runs BACKWARD: binding `lowest_point` recalls `concave`,
    // and binding `highest_point` recalls `convex`.
    assert!(
        out.contains("meniscus_reading_point(concave, lowest_point)"),
        "lowest_point recalls concave: {out}"
    );
    assert!(
        out.contains("meniscus_reading_point(convex, highest_point)"),
        "highest_point recalls convex: {out}"
    );
}

#[test]
fn meniscus_reading_point_abstains_honestly_on_flat() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"meniscus-reading-point.adj\"\n\
         ? meniscus_reading_point(flat, $Point)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a perfectly flat liquid surface is never named as a real meniscus shape by the \
         cited NIST document, which discusses exactly two shapes, concave and convex -- \
         honest abstention, never invented: {out}"
    );
}
