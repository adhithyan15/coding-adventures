//! Hostile-input and cross-stack regression tests for ca-capability-analyzer (VER08a / VER15).
#![forbid(unsafe_code)]

mod common;

use std::path::PathBuf;

use coding_adventures_ca_capability_analyzer::rules::{classify_path, Classification};
use coding_adventures_ca_capability_analyzer::{analyze_package, scan_source};
use common::{pure_manifest, Pkg};

#[test]
fn hostile_malformed_rust_inputs_never_panic() {
    let payloads = [
        "",
        "\0\0\0",
        "///",
        "/* unclosed block comment",
        "\"unterminated string",
        "r###\"unterminated raw string",
        "#!/usr/bin/env bash\n\"unclosed",
        "fn broken( {",
        "use std::{fs::{read, ",
        "macro_rules! m { ($($x:tt)*) => { $($x)* }; }",
        &"(".repeat(2_000),
        &format!("{}1{}", "{".repeat(600), "}".repeat(600)),
        &format!("{}fn f() {{}}", "#[cfg(all())]".repeat(500)),
        &format!("a::{}", "b::".repeat(128)),
    ];

    for (idx, payload) in payloads.iter().enumerate() {
        let res = scan_source("src/hostile.rs", payload);
        if let Ok(r) = res {
            // Must remain bounded and deterministic on repeat.
            let r2 = scan_source("src/hostile.rs", payload).unwrap();
            assert_eq!(r.detections.len(), r2.detections.len(), "case {idx}");
            assert_eq!(r.banned.len(), r2.banned.len(), "case {idx}");
            assert_eq!(r.unanalyzable.len(), r2.unanalyzable.len(), "case {idx}");
        }
    }
}

#[test]
fn hostile_non_utf8_source_and_malformed_cargo_fail_closed() {
    let p = Pkg::empty("nonutf8_src");
    p.write("required_capabilities.json", &pure_manifest("nonutf8_src"));
    std::fs::create_dir_all(p.path("src")).unwrap();
    std::fs::write(p.path("src/lib.rs"), [0xff, 0xfe, 0x00, 0x20]).unwrap();
    assert!(analyze_package(p.dir(), false).is_err());

    let p_toml = Pkg::with(
        "bad_toml",
        &[
            ("required_capabilities.json", &pure_manifest("bad_toml")),
            ("Cargo.toml", "[package\nname = \"x\"\nproc-macro = true\n"),
            ("src/lib.rs", "pub fn ok() -> i32 { 1 }\n"),
        ],
    );
    let rep = analyze_package(p_toml.dir(), false).unwrap();
    assert!(!rep.is_clean(), "malformed Cargo.toml must fail closed");
}

#[test]
fn cross_stack_regression_local_var_named_like_third_party_crate_is_pure() {
    // Regression for rust/dsp-filters false positive: `let windows = [...];`
    // and `windows.len()` must NOT trigger `ffi:call:*`.
    let src = r#"
        pub fn pick_window(idx: usize) -> u32 {
            let windows = [10u32, 20, 30, 40];
            let rand = 7u32;
            let nix = 1u32;
            let hyper = 2u32;
            let smol = 3u32;
            windows[idx % windows.len()] + rand + nix + hyper + smol
        }
    "#;
    let p = Pkg::with(
        "dsp_filters_reg",
        &[
            ("required_capabilities.json", &pure_manifest("dsp_filters_reg")),
            ("src/lib.rs", src),
        ],
    );
    let rep = analyze_package(p.dir(), false).unwrap();
    assert!(rep.is_clean(), "unexpected violations: {:?}", rep.violations);
    assert!(rep.detected_capabilities.is_empty());

    // Bare single-segment names are pure in classify_path, while qualified paths
    // (`windows::...`, `rand::...`) and bare `use windows;` still detect capability.
    assert_eq!(classify_path("windows"), Classification::AllowedPure);
    assert_eq!(
        classify_path("windows::Win32::System::Threading::GetCurrentProcessId"),
        Classification::Capability {
            category: "ffi",
            action: "call",
        }
    );

    let p_use = Pkg::with(
        "bare_use_windows",
        &[
            ("required_capabilities.json", &pure_manifest("bare_use_windows")),
            ("src/lib.rs", "use windows;\npub fn f() {}\n"),
        ],
    );
    let rep_use = analyze_package(p_use.dir(), false).unwrap();
    assert!(!rep_use.is_clean());
    assert!(rep_use.detected_capabilities.contains("ffi:call:*"));
}

#[test]
fn cross_stack_real_dsp_filters_passes_capability_audit() {
    let dsp_filters_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("dsp-filters");
    if dsp_filters_dir.is_dir() {
        let rep = analyze_package(&dsp_filters_dir, false).unwrap();
        assert!(
            rep.is_clean(),
            "rust/dsp-filters should pass without false positives: {:?}",
            rep.violations
        );
    }
}
