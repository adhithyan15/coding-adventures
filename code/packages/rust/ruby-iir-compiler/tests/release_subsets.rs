//! LANG84 stage **I(rb-int-puts)** for every Ruby release that claims it.
//!
//! Stage I means: every corpus case of the subset that applies to the
//! release compiles to IIR that validates and passes C7, and every negative
//! case is rejected with the typed error its `.reject` file names, naming the
//! release. The CSV (`code/grammars/ruby/releases.csv`) may claim only what
//! this file proves, so the claim is read from the CSV and checked here:
//!
//! ```text
//!   releases.csv  iir_status ∋ rb-int-puts ──► every release R so marked
//!   subsets.csv   rb-int-puts → corpus dir ──► every case C in it
//!   for each (R, C): compile_source_at(C, R) ─► validate, C7, run on vm-core
//!                    and compare with C.expect (stdout, then any trap)
//! ```
//!
//! On top of stage I it pins the IIR **identity class**: the releases that
//! share a grammar class and a profile must emit byte-identical IIR, which is
//! what lets the `lang_matrix` rows on the default release stand for the
//! others (LANG84 "CI bounding").

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use interpreter_ir::op_registry::registry_name;
use ruby_iir_compiler::profiles::profile_for;
use ruby_iir_compiler::{
    compile_source_at, run_module, supported_releases, FrontendBudgets, FrontendError, DEFAULT_RELEASE, SUBSET,
};

fn ruby_grammars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/ruby")
}

/// Rows of a CSV with `#` provenance lines, as maps keyed by header name
/// (lesson: read the header, never assume a column position).
fn csv_rows(path: &Path) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    let header: Vec<String> = lines.next().unwrap().split(',').map(str::to_string).collect();
    lines
        .map(|line| header.iter().cloned().zip(line.split(',').map(str::to_string)).collect())
        .collect()
}

fn inventory() -> Vec<BTreeMap<String, String>> {
    csv_rows(&ruby_grammars().join("releases.csv"))
}

/// Releases whose `iir_status` lists the subset, in inventory order.
fn claiming_releases() -> Vec<String> {
    inventory()
        .into_iter()
        .filter(|row| row["iir_status"].split(';').any(|s| s == SUBSET))
        .map(|row| row["release"].clone())
        .collect()
}

/// One corpus case: `<name>.rb` plus `.expect` (accepted) or `.reject`.
struct Case {
    name: String,
    source: String,
    outcome: Outcome,
}

enum Outcome {
    /// Exact stdout, then the LANG83 trap kind if the program stops on one.
    Runs { stdout: String, trap: Option<String> },
    /// The C6 category the compile must fail with.
    Rejected(String),
}

fn corpus_dir() -> PathBuf {
    let rows = csv_rows(&ruby_grammars().join("subsets.csv"));
    let row = rows.iter().find(|row| row["subset"] == SUBSET).expect("subsets.csv declares rb-int-puts");
    ruby_grammars().join(&row["corpus_dir"])
}

fn corpus() -> Vec<Case> {
    let dir = corpus_dir();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension()? == "rb").then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let read = |ext: &str| std::fs::read_to_string(dir.join(format!("{name}.{ext}"))).ok();
            let source = read("rb").unwrap();
            assert!(read("cite").is_some_and(|c| c.contains("oracle:")), "{name}.cite must record its oracle");
            let outcome = match (read("expect"), read("reject")) {
                (Some(expect), None) => {
                    let (stdout, trap) = match expect.rfind("trap:") {
                        Some(at) => (expect[..at].to_string(), Some(expect[at + 5..].trim().to_string())),
                        None => (expect, None),
                    };
                    Outcome::Runs { stdout, trap }
                }
                (None, Some(reject)) => Outcome::Rejected(reject.trim().to_string()),
                _ => panic!("{name}: exactly one of .expect and .reject"),
            };
            Case { name, source, outcome }
        })
        .collect()
}

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
    assert_eq!(profiled, 3, "3.3.5, 3.3.6 and 3.3.7");
}

/// LANG84 C4.4: the default is an exact row that claims an IIR subset.
#[test]
fn the_default_release_claims_rb_int_puts() {
    assert!(claiming_releases().iter().any(|r| r == DEFAULT_RELEASE), "{DEFAULT_RELEASE}");
}

#[test]
fn rb_int_puts_corpus_holds_on_every_claiming_release() {
    let cases = corpus();
    assert!(cases.len() >= 20, "{}", cases.len());
    let releases = claiming_releases();
    assert_eq!(releases, ["3.3.5", "3.3.6", "3.3.7"]);
    let budgets = FrontendBudgets::default();
    let mut checked = 0;
    for release in &releases {
        for case in &cases {
            let compiled = compile_source_at(&case.source, Some(release), &case.name, &budgets);
            match (&case.outcome, compiled) {
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
                    assert_eq!(&ran.stdout, stdout, "{release} {}: stdout", case.name);
                    assert_eq!(&ran.trap, trap, "{release} {}: trap", case.name);
                }
                (Outcome::Rejected(category), Err(error)) => {
                    assert_eq!(error.category(), category, "{release} {}: {error}", case.name);
                    assert_eq!(error.release(), Some(release.as_str()), "{}", case.name);
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
}

/// Releases that share a grammar class and a profile are indistinguishable
/// to every backend: their IIR is identical for every case.
#[test]
fn claiming_releases_form_one_iir_identity_class() {
    let budgets = FrontendBudgets::default();
    let releases = claiming_releases();
    for case in corpus().iter().filter(|c| matches!(c.outcome, Outcome::Runs { .. })) {
        let dumps: Vec<String> = releases
            .iter()
            .map(|r| format!("{:?}", compile_source_at(&case.source, Some(r), "id", &budgets).unwrap()))
            .collect();
        assert!(dumps.windows(2).all(|w| w[0] == w[1]), "{}: IIR differs between {releases:?}", case.name);
    }
}

/// A release with a grammar pair but no profile parses the corpus (stage P)
/// yet refuses to compile it, with a typed error naming the release.
#[test]
fn paired_releases_without_a_profile_refuse_every_case() {
    let claiming = claiming_releases();
    let unprofiled: Vec<_> = supported_releases().iter().filter(|r| !claiming.iter().any(|c| c == *r)).collect();
    assert_eq!(unprofiled, [&"0.49", &"0.50"]);
    for release in unprofiled {
        for case in corpus() {
            let error = compile_source_at(&case.source, Some(release), "t", &FrontendBudgets::default())
                .expect_err(&case.name);
            assert!(
                matches!(&error, FrontendError::ReleaseNotAvailable { release: r, .. } if r == release),
                "{release} {}: {error:?}",
                case.name
            );
        }
    }
}

/// The oracle: host ruby must produce every `.expect` (and accept every
/// `.reject`, which is refused for a subset reason, not because Ruby
/// rejects it). The compiled programs never run on it. Skips loudly unless
/// the host ruby is on the 3.3 line the expectations came from.
#[test]
fn corpus_expectations_agree_with_the_host_ruby_oracle() {
    let version = match Command::new("ruby").arg("-v").output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => {
            eprintln!("SKIP Ruby oracle: no `ruby` on PATH");
            return;
        }
    };
    if !version.starts_with("ruby 3.3.") {
        eprintln!("SKIP Ruby oracle: {version} is not on the 3.3 line the corpus records");
        return;
    }
    let dir = corpus_dir();
    for case in corpus() {
        // The file path as one argument: no shell, no interpolation.
        let out = Command::new("ruby").arg(dir.join(format!("{}.rb", case.name))).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr);
        match case.outcome {
            Outcome::Runs { stdout: want, trap } => {
                assert_eq!(stdout, want, "{version}: {}", case.name);
                match trap.as_deref() {
                    None => assert!(out.status.success(), "{version}: {} failed: {stderr}", case.name),
                    Some("Trap.DivZero") => {
                        assert!(!out.status.success() && stderr.contains("(ZeroDivisionError)"), "{}", case.name)
                    }
                    Some(other) => panic!("{}: no Ruby oracle mapping for {other}", case.name),
                }
            }
            Outcome::Rejected(_) => assert!(out.status.success(), "{version} rejects {}: {stderr}", case.name),
        }
    }
    eprintln!("Ruby oracle: corpus agrees with {version}");
}
