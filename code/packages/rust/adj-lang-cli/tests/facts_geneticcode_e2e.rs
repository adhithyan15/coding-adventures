//! End-to-end test for the BIOLOGY FACTS library
//! (`adj-facts-stdlib/biology/genetic-code.adj`) driven through the built CLI:
//! a native `table` of the STANDARD GENETIC CODE (NCBI translation table 1) maps
//! each mRNA codon to its amino acid. A binding-query recall returns the amino
//! acid with the NCBI citation, and abstains on a triplet that is not a real
//! codon (`xyz`) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsgc_{tag}_{}", std::process::id()));
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
fn biology_genetic_code_recall_binds_amino_acid_with_citation() {
    let dir = scratch("gc");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/genetic-code.adj");
    std::fs::copy(&src, dir.join("genetic-code.adj")).expect("copy shipped genetic-code.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"genetic-code.adj\"\n\
         ? codon_amino_acid(atg, $A)\n\
         ? codon_amino_acid(taa, $A)\n\
         ? codon_amino_acid(gag, $A)\n\
         ? codon_amino_acid(xyz, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Standard genetic code: AUG (atg) codes methionine (also START);
    // a third-base wobble / sense-vs-stop decode is what a mutation reasoner reads.
    assert!(out.contains("\"A\":\"m\""), "atg → m (methionine): {out}");
    // A STOP codon terminates translation — maps to the atom `stop`.
    assert!(out.contains("\"A\":\"stop\""), "taa → stop: {out}");
    // Glutamate — the sickle-cell wild-type codon (gag e → gtg v is the mutation).
    assert!(out.contains("\"A\":\"e\""), "gag → e (glutamate): {out}");
    // The answer carries the NCBI translation-table citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov/Taxonomy/Utils/wprintgc.cgi")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NCBI source citation: {out}"
    );
    // `xyz` is not a codon — honest abstention, never a fabricated amino acid.
    assert!(out.contains("\"abstained\":true"), "xyz abstains: {out}");
}

/// Installment 4h (#13934): NCBI serves this table inside a `<pre>` that indents LINE 1 BY FOUR SPACES and every later line by two, so all five data columns land at offset 11. Until installment 4h the field had all TWELVE of those characters stripped, so the LIBRARY header's claim that the block is "copied VERBATIM" was false (that phrase is `genetic-code.adj`'s; this test file makes no such claim) -- and the first attempt at the repair restored only eight, leaving the block MORE column-misaligned than it found it, which security review caught. Whitespace inside a `<pre>` is RENDERED whitespace, so restoring it settles none of the questions held on #14111 -- unlike newlines inside a wrapped `<p>`, which are source formatting and are the owner's call.
///
/// FULL ANCHORED CITATION PIN -- anchored on the `"source":"` key and
/// closed on the terminating quote, so head, tail, punctuation,
/// whitespace and length are pinned at once. The needle is generated
/// FROM the .adj field rather than retyped, because a pin and a field
/// that drift apart is a defect this effort has shipped twice.
#[test]
fn genetic_code_source_carries_the_pages_own_indentation() {
    let dir = scratch("pre_indent");
    let src = facts_stdlib().join("biology/genetic-code.adj");
    std::fs::copy(&src, dir.join("genetic-code.adj")).expect("copy shipped genetic-code.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"genetic-code.adj\"\n\
         ? codon_amino_acid(atg, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"source\":\"    AAs  = FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG\\n  Starts = ---M------**--*----M---------------M----------------------------\\n  Base1  = TTTTTTTTTTTTTTTTCCCCCCCCCCCCCCCCAAAAAAAAAAAAAAAAGGGGGGGGGGGGGGGG\\n  Base2  = TTTTCCCCAAAAGGGGTTTTCCCCAAAAGGGGTTTTCCCCAAAAGGGGTTTTCCCCAAAAGGGG\\n  Base3  = TCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAGTCAG\""),
        "the citation is the page's own text, exactly: {out}"
    );
}
