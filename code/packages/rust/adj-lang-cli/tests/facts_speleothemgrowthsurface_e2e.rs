//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/speleothem-growth-surface.adj`) driven
//! through the built CLI: a native `table` recording which cave surface a
//! named dripstone speleothem grows from, grounding the U.S. National Park
//! Service's "Speleothems" article.
//!
//! The FIRST cave/karst library in this stdlib. "Which one hangs from the
//! ceiling -- the stalactite or the stalagmite?" is the most-confused pair
//! in elementary Earth science, and exactly the kind of question that
//! should be answered from a citation rather than from a mnemonic.
//!
//! The abstentions carry as much of this library's content as the two rows
//! do, so two of the five tests below are about what the table declines to
//! say:
//!   * `column` -- the source itself explains why: "Columns are not
//!     stalactites nor are they stalagmites; they are both, together." A
//!     column is produced by two speleothems JOINING, so it has no single
//!     growth surface to bind. Recording either surface would be false.
//!   * `helictite` -- the source DOES place it ("cave ceilings, walls, and
//!     less often on cave floors"), but on three surfaces with a frequency
//!     hedge on the third. One surface would drop the others; three as
//!     equals would flatten the hedge. It needs a different shape than this
//!     table's one-surface-per-speleothem relation.
//!
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
    let dir = std::env::temp_dir().join(format!(
        "adjcli_factsspeleothem_{tag}_{}",
        std::process::id()
    ));
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

fn place(dir: &Path) {
    let src = facts_stdlib().join("earth-science/speleothem-growth-surface.adj");
    std::fs::copy(&src, dir.join("speleothem-growth-surface.adj"))
        .expect("copy shipped speleothem-growth-surface.adj");
}

#[test]
fn speleothem_growth_surface_settles_the_classic_confusion() {
    let dir = scratch("classic");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-growth-surface.adj\"\n\
         ? speleothem_growth_surface(stalactite, $S)\n\
         ? speleothem_growth_surface(stalagmite, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN. A fragment needle in this file
    // matched only part of the sentence, so the citation could be
    // truncated AT that point -- deleting everything after it -- while
    // the test stayed green. Anchoring on the `"source":"` key and
    // closing on the terminating quote pins head, tail, punctuation and
    // length at once.
    //
    // Several tests load this library, because siblings import it as a
    // dependency. The pin belongs in its OWN test: the others are not
    // responsible for its provenance. That is also why the assertion has
    // to be unique -- where a co-loaded sibling carries a byte-identical
    // citation, an assertion either one satisfies pins neither.
    // See issues #13916 and #13918.
    assert!(
        out.contains("\"source\":\"Stalactites are the most common and most familiar of all speleothems; they resemble icicles or carrots hanging from cave ceilings.\""),
        "the citation is the whole source sentence, exactly: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The pair the whole library exists to disambiguate.
    assert!(
        out.contains("\"S\":\"cave_ceiling\""),
        "stalactites hang from the ceiling: {out}"
    );
    assert!(
        out.contains("\"S\":\"cave_floor\""),
        "stalagmites build up from the floor: {out}"
    );
    // And the answer must not be a mnemonic -- it arrives with its source.
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn speleothem_growth_surface_carries_both_grounding_sentences() {
    let dir = scratch("cites");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-growth-surface.adj\"\n\
         ? speleothem_growth_surface(stalactite, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Two rows from two different sentences: one `source`, one `cites`.
    for sentence in [
        "they resemble icicles or carrots hanging from cave ceilings.",
        "Stalagmites are convex floor deposits built up by water dripping from an overhead stalactite",
    ] {
        assert!(
            out.contains(sentence),
            "grounding sentence carried: {sentence}: {out}"
        );
    }
}

#[test]
fn speleothem_growth_surface_runs_backward_from_the_surface() {
    let dir = scratch("reverse");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-growth-surface.adj\"\n\
         ? speleothem_growth_surface($P, cave_floor)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // "Which one grows up from the floor?" is the direction the classic
    // confusion actually runs in.
    assert!(
        out.contains("speleothem_growth_surface(stalagmite, cave_floor)"),
        "the floor-grown one is the stalagmite: {out}"
    );
}

#[test]
fn speleothem_growth_surface_abstains_on_the_column_which_is_both() {
    let dir = scratch("column");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-growth-surface.adj\"\n\
         ? speleothem_growth_surface(column, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "column has no single growth surface, so the recall abstains: {out}"
    );
    // Guard the specific wrong answers this design avoids. A column is
    // formed when a stalagmite grows together with its feeder stalactite --
    // it is BOTH, so naming either surface would be false, and naming both
    // would misrepresent the relation this table holds.
    for surface in ["cave_ceiling", "cave_floor"] {
        assert!(
            !out.contains(&format!("speleothem_growth_surface(column, {surface})")),
            "must not place the column on {surface}: {out}"
        );
    }
}

#[test]
fn speleothem_growth_surface_abstains_on_the_multi_surface_helictite() {
    let dir = scratch("helictite");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speleothem-growth-surface.adj\"\n\
         ? speleothem_growth_surface(helictite, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The source DOES place helictites -- "on cave ceilings, walls, and
    // less often on cave floors" -- but on three surfaces with a frequency
    // hedge on the third. Tabling one would silently drop the others;
    // tabling all three as equals would flatten the "less often" the source
    // deliberately states. Abstaining is the honest option until a shape
    // exists that can carry the hedge.
    assert!(
        out.contains("\"abstained\":true"),
        "helictite abstains rather than being distorted into one surface: {out}"
    );
    assert!(
        !out.contains("speleothem_growth_surface(helictite, cave_ceiling)"),
        "must not pick one of the three surfaces as THE surface: {out}"
    );
}
