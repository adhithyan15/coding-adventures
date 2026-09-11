//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/mitosis-phase-ordinal-position.adj`) driven
//! through the built CLI: a `rule` composing the NEW `mitosis_phase_order`
//! table (`biology/mitosis-phase-order.adj`) with the already-shipped
//! `ordinal_number` table (`mathematics/ordinal-numbers.adj`, a
//! CROSS-DIRECTORY import via `../mathematics/ordinal-numbers.adj`, the
//! same shape `astronomy/planet-ordinal-position.adj` and
//! `astronomy/moon-phase-ordinal-position.adj` already established) to
//! DERIVE `mitosis_phase_ordinal_position($Phase, $Ordinal)` -- the FOURTH
//! cross-directory `rule` composition in this loop's science curriculum
//! sweep, and the FIRST in the biology domain. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_mitosisordinal_{tag}_{}", std::process::id()));
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

/// Copy BOTH shipped libraries, preserving their real relative directory
/// structure: `mitosis-phase-ordinal-position.adj` (in `biology/`) imports
/// `mitosis-phase-order.adj` (same dir) and
/// `../mathematics/ordinal-numbers.adj` (cross-directory), so the entry
/// program must sit at a root that contains both subtrees.
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for (rel_src, rel_dst) in [
        ("biology/mitosis-phase-order.adj", "biology/mitosis-phase-order.adj"),
        (
            "biology/mitosis-phase-ordinal-position.adj",
            "biology/mitosis-phase-ordinal-position.adj",
        ),
        (
            "mathematics/ordinal-numbers.adj",
            "mathematics/ordinal-numbers.adj",
        ),
    ] {
        let dst = dir.join(rel_dst);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel_src), &dst)
            .unwrap_or_else(|e| panic!("copy shipped {rel_src}: {e}"));
    }
}

#[test]
fn anaphase_derives_third_with_dual_citations() {
    let dir = scratch("anaphase");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/mitosis-phase-ordinal-position.adj\"\n\
         ? mitosis_phase_ordinal_position(anaphase, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"O\":\"third\""),
        "anaphase is the third phase of mitosis: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries: NCI
    // SEER (mitosis_phase_order) AND the ordinal-word convention
    // (ordinal_number).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derivation is a rule composing two fact steps: {out}"
    );
    // The SEER side is pinned as TEXT JOINED TO LOCATOR, not as a bare host.
    // A host-only probe is satisfied by any string whatsoever sitting in the
    // `source` slot -- it was satisfied by the fabricated composite this
    // installment removed ("The four phases of mitosis are Prophase ...
    // Metaphase ... Anaphase ... Telophase.", a string that occurs nowhere on
    // the page). Joining the pair also stops the two halves drifting apart
    // into a text from one citation and a locator from another.
    //
    // The needle CLOSES on `"corroborations":[]`, which buys two more things.
    // It bounds the locator at its RIGHT end -- an open needle was equally
    // happy with `cycle.html.attacker.example/`. And it pins the corroboration
    // list EMPTY: without that, a fabricated `cites` carrying the exact string
    // this installment deleted can be appended to the library and the whole
    // suite stays green. That is installment 4i's strongest finding, which
    // recurs wherever a provenance CONTAINER is left unbounded (#14735). The
    // needle is taken from the serialiser's real output, not from memory of
    // it. It therefore asserts something strong -- this citation and no
    // corroborations -- so a future genuine corroboration will redden it and
    // require a deliberate edit here. For a provenance pin that is the right
    // trade.
    //
    // The other half of the trade, stated plainly: a needle spanning four
    // adjacent keys also pins compact-JSON FIELD ORDER and separators. A key
    // renamed, a field inserted between `source` and `locator`, or output
    // pretty-printed will redden these for reasons that have nothing to do
    // with provenance. That is accepted deliberately: a serialisation change
    // SHOULD make someone re-read the provenance assertions.
    assert!(
        out.contains(
            "\"source\":\"The four phases of mitosis are\",\"locator\":\"https://training.seer.cancer.gov/disease/cancer/biology/cycle.html\",\"trust\":\"authoritative\",\"corroborations\":[]"
        ),
        "the SEER citation is the page's own contiguous lead-in, with its locator: {out}"
    );
    // The OTHER composed library gets the same treatment, and for the same
    // reason. Round 2 closed the SEER needle and left this one as the bare
    // host `ef.edu` -- so a fabricated `cites` appended to ordinal-numbers.adj
    // carrying the exact composite this installment deleted, under an
    // attacker-controlled locator, kept every assertion green. Half-closing a
    // provenance container is not closing it (#14735).
    //
    // BUT BE CLEAR ABOUT WHAT THIS NEEDLE DOES NOT DO, because the first
    // version of this comment overstated it. `mitosis-phase-ordinal-position`
    // carries its OWN INLINE COPY of this citation, and the duplicate covers
    // symmetrically IN BOTH DIRECTIONS: the copy satisfies this needle when
    // ordinal-numbers.adj drifts, and ordinal-numbers.adj satisfies it when the
    // rule drifts. Four single-file mutations therefore ship green -- either
    // side's `source` text and either side's locator (including a look-alike
    // suffix). Both copies would have to be mutated together to redden this.
    // So this needle constrains the SHAPE of the citation, not its content;
    // the SEER needle above genuinely does constrain content, which is why the
    // same mutation reddens there and not here. Filed as #14745.
    assert!(
        out.contains(
            "\"source\":\"1 one first, 2 two second, 3 three third, 4 four fourth, 5 five fifth\",\"locator\":\"https://www.ef.edu/english-resources/english-grammar/numbers-english/\",\"trust\":\"consensus\",\"corroborations\":[]"
        ),
        "carries citations from BOTH composed libraries (mitosis-phase-order.adj and ordinal-numbers.adj): {out}"
    );
    // AND A STRUCTURAL BOUND OVER THE WHOLE OUTPUT, which is the only thing here
    // that actually closes the container class (#14735).
    //
    // Why the needles above are not enough, which took three review rounds and a
    // surviving mutant to learn: `mitosis-phase-ordinal-position.adj:64` carries
    // its OWN INLINE COPY of the ordinal-numbers citation. A `contains` needle is
    // satisfied by that copy, so it says nothing whatever about the composed
    // library -- append a fabricated `cites` to `ordinal-numbers.adj` and the
    // needle still matches the rule's duplicate while the library drifts. A pin
    // matched by a duplicate is a pin on the duplicate. That is filed as #14745:
    // this assertion catches a fabricated cites on any library, but NOT a drift in
    // the composed library's own text, which the duplicate still covers for it.
    //
    // This assertion does not name a citation at all. It says that NO citation
    // anywhere in this derivation carries a corroboration, which is true of every
    // library involved and cannot be satisfied by a copy.
    assert!(
        !out.contains("\"corroborations\":[{"),
        "no citation in this derivation carries a corroboration; a non-empty one means a `cites` was introduced in the rule or in either composed library: {out}"
    );
}

#[test]
fn first_reverse_binds_to_prophase() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/mitosis-phase-ordinal-position.adj\"\n\
         ? mitosis_phase_ordinal_position($P, first)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"prophase\""),
        "prophase is the first phase of mitosis: {out}"
    );
}

#[test]
fn interphase_abstains_honestly_as_not_one_of_the_four_ordered_phases() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/mitosis-phase-ordinal-position.adj\"\n\
         ? mitosis_phase_ordinal_position(interphase, $O)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "interphase is the resting phase BETWEEN divisions, not one of the four ordered mitotic phases -- honest abstention, never invented: {out}"
    );
}
