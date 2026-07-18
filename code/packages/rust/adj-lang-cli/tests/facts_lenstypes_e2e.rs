//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/lens-types.adj`) driven through the built CLI:
//! a native `table` of the four basic optical elements (converging vs diverging
//! lens and mirror) → how each acts on parallel light resolves binding-query
//! recalls (forward AND backward) with the source's OpenStax / Physics
//! LibreTexts citation, and abstains on an element that is not one of the four
//! basic converging/diverging types (a prism) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn physics_lens_types_recall_binds_action_with_citation() {
    let dir = scratch("lenstypes");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/lens-types.adj");
    std::fs::copy(&src, dir.join("lens-types.adj")).expect("copy shipped lens-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"lens-types.adj\"\n\
         ? optic_action(convex_lens, $Action)\n\
         ? optic_action(concave_lens, $Action)\n\
         ? optic_action(convex_mirror, $Action)\n\
         ? optic_action($Optic, converges_light)\n\
         ? optic_action(prism, $Action)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A convex lens and a concave mirror converge light; a concave lens and a
    // convex mirror diverge it — the recalled actions (forward binds).
    assert!(
        out.contains("\"Action\":\"converges_light\""),
        "convex_lens → converges_light: {out}"
    );
    assert!(
        out.contains("\"Action\":\"diverges_light\""),
        "concave_lens / convex_mirror → diverges_light: {out}"
    );
    // The relation runs BACKWARD: bind the action `converges_light`, recall which
    // elements do it. Both the convex lens and the concave mirror converge light,
    // so the reverse bind surfaces the mirror row too.
    assert!(
        out.contains("\"Optic\":\"concave_mirror\""),
        "converges_light → concave_mirror (reverse recall): {out}"
    );
    // The answer carries the OpenStax / Physics LibreTexts citation as its proof,
    // at the `consensus` trust tier for a widely used teaching resource (not a
    // primary government/standards body).
    assert!(
        out.contains("phys.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // A prism disperses light — it is not a converging or diverging lens/mirror,
    // so it is not a row: honest abstention, never a fabricated action.
    assert!(out.contains("\"abstained\":true"), "prism abstains: {out}");
}
