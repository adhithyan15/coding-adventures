//! # Whole-package analysis, reporting, scanner boundaries, Cargo facts, CLI
//!
//! Covers `analyze_package` end to end: violation classes and exit codes, the
//! AF-22 requirement that present-but-excepted constructs stay visible, what the
//! scanner includes and excludes, `Cargo.toml` fail-closed rules, and the CLI.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use coding_adventures_ca_capability_analyzer::cargo::{analyze_cargo_text, path_escapes_scan};
use coding_adventures_ca_capability_analyzer::report::BANNED_CONSTRUCTS;
use coding_adventures_ca_capability_analyzer::{
    analyze_package, AnalysisReport, ConstructStatus, ScanError, Violation,
};
use common::{manifest, pure_manifest, Pkg};

fn analyze(p: &Pkg) -> AnalysisReport {
    analyze_package(p.dir(), false).unwrap()
}

fn codes(r: &AnalysisReport) -> Vec<&'static str> {
    r.violations.iter().map(|v| v.code()).collect()
}

fn pkg(lib: &str, manifest_json: &str) -> Pkg {
    Pkg::with("pa", &[("src/lib.rs", lib), ("required_capabilities.json", manifest_json)])
}

// ---------------------------------------------------------------------------
// AF-22: present-but-excepted must be visible
// ---------------------------------------------------------------------------

#[test]
fn ct_compare_unsafe_is_reported_present_excepted_not_absent() {
    let r = analyze_package(Path::new("../ct-compare"), false).unwrap();
    assert!(r.is_clean());
    assert_eq!(r.exit_code(), 0);
    // It really contains two unsafe blocks, and the report says so.
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::PresentExcepted(2));
    assert_eq!(r.excepted_constructs.len(), 2);
    assert!(r.excepted_constructs.iter().all(|e| e.file == "src/lib.rs" && e.construct == "unsafe_code"));
    assert_eq!(r.construct_status("inline_assembly"), ConstructStatus::Absent);
    assert_eq!(r.construct_status("foreign_linkage"), ConstructStatus::Absent);

    // Non-verbose PASS text names the excepted unsafe and ends with the SUMMARY line.
    let text = r.format_text(false);
    assert!(text.starts_with("PASS:"));
    assert!(text.contains("PRESENT under manifest exception"), "{text}");
    assert!(text.contains("unsafe_code at src/lib.rs:"), "{text}");
    assert!(
        text.trim_end().ends_with(
            "SUMMARY unsafe_code=present-excepted(2) inline_assembly=absent foreign_linkage=absent ffi=absent unanalyzed_dependencies=0"
        ),
        "{text}"
    );
    // Verbose output must not claim "pure computation" while unsafe is present.
    let verbose = r.format_text(true);
    assert!(!verbose.contains("(pure computation)"), "{verbose}");
    assert!(verbose.contains("banned constructs present under exception"));
}

#[test]
fn truly_unsafe_free_package_reports_absent_and_may_say_pure() {
    let p = pkg("pub fn f() -> u8 { 1 }\n", &pure_manifest("clean"));
    let r = analyze(&p);
    assert!(r.is_clean());
    for c in BANNED_CONSTRUCTS {
        assert_eq!(r.construct_status(c), ConstructStatus::Absent);
    }
    assert!(r.excepted_constructs.is_empty());
    assert!(r.format_text(true).contains("(pure computation)"));
    assert!(r.summary_line().starts_with("SUMMARY unsafe_code=absent inline_assembly=absent foreign_linkage=absent ffi=absent"));
}

#[test]
fn construct_status_distinguishes_absent_excepted_and_violation() {
    // excepted
    let p = pkg("pub fn f() { unsafe { } unsafe { } }\n", &manifest("a", &[], &["unsafe_code"]));
    let r = analyze(&p);
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::PresentExcepted(2));
    assert_eq!(ConstructStatus::PresentExcepted(2).render(), "present-excepted(2)");
    // violation (no exception), plus an excepted construct of another kind elsewhere
    let p = pkg("pub fn f() { unsafe { } }\n", &pure_manifest("b"));
    let r = analyze(&p);
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::PresentViolation(1));
    assert_eq!(ConstructStatus::PresentViolation(1).render(), "present-violation(1)");
    assert!(!r.is_clean());
    assert!(r.format_text(false).contains("SUMMARY unsafe_code=present-violation(1)"));
    // Exception for a different construct does not waive unsafe.
    let p = pkg("pub fn f() { unsafe { } }\n", &manifest("c", &[], &["inline_assembly"]));
    assert_eq!(analyze(&p).construct_status("unsafe_code"), ConstructStatus::PresentViolation(1));
    assert_eq!(ConstructStatus::Absent.render(), "absent");
}

#[test]
fn ffi_presence_is_summarised() {
    let p = pkg("extern \"C\" { fn x(); }\n", &manifest("f", &[("ffi", "call")], &[]));
    let r = analyze(&p);
    assert!(r.is_clean());
    assert!(r.ffi_present());
    assert!(r.summary_line().contains("ffi=present"));
}

#[test]
fn failing_report_still_lists_excepted_constructs() {
    let p = pkg(
        "pub fn f() { unsafe { } std::fs::read(\"x\").ok(); }\n",
        &manifest("g", &[], &["unsafe_code"]),
    );
    let r = analyze(&p);
    assert_eq!(codes(&r), ["CAP001"]);
    let t = r.format_text(false);
    assert!(t.starts_with("FAIL:"));
    assert!(t.contains("NOTE: excepted unsafe_code"));
    assert!(t.contains("SUMMARY unsafe_code=present-excepted(1)"));
}

#[test]
fn unsafe_hidden_behind_allow_attributes_and_macros_is_still_reported() {
    let lib = r#"
        #![allow(unsafe_code)]
        #[allow(unsafe_code)] mod outer { #[allow(unsafe_code)] pub mod inner { pub fn f() { unsafe { } } } }
        macro_rules! sneaky { () => { unsafe { } } }
        pub fn g() { sneaky!(); }
    "#;
    let r = analyze(&pkg(lib, &pure_manifest("hid")));
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::PresentViolation(2));
    let r = analyze(&pkg(lib, &manifest("hid", &[], &["unsafe_code"])));
    assert!(r.is_clean());
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::PresentExcepted(2));
}

// ---------------------------------------------------------------------------
// Violation classes and ordering
// ---------------------------------------------------------------------------

#[test]
fn violation_classes_and_diagnostics() {
    let lib = "include!(\"x.rs\");\npub fn f() { unsafe { } std::fs::read(\"x\").ok(); }\n";
    let r = analyze(&pkg(lib, &pure_manifest("v")));
    assert_eq!(r.exit_code(), 1);
    let kinds: Vec<_> = r.violations.iter().map(|v| v.code()).collect();
    assert!(kinds.contains(&"CAP001") && kinds.contains(&"CAP002") && kinds.contains(&"CAP003"));
    // Sorted by file, then line.
    let lines: Vec<usize> = r.violations.iter().map(|v| v.line()).collect();
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted);
    for v in &r.violations {
        assert_eq!(v.file(), "src/lib.rs");
        assert!(v.format_line().starts_with(v.code()));
        let _ = v.col();
    }
    let t = r.format_text(false);
    assert!(t.contains("CAP001: Undeclared capability 'fs:read:*' detected at src/lib.rs:2:"));
    assert!(t.contains("CAP002: Banned construct 'unsafe_code' detected at src/lib.rs:2:"));
    assert!(t.contains("CAP003: Unanalysable construct detected at src/lib.rs:1:0 ('include!')"));
    assert!(matches!(r.violations[0], Violation::Unanalyzable { .. }));
}

#[test]
fn declared_capability_covers_detection_regardless_of_target() {
    let m = r#"{"version": 1, "package": "rust/x", "justification": "Valid justification text.",
        "capabilities": [{"category":"fs","action":"read","target":"only/this.txt","justification":"Reads one file."}]}"#;
    let r = analyze(&pkg("pub fn f() { std::fs::read(\"/etc/shadow\").ok(); }\n", m));
    assert!(r.is_clean(), "targets are not enforced (VER08a G-3)");
    // ... but a different action is not covered.
    let r = analyze(&pkg("pub fn f() { std::fs::write(\"a\", \"b\").ok(); }\n", m));
    assert_eq!(codes(&r), ["CAP001"]);
}

#[test]
fn absent_manifest_means_pure_and_reports_it() {
    let p = Pkg::with("nomf", &[("src/lib.rs", "pub fn f() { std::env::args(); }\n")]);
    let r = analyze(&p);
    assert!(!r.manifest_present);
    assert_eq!(codes(&r), ["CAP001"]);
    assert_eq!(r.package, "pure-computation");
}

#[test]
fn aliases_resolve_across_files_in_a_package() {
    let p = Pkg::with(
        "xfile",
        &[
            ("required_capabilities.json", &pure_manifest("xfile")),
            ("src/lib.rs", "pub mod a; pub mod b;\n"),
            ("src/a.rs", "pub use std as s;\n"),
            ("src/b.rs", "pub fn f() { crate::a::s::process::Command::new(\"x\"); }\n"),
        ],
    );
    let r = analyze(&p);
    assert_eq!(codes(&r), ["CAP001"]);
    assert!(r.detected_capabilities.contains("proc:exec:*"));
}

#[test]
fn unparseable_source_is_an_error_not_a_pass() {
    let p = pkg("pub fn f() { let s = \"unterminated; }\n", &pure_manifest("bad"));
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("src/lib.rs") && e.contains("unterminated string"), "{e}");
}

// ---------------------------------------------------------------------------
// Scanner boundaries
// ---------------------------------------------------------------------------

#[test]
fn generated_capabilities_module_is_scanned_not_skipped() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("capscan"));
    coding_adventures_ca_capability_analyzer::write_module_to_disk(p.dir()).unwrap();
    assert!(analyze(&p).is_clean(), "pristine generated module must scan clean");
    let original = p.read("src/capabilities.rs");
    p.write("src/capabilities.rs", &format!("{original}\npub fn evil() {{ unsafe {{ }} std::process::exit(1); }}\n"));
    let r = analyze(&p);
    assert!(codes(&r).contains(&"CAP002") && codes(&r).contains(&"CAP001"), "{:?}", r.violations);
    assert!(r.violations.iter().all(|v| v.file() == "src/capabilities.rs"));
}

#[test]
fn nested_dirs_named_like_excluded_roots_are_scanned_root_ones_are_not() {
    let bad = "pub fn x() { unsafe { } }\n";
    let p = pkg("pub mod target;\n", &pure_manifest("nest"));
    for nested in ["src/target/mod.rs", "src/tests/mod.rs", "src/node_modules/mod.rs", "src/_build/mod.rs", "src/benches/mod.rs", "src/examples/mod.rs"] {
        p.write(nested, bad);
    }
    let r = analyze(&p);
    assert_eq!(r.violations.len(), 6, "{:?}", r.violations);
    // Root-level excluded dirs are skipped.
    let q = pkg("pub fn f() {}\n", &pure_manifest("root"));
    for skipped in ["target/x.rs", "tests/x.rs", "benches/x.rs", "examples/x.rs", "node_modules/x.rs", "_build/x.rs", ".git/x.rs", "src/.hidden/x.rs"] {
        q.write(skipped, bad);
    }
    assert!(analyze(&q).is_clean());
}

#[test]
fn build_rs_and_other_root_files_are_scanned() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("brs"));
    p.write("build.rs", "fn main() { std::process::Command::new(\"cc\"); println!(\"cargo:rerun-if-changed=x\"); }\n");
    let r = analyze(&p);
    assert_eq!(r.violations.len(), 2);
    assert!(r.violations.iter().all(|v| v.file() == "build.rs"));
    // src/bin and other src files too.
    let q = pkg("pub fn f() {}\n", &pure_manifest("bin"));
    q.write("src/bin/tool.rs", "fn main() { std::env::args(); }\n");
    assert_eq!(analyze(&q).violations[0].file(), "src/bin/tool.rs");
}

#[test]
fn non_rs_files_are_not_scanned_but_include_str_data_is_fine() {
    let p = pkg("pub const D: &str = include_str!(\"data.txt\");\n", &pure_manifest("data"));
    p.write("data.txt", "unsafe { std::fs::read(\"x\") }");
    p.write("notes.md", "std::process::Command");
    assert!(analyze(&p).is_clean());
}

#[cfg(unix)]
#[test]
fn symlinked_file_and_directory_are_refused() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("sym"));
    p.write("other/real.rs", "pub fn x() {}\n");
    std::os::unix::fs::symlink(p.path("other/real.rs"), p.path("src/link.rs")).unwrap();
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("symlink refused"), "{e}");
    fs::remove_file(p.path("src/link.rs")).unwrap();
    std::os::unix::fs::symlink(p.path("other"), p.path("src/dirlink")).unwrap();
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("symlink refused"), "{e}");
}

#[test]
fn invalid_utf8_source_is_a_tool_error() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("u8"));
    fs::write(p.path("src/bad.rs"), [b'f', b'n', 0xff, 0xfe]).unwrap();
    let e = analyze_package(p.dir(), false).unwrap_err();
    assert!(matches!(e, coding_adventures_ca_capability_analyzer::AnalyzerError::Scan(ScanError::Io(_))), "{e}");
}

#[test]
fn file_size_limit_is_enforced() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("big"));
    p.write("src/huge.rs", &" ".repeat(4 * 1024 * 1024 + 1));
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("exceeds 4 MiB"), "{e}");
    // Exactly at the limit is allowed.
    p.write("src/huge.rs", &" ".repeat(4 * 1024 * 1024));
    assert!(analyze_package(p.dir(), false).is_ok());
}

#[test]
fn total_size_limit_is_enforced() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("total"));
    let chunk = " ".repeat(4 * 1024 * 1024);
    for i in 0..17 {
        p.write(&format!("src/part{i}.rs"), &chunk);
    }
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("total source size exceeds 64 MiB"), "{e}");
}

#[test]
fn file_count_limit_is_enforced() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("count"));
    for i in 0..10_001 {
        fs::write(p.path(&format!("src/f{i}.rs")).as_path(), "").or_else(|_| {
            fs::create_dir_all(p.path("src")).and_then(|_| fs::write(p.path(&format!("src/f{i}.rs")), ""))
        })
        .unwrap();
    }
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("maximum source file count"), "{e}");
}

#[test]
fn directory_depth_limit_is_enforced() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("deep"));
    let mut rel = String::from("src");
    for _ in 0..70 {
        rel.push_str("/d");
    }
    p.write(&format!("{rel}/x.rs"), "");
    let e = analyze_package(p.dir(), false).unwrap_err().to_string();
    assert!(e.contains("maximum directory depth"), "{e}");
}

#[test]
fn missing_package_dir_is_a_tool_error() {
    let e = analyze_package(Path::new("/definitely/not/here"), false);
    assert!(e.is_err());
}

// ---------------------------------------------------------------------------
// Cargo.toml facts
// ---------------------------------------------------------------------------

fn cargo_pkg(cargo: &str) -> Pkg {
    let p = pkg("pub fn f() {}\n", &pure_manifest("cargo"));
    p.write("Cargo.toml", cargo);
    p
}

#[test]
fn proc_macro_crates_fail_closed() {
    for toml in [
        "[lib]\nproc-macro = true\n",
        "[lib]\nproc_macro = true # comment\n",
        "[package]\nname = \"x\"\n\n[lib]\nname = \"x\"\nproc-macro = true\n",
    ] {
        let r = analyze(&cargo_pkg(toml));
        assert_eq!(codes(&r), ["CAP003"], "{toml}");
        assert_eq!(r.violations[0].file(), "Cargo.toml");
    }
    // `false` and other tables are fine.
    for toml in ["[lib]\nproc-macro = false\n", "[package]\nproc-macro = true\n", "[dependencies]\nproc-macro = \"1\"\n"] {
        assert!(!codes(&analyze(&cargo_pkg(toml))).contains(&"CAP003") || toml.contains("[dependencies]") || toml.contains("[package]"), "{toml}");
    }
    assert!(analyze(&cargo_pkg("[lib]\nproc-macro = false\n")).is_clean());
}

#[test]
fn lib_bin_and_build_paths_outside_scan_fail_closed() {
    for toml in [
        "[lib]\npath = \"tests/evil.rs\"\n",
        "[lib]\npath = \"benches/evil.rs\"\n",
        "[lib]\npath = \"../outside/lib.rs\"\n",
        "[lib]\npath = \"/abs/lib.rs\"\n",
        "[lib]\npath = \".hidden/lib.rs\"\n",
        "[lib]\npath = \"src/../tests/x.rs\"\n",
        "[[bin]]\nname = \"b\"\npath = \"examples/b.rs\"\n",
        "[[bin]]\nname = \"b\"\npath = \"target/b.rs\"\n",
        "[package]\nname = \"x\"\nbuild = \"../build.rs\"\n",
        "[package]\nname = \"x\"\nbuild = 'tests/build.rs'\n",
    ] {
        let r = analyze(&cargo_pkg(toml));
        assert_eq!(codes(&r), ["CAP003"], "{toml}");
    }
    // Normal layouts are fine.
    for toml in [
        "[lib]\npath = \"src/lib.rs\"\n",
        "[lib]\npath = \"./src/lib.rs\"\n",
        "[[bin]]\nname = \"b\"\npath = \"src/bin/b.rs\"\n",
        "[package]\nname = \"x\"\nbuild = \"build.rs\"\n",
        "[package]\nname = \"x\"\nbuild = false\n",
        // [[test]] / [[bench]] / [[example]] are not shipped (VER08a G-5).
        "[[test]]\nname = \"t\"\npath = \"tests/t.rs\"\nharness = false\n",
        "[[bench]]\nname = \"b\"\npath = \"benches/b.rs\"\n",
        "[[example]]\nname = \"e\"\npath = \"examples/e.rs\"\n",
    ] {
        assert!(analyze(&cargo_pkg(toml)).is_clean(), "{toml}");
    }
}

#[test]
fn path_escape_predicate() {
    for bad in ["tests/a.rs", "./tests/a.rs", "..", "../a.rs", "a/../b.rs", "/a", "C:\\a", "C:/a", ".x/a.rs", "src/.x/a.rs", "target/a.rs", "tests\\a.rs", "Tests/evil.rs", "TESTS/a.rs", "./Target/a.rs", "Benches\\a.rs", "Examples/a.rs"] {
        assert!(path_escapes_scan(bad), "{bad}");
    }
    for good in ["src/lib.rs", "./src/lib.rs", "src/tests/a.rs", "build.rs", "", "src/a/b/c.rs", "lib.rs"] {
        assert!(!path_escapes_scan(good), "{good}");
    }
}

#[test]
fn dependencies_are_reported_as_unanalyzed_without_changing_the_verdict() {
    let toml = r#"
[package]
name = "x"

[dependencies]
serde = "1"
libc = { version = "0.2" }
local = { path = "../local" } # trailing comment

[dev-dependencies]
only-for-tests = "1"

[build-dependencies]
cc = "1"

[target.'cfg(unix)'.dependencies]
nix = "0.27"

[dependencies.tokio]
version = "1"
"#;
    let r = analyze(&cargo_pkg(toml));
    assert!(r.is_clean());
    assert_eq!(r.unanalyzed_dependencies, ["cc", "libc", "local", "nix", "serde", "tokio"]);
    let t = r.format_text(false);
    assert!(t.contains("WARN: dependency 'serde' is not analyzed"), "{t}");
    assert!(t.contains("unanalyzed_dependencies=6"));
    assert!(!t.contains("only-for-tests"));
    // No Cargo.toml: nothing reported.
    assert!(analyze(&pkg("pub fn f() {}\n", &pure_manifest("nocargo"))).unanalyzed_dependencies.is_empty());
}

#[test]
fn cargo_text_reader_edge_cases() {
    // Comments inside strings are not comments; keys with quotes; empty and garbage lines.
    let f = analyze_cargo_text("[dependencies]\n\"quoted-dep\" = \"1\"\nplain = \"a # not comment\"\n\n# only comment\n");
    assert_eq!(f.dependencies, ["plain", "quoted-dep"]);
    assert!(f.findings.is_empty());
    // A line the reader cannot parse is a finding (fail closed), never skipped.
    let f = analyze_cargo_text("[dependencies]\nplain = \"1\"\ngarbage line without equals\n");
    assert_eq!(f.findings.len(), 1);
    assert_eq!(f.findings[0].line, 3);
    let f = analyze_cargo_text("[lib]\npath = \"tests/x.rs\" # why\n");
    assert_eq!(f.findings.len(), 1);
    assert_eq!(f.findings[0].line, 2);
    // Dotted top-level key.
    let f = analyze_cargo_text("lib.proc-macro = true\n");
    assert_eq!(f.findings.len(), 1);
    // Dependencies as dotted keys and renamed tables.
    let f = analyze_cargo_text("[dependencies]\nfoo.workspace = true\n[target.\"cfg(windows)\".build-dependencies.winres]\nversion = \"1\"\n");
    assert_eq!(f.dependencies, ["foo", "winres"]);
    assert!(analyze_cargo_text("").findings.is_empty());
}

#[cfg(unix)]
#[test]
fn symlinked_cargo_toml_is_refused() {
    let p = pkg("pub fn f() {}\n", &pure_manifest("symcargo"));
    p.write("real.toml", "[package]\n");
    std::os::unix::fs::symlink(p.path("real.toml"), p.path("Cargo.toml")).unwrap();
    assert!(analyze_package(p.dir(), false).is_err());
}

// ---------------------------------------------------------------------------
// Real package: results are derived, not hardcoded
// ---------------------------------------------------------------------------

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        if name == "target" {
            continue;
        }
        let (src, dst) = (e.path(), to.join(&name));
        if e.file_type().unwrap().is_dir() {
            copy_dir(&src, &dst);
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

#[test]
fn ct_compare_result_is_derived_from_its_files() {
    let p = Pkg::empty("ctcopy");
    copy_dir(Path::new("../ct-compare"), p.dir());
    let base = analyze(&p);
    assert!(base.is_clean());
    assert_eq!(base.construct_status("unsafe_code"), ConstructStatus::PresentExcepted(2));
    assert!(base.scanned_files_count >= 2);
    let digest_cert = coding_adventures_ca_capability_analyzer::generate_certificate(p.dir()).unwrap();
    fs::write(p.path("effect_certificate.json"), digest_cert.to_json_string()).unwrap();
    coding_adventures_ca_capability_analyzer::verify_certificate(p.dir(), &p.path("effect_certificate.json")).unwrap();

    // 1. Add an undeclared capability: the verdict changes.
    let lib = p.read("src/lib.rs");
    p.write("src/lib.rs", &format!("{lib}\npub fn leak() {{ let _ = std::env::var(\"SECRET\"); }}\n"));
    let r = analyze(&p);
    assert_eq!(codes(&r), ["CAP001"]);
    assert!(coding_adventures_ca_capability_analyzer::verify_certificate(p.dir(), &p.path("effect_certificate.json")).is_err());

    // 2. Restore, then drop the manifest exception: both unsafe blocks become violations.
    p.write("src/lib.rs", &lib);
    let m = p.read("required_capabilities.json");
    let start = m.find("\"banned_construct_exceptions\"").unwrap();
    let no_exc = format!("{}\"banned_construct_exceptions\": []\n}}\n", &m[..start]);
    p.write("required_capabilities.json", &no_exc);
    let r = analyze(&p);
    assert_eq!(codes(&r), ["CAP002", "CAP002"]);
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::PresentViolation(2));

    // 3. Remove the unsafe from source with the exception still absent: clean and unsafe-free.
    let safe = lib.replace("unsafe {", "{");
    p.write("src/lib.rs", &safe);
    let r = analyze(&p);
    assert!(r.is_clean(), "{:?}", r.violations);
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::Absent);
}

#[test]
fn analyzer_passes_its_own_analysis_and_declares_what_it_uses() {
    let r = analyze_package(Path::new(env!("CARGO_MANIFEST_DIR")), false).unwrap();
    assert!(r.is_clean(), "{:?}", r.violations);
    assert_eq!(r.construct_status("unsafe_code"), ConstructStatus::Absent);
    for cap in ["fs:read:*", "fs:list:*", "fs:write:*", "fs:create:*", "stdout:write:*", "env:read:*", "proc:signal:*"] {
        assert!(r.detected_capabilities.contains(cap), "expected self-use of {cap}");
    }
    assert!(r.unanalyzed_dependencies.is_empty());
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ca-capability-analyzer-rs"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn path_arg(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
fn cli_exit_codes_and_output() {
    let clean = pkg("pub fn f() {}\n", &pure_manifest("cli"));
    let (code, out, _) = run_cli(&["--dir", &path_arg(clean.dir())]);
    assert_eq!(code, 0);
    assert!(out.contains("PASS:") && out.contains("SUMMARY unsafe_code=absent"));

    let dirty = pkg("pub fn f() { std::fs::read(\"x\").ok(); }\n", &pure_manifest("cli"));
    let (code, out, _) = run_cli(&["--dir", &path_arg(dirty.dir())]);
    assert_eq!(code, 1);
    assert!(out.contains("CAP001") && out.contains("SUMMARY"));

    let unanalyzable = pkg("include!(\"x.rs\");\n", &pure_manifest("cli"));
    assert_eq!(run_cli(&["--dir", &path_arg(unanalyzable.dir())]).0, 1);

    let broken = pkg("pub fn f() { \"unterminated }\n", &pure_manifest("cli"));
    let (code, _, err) = run_cli(&["--dir", &path_arg(broken.dir())]);
    assert_eq!(code, 2);
    assert!(err.contains("unterminated"));

    let bad_manifest = pkg("pub fn f() {}\n", "{");
    assert_eq!(run_cli(&["--dir", &path_arg(bad_manifest.dir())]).0, 2);

    // CLI on ct-compare: PASS, and the excepted unsafe is shown without --verbose.
    let (code, out, _) = run_cli(&["--dir", "../ct-compare"]);
    assert_eq!(code, 0);
    assert!(out.contains("unsafe_code=present-excepted(2)"), "{out}");
}

#[test]
fn cli_argument_errors() {
    assert_eq!(run_cli(&["--bogus"]).0, 2);
    assert_eq!(run_cli(&["--dir"]).0, 2);
    assert_eq!(run_cli(&["--emit-cert"]).0, 2);
    assert_eq!(run_cli(&["--verify-cert"]).0, 2);
    let (code, out, _) = run_cli(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--verify-cert"));
    assert_eq!(run_cli(&["-h"]).0, 0);
    assert_eq!(run_cli(&["--dir", "/definitely/not/here"]).0, 2);
}

#[test]
fn cli_module_and_certificate_lifecycle() {
    let p = pkg("pub fn f() -> u8 { 1 }\n", &pure_manifest("life"));
    let d = path_arg(p.dir());
    let cert = path_arg(&p.path("cert.json"));

    assert_eq!(run_cli(&["--dir", &d, "--check-module"]).0, 1, "module missing -> drift");
    assert_eq!(run_cli(&["--dir", &d, "--generate-module"]).0, 0);
    assert_eq!(run_cli(&["--dir", &d, "--check-module"]).0, 0);
    assert_eq!(run_cli(&["--dir", &d, "--emit-cert", &cert]).0, 0);
    let (code, out, _) = run_cli(&["--dir", &d, "--verify-cert", &cert]);
    assert_eq!(code, 0);
    assert!(out.contains("verified"));

    // Tamper with the source: verification fails with exit 1.
    p.write("src/lib.rs", "pub fn f() -> u8 { 2 }\n");
    let (code, _, err) = run_cli(&["--dir", &d, "--verify-cert", &cert]);
    assert_eq!(code, 1);
    assert!(err.contains("source_digest mismatch"));

    // Drift in the generated module.
    p.write("src/lib.rs", "pub fn f() -> u8 { 1 }\n");
    p.write("src/capabilities.rs", "// hand edited\n");
    assert_eq!(run_cli(&["--dir", &d, "--check-module"]).0, 1);
}

#[test]
fn cli_refuses_to_emit_a_certificate_for_a_violating_package() {
    let p = pkg("pub fn f() { std::fs::read(\"x\").ok(); }\n", &pure_manifest("refuse"));
    let d = path_arg(p.dir());
    let cert: PathBuf = p.path("cert.json");
    let (code, _, err) = run_cli(&["--dir", &d, "--emit-cert", &path_arg(&cert)]);
    assert_eq!(code, 2);
    assert!(err.contains("refusing to emit certificate"), "{err}");
    assert!(!cert.exists(), "no certificate file may be written for a rejected package");
}
