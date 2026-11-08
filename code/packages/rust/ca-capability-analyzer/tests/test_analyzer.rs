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
    let ct_compare_dir = PathBuf::from("../ct-compare");
    if ct_compare_dir.exists() {
        let report = analyze_package(&ct_compare_dir, true).unwrap();
        assert!(
            report.is_clean(),
            "ct-compare should pass capability check: {:?}",
            report.violations
        );
        assert_eq!(report.exit_code(), 0);
        assert!(report.scanned_files_count >= 1);
    }
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
