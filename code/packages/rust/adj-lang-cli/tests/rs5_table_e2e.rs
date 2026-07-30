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
// (b5) Shipped table — reference/si-prefixes.adj resolves via import (prefix →
//      power-of-ten exponent, incl. NEGATIVE exponents), and an absent prefix
//      abstains. This is the original concrete trigger for the `table` construct.
// ---------------------------------------------------------------------------

#[test]
fn shipped_si_prefixes_table_resolves_signed_exponents_and_abstains() {
    let dir = scratch("shipped_siprefix");
    let src = stdlib().join("reference/si-prefixes.adj");
    std::fs::copy(&src, dir.join("si-prefixes.adj")).expect("copy shipped si-prefixes.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"si-prefixes.adj\"\n\
         ? si_prefix_to_exponent(kilo, $E)\n\
         ? si_prefix_to_exponent(milli, $E)\n\
         ? si_prefix_to_exponent(pico, $E)\n\
         ? si_prefix_to_exponent(zetta, $E)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST prefix → integer-exponent rows resolve, including the negatives.
    assert!(out.contains("\"E\":\"3\""), "kilo = 3: {out}");
    assert!(out.contains("\"E\":\"-3\""), "milli = -3 (negative exponent): {out}");
    assert!(out.contains("\"E\":\"-12\""), "pico = -12: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("metric-si-prefixes"),
        "carries the NIST SI-prefixes locator: {out}"
    );
    // `zetta` is a real SI prefix but not in this row set — the engine abstains
    // rather than inventing an exponent.
    assert!(
        out.contains("\"abstained\":true"),
        "an absent prefix abstains, never a fabricated exponent: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b6) Shipped table — reference/time-conversions.adj resolves via import (time
//      unit → seconds, EXACT defined factors), and `year` — which has no single
//      exact factor and therefore no row — abstains honestly.
// ---------------------------------------------------------------------------

#[test]
fn shipped_time_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_time");
    let src = stdlib().join("reference/time-conversions.adj");
    std::fs::copy(&src, dir.join("time-conversions.adj"))
        .expect("copy shipped time-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"time-conversions.adj\"\n\
         ? time_to_seconds(minute, $S)\n\
         ? time_to_seconds(hour, $S)\n\
         ? time_to_seconds(day, $S)\n\
         ? time_to_seconds(year, $S)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Time" exact factors resolve, character-for-character
    // from the table (boldface = exact; scientific notation to plain integer).
    assert!(out.contains("\"S\":\"60\""), "minute = 60 s: {out}");
    assert!(out.contains("\"S\":\"3600\""), "hour = 3600 s: {out}");
    assert!(out.contains("\"S\":\"86400\""), "day = 86400 s: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `year` is NOT a single exact factor (SP 811 lists only non-exact,
    // inequivalent variants) — no row, so the engine abstains rather than
    // committing to one of several lengths.
    assert!(
        out.contains("\"abstained\":true"),
        "year abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b7) Shipped table — reference/energy-conversions.adj resolves via import (energy
//      unit → joules, EXACT SP 811 B.9 boldface factors), and a non-exact unit
//      (`btu`) — which has no single exact factor and therefore no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_energy_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_energy");
    let src = stdlib().join("reference/energy-conversions.adj");
    std::fs::copy(&src, dir.join("energy-conversions.adj"))
        .expect("copy shipped energy-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"energy-conversions.adj\"\n\
         ? energy_to_joules(calorie_th, $J)\n\
         ? energy_to_joules(kilowatt_hour, $J)\n\
         ? energy_to_joules(erg, $J)\n\
         ? energy_to_joules(btu, $J)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Energy" exact factors resolve, character-for-character
    // from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"J\":\"4.184\""), "thermochemical calorie = 4.184 J: {out}");
    assert!(out.contains("\"J\":\"3600000\""), "kilowatt-hour = 3600000 J: {out}");
    assert!(out.contains("\"J\":\"0.0000001\""), "erg = 1e-7 J: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `btu` is NOT a single exact factor (SP 811 lists it as a rounded measured
    // value) — no row, so the engine abstains rather than commit to a rounded factor.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b8) Shipped table — reference/pressure-conversions.adj resolves via import
//      (pressure/stress unit → pascals, EXACT SP 811 B.9 boldface factors), and a
//      non-exact unit (`torr`) — a rounded measured factor with no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_pressure_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_pressure");
    let src = stdlib().join("reference/pressure-conversions.adj");
    std::fs::copy(&src, dir.join("pressure-conversions.adj"))
        .expect("copy shipped pressure-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"pressure-conversions.adj\"\n\
         ? pressure_to_pascals(atmosphere_standard, $Pa)\n\
         ? pressure_to_pascals(bar, $Pa)\n\
         ? pressure_to_pascals(kilogram_force_per_square_meter, $Pa)\n\
         ? pressure_to_pascals(torr, $Pa)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Pressure or stress" exact factors resolve, character-for-
    // character from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"Pa\":\"101325\""), "standard atmosphere = 101325 Pa: {out}");
    assert!(out.contains("\"Pa\":\"100000\""), "bar = 100000 Pa: {out}");
    assert!(out.contains("\"Pa\":\"9.80665\""), "kgf/m² = 9.80665 Pa: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `torr` is NOT a single exact factor (SP 811 lists it as a rounded measured
    // value, not boldface) — no row, so the engine abstains rather than commit.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b9) Shipped table — reference/speed-conversions.adj resolves via import (speed
//      unit → metres per second, EXACT SP 811 B.9 boldface factors), and a non-exact
//      unit (`kilometer_per_hour`) — a rounded factor with no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_speed_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_speed");
    let src = stdlib().join("reference/speed-conversions.adj");
    std::fs::copy(&src, dir.join("speed-conversions.adj"))
        .expect("copy shipped speed-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"speed-conversions.adj\"\n\
         ? speed_to_metres_per_second(mile_per_hour, $mps)\n\
         ? speed_to_metres_per_second(foot_per_second, $mps)\n\
         ? speed_to_metres_per_second(mile_per_second, $mps)\n\
         ? speed_to_metres_per_second(kilometer_per_hour, $mps)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Speed or velocity" exact factors resolve, character-for-
    // character from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"mps\":\"0.44704\""), "mile/hour = 0.44704 m/s: {out}");
    assert!(out.contains("\"mps\":\"0.3048\""), "foot/second = 0.3048 m/s: {out}");
    assert!(out.contains("\"mps\":\"1609.344\""), "mile/second = 1609.344 m/s: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `kilometer_per_hour` is NOT a single exact factor (SP 811 lists 1000/3600 as a
    // rounded value, not boldface) — no row, so the engine abstains rather than commit.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b10) Shipped table — reference/force-conversions.adj resolves via import (force
//       unit → newtons, EXACT SP 811 B.9 boldface factors), and a non-exact unit
//       (`pound_force`) — a rounded factor with no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_force_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_force");
    let src = stdlib().join("reference/force-conversions.adj");
    std::fs::copy(&src, dir.join("force-conversions.adj"))
        .expect("copy shipped force-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"force-conversions.adj\"\n\
         ? force_to_newtons(kilogram_force, $n)\n\
         ? force_to_newtons(kilopond, $n)\n\
         ? force_to_newtons(dyne, $n)\n\
         ? force_to_newtons(pound_force, $n)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Force" exact factors resolve, character-for-character from
    // the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"n\":\"9.80665\""), "kilogram-force = 9.80665 N: {out}");
    assert!(out.contains("\"n\":\"0.00001\""), "dyne = 0.00001 N: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `pound_force` is NOT a single exact factor (SP 811 prints it rounded, not
    // boldface) — no row, so the engine abstains rather than commit to a rounded value.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b11) Shipped table — reference/acceleration-conversions.adj resolves via import
//       (acceleration unit → metres per second squared, EXACT SP 811 B.9 boldface
//       factors, incl. the conventional standard gravity gₙ), and the SI unit itself
//       (`metre_per_second_squared`) — which has no customary-conversion row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_acceleration_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_accel");
    let src = stdlib().join("reference/acceleration-conversions.adj");
    std::fs::copy(&src, dir.join("acceleration-conversions.adj"))
        .expect("copy shipped acceleration-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"acceleration-conversions.adj\"\n\
         ? acceleration_to_metres_per_second_squared(free_fall_standard, $a)\n\
         ? acceleration_to_metres_per_second_squared(foot_per_second_squared, $a)\n\
         ? acceleration_to_metres_per_second_squared(gal, $a)\n\
         ? acceleration_to_metres_per_second_squared(metre_per_second_squared, $a)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Acceleration" exact factors resolve, character-for-
    // character from the table (boldface = exact; scientific notation to plain decimal).
    assert!(
        out.contains("\"a\":\"9.80665\""),
        "standard acceleration of free fall gₙ = 9.80665 m/s²: {out}"
    );
    assert!(
        out.contains("\"a\":\"0.3048\""),
        "foot/second² = 0.3048 m/s²: {out}"
    );
    assert!(out.contains("\"a\":\"0.01\""), "gal = 0.01 m/s²: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `metre_per_second_squared` is the SI unit itself, not a customary-conversion row
    // — the engine abstains rather than fabricating a factor.
    assert!(
        out.contains("\"abstained\":true"),
        "the SI unit itself abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b12) Shipped table — reference/dynamic-viscosity-conversions.adj resolves via
//       import (a NEW dimension: dynamic viscosity → pascal second, the EXACT SP 811
//       B.9 boldface CGS factors poise and centipoise), and a non-exact customary
//       unit (`pound_force_second_per_square_foot`) — a rounded factor with no row —
//       abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_dynamic_viscosity_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_visc");
    let src = stdlib().join("reference/dynamic-viscosity-conversions.adj");
    std::fs::copy(&src, dir.join("dynamic-viscosity-conversions.adj"))
        .expect("copy shipped dynamic-viscosity-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"dynamic-viscosity-conversions.adj\"\n\
         ? dynamic_viscosity_to_pascal_seconds(poise, $v)\n\
         ? dynamic_viscosity_to_pascal_seconds(centipoise, $v)\n\
         ? dynamic_viscosity_to_pascal_seconds(pound_force_second_per_square_foot, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Viscosity, dynamic" exact factors resolve, character-for-
    // character from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"v\":\"0.1\""), "poise = 0.1 Pa·s: {out}");
    assert!(out.contains("\"v\":\"0.001\""), "centipoise = 0.001 Pa·s: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `pound_force_second_per_square_foot` is a NIST customary row printed as a ROUNDED
    // (non-boldface) factor — not an exact row here, so the engine abstains rather than
    // committing to a rounded value.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b13) Shipped table — reference/kinematic-viscosity-conversions.adj resolves via
//       import (a NEW dimension: kinematic viscosity → square metre per second, the
//       EXACT SP 811 B.9 boldface CGS factors stokes and centistokes), and a unit
//       with no row (`square_foot_per_second`) abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_kinematic_viscosity_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_kvisc");
    let src = stdlib().join("reference/kinematic-viscosity-conversions.adj");
    std::fs::copy(&src, dir.join("kinematic-viscosity-conversions.adj"))
        .expect("copy shipped kinematic-viscosity-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"kinematic-viscosity-conversions.adj\"\n\
         ? kinematic_viscosity_to_square_metres_per_second(stokes, $v)\n\
         ? kinematic_viscosity_to_square_metres_per_second(centistokes, $v)\n\
         ? kinematic_viscosity_to_square_metres_per_second(square_foot_per_second, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Viscosity, kinematic" exact factors resolve, character-for-
    // character from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"v\":\"0.0001\""), "stokes = 0.0001 m²/s: {out}");
    assert!(
        out.contains("\"v\":\"0.000001\""),
        "centistokes = 0.000001 m²/s: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `square_foot_per_second` has no row in this CGS-stokes-family table, so the engine
    // abstains rather than inventing a factor the table does not carry.
    assert!(
        out.contains("\"abstained\":true"),
        "a unit with no row abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b14) Shipped table — reference/magnetic-flux-density-conversions.adj resolves via
//       import (a NEW dimension: magnetic flux density → tesla, the EXACT SP 811 B.9
//       boldface factors gauss and gamma), and a unit with no row (`kilogauss`)
//       abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_magnetic_flux_density_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_mfd");
    let src = stdlib().join("reference/magnetic-flux-density-conversions.adj");
    std::fs::copy(&src, dir.join("magnetic-flux-density-conversions.adj"))
        .expect("copy shipped magnetic-flux-density-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"magnetic-flux-density-conversions.adj\"\n\
         ? magnetic_flux_density_to_teslas(gauss, $v)\n\
         ? magnetic_flux_density_to_teslas(gamma, $v)\n\
         ? magnetic_flux_density_to_teslas(kilogauss, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 magnetic-flux-density exact factors resolve, character-for-
    // character from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"v\":\"0.0001\""), "gauss = 0.0001 T: {out}");
    assert!(
        out.contains("\"v\":\"0.000000001\""),
        "gamma = 0.000000001 T: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `kilogauss` has no row in this base-unit table, so the engine abstains rather than
    // inventing a factor the table does not carry.
    assert!(
        out.contains("\"abstained\":true"),
        "a unit with no row abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b15) Shipped table — reference/illuminance-conversions.adj resolves via import (a
//       NEW dimension: illuminance → lux, the EXACT SP 811 B.9 boldface CGS factor
//       phot), and a non-exact customary unit (`footcandle`) — a rounded factor with no
//       row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_illuminance_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_illum");
    let src = stdlib().join("reference/illuminance-conversions.adj");
    std::fs::copy(&src, dir.join("illuminance-conversions.adj"))
        .expect("copy shipped illuminance-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"illuminance-conversions.adj\"\n\
         ? illuminance_to_lux(phot, $v)\n\
         ? illuminance_to_lux(footcandle, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Illuminance" exact factor resolves, character-for-character
    // from the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"v\":\"10000\""), "phot = 10000 lx: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `footcandle` is a NIST customary row printed as a ROUNDED (non-boldface) factor —
    // not an exact row here, so the engine abstains rather than committing to a rounded
    // value.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b16) Shipped table — reference/luminance-conversions.adj resolves via import (a
//       NEW dimension: luminance → candela per square metre, the EXACT SP 811 B.9
//       boldface CGS factor stilb), and a non-exact customary unit (`footlambert`) — a
//       rounded factor with no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_luminance_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_lumin");
    let src = stdlib().join("reference/luminance-conversions.adj");
    std::fs::copy(&src, dir.join("luminance-conversions.adj"))
        .expect("copy shipped luminance-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"luminance-conversions.adj\"\n\
         ? luminance_to_candela_per_square_metre(stilb, $v)\n\
         ? luminance_to_candela_per_square_metre(footlambert, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Luminance" exact factor resolves, character-for-character from
    // the table (boldface = exact; scientific notation to plain decimal).
    assert!(out.contains("\"v\":\"10000\""), "stilb = 10000 cd/m^2: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `footlambert` is a NIST customary row printed as a ROUNDED (non-boldface) factor —
    // not an exact row here, so the engine abstains rather than committing to a rounded
    // value.
    assert!(
        out.contains("\"abstained\":true"),
        "a non-exact unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b17) Shipped table — reference/radioactivity-conversions.adj resolves via import (a
//       NEW dimension: radioactivity/activity → becquerel, the EXACT SP 811 B.9 boldface
//       factor curie = 3.7 E+10), and a traditional activity unit NIST does not tabulate
//       (`rutherford`) — no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_radioactivity_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_radioact");
    let src = stdlib().join("reference/radioactivity-conversions.adj");
    std::fs::copy(&src, dir.join("radioactivity-conversions.adj"))
        .expect("copy shipped radioactivity-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"radioactivity-conversions.adj\"\n\
         ? activity_to_becquerel(curie, $v)\n\
         ? activity_to_becquerel(rutherford, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Radiology" exact factor resolves, character-for-character from
    // the table (boldface = exact; scientific notation to plain decimal).
    assert!(
        out.contains("\"v\":\"37000000000\""),
        "curie = 37000000000 Bq: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `rutherford` is a real activity unit that NIST SP 811 B.9 does NOT tabulate — no row
    // here, so the engine abstains rather than committing to a fabricated factor.
    assert!(
        out.contains("\"abstained\":true"),
        "an untabulated unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b18) Shipped table — reference/absorbed-dose-conversions.adj resolves via import (a
//       NEW dimension: absorbed dose → gray, the EXACT SP 811 B.9 boldface factor
//       rad = 1.0 E-02 = 0.01, the rad DEFINED as exactly 10⁻² gray), and a traditional
//       absorbed-dose unit NIST does not tabulate (`rep`) — no row — abstains.
// ---------------------------------------------------------------------------

#[test]
fn shipped_absorbed_dose_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_absdose");
    let src = stdlib().join("reference/absorbed-dose-conversions.adj");
    std::fs::copy(&src, dir.join("absorbed-dose-conversions.adj"))
        .expect("copy shipped absorbed-dose-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"absorbed-dose-conversions.adj\"\n\
         ? absorbed_dose_to_gray(rad, $v)\n\
         ? absorbed_dose_to_gray(rep, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Radiology" exact factor resolves, character-for-character from
    // the table (boldface = exact; the rad is defined as exactly 10⁻² gray = 0.01).
    assert!(out.contains("\"v\":\"0.01\""), "rad = 0.01 Gy: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `rep` (roentgen equivalent physical) is a historical absorbed-dose unit that NIST SP
    // 811 B.9 does NOT tabulate — no row here, so the engine abstains rather than committing
    // to a fabricated factor.
    assert!(
        out.contains("\"abstained\":true"),
        "an untabulated unit abstains, never a fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b19) Shipped table — reference/dose-equivalent-conversions.adj resolves via import (a
//       NEW dimension: dose equivalent → sievert, the EXACT SP 811 B.9 boldface factor
//       rem = 1.0 E-02 = 0.01, the rem DEFINED as exactly 10⁻² sievert), and a unit of a
//       DIFFERENT quantity (`rad`, an absorbed-dose unit → gray, not sievert) — no row —
//       abstains rather than mis-converting across dimensions.
// ---------------------------------------------------------------------------

#[test]
fn shipped_dose_equivalent_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_doseeq");
    let src = stdlib().join("reference/dose-equivalent-conversions.adj");
    std::fs::copy(&src, dir.join("dose-equivalent-conversions.adj"))
        .expect("copy shipped dose-equivalent-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"dose-equivalent-conversions.adj\"\n\
         ? dose_equivalent_to_sievert(rem, $v)\n\
         ? dose_equivalent_to_sievert(rad, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Radiology" exact factor resolves, character-for-character from
    // the table (boldface = exact; the rem is defined as exactly 10⁻² sievert = 0.01).
    assert!(out.contains("\"v\":\"0.01\""), "rem = 0.01 Sv: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `rad` is an ABSORBED-DOSE unit (it converts to the gray, a DIFFERENT quantity), so this
    // dose-equivalent table has no row for it — the engine abstains rather than mis-converting
    // an absorbed-dose unit as if it were a dose-equivalent unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b20) Shipped table — reference/exposure-conversions.adj resolves via import (a
//       NEW dimension: exposure → coulomb per kilogram, the EXACT SP 811 B.9 boldface factor
//       roentgen = 2.58 E-04 = 0.000258, the roentgen DEFINED as exactly 2.58×10⁻⁴ C/kg), and
//       a unit of a DIFFERENT quantity (`rad`, an absorbed-dose unit → gray, not C/kg) — no
//       row — abstains rather than mis-converting across dimensions. Completes the radiological
//       quartet: radioactivity (curie→becquerel), absorbed dose (rad→gray), dose equivalent
//       (rem→sievert), exposure (roentgen→coulomb per kilogram).
// ---------------------------------------------------------------------------

#[test]
fn shipped_exposure_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_exposure");
    let src = stdlib().join("reference/exposure-conversions.adj");
    std::fs::copy(&src, dir.join("exposure-conversions.adj"))
        .expect("copy shipped exposure-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"exposure-conversions.adj\"\n\
         ? exposure_to_coulomb_per_kilogram(roentgen, $v)\n\
         ? exposure_to_coulomb_per_kilogram(rad, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 "Radiology" exact factor resolves, character-for-character from
    // the table (boldface = exact; the roentgen is defined as exactly 2.58×10⁻⁴ C/kg).
    assert!(
        out.contains("\"v\":\"0.000258\""),
        "roentgen = 0.000258 C/kg: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `rad` is an ABSORBED-DOSE unit (it converts to the gray, a DIFFERENT quantity), so this
    // exposure table has no row for it — the engine abstains rather than mis-converting an
    // absorbed-dose unit as if it were an exposure unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b21) Shipped table — reference/magnetic-flux-conversions.adj resolves via import (a
//       NEW dimension: magnetic flux → weber, the EXACT SP 811 B.9 boldface factor
//       maxwell = 1.0 E-08 = 0.00000001, the maxwell DEFINED as exactly 10⁻⁸ Wb), and a unit
//       of a DIFFERENT quantity (`gauss`, a magnetic-flux-DENSITY unit → tesla, not the weber)
//       — no row — abstains rather than mis-converting across dimensions. Distinct from
//       magnetic-flux-density (gauss→tesla): flux is the total field through a surface, flux
//       density is the field per unit area; one weber over one square metre is one tesla.
// ---------------------------------------------------------------------------

#[test]
fn shipped_magnetic_flux_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_magnetic_flux");
    let src = stdlib().join("reference/magnetic-flux-conversions.adj");
    std::fs::copy(&src, dir.join("magnetic-flux-conversions.adj"))
        .expect("copy shipped magnetic-flux-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"magnetic-flux-conversions.adj\"\n\
         ? magnetic_flux_to_weber(maxwell, $v)\n\
         ? magnetic_flux_to_weber(gauss, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the maxwell is defined as exactly 10^-8 Wb).
    assert!(
        out.contains("\"v\":\"0.00000001\""),
        "maxwell = 0.00000001 Wb: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `gauss` is a magnetic-flux-DENSITY unit (it converts to the tesla, a DIFFERENT quantity),
    // so this flux table has no row for it — the engine abstains rather than mis-converting a
    // flux-density unit as if it were a flux unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b22) Shipped table — reference/power-conversions.adj resolves via import (a NEW
//       dimension: power → watt, the EXACT SP 811 B.9 boldface factor
//       erg per second = 1.0 E-07 = 0.0000001, the erg DEFINED as exactly 10⁻⁷ J so an erg
//       per second is exactly 10⁻⁷ W), and a unit of a DIFFERENT quantity (`erg`, an ENERGY
//       unit → joule, not the watt) — no row — abstains rather than mis-converting across
//       dimensions. The sharp case: the erg (energy) and the erg per second (power) share the
//       numeric factor 1.0 E-07, but they name DIFFERENT SI units (joule vs watt), so the
//       abstain proves dimension safety, not mere absence of a value: power is energy per unit
//       time; one watt is one joule per second.
// ---------------------------------------------------------------------------

#[test]
fn shipped_power_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_power");
    let src = stdlib().join("reference/power-conversions.adj");
    std::fs::copy(&src, dir.join("power-conversions.adj")).expect("copy shipped power-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"power-conversions.adj\"\n\
         ? power_to_watt(erg_per_second, $v)\n\
         ? power_to_watt(erg, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the erg is defined as exactly 10^-7 J, so erg/s is exactly 10^-7 W).
    assert!(
        out.contains("\"v\":\"0.0000001\""),
        "erg per second = 0.0000001 W: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `erg` is an ENERGY unit (it converts to the joule, a DIFFERENT quantity), so this power
    // table has no row for it — the engine abstains rather than mis-converting an energy unit
    // as if it were a power unit (and rather than silently reusing the coincidentally-equal
    // 1.0 E-07 factor for the wrong SI unit).
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b23) Shipped table — reference/electric-charge-conversions.adj resolves via import (a NEW
//       dimension: electric charge → coulomb, the EXACT SP 811 B.9 boldface factor
//       abcoulomb = 1.0 E+01 = 10, the abcoulomb DEFINED as exactly 10 C), and a unit of a
//       DIFFERENT quantity (`abampere`, an electric-CURRENT unit → ampere, not the coulomb)
//       — no row — abstains rather than mis-converting across dimensions. The sharp case: the
//       abcoulomb (charge) and the abampere (current) share the numeric factor 1.0 E+01, but
//       they name DIFFERENT SI units (coulomb vs ampere), so the abstain proves dimension
//       safety, not mere absence of a value: current is charge per unit time; one coulomb is
//       one ampere-second.
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_charge_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_charge");
    let src = stdlib().join("reference/electric-charge-conversions.adj");
    std::fs::copy(&src, dir.join("electric-charge-conversions.adj"))
        .expect("copy shipped electric-charge-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-charge-conversions.adj\"\n\
         ? electric_charge_to_coulomb(abcoulomb, $v)\n\
         ? electric_charge_to_coulomb(abampere, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abcoulomb is defined as exactly 10 C).
    assert!(out.contains("\"v\":\"10\""), "abcoulomb = 10 C: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abampere` is an electric-CURRENT unit (it converts to the ampere, a DIFFERENT quantity),
    // so this charge table has no row for it — the engine abstains rather than mis-converting a
    // current unit as if it were a charge unit (and rather than silently reusing the
    // coincidentally-equal 1.0 E+01 factor for the wrong SI unit).
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b24) The shipped NIST SP 811 B.9 electric-potential → volt table resolves the exact abvolt
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. The abvolt (potential)
//       and the abohm (resistance) are DIFFERENT quantities that convert to DIFFERENT SI units
//       (volt vs ohm), so the abstain proves dimension safety, not mere absence of a value: the
//       volt is energy per unit charge; the ohm is potential per unit current.
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_potential_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_potential");
    let src = stdlib().join("reference/electric-potential-conversions.adj");
    std::fs::copy(&src, dir.join("electric-potential-conversions.adj"))
        .expect("copy shipped electric-potential-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-potential-conversions.adj\"\n\
         ? electric_potential_to_volt(abvolt, $v)\n\
         ? electric_potential_to_volt(abohm, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abvolt is defined as exactly 1.0 E-08 V).
    assert!(out.contains("\"v\":\"0.00000001\""), "abvolt = 1.0 E-08 V: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abohm` is an electric-RESISTANCE unit (it converts to the ohm, a DIFFERENT quantity), so
    // this potential table has no row for it — the engine abstains rather than mis-converting a
    // resistance unit as if it were a potential unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b25) The shipped NIST SP 811 B.9 electric-resistance → ohm table resolves the exact abohm
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. This completes the
//       electromagnetic-EMU trio (charge/potential/resistance). The abohm (resistance) and the
//       abvolt (potential) are DIFFERENT quantities that convert to DIFFERENT SI units (ohm vs
//       volt), so the abstain proves dimension safety, not mere absence of a value: the ohm is
//       potential per unit current.
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_resistance_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_resistance");
    let src = stdlib().join("reference/electric-resistance-conversions.adj");
    std::fs::copy(&src, dir.join("electric-resistance-conversions.adj"))
        .expect("copy shipped electric-resistance-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-resistance-conversions.adj\"\n\
         ? electric_resistance_to_ohm(abohm, $v)\n\
         ? electric_resistance_to_ohm(abvolt, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abohm is defined as exactly 1.0 E-09 ohm).
    assert!(out.contains("\"v\":\"0.000000001\""), "abohm = 1.0 E-09 ohm: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abvolt` is an electric-POTENTIAL unit (it converts to the volt, a DIFFERENT quantity), so
    // this resistance table has no row for it — the engine abstains rather than mis-converting a
    // potential unit as if it were a resistance unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b26) The shipped NIST SP 811 B.9 electric-capacitance → farad table resolves the exact abfarad
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. This EXTENDS the
//       electromagnetic-EMU family (charge/potential/resistance) with capacitance. The abfarad
//       (capacitance) and the abhenry (inductance) are DIFFERENT quantities that convert to
//       DIFFERENT SI units (farad vs henry), so the abstain proves dimension safety, not mere
//       absence of a value: the farad is charge per unit potential.
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_capacitance_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_capacitance");
    let src = stdlib().join("reference/electric-capacitance-conversions.adj");
    std::fs::copy(&src, dir.join("electric-capacitance-conversions.adj"))
        .expect("copy shipped electric-capacitance-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-capacitance-conversions.adj\"\n\
         ? electric_capacitance_to_farad(abfarad, $v)\n\
         ? electric_capacitance_to_farad(abhenry, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abfarad is defined as exactly 1.0 E+09 farad).
    assert!(out.contains("\"v\":\"1000000000\""), "abfarad = 1.0 E+09 F: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abhenry` is an INDUCTANCE unit (it converts to the henry, a DIFFERENT quantity), so this
    // capacitance table has no row for it — the engine abstains rather than mis-converting an
    // inductance unit as if it were a capacitance unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b27) The shipped NIST SP 811 B.9 electric-inductance → henry table resolves the exact abhenry
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. This CLOSES the
//       electromagnetic-EMU family (charge/potential/resistance/capacitance) with inductance. The
//       abhenry (inductance) and the abfarad (capacitance) are DIFFERENT quantities that convert
//       to DIFFERENT SI units (henry vs farad), so the abstain proves dimension safety, not mere
//       absence of a value: the henry is flux linkage per unit current.
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_inductance_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_inductance");
    let src = stdlib().join("reference/electric-inductance-conversions.adj");
    std::fs::copy(&src, dir.join("electric-inductance-conversions.adj"))
        .expect("copy shipped electric-inductance-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-inductance-conversions.adj\"\n\
         ? electric_inductance_to_henry(abhenry, $v)\n\
         ? electric_inductance_to_henry(abfarad, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abhenry is defined as exactly 1.0 E-09 henry).
    assert!(out.contains("\"v\":\"0.000000001\""), "abhenry = 1.0 E-09 H: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abfarad` is a CAPACITANCE unit (it converts to the farad, a DIFFERENT quantity), so this
    // inductance table has no row for it — the engine abstains rather than mis-converting a
    // capacitance unit as if it were an inductance unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b28) The shipped NIST SP 811 B.9 electric-conductance → siemens table resolves the exact abmho
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. This EXTENDS the
//       electromagnetic-EMU family (charge/potential/resistance/capacitance/inductance) with
//       conductance. The abmho (conductance) and the abohm (resistance) are RECIPROCAL quantities
//       that convert to DIFFERENT, reciprocal SI units (siemens vs ohm) with reciprocal factors
//       (1.0 E+09 vs 1.0 E-09), so the abstain proves dimension safety even against the reciprocal
//       quantity, not mere absence of a value: the siemens is one reciprocal ohm (an ampere per volt).
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_conductance_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_conductance");
    let src = stdlib().join("reference/electric-conductance-conversions.adj");
    std::fs::copy(&src, dir.join("electric-conductance-conversions.adj"))
        .expect("copy shipped electric-conductance-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-conductance-conversions.adj\"\n\
         ? electric_conductance_to_siemens(abmho, $v)\n\
         ? electric_conductance_to_siemens(abohm, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abmho is defined as exactly 1.0 E+09 siemens).
    assert!(out.contains("\"v\":\"1000000000\""), "abmho = 1.0 E+09 S: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abohm` is a RESISTANCE unit (it converts to the ohm, the reciprocal quantity), so this
    // conductance table has no row for it — the engine abstains rather than mis-converting a
    // resistance unit as if it were a conductance unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b29) The shipped NIST SP 811 B.9 electric-current → ampere table resolves the exact abampere
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. This EXTENDS the
//       electromagnetic-EMU family (charge/potential/resistance/capacitance/inductance/conductance)
//       with current — the abampere is the BASE EMU electrical unit. The abampere (current) and the
//       abcoulomb (charge) are DIFFERENT quantities that convert to DIFFERENT SI units (ampere vs
//       coulomb) yet NIST prints the SAME numeric factor 1.0 E+01 for both, so the abstain is a
//       genuine dimension check rather than a value coincidence, not mere absence of a value.
// ---------------------------------------------------------------------------

#[test]
fn shipped_electric_current_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_current");
    let src = stdlib().join("reference/electric-current-conversions.adj");
    std::fs::copy(&src, dir.join("electric-current-conversions.adj"))
        .expect("copy shipped electric-current-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"electric-current-conversions.adj\"\n\
         ? electric_current_to_ampere(abampere, $v)\n\
         ? electric_current_to_ampere(abcoulomb, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the abampere is defined as exactly 1.0 E+01 ampere, the base EMU unit).
    assert!(out.contains("\"v\":\"10\""), "abampere = 1.0 E+01 A: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `abcoulomb` is a CHARGE unit (it converts to the coulomb, a DIFFERENT quantity), so this
    // current table has no row for it — the engine abstains rather than mis-converting a charge
    // unit as if it were a current unit, even though NIST prints the same 1.0 E+01 factor for it.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b30) The shipped NIST SP 811 B.9 wave-number → reciprocal-metre table resolves the exact kayser
//       factor with its citation, and ABSTAINS on a wrong-dimension unit. This opens a NEW dimension
//       — WAVE NUMBER (spatial frequency, wavelengths per unit length) — beyond the length/mass/…/
//       electric family. The kayser (wave number) and the angstrom (length/wavelength) are RECIPROCAL
//       quantities that convert to DIFFERENT SI units (m⁻¹ vs m), so the abstain guards the
//       wavelength-versus-wavenumber confusion directly, not mere absence of a value: the kayser is
//       one reciprocal centimetre = exactly 100 reciprocal metres.
// ---------------------------------------------------------------------------

#[test]
fn shipped_wave_number_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_wavenumber");
    let src = stdlib().join("reference/wave-number-conversions.adj");
    std::fs::copy(&src, dir.join("wave-number-conversions.adj"))
        .expect("copy shipped wave-number-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"wave-number-conversions.adj\"\n\
         ? wave_number_to_reciprocal_metre(kayser, $v)\n\
         ? wave_number_to_reciprocal_metre(angstrom, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the kayser is one reciprocal centimetre = exactly 1 E+02 reciprocal metres).
    assert!(out.contains("\"v\":\"100\""), "kayser = 1 E+02 m^-1: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `angstrom` is a LENGTH (wavelength) unit (it converts to the metre, a DIFFERENT quantity), so
    // this wave-number table has no row for it — the engine abstains rather than mis-converting a
    // length unit as if it were a wave-number unit (its reciprocal).
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b31) The shipped NIST SP 811 B.9 linear-mass-density → kilogram-per-metre table resolves the exact
//       tex factor with its citation, and ABSTAINS on a wrong-dimension unit. This opens a NEW
//       dimension — LINEAR MASS DENSITY (mass per unit length, the titre of a fibre/yarn) — beyond
//       the length/mass/…/wave-number and electric families. The tex (linear mass density → kg/m) and
//       the pound (plain mass → kg) are DIFFERENT quantities that convert to DIFFERENT SI units
//       (kg/m vs kg), so the abstain guards the mass-versus-mass-per-length confusion (dropping the
//       "per metre") directly, not mere absence of a value: the tex is one gram per kilometre =
//       exactly 1 E-06 kilogram per metre.
// ---------------------------------------------------------------------------

#[test]
fn shipped_linear_mass_density_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_lineardensity");
    let src = stdlib().join("reference/linear-mass-density-conversions.adj");
    std::fs::copy(&src, dir.join("linear-mass-density-conversions.adj"))
        .expect("copy shipped linear-mass-density-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"linear-mass-density-conversions.adj\"\n\
         ? linear_mass_density_to_kilogram_per_metre(tex, $v)\n\
         ? linear_mass_density_to_kilogram_per_metre(pound, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; the tex is one gram per kilometre = exactly 1 E-06 kilogram per metre).
    assert!(out.contains("\"v\":\"0.000001\""), "tex = 1 E-06 kg/m: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `pound` is a MASS unit (it converts to the kilogram, a DIFFERENT quantity), so this
    // linear-mass-density table has no row for it — the engine abstains rather than mis-converting a
    // mass unit as if it were a mass-per-length unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b32) The shipped NIST SP 811 B.9 temperature-interval → kelvin table resolves the exact degree-
//       Celsius factor with its citation, and ABSTAINS on a wrong-dimension unit. This opens a NEW
//       dimension — TEMPERATURE INTERVAL (a DIFFERENCE of temperatures, not a point reading) — beyond
//       the length/mass/…/linear-mass-density and electric families. A Celsius-degree interval is
//       EXACTLY one kelvin (the scales share a unit step; the 273.15 offset applies only to POINTS,
//       which are an ADDITION, not a factor, hence absent). The Celsius degree (temperature interval →
//       kelvin) and the bare "degree" (plane angle → radian) are DIFFERENT quantities that convert to
//       DIFFERENT SI units (K vs rad) yet share the "°" symbol, so the abstain guards that notational
//       confusion directly, not mere absence of a value.
// ---------------------------------------------------------------------------

#[test]
fn shipped_temperature_interval_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_tempinterval");
    let src = stdlib().join("reference/temperature-interval-conversions.adj");
    std::fs::copy(&src, dir.join("temperature-interval-conversions.adj"))
        .expect("copy shipped temperature-interval-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"temperature-interval-conversions.adj\"\n\
         ? temperature_interval_to_kelvin(degree_celsius, $v)\n\
         ? temperature_interval_to_kelvin(degree, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; a Celsius-degree interval is exactly 1 kelvin).
    assert!(out.contains("\"v\":\"1\""), "degree Celsius interval = 1.0 E+00 K: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // The bare `degree` is a PLANE-ANGLE unit (it converts to the radian, a DIFFERENT quantity), so
    // this temperature-interval table has no row for it — the engine abstains rather than mis-
    // converting an angle as if it were a temperature, despite the shared "°" symbol.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b33) The shipped NIST SP 811 B.9 mass-density → kilogram-per-cubic-metre table resolves the exact
//       gram-per-cubic-centimetre factor with its citation, and ABSTAINS on a wrong-dimension unit.
//       This opens a NEW dimension — MASS DENSITY (mass per unit VOLUME) — beyond the length/mass/…/
//       linear-mass-density/temperature-interval and electric families. One gram per cubic centimetre
//       is exactly 1000 kg/m³ (the metric units relate by exact powers of ten). The gram per cubic
//       centimetre (mass density → kg/m³) and the pound (plain mass → kg) are DIFFERENT quantities
//       converting to DIFFERENT SI units (kg/m³ vs kg), so the abstain guards the mass-versus-mass-
//       per-volume confusion (dropping the "per cubic metre") directly, not mere absence of a value.
// ---------------------------------------------------------------------------

#[test]
fn shipped_mass_density_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_massdensity");
    let src = stdlib().join("reference/mass-density-conversions.adj");
    std::fs::copy(&src, dir.join("mass-density-conversions.adj"))
        .expect("copy shipped mass-density-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"mass-density-conversions.adj\"\n\
         ? mass_density_to_kilogram_per_cubic_metre(gram_per_cubic_centimetre, $v)\n\
         ? mass_density_to_kilogram_per_cubic_metre(pound, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factor resolves, character-for-character from the table
    // (boldface = exact; one gram per cubic centimetre is exactly 1 E+03 kilograms per cubic metre).
    assert!(out.contains("\"v\":\"1000\""), "gram per cubic centimetre = 1 E+03 kg/m^3: {out}");
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `pound` is a MASS unit (it converts to the kilogram, a DIFFERENT quantity), so this mass-density
    // table has no row for it — the engine abstains rather than mis-converting a mass unit as if it
    // were a mass-per-volume unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// Shipped moment-of-force (torque) table — two EXACT NIST SP 811 B.9 factors resolve, and a
// wrong-dimension unit (a mass unit) abstains. Guards the SLICE of a NEW dimension (moment of force,
// the newton metre) and the honest cross-dimension abstention: `pound` is a mass unit, so a torque
// lookup for it must abstain, catching the torque/mass confusion directly, not mere absence of a
// value. (Torque shares ENERGY's dimensions but is a distinct quantity keeping its own N·m unit.)
// ---------------------------------------------------------------------------

#[test]
fn shipped_moment_of_force_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_momentofforce");
    let src = stdlib().join("reference/moment-of-force-conversions.adj");
    std::fs::copy(&src, dir.join("moment-of-force-conversions.adj"))
        .expect("copy shipped moment-of-force-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"moment-of-force-conversions.adj\"\n\
         ? moment_of_force_to_newton_metre(dyne_centimetre, $v)\n\
         ? moment_of_force_to_newton_metre(kilogram_force_metre, $v)\n\
         ? moment_of_force_to_newton_metre(pound, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factors resolve, character-for-character from the table (boldface =
    // exact; one dyne centimetre is exactly 1 E-07 N·m, one kilogram-force metre is exactly 9.806 65 N·m).
    assert!(
        out.contains("\"v\":\"0.0000001\""),
        "dyne centimetre = 1 E-07 N*m: {out}"
    );
    assert!(
        out.contains("\"v\":\"9.80665\""),
        "kilogram-force metre = 9.806 65 E+00 N*m: {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `pound` is a MASS unit (it converts to the kilogram, a DIFFERENT quantity), so this
    // moment-of-force table has no row for it — the engine abstains rather than mis-converting a mass
    // unit as if it were a torque unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
    );
}

// ---------------------------------------------------------------------------
// Shipped specific-heat-capacity table — two EXACT NIST SP 811 B.9 factors resolve, and a
// wrong-dimension unit (a mass unit) abstains. Guards the SLICE of a NEW dimension (specific heat
// capacity / specific entropy, the joule per kilogram kelvin) and the honest cross-dimension
// abstention: `pound` is a mass unit, so a specific-heat lookup for it must abstain, catching the
// specific-heat/mass confusion directly, not mere absence of a value. (Specific heat capacity is an
// intensive per-unit-mass quantity — energy/(mass·temperature) — distinct from plain heat capacity,
// J/K, and from specific energy, J/kg.)
// ---------------------------------------------------------------------------

#[test]
fn shipped_specific_heat_capacity_conversions_table_resolves_and_abstains() {
    let dir = scratch("shipped_specificheatcapacity");
    let src = stdlib().join("reference/specific-heat-capacity-conversions.adj");
    std::fs::copy(&src, dir.join("specific-heat-capacity-conversions.adj"))
        .expect("copy shipped specific-heat-capacity-conversions.adj");
    write(
        dir.as_path(),
        "case.adj",
        "import \"specific-heat-capacity-conversions.adj\"\n\
         ? specific_heat_capacity_to_joule_per_kilogram_kelvin(calorie_it_per_gram_celsius, $v)\n\
         ? specific_heat_capacity_to_joule_per_kilogram_kelvin(calorie_th_per_gram_celsius, $v)\n\
         ? specific_heat_capacity_to_joule_per_kilogram_kelvin(pound, $v)\n",
    );
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}{err}");
    // The NIST SP 811 B.9 exact factors resolve, character-for-character from the table (boldface =
    // exact; the International-Table calorie is exactly 4.1868 J and the thermochemical calorie is
    // exactly 4.184 J, so calIT/(g*degC) = 4186.8 and calth/(g*degC) = 4184 J/(kg*K), exactly).
    assert!(
        out.contains("\"v\":\"4186.8\""),
        "calorieIT per gram degree Celsius = 4.1868 E+03 J/(kg*K): {out}"
    );
    assert!(
        out.contains("\"v\":\"4184\""),
        "calorieth per gram degree Celsius = 4.184 E+03 J/(kg*K): {out}"
    );
    // The table's citation rides along on the answer.
    assert!(
        out.contains("nist-guide-si-appendix-b9"),
        "carries the NIST SP 811 B.9 locator: {out}"
    );
    // `pound` is a MASS unit (it converts to the kilogram, a DIFFERENT quantity), so this
    // specific-heat-capacity table has no row for it — the engine abstains rather than mis-converting a
    // mass unit as if it were a specific-heat unit.
    assert!(
        out.contains("\"abstained\":true"),
        "a wrong-dimension unit abstains, never a cross-dimension fabricated factor: {out}"
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
