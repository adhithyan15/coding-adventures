//! The LANG84 release sweep for Perl (phase 4c, "CI bounding" steps 1–2).
//!
//! There are 118 checked Perl releases and eight executable backends. Running
//! every release on every backend in every PR would be ~1,000 cells per case,
//! so LANG84 bounds the work by **IIR identity**: two releases that produce
//! byte-identical IIR for every case are indistinguishable to every backend,
//! so running one of them proves the other.
//!
//! ```text
//!   step 1  compile every case under every checked release   (in-process, cheap)
//!   step 2  outcome(R) = [ for each case C: digest(IIR of C under R) | rejected ]
//!           class(R)   = SHA-256(outcome(R)), first 16 hex digits
//!   step 3  representative(class) = its first release in inventory order;
//!           lang_matrix runs every case a representative accepts on every
//!           backend, and `release_inventory.py check --matrix` verifies it
//! ```
//!
//! `IIRModule::release` is not part of the digest: `interpreter_ir::serialise`
//! does not write it (it is provenance, not code), and every case is compiled
//! under the same module name. A case R does not accept (its grammar does not
//! parse it, or a `.reject@<profile>` override puts it outside R's profile) is
//! `rejected` in the outcome; anything else that fails is a test failure.
//!
//! The result is `code/grammars/perl/release_classes.csv`
//! (`subset,case,release,class`, one row per positive case and checked
//! release). This test regenerates it and fails if the committed file
//! differs, so a release cannot change class without the CSV changing in the
//! same PR. To accept a deliberate change (the run that writes the file
//! fails on purpose, and refuses to run under `CI`/`GITHUB_ACTIONS`, so the
//! switch can never turn the check into a pass):
//!
//! ```text
//!   PERL_RELEASE_SWEEP_WRITE=1 cargo test -p perl-iir-compiler --test release_sweep
//! ```

mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;

use coding_adventures_perl_parser::SUPPORTED_RELEASES;
use coding_adventures_sha256::sha256_hex;
use common::{corpus, inventory, perl_grammars, Case, Outcome};
use interpreter_ir::serialise::serialise;
use perl_iir_compiler::{compile_source_at, profile_for, FrontendBudgets, FrontendError, DEFAULT_RELEASE_ID, SUBSET};

/// What one release does with one positive case.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Cell {
    /// SHA-256 of the serialised IIR.
    Iir(String),
    /// The release does not accept the case.
    Rejected,
}

/// Compile one positive case under one release, insisting that any refusal
/// is one the corpus allows.
fn sweep_cell(case: &Case, release: &str, budgets: &FrontendBudgets) -> Cell {
    let profile = profile_for(release).unwrap_or_else(|| panic!("{release}: no profile"));
    let expected = case.expected_under(profile.name);
    match compile_source_at(&case.source, Some(release), "sweep", budgets) {
        Ok(module) => {
            assert!(matches!(expected, Outcome::Prints(_)), "{release} {}: compiled, expected {expected:?}", case.name);
            assert_eq!(module.release.as_deref(), Some(release));
            Cell::Iir(sha256_hex(&serialise(&module)))
        }
        // The grammar does not parse it: the case does not apply.
        Err(FrontendError::Lex { .. } | FrontendError::Parse { .. }) if matches!(expected, Outcome::Prints(_)) => {
            Cell::Rejected
        }
        Err(error) => {
            assert_eq!(
                expected,
                &Outcome::Rejected(error.category().to_string()),
                "{release} {}: {error}",
                case.name
            );
            Cell::Rejected
        }
    }
}

/// The sweep's result: per release (inventory order), its cells by case.
struct Sweep {
    cases: Vec<Case>,
    cells: Vec<(&'static str, Vec<Cell>)>,
}

fn run_sweep() -> Sweep {
    let budgets = FrontendBudgets::default();
    let cases: Vec<Case> = corpus().into_iter().filter(Case::is_positive).collect();
    let cells = SUPPORTED_RELEASES
        .iter()
        .map(|release| (*release, cases.iter().map(|case| sweep_cell(case, release, &budgets)).collect()))
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
fn classes(sweep: &Sweep) -> Vec<(String, &'static str, Vec<&'static str>)> {
    let mut found: Vec<(String, &'static str, Vec<&'static str>)> = Vec::new();
    for (release, cells) in &sweep.cells {
        let class = class_of(&sweep.cases, cells);
        match found.iter_mut().find(|(c, _, _)| *c == class) {
            Some((_, _, members)) => members.push(*release),
            None => found.push((class, *release, vec![*release])),
        }
    }
    found
}

/// The sweep covers exactly the checked releases, in inventory order.
#[test]
fn the_sweep_covers_every_checked_release_in_inventory_order() {
    let checked: Vec<String> = inventory()
        .into_iter()
        .filter(|row| row["grammar_status"] != "pending")
        .map(|row| row["release"].clone())
        .collect();
    assert_eq!(checked, SUPPORTED_RELEASES);
    assert_eq!(checked.len(), 118);
}

/// Steps 1 and 2, compared with the committed `release_classes.csv`.
#[test]
fn release_classes_csv_matches_the_sweep() {
    let sweep = run_sweep();
    let rendered = render(&sweep);
    let path = perl_grammars().join("release_classes.csv");
    if std::env::var_os("PERL_RELEASE_SWEEP_WRITE").is_some() {
        // Writing and then comparing would always pass, so a write never
        // counts as a pass, and CI never writes.
        assert!(
            std::env::var_os("CI").is_none() && std::env::var_os("GITHUB_ACTIONS").is_none(),
            "PERL_RELEASE_SWEEP_WRITE is for a developer's checkout, never for CI"
        );
        std::fs::write(&path, &rendered).unwrap();
        panic!("wrote {}; review the diff, then re-run without PERL_RELEASE_SWEEP_WRITE", path.display());
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == rendered,
        "{} is stale: a release changed IIR-identity class. Review the change, then run \
         PERL_RELEASE_SWEEP_WRITE=1 cargo test -p perl-iir-compiler --test release_sweep",
        path.display()
    );
    for (class, representative, members) in classes(&sweep) {
        eprintln!("class {class}: representative {representative}, {} release(s)", members.len());
    }
}

/// Whatever separates the classes, it is never the code: every release that
/// accepts a case lowers it to the same IIR. Releases differ only in which
/// cases they accept (the grammar's CR rule, the profile's negative zero).
#[test]
fn every_accepting_release_lowers_a_case_to_the_same_iir() {
    let sweep = run_sweep();
    for (index, case) in sweep.cases.iter().enumerate() {
        let digests: BTreeMap<&str, Vec<&str>> =
            sweep.cells.iter().fold(BTreeMap::new(), |mut by, (release, cells)| {
                if let Cell::Iir(digest) = &cells[index] {
                    by.entry(digest.as_str()).or_default().push(release);
                }
                by
            });
        assert_eq!(digests.len(), 1, "{}: {digests:?}", case.name);
    }
}

/// The boundary set (LANG84 "CI bounding" step 3), pinned so that a new class
/// is a reviewed change: these are the releases `lang_matrix` must run.
#[test]
fn the_boundary_releases_are_the_class_representatives() {
    let sweep = run_sweep();
    let classes = classes(&sweep);
    let representatives: Vec<&str> = classes.iter().map(|(_, r, _)| *r).collect();
    // 1.000: perl_nv, CR is whitespace. 5.003_26: perl_nv, CR is a lex error
    // (through 5.004_71). 5.38.0: perl_iv, the default.
    assert_eq!(representatives, ["1.000", "5.003_26", "5.38.0"]);
    let sizes: Vec<usize> = classes.iter().map(|(_, _, m)| m.len()).collect();
    assert_eq!(sizes.iter().sum::<usize>(), SUPPORTED_RELEASES.len());
    // The default and the newest release are in a class, so a representative
    // row covers them (the default is itself a representative).
    let newest = SUPPORTED_RELEASES.last().unwrap();
    for release in [DEFAULT_RELEASE_ID, newest] {
        assert!(classes.iter().any(|(_, _, m)| m.contains(&release)), "{release}");
    }
    eprintln!("Perl release sweep: {} classes {representatives:?}, sizes {sizes:?}", classes.len());
}
