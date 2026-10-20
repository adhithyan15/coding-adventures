//! LANG84 stage **I(pl-int-print)** for every Perl release that claims it.
//!
//! Stage I means: every corpus case of the subset that applies to the release
//! compiles to IIR that validates and passes C7, and every negative case is
//! rejected with the typed error its `.reject` file names, naming the
//! release. `code/grammars/perl/releases.csv` may claim only what this file
//! proves, so the claim is read from the CSV and checked here:
//!
//! ```text
//!   releases.csv  iir_status ∋ pl-int-print ──► every release R so marked
//!   profiles.rs   profile_for(R)            ──► the profile P the CSV names
//!   corpus/pl-int-print/                    ──► every case C
//!   for each (R, C): compile_source_at(C, R) and compare with C's outcome
//!                    under P (.expect, or a .reject@P override, or .reject)
//! ```
//!
//! A positive case "applies" to R when R's grammar parses it (LANG84
//! "Corpora and oracles"). The CRLF case does not apply to 5.003_26 through
//! 5.004_71, whose pairs make a carriage return a lex error; that is allowed
//! here and pinned, release by release, in `release_classes.csv` by
//! `release_sweep.rs`, so it cannot widen silently.

mod common;

use std::process::Command;

use common::{corpus, corpus_dir, inventory, lists_subset, Case, Outcome};
use coding_adventures_perl_parser::SUPPORTED_RELEASES;
use interpreter_ir::op_registry::{registry_entry, OpKind};
use perl_iir_compiler::{
    compile_source_at, profile_for, run_source_at, FrontendBudgets, FrontendError, DEFAULT_RELEASE_ID,
    SUBSET,
};

/// Releases whose `iir_status` lists the subset, in inventory order.
fn claiming_releases() -> Vec<String> {
    inventory()
        .into_iter()
        .filter(|row| lists_subset(&row["iir_status"]))
        .map(|row| row["release"].clone())
        .collect()
}

#[test]
fn profiles_match_the_inventory() {
    let mut profiled = 0;
    for row in inventory() {
        let release = &row["release"];
        let profile = profile_for(release);
        assert_eq!(row["profile"], profile.map_or("-", |p| p.name), "{release}: releases.csv profile vs profiles.rs");
        assert_eq!(row["iir_status"] != "-", profile.is_some(), "{release}: a profile exactly when iir_status claims");
        profiled += usize::from(profile.is_some());
    }
    assert_eq!(profiled, SUPPORTED_RELEASES.len(), "every checked release has a profile");
}

/// LANG84 C4.4: the default is an exact row that claims an IIR subset.
#[test]
fn the_default_release_claims_pl_int_print() {
    assert!(claiming_releases().iter().any(|r| r == DEFAULT_RELEASE_ID), "{DEFAULT_RELEASE_ID}");
}

/// The claim covers exactly the releases with a compiled pair: a pending row
/// cannot claim stage I, and no checked pair is left unclaimed.
#[test]
fn every_checked_release_and_only_those_claim_the_subset() {
    assert_eq!(claiming_releases(), SUPPORTED_RELEASES.iter().map(|r| r.to_string()).collect::<Vec<_>>());
    for row in inventory() {
        assert!(
            !lists_subset(&row["backend_status"]) || lists_subset(&row["iir_status"]),
            "{}: B(S) without I(S)",
            row["release"]
        );
    }
}

/// C7: a LANG83 registry builtin, or a pre-registry scalar op the registry
/// does not gate.
fn assert_c7(module: &interpreter_ir::IIRModule, what: &str) {
    for instruction in module.functions.iter().flat_map(|f| &f.instructions) {
        if instruction.op == "call_builtin" {
            let entry = registry_entry(instruction).unwrap_or_else(|| panic!("{what}: {instruction:?}"));
            assert_eq!(entry.kind, OpKind::Builtin, "{what}");
        } else {
            assert!(["const", "neg", "add", "sub", "mul", "ret"].contains(&instruction.op.as_str()), "{what}: {}", instruction.op);
        }
    }
}

fn check_case(release: &str, case: &Case, budgets: &FrontendBudgets) -> &'static str {
    let profile = profile_for(release).expect("claiming releases have a profile");
    let what = format!("{release} ({}) {}", profile.name, case.name);
    let compiled = compile_source_at(&case.source, Some(release), &case.name, budgets);
    match (case.expected_under(profile.name), compiled) {
        (Outcome::Prints(stdout), Ok(module)) => {
            assert!(module.validate().is_empty(), "{what}");
            assert_eq!(module.release.as_deref(), Some(release), "{what}");
            assert_c7(&module, &what);
            let ran = run_source_at(&case.source, Some(release)).unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_eq!(&ran, stdout, "{what}: stdout");
            "ran"
        }
        // The case does not apply: this release's grammar does not parse it.
        (Outcome::Prints(_), Err(error @ (FrontendError::Lex { .. } | FrontendError::Parse { .. }))) => {
            assert!(error.to_string().starts_with(&format!("Perl {release}: ")), "{what}: {error}");
            "not applicable"
        }
        (Outcome::Rejected(category), Err(error)) => {
            assert_eq!(error.category(), category, "{what}: {error}");
            assert!(error.to_string().starts_with(&format!("Perl {release}: ")), "{what}: {error}");
            "rejected"
        }
        (Outcome::Prints(_), Err(error)) => panic!("{what}: refused: {error}"),
        (Outcome::Rejected(category), Ok(_)) => panic!("{what}: compiled, expected {category}"),
    }
}

#[test]
fn pl_int_print_corpus_holds_on_every_claiming_release() {
    let cases = corpus();
    assert!(cases.iter().filter(|c| c.is_positive()).count() >= 12, "positive cases");
    assert!(cases.iter().filter(|c| !c.is_positive()).count() >= 7, "negative cases");
    let budgets = FrontendBudgets::default();
    let releases = claiming_releases();
    let mut tally = std::collections::BTreeMap::new();
    for release in &releases {
        for case in &cases {
            *tally.entry(check_case(release, case, &budgets)).or_insert(0usize) += 1;
        }
    }
    assert_eq!(tally.values().sum::<usize>(), releases.len() * cases.len());
    eprintln!("{SUBSET}: {} releases x {} cases: {tally:?}", releases.len(), cases.len());
}

/// Every override names a profile `profiles.rs` defines, and only a positive
/// case carries one.
#[test]
fn overrides_name_real_profiles() {
    let names: Vec<&str> = perl_iir_compiler::profiles::PROFILES.iter().map(|p| p.name).collect();
    for case in corpus() {
        for profile in case.overrides.keys() {
            assert!(names.contains(&profile.as_str()), "{}: override for unknown profile {profile}", case.name);
            assert!(case.is_positive(), "{}: an override needs the plain .expect", case.name);
        }
    }
}

/// The host interpreter for this record: `perl` on PATH, on the 5.38 line
/// the corpus cites, or `None` (a loud skip).
fn host_perl() -> Option<String> {
    let out = Command::new("perl").args(["-e", "print $^V"]).output().ok()?;
    let version = String::from_utf8_lossy(&out.stdout).into_owned();
    if !out.status.success() || !version.starts_with("v5.38.") {
        eprintln!("SKIP Perl oracle: host perl {version:?} is not on the v5.38 line the corpus cites");
        return None;
    }
    Some(version)
}

/// The oracle: host `perl` must print every `.expect`, and must do what each
/// negative case's `.cite` says it does (some are valid Perl the subset
/// refuses, some are errors in Perl too). The compiled programs never run
/// on it.
#[test]
fn corpus_agrees_with_the_host_perl_oracle() {
    let Some(version) = host_perl() else { return };
    let dir = corpus_dir();
    for case in corpus() {
        assert!(case.cite.contains(&format!("host perl {version} ")), "{}: .cite names {version}", case.name);
        // The file path as one argument: no shell, no interpolation.
        let out = Command::new("perl").arg(dir.join(format!("{}.pl", case.name))).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        match &case.outcome {
            Outcome::Prints(want) => {
                assert!(out.status.success(), "{version}: {} failed", case.name);
                assert_eq!(&stdout, want, "{version}: {}", case.name);
            }
            Outcome::Rejected(_) => {
                let line = case.cite.lines().find_map(|l| l.strip_prefix("perl: ")).expect("a perl: line");
                if line.starts_with("rejects") {
                    assert!(!out.status.success(), "{version} accepts {}", case.name);
                } else {
                    assert!(out.status.success(), "{version} rejects {}", case.name);
                    assert_eq!(line, format!("exit 0, stdout '{stdout}'"), "{}", case.name);
                }
            }
        }
    }
    eprintln!("Perl oracle: the {SUBSET} corpus agrees with host perl {version}");
}
