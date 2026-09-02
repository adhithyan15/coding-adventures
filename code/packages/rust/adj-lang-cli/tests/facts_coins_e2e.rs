//! End-to-end test for the US-coins FACTS library
//! (`adj-facts-stdlib/money/us-coins.adj`) driven through the built CLI:
//! a native `table` of coin → value-in-cents resolves a binding-query recall
//! with the U.S. Mint's citation, runs the relation backwards (value → coin),
//! and abstains on a non-coin — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_facts_{tag}_{}", std::process::id()));
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
fn coins_recall_binds_cent_value_with_citation() {
    let dir = scratch("coins");
    // Copy the shipped money table beside the entry program and import it.
    let src = facts_stdlib().join("money/us-coins.adj");
    std::fs::copy(&src, dir.join("us-coins.adj")).expect("copy shipped us-coins.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"us-coins.adj\"\n\
         ? coin_cents(quarter, $Cents)\n\
         ? coin_cents(penny, $Cents)\n\
         ? coin_cents($Coin, 5)\n\
         ? coin_cents(doubloon, $Cents)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A quarter is worth twenty-five cents; a penny is worth one — the recalled values.
    assert!(out.contains("\"Cents\":\"25\""), "quarter → 25: {out}");
    assert!(out.contains("\"Cents\":\"1\""), "penny → 1: {out}");
    // The relation runs backwards: the coin worth five cents is the nickel.
    assert!(out.contains("\"Coin\":\"nickel\""), "5 cents → nickel: {out}");
    // The answer carries the U.S. Mint citation as its proof.
    assert!(
        out.contains("kids.usmint.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the U.S. Mint citation: {out}"
    );
    // A doubloon is not a US coin — honest abstention, never a fabricated value.
    assert!(out.contains("\"abstained\":true"), "doubloon abstains: {out}");
}

const COINS_PREFIX_PIN: &str = r#""bindings":{"Coin":"nickel"},"citations":[{"source":"The denominations you'll see the most are the penny, nickel, dime, and quarter. The Mint makes half dollars and dollars for collecting, but you can still spend them.","locator":"https://kids.usmint.gov/about-the-mint","trust":"authoritative","corroborations":[{"source":"The one-cent coin ceased circulating in 2025 after 232 years of production.","locator":"https://kids.usmint.gov/about-the-mint/penny""#;

const COINS_ALL_PIN: &str = r#""bindings":{"Cents":"25"},"citations":[{"source":"The denominations you'll see the most are the penny, nickel, dime, and quarter. The Mint makes half dollars and dollars for collecting, but you can still spend them.","locator":"https://kids.usmint.gov/about-the-mint","trust":"authoritative","corroborations":[{"source":"The one-cent coin ceased circulating in 2025 after 232 years of production.","locator":"https://kids.usmint.gov/about-the-mint/penny"},{"source":"The nickel is the United States' five-cent coin.","locator":"https://kids.usmint.gov/about-the-mint/nickel"},{"source":"The dime is the United States' 10-cent coin.","locator":"https://kids.usmint.gov/about-the-mint/dime"},{"source":"In 1804, the Mint marked the quarter with \"25c,\" meaning 25 cents.","locator":"https://kids.usmint.gov/about-the-mint/quarter"},{"source":"The half dollar is the United States' 50-cent coin.","locator":"https://kids.usmint.gov/about-the-mint/half-dollar"},{"source":"The dollar is the United States' 100-cent coin. It takes 100 pennies to equal a dollar!","locator":"https://kids.usmint.gov/about-the-mint/dollar""#;

#[test]
fn coins_nickel_answer_carries_its_us_mint_corroboration_intact() {
    let dir = scratch("cite_nickel");
    std::fs::copy(
        facts_stdlib().join("money/us-coins.adj"),
        dir.join("us-coins.adj"),
    )
    .expect("copy shipped us-coins.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"us-coins.adj\"\n? coin_cents($Coin, 5)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // ANCHORED and JOINT: bindings + envelope + first corroboration in ONE
    // contiguous span, ending on a closing quote.
    assert!(
        out.contains(COINS_PREFIX_PIN),
        "nickel's answer carries the Mint's penny corroboration intact: {out}"
    );
}

#[test]
fn coins_quarter_answer_carries_all_six_denomination_corroborations_in_order() {
    let dir = scratch("cite_quarter");
    std::fs::copy(
        facts_stdlib().join("money/us-coins.adj"),
        dir.join("us-coins.adj"),
    )
    .expect("copy shipped us-coins.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"us-coins.adj\"\n? coin_cents(quarter, $Cents)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Spans the WHOLE corroboration list, so a reorder or a dropped middle
    // entry fails even though every sentence is still present somewhere.
    //
    // The quarter's own sentence carries EMBEDDED DOUBLE QUOTES -- the Mint
    // "marked the quarter with \"25c,\"" -- making this the second string in
    // the stdlib to use the lexer's `\"` escape. The pin covers the whole
    // round trip: escaped in the .adj, a real quote in the value, re-escaped
    // in the JSON.
    //
    // FOUR of these six are present-tense definitions ("The nickel is the
    // United States' five-cent coin"); penny and quarter are NOT. Their pages
    // are written historically -- penny's sentence is about the coin ceasing
    // to circulate, quarter's about an 1804 marking -- so each states its
    // value obliquely. Both still state it, and both locators resolve to
    // exactly one coin, which is why they are cited rather than refused.
    assert!(
        out.contains(COINS_ALL_PIN),
        "quarter's answer carries all six Mint sentences in order: {out}"
    );
}
