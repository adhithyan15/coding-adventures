//! End-to-end tests for ADJ-TABLES RS-5c — the RANGE / BRACKET lookup tactic —
//! driven through the built CLI binary. A `table` is read as a step function:
//! `? lookup <table> <key_col> = <n> mode range give <value_col>` selects the
//! breakpoint row whose key is the greatest key `<= n` and returns its value
//! column WITH that row's citation. Four things are proven:
//!
//!   (a) BRACKET HIT: a value strictly inside a band binds the band's value AND
//!       the selected breakpoint row's citation, and the audit names the matched
//!       key (which bracket it fell in).
//!   (b) EXACT BOUNDARY: a value equal to a breakpoint selects THAT breakpoint
//!       (greatest key `<=` value is the value itself) — the exact comparison,
//!       no `f64` fuzz.
//!   (c) TOP-OPEN BAND + BELOW-DOMAIN ABSTENTION: a value above the last
//!       breakpoint falls in the top band; a value below the smallest key has no
//!       key `<=` it and honestly ABSTAINS ("below the table's domain"), never a
//!       fabricated classification.
//!   (d) GUARDS: `mode interpolated give <non-numeric col>` is rejected (RS-5d
//!       interpolation needs a numeric value column — you cannot interpolate a
//!       category label), an unknown value column is a clean compile error, and a
//!       non-numeric key column is rejected (a range key must be a number).

use std::path::Path;
use std::process::Command;

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs5c_{tag}_{}", std::process::id()));
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

/// A self-contained BMI-category breakpoint table used across the hit/abstain
/// cases. Four bands keyed by the band's minimum BMI; the key column is numeric.
const BMI_TABLE: &str = r#"
table bmi_categories {
    columns min_bmi, category
    row (0, underweight)
    row (18.5, normal)
    row (25, overweight)
    row (30, obese)
    source "A breakpoint table of BMI weight-status bands keyed by band minimum."
    locator "https://example.test/bmi-bands"
    trust consensus
}
"#;

// ---------------------------------------------------------------------------
// (a) Bracket hit — inside a band, carries the selected row's citation + key.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_bracket_hit_binds_band_value_with_citation_and_matched_key() {
    let dir = scratch("hit");
    let src =
        format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 27.3 mode range give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(ok, "CLI should succeed; stderr={err}");
    assert!(
        out.contains("\"lookups\""),
        "expected a lookups section: {out}"
    );
    // 27.3 falls in [25, 30) → overweight, selected breakpoint min_bmi = 25.
    assert!(
        out.contains("\"category\":\"overweight\""),
        "wrong band: {out}"
    );
    assert!(
        out.contains("\"min_bmi\":\"25\""),
        "audit should name the matched breakpoint: {out}"
    );
    // The selected row's citation travels with the answer.
    assert!(
        out.contains("example.test/bmi-bands"),
        "answer must carry the row citation: {out}"
    );
    assert!(
        out.contains("\"trust\":\"consensus\""),
        "citation must carry the trust tier: {out}"
    );
    assert!(
        out.contains("\"abstained\":false"),
        "a hit is not an abstention: {out}"
    );
}

// ---------------------------------------------------------------------------
// (b) Exact boundary — a value equal to a breakpoint selects that breakpoint.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_on_an_exact_breakpoint_selects_that_breakpoint() {
    let dir = scratch("boundary");
    let src =
        format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 18.5 mode range give category\n");
    write(&dir, "case.adj", &src);
    let (ok, out, _err) = run_full(&dir.join("case.adj"));
    assert!(ok);
    // greatest key <= 18.5 is 18.5 itself → normal (exact comparison, no f64 fuzz).
    assert!(
        out.contains("\"category\":\"normal\""),
        "exact boundary must land on the breakpoint: {out}"
    );
    assert!(
        out.contains("\"min_bmi\":\"18.5\""),
        "matched key should be the breakpoint itself: {out}"
    );
}

// ---------------------------------------------------------------------------
// (c) Top-open band + below-domain abstention.
// ---------------------------------------------------------------------------

#[test]
fn range_lookup_top_band_is_open_and_below_domain_abstains() {
    let dir = scratch("edges");
    // A value above the last breakpoint falls in the top (open) band.
    let top =
        format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 42 mode range give category\n");
    write(&dir, "top.adj", &top);
    let (ok, out, _e) = run_full(&dir.join("top.adj"));
    assert!(ok);
    assert!(
        out.contains("\"category\":\"obese\""),
        "top-open band should be selected: {out}"
    );

    // A value below the smallest key has no key <= it → honest abstention.
    let below =
        format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = -1 mode range give category\n");
    write(&dir, "below.adj", &below);
    let (ok2, out2, _e2) = run_full(&dir.join("below.adj"));
    assert!(ok2);
    assert!(
        out2.contains("\"abstained\":true"),
        "below the domain must abstain, not fabricate: {out2}"
    );
    assert!(
        !out2.contains("\"category\":\""),
        "an abstention has no bound band: {out2}"
    );
}

// ---------------------------------------------------------------------------
// (d) Guards — reserved mode, unknown column, non-numeric key.
// ---------------------------------------------------------------------------

#[test]
fn interpolated_mode_over_a_non_numeric_value_column_is_rejected() {
    // `interpolated` (RS-5d) is now a built tactic, but it computes on the VALUE
    // column, so a non-numeric `give` column (here `category`) is a clean compile
    // error — you cannot linearly blend "overweight" and "obese".
    let dir = scratch("mode");
    let src = format!(
        "{BMI_TABLE}\n? lookup bmi_categories min_bmi = 27 mode interpolated give category\n"
    );
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(
        !ok,
        "interpolating a non-numeric value column must be rejected"
    );
    let diag = format!("{out}{err}");
    assert!(
        diag.contains("LookupNonNumericValueColumn") || diag.to_lowercase().contains("category"),
        "diag names the non-numeric value column: {diag}"
    );
}

#[test]
fn range_lookup_unknown_value_column_is_a_clean_compile_error() {
    let dir = scratch("col");
    let src =
        format!("{BMI_TABLE}\n? lookup bmi_categories min_bmi = 27 mode range give bmi_class\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "an unknown value column must be a compile error");
    let diag = format!("{out}{err}");
    assert!(
        diag.contains("LookupUnknownColumn") || diag.contains("bmi_class"),
        "diag: {diag}"
    );
}

#[test]
fn range_lookup_non_numeric_key_column_is_rejected() {
    let dir = scratch("nonnum");
    // Here the *category* (an atom column) is (mis)used as the key — a range key
    // must be numeric, so this is a clean compile error, not a silent skip.
    let src =
        format!("{BMI_TABLE}\n? lookup bmi_categories category = 27 mode range give min_bmi\n");
    write(&dir, "case.adj", &src);
    let (ok, out, err) = run_full(&dir.join("case.adj"));
    assert!(!ok, "a non-numeric key column must be rejected");
    let diag = format!("{out}{err}");
    assert!(
        diag.contains("LookupNonNumericKeyColumn") || diag.contains("category"),
        "diag: {diag}"
    );
}

/// A duplicate breakpoint reaching selection through a route the DECLARATION
/// gate cannot see.
///
/// `lower` rejects a `table` block that repeats a key, but the runtime never
/// reads that block — `range_lookup_json` enumerates every fact with the table's
/// functor and arity. A `relate` fact colliding with the relation contributes a
/// row the declaration-time check never examined, so before the runtime gate
/// this answered `first_ten` and cited only that row, while the separately
/// sourced `rogue_ten` vanished with `abstained: false`.
#[test]
fn a_relate_fact_cannot_smuggle_in_a_duplicate_breakpoint() {
    let dir = scratch("relate_dup");
    write(
        &dir,
        "p.adj",
        r#"
table band {
    columns min_v, label
    row (0, low) { source "zero band" }
    row (10, first_ten) { source "first ten band" }
    source "framing"
    locator "https://example.test/one"
    trust authoritative
}
relate band(10, rogue_ten) source "rogue row" locator "https://example.test/rogue" trust authoritative
? lookup band min_v = 12 mode range give label
"#,
    );
    let (ok, out, err) = run_full(&dir.join("p.adj"));
    assert!(ok, "{err}");
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"ambiguous_breakpoint\""),
        "a smuggled duplicate breakpoint must abstain, not pick one: {out}"
    );
    assert!(
        !out.contains("first_ten") && !out.contains("rogue_ten"),
        "neither tied row may be answered: {out}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// The same hole through a second `table` block of the same name — now closed
/// one layer EARLIER than the abstention above.
///
/// The lowerer kept only the last declaration while every block's rows were
/// lowered into the KB, so a shadowed block was invisible to the declaration
/// gate and fully visible to enumeration. That is now `LowerError::DuplicateTable`,
/// so this program never runs at all; the assertion is the compile error rather
/// than the runtime abstention it used to reach.
///
/// The runtime abstention is still the guarantee and still exercised — see the
/// `relate`-smuggling tests, which remain reachable because a `relate` fact is a
/// legitimate second producer of a relation rather than a name collision.
#[test]
fn a_second_table_block_cannot_smuggle_in_a_duplicate_breakpoint() {
    let dir = scratch("dup_table_block");
    write(
        &dir,
        "p.adj",
        r#"
table band {
    columns min_v, label
    row (0, low) { source "zero band" }
    row (10, first_ten) { source "first ten band" }
    source "framing one"
    locator "https://example.test/one"
    trust authoritative
}
table band {
    columns min_v, label
    row (10, shadow_ten) { source "shadow ten band" }
    source "framing two"
    locator "https://example.test/two"
    trust authoritative
}
? lookup band min_v = 12 mode range give label
"#,
    );
    let (_ok, out, err) = run_full(&dir.join("p.adj"));
    let reported = format!("{out}{err}");
    assert!(
        reported.contains("DuplicateTable"),
        "a shadowed table block must be rejected outright: {reported}"
    );
    assert!(
        !reported.contains("first_ten") && !reported.contains("shadow_ten"),
        "no row may be answered from a collided relation: {reported}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// `nearest` snaps to the closest key, and its documented tie-break (toward the
/// smaller key) settles two rows sitting either side of the query. A tie on the
/// KEY ITSELF is a different thing — one breakpoint with two differently sourced
/// answers — and must abstain rather than snap.
#[test]
fn nearest_abstains_on_a_duplicated_key_rather_than_snapping() {
    let dir = scratch("nearest_dup");
    write(
        &dir,
        "p.adj",
        r#"
table band {
    columns min_v, label
    row (0, low) { source "zero band" }
    row (10, first_ten) { source "first ten band" }
    source "framing"
    locator "https://example.test/one"
    trust authoritative
}
relate band(10, rogue_ten) source "rogue row" locator "https://example.test/rogue" trust authoritative
? lookup band min_v = 12 mode nearest give label
"#,
    );
    let (ok, out, err) = run_full(&dir.join("p.adj"));
    assert!(ok, "{err}");
    assert!(
        out.contains("\"abstained\":true") && out.contains("\"ambiguous_breakpoint\""),
        "nearest must abstain on a duplicated key: {out}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// The gate stays scoped: an ordinary table with distinct breakpoints still
/// answers, and `nearest`'s genuine either-side tie-break is untouched.
#[test]
fn distinct_breakpoints_are_unaffected_by_the_ambiguity_gate() {
    let dir = scratch("distinct_ok");
    write(
        &dir,
        "p.adj",
        r#"
table band {
    columns min_v, label
    row (0, low) { source "zero band" }
    row (10, mid) { source "ten band" }
    row (20, high) { source "twenty band" }
    source "framing"
    locator "https://example.test/one"
    trust authoritative
}
? lookup band min_v = 12 mode range give label
"#,
    );
    let (ok, out, err) = run_full(&dir.join("p.adj"));
    assert!(ok, "{err}");
    assert!(
        out.contains("\"abstained\":false") && out.contains("mid"),
        "a distinct-breakpoint table must still answer: {out}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// `interpolated` is the sharpest form of the shared-breakpoint problem: the
/// dropped row's VALUE feeds the blend, so the emitted NUMBER changes with which
/// row enumeration reached first. Before the fix this same knowledge base
/// answered `150` or `599.5` — each fully cited, neither abstaining — depending
/// only on whether the smuggled row was declared before or after the table.
///
/// The test runs both orders precisely because a mode-parity gap in this file is
/// what let the bug through: `range` and `nearest` had smuggling coverage and
/// `interpolated` did not.
#[test]
fn interpolated_abstains_on_a_smuggled_duplicate_in_either_order() {
    for (tag, before, after) in [
        ("rogue_after", "", ROGUE_TEN),
        ("rogue_before", ROGUE_TEN, ""),
    ] {
        let dir = scratch(tag);
        write(
            &dir,
            "p.adj",
            &format!(
                r#"
{before}
table band {{
    columns min_v, val
    row (0, 0) {{ source "zero band" }}
    row (10, 100) {{ source "first ten band" }}
    row (20, 200) {{ source "twenty band" }}
    source "framing"
    locator "https://example.test/one"
    trust authoritative
}}
{after}
? lookup band min_v = 15 mode interpolated give val
"#
            ),
        );
        let (ok, out, err) = run_full(&dir.join("p.adj"));
        assert!(ok, "{err}");
        assert!(
            out.contains("\"abstained\":true") && out.contains("\"ambiguous_breakpoint\""),
            "{tag}: a tied bracket endpoint must abstain: {out}"
        );
        assert!(
            !out.contains("150") && !out.contains("599.5"),
            "{tag}: no blended number may be emitted from a tied bracket: {out}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// A duplicate at a key the selection did NOT choose costs the answer nothing,
/// and must not abstain it.
///
/// The first version of this gate flagged ambiguity whenever an incoming key tied
/// the best-SO-FAR, so a duplicate on a losing key poisoned the query or not
/// purely by enumeration order — the same knowledge base answering `high` in one
/// declaration order and abstaining in the other. Ambiguity is now decided after
/// selection, against the key that actually won.
#[test]
fn a_duplicate_on_an_unselected_key_does_not_abstain_the_answer() {
    for (tag, before, after) in [
        ("losing_after", "", ROGUE_TEN_LABEL),
        ("losing_before", ROGUE_TEN_LABEL, ""),
    ] {
        let dir = scratch(tag);
        write(
            &dir,
            "p.adj",
            &format!(
                r#"
{before}
table band {{
    columns min_v, label
    row (0, low) {{ source "zero band" }}
    row (10, ten_a) {{ source "ten a" }}
    row (20, high) {{ source "twenty band" }}
    source "framing"
    locator "https://example.test/one"
    trust authoritative
}}
{after}
? lookup band min_v = 25 mode range give label
"#
            ),
        );
        let (ok, out, err) = run_full(&dir.join("p.adj"));
        assert!(ok, "{err}");
        assert!(
            out.contains("\"abstained\":false") && out.contains("high"),
            "{tag}: a duplicate at the unselected key 10 must not abstain a query that selected 20: {out}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// `nearest`'s documented tie-break survives: two rows equidistant on OPPOSITE
/// sides are a genuine choice, settled toward the smaller key, not an ambiguous
/// breakpoint.
#[test]
fn nearest_equidistant_tie_break_is_not_treated_as_ambiguous() {
    let dir = scratch("near_equidistant");
    write(
        &dir,
        "p.adj",
        r#"
table band {
    columns min_v, label
    row (8, eight) { source "eight band" }
    row (12, twelve) { source "twelve band" }
    source "framing"
    locator "https://example.test/one"
    trust authoritative
}
? lookup band min_v = 10 mode nearest give label
"#,
    );
    let (ok, out, err) = run_full(&dir.join("p.adj"));
    assert!(ok, "{err}");
    assert!(
        out.contains("\"abstained\":false") && out.contains("eight"),
        "an equidistant tie between DIFFERENT keys still resolves to the smaller: {out}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

const ROGUE_TEN: &str = r#"relate band(10, 999) source "rogue row" locator "https://example.test/rogue" trust authoritative"#;
const ROGUE_TEN_LABEL: &str = r#"relate band(10, ten_b) source "rogue ten" locator "https://example.test/rogue" trust authoritative"#;
