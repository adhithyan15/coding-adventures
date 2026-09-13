//! End-to-end test for the metrology SI-BASE-UNITS facts library
//! (`adj-facts-stdlib/metrology/si-base-units.adj`) driven through the built CLI:
//! a native `table` of base-quantity → unit → symbol resolves a binding-query
//! recall with the NIST citation, runs the relation backwards (unit → quantity),
//! and abstains on anything that is not one of the seven base quantities — 0
//! model calls, never a fabricated unit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factssi_{tag}_{}", std::process::id()));
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

fn with_case(dir: &Path, body: &str) -> PathBuf {
    let src = facts_stdlib().join("metrology/si-base-units.adj");
    std::fs::copy(&src, dir.join("si-base-units.adj")).expect("copy shipped si-base-units.adj");
    let p = dir.join("case.adj");
    std::fs::write(&p, format!("import \"si-base-units.adj\"\n{body}")).unwrap();
    p
}

#[test]
fn si_base_unit_forward_recall_binds_unit_and_symbol_with_citation() {
    let dir = scratch("forward");
    let p = with_case(
        &dir,
        "? si_base_unit(mass, $Unit, $Symbol)\n\
         ? si_base_unit(temperature, $Unit, $Symbol)\n",
    );

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Mass -> kilogram (kg); temperature -> kelvin (K).
    assert!(
        out.contains("\"Unit\":\"kilogram\""),
        "mass binds the kilogram: {out}"
    );
    assert!(out.contains("kg"), "mass carries the symbol kg: {out}");
    assert!(
        out.contains("\"Unit\":\"kelvin\""),
        "temperature binds the kelvin: {out}"
    );
    // The answer carries the NIST citation as its proof.
    assert!(
        out.contains("nist.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NIST source citation: {out}"
    );
}

#[test]
fn si_base_unit_runs_backwards_from_unit_to_quantity() {
    let dir = scratch("reverse");
    // Given the unit `second`, recall the base quantity it measures — time.
    let p = with_case(&dir, "? si_base_unit($Quantity, second, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Quantity\":\"time\""),
        "the second measures time (reverse lookup): {out}"
    );
}

#[test]
fn a_non_base_quantity_abstains_rather_than_inventing_a_unit() {
    let dir = scratch("abstain");
    // Luminance is a real photometric quantity but NOT one of the seven SI base
    // quantities — the table must abstain, never fabricate a unit.
    let p = with_case(&dir, "? si_base_unit(luminance, $Unit, $Symbol)\n");

    let (ok, out) = run(&p);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a non-base quantity abstains: {out}"
    );
}

/// The envelope span — what defends the TABLE, not any one row. It states that
/// the SI has seven base units and names none of them, so it must no longer be
/// the warrant on a row answer.
const SI_ENVELOPE_SPAN: &str = r#""source":"The SI is made up of 7 base units that define the 22 derived units with special names and symbols, which are illustrated in NIST SP 1247, SI Base Units Relationship Poster.""#;

/// #14986: every row of this table used to answer with the envelope sentence,
/// which names none of the seven units. A recall of `mass` shipped NIST, an
/// `authoritative` tier, and a sentence that does not contain "kilogram".
///
/// The previous version of this test pinned that envelope on a row query and
/// said so in its own comment: *"a row-binding pin today would freeze the
/// #14124 defect into a test"*, and it recorded all seven per-row spans as
/// present on the page, *"Deferred to #14124 with the text recorded, NOT
/// unavailable."* Those spans are now shipped as RS-5e per-row `source`s, so
/// the sound row-binding pin that comment was waiting for is available, and
/// this is it.
fn assert_row_carries_its_own_span(tag: &str, query: &str, binding: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("metrology/si-base-units.adj"),
        dir.join("si-base-units.adj"),
    )
    .expect("copy shipped si-base-units.adj");
    let case = with_case(&dir, query);
    let (ok, out) = run(&case);
    assert!(ok, "cli should succeed: {out}");

    // The row resolves. A provenance assertion over a row the engine never
    // reaches proves nothing.
    assert!(out.contains(binding), "query binds {binding}: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so every needle below belongs to THIS row: {out}"
    );

    // The row's own span, key-anchored to `source` so a loose substring
    // elsewhere in the output cannot satisfy it. Each of these occurs EXACTLY
    // ONCE on the NIST page — in the raw HTML, under a block-only extractor,
    // and under a crude every-tag-is-a-break one. The separator is SPACE,
    // U+002D HYPHEN-MINUS, SPACE, read out of the page rather than assumed;
    // an en-dash variant and a no-spaces variant each occur zero times.
    assert!(
        out.contains(&format!("\"source\":\"{span}\"")),
        "row is warranted by the page's own line for it ({span}): {out}"
    );

    // `locator` and `trust` are NOT restated per row — all seven rows come
    // from the one NIST page at one tier, and a row inherits every field it
    // does not write.
    //
    // The `locator` pin is load-bearing: deleting the envelope's `locator`
    // reddens this.
    assert!(
        out.contains("\"locator\":\"https://www.nist.gov/pml/owm/metric-si/si-units\""),
        "row inherits the envelope locator: {out}"
    );
    // The `trust` pin is WEAKER THAN IT LOOKS, and saying so is the honest
    // version. `annotations_to_provenance` (adj-lang/src/lower.rs:2622)
    // defaults a tier to Authoritative whenever a `source` is present, so this
    // cannot distinguish "inherited from the envelope's declared tier" from
    // "silently defaulted because the row has a source" — deleting the
    // envelope's `trust authoritative` leaves this GREEN. What it does
    // discriminate is the other four tiers: setting the envelope to
    // `trust inferred` propagates and reddens it.
    //
    // It is kept because `authoritative` is the correct tier here for a reason
    // worth pinning: the span STATES the row, so the claim is read, not
    // reasoned.
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "tier is authoritative — the span STATES the row, read not reasoned: {out}"
    );

    // NAMED NEGATIVE, and the whole point of #14986: the framing sentence is
    // no longer any row's warrant.
    assert!(
        !out.contains(SI_ENVELOPE_SPAN),
        "the framing sentence does not warrant a row: {out}"
    );
}

#[test]
fn si_base_unit_mass_row_carries_the_pages_own_line() {
    assert_row_carries_its_own_span(
        "sibaseunitsmass",
        "? si_base_unit(mass, $Unit, $Symbol)",
        "\"Unit\":\"kilogram\"",
        "Mass - kilogram (kg)",
    );
}

#[test]
fn si_base_unit_candela_row_carries_the_pages_own_line_in_reverse() {
    assert_row_carries_its_own_span(
        "sibaseunitscandela",
        "? si_base_unit($Quantity, candela, $Symbol)",
        "\"Quantity\":\"luminous_intensity\"",
        "Luminous intensity - candela (cd)",
    );
}
