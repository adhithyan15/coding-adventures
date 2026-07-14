//! End-to-end test for the mathematics CONSTANTS facts library
//! (`adj-facts-stdlib/mathematics/constants.adj`) driven through the built CLI:
//! a native `table` of named constant → decimal value resolves a binding-query
//! recall with the source's citation, and abstains on a name that is not a row —
//! 0 model calls.

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
fn mathematics_constants_recall_binds_values_with_citation() {
    let dir = scratch("mathconstants");
    // Copy the shipped mathematics constants table beside the entry program and
    // import it.
    let src = facts_stdlib().join("mathematics/constants.adj");
    std::fs::copy(&src, dir.join("constants.adj")).expect("copy shipped constants.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"constants.adj\"\n\
         ? math_constant(pi, $V)\n\
         ? math_constant(e, $V)\n\
         ? math_constant(unicorn_constant, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // pi and e bind their MathWorld decimal values. The runtime binds a
    // double-precision float, so we assert on the leading-digit substring (robust
    // to how many low-order digits the float prints) rather than all 39 digits.
    assert!(
        out.contains("\"V\":\"3.14159265358979"),
        "pi binds its decimal value: {out}"
    );
    assert!(
        out.contains("\"V\":\"2.71828182845904"),
        "e binds its decimal value: {out}"
    );

    // The answer carries the Wolfram MathWorld citation, at the authoritative
    // trust tier, as its proof.
    assert!(
        out.contains("mathworld.wolfram.com/Pi.html")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld source citation: {out}"
    );
    // The `source` span quotes the verbatim text that carries pi's digits.
    assert!(
        out.contains("pi has decimal expansion given by pi=3.141592653589793238462643"),
        "source span quotes the verbatim digit text: {out}"
    );

    // `unicorn_constant` is not a row — honest abstention, never a fabricated
    // value.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown constant abstains: {out}"
    );
}
