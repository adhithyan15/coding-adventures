//! End-to-end tests for ADJ-TABLES RS-5 — the native `table` construct — driven
//! through the built CLI binary. Five things are proven:
//!
//!   (a) EXACT LOOKUP: a self-contained inline `table` answers a binding query
//!       `? t(key, $V)` with the right value AND the table's citation, and a miss
//!       abstains honestly (no fabricated value) — all via the existing SLD path.
//!   (b) SHIPPED TABLE: the shipped `reference/length-conversions.adj` — the NIST
//!       exact length→metre factors — resolves through an `import`, carrying its
//!       locator. This is the artifact that unblocks the Facts front.
//!   (c) ARITY GUARD: a row whose cell count differs from the declared `columns`
//!       is a clean compile error, never a silently-mismatched relation.
//!   (d) PROVENANCE GUARD: a `table` with no `source` is rejected (a shipped
//!       table must be cited), mirroring the formula/relate write gate.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs5_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the CLI, returning (exit-ok, stdout, stderr) so the error-path tests can
/// assert on the diagnostic regardless of which stream it lands on.
fn run_full(program: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn write(dir: &Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
}

// ---------------------------------------------------------------------------
// (a) Exact lookup — inline table, hit carries a citation, miss abstains.
// ---------------------------------------------------------------------------

#[test]
fn table_exact_lookup_binds_value_with_citation() {
    let dir = scratch("exact");
    write(
        dir.as_path(),
        "case.adj",
        "table length_to_metres {\n\
         \x20   columns unit, metres\n\
         \x20   row (foot, 0.3048)\n\
         \x20   row (mile, 1609.344)\n\
         \x20   source \"Defined with respect to meter\"\n\
         \x20   locator \"https://www.nist.gov/pml/us-surveyfoot/revised-unit-conversion-factors\"\n\
         \x20   trust authoritative\n\
         }\n\
         ? length_to_metres(foot, $Metres)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The row value is returned EXACTLY (Big-number rendering), with the table's
    // provenance riding on the answer.
    assert!(
        out.contains("\"Metres\":\"0.3048\""),
        "binds the exact factor: {out}"
    );
    assert!(
        out.contains("Defined with respect to meter") && out.contains("\"trust\":\"authoritative\""),
        "carries the table's citation: {out}"
    );
    assert!(out.contains("\"abstained\":false"), "not an abstention: {out}");
}

#[test]
fn table_high_precision_pi_binds_all_39_digits() {
    // The exact-numbers win (ADJ-EXACT-NUMBERS NX-2): a table cell written to 39 decimal places
    // binds and RENDERS with every digit, instead of being truncated to the ~16 an `f64` carries
    // the moment it is parsed. This drives the full parse → store → query → render path through
    // the built CLI, proving the digits survive end-to-end.
    let dir = scratch("pi39");
    write(
        dir.as_path(),
        "case.adj",
        "table math_constant {\n\
         \x20   columns name, value\n\
         \x20   row (pi, 3.141592653589793238462643383279502884197)\n\
         \x20   source \"Wolfram MathWorld — Pi\"\n\
         \x20   locator \"https://mathworld.wolfram.com/Pi.html\"\n\
         \x20   trust authoritative\n\
         }\n\
         ? math_constant(pi, $V)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Every one of the 39 fractional digits is present in the binding — not the f64-truncated
    // prefix. Before NX-2 this came back as `3.141592653589793`.
    assert!(
        out.contains("\"V\":\"3.141592653589793238462643383279502884197\""),
        "pi binds ALL 39 decimal places exactly, not the f64-truncated ~16: {out}"
    );
    assert!(out.contains("\"abstained\":false"), "not an abstention: {out}");
}

#[test]
fn table_absent_key_abstains() {
    let dir = scratch("absent");
    write(
        dir.as_path(),
        "case.adj",
        "table length_to_metres {\n\
         \x20   columns unit, metres\n\
         \x20   row (foot, 0.3048)\n\
         \x20   source \"Defined with respect to meter\"\n\
         \x20   trust authoritative\n\
         }\n\
         ? length_to_metres(furlong, $Metres)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // A key not in the table is an honest abstention — the engine never invents.
    assert!(out.contains("\"abstained\":true"), "absent key abstains: {out}");
}

// ---------------------------------------------------------------------------
// (b) Shipped table — reference/length-conversions.adj resolves via import.
// ---------------------------------------------------------------------------

#[test]
fn shipped_length_conversions_table_resolves_with_locator() {
    let dir = scratch("shipped");
    // Copy the shipped table beside the entry program and import it by name.
    let src = stdlib().join("reference/length-conversions.adj");
    std::fs::copy(&src, dir.join("length-conversions.adj"))
        .expect("copy shipped length-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"length-conversions.adj\"\n\
         ? length_to_metres(mile, $Metres)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    assert!(
        out.contains("\"Metres\":\"1609.344\""),
        "shipped mile factor: {out}"
    );
    assert!(
        out.contains("revised-unit-conversion-factors"),
        "carries the NIST locator: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b2) Shipped table — reference/mass-conversions.adj resolves via import, and a
//      unit absent from the table abstains (never a fabricated factor).
// ---------------------------------------------------------------------------

#[test]
fn shipped_mass_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_mass");
    let src = stdlib().join("reference/mass-conversions.adj");
    std::fs::copy(&src, dir.join("mass-conversions.adj"))
        .expect("copy shipped mass-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"mass-conversions.adj\"\n\
         ? mass_to_kilograms(pound, $Kg)\n\
         ? mass_to_kilograms(short_ton, $Kg)\n\
         ? mass_to_kilograms(stone, $Kg)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 factors resolve, character-for-character from the table.
    assert!(
        out.contains("\"Kg\":\"0.4535924\""),
        "shipped pound factor: {out}"
    );
    assert!(
        out.contains("\"Kg\":\"907.1847\""),
        "shipped short-ton factor: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `stone` is not a row — the engine abstains rather than inventing a factor.
    assert!(
        out.contains("\"abstained\":true"),
        "an absent unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b3) Shipped table — reference/area-conversions.adj resolves via import (a
//      second dimension: AREA), and a unit absent from the table abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_area_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_area");
    let src = stdlib().join("reference/area-conversions.adj");
    std::fs::copy(&src, dir.join("area-conversions.adj"))
        .expect("copy shipped area-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"area-conversions.adj\"\n\
         ? area_to_square_metres(acre, $SqMetres)\n\
         ? area_to_square_metres(square_mile, $SqMetres)\n\
         ? area_to_square_metres(hectare, $SqMetres)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST exact-column factors resolve, character-for-character from the
    // table (digit-group spaces removed, no digit changed).
    assert!(
        out.contains("\"SqMetres\":\"4046.8564224\""),
        "shipped acre factor: {out}"
    );
    assert!(
        out.contains("\"SqMetres\":\"2589988.110336\""),
        "shipped square-mile factor: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("revised-unit-conversion-factors"),
        "carries the NIST locator: {out}"
    );
    // `hectare` is not a row — the engine abstains rather than inventing a factor.
    assert!(
        out.contains("\"abstained\":true"),
        "an absent unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b4) Shipped table — reference/volume-conversions.adj resolves via import (a
//      third dimension: VOLUME), and a unit absent from the table abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_volume_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_volume");
    let src = stdlib().join("reference/volume-conversions.adj");
    std::fs::copy(&src, dir.join("volume-conversions.adj"))
        .expect("copy shipped volume-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"volume-conversions.adj\"\n\
         ? volume_to_cubic_metres(gallon, $M3)\n\
         ? volume_to_cubic_metres(cubic_foot, $M3)\n\
         ? volume_to_cubic_metres(barrel, $M3)\n\
         ? volume_to_cubic_metres(litre, $M3)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 7-figure factors resolve, character-for-character from
    // the table (scientific notation converted to the same plain decimal).
    assert!(
        out.contains("\"M3\":\"0.003785412\""),
        "shipped U.S. gallon factor: {out}"
    );
    assert!(
        out.contains("\"M3\":\"0.02831685\""),
        "shipped cubic-foot factor: {out}"
    );
    assert!(
        out.contains("\"M3\":\"0.1589873\""),
        "shipped petroleum-barrel factor: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `litre` is SI, not a customary unit — not a row, so the engine abstains.
    assert!(
        out.contains("\"abstained\":true"),
        "an absent unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (c) Arity guard — a row of the wrong length is a clean compile error.
// ---------------------------------------------------------------------------

#[test]
fn table_row_arity_mismatch_is_a_compile_error() {
    let dir = scratch("arity");
    write(
        dir.as_path(),
        "case.adj",
        "table length_to_metres {\n\
         \x20   columns unit, metres\n\
         \x20   row (foot, 0.3048, extra)\n\
         \x20   source \"Defined with respect to meter\"\n\
         \x20   trust authoritative\n\
         }\n\
         ? length_to_metres(foot, $Metres)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "arity mismatch must fail: {out}{err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("TableArity") || combined.to_lowercase().contains("arity"),
        "diagnostic names the arity mismatch: {combined}"
    );
}

// ---------------------------------------------------------------------------
// (d) Provenance guard — an unsourced table is rejected.
// ---------------------------------------------------------------------------

#[test]
fn table_without_source_is_rejected() {
    let dir = scratch("nosrc");
    write(
        dir.as_path(),
        "case.adj",
        "table length_to_metres {\n\
         \x20   columns unit, metres\n\
         \x20   row (foot, 0.3048)\n\
         }\n\
         ? length_to_metres(foot, $Metres)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "unsourced table must fail: {out}{err}");
    let combined = format!("{out}{err}");
    assert!(
        combined.contains("TableMissingProvenance") || combined.to_lowercase().contains("provenance") || combined.to_lowercase().contains("source"),
        "diagnostic names the missing provenance: {combined}"
    );
}
