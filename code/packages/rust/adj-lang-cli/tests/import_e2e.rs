//! End-to-end tests for `import` (MYCIN-2026 M3) through the built CLI binary.
//!
//! These exercise the real filesystem provider — the `import` trust boundary —
//! on multi-file `.adj` graphs: a dictionary, a rulebook that imports + `use`s
//! it, and a case that imports the rulebook. They also confirm the sandbox
//! rejects path-traversal / absolute / cyclic imports cleanly (no panic, no
//! infinite loop, no reading outside the program's directory).

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh, uniquely-named temp directory under the system temp dir. (Avoids a
/// dev-dependency on `tempfile`; `line!()` keeps concurrent tests from
/// colliding.)
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_import_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, src).unwrap();
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn three_file_chain_dictionary_rulebook_case_decides() {
    let dir = scratch("chain");
    write(
        &dir,
        "dictionary.adj",
        "dictionary meningitis_vocab {\n\
           define bacterial : hypothesis surface \"bacterial meningitis\"\n\
           define viral : hypothesis surface \"viral meningitis\"\n\
           define csf_glucose : finding values [low, normal] surface \"CSF glucose\"\n\
         }\n",
    );
    write(
        &dir,
        "rulebook.adj",
        "import \"dictionary.adj\"\n\
         rulebook meningitis {\n\
           use meningitis_vocab\n\
           prior 0.30 for bacterial\n  source \"Tunkel IDSA 2004\" trust authoritative\n\
           prior 0.30 for viral\n  source \"Tunkel IDSA 2004\" trust authoritative\n\
           contributes 5 from csf_glucose(low) to bacterial\n  source \"low CSF glucose favors bacterial\" trust empirical\n\
         }\n",
    );
    write(
        &dir,
        "case.adj",
        "import \"rulebook.adj\"\nobserve csf_glucose(low)\n? bacterial\n? viral\n",
    );

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    // The composed program decides: bacterial leads on low CSF glucose, and the
    // proof cites the rulebook clause's source — proving the imported rulebook +
    // dictionary were merged and lowered.
    assert!(s.contains("\"leader\":\"bacterial\""), "{s}");
    assert!(
        s.contains("\"source\":\"low CSF glucose favors bacterial\""),
        "{s}"
    );
    // closed-vocabulary enforcement from the imported dictionary held (no error).
    assert!(!s.contains("\"error\""), "{s}");
}

#[test]
fn diamond_import_does_not_duplicate_the_shared_dictionary() {
    // case imports two rulebooks, both importing the same dictionary. If the
    // dictionary's prior were merged twice it would be a DuplicatePrior error.
    let dir = scratch("diamond");
    write(
        &dir,
        "dict.adj",
        "dictionary v { define dx : hypothesis }\nprior 0.20 for dx\n  source \"s\" trust empirical\n",
    );
    write(&dir, "rb_a.adj", "import \"dict.adj\"\n");
    write(&dir, "rb_b.adj", "import \"dict.adj\"\n");
    write(
        &dir,
        "case.adj",
        "import \"rb_a.adj\"\nimport \"rb_b.adj\"\n? dx\n",
    );
    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero (duplicate prior?): {s}");
    assert!(s.contains("\"hypothesis\":\"dx\""), "{s}");
    assert!(!s.contains("\"error\""), "{s}");
}

#[test]
fn a_traversal_import_is_refused() {
    // A case in a subdirectory tries to `../`-escape to a sibling of the root.
    let dir = scratch("traversal");
    write(&dir, "secret.adj", "prior 0.99 for leaked\n? leaked\n");
    write(&dir, "sub/case.adj", "import \"../secret.adj\"\n? leaked\n");
    let (ok, s) = run(&dir.join("sub/case.adj"));
    assert!(!ok, "traversal import should fail: {s}");
    assert!(s.contains("escapes the import root"), "{s}");
}

#[test]
fn an_absolute_import_is_refused() {
    let dir = scratch("absolute");
    write(&dir, "secret.adj", "prior 0.99 for leaked\n? leaked\n");
    let abs = dir.join("secret.adj");
    write(
        &dir,
        "case.adj",
        &format!("import \"{}\"\n? leaked\n", abs.display()),
    );
    let (ok, s) = run(&dir.join("case.adj"));
    assert!(!ok, "absolute import should fail: {s}");
    assert!(s.contains("must be relative"), "{s}");
}

#[test]
fn an_import_cycle_is_refused_without_hanging() {
    let dir = scratch("cycle");
    write(&dir, "a.adj", "import \"b.adj\"\n? x\n");
    write(&dir, "b.adj", "import \"a.adj\"\n? x\n");
    let (ok, s) = run(&dir.join("a.adj"));
    assert!(!ok, "cyclic import should fail: {s}");
    assert!(s.contains("Cycle"), "{s}");
}
