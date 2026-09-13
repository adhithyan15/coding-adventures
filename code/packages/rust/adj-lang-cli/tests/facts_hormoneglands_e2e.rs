//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/hormone-glands.adj`) driven through the built CLI:
//! a native `table` mapping each hormone → the endocrine gland that secretes it
//! resolves binding-query recalls (forward AND backward) with the source's NCI
//! SEER Training Modules citation, and abstains on a hormone that is not one of
//! the grounded rows (aldosterone) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsh_{tag}_{}", std::process::id()));
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
fn biology_hormone_glands_recall_binds_gland_with_citation() {
    let dir = scratch("hormoneglands");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/hormone-glands.adj");
    std::fs::copy(&src, dir.join("hormone-glands.adj")).expect("copy shipped hormone-glands.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"hormone-glands.adj\"\n\
         ? hormone_gland(insulin, $Gland)\n\
         ? hormone_gland(cortisol, $Gland)\n\
         ? hormone_gland(melatonin, $Gland)\n\
         ? hormone_gland($Hormone, pituitary)\n\
         ? hormone_gland(aldosterone, $Gland)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Insulin comes from the pancreas, cortisol from the adrenal glands, melatonin
    // from the pineal gland — the recalled glands (forward binds).
    assert!(
        out.contains("\"Gland\":\"pancreas\""),
        "insulin → pancreas: {out}"
    );
    assert!(
        out.contains("\"Gland\":\"adrenal_gland\""),
        "cortisol → adrenal_gland: {out}"
    );
    assert!(
        out.contains("\"Gland\":\"pineal_gland\""),
        "melatonin → pineal_gland: {out}"
    );
    // The relation runs BACKWARD: bind the gland `pituitary`, recall the hormone
    // it makes.
    assert!(
        out.contains("\"Hormone\":\"growth_hormone\""),
        "pituitary → growth_hormone (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    //
    // THE WHOLE LOCATOR BOUND TO THE TIER, not the host and the tier as two
    // loose substrings. A host-only pin cannot tell one SEER page from another,
    // which matters now that twelve rows cite seven different pages — and #15139
    // found eight files whose locators had rotted underneath exactly this shape
    // of assertion without a single test noticing.
    assert!(
        out.contains(
            "\"locator\":\"https://training.seer.cancer.gov/anatomy/endocrine/glands/pancreas.html\",\"trust\":\"authoritative\""
        ),
        "insulin's citation names the pancreas page specifically: {out}"
    );
    // Aldosterone is a real adrenal hormone but is deliberately NOT a row (its
    // fetched span did not name the adrenal gland) — honest abstention, never a
    // fabricated gland.
    assert!(
        out.contains("\"abstained\":true"),
        "aldosterone abstains: {out}"
    );
}

const SEER: &str = "https://training.seer.cancer.gov/anatomy/endocrine/glands/";

/// Assert one row carries its own span AND its own locator, in a program that
/// returns only that row.
///
/// The gland is pinned in the same call. Review found `mesosphere -> meteors`
/// entirely unpinned in the atmosphere entry — the span assertion never read
/// the binding, so the value could be changed to anything and the suite stayed
/// green. Here every hormone's gland, span and locator move together.
///
/// The needle binds `locator` to `trust`, not two loose substrings: a
/// corroboration entry serializes with `locator` as its last key, so only a
/// row's own warrant can match. A host-only pin (`contains("seer.cancer.gov")`)
/// would survive a row citing entirely the wrong page on that host, and would
/// have survived the locator rot #15139 found in eight other files.
fn assert_hormone(tag: &str, hormone: &str, gland: &str, span: &str, locator: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("biology/hormone-glands.adj"),
        dir.join("hormone-glands.adj"),
    )
    .expect("copy shipped hormone-glands.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"hormone-glands.adj\"\n? hormone_gland({hormone}, $Gland)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so every needle below belongs to {hormone}: {out}"
    );
    assert!(
        out.contains(&format!("\"Gland\":\"{gland}\"")),
        "{hormone} binds {gland}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{locator}\",\"trust\":\"authoritative\""
        )),
        "{hormone} carries its own span and locator: {out}"
    );
}

/// #14986: this table had ONE envelope and it was the INSULIN row's own
/// sentence, so recalling progesterone came back warranted by a sentence about
/// beta cells. Twelve rows, seven pages — the locator is part of each row's
/// warrant here, not a table-level detail.
#[test]
fn every_hormone_row_carries_its_own_span_and_locator() {
    assert_hormone(
        "hginsulin", "insulin", "pancreas",
        "Beta cells in the pancreatic islets secrete the hormone insulin in response to a high concentration of glucose in the blood.",
        &format!("{SEER}pancreas.html"),
    );
    assert_hormone(
        "hgcalcitonin", "calcitonin", "thyroid",
        "Calcitonin is secreted by the parafollicular cells of the thyroid gland.",
        &format!("{SEER}thyroid.html"),
    );
    assert_hormone(
        "hgepinephrine", "epinephrine", "adrenal_gland",
        "The adrenal medulla develops from neural tissue and secretes two hormones, epinephrine and norepinephrine.",
        &format!("{SEER}adrenal.html"),
    );
    assert_hormone(
        "hgpth", "parathyroid_hormone", "parathyroid_gland",
        "These are parathyroid glands, and they secrete parathyroid hormone or parathormone.",
        &format!("{SEER}thyroid.html"),
    );
    assert_hormone(
        "hgtestosterone", "testosterone", "testis",
        "The principal androgen is testosterone, which is secreted by the testes.",
        &format!("{SEER}gonads.html"),
    );
    assert_hormone(
        "hgcortisol", "cortisol", "adrenal_gland",
        "Cortisol is a hormone made by your adrenal glands, two small glands that sit above your kidneys.",
        "https://medlineplus.gov/lab-tests/cortisol-test/",
    );
}

/// The three rows whose own sentence does not name its subject, widened per
/// ../README.md ("A citation must name its own subject"): `the gland` for
/// thyroxine, `the pinealocytes` for melatonin, and the bare initialism `GH`
/// for growth hormone. Each span is widened to the nearest sentence supplying
/// the missing name and no further.
#[test]
fn the_three_widened_rows_name_their_own_subject() {
    assert_hormone(
        "hgthyroxine", "thyroxine", "thyroid",
        "The thyroid gland is a very vascular organ that is located in the neck. It consists of two lobes, one on each side of the trachea, just below the larynx or voice box. The two lobes are connected by a narrow band of tissue called the isthmus. Internally, the gland consists of follicles, which produce thyroxine and triiodothyronine hormones.",
        &format!("{SEER}thyroid.html"),
    );
    assert_hormone(
        "hgmelatonin", "melatonin", "pineal_gland",
        "The pineal gland consists of portions of neurons, neuroglial cells, and specialized secretory cells called pinealocytes. The pinealocytes synthesize the hormone melatonin and secrete it directly into the cerebrospinal fluid, which takes it into the blood.",
        &format!("{SEER}pituitary.html"),
    );
    assert_hormone(
        "hggh", "growth_hormone", "pituitary",
        "GH, also known as human growth hormone, controls your body's growth. It also helps control metabolism, the process your body uses to make energy from the food you eat. GH is made in the pituitary gland, a small organ at the base of your brain that controls many functions, including growth.",
        "https://medlineplus.gov/lab-tests/growth-hormone-tests/",
    );
}

/// `estrogen` and `progesterone` are stated by ONE sentence. Each row carries
/// its own copy rather than one row citing the other's, so either can be
/// re-checked or changed alone — and so a mutation of one does not silently
/// take the other with it.
#[test]
fn the_two_ovary_rows_each_carry_their_own_copy_of_the_shared_sentence() {
    let span = "Two groups of female sex hormones are produced in the ovaries, the estrogens and progesterone.";
    assert_hormone("hgestrogen", "estrogen", "ovary", span, &format!("{SEER}gonads.html"));
    assert_hormone("hgprogesterone", "progesterone", "ovary", span, &format!("{SEER}gonads.html"));
}

/// The SEER page writes "glucagons". The span is quoted as the page has it —
/// silently correcting a quote is how a citation stops being checkable — while
/// the row atom stays `glucagon`. This test pins BOTH halves of that split, so
/// nobody "fixes" the quote later without the test objecting.
#[test]
fn the_glucagon_row_quotes_the_pages_own_spelling() {
    assert_hormone(
        "hgglucagon", "glucagon", "pancreas",
        "Alpha cells in the pancreatic islets secrete the hormone glucagons in response to a low concentration of glucose in the blood.",
        &format!("{SEER}pancreas.html"),
    );
}

/// The envelope is unreachable by construction — every row overrides `source`
/// and `locator`. Its wording cannot be pinned by any output test (fabricating
/// it leaves this suite green, which is disclosed rather than implied); what
/// CAN be pinned is that it warrants no row.
#[test]
fn the_framing_envelope_never_reaches_an_answer() {
    let dir = scratch("hgenvelope");
    std::fs::copy(
        facts_stdlib().join("biology/hormone-glands.adj"),
        dir.join("hormone-glands.adj"),
    )
    .expect("copy shipped hormone-glands.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"hormone-glands.adj\"\n? hormone_gland($H, $G)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        12,
        "all twelve rows answer: {out}"
    );
    assert!(
        !out.contains("The endocrine system is made up of the endocrine glands"),
        "the framing span warrants no row: {out}"
    );
    // The endocrine INDEX page is the envelope's locator and no row's. If a row
    // ever loses its own locator it silently inherits this one, which does not
    // carry that row's sentence — the regression this conversion could most
    // easily have introduced, since two rows needed a locator added by hand
    // once the envelope moved off pancreas.html.
    assert!(
        !out.contains("\"locator\":\"https://training.seer.cancer.gov/anatomy/endocrine/glands/\""),
        "no row inherits the index locator: {out}"
    );
}
