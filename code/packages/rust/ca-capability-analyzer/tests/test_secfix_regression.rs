//! # Security-review regressions that are not one-line scan verdicts
//!
//! * **M3** - every way found to slip a `path` / `build` / `proc-macro` key past
//!   the old line-oriented `Cargo.toml` reader (VER08a §3.5);
//! * **M4** - the reviewer's denial-of-service inputs: each must finish quickly
//!   with a bounded result.  A `CAP003` verdict is the intended outcome (the
//!   analyzer refuses what it cannot bound); a hang or an allocation abort is not.
//!
//! The one-line verdict cases for M1, M2 and L1-L4 live in the evasion corpus
//! (`test_evasion_corpus.rs`, group `SECFIX`).

use std::time::{Duration, Instant};

use coding_adventures_ca_capability_analyzer::cargo::analyze_cargo_text;
use coding_adventures_ca_capability_analyzer::{scan_source, scan_sources, ScanResult};

// ---------------------------------------------------------------------------
// M3: Cargo.toml bypasses
// ---------------------------------------------------------------------------

fn findings(toml: &str) -> usize {
    analyze_cargo_text(toml).findings.len()
}

#[test]
fn m3_top_level_inline_tables_are_inspected() {
    for toml in [
        "lib = { path = \"../tests/evil.rs\" }\n",
        "lib = { name = \"x\", path = \"tests/evil.rs\" }\n",
        "lib = { proc-macro = true }\n",
        "lib = { proc_macro = true }\n",
        "package = { name = \"x\", build = \"../b.rs\" }\n",
        "bin = [{ name = \"b\", path = \"../x.rs\" }]\n",
        "bin = [ { name = \"a\", path = \"src/a.rs\" }, { name = \"b\", path = \"/abs/b.rs\" } ]\n",
        "bin = [\n  { name = \"b\",\n    path = \"../x.rs\" },\n]\n",
        "[package]\nname = \"x\"\nmetadata = { path = \"../x\" }\n",
    ] {
        assert!(findings(toml) >= 1, "bypass not caught: {toml:?}");
    }
}

#[test]
fn m3_dotted_and_quoted_spellings_are_inspected() {
    for toml in [
        "lib.path = \"../x.rs\"\n",
        "package.build = \"../b.rs\"\n",
        "lib.proc-macro = true\n",
        "\"lib\".proc-macro = true\n",
        "'lib'.path = \"../x.rs\"\n",
        "[\"lib\"]\nproc-macro = true\n",
        "['lib']\npath = \"../tests/x.rs\"\n",
        "[ \"lib\" ]\nproc_macro = true\n",
        "[lib]\n\"proc-macro\" = true\n",
        "[lib]\n'path' = \"../x.rs\"\n",
        "[\"package\"]\nbuild = \"../b.rs\"\n",
        "[[\"bin\"]]\npath = \"../x.rs\"\n",
        "[lib . path]\nx = 1\n",
        "[lib]\npath.x = 1\n",
    ] {
        assert!(findings(toml) >= 1, "bypass not caught: {toml:?}");
    }
}

#[test]
fn m3_multi_line_strings_cannot_hide_or_reset_a_section() {
    // A multi-line string value for a watched key is refused outright.
    for toml in [
        "[lib]\npath = \"\"\"\n../tests/evil.rs\"\"\"\n",
        "[lib]\npath = '''\n../tests/evil.rs'''\n",
        "[lib]\npath = \"\"\"../tests/evil.rs\"\"\"\n",
        "[package]\nbuild = '''\n../b.rs\n'''\n",
        // An escape sequence can spell `tests` without the letters appearing.
        "[lib]\npath = \"t\\u0065sts/evil.rs\"\n",
        "[lib]\npath = \"\\x2e\\x2e/evil.rs\"\n",
    ] {
        assert!(findings(toml) >= 1, "bypass not caught: {toml:?}");
    }
    // Header-lookalike lines inside a multi-line string do not change the section,
    // so the real key after the string is still judged in `[lib]`.
    let toml = "[lib]\ndoc = '''\n[dependencies]\n'''\npath = \"../tests/evil.rs\"\n";
    assert_eq!(findings(toml), 1, "{toml:?}");
    let toml = "[lib]\ndoc = \"\"\"\n[dependencies]\nfoo = 1\n\"\"\"\nproc-macro = true\n";
    assert_eq!(findings(toml), 1, "{toml:?}");
    // ... and a `path` line inside a string is data, not a key.
    let toml = "[package]\ndescription = '''\n[lib]\npath = \"../tests/evil.rs\"\n'''\n";
    assert_eq!(findings(toml), 0, "{toml:?}");
}

#[test]
fn m3_multi_line_arrays_cannot_reset_the_section() {
    // The old reader treated any line starting with `[` as a table header, so an
    // array-of-arrays inside `[lib]` moved "the current section" and the
    // `path` below was judged outside `[lib]`.
    for toml in [
        "[lib]\nfeatures = [\n[\"x\"],\n]\npath = \"../tests/evil.rs\"\n",
        "[lib]\nx = [\n  [1],\n  [2],\n]\nproc-macro = true\n",
        "[lib]\nx = [\n  [\"a\", \"b\"]\n]\npath = \"tests/evil.rs\"\n",
        "[package]\nx = [\n[\"y\"]\n]\nbuild = \"../b.rs\"\n",
        "[lib]\nx = [ # comment ]\n  [\"]\"],\n]\npath = \"../x.rs\"\n",
    ] {
        assert!(findings(toml) >= 1, "bypass not caught: {toml:?}");
    }
}

#[test]
fn m3_flagged_keys_with_unverifiable_values_are_refused() {
    for toml in [
        "[lib]\npath = [\"src/lib.rs\"]\n",
        "[lib]\npath = { x = 1 }\n",
        "[lib]\npath = 7\n",
        "[lib]\nproc-macro = \"yes\"\n",
        "[lib]\nproc-macro = [true]\n",
        "[lib]\nproc-macro = 1\n",
        "[package]\nbuild = true\n",
        "[package]\nbuild = [\"b.rs\"]\n",
        "[package]\nbuild = 1\n",
    ] {
        assert!(findings(toml) >= 1, "unverifiable value accepted: {toml:?}");
    }
}

#[test]
fn m3_unparseable_files_fail_closed() {
    for toml in [
        "garbage line without equals\n",
        "[lib\npath = \"src/lib.rs\"\n",
        "[lib]\npath = \"unterminated\n",
        "[lib]\npath = \"\"\"never closed\n",
        "[lib]\nx = [1, 2\n",
        "[lib]\nx = { a = 1\n",
        "[lib]\nx = = 1\n",
        "[lib]\n= 1\n",
        "[lib]\nx = 1 2\n",
        // Nesting deeper than the reader will follow.
        &format!("x = {}1{}\n", "[".repeat(40), "]".repeat(40)),
        // A quoted key spelled with an escape could decode to `lib`.
        "[\"l\\u0069b\"]\npath = \"../x.rs\"\n",
    ] {
        assert!(findings(toml) >= 1, "unparseable input accepted: {toml:?}");
    }
    let f = analyze_cargo_text("[lib]\nx = = 1\n");
    assert!(f.findings[0].reason.contains("could not be parsed"), "{:?}", f.findings);
}

#[test]
fn m3_legitimate_manifests_stay_clean() {
    for toml in [
        "",
        "# only a comment\n",
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = false\nauthors = [\"a <a@b.c>\"]\n",
        "[package]\nname = \"x\"\nversion.workspace = true\ndescription = \"a # not a comment\"\n",
        "[package]\nname = \"x\"\nbuild = \"build.rs\"\n",
        "[lib]\nname = \"x\"\npath = \"src/lib.rs\"\nproc-macro = false\ncrate-type = [\"lib\", \"cdylib\"]\n",
        "[lib]\npath = \"./src/lib.rs\"\n",
        "[[bin]]\nname = \"b\"\npath = \"src/bin/b.rs\"\n",
        "bin = [{ name = \"b\", path = \"src/bin/b.rs\" }]\n",
        "[[test]]\nname = \"t\"\npath = \"tests/t.rs\"\n[[bench]]\nname = \"b\"\npath = \"benches/b.rs\"\n[[example]]\nname = \"e\"\npath = \"examples/e.rs\"\n",
        "[dependencies]\nfoo = { path = \"../foo\", version = \"1\" }\nbar = { version = \"1\", features = [\"a\", \"b\"] }\n",
        "[dependencies.foo]\npath = \"../foo\"\n",
        "[dev-dependencies]\nfoo = { path = \"../foo\" }\n",
        "[build-dependencies]\nfoo = { path = \"../../foo\" }\n",
        "[target.'cfg(unix)'.dependencies]\nfoo = { path = \"../foo\" }\n",
        "[target.\"cfg(a.b)\".dependencies.foo]\npath = \"../foo\"\n",
        "[workspace.dependencies]\nfoo = { path = \"../foo\" }\n",
        "[patch.crates-io]\nfoo = { path = \"../foo\" }\n",
        "[features]\ndefault = []\nbuild = [\"x\"]\npath = []\nproc-macro = []\n",
        "[profile.release]\nlto = true\nopt-level = 3\n[profile.dev.build-override]\nopt-level = 0\n",
        "[lints.rust]\nunsafe_code = \"forbid\"\n[lints.clippy]\nall = { level = \"warn\", priority = -1 }\n",
        "[package]\r\nname = \"x\"\r\nbuild = false\r\n[lib]\r\npath = \"src/lib.rs\"\r\n",
        "\u{feff}[package]\nname = \"x\"\n",
        "[package]\nname = \"x\"\ndescription = '''\nline one\n[lib]\npath = \"../tests/not-a-key.rs\"\n'''\n",
        "[package]\nname = \"x\"\nreadme = \"\"\"\nmulti\n\"\"\"\n",
        "[package]\nname = \"x\"\ndate = 1979-05-27T07:32:00Z\nx = [ 1, 2, 3, ]\ny = { a = 1, b = { c = [1, 2] } }\nz = 0x1F\nw = -1.5e3\n",
        "[package.metadata.docs.rs]\nall-features = true\nrustdoc-args = [\"--cfg\", \"docsrs\"]\n",
    ] {
        assert_eq!(findings(toml), 0, "false positive on: {toml:?}\n{:?}", analyze_cargo_text(toml).findings);
    }
}

#[test]
fn m3_dependencies_are_still_reported() {
    let f = analyze_cargo_text(
        "[dependencies]\nserde = \"1\"\nlocal = { path = \"../l\" }\n[dev-dependencies]\nt = \"1\"\n\
         [build-dependencies.cc]\nversion = \"1\"\n[target.'cfg(unix)'.dependencies]\nnix = \"0.27\"\n",
    );
    assert_eq!(f.dependencies, ["cc", "local", "nix", "serde"]);
    assert!(f.findings.is_empty());
}

#[test]
fn m3_findings_carry_the_line_of_the_offending_key() {
    let f = analyze_cargo_text("[package]\nname = \"x\"\n\n[lib]\nname = \"x\"\npath = \"../x.rs\"\n");
    assert_eq!(f.findings.len(), 1);
    assert_eq!(f.findings[0].line, 6);
    let f = analyze_cargo_text("bin = [\n  { name = \"a\" },\n  { path = \"../b.rs\" },\n]\n");
    assert_eq!(f.findings.len(), 1);
    assert_eq!(f.findings[0].line, 3);
}

// ---------------------------------------------------------------------------
// M4: denial-of-service inputs
// ---------------------------------------------------------------------------

/// Run `f`, assert it finishes within 2 s, and return its result and duration.
fn timed<T>(label: &str, f: impl FnOnce() -> T) -> (T, Duration) {
    let t = Instant::now();
    let out = f();
    let d = t.elapsed();
    eprintln!("[M4 timing] {label}: {d:?}");
    assert!(d < Duration::from_secs(2), "{label} took {d:?}");
    (out, d)
}

fn unanalyzable_reasons(r: &ScanResult) -> Vec<&str> {
    r.unanalyzable.iter().map(|u| u.reason.as_str()).collect()
}

#[test]
fn m4_quadratic_alias_expansion_is_refused_before_it_is_built() {
    // 7000 bindings named `x` x 7000 entries headed by `x` = 49M expansions
    // (2.3 GB of Vec<String> before the old cap check ran).
    let n = 7000;
    let mut src = String::new();
    for i in 0..n {
        src.push_str(&format!("use m{i}::T as x;\n"));
    }
    for i in 0..n {
        src.push_str(&format!("use x::s{i} as y{i};\n"));
    }
    assert!(src.len() > 200_000, "{}", src.len());
    let (res, _) = timed("alias N^2", || scan_source("src/lib.rs", &src).unwrap());
    assert!(
        unanalyzable_reasons(&res).iter().any(|r| r.contains("alias expansion grew without bound")),
        "{:?}",
        unanalyzable_reasons(&res)
    );
    assert!(res.detections.is_empty());
}

#[test]
fn m4_duplicate_bindings_do_not_multiply() {
    // Thousands of *identical* bindings collapse to one before expansion.
    let mut src = String::new();
    for _ in 0..5000 {
        src.push_str("use std::fs as x;\nuse x::File as y;\n");
    }
    let (res, _) = timed("duplicate bindings", || scan_source("src/lib.rs", &src).unwrap());
    assert!(res.unanalyzable.is_empty(), "{:?}", unanalyzable_reasons(&res));
    assert!(res.detections.iter().all(|d| d.category == "fs"));
}

#[test]
fn m4_long_paths_are_refused_not_resolved_quadratically() {
    let long = "a::".repeat(60_000) + "read";
    let src = format!("use q::r as a;\nfn f() {{ {long}(); }}\n");
    let (res, _) = timed("path length", || scan_source("src/lib.rs", &src).unwrap());
    assert!(unanalyzable_reasons(&res).iter().any(|r| r.contains("too many segments")), "{:?}", unanalyzable_reasons(&res));
    // A path at the limit is still resolved normally.
    let ok = "a::".repeat(30) + "read";
    let (res, _) = timed("path at limit", || scan_source("src/lib.rs", &format!("fn f() {{ {ok}(); }}")).unwrap());
    assert!(res.unanalyzable.is_empty());
}

#[test]
fn m4_many_spellings_of_one_name_are_refused() {
    // 300 different meanings for `x`: 64+ candidate spellings for `x::y`.
    let mut src = String::new();
    for i in 0..300 {
        src.push_str(&format!("use m{i}::T as x;\n"));
    }
    src.push_str("fn f() { x::y(); }\n");
    let (res, _) = timed("candidate explosion", || scan_source("src/lib.rs", &src).unwrap());
    assert!(unanalyzable_reasons(&res).iter().any(|r| r.contains("too many alias spellings")), "{:?}", unanalyzable_reasons(&res));
}

#[test]
fn m4_nested_and_unbalanced_attributes_are_linear() {
    let n = 100_000;
    for (label, src) in [
        ("#[#[ unbalanced", "#[".repeat(n)),
        ("#![#![ unbalanced", "#![".repeat(n)),
        ("#[#[ balanced", "#[".repeat(n) + &"]".repeat(n)),
        ("#[ nested with idents", "#[a ".repeat(n) + &"]".repeat(n)),
        ("(((( unbalanced", "(".repeat(n * 2)),
        ("#[ ))) stray closers", ")".repeat(n) + &"#[x]".repeat(n)),
    ] {
        let (res, _) = timed(label, || scan_source("src/lib.rs", &src).unwrap());
        assert!(res.detections.len() < 100 && res.banned.len() < 100 && res.unanalyzable.len() < 100, "{label}");
    }
}

#[test]
fn m4_nested_macro_rules_are_budgeted() {
    let n = 20_000;
    let src = "macro_rules! m { ".repeat(n) + &"}".repeat(n);
    let (res, _) = timed("nested macro_rules", || scan_source("src/lib.rs", &src).unwrap());
    assert!(unanalyzable_reasons(&res).iter().any(|r| r.contains("exceeds the analysis budget")), "{:?}", unanalyzable_reasons(&res));
    // Attribute nesting inside a macro body is linear as well.
    let src = format!("macro_rules! m {{ () => {{ {} }} }}", "#[".repeat(50_000) + &"]".repeat(50_000));
    timed("attributes in macro body", || scan_source("src/lib.rs", &src).unwrap());
}

#[test]
fn m4_glob_count_is_capped() {
    let mut src = String::new();
    for i in 0..7000 {
        src.push_str(&format!("use std::g{i}::*;\n"));
    }
    src.push_str("fn f() { a(); b(); }\n");
    let (res, _) = timed("glob flood", || scan_source("src/lib.rs", &src).unwrap());
    assert!(unanalyzable_reasons(&res).iter().any(|r| r.contains("too many glob imports")), "{:?}", unanalyzable_reasons(&res));
    // A moderate number of globs times many paths stays inside the per-file budget.
    let mut src = String::new();
    for i in 0..200 {
        src.push_str(&format!("use std::g{i}::*;\n"));
    }
    for i in 0..20_000 {
        src.push_str(&format!("fn f{i}() {{ p{i}(); }}\n"));
    }
    let (res, _) = timed("globs x paths", || scan_source("src/lib.rs", &src).unwrap());
    assert!(res.detections.len() + res.banned.len() + res.unanalyzable.len() < 100_000);
    assert!(!res.unanalyzable.is_empty(), "work budget must trip");
}

#[test]
fn m4_multi_file_packages_share_the_caps() {
    let big: String = (0..4000).map(|i| format!("use m{i}::T as x;\n")).collect();
    let big2: String = (0..4000).map(|i| format!("use x::s{i} as y{i};\n")).collect();
    let (res, _) = timed("multi-file alias flood", || {
        scan_sources(&[("src/a.rs", big.as_str()), ("src/b.rs", big2.as_str())]).unwrap()
    });
    assert!(unanalyzable_reasons(&res).iter().any(|r| r.contains("grew without bound")));
}
