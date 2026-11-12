//! # Test Suite for Rust Capability Analyzer (VER08a / VER-P027a)

#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use coding_adventures_ca_capability_analyzer::manifest::{
    is_valid_capability_pair, load_manifest, JsonValue, ManifestData,
};
use coding_adventures_ca_capability_analyzer::parser::scan_source;
use coding_adventures_ca_capability_analyzer::rules::{classify_path, Classification};
use coding_adventures_ca_capability_analyzer::{analyze_package, Violation};

// ===========================================================================
// 1. JSON Parser Tests
// ===========================================================================

#[test]
fn test_json_parser_primitives() {
    assert_eq!(JsonValue::parse("null").unwrap(), JsonValue::Null);
    assert_eq!(JsonValue::parse("true").unwrap(), JsonValue::Bool(true));
    assert_eq!(JsonValue::parse("false").unwrap(), JsonValue::Bool(false));
    assert_eq!(
        JsonValue::parse("\"hello\\nworld\"").unwrap(),
        JsonValue::String("hello\nworld".into())
    );
    assert_eq!(
        JsonValue::parse("\"unicode: \\u0041\"").unwrap(),
        JsonValue::String("unicode: A".into())
    );
    assert_eq!(
        JsonValue::parse("42").unwrap(),
        JsonValue::Number(42.0, Some(42))
    );
    assert_eq!(
        JsonValue::parse("-123.45").unwrap(),
        JsonValue::Number(-123.45, None)
    );
}

#[test]
fn test_json_parser_structures() {
    let json = r#"
    {
        "version": 1,
        "name": "test",
        "tags": ["a", "b", 3],
        "nested": { "flag": true }
    }
    "#;
    let val = JsonValue::parse(json).unwrap();
    let obj = val.as_object().unwrap();
    assert_eq!(obj.get("version").unwrap().as_i64(), Some(1));
    assert_eq!(obj.get("name").unwrap().as_str(), Some("test"));
    let tags = obj.get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 3);
}

#[test]
fn test_json_parser_errors() {
    assert!(JsonValue::parse("{ invalid }").is_err());
    assert!(JsonValue::parse("[1, 2,").is_err());
    assert!(JsonValue::parse("\"unterminated").is_err());
    assert!(JsonValue::parse("true trailing").is_err());
}

// ===========================================================================
// 2. Manifest Validation Tests
// ===========================================================================

#[test]
fn test_spec13_valid_capability_pairs() {
    assert!(is_valid_capability_pair("fs", "read"));
    assert!(is_valid_capability_pair("fs", "write"));
    assert!(is_valid_capability_pair("fs", "create"));
    assert!(is_valid_capability_pair("fs", "delete"));
    assert!(is_valid_capability_pair("fs", "list"));
    assert!(is_valid_capability_pair("net", "connect"));
    assert!(is_valid_capability_pair("net", "listen"));
    assert!(is_valid_capability_pair("net", "dns"));
    assert!(is_valid_capability_pair("proc", "exec"));
    assert!(is_valid_capability_pair("proc", "fork"));
    assert!(is_valid_capability_pair("proc", "signal"));
    assert!(is_valid_capability_pair("env", "read"));
    assert!(is_valid_capability_pair("env", "write"));
    assert!(is_valid_capability_pair("ffi", "call"));
    assert!(is_valid_capability_pair("ffi", "load"));
    assert!(is_valid_capability_pair("time", "read"));
    assert!(is_valid_capability_pair("time", "sleep"));
    assert!(is_valid_capability_pair("stdin", "read"));
    assert!(is_valid_capability_pair("stdout", "write"));
    assert!(is_valid_capability_pair("rand", "read"));
    assert!(is_valid_capability_pair("thread", "spawn"));
    assert!(is_valid_capability_pair("thread", "message"));
    assert!(is_valid_capability_pair("ui", "read"));
    assert!(is_valid_capability_pair("ui", "write"));
    assert!(is_valid_capability_pair("ui", "event"));

    // Invalid pairs
    assert!(!is_valid_capability_pair("fs", "spawn"));
    assert!(!is_valid_capability_pair("net", "delete"));
    assert!(!is_valid_capability_pair("unknown", "read"));
}

#[test]
fn test_manifest_absent_defaults_to_pure_computation() {
    let tmp = tempfile_dir("manifest_absent");
    let manifest = load_manifest(&tmp).unwrap();
    assert_eq!(manifest.package, "pure-computation");
    assert!(!manifest.is_explicit);
    assert!(manifest.capabilities.is_empty());
    assert!(!manifest.covers_capability("fs", "read"));
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_manifest_validation_failures() {
    // 1. Invalid version
    let bad_ver = r#"{"version": 2, "package": "rust/foo", "justification": "Valid length justification", "capabilities": []}"#;
    assert!(test_manifest_string(bad_ver).is_err());

    // 2. Invalid package prefix (must start with rust/)
    let bad_pkg = r#"{"version": 1, "package": "go/foo", "justification": "Valid length justification", "capabilities": []}"#;
    assert!(test_manifest_string(bad_pkg).is_err());

    // 3. Short justification (< 10 chars)
    let short_just =
        r#"{"version": 1, "package": "rust/foo", "justification": "short", "capabilities": []}"#;
    assert!(test_manifest_string(short_just).is_err());

    // 4. Invalid capability pair
    let bad_pair = r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [{"category": "fs", "action": "spawn", "target": "*", "justification": "Valid length justification"}]}"#;
    assert!(test_manifest_string(bad_pair).is_err());

    // 5. Unknown top-level field
    let unknown_field = r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [], "unexpected": 123}"#;
    assert!(test_manifest_string(unknown_field).is_err());

    // 6. Duplicate capability
    let duplicate = r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [
        {"category": "fs", "action": "read", "target": "*", "justification": "Valid length justification"},
        {"category": "fs", "action": "read", "target": "*", "justification": "Valid length justification"}
    ]}"#;
    assert!(test_manifest_string(duplicate).is_err());

    // 7. Non-rust banned exception language
    let bad_lang = r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [], "banned_construct_exceptions": [
        {"construct": "unsafe_code", "language": "python", "justification": "Valid length justification"}
    ]}"#;
    assert!(test_manifest_string(bad_lang).is_err());
}

// ===========================================================================
// 3. Rules & Classification Tests
// ===========================================================================

#[test]
fn test_path_classification() {
    assert_eq!(
        classify_path("std::fs::read_to_string"),
        Classification::Capability {
            category: "fs",
            action: "read"
        }
    );
    assert_eq!(
        classify_path("std::process::Command::new"),
        Classification::Capability {
            category: "proc",
            action: "exec"
        }
    );
    assert_eq!(
        classify_path("std::net::TcpStream::connect"),
        Classification::Capability {
            category: "net",
            action: "connect"
        }
    );
    assert_eq!(
        classify_path("std::env::var"),
        Classification::Capability {
            category: "env",
            action: "read"
        }
    );
    assert_eq!(
        classify_path("std::time::Instant::now"),
        Classification::Capability {
            category: "time",
            action: "read"
        }
    );
    assert_eq!(
        classify_path("std::io::stdin"),
        Classification::Capability {
            category: "stdin",
            action: "read"
        }
    );
    assert_eq!(
        classify_path("println"),
        Classification::Capability {
            category: "stdout",
            action: "write"
        }
    );
    assert_eq!(
        classify_path("std::thread::spawn"),
        Classification::Capability {
            category: "thread",
            action: "spawn"
        }
    );
    assert_eq!(
        classify_path("libc::getpid"),
        Classification::Capability {
            category: "ffi",
            action: "call"
        }
    );
    assert_eq!(
        classify_path("asm"),
        Classification::BannedConstruct {
            construct: "inline_assembly"
        }
    );

    // Pure computation allowlist
    assert_eq!(
        classify_path("core::hint::black_box"),
        Classification::AllowedPure
    );
    assert_eq!(classify_path("std::vec::Vec"), Classification::AllowedPure);
    assert_eq!(classify_path("std::cmp::min"), Classification::AllowedPure);
    assert_eq!(classify_path("assert_eq"), Classification::AllowedPure);
}

// ===========================================================================
// 4. Source Scanning Tests
// ===========================================================================

#[test]
fn test_source_scanner_ignores_comments_and_strings() {
    let src = r##"
        // std::fs::read("should be ignored in line comment");
        /*
           std::process::Command::new("should be ignored in block comment");
           unsafe { should_be_ignored(); }
        */
        let msg = "std::net::TcpStream::connect should be ignored in string literal";
        let raw = r#"std::env::var and unsafe in raw string"#;
    "##;
    let res = scan_source("src/dummy.rs", src).unwrap();
    assert!(res.detections.is_empty());
    assert!(res.banned.is_empty());
}

#[test]
fn test_source_scanner_detects_real_accesses() {
    let src = r#"
        pub fn do_work() {
            let _ = std::fs::read("/etc/passwd");
            println!("hello");
            unsafe {
                do_raw();
            }
        }
    "#;
    let res = scan_source("src/work.rs", src).unwrap();
    assert_eq!(res.detections.len(), 2);
    assert_eq!(res.detections[0].category, "fs");
    assert_eq!(res.detections[0].action, "read");
    assert_eq!(res.detections[1].category, "stdout");
    assert_eq!(res.detections[1].action, "write");

    assert_eq!(res.banned.len(), 1);
    assert_eq!(res.banned[0].construct, "unsafe_code");
}

#[test]
fn test_source_scanner_detects_attributes_and_extern() {
    let src = r#"
        #[no_mangle]
        pub extern "C" fn custom_entry() {}
    "#;
    let res = scan_source("src/ffi.rs", src).unwrap();
    assert_eq!(res.banned.len(), 1);
    assert_eq!(res.banned[0].construct, "foreign_linkage");

    assert_eq!(res.detections.len(), 1);
    assert_eq!(res.detections[0].category, "ffi");
    assert_eq!(res.detections[0].action, "call");
}

// ===========================================================================
// 5. Package End-to-End Analysis Tests
// ===========================================================================

#[test]
fn test_analyze_real_ct_compare_package() {
    // ct-compare has an audited volatile barrier (unsafe_code exception) and 0 capabilities
    // No `exists()` guard: a missing ct-compare directory must fail the test, not skip it.
    let ct_compare_dir = PathBuf::from("../ct-compare");
    let report = analyze_package(&ct_compare_dir, true).unwrap();
    assert!(
        report.is_clean(),
        "ct-compare should pass capability check: {:?}",
        report.violations
    );
    assert_eq!(report.exit_code(), 0);
    assert!(report.scanned_files_count >= 1);
    // AF-22: the audited unsafe is PRESENT (excepted), and the report must say so.
    assert!(matches!(
        report.construct_status("unsafe_code"),
        coding_adventures_ca_capability_analyzer::ConstructStatus::PresentExcepted(n) if n >= 1
    ));
}

#[test]
fn test_synthetic_undeclared_capability_violation() {
    let tmp = tempfile_dir("synthetic_cap001");
    fs::create_dir_all(tmp.join("src")).unwrap();

    let manifest = r#"{
        "version": 1,
        "package": "rust/leak",
        "justification": "Pure computation package attempting undeclared filesystem read.",
        "capabilities": []
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest).unwrap();

    let src = "pub fn leak() { let _ = std::fs::read(\"secret.txt\"); }";
    fs::write(tmp.join("src/lib.rs"), src).unwrap();

    let report = analyze_package(&tmp, false).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.exit_code(), 1);
    assert_eq!(report.violations.len(), 1);

    match &report.violations[0] {
        Violation::UndeclaredCapability {
            capability,
            file,
            line,
            ..
        } => {
            assert_eq!(capability, "fs:read:*");
            assert_eq!(file, "src/lib.rs");
            assert_eq!(*line, 1);
        }
        _ => panic!("expected UndeclaredCapability violation"),
    }

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_synthetic_banned_construct_violation() {
    let tmp = tempfile_dir("synthetic_cap002");
    fs::create_dir_all(tmp.join("src")).unwrap();

    let manifest = r#"{
        "version": 1,
        "package": "rust/unsafe-test",
        "justification": "Package containing unexempt unsafe code.",
        "capabilities": []
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest).unwrap();

    let src = "pub fn do_unsafe() { unsafe { let _ = 42; } }";
    fs::write(tmp.join("src/lib.rs"), src).unwrap();

    let report = analyze_package(&tmp, false).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.exit_code(), 1);
    assert_eq!(report.violations.len(), 1);

    match &report.violations[0] {
        Violation::BannedConstruct {
            construct, file, ..
        } => {
            assert_eq!(construct, "unsafe_code");
            assert_eq!(file, "src/lib.rs");
        }
        _ => panic!("expected BannedConstruct violation"),
    }

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_synthetic_exempted_banned_construct_passes() {
    let tmp = tempfile_dir("synthetic_exempted");
    fs::create_dir_all(tmp.join("src")).unwrap();

    let manifest = r#"{
        "version": 1,
        "package": "rust/exempted-test",
        "justification": "Package containing audited and exempted unsafe code.",
        "capabilities": [],
        "banned_construct_exceptions": [
            {
                "construct": "unsafe_code",
                "language": "rust",
                "justification": "Audited raw pointer barrier verified by WIR formal model."
            }
        ]
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest).unwrap();

    let src = "pub fn do_unsafe() { unsafe { let _ = 42; } }";
    fs::write(tmp.join("src/lib.rs"), src).unwrap();

    let report = analyze_package(&tmp, false).unwrap();
    assert!(report.is_clean());
    assert_eq!(report.exit_code(), 0);

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_scanner_refuses_symlinks() {
    let tmp = tempfile_dir("symlink_refusal");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src/real.rs"), "pub fn ok() {}").unwrap();

    // Create a symlink
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(tmp.join("src/real.rs"), tmp.join("src/symlink.rs")).unwrap();
        let res = analyze_package(&tmp, false);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("symlink refused"));
    }

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_report_formatting_and_accessors() {
    let tmp = tempfile_dir("report_fmt");
    fs::create_dir_all(tmp.join("src")).unwrap();

    let manifest = r#"{
        "version": 1,
        "package": "rust/report-test",
        "justification": "Testing report formatting under clean and failing conditions.",
        "capabilities": [
            {
                "category": "stdout",
                "action": "write",
                "target": "*",
                "justification": "Allows printing output in test."
            }
        ]
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest).unwrap();

    let clean_src = "pub fn work() { println!(\"clean\"); }";
    fs::write(tmp.join("src/lib.rs"), clean_src).unwrap();

    let clean_report = analyze_package(&tmp, true).unwrap();
    assert!(clean_report.is_clean());
    let clean_text = clean_report.format_text(true);
    assert!(clean_text.contains("PASS: package 'rust/report-test'"));
    assert!(clean_text.contains("stdout:write:*"));

    // Add undeclared capability
    let fail_src = "pub fn bad() { let _ = std::fs::read(\"foo\"); }";
    fs::write(tmp.join("src/lib.rs"), fail_src).unwrap();

    let fail_report = analyze_package(&tmp, false).unwrap();
    assert!(!fail_report.is_clean());
    assert_eq!(fail_report.exit_code(), 1);
    let fail_text = fail_report.format_text(false);
    assert!(fail_text.contains("FAIL: package 'rust/report-test' has 1 capability violation"));

    let v = &fail_report.violations[0];
    assert_eq!(v.code(), "CAP001");
    assert_eq!(v.file(), "src/lib.rs");
    assert_eq!(v.line(), 1);
    assert_eq!(v.col(), 23);
    assert!(v
        .format_line()
        .contains("CAP001: Undeclared capability 'fs:read:*'"));

    let b_violation = Violation::BannedConstruct {
        construct: "unsafe_code".into(),
        file: "src/foo.rs".into(),
        line: 10,
        col: 5,
        evidence: "unsafe".into(),
    };
    assert_eq!(b_violation.code(), "CAP002");
    assert_eq!(b_violation.file(), "src/foo.rs");
    assert_eq!(b_violation.line(), 10);
    assert_eq!(b_violation.col(), 5);
    assert!(b_violation
        .format_line()
        .contains("CAP002: Banned construct 'unsafe_code'"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_all_capability_categories_in_rules() {
    assert_eq!(
        classify_path("std::fs::create_dir"),
        Classification::Capability {
            category: "fs",
            action: "create"
        }
    );
    assert_eq!(
        classify_path("std::fs::remove_file"),
        Classification::Capability {
            category: "fs",
            action: "delete"
        }
    );
    assert_eq!(
        classify_path("std::fs::write"),
        Classification::Capability {
            category: "fs",
            action: "write"
        }
    );
    assert_eq!(
        classify_path("std::net::TcpListener::bind"),
        Classification::Capability {
            category: "net",
            action: "listen"
        }
    );
    assert_eq!(
        classify_path("std::net::ToSocketAddrs"),
        Classification::Capability {
            category: "net",
            action: "dns"
        }
    );
    assert_eq!(
        classify_path("std::process::abort"),
        Classification::Capability {
            category: "proc",
            action: "signal"
        }
    );
    assert_eq!(
        classify_path("std::process::exit"),
        Classification::Capability {
            category: "proc",
            action: "signal"
        }
    );
    assert_eq!(
        classify_path("std::env::set_var"),
        Classification::Capability {
            category: "env",
            action: "write"
        }
    );
    assert_eq!(
        classify_path("std::env::remove_var"),
        Classification::Capability {
            category: "env",
            action: "write"
        }
    );
    assert_eq!(
        classify_path("std::thread::sleep"),
        Classification::Capability {
            category: "time",
            action: "sleep"
        }
    );
    assert_eq!(
        classify_path("std::thread::spawn"),
        Classification::Capability {
            category: "thread",
            action: "spawn"
        }
    );
    assert_eq!(
        classify_path("std::io::stderr"),
        Classification::Capability {
            category: "stdout",
            action: "write"
        }
    );
    assert_eq!(
        classify_path("std::io::stdin"),
        Classification::Capability {
            category: "stdin",
            action: "read"
        }
    );
    assert_eq!(
        classify_path("core::arch::global_asm"),
        Classification::BannedConstruct {
            construct: "inline_assembly"
        }
    );
    assert_eq!(
        classify_path("std::ffi::CString"),
        Classification::Capability {
            category: "ffi",
            action: "call"
        }
    );
    assert_eq!(
        classify_path("std::boxed::Box"),
        Classification::AllowedPure
    );
}

#[test]
fn test_error_displays() {
    use coding_adventures_ca_capability_analyzer::manifest::ManifestError;
    use coding_adventures_ca_capability_analyzer::scanner::ScanError;
    use coding_adventures_ca_capability_analyzer::AnalyzerError;

    let me1 = ManifestError::Io("not found".into());
    assert!(me1.to_string().contains("manifest I/O error"));

    let me2 = ManifestError::SizeLimitExceeded(2_000_000);
    assert!(me2.to_string().contains("exceeds 1 MiB limit"));

    let se1 = ScanError::FileLimitExceeded(10_000);
    assert!(se1
        .to_string()
        .contains("exceeded maximum source file count"));

    let se2 = ScanError::FileSizeLimitExceeded("big.rs".into(), 5_000_000);
    assert!(se2.to_string().contains("exceeds 4 MiB limit"));

    let se3 = ScanError::TotalSizeLimitExceeded(70_000_000);
    assert!(se3.to_string().contains("exceeds 64 MiB limit"));

    let ae1: AnalyzerError = me1.into();
    assert!(ae1.to_string().contains("manifest I/O error"));

    let ae2: AnalyzerError = se1.into();
    assert!(ae2
        .to_string()
        .contains("exceeded maximum source file count"));

    let ae3 = AnalyzerError::Parse("bad token".into());
    assert!(ae3.to_string().contains("parse error: bad token"));
}

#[test]
fn test_deep_json_and_escape_coverage() {
    let escapes = r#""escapes: \b \f \r \t \/ \" \\""#;
    assert!(JsonValue::parse(escapes).is_ok());

    // Unterminated array & object
    assert!(JsonValue::parse("[1, 2").is_err());
    assert!(JsonValue::parse("{\"key\"").is_err());
    assert!(JsonValue::parse("{\"key\": 1").is_err());
    assert!(JsonValue::parse("{\"key\" 1}").is_err());
    assert!(JsonValue::parse("[1 2]").is_err());

    // Depth limit
    let deep = "[".repeat(105) + &"]".repeat(105);
    assert!(JsonValue::parse(&deep).is_err());

    // Accessors
    let j_null = JsonValue::Null;
    assert_eq!(j_null.as_str(), None);
    assert_eq!(j_null.as_i64(), None);
    assert_eq!(j_null.as_array(), None);
    assert_eq!(j_null.as_object(), None);
}

#[test]
fn test_parser_advanced_constructs() {
    let src = r##"
        /* outer /* nested */ comment */
        let _ = br#"raw byte string"#;
        let _ = b"regular byte string";
        extern "system" fn sys_call() {}
        #[link_name = "secret_symbol"]
        extern "C" fn linked_call() {}
    "##;
    let res = scan_source("src/adv.rs", src).unwrap();
    assert_eq!(res.banned.len(), 1);
    assert_eq!(res.banned[0].construct, "foreign_linkage");
    assert_eq!(res.detections.len(), 2);
}

#[test]
fn test_capability_methods() {
    use coding_adventures_ca_capability_analyzer::manifest::Capability;

    let cap = Capability {
        category: "fs".into(),
        action: "read".into(),
        target: "data.txt".into(),
        justification: "Valid length justification".into(),
    };
    assert_eq!(cap.wildcard_string(), "fs:read:*");
    assert_eq!(cap.exact_string(), "fs:read:data.txt");
}

#[test]
fn test_additional_manifest_edge_cases() {
    // Non-object root
    assert!(test_manifest_string("[]").is_err());
    assert!(test_manifest_string("\"just a string\"").is_err());

    // Non-integer version
    assert!(test_manifest_string(r#"{"version": "one", "package": "rust/foo", "justification": "Valid length justification", "capabilities": []}"#).is_err());

    // Non-string package
    assert!(test_manifest_string(r#"{"version": 1, "package": 123, "justification": "Valid length justification", "capabilities": []}"#).is_err());

    // Non-string justification
    assert!(test_manifest_string(
        r#"{"version": 1, "package": "rust/foo", "justification": true, "capabilities": []}"#
    )
    .is_err());

    // Non-array capabilities
    assert!(test_manifest_string(r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": "none"}"#).is_err());

    // Capability item not an object
    assert!(test_manifest_string(r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [123]}"#).is_err());

    // Capability item with empty target
    assert!(test_manifest_string(r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [{"category": "fs", "action": "read", "target": "", "justification": "Valid length justification"}]}"#).is_err());

    // Banned construct exception not an array
    assert!(test_manifest_string(r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [], "banned_construct_exceptions": "invalid"}"#).is_err());

    // Banned construct exception item not an object
    assert!(test_manifest_string(r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [], "banned_construct_exceptions": [42]}"#).is_err());

    // Banned construct exception with short justification
    assert!(test_manifest_string(r#"{"version": 1, "package": "rust/foo", "justification": "Valid length justification", "capabilities": [], "banned_construct_exceptions": [{"construct": "unsafe_code", "language": "rust", "justification": "short"}]}"#).is_err());
}

#[test]
fn test_cli_error_paths() {
    // Missing argument to --dir
    let status_missing =
        std::process::Command::new(env!("CARGO_BIN_EXE_ca-capability-analyzer-rs"))
            .arg("--dir")
            .status()
            .expect("failed to execute binary");
    assert_eq!(status_missing.code(), Some(2));

    // Unrecognized argument
    let status_unrec = std::process::Command::new(env!("CARGO_BIN_EXE_ca-capability-analyzer-rs"))
        .arg("--bogus-flag")
        .status()
        .expect("failed to execute binary");
    assert_eq!(status_unrec.code(), Some(2));
}

#[test]
fn test_cli_execution() {
    let ct_compare_dir = PathBuf::from("../ct-compare");
    if ct_compare_dir.exists() {
        let status = std::process::Command::new(env!("CARGO_BIN_EXE_ca-capability-analyzer-rs"))
            .arg("--dir")
            .arg(&ct_compare_dir)
            .arg("--verbose")
            .status()
            .expect("failed to execute binary");
        assert!(status.success());
    }

    let help_status = std::process::Command::new(env!("CARGO_BIN_EXE_ca-capability-analyzer-rs"))
        .arg("--help")
        .status()
        .expect("failed to execute binary");
    assert!(help_status.success());
}

#[test]
fn test_analyze_self_package() {
    let self_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let report = analyze_package(&self_dir, true).expect("self analysis should succeed");
    assert_eq!(report.exit_code(), 0);
    assert!(report.violations.is_empty());
    assert!(!report.detected_capabilities.is_empty());
}

#[test]
fn test_evasive_whitespace_and_comments_in_paths() {
    let src = r#"
        fn sneaky() {
            let _ = std :: fs :: read_to_string("hello");
            let _ = std /* comment */ :: /* comment */ net :: TcpListener::bind("0.0.0.0:80");
        }
    "#;
    let res = scan_source("sneaky.rs", src).unwrap();
    assert!(res.detections.iter().any(|d| d.category == "fs" && d.action == "read"));
    assert!(res.detections.iter().any(|d| d.category == "net" && d.action == "listen"));
}

#[test]
fn test_leading_colon_paths() {
    let src = r#"
        fn root_path() {
            let _ = ::std::fs::write("test.txt", b"abc");
        }
    "#;
    let res = scan_source("root.rs", src).unwrap();
    assert!(res.detections.iter().any(|d| d.category == "fs" && d.action == "write"));
}

#[test]
fn test_extern_fn_and_foreign_blocks_and_export_name() {
    let src = r#"
        extern fn foreign_func() {}
        extern {
            fn foreign_block_func();
        }
        #[export_name = "exported_symbol"]
        pub fn exported() {}
    "#;
    let res = scan_source("foreign.rs", src).unwrap();
    let ffi_calls: Vec<_> = res.detections.iter().filter(|d| d.category == "ffi" && d.action == "call").collect();
    assert_eq!(ffi_calls.len(), 2);
    let linkage_banned: Vec<_> = res.banned.iter().filter(|b| b.construct == "foreign_linkage").collect();
    assert_eq!(linkage_banned.len(), 1);
    assert!(linkage_banned[0].evidence.contains("export_name"));
}

#[test]
fn test_banned_construct_unknown_property_rejection() {
    let json = r#"{
        "version": 1,
        "package": "rust/test-pkg",
        "capabilities": [],
        "justification": "Test package with invalid banned exception property.",
        "banned_construct_exceptions": [
            {
                "construct": "unsafe_code",
                "language": "rust",
                "justification": "Audited memory barrier implementation.",
                "fingerprint_bogus": "unauthorized-extra-field"
            }
        ]
    }"#;
    let res = test_manifest_string(json);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("unknown property 'fingerprint_bogus'"));
}

#[test]
fn test_scanner_depth_limit() {
    use coding_adventures_ca_capability_analyzer::scanner::{discover_sources, ScanError, MAX_DIR_DEPTH};
    let root = tempfile_dir("depth_limit");
    let mut curr = root.clone();
    for i in 0..(MAX_DIR_DEPTH + 2) {
        curr = curr.join(format!("level_{i}"));
    }
    fs::create_dir_all(&curr).unwrap();
    fs::write(curr.join("deep.rs"), "pub fn deep() {}").unwrap();

    let res = discover_sources(&root);
    assert!(matches!(res, Err(ScanError::DepthLimitExceeded(_))));

    fs::remove_dir_all(&root).ok();
}

// ===========================================================================
// Helpers
// ===========================================================================

use std::sync::atomic::{AtomicUsize, Ordering};
static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tempfile_dir(suffix: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "test_ca_analyzer_{suffix}_{}_{id}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_manifest_string(json: &str) -> Result<ManifestData, String> {
    let tmp = tempfile_dir("manifest_str");
    fs::write(tmp.join("required_capabilities.json"), json).unwrap();
    let res = load_manifest(&tmp).map_err(|e| e.to_string());
    fs::remove_dir_all(&tmp).ok();
    res
}

#[test]
fn test_windows_path_normalization() {
    let raw_win_path = r"src\sub\module.rs";
    let normalized = raw_win_path.replace('\\', "/");
    assert_eq!(normalized, "src/sub/module.rs");
}

#[test]
fn test_sha256_full_suite() {
    use coding_adventures_ca_capability_analyzer::sha256::{sha256, sha256_hex, Sha256};

    // Standard NIST test vectors
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );

    // Multi-block and streaming equivalence
    let long_data = vec![0x61u8; 1_000_000]; // 1,000,000 'a's
    let mut streaming = Sha256::new();
    for chunk in long_data.chunks(1024) {
        streaming.update(chunk);
    }
    let stream_digest = streaming.finalize();
    let direct_digest = sha256(&long_data);
    assert_eq!(stream_digest, direct_digest);
    assert_eq!(
        sha256_hex(&long_data),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn test_module_generator_pure() {
    use coding_adventures_ca_capability_analyzer::generator::{
        check_module_on_disk, write_module_to_disk,
    };

    let tmp = tempfile_dir("module_pure");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn foo() {}").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/pure-crate",
        "capabilities": [],
        "justification": "Pure crate with no ambient effects."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();

    // Module does not exist yet
    assert!(!check_module_on_disk(&tmp).unwrap());

    // Write module
    write_module_to_disk(&tmp).unwrap();
    assert!(check_module_on_disk(&tmp).unwrap());

    let content = fs::read_to_string(tmp.join("src").join("capabilities.rs")).unwrap();
    assert!(content.contains("PureCapabilities"));
    assert!(content.contains("#![deny(unsafe_code)]"));
    assert!(content.contains("Package: rust/pure-crate"));

    // Modify module to simulate drift
    fs::write(tmp.join("src").join("capabilities.rs"), "// drifted content").unwrap();
    assert!(!check_module_on_disk(&tmp).unwrap());

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_module_generator_capabilities() {
    use coding_adventures_ca_capability_analyzer::generator::{
        check_module_on_disk, write_module_to_disk,
    };

    let tmp = tempfile_dir("module_caps");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn foo() {}").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/caps-crate",
        "capabilities": [
            {
                "category": "stdout",
                "action": "write",
                "target": "*",
                "justification": "Writing diagnostic messages to standard output."
            },
            {
                "category": "fs",
                "action": "read",
                "target": "cache/*",
                "justification": "Reading cached configuration files."
            }
        ],
        "justification": "Capability-bearing crate with file read and stdout write."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();

    write_module_to_disk(&tmp).unwrap();
    assert!(check_module_on_disk(&tmp).unwrap());

    let content = fs::read_to_string(tmp.join("src").join("capabilities.rs")).unwrap();
    assert!(content.contains("PackageCapabilities"));
    assert!(content.contains("StdoutWriteAllCap"));
    assert!(content.contains("FsReadCacheCap"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_generation_and_verification() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, parse_certificate, verify_certificate,
    };

    let tmp = tempfile_dir("cert_flow");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(
        tmp.join("src").join("lib.rs"),
        "pub fn calculate(a: u32, b: u32) -> u32 { a + b }\npub const fn zero() -> u32 { 0 }",
    ).unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/cert-test",
        "capabilities": [],
        "justification": "Pure mathematical calculation crate."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let cert = generate_certificate(&tmp).unwrap();
    assert_eq!(cert.package, "rust/cert-test");
    assert_eq!(cert.label, "proved");
    assert_eq!(cert.entry_points.len(), 2);
    assert_eq!(cert.declared_effects.len(), 0);

    let cert_json = cert.to_json_string();
    let cert_path = tmp.join("effect_certificate.json");
    fs::write(&cert_path, &cert_json).unwrap();

    let parsed = parse_certificate(&cert_json).unwrap();
    assert_eq!(parsed.package, cert.package);
    assert_eq!(parsed.source_digest, cert.source_digest);

    // Verify passes cleanly
    let verify_res = verify_certificate(&tmp, &cert_path).unwrap();
    assert_eq!(verify_res.package, "rust/cert-test");
    assert_eq!(verify_res.label, "proved");
    assert_eq!(verify_res.entry_points_count, 2);

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_sabotage_source_digest() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, verify_certificate,
    };

    let tmp = tempfile_dir("cert_sabotage_src");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn op() -> u32 { 42 }").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/sabotage-src",
        "capabilities": [],
        "justification": "Test package for source digest sabotage detection."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let cert = generate_certificate(&tmp).unwrap();
    let cert_path = tmp.join("effect_certificate.json");
    fs::write(&cert_path, cert.to_json_string()).unwrap();

    // Verify passes initially
    assert!(verify_certificate(&tmp, &cert_path).is_ok());

    // Tamper with source code
    fs::write(tmp.join("src").join("lib.rs"), "pub fn op() -> u32 { 43 }").unwrap();

    // Verification must fail with digest mismatch
    let err = verify_certificate(&tmp, &cert_path).unwrap_err();
    assert!(err.contains("source_digest mismatch"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_sabotage_manifest_digest() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, verify_certificate,
    };

    let tmp = tempfile_dir("cert_sabotage_manifest");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn op() {}").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/sabotage-manifest",
        "capabilities": [],
        "justification": "Test package for manifest digest sabotage detection."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let cert = generate_certificate(&tmp).unwrap();
    let cert_path = tmp.join("effect_certificate.json");
    fs::write(&cert_path, cert.to_json_string()).unwrap();

    assert!(verify_certificate(&tmp, &cert_path).is_ok());

    // Tamper with manifest file
    let tampered_manifest = r#"{
        "version": 1,
        "package": "rust/sabotage-manifest",
        "capabilities": [],
        "justification": "Tampered justification text here."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), tampered_manifest).unwrap();

    let err = verify_certificate(&tmp, &cert_path).unwrap_err();
    assert!(err.contains("manifest_digest mismatch"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_sabotage_uncertified_entry_point() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, verify_certificate,
    };

    let tmp = tempfile_dir("cert_sabotage_entry");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn legitimate() {}").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/sabotage-entry",
        "capabilities": [],
        "justification": "Test package for uncertified entry point detection."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let mut cert = generate_certificate(&tmp).unwrap();
    // Tamper certificate: remove entry points
    cert.entry_points.clear();
    let cert_path = tmp.join("effect_certificate.json");
    fs::write(&cert_path, cert.to_json_string()).unwrap();

    let err = verify_certificate(&tmp, &cert_path).unwrap_err();
    assert!(err.contains("mismatch") || err.contains("Uncertified"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_sabotage_pure_claiming_effects() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, verify_certificate,
    };

    let tmp = tempfile_dir("cert_sabotage_pure");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn pure_fn() {}").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/sabotage-pure",
        "capabilities": [],
        "justification": "Test package for pure claiming effects."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let mut cert = generate_certificate(&tmp).unwrap();
    // Tamper certificate: pure package entry point claims side effect
    cert.entry_points[0].effects.push("fs:read:*".to_string());
    let cert_path = tmp.join("effect_certificate.json");
    fs::write(&cert_path, cert.to_json_string()).unwrap();

    let err = verify_certificate(&tmp, &cert_path).unwrap_err();
    assert!(err.contains("must equal declared_effects") || err.contains("claims non-empty effects"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_sabotage_dishonest_label() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, verify_certificate,
    };

    let tmp = tempfile_dir("cert_sabotage_label");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn f() {}").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/sabotage-label",
        "capabilities": [],
        "justification": "Test package for dishonest label detection."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let mut cert = generate_certificate(&tmp).unwrap();
    cert.label = "unverified-passed".to_string();
    let cert_path = tmp.join("effect_certificate.json");
    fs::write(&cert_path, cert.to_json_string()).unwrap();

    let err = verify_certificate(&tmp, &cert_path).unwrap_err();
    assert!(err.contains("Invalid certificate label 'unverified-passed'"));

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_cli_module_and_cert_flags() {
    let tmp = tempfile_dir("cli_flags");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn compute() -> u32 { 10 }").unwrap();

    let manifest_json = r#"{
        "version": 1,
        "package": "rust/cli-test",
        "capabilities": [],
        "justification": "Package for testing CLI module and certificate flags."
    }"#;
    fs::write(tmp.join("required_capabilities.json"), manifest_json).unwrap();

    let bin = env!("CARGO_BIN_EXE_ca-capability-analyzer-rs");

    // 1. --check-module fails before generation
    let status_before = std::process::Command::new(bin)
        .arg("--dir")
        .arg(&tmp)
        .arg("--check-module")
        .status()
        .unwrap();
    assert_eq!(status_before.code(), Some(1));

    // 2. --generate-module succeeds
    let status_gen = std::process::Command::new(bin)
        .arg("--dir")
        .arg(&tmp)
        .arg("--generate-module")
        .status()
        .unwrap();
    assert!(status_gen.success());

    // 3. --check-module succeeds now
    let status_check = std::process::Command::new(bin)
        .arg("--dir")
        .arg(&tmp)
        .arg("--check-module")
        .status()
        .unwrap();
    assert!(status_check.success());

    // 4. --emit-cert succeeds
    let cert_file = tmp.join("cert.json");
    let status_emit = std::process::Command::new(bin)
        .arg("--dir")
        .arg(&tmp)
        .arg("--emit-cert")
        .arg(&cert_file)
        .status()
        .unwrap();
    assert!(status_emit.success());
    assert!(cert_file.exists());

    // 5. --verify-cert succeeds
    let status_verify = std::process::Command::new(bin)
        .arg("--dir")
        .arg(&tmp)
        .arg("--verify-cert")
        .arg(&cert_file)
        .status()
        .unwrap();
    assert!(status_verify.success());

    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_certificate_and_generator_edge_cases() {
    use coding_adventures_ca_capability_analyzer::certificate::{
        generate_certificate, parse_certificate, verify_certificate,
        BannedExceptionCert, EffectCertificate, EntryPointCert,
    };
    use coding_adventures_ca_capability_analyzer::generator::generate_module_source;

    // 1. Test None manifest for generator
    let none_source = generate_module_source("none-pkg", None);
    assert!(none_source.contains("PureCapabilities"));

    // 2. Full EffectCertificate JSON serialization with all non-empty fields
    let full_cert = EffectCertificate {
        schema_version: "ver24-effect-cert-v1".to_string(),
        package: "rust/full-pkg".to_string(),
        source_digest: "abcd".to_string(),
        manifest_digest: "1234".to_string(),
        declared_effects: vec!["stdout:write:*".to_string(), "fs:read:cache/*".to_string()],
        entry_points: vec![EntryPointCert {
            name: "log_message".to_string(),
            signature: "pub fn log_message(msg: &str)".to_string(),
            effects: vec!["stdout:write:*".to_string()],
        }],
        unconditional_roots: vec!["init".to_string()],
        banned_construct_exceptions: vec![BannedExceptionCert {
            construct: "unsafe_code".to_string(),
            language: "rust".to_string(),
            justification: "Checked volatile barrier".to_string(),
        }],
        label: "proved".to_string(),
    };
    let json = full_cert.to_json_string();
    assert!(json.contains("declared_effects"));
    assert!(json.contains("unconditional_roots"));
    assert!(json.contains("banned_construct_exceptions"));

    let parsed = parse_certificate(&json).unwrap();
    assert_eq!(parsed.declared_effects.len(), 2);
    assert_eq!(parsed.unconditional_roots.len(), 1);
    assert_eq!(parsed.banned_construct_exceptions.len(), 1);

    // 3. parse_certificate error branches
    assert!(parse_certificate(r#"{"package": "x"}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "wrong"}"#).unwrap_err().contains("Unsupported certificate schema_version"));
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1"}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p"}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s"}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m"}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "declared_effects": [123]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "entry_points": [{}]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "entry_points": [{"name": "n"}]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "entry_points": [{"name": "n", "signature": "s", "effects": [456]}]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "unconditional_roots": [789]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "banned_construct_exceptions": [{}]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "banned_construct_exceptions": [{"construct": "c"}]}"#).is_err());
    assert!(parse_certificate(r#"{"schema_version": "ver24-effect-cert-v1", "package": "p", "source_digest": "s", "manifest_digest": "m", "label": "proved", "banned_construct_exceptions": [{"construct": "c", "language": "l"}]}"#).is_err());

    // 4. verify_certificate error conditions
    let tmp = tempfile_dir("cert_errors");
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("lib.rs"), "pub fn test_fn() {}\n").unwrap();
    fs::write(tmp.join("required_capabilities.json"), r#"{"version": 1, "package": "rust/tmp", "capabilities": [], "justification": "Testing cert errors."}"#).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();

    let cert = generate_certificate(&tmp).unwrap();
    let cert_file = tmp.join("cert.json");
    fs::write(&cert_file, cert.to_json_string()).unwrap();

    // Non-existent cert path
    assert!(verify_certificate(&tmp, &tmp.join("nonexistent.json")).unwrap_err().contains("does not exist"));

    // Package name mismatch
    let mut bad_pkg_cert = cert.clone();
    bad_pkg_cert.package = "rust/mismatch".to_string();
    let bad_pkg_path = tmp.join("bad_pkg.json");
    fs::write(&bad_pkg_path, bad_pkg_cert.to_json_string()).unwrap();
    assert!(verify_certificate(&tmp, &bad_pkg_path).unwrap_err().contains("does not match manifest package"));

    // Declared effects mismatch
    let mut bad_eff_cert = cert.clone();
    bad_eff_cert.declared_effects = vec!["stdout:write:*".to_string()];
    let bad_eff_path = tmp.join("bad_eff.json");
    fs::write(&bad_eff_path, bad_eff_cert.to_json_string()).unwrap();
    assert!(verify_certificate(&tmp, &bad_eff_path).unwrap_err().contains("declared_effects mismatch"));

    // Entry point undeclared effect
    let mut bad_ep_cert = cert.clone();
    bad_ep_cert.declared_effects = vec!["stdout:write:*".to_string()];
    bad_ep_cert.entry_points[0].effects = vec!["fs:read:*".to_string()];
    // create manifest matching declared_effects so we get past step 3
    fs::write(tmp.join("required_capabilities.json"), r#"{"version": 1, "package": "rust/tmp", "capabilities": [{"category": "stdout", "action": "write", "target": "*", "justification": "stdout write justification."}], "justification": "Testing cert errors."}"#).unwrap();
    coding_adventures_ca_capability_analyzer::generator::write_module_to_disk(&tmp).unwrap();
    bad_ep_cert.manifest_digest = coding_adventures_ca_capability_analyzer::certificate::compute_manifest_digest(&tmp).unwrap();
    let bad_ep_path = tmp.join("bad_ep.json");
    fs::write(&bad_ep_path, bad_ep_cert.to_json_string()).unwrap();
    assert!(verify_certificate(&tmp, &bad_ep_path).unwrap_err().contains("must equal declared_effects"));

    // Package static analysis failure
    fs::write(tmp.join("src").join("lib.rs"), "pub fn test_fn() { std::fs::read(\"bad\").ok(); }\n").unwrap();
    // A package the analyzer rejects must not receive a certificate at all.
    let refusal = generate_certificate(&tmp).unwrap_err();
    assert!(refusal.contains("refusing to emit certificate"), "{refusal}");

    fs::remove_dir_all(&tmp).ok();
}
