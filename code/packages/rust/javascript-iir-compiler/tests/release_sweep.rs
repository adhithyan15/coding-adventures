//! The LANG84 release sweep for ECMAScript (phase 6d, "CI bounding" steps
//! 1–2), on the model of Ruby's (phase 5b) and Python's (phase 7b).
//!
//! Ten editions claim `es-number-log` at stage I (es2016 … es2025). Running
//! every edition on every backend would multiply the matrix by ten; LANG84
//! bounds the work by **IIR identity** instead: two editions that produce
//! byte-identical IIR for every case (and refuse the same cases) are
//! indistinguishable to every backend, so running one proves the other.
//!
//! ```text
//!   step 1  compile every positive case under every claiming release
//!   step 2  cell(R, C) = SHA-256(serialised IIR of C under R) | "rejected"
//!           class(R)   = SHA-256(all of R's cells), first 16 hex digits
//!   step 3  representative(class) = its first release in inventory order;
//!           lang_matrix (phase 6e) runs every case a representative accepts
//! ```
//!
//! A positive case may be refused under a release only where the corpus says
//! so (`<case>.reject@<release>`); that cell is `rejected`, and every other
//! refusal fails the test. `IIRModule::release` is provenance, not code:
//! `interpreter_ir::serialise` does not write it.
//!
//! The result is `code/grammars/ecmascript/release_classes.csv`. This test
//! regenerates it and fails if the committed file differs. To accept a
//! deliberate change (the write itself fails, and is refused under CI):
//!
//! ```text
//!   JS_RELEASE_SWEEP_WRITE=1 cargo test -p javascript-iir-compiler --test release_sweep
//! ```

mod common;

use std::fmt::Write as _;

use coding_adventures_sha256::sha256_hex;
use common::{claiming_releases, corpus, ecmascript_grammars, Case, Outcome};
use interpreter_ir::serialise::serialise;
use javascript_iir_compiler::{compile_source_at, DEFAULT_RELEASE, SUBSET};

/// One release's verdict on one positive case.
fn sweep_cell(case: &Case, release: &str) -> String {
    match case.outcome_on(release) {
        Outcome::Rejected(category) => {
            let error = compile_source_at(&case.source, Some(release), "sweep")
                .expect_err(&format!("{release} {}: the corpus says it is refused", case.name));
            assert_eq!(error.category(), category, "{release} {}", case.name);
            "rejected".to_string()
        }
        Outcome::Runs(_) => {
            let module = compile_source_at(&case.source, Some(release), "sweep").unwrap_or_else(|error| {
                panic!("{release} {}: a claiming release refused a positive case: {error}", case.name)
            });
            sha256_hex(&serialise(&module))
        }
    }
}

struct Sweep {
    cases: Vec<Case>,
    cells: Vec<(String, Vec<String>)>,
}

fn run_sweep() -> Sweep {
    let cases: Vec<Case> = corpus().into_iter().filter(Case::is_positive).collect();
    let cells = claiming_releases()
        .into_iter()
        .map(|release| {
            let cells = cases.iter().map(|case| sweep_cell(case, &release)).collect();
            (release, cells)
        })
        .collect();
    Sweep { cases, cells }
}

fn class_of(cases: &[Case], cells: &[String]) -> String {
    let mut outcome = String::new();
    for (case, cell) in cases.iter().zip(cells) {
        writeln!(outcome, "{SUBSET}/{}={cell}", case.name).unwrap();
    }
    sha256_hex(outcome.as_bytes())[..16].to_string()
}

/// `release_classes.csv` as the sweep sees it. A refused cell is written as
/// `rejected`; every other cell carries its release's class.
fn render(sweep: &Sweep) -> String {
    let classes: Vec<String> = sweep.cells.iter().map(|(_, cells)| class_of(&sweep.cases, cells)).collect();
    let mut csv = String::from("subset,case,release,class\n");
    for (index, case) in sweep.cases.iter().enumerate() {
        for ((release, cells), class) in sweep.cells.iter().zip(&classes) {
            let shown = if cells[index] == "rejected" { "rejected" } else { class.as_str() };
            writeln!(csv, "{SUBSET},{},{release},{shown}", case.name).unwrap();
        }
    }
    csv
}

/// Classes in order of first appearance: (class, representative, members).
fn classes(sweep: &Sweep) -> Vec<(String, &str, Vec<&str>)> {
    let mut found: Vec<(String, &str, Vec<&str>)> = Vec::new();
    for (release, cells) in &sweep.cells {
        let class = class_of(&sweep.cases, cells);
        match found.iter_mut().find(|(c, _, _)| *c == class) {
            Some((_, _, members)) => members.push(release),
            None => found.push((class, release, vec![release])),
        }
    }
    found
}

#[test]
fn release_classes_csv_matches_the_sweep() {
    let sweep = run_sweep();
    let rendered = render(&sweep);
    let path = ecmascript_grammars().join("release_classes.csv");
    if std::env::var_os("JS_RELEASE_SWEEP_WRITE").is_some() {
        assert!(
            std::env::var_os("CI").is_none() && std::env::var_os("GITHUB_ACTIONS").is_none(),
            "JS_RELEASE_SWEEP_WRITE is for a developer's checkout, never for CI"
        );
        std::fs::write(&path, &rendered).unwrap();
        panic!("wrote {}; review the diff, then re-run without JS_RELEASE_SWEEP_WRITE", path.display());
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == rendered,
        "{} is stale: an edition changed IIR-identity class. Review the change, then run \
         JS_RELEASE_SWEEP_WRITE=1 cargo test -p javascript-iir-compiler --test release_sweep",
        path.display()
    );
}

/// A case either lowers to the same IIR on every edition that accepts it, or
/// is refused: editions never disagree about what an accepted program means.
#[test]
fn accepting_editions_lower_a_case_to_the_same_iir() {
    let sweep = run_sweep();
    for (index, case) in sweep.cases.iter().enumerate() {
        let mut digests: Vec<&str> = sweep
            .cells
            .iter()
            .map(|(_, cells)| cells[index].as_str())
            .filter(|cell| *cell != "rejected")
            .collect();
        digests.dedup();
        assert_eq!(digests.len(), 1, "{}: {digests:?}", case.name);
    }
}

/// The boundary set (LANG84 "CI bounding" step 3), pinned so a new class is
/// a reviewed change: phase 6e's `lang_matrix` rows run these editions.
#[test]
fn the_boundary_releases_are_the_class_representatives() {
    let sweep = run_sweep();
    let swept: Vec<&str> = sweep.cells.iter().map(|(r, _)| r.as_str()).collect();
    assert_eq!(swept.first(), Some(&"es2016"));
    assert_eq!(swept.len(), 10);
    let classes = classes(&sweep);
    let representatives: Vec<&str> = classes.iter().map(|(_, r, _)| *r).collect();
    // Two classes: es2016–es2020 refuse numeric separators, es2021+ accept
    // them. Within each class every edition compiles every case to the same
    // IIR, so its first edition represents it.
    assert_eq!(representatives, ["es2016", "es2021"]);
    let sizes: Vec<usize> = classes.iter().map(|(_, _, m)| m.len()).collect();
    assert_eq!(sizes, [5, 5]);
    assert!(classes.iter().any(|(_, _, m)| m.contains(&DEFAULT_RELEASE)));
}
