//! End-to-end tests for ADJ-TABLES RS-5e — PER-ROW provenance on a `table` —
//! driven through the built CLI binary.
//!
//! ## What is being fixed
//!
//! A table carries one `source`/`locator`/`trust` envelope. With six bands and one
//! envelope, EVERY answer — in every band — quoted the SAME sentence. That is an
//! accounting error: the audit trail asserts a fact and cites a span that does not
//! defend it. A range lookup makes it glaring (the selected row is explicit in the
//! audit), but exact lookup was equally mis-cited.
//!
//! RS-5e lets a row carry its own `{ … }` block, folded OVER the envelope field by
//! field. Five things are proven:
//!
//!   (a) RANGE lookup cites the span of the row it SELECTED — not the envelope's.
//!   (b) EXACT lookup likewise cites its own row's span.
//!   (c) INHERITANCE: a row that writes only `source` keeps the table's `locator`
//!       and `trust` (so the common case stays terse — one span per row, one
//!       locator for the page).
//!   (d) OVERRIDE: a row may also override `trust`, and only that row changes.
//!   (e) BACKWARD COMPATIBILITY: a table whose rows write no block behaves exactly
//!       as before — every row inherits the whole envelope.

use std::path::Path;
use std::process::Command;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs5e_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String, String) {
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

/// A breakpoint table whose rows each carry their OWN defending span, while
/// sharing the table's `locator` and (mostly) its `trust`.
const PER_ROW_TABLE: &str = r#"
table aqi {
    columns min_aqi, category
    row (0, good)                             { source "Green   Good   0 to 50" }
    row (51, moderate)                        { source "Yellow   Moderate   51 to 100" }
    row (101, unhealthy_for_sensitive_groups) { source "Orange   Unhealthy for Sensitive Groups   101 to 150" }
    row (301, hazardous)                      { source "Maroon   Hazardous   301 and higher"  trust consensus }
    source  "The AQI includes six color-coded categories, each corresponding to a range of index values."
    locator "https://www.airnow.gov/aqi/aqi-basics/"
    trust   authoritative
}
"#;

// ---------------------------------------------------------------------------
// (a) Range lookup cites the SELECTED row — the heart of the fix.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_cites_the_selected_rows_own_span_not_the_table_envelope() {
    let dir = scratch("range");
    let src = format!("{PER_ROW_TABLE}\n? lookup aqi min_aqi = 120 mode range give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(
        out.contains("\"category\":\"unhealthy_for_sensitive_groups\""),
        "wrong band: {out}"
    );
    // The answer must quote the span that defends THIS band …
    assert!(
        out.contains("Orange   Unhealthy for Sensitive Groups   101 to 150"),
        "answer must cite the selected row's span: {out}"
    );
    // … and must NOT fall back to the table's framing sentence.
    assert!(
        !out.contains("six color-coded categories"),
        "the envelope's span must not be cited for a row that has its own: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b) Exact lookup cites its own row too — this was never range-specific.
// ---------------------------------------------------------------------------

#[test]
fn exact_lookup_cites_the_matching_rows_own_span() {
    let dir = scratch("exact");
    let src = format!("{PER_ROW_TABLE}\n? aqi(51, $Category)\n");
    write(&dir, "case.adj", &src);
    let (ok, out, _e) = run(&dir.join("case.adj"));
    assert!(ok);
    assert!(out.contains("\"Category\":\"moderate\""), "bad bind: {out}");
    assert!(
        out.contains("Yellow   Moderate   51 to 100"),
        "exact lookup must cite its own row's span: {out}"
    );
    assert!(
        !out.contains("six color-coded categories"),
        "envelope span must not be cited here: {out}"
    );
}

// ---------------------------------------------------------------------------
// (c) A row that writes only `source` inherits locator + trust.
// ---------------------------------------------------------------------------

#[test]
fn a_row_that_writes_only_source_inherits_the_tables_locator_and_trust() {
    let dir = scratch("inherit");
    let src = format!("{PER_ROW_TABLE}\n? lookup aqi min_aqi = 75 mode range give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, _e) = run(&dir.join("case.adj"));
    assert!(ok);
    assert!(out.contains("Yellow   Moderate   51 to 100"), "own span: {out}");
    // Inherited from the envelope — the row wrote neither.
    assert!(
        out.contains("airnow.gov/aqi/aqi-basics"),
        "locator must be inherited: {out}"
    );
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "trust must be inherited: {out}"
    );
}

// ---------------------------------------------------------------------------
// (d) A row may override `trust`, and ONLY that row changes.
// ---------------------------------------------------------------------------

#[test]
fn a_row_can_override_trust_without_affecting_its_siblings() {
    let dir = scratch("override");
    // The 301 row downgrades itself to `consensus`; the 51 row does not.
    let src = format!(
        "{PER_ROW_TABLE}\n\
         ? lookup aqi min_aqi = 400 mode range give category\n\
         ? lookup aqi min_aqi = 75 mode range give category\n"
    );
    write(&dir, "case.adj", &src);
    let (ok, out, _e) = run(&dir.join("case.adj"));
    assert!(ok);
    assert!(out.contains("\"trust\":\"consensus\""), "row override missing: {out}");
    assert!(
        out.contains("\"trust\":\"authoritative\""),
        "sibling row must keep the inherited tier: {out}"
    );
}

// ---------------------------------------------------------------------------
// (e) Backward compatibility — no row blocks means the old behaviour exactly.
// ---------------------------------------------------------------------------

#[test]
fn a_table_with_no_row_blocks_still_inherits_the_envelope_everywhere() {
    let dir = scratch("compat");
    let src = r#"
table bands {
    columns min_v, label
    row (0, low)
    row (10, high)
    source  "One sentence defending the whole table."
    locator "https://example.test/bands"
    trust   consensus
}

? lookup bands min_v = 12 mode range give label
"#;
    write(&dir, "case.adj", src);
    let (ok, out, _e) = run(&dir.join("case.adj"));
    assert!(ok);
    assert!(out.contains("\"label\":\"high\""), "bad band: {out}");
    assert!(
        out.contains("One sentence defending the whole table."),
        "a row with no block must inherit the envelope: {out}"
    );
    assert!(out.contains("\"trust\":\"consensus\""), "tier inherited: {out}");
}
