//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/simile-meaning.adj`) driven through the
//! built CLI: a native `table` naming three common similes and what each
//! actually means, quoted verbatim from Grammarly's "Simile: Definition
//! and Examples" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_simile_meaning_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/simile-meaning.adj");
    std::fs::copy(&src, dir.join("simile-meaning.adj"))
        .expect("copy shipped simile-meaning.adj");
}

#[test]
fn simile_meaning_recall_binds_the_meaning_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simile-meaning.adj\"\n\
         ? simile_meaning(as_brave_as_a_lion, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"M\":\"extremely_courageous\""),
        "as brave as a lion means extremely_courageous: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn simile_meaning_reverse_binds_the_simile_for_that_meaning() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simile-meaning.adj\"\n\
         ? simile_meaning($S, free_or_unrestricted)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"as_free_as_a_bird\""),
        "the shipped free_or_unrestricted example is as_free_as_a_bird: {out}"
    );
}

#[test]
fn simile_meaning_abstains_honestly_on_an_untabled_simile() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simile-meaning.adj\"\n\
         ? simile_meaning(as_busy_as_a_bee, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "as_busy_as_a_bee is a real simile but not one of the tabled ones -- honest abstention, never invented: {out}"
    );
}

#[test]
fn simile_meaning_extension_recalls_the_newly_added_rows() {
    let dir = scratch("ext");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"simile-meaning.adj\"\n\
         ? simile_meaning(like_a_fish_out_of_water, $M)\n\
         ? simile_meaning($S, very_strong)\n\
         ? simile_meaning(as_hungry_as_a_horse, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // 12 similes were added this cycle from the SAME already-cited Grammarly
    // "Common simile examples" table (originally only 3 of its 15 rows were
    // shipped).
    assert!(
        out.contains("\"M\":\"uncomfortable_or_out_of_place\""),
        "like a fish out of water → uncomfortable_or_out_of_place: {out}"
    );
    assert!(
        out.contains("\"S\":\"as_strong_as_an_ox\""),
        "very_strong → as_strong_as_an_ox (reverse recall): {out}"
    );
    assert!(
        out.contains("\"M\":\"extremely_eager_to_eat\""),
        "as hungry as a horse → extremely_eager_to_eat: {out}"
    );
}
