//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/muscle-nuclei-count.adj`) driven through the
//! built CLI: a native `table` recording whether skeletal or cardiac
//! muscle fibers are multinucleated or have a single nucleus -- a sibling
//! to the already-shipped `tissue-types.adj` (which only carries a
//! representative example/location per tissue type), decoding the
//! nuclei-count clause already sitting unused inside that table's own
//! muscle-row header quotes. Resolves forward and backward recall queries
//! with the source's citation, plus honest abstention on smooth muscle
//! (outside this cited span) -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_musclenucleicount_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/muscle-nuclei-count.adj");
    std::fs::copy(&src, dir.join("muscle-nuclei-count.adj"))
        .expect("copy shipped muscle-nuclei-count.adj");
}

#[test]
fn muscle_nuclei_count_recalls_skeletal_as_multinucleated_with_citation() {
    let dir = scratch("skeletal");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-nuclei-count.adj\"\n\
         ? muscle_nuclei_count(skeletal, $Nuclei)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"muscle_nuclei_count(skeletal, multinucleated)\""),
        "skeletal muscle should recall as multinucleated: {out}"
    );
    assert!(
        out.contains("seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn muscle_nuclei_count_backward_recalls_cardiac_for_single_nucleus() {
    let dir = scratch("cardiac");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-nuclei-count.adj\"\n\
         ? muscle_nuclei_count($Muscle, single_nucleus)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"muscle_nuclei_count(cardiac, single_nucleus)\""),
        "cardiac muscle should be the only recalled single-nucleus type: {out}"
    );
    assert!(
        !out.contains("muscle_nuclei_count(skeletal, single_nucleus)"),
        "skeletal muscle is multinucleated, not single-nucleus: {out}"
    );
}

#[test]
fn muscle_nuclei_count_abstains_on_smooth_muscle() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-nuclei-count.adj\"\n\
         ? muscle_nuclei_count(smooth, $Nuclei)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "smooth muscle is not part of tissue-types.adj's muscle citation -- honest abstention expected: {out}"
    );
}
