//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/solid-vertices-edges.adj`) driven through the
//! built CLI: native `table`s of solid → vertices and solid → edges resolve
//! binding-query recalls with the source's citation, and abstain on a non-solid
//! — 0 model calls. Completes the (V, E, F) triple whose faces sibling ships in
//! `solid-shapes.adj`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsve_{tag}_{}", std::process::id()));
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

fn with_lib(dir: &Path) {
    let src = facts_stdlib().join("geometry/solid-vertices-edges.adj");
    std::fs::copy(&src, dir.join("solid-vertices-edges.adj"))
        .expect("copy shipped solid-vertices-edges.adj");
}

#[test]
fn vertices_recall_binds_count_with_citation() {
    let dir = scratch("vertices");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solid-vertices-edges.adj\"\n\
         ? solid_vertices(cube, $V)\n\
         ? solid_vertices(dodecahedron, $V)\n\
         ? solid_vertices(sphere, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A cube has eight vertices; a dodecahedron has twenty.
    assert!(out.contains("\"V\":\"8\""), "cube → 8 vertices: {out}");
    assert!(out.contains("\"V\":\"20\""), "dodecahedron → 20 vertices: {out}");
    assert!(
        out.contains("mathworld.wolfram.com") && out.contains("\"trust\":\"authoritative\""),
        "carries the MathWorld citation: {out}"
    );
    // A sphere is not a Platonic solid — honest abstention, never a fabricated count.
    assert!(out.contains("\"abstained\":true"), "sphere abstains: {out}");
}

#[test]
fn edges_recall_binds_count_with_citation() {
    let dir = scratch("edges");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solid-vertices-edges.adj\"\n\
         ? solid_edges(cube, $E)\n\
         ? solid_edges(icosahedron, $E)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // A cube has twelve edges; an icosahedron has thirty.
    assert!(out.contains("\"E\":\"12\""), "cube → 12 edges: {out}");
    assert!(out.contains("\"E\":\"30\""), "icosahedron → 30 edges: {out}");
    assert!(
        out.contains("mathworld.wolfram.com/PlatonicSolid.html"),
        "carries the MathWorld PlatonicSolid citation: {out}"
    );
}

#[test]
fn the_three_libraries_together_satisfy_eulers_formula() {
    // The payoff of shipping V and E beside the existing F: recall all three and
    // Euler's V - E + F = 2 holds for every Platonic solid, checkable on the CPU
    // from grounded facts alone. Here we assert the recalled counts for the cube
    // (8 - 12 + 6 = 2); the arithmetic itself is exercised elsewhere.
    let dir = scratch("euler");
    with_lib(&dir);
    let src = facts_stdlib().join("geometry/solid-shapes.adj");
    std::fs::copy(&src, dir.join("solid-shapes.adj")).expect("copy shipped solid-shapes.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"solid-vertices-edges.adj\"\n\
         import \"solid-shapes.adj\"\n\
         ? solid_vertices(cube, $V)\n\
         ? solid_edges(cube, $E)\n\
         ? solid_faces(cube, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // 8 vertices, 12 edges, 6 faces — the operands of 8 - 12 + 6 = 2, all grounded.
    assert!(out.contains("\"V\":\"8\""), "cube V=8: {out}");
    assert!(out.contains("\"E\":\"12\""), "cube E=12: {out}");
    assert!(out.contains("\"F\":\"6\""), "cube F=6: {out}");
}

/// Installment 4h (#13934): Until installment 4h this field held
/// author-composed prose ABOUT the page -- "MathWorld's Platonic Solid
/// table gives, for each solid, its number of vertices: tetrahedron 4, cube
/// 8, ..." -- which names the source in the third person, a thing no
/// verbatim span ever does. It is now the page's own sentence. NOTE the
/// weaker claim than installment 4g's: this span is verbatim under the
/// extractor's whitespace collapse, NOT byte-exact against raw HTML,
/// because the page's own bytes wrap the paragraph across three newlines.
/// See #14111.
///
/// FULL ANCHORED CITATION PIN -- anchored on the `"source":"` key and
/// closed on the terminating quote, so head, tail, punctuation,
/// whitespace and length are pinned at once. The needle is generated
/// FROM the .adj field rather than retyped, because a pin and a field
/// that drift apart is a defect this effort has shipped twice.
#[test]
fn solid_vertices_source_is_the_pages_sentence_not_a_description() {
    let dir = scratch("span_not_description");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solid-vertices-edges.adj\"\n\
         ? solid_vertices(cube, $V)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"source\":\"The ordered number of faces for the Platonic solids are 4, 6, 8, 12, 20 (OEIS A053016; in the order tetrahedron, cube, octahedron, dodecahedron, icosahedron), which is also the ordered number of vertices (in the order tetrahedron, octahedron, cube, icosahedron, dodecahedron).\""),
        "the citation is the page's own text, exactly: {out}"
    );
}

/// Installment 4h (#13934): The edges half of the same repair. This sentence needs less care than the vertices one: it states its own solid order inline, with `=` marking the two pairs that share a count, so no re-ordering inference is involved.
///
/// FULL ANCHORED CITATION PIN -- anchored on the `"source":"` key and
/// closed on the terminating quote, so head, tail, punctuation,
/// whitespace and length are pinned at once. The needle is generated
/// FROM the .adj field rather than retyped, because a pin and a field
/// that drift apart is a defect this effort has shipped twice.
#[test]
fn solid_edges_source_is_the_pages_sentence_not_a_description() {
    let dir = scratch("edges_span_not_description");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"solid-vertices-edges.adj\"\n\
         ? solid_edges(cube, $E)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"source\":\"The ordered number of edges are 6, 12, 12, 30, 30 (OEIS A063722; in the order tetrahedron, octahedron = cube, dodecahedron = icosahedron).\""),
        "the citation is the page's own text, exactly: {out}"
    );
}
