//! The LANG84 release sweep for Ruby (phase 5b slice 2, "CI bounding"
//! steps 1–2), on the model of Perl's (phase 4c).
//!
//! Three Ruby releases claim `rb-int-puts` at stage I (3.3.5, 3.3.6, 3.3.7)
//! and there are eight executable backends. Running every release on every
//! backend is 24 cells per case; LANG84 bounds the work by **IIR identity**
//! instead: two releases that produce byte-identical IIR for every case are
//! indistinguishable to every backend, so running one of them proves the
//! other.
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
//! Only claiming releases are swept. 0.49 and 0.50 have grammar pairs but no
//! IIR profile, so they refuse every case with `ReleaseNotAvailable`
//! (`release_subsets.rs` proves that) and have no class to be in.
//!
//! `IIRModule::release` is not part of the digest: `interpreter_ir::serialise`
//! does not write it (it is provenance, not code), and every case is compiled
//! under the same module name. Ruby's corpus has no per-release override, so
//! every claiming release must compile every positive case; a refusal is a
//! test failure here (and in stage I), never a quiet `rejected`.
//!
//! The result is `code/grammars/ruby/release_classes.csv`
//! (`subset,case,release,class`, one row per positive case and claiming
//! release). This test regenerates it and fails if the committed file
//! differs, so a release cannot change class without the CSV changing in the
//! same PR. To accept a deliberate change (the run that writes the file
//! fails on purpose, and refuses to run under `CI`/`GITHUB_ACTIONS`, so the
//! switch can never turn the check into a pass):
//!
//! ```text
//!   RUBY_RELEASE_SWEEP_WRITE=1 cargo test -p ruby-iir-compiler --test release_sweep
//! ```

mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;

use coding_adventures_sha256::sha256_hex;
use common::{claiming_releases, corpus, ruby_grammars, Case};
use interpreter_ir::serialise::serialise;
use ruby_iir_compiler::{compile_source_at, FrontendBudgets, DEFAULT_RELEASE, SUBSET};

/// What one release does with one positive case: the SHA-256 of its
/// serialised IIR. (Perl's sweep also has a `rejected` cell; Ruby has no
/// case a claiming release refuses, see the module docs, so its CSV never
/// holds one.)
type Cell = String;

/// Compile one positive case under one release.
fn sweep_cell(case: &Case, release: &str, budgets: &FrontendBudgets) -> Cell {
    let module = compile_source_at(&case.source, Some(release), "sweep", budgets)
        .unwrap_or_else(|error| panic!("{release} {}: a claiming release refused a positive case: {error}", case.name));
    assert_eq!(module.release.as_deref(), Some(release));
    sha256_hex(&serialise(&module))
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
    for (case, digest) in cases.iter().zip(cells) {
        writeln!(outcome, "{SUBSET}/{}={digest}", case.name).unwrap();
    }
    sha256_hex(outcome.as_bytes())[..16].to_string()
}

/// `release_classes.csv`, exactly as committed.
fn render(sweep: &Sweep) -> String {
    let classes: Vec<String> = sweep.cells.iter().map(|(_, cells)| class_of(&sweep.cases, cells)).collect();
    let mut csv = String::from("subset,case,release,class\n");
    for case in &sweep.cases {
        for ((release, _), class) in sweep.cells.iter().zip(&classes) {
            writeln!(csv, "{SUBSET},{},{release},{class}", case.name).unwrap();
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
    assert_eq!(swept, ["3.3.5", "3.3.6", "3.3.7"]);
    assert_eq!(sweep.cases.len(), 14, "every positive rb-int-puts case");
}

/// Steps 1 and 2, compared with the committed `release_classes.csv`.
#[test]
fn release_classes_csv_matches_the_sweep() {
    let sweep = run_sweep();
    let rendered = render(&sweep);
    let path = ruby_grammars().join("release_classes.csv");
    if std::env::var_os("RUBY_RELEASE_SWEEP_WRITE").is_some() {
        // Writing and then comparing would always pass, so a write never
        // counts as a pass, and CI never writes.
        assert!(
            std::env::var_os("CI").is_none() && std::env::var_os("GITHUB_ACTIONS").is_none(),
            "RUBY_RELEASE_SWEEP_WRITE is for a developer's checkout, never for CI"
        );
        std::fs::write(&path, &rendered).unwrap();
        panic!("wrote {}; review the diff, then re-run without RUBY_RELEASE_SWEEP_WRITE", path.display());
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == rendered,
        "{} is stale: a release changed IIR-identity class. Review the change, then run \
         RUBY_RELEASE_SWEEP_WRITE=1 cargo test -p ruby-iir-compiler --test release_sweep",
        path.display()
    );
    for (class, representative, members) in classes(&sweep) {
        eprintln!("class {class}: representative {representative}, {} release(s)", members.len());
    }
}

/// Every claiming release lowers each case to the same IIR digest (the
/// serialised counterpart of `release_subsets.rs`'s Debug comparison).
#[test]
fn every_claiming_release_lowers_a_case_to_the_same_iir() {
    let sweep = run_sweep();
    for (index, case) in sweep.cases.iter().enumerate() {
        let digests: BTreeMap<&str, Vec<&str>> = sweep.cells.iter().fold(BTreeMap::new(), |mut by, (release, cells)| {
            by.entry(cells[index].as_str()).or_default().push(release);
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
    // One class: every claiming release shares the ruby33 profile and the
    // three pairs lex and parse the corpus alike. Its representative is the
    // first claiming row in releases.csv, 3.3.5, not the default 3.3.6.
    assert_eq!(representatives, ["3.3.5"]);
    let sizes: Vec<usize> = classes.iter().map(|(_, _, m)| m.len()).collect();
    assert_eq!(sizes, [3]);
    // The default and the newest claiming release are in a class, so a
    // representative row covers them.
    let newest = sweep.cells.last().map(|(r, _)| r.as_str()).unwrap();
    for release in [DEFAULT_RELEASE, newest] {
        assert!(classes.iter().any(|(_, _, m)| m.contains(&release)), "{release}");
    }
    eprintln!("Ruby release sweep: {} class(es) {representatives:?}, sizes {sizes:?}", classes.len());
}
