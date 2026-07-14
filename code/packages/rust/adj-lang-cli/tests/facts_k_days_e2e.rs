//! End-to-end test for the kindergarten calendar FACTS library
//! (`adj-facts-stdlib/kindergarten/days-of-week.adj`) driven through the built CLI:
//! a native `table` of day → ISO weekday number resolves a binding-query recall
//! with the source's citation, and abstains on a non-day — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn kindergarten_days_recall_binds_iso_weekday_number_with_citation() {
    let dir = scratch("days");
    // Copy the shipped kindergarten table beside the entry program and import it.
    let src = facts_stdlib().join("kindergarten/days-of-week.adj");
    std::fs::copy(&src, dir.join("days-of-week.adj")).expect("copy shipped days-of-week.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"days-of-week.adj\"\n\
         ? iso_weekday(monday, $N)\n\
         ? iso_weekday(sunday, $N)\n\
         ? iso_weekday(funday, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Monday is ISO weekday 1; Sunday is 7 — the recalled numbers.
    assert!(out.contains("\"N\":\"1\""), "monday → 1: {out}");
    assert!(out.contains("\"N\":\"7\""), "sunday → 7: {out}");
    // The answer carries the ISO week-date citation as its proof. Wikipedia is a
    // secondary consensus summary of the (paywalled) ISO standard, so the tier is
    // `consensus`, not `authoritative`.
    assert!(
        out.contains("en.wikipedia.org/wiki/ISO_week_date")
            && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // "funday" is not a day of the week — honest abstention, never a fabricated number.
    assert!(out.contains("\"abstained\":true"), "funday abstains: {out}");
}
