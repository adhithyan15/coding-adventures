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
//! what lets the `lang_matrix` rows on the class representative stand for
//! the others (LANG84 "CI bounding"; `release_sweep.rs` computes the class
//! and writes `code/grammars/ruby/release_classes.csv`).

mod common;

use std::process::Command;

use common::{claiming_releases, corpus, corpus_dir, inventory, Outcome};
use interpreter_ir::op_registry::registry_name;
use ruby_iir_compiler::profiles::profile_for;
use ruby_iir_compiler::{
    compile_source_at, run_module, supported_releases, FrontendBudgets, FrontendError, DEFAULT_RELEASE,
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
/// to every backend: their IIR is identical for every case. The one field
/// that differs is `IIRModule::release`, which names the release on purpose
/// (LANG84 phase 5b); it is checked and then set aside before comparing.
#[test]
fn claiming_releases_form_one_iir_identity_class() {
    let budgets = FrontendBudgets::default();
    let releases = claiming_releases();
    for case in corpus().iter().filter(|c| matches!(c.outcome, Outcome::Runs { .. })) {
        let dumps: Vec<String> = releases
            .iter()
            .map(|r| {
                let mut module = compile_source_at(&case.source, Some(r), "id", &budgets).unwrap();
                assert_eq!(module.release.as_deref(), Some(r.as_str()), "{}", case.name);
                module.release = None;
                format!("{module:?}")
            })
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
        // Ruby on Windows writes stdout in text mode, so every `puts`
        // ends in "\r\n" there. The `.expect` files record the
        // language's output ("\n"), so fold the platform's line ending
        // back before comparing.
        let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
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
