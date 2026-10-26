//! Release-codegen regression gate for the constant-work equality boundary.
//!
//! Functional tests prove equality results, but cannot prove that an optimizer
//! retained every input read. This test asks the active Rust compiler for
//! optimization-level-3 LLVM IR and verifies that the common equal-length scan
//! still contains the volatile operations which make each iteration observable.

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn optimized_equal_length_scan_retains_volatile_operations() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = manifest_dir.join("src/lib.rs");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let output = std::env::temp_dir().join(format!(
        "coding-adventures-ct-compare-{}-{unique}.ll",
        std::process::id()
    ));
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());

    let compilation = Command::new(rustc)
        .arg("--crate-name=ct_compare_codegen_fixture")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg("-Copt-level=3")
        .arg("--emit=llvm-ir")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("rustc must be available to the Cargo test process");

    assert!(
        compilation.status.success(),
        "optimized IR compilation failed:\n{}",
        String::from_utf8_lossy(&compilation.stderr)
    );

    let ir = fs::read_to_string(&output).expect("rustc must emit readable LLVM IR");
    fs::remove_file(&output).expect("temporary LLVM IR should be removable");
    let scan = function_body(&ir, "ct_eq_equal_length17h");

    assert!(
        scan.matches("load volatile i8").count() >= 3,
        "optimized equality scan must retain volatile reads of both inputs and the accumulator"
    );
    assert!(
        scan.contains("store volatile i8"),
        "optimized equality scan must retain a volatile accumulator write"
    );
    assert!(
        !scan.contains("memcmp") && !scan.contains("bcmp"),
        "optimized equality scan must not lower to an early-exit comparison routine"
    );
}

fn function_body<'a>(ir: &'a str, symbol_fragment: &str) -> &'a str {
    let symbol = ir
        .find(symbol_fragment)
        .unwrap_or_else(|| panic!("optimized IR did not contain {symbol_fragment}"));
    let definition = ir[..symbol]
        .rfind("define ")
        .expect("symbol must occur in a function definition");
    let end = ir[symbol..]
        .find("\n}")
        .map(|offset| symbol + offset + 2)
        .expect("function definition must have a closing brace");
    &ir[definition..end]
}
