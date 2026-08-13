//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/energy-form-family.adj`) driven through the
//! built CLI: a native `table` naming which of the source's two families
//! (potential/kinetic) an energy form belongs to -- a sibling to the
//! already-shipped `energy-forms.adj` (which only carries each form's
//! defining TOKEN), decoding the SAME EIA page's own two-heading structure,
//! re-verified live via WebFetch this cycle. Resolves binding-query recall
//! (both directions, including a genuine one-to-many reverse recall) with
//! the source's citation, and abstains on a form (sound) the source's page
//! names but `energy-forms.adj` never tabled -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_energyformfamily_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/energy-form-family.adj");
    std::fs::copy(&src, dir.join("energy-form-family.adj"))
        .expect("copy shipped energy-form-family.adj");
}

#[test]
fn energy_form_family_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-form-family.adj\"\n\
         ? energy_form_family(chemical, $Family)\n\
         ? energy_form_family(mechanical, $Family)\n\
         ? energy_form_family(nuclear, $Family)\n\
         ? energy_form_family(gravitational, $Family)\n\
         ? energy_form_family(radiant, $Family)\n\
         ? energy_form_family(thermal, $Family)\n\
         ? energy_form_family(motion, $Family)\n\
         ? energy_form_family(electrical, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for (form, family) in [
        ("chemical", "potential"),
        ("mechanical", "potential"),
        ("nuclear", "potential"),
        ("gravitational", "potential"),
        ("radiant", "kinetic"),
        ("thermal", "kinetic"),
        ("motion", "kinetic"),
        ("electrical", "kinetic"),
    ] {
        let term = format!("\"term\":\"energy_form_family({form}, {family})\"");
        assert!(out.contains(&term), "{form} should be {family}: {out}");
    }
    assert!(
        out.contains("eia.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the EIA citation: {out}"
    );
}

#[test]
fn energy_form_family_recalls_backward_all_four_kinetic_forms() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-form-family.adj\"\n\
         ? energy_form_family($Form, kinetic)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for form in ["radiant", "thermal", "motion", "electrical"] {
        let term = format!("\"term\":\"energy_form_family({form}, kinetic)\"");
        assert!(out.contains(&term), "kinetic should include {form}: {out}");
    }
}

#[test]
fn energy_form_family_abstains_honestly_on_sound() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"energy-form-family.adj\"\n\
         ? energy_form_family(sound, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "sound is not one of energy-forms.adj's eight tabled forms -- honest abstention: {out}"
    );
}
