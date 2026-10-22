//! LANG84 stage **I(py-int-print)** for every Python release that claims it.
//!
//! Stage I means: every corpus case of the subset that applies to the
//! release compiles to IIR that validates and passes C7, and every negative
//! case is rejected with the typed error its `.reject` file names, naming the
//! release. The CSV (`code/grammars/python/releases.csv`) may claim only what
//! this file proves, so the claim is read from the CSV and checked here:
//!
//! ```text
//!   releases.csv  iir_status ∋ py-int-print ──► every release R so marked
//!   subsets.csv   py-int-print → corpus dir ──► every case C in it
//!   for each (R, C): compile_source_at(C, R) ──► validate, C7, run on vm-core
//!                    and compare with C.expect (stdout, then any trap)
//!   C.reject@R overrides C's outcome for release R only.
//! ```
//!
//! On top of stage I it pins the IIR **identity classes**: releases that
//! share a profile emit byte-identical IIR for every case they all accept,
//! which is what lets the `lang_matrix` rows on each class representative
//! stand for the others (LANG84 "CI bounding"; `release_sweep.rs` computes
//! the classes and writes `code/grammars/python/release_classes.csv`).

mod common;

use std::process::Command;

use common::{claiming_releases, corpus, corpus_dir, inventory, Outcome};
use interpreter_ir::op_registry::registry_name;
use python_iir_compiler::profiles::profile_for;
use python_iir_compiler::{
    compile_source_at, run_module, supported_releases, FrontendBudgets, FrontendError, DEFAULT_RELEASE, SUBSET,
};

#[test]
fn profiles_match_the_inventory() {
    let mut profiled = 0;
    for row in inventory() {
        let release = &row["release"];
        let expected = profile_for(release).map_or("-", |p| p.name);
        assert_eq!(row["profile"], expected, "{release}: releases.csv profile vs profiles.rs");
        let claims = row["iir_status"] != "-";
        assert_eq!(claims, profile_for(release).is_some(), "{release}: a profile exactly when iir_status claims");
        profiled += usize::from(claims);
    }
    assert_eq!(profiled, 5, "3.0a1, 3.11.17, 3.12.3, 3.13.16 and 3.16.0a1");
}

/// LANG84 C4.4: the default is an exact row that claims an IIR subset, and
/// the only row claiming `backend_status` (the matrix compiles it).
#[test]
fn the_default_release_claims_py_int_print_on_the_backends() {
    assert!(claiming_releases().iter().any(|r| r == DEFAULT_RELEASE), "{DEFAULT_RELEASE}");
    let backend: Vec<String> = inventory()
        .into_iter()
        .filter(|row| row["backend_status"].split(';').any(|s| s == SUBSET))
        .map(|row| row["release"].clone())
        .collect();
    assert_eq!(backend, [DEFAULT_RELEASE]);
}

#[test]
fn py_int_print_corpus_holds_on_every_claiming_release() {
    let cases = corpus();
    assert!(cases.len() >= 30, "{}", cases.len());
    let releases = claiming_releases();
    assert_eq!(releases, ["3.0a1", "3.12.3", "3.13.16", "3.11.17", "3.16.0a1"]);
    let budgets = FrontendBudgets::default();
    let (mut checked, mut trapped) = (0, 0);
    for release in &releases {
        for case in &cases {
            let compiled = compile_source_at(&case.source, Some(release), &case.name, &budgets);
            match (case.outcome_on(release), compiled) {
                (Outcome::Runs { stdout, trap }, Ok(module)) => {
                    assert!(module.validate().is_empty(), "{release} {}", case.name);
                    // C7: registry entries plus the ungated `const`/`ret`.
                    for instr in &module.functions[0].instructions {
                        assert!(
                            registry_name(instr).is_some() || ["const", "ret"].contains(&instr.op.as_str()),
                            "{release} {}: {instr:?} is neither a registry op nor ungated IIR",
                            case.name
                        );
                    }
                    let ran = run_module(module).unwrap_or_else(|e| panic!("{release} {}: {e}", case.name));
                    assert_eq!(ran.stdout, stdout, "{release} {}: stdout", case.name);
                    assert_eq!(ran.trap, trap, "{release} {}: trap", case.name);
                    trapped += usize::from(ran.trap.is_some());
                }
                (Outcome::Rejected(category), Err(error)) => {
                    assert_eq!(error.category(), category, "{release} {}: {error}", case.name);
                    assert_eq!(error.release(), Some(release.as_str()), "{}", case.name);
                    assert!(error.to_string().contains(release.as_str()), "{}: {error}", case.name);
                }
                (Outcome::Runs { .. }, Err(error)) => panic!("{release} {}: refused: {error}", case.name),
                (Outcome::Rejected(category), Ok(_)) => {
                    panic!("{release} {}: compiled, expected {category}", case.name)
                }
            }
            checked += 1;
        }
    }
    assert_eq!(checked, releases.len() * cases.len());
    assert!(trapped >= 4 * 9, "{trapped}");
}

/// Releases with one profile are indistinguishable to every backend: their
/// IIR is identical for every case they all accept.
#[test]
fn claiming_releases_form_one_iir_identity_class() {
    let budgets = FrontendBudgets::default();
    let releases = claiming_releases();
    let mut compared = 0;
    for case in corpus().iter().filter(|c| matches!(c.outcome, Outcome::Runs { .. })) {
        let dumps: Vec<String> = releases
            .iter()
            .filter(|r| !case.overrides.contains_key(r.as_str()))
            .map(|r| {
                // `release` names the release on purpose (LANG84 phase 7b);
                // check it, then set it aside before comparing.
                let mut module = compile_source_at(&case.source, Some(r), "id", &budgets).unwrap();
                assert_eq!(module.release.as_deref(), Some(r.as_str()), "{}", case.name);
                module.release = None;
                format!("{module:?}")
            })
            .collect();
        assert!(dumps.len() >= 4, "{}", case.name);
        assert!(dumps.windows(2).all(|w| w[0] == w[1]), "{}: IIR differs between {releases:?}", case.name);
        compared += 1;
    }
    assert!(compared >= 18);
}

/// Every phase-7a release has a profile, so a paired release can never be
/// refused for want of one; an unpaired row and a minor line are typed
/// errors naming the request.
#[test]
fn unpaired_rows_and_minor_lines_are_typed_errors() {
    assert!(supported_releases().iter().all(|r| profile_for(r).is_some()));
    let budgets = FrontendBudgets::default();
    for (release, category) in [("3.12.2", "ReleaseNotAvailable"), ("3.0", "ReleaseNotAvailable"), ("3.12", "UnknownRelease"), ("", "UnknownRelease")] {
        let error = compile_source_at("print(1)\n", Some(release), "t", &budgets).unwrap_err();
        assert_eq!(error.category(), category, "{release:?}: {error}");
    }
    assert!(matches!(
        compile_source_at("print(1)\n", Some("3.12.2"), "t", &budgets),
        Err(FrontendError::ReleaseNotAvailable { release, .. }) if release == "3.12.2"
    ));
}

/// The host CPython that a release names, with its exact version.
fn host_for(release: &str) -> Option<&'static str> {
    match release {
        "3.11.17" => Some("python3.11"),
        "3.12.3" => Some("python3.12"),
        "3.13.16" => Some("python3"),
        _ => None,
    }
}

/// The oracle: each host CPython must reproduce every `.expect` (and the
/// `ZeroDivisionError` of a `Trap.DivZero` case); for a `Trap.Overflow` case
/// it must print the expected prefix and then an integer outside `i64`
/// (this subset's stated divergence). Every `.reject` must match its cite's
/// `host-python:` line: CPython either accepts the program (a subset limit)
/// or rejects it too. The compiled programs never run on CPython. A host
/// that is absent or on another version is skipped, loudly.
#[test]
fn corpus_agrees_with_the_host_cpython_oracles() {
    let dir = corpus_dir();
    let mut agreed = 0;
    for release in ["3.11.17", "3.12.3", "3.13.16"] {
        let host = host_for(release).unwrap();
        let version = match Command::new(host).args(["-I", "-c", "import sys; print(sys.version.split()[0])"]).output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            _ => {
                eprintln!("SKIP {release} oracle: no `{host}` on PATH");
                continue;
            }
        };
        if version != release {
            eprintln!("SKIP {release} oracle: `{host}` is {version}");
            continue;
        }
        for case in corpus() {
            // The file path as one argument: no shell, no interpolation.
            let out = Command::new(host).arg("-I").arg(dir.join(format!("{}.py", case.name))).output().unwrap();
            // Windows Python prints in text mode ("\r\n"); the `.expect` files
            // record the language's output, so fold the line ending back.
            let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
            let stderr = String::from_utf8_lossy(&out.stderr);
            match case.outcome_on(release) {
                Outcome::Runs { stdout: want, trap } => match trap.as_deref() {
                    None => {
                        assert!(out.status.success(), "{version}: {} failed: {stderr}", case.name);
                        assert_eq!(stdout, want, "{version}: {}", case.name);
                    }
                    Some("Trap.DivZero") => {
                        assert!(!out.status.success() && stderr.contains("ZeroDivisionError"), "{}", case.name);
                        assert_eq!(stdout, want, "{version}: {}", case.name);
                    }
                    Some("Trap.Overflow") => {
                        assert!(out.status.success(), "{version}: {} failed: {stderr}", case.name);
                        let rest = stdout.strip_prefix(want.as_str()).unwrap_or_else(|| {
                            panic!("{version}: {} printed {stdout:?}, not {want:?} first", case.name)
                        });
                        let next = rest.lines().next().unwrap_or_default();
                        let value: i128 = next.parse().unwrap_or_else(|_| panic!("{}: {next:?}", case.name));
                        assert!(i64::try_from(value).is_err(), "{version}: {} stays in i64: {value}", case.name);
                        assert!(case.cite.contains(next), "{}.cite must record CPython's {next}", case.name);
                    }
                    Some(other) => panic!("{}: no CPython oracle mapping for {other}", case.name),
                },
                Outcome::Rejected(_) => {
                    let host_line = case
                        .cite
                        .lines()
                        .find_map(|l| l.strip_prefix("host-python: "))
                        .unwrap_or_else(|| panic!("{}.cite has no host-python line", case.name));
                    if host_line == "accepts" {
                        assert!(out.status.success(), "{version} rejects {}: {stderr}", case.name);
                    } else {
                        assert!(!out.status.success(), "{version} accepts {}", case.name);
                        assert!(stderr.contains(host_line), "{version} {}: {stderr}", case.name);
                    }
                }
            }
        }
        agreed += 1;
        eprintln!("CPython {version} oracle: the corpus agrees");
    }
    eprintln!("{agreed} host oracle(s) ran");
}
