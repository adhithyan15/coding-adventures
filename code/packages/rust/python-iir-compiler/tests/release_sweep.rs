//! The LANG84 release sweep for Python (phase 7b slice 2, "CI bounding"
//! steps 1–2), on the model of Perl's (phase 4c).
//!
//! Five Python releases claim `py-int-print` at stage I (3.0a1, 3.12.3,
//! 3.13.16, 3.11.17 and 3.16.0a1, in `releases.csv` order, which is date
//! order) and there are eight executable backends. LANG84 bounds the matrix
//! by **IIR identity**: two releases that produce byte-identical IIR for
//! every case are indistinguishable to every backend, so running one of them
//! proves the other.
//!
//! ```text
//!   step 1  compile every positive case under every claiming release
//!   step 2  outcome(R) = [ for each case C: digest(IIR of C under R) | rejected ]
//!           class(R)   = SHA-256(outcome(R)), first 16 hex digits
//!   step 3  representative(class) = its first release in inventory order;
//!           lang_matrix runs every case a representative accepts on every
//!           backend, and `release_inventory.py check --matrix` verifies it
//! ```
//!
//! A cell is `rejected` only when the corpus says so: a `.reject@<release>`
//! override (`int_underscores.reject@3.0a1`: PEP 515's `_` separators arrived
//! in 3.6, so 3.0a1's pair lexes `1_000_000` as an error) whose category the
//! compile error matches. Any other refusal of a positive case by a claiming
//! release is a test failure, never a quiet `rejected`, exactly as in stage I.
//!
//! `IIRModule::release` is not part of the digest: `interpreter_ir::serialise`
//! does not write it (it is provenance, not code), and every case is compiled
//! under the same module name.
//!
//! The result is `code/grammars/python/release_classes.csv`
//! (`subset,case,release,class`, one row per positive case and claiming
//! release). This test regenerates it and fails if the committed file
//! differs, so a release cannot change class without the CSV changing in the
//! same PR. To accept a deliberate change (the run that writes the file
//! fails on purpose, and refuses to run under `CI`/`GITHUB_ACTIONS`, so the
//! switch can never turn the check into a pass):
//!
//! ```text
//!   PYTHON_RELEASE_SWEEP_WRITE=1 cargo test -p python-iir-compiler --test release_sweep
//! ```

mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;

use coding_adventures_sha256::sha256_hex;
use common::{claiming_releases, corpus, python_grammars, Case, Outcome};
use interpreter_ir::serialise::serialise;
use python_iir_compiler::{compile_source_at, FrontendBudgets, DEFAULT_RELEASE, SUBSET};

/// What one release does with one positive case.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Cell {
    /// SHA-256 of the serialised IIR.
    Iir(String),
    /// The release does not accept the case, as a `.reject@<release>`
    /// override records.
    Rejected,
}

/// Compile one positive case under one release, insisting that a refusal is
/// exactly the one the corpus records for that release.
fn sweep_cell(case: &Case, release: &str, budgets: &FrontendBudgets) -> Cell {
    let compiled = compile_source_at(&case.source, Some(release), "sweep", budgets);
    match (case.outcome_on(release), compiled) {
        (Outcome::Runs { .. }, Ok(module)) => {
            assert_eq!(module.release.as_deref(), Some(release));
            Cell::Iir(sha256_hex(&serialise(&module)))
        }
        (Outcome::Rejected(category), Err(error)) => {
            assert_eq!(error.category(), category, "{release} {}: {error}", case.name);
            Cell::Rejected
        }
        (Outcome::Runs { .. }, Err(error)) => panic!("{release} {}: refused without an override: {error}", case.name),
        (Outcome::Rejected(category), Ok(_)) => panic!("{release} {}: compiled, expected {category}", case.name),
    }
}

/// The sweep's result: per claiming release (inventory order), its cells by
/// case.
struct Sweep {
    cases: Vec<Case>,
    cells: Vec<(String, Vec<Cell>)>,
}

fn run_sweep() -> Sweep {
    let budgets = FrontendBudgets::default();
    let cases: Vec<Case> = corpus().into_iter().filter(Case::is_positive).collect();
    let cells = claiming_releases()
        .into_iter()
        .map(|release| {
            let cells = cases.iter().map(|case| sweep_cell(case, &release, &budgets)).collect();
            (release, cells)
        })
        .collect();
    Sweep { cases, cells }
}

/// The class of one release: a digest of its whole outcome vector.
fn class_of(cases: &[Case], cells: &[Cell]) -> String {
    let mut outcome = String::new();
    for (case, cell) in cases.iter().zip(cells) {
        let shown = match cell {
            Cell::Iir(digest) => digest.as_str(),
            Cell::Rejected => "rejected",
        };
        writeln!(outcome, "{SUBSET}/{}={shown}", case.name).unwrap();
    }
    sha256_hex(outcome.as_bytes())[..16].to_string()
}

/// `release_classes.csv`, exactly as committed.
fn render(sweep: &Sweep) -> String {
    let classes: Vec<String> = sweep.cells.iter().map(|(_, cells)| class_of(&sweep.cases, cells)).collect();
    let mut csv = String::from("subset,case,release,class\n");
    for (index, case) in sweep.cases.iter().enumerate() {
        for ((release, cells), class) in sweep.cells.iter().zip(&classes) {
            let shown = match cells[index] {
                Cell::Iir(_) => class.as_str(),
                Cell::Rejected => "rejected",
            };
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

/// The sweep covers exactly the claiming releases, in inventory order.
#[test]
fn the_sweep_covers_every_claiming_release_in_inventory_order() {
    let sweep = run_sweep();
    let swept: Vec<&str> = sweep.cells.iter().map(|(r, _)| r.as_str()).collect();
    assert_eq!(swept, ["3.0a1", "3.12.3", "3.13.16", "3.11.17", "3.16.0a1"]);
    assert_eq!(sweep.cases.len(), 18, "every positive py-int-print case");
}

/// Steps 1 and 2, compared with the committed `release_classes.csv`.
#[test]
fn release_classes_csv_matches_the_sweep() {
    let sweep = run_sweep();
    let rendered = render(&sweep);
    let path = python_grammars().join("release_classes.csv");
    if std::env::var_os("PYTHON_RELEASE_SWEEP_WRITE").is_some() {
        // Writing and then comparing would always pass, so a write never
        // counts as a pass, and CI never writes.
        assert!(
            std::env::var_os("CI").is_none() && std::env::var_os("GITHUB_ACTIONS").is_none(),
            "PYTHON_RELEASE_SWEEP_WRITE is for a developer's checkout, never for CI"
        );
        std::fs::write(&path, &rendered).unwrap();
        panic!("wrote {}; review the diff, then re-run without PYTHON_RELEASE_SWEEP_WRITE", path.display());
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == rendered,
        "{} is stale: a release changed IIR-identity class. Review the change, then run \
         PYTHON_RELEASE_SWEEP_WRITE=1 cargo test -p python-iir-compiler --test release_sweep",
        path.display()
    );
    for (class, representative, members) in classes(&sweep) {
        eprintln!("class {class}: representative {representative}, {} release(s)", members.len());
    }
}

/// Whatever separates the classes, it is never the code: every release that
/// accepts a case lowers it to the same IIR. Releases differ only in which
/// cases they accept.
#[test]
fn every_accepting_release_lowers_a_case_to_the_same_iir() {
    let sweep = run_sweep();
    for (index, case) in sweep.cases.iter().enumerate() {
        let digests: BTreeMap<&str, Vec<&str>> = sweep.cells.iter().fold(BTreeMap::new(), |mut by, (release, cells)| {
            if let Cell::Iir(digest) = &cells[index] {
                by.entry(digest.as_str()).or_default().push(release);
            }
            by
        });
        assert_eq!(digests.len(), 1, "{}: {digests:?}", case.name);
    }
}

/// The boundary set (LANG84 "CI bounding" step 3), pinned so that a new class
/// is a reviewed change: these are the releases `lang_matrix` must run, on
/// top of the default release's own rows.
#[test]
fn the_boundary_releases_are_the_class_representatives() {
    let sweep = run_sweep();
    let classes = classes(&sweep);
    let representatives: Vec<&str> = classes.iter().map(|(_, r, _)| *r).collect();
    // 3.0a1: refuses `int_underscores` (no PEP 515 separators). 3.12.3: the
    // first of the four releases that accept the whole corpus; the default
    // 3.13.16 is in its class but comes after it in releases.csv.
    assert_eq!(representatives, ["3.0a1", "3.12.3"]);
    let members: Vec<&[&str]> = classes.iter().map(|(_, _, m)| m.as_slice()).collect();
    assert_eq!(members, [&["3.0a1"][..], &["3.12.3", "3.13.16", "3.11.17", "3.16.0a1"][..]]);
    // The default and the newest claiming release are in a class, so a
    // representative row covers them.
    let newest = sweep.cells.last().map(|(r, _)| r.as_str()).unwrap();
    for release in [DEFAULT_RELEASE, newest] {
        assert!(classes.iter().any(|(_, _, m)| m.contains(&release)), "{release}");
    }
    eprintln!("Python release sweep: {} classes {representatives:?}", classes.len());
}
