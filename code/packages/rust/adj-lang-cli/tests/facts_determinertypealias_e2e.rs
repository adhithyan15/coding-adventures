//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/determiner-type-alias.adj`) driven through
//! the built CLI: a native `table` naming the alternate name for the
//! demonstrative determiner, decoded from a span already sitting unused
//! inside the SAME Grammarly quote `determiner-type.adj`'s own header
//! already reproduces -- a sibling to that table. Resolves binding-query
//! recall (both directions) with the source's citation, and abstains on a
//! real, already-tabled determiner type (article) whose own sentence
//! states no alias -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_determinertypealias_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/determiner-type-alias.adj");
    std::fs::copy(&src, dir.join("determiner-type-alias.adj"))
        .expect("copy shipped determiner-type-alias.adj");
}

#[test]
fn determiner_type_alias_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"determiner-type-alias.adj\"\n\
         ? determiner_type_alias(demonstrative_determiner, $Alias)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"determiner_type_alias(demonstrative_determiner, demonstrative_adjective)\""),
        "demonstrative determiners are also known as demonstrative adjectives: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn determiner_type_alias_recalls_backward_from_a_bound_alias() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"determiner-type-alias.adj\"\n\
         ? determiner_type_alias($Type, demonstrative_adjective)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"determiner_type_alias(demonstrative_determiner, demonstrative_adjective)\""),
        "the alias names demonstrative_determiner: {out}"
    );
}

#[test]
fn determiner_type_alias_abstains_honestly_on_article() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"determiner-type-alias.adj\"\n\
         ? determiner_type_alias(article, $Alias)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "article's own sentence states no alias -- honest abstention: {out}"
    );
}
