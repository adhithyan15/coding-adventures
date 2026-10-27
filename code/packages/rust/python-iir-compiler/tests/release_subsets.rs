//! LANG84 stage **I(py-int-print)** and, since phase 7c slices 1 and 3,
//! **I(py2-int-print)** and **I(py-float-print)**, for every Python release
//! that claims them.
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
    SUBSETS,
};

/// Each subset's claiming releases, in `releases.csv` order.
const PY3_RELEASES: [&str; 7] = ["3.0a1", "3.6.0", "3.5.10", "3.12.3", "3.13.16", "3.11.17", "3.16.0a1"];
const PY2_RELEASES: [&str; 4] = ["2.2", "2.4", "2.6", "2.7.18"];
/// `py-float-print` (phase 7c slice 3): the `py3` profile's releases, all of
/// Python 3 but 3.0a1 (whose `str(float)` is `%.12g`).
const PY_FLOAT_RELEASES: [&str; 6] = ["3.6.0", "3.5.10", "3.12.3", "3.13.16", "3.11.17", "3.16.0a1"];

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
        if let Some(profile) = profile_for(release) {
            // A release claims exactly its profile's subsets, at I and at B,
            // and its grammar covers the widest of them.
            let subsets = profile.subsets.join(";");
            assert_eq!(row["iir_status"], subsets, "{release}: iir_status vs profile subsets");
            assert_eq!(row["backend_status"], subsets, "{release}: backend_status vs profile subsets");
            assert_eq!(row["grammar_subset"], profile.subset(), "{release}: grammar_subset");
        }
    }
    assert_eq!(profiled, 11, "2.2, 2.4, 2.6, 2.7.18, 3.0a1, 3.5.10, 3.6.0, 3.11.17, 3.12.3, 3.13.16 and 3.16.0a1");
}

/// LANG84 C4.4: the default is an exact row that claims an IIR subset. It
/// claims `backend_status` too, and since phase 7b slice 2 so does every
/// other claiming release: `lang_matrix` runs the corpus on each IIR-identity
/// class representative, and `release_inventory.py check --matrix` verifies
/// each B claim case by case. B is never claimed without I.
#[test]
fn the_default_release_claims_py_int_print_on_the_backends() {
    assert!(claiming_releases(SUBSET).iter().any(|r| r == DEFAULT_RELEASE), "{DEFAULT_RELEASE}");
    let backend: Vec<String> = inventory()
        .into_iter()
        .filter(|row| row["backend_status"].split(';').any(|s| s == SUBSET))
        .map(|row| row["release"].clone())
        .collect();
    assert!(backend.iter().any(|r| r == DEFAULT_RELEASE), "{DEFAULT_RELEASE}: {backend:?}");
    // For every subset, B is claimed by exactly the releases that claim I.
    for subset in SUBSETS {
        let backend: Vec<String> = inventory()
            .into_iter()
            .filter(|row| row["backend_status"].split(';').any(|s| s == *subset))
            .map(|row| row["release"].clone())
            .collect();
        assert_eq!(backend, claiming_releases(subset), "{subset}");
    }
}

#[test]
fn py_int_print_corpus_holds_on_every_claiming_release() {
    let (checked, trapped) = corpus_holds("py-int-print", &PY3_RELEASES, 30);
    assert!(checked >= 7 * 30 && trapped >= 7 * 9, "{checked} {trapped}");
}

#[test]
fn py_float_print_corpus_holds_on_every_claiming_release() {
    let (checked, trapped) = corpus_holds("py-float-print", &PY_FLOAT_RELEASES, 24);
    assert!(checked >= 6 * 24 && trapped >= 6 * 5, "{checked} {trapped}");
}

#[test]
fn py2_int_print_corpus_holds_on_every_claiming_release() {
    let (checked, trapped) = corpus_holds("py2-int-print", &PY2_RELEASES, 40);
    assert!(checked >= 4 * 40 && trapped >= 4 * 10, "{checked} {trapped}");
}

/// Stage I for one subset: every case on every claiming release, as its
/// `.expect`/`.reject` (and any `.reject@<release>`) says. Returns how many
/// (release, case) pairs were checked and how many trapped.
fn corpus_holds(subset: &str, expected_releases: &[&str], min_cases: usize) -> (usize, usize) {
    let cases = corpus(subset);
    assert!(cases.len() >= min_cases, "{subset}: {}", cases.len());
    let releases = claiming_releases(subset);
    assert_eq!(releases, expected_releases, "{subset}");
    let budgets = FrontendBudgets::default();
    let (mut checked, mut trapped) = (0, 0);
    for release in &releases {
        for case in &cases {
            let compiled = compile_source_at(&case.source, Some(release), &case.name, &budgets);
            match (case.outcome_on(release), compiled) {
                (Outcome::Runs { stdout, trap }, Ok(module)) => {
                    assert!(module.validate().is_empty(), "{release} {}", case.name);
                    // C7: registry entries plus the ungated `const`/`ret`; since
                    // LANG83 PR7c-py also the branches of the module's own
                    // `repr(float)` helpers and the `call`s between them. No op or
                    // builtin names Python (`write_f64_py` is gone).
                    let helpers: Vec<&str> = module.functions.iter().skip(1).map(|f| f.name.as_str()).collect();
                    for instr in module.functions.iter().flat_map(|f| &f.instructions) {
                        let ungated = match instr.op.as_str() {
                            "const" | "ret" | "jmp" | "jmp_if_false" | "label" => true,
                            "call" => instr.srcs.first().and_then(|s| s.as_var()).is_some_and(|f| helpers.contains(&f)),
                            _ => false,
                        };
                        assert!(
                            registry_name(instr).is_some() || ungated,
                            "{release} {}: {instr:?} is neither a registry op nor ungated IIR",
                            case.name
                        );
                        assert_ne!(registry_name(instr), Some("write_f64_py"), "{release} {}", case.name);
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
    (checked, trapped)
}

/// Releases with one profile are indistinguishable to every backend: their
/// IIR is identical for every case they all accept.
#[test]
fn claiming_releases_form_one_iir_identity_class() {
    assert!(identity_classes_hold("py-int-print", 5) >= 18);
    // `pep3127_literals` is accepted by 2.6 and 2.7.18 only.
    assert!(identity_classes_hold("py2-int-print", 2) >= 24);
    // `float_underscores` is refused by 3.5.10 only.
    assert!(identity_classes_hold("py-float-print", 5) >= 13);
}

/// Every positive case of `subset` compiles to the same IIR on every
/// claiming release that accepts it (at least `min_releases` of them).
fn identity_classes_hold(subset: &str, min_releases: usize) -> usize {
    let budgets = FrontendBudgets::default();
    let releases = claiming_releases(subset);
    let mut compared = 0;
    for case in corpus(subset).iter().filter(|c| matches!(c.outcome, Outcome::Runs { .. })) {
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
        assert!(dumps.len() >= min_releases, "{}", case.name);
        assert!(dumps.windows(2).all(|w| w[0] == w[1]), "{}: IIR differs between {releases:?}", case.name);
        compared += 1;
    }
    compared
}

/// The two subsets never borrow each other's syntax: a Python 3 print call
/// with two arguments is a tuple to Python 2 (refused), and a Python 2 print
/// statement is a parse error to Python 3. Same arithmetic, different `/`:
/// classic division floors on a py2 release, true division is refused on a
/// py3 release.
#[test]
fn python_2_and_python_3_disagree_where_the_languages_do() {
    let budgets = FrontendBudgets::default();
    let category = |source: &str, release: &str| {
        compile_source_at(source, Some(release), "t", &budgets).err().map(|e| e.category().to_string())
    };
    assert_eq!(category("print 1\n", "3.13.16").as_deref(), Some("Parse"));
    assert_eq!(category("print(1, 2)\n", "2.7.18").as_deref(), Some("Parse"));
    // True division: a float on the py-float-print releases, refused on
    // 3.0a1 (profile `py30`), floor division on Python 2.
    assert_eq!(category("print(7 / 2)\n", "3.0a1").as_deref(), Some("Unsupported"));
    let run = |source: &str, release: &str| {
        run_module(compile_source_at(source, Some(release), "t", &budgets).unwrap()).unwrap().stdout
    };
    assert_eq!(run("print 7 / 2\n", "2.2"), "3\n");
    assert_eq!(run("print(7 / 2)\n", "3.13.16"), "3.5\n");
    assert_eq!(run("print(-7 // 2)\n", "3.0a1"), "-4\n");
    // `print(-7 / 2)` is the Python 2 *statement* `print (-7 / 2)`.
    assert_eq!(run("print(-7 / 2)\n", "2.7.18"), "-4\n");
    // Python 2's `/` lowers exactly like `//`.
    let iir = |source: &str| {
        let mut module = compile_source_at(source, Some("2.7.18"), "t", &budgets).unwrap();
        module.release = None;
        format!("{module:?}")
    };
    assert_eq!(iir("print 9 / 4\n"), iir("print 9 // 4\n"));
    assert_eq!(iir("print 9 / -1\n"), iir("print 9 // -1\n"));
}

/// Phase 7c slice 2: a leading `0` is octal in Python 2 and an error in
/// Python 3; `0o`/`0b` arrived in 2.6 (PEP 3127); an `L` suffix is a long,
/// which prints without the `L`; and every spelling of one value lowers to
/// the same IIR, because the decoder, not the IIR, knows the spelling.
#[test]
fn python_2_integer_literals_follow_each_release() {
    let budgets = FrontendBudgets::default();
    let category = |source: &str, release: &str| {
        compile_source_at(source, Some(release), "t", &budgets).err().map(|e| e.category().to_string())
    };
    let run = |source: &str, release: &str| {
        run_module(compile_source_at(source, Some(release), "t", &budgets).unwrap()).unwrap().stdout
    };
    for release in PY2_RELEASES {
        assert_eq!(run("print 010\n", release), "8\n", "{release}");
        assert_eq!(run("print 0x1F + 1L\n", release), "32\n", "{release}");
        assert_eq!(run("print 0XFFFFFFFFFFFFFFFl / 16\n", release), "72057594037927935\n", "{release}");
        // i64::MAX in octal is accepted; one more is a long outside i64.
        assert_eq!(run("print 0777777777777777777777L\n", release), "9223372036854775807\n", "{release}");
        assert_eq!(category("print 01000000000000000000000\n", release).as_deref(), Some("Unsupported"));
        assert_eq!(category("print 0x8000000000000000\n", release).as_deref(), Some("Unsupported"));
        assert_eq!(category("print 09\n", release).as_deref(), Some("Lex"), "{release}");
    }
    for (release, has_pep3127) in [("2.2", false), ("2.4", false), ("2.6", true), ("2.7.18", true)] {
        for source in ["print 0o17\n", "print 0b1111\n", "print 0O17L\n"] {
            if has_pep3127 {
                assert_eq!(run(source, release), "15\n", "{release} {source:?}");
            } else {
                assert_eq!(category(source, release).as_deref(), Some("Lex"), "{release} {source:?}");
            }
        }
    }
    // Python 3 never reads a leading zero as octal, and has no `L`.
    for release in PY3_RELEASES {
        assert_eq!(category("print(010)\n", release).as_deref(), Some("Lex"), "{release}");
        assert_eq!(category("print(1L)\n", release).as_deref(), Some("Lex"), "{release}");
    }
    let iir = |source: &str| {
        let mut module = compile_source_at(source, Some("2.7.18"), "t", &budgets).unwrap();
        module.release = None;
        format!("{module:?}")
    };
    for spelling in ["8", "010", "0x8", "0o10", "0b1000", "8L", "0010l", "0X08L"] {
        assert_eq!(iir(&format!("print {spelling}\n")), iir("print 8\n"), "{spelling}");
    }
}

/// Every phase-7a release has a profile, so a paired release can never be
/// refused for want of one; an unpaired row and a minor line are typed
/// errors naming the request.
#[test]
fn unpaired_rows_and_minor_lines_are_typed_errors() {
    assert!(supported_releases().iter().all(|r| profile_for(r).is_some()));
    let budgets = FrontendBudgets::default();
    for (release, category) in [
        ("3.12.2", "ReleaseNotAvailable"),
        ("3.0", "ReleaseNotAvailable"),
        ("2.7.17", "ReleaseNotAvailable"),
        ("2.2a3", "ReleaseNotAvailable"),
        ("3.12", "UnknownRelease"),
        // Release 2.7 is a row without a pair: the legacy `python2.7.*`
        // minor-line pair shares its name and must not be reached.
        ("2.7", "ReleaseNotAvailable"),
        ("2.6.9", "ReleaseNotAvailable"),
        ("2", "UnknownRelease"),
        ("", "UnknownRelease"),
    ] {
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
    for subset in [SUBSET, "py-float-print"] {
        corpus_agrees_with_the_host_cpython_oracles_on(subset);
    }
}

/// The oracle for one Python 3 subset's corpus.
fn corpus_agrees_with_the_host_cpython_oracles_on(subset: &str) {
    let dir = corpus_dir(subset);
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
        for case in corpus(subset) {
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
                        assert!(beyond_i64(next), "{version}: {} printed {next:?}, inside i64", case.name);
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
        eprintln!("CPython {version} oracle: the {subset} corpus agrees");
    }
    eprintln!("{agreed} host oracle(s) ran on {subset}");
}

/// What CPython prints where the subset traps `Trap.Overflow`: an integer
/// outside `i64` (`py-int-print`), or the float of one (`py-float-print`,
/// where the overflowing int meets a float), at least 2^63 in magnitude.
fn beyond_i64(printed: &str) -> bool {
    match printed.parse::<i128>() {
        Ok(value) => i64::try_from(value).is_err(),
        Err(_) => printed.parse::<f64>().is_ok_and(|value| value.abs() >= 9_223_372_036_854_775_808.0),
    }
}

/// 3.0a1's profile (`py30`) has no floats: its `str(float)` is `%.12g`
/// (`PREC_STR`), which no LANG83 writer produces. So true division and a
/// float are refused there, whether its own lexer saw the float or a tree
/// from a 3.13.16 parse is handed to it.
#[test]
fn release_3_0a1_refuses_floats_and_true_division() {
    let budgets = FrontendBudgets::default();
    let category = |source: &str, release: &str| {
        compile_source_at(source, Some(release), "t", &budgets).err().map(|e| e.category().to_string())
    };
    assert_eq!(category("print(7 / 2)\n", "3.0a1").as_deref(), Some("Unsupported"));
    // Its pair never lexed a float: `1.5` is INT DOT INT, a parse error.
    assert_eq!(category("print(1.5)\n", "3.0a1").as_deref(), Some("Parse"));
    assert_eq!(category("print(1e3)\n", "3.0a1").as_deref(), Some("Lex"));
    // A FLOAT token from another release's grammar is refused by the profile.
    let tree = coding_adventures_python_parser::parse_python_at("print(1.5)\n", Some("3.13.16"), &budgets)
        .unwrap()
        .ast;
    let error = python_iir_compiler::compile_cst(&tree, Some("3.0a1"), "t").unwrap_err();
    assert_eq!(error.category(), "Unsupported", "{error}");
    assert!(error.to_string().contains("floats are not in py-int-print"), "{error}");
    for release in PY_FLOAT_RELEASES {
        assert!(category("print(7 / 2)\n", release).is_none(), "{release}");
    }
}

/// `py2-int-print` has no host `python2`, so its `.expect` files rest on
/// the tags' own source (each `.cite` says so). As an independent check of
/// the arithmetic, every positive case is translated to Python 3 and run on
/// the host `python3`: `print X` becomes `print(X)`, a bare `print` becomes
/// `print()`, classic `/` becomes `//`, an old-style octal `010` becomes
/// `0o10`, and an `L` suffix is dropped. That translation is exact for
/// this subset, because on ints Python 2's `/` and `//` both floor
/// (`i_divmod`) and Python 2 promotes past the C long exactly where Python 3's
/// unbounded int keeps going, and a Python 2 long is Python 3's int (which
/// `str()` prints without an `L`, as Python 2's `long_str` does). So:
///
/// - a case without a trap must print its `.expect`;
/// - a `Trap.DivZero` case must print the prefix and raise
///   `ZeroDivisionError`;
/// - a `Trap.Overflow` case must print the prefix and then the bigger
///   integer that its `.cite` records CPython 2 printing.
///
/// A host without `python3` is skipped, loudly.
#[test]
fn py2_corpus_agrees_with_host_python3_after_translation() {
    if Command::new("python3").args(["-I", "-c", "pass"]).output().map(|o| !o.status.success()).unwrap_or(true) {
        eprintln!("SKIP py2 translation oracle: no `python3` on PATH");
        return;
    }
    let mut agreed = 0;
    for case in corpus("py2-int-print") {
        let Outcome::Runs { stdout: want, trap } = case.outcome.clone() else {
            continue;
        };
        let translated = translate_py2_to_py3(&case.source);
        let out = Command::new("python3").args(["-I", "-c", &translated]).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
        let stderr = String::from_utf8_lossy(&out.stderr);
        match trap.as_deref() {
            None => {
                assert!(out.status.success(), "{}: {translated:?}: {stderr}", case.name);
                assert_eq!(stdout, want, "{}: {translated:?}", case.name);
            }
            Some("Trap.DivZero") => {
                assert!(!out.status.success() && stderr.contains("ZeroDivisionError"), "{}", case.name);
                assert_eq!(stdout, want, "{}", case.name);
            }
            Some("Trap.Overflow") => {
                assert!(out.status.success(), "{}: {stderr}", case.name);
                let rest = stdout.strip_prefix(want.as_str()).unwrap_or_else(|| panic!("{}: {stdout:?}", case.name));
                let next = rest.lines().next().unwrap_or_default();
                let value: i128 = next.parse().unwrap_or_else(|_| panic!("{}: {next:?}", case.name));
                assert!(i64::try_from(value).is_err(), "{}: {value} stays in i64", case.name);
                assert!(case.cite.contains(next), "{}.cite must record CPython 2's {next}", case.name);
            }
            Some(other) => panic!("{}: no oracle mapping for {other}", case.name),
        }
        agreed += 1;
    }
    assert!(agreed >= 24, "{agreed}");
}

/// The translation described above, for this corpus's shapes only: one or
/// more `;`-separated print statements per logical line, with `\`-newline
/// and parenthesised continuations kept as they are. The corpus has no
/// strings, so `;`, `#` and `/` never occur inside a literal.
fn translate_py2_to_py3(source: &str) -> String {
    // Join physical lines into logical ones first (backslash-newline, and
    // newlines inside parentheses), so each statement is one string.
    let mut logical: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    for raw in source.lines() {
        let line = raw.split('#').next().unwrap_or("");
        let (line, continued) = match line.trim_end().strip_suffix('\\') {
            Some(head) => (head.to_string(), true),
            None => (line.to_string(), false),
        };
        depth += line.matches('(').count() as i32 - line.matches(')').count() as i32;
        current.push_str(&line);
        current.push(' ');
        if !continued && depth == 0 {
            logical.push(std::mem::take(&mut current));
        }
    }
    let mut out = String::new();
    for line in logical {
        for statement in line.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            let expression = statement.strip_prefix("print").expect("every py2 corpus statement is a print").trim();
            // Classic `/` -> `//`, leaving an existing `//` alone; integer
            // literals through `py3_literal`. A literal starts at a digit
            // (the corpus has no names, so a digit never ends a name).
            let mut floor = String::new();
            let mut chars = expression.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '/' {
                    if chars.peek() == Some(&'/') {
                        chars.next();
                    }
                    floor.push_str("//");
                } else if c.is_ascii_digit() {
                    let mut literal = String::from(c);
                    while let Some(&next) = chars.peek().filter(|n| n.is_ascii_alphanumeric()) {
                        literal.push(next);
                        chars.next();
                    }
                    floor.push_str(&py3_literal(&literal));
                } else {
                    floor.push(c);
                }
            }
            out.push_str(&format!("print({floor})\n"));
        }
    }
    out
}

/// One Python 2 integer literal spelled for Python 3, with the same value:
/// the `l`/`L` suffix goes (Python 3's int is Python 2's long), and an
/// old-style octal `0NNN` gets the `0o` prefix PEP 3127 requires. Hex,
/// `0o` and `0b` literals are already Python 3 spellings.
fn py3_literal(literal: &str) -> String {
    let body = literal.strip_suffix(['l', 'L']).unwrap_or(literal);
    match body.as_bytes() {
        [b'0', rest @ ..] if !rest.is_empty() && rest.iter().all(u8::is_ascii_digit) => format!("0o{}", &body[1..]),
        _ => body.to_string(),
    }
}

#[test]
fn the_translation_keeps_every_literal_value() {
    for (python2, python3) in [
        ("010", "0o10"),
        ("00", "0o0"),
        ("0", "0"),
        ("0L", "0"),
        ("7l", "7"),
        ("0777L", "0o777"),
        ("0x1F", "0x1F"),
        ("0XffL", "0Xff"),
        ("0o17", "0o17"),
        ("0b101L", "0b101"),
    ] {
        assert_eq!(py3_literal(python2), python3, "{python2}");
    }
    assert_eq!(translate_py2_to_py3("print 010 / 3; print -7L\nprint\n"), "print(0o10 // 3)\nprint(-7)\nprint()\n");
}
