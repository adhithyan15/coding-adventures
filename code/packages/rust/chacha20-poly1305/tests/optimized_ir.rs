//! Release-codegen regression gate for the fixed-memory ChaCha20 boundary.
//!
//! Source ownership alone cannot prove that optimized code retains the wipes
//! for the reusable keystream and both key-derived word states. This test asks
//! the active Rust compiler for optimized LLVM IR and checks the exact exported
//! boundary, block helper, and word-state drop path. It deliberately makes no
//! claim about the allocation-returning compatibility APIs or Poly1305.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const EXPORTED_SYMBOL: &str = "coding_adventures_chacha20_xor_fixed_ir_audit";
const WORD_WIPE_SYMBOL: &str = "coding_adventures_chacha20_secret_words_wipe_ir_audit";

#[test]
fn optimized_fixed_boundary_scrubs_secret_owners_without_allocating() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("ChaCha20 package must be inside the Rust workspace");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "coding-adventures-chacha20-fixed-ir-{}-{unique}",
        std::process::id()
    ));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let compilation = Command::new(cargo)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "rustc",
            "-p",
            "coding_adventures_chacha20_poly1305",
            "--release",
            "--lib",
            "--",
            "--emit=llvm-ir",
        ])
        .output()
        .expect("Cargo must be available to the test process");

    assert!(
        compilation.status.success(),
        "optimized IR compilation failed:\n{}",
        String::from_utf8_lossy(&compilation.stderr)
    );

    let deps_dir = target_dir.join("release/deps");
    let ir_path = fs::read_dir(&deps_dir)
        .expect("release dependency directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|extension| extension == "ll")
                && path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("coding_adventures_chacha20_poly1305-")
                })
        })
        .expect("Cargo must emit ChaCha20 LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let fixed = exported_function_body(&ir, EXPORTED_SYMBOL);
    let block = rust_function_body(&ir, "chacha20_block_into");
    let word_wipe = exported_function_body(&ir, WORD_WIPE_SYMBOL);

    for function in [fixed, block, word_wipe] {
        assert_no_allocator(function);
    }

    assert_eq!(
        alloca_names(fixed, 64, 1),
        ["%keystream"],
        "fixed boundary must have exactly one reusable 64-byte keystream owner:\n{fixed}"
    );
    assert!(
        fixed.contains("chacha20_block_into"),
        "fixed boundary must derive blocks through the audited helper:\n{fixed}"
    );
    assert_eq!(
        zeroize_calls(fixed, "%keystream", 64),
        2,
        "keystream owner must be scrubbed on normal and unwind paths:\n{fixed}"
    );

    let state_drop_calls = block
        .lines()
        .filter(|line| is_call(line) && line.contains(WORD_WIPE_SYMBOL))
        .count();
    assert!(
        state_drop_calls >= 3,
        "block helper must scrub both word states and its unwind path:\n{block}"
    );

    let word_wipes = word_wipe
        .lines()
        .filter(|line| is_call(line) && line.contains("zeroize"))
        .count();
    assert_eq!(
        word_wipes, 16,
        "word-state wipe must scrub all 16 words:\n{word_wipe}"
    );
}

fn assert_no_allocator(function: &str) {
    for forbidden in [
        "__rust_alloc",
        "__rust_realloc",
        "alloc::alloc",
        "exchange_malloc",
        "RawVec",
    ] {
        assert!(
            !function.contains(forbidden),
            "fixed-memory boundary unexpectedly references allocator symbol {forbidden}:\n{function}"
        );
    }
}

fn alloca_names(function: &str, size: usize, alignment: usize) -> Vec<&str> {
    let suffix = format!(" = alloca [{size} x i8], align {alignment}");
    function
        .lines()
        .filter_map(|line| line.trim().strip_suffix(&suffix))
        .collect()
}

fn zeroize_calls(function: &str, owner: &str, size: usize) -> usize {
    let size_marker = format!("i64 noundef {size}");
    function
        .lines()
        .filter(|line| {
            is_call(line)
                && line.contains("zeroize_bytes")
                && line.contains(owner)
                && line.contains(&size_marker)
        })
        .count()
}

fn is_call(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("call ") || line.starts_with("tail call ") || line.starts_with("invoke ")
}

fn rust_function_body<'a>(ir: &'a str, rust_name: &str) -> &'a str {
    let definition = ir
        .match_indices("define ")
        .find_map(|(offset, _)| {
            let signature_end = ir[offset..]
                .find('\n')
                .map_or(ir.len(), |line_end| offset + line_end);
            ir[offset..signature_end]
                .contains(rust_name)
                .then_some(offset)
        })
        .unwrap_or_else(|| panic!("optimized IR did not identify Rust function {rust_name}"));
    function_body_from(ir, definition)
}

fn exported_function_body<'a>(ir: &'a str, symbol: &str) -> &'a str {
    let marker = format!("@{symbol}(");
    let definition = ir
        .match_indices("define ")
        .find_map(|(offset, _)| {
            let signature_end = ir[offset..]
                .find('\n')
                .map_or(ir.len(), |line_end| offset + line_end);
            ir[offset..signature_end]
                .contains(&marker)
                .then_some(offset)
        })
        .unwrap_or_else(|| panic!("optimized IR did not identify exported symbol {symbol}"));
    function_body_from(ir, definition)
}

fn function_body_from(ir: &str, definition: usize) -> &str {
    let end = ir[definition..]
        .find("\n}")
        .map(|offset| definition + offset + 2)
        .expect("optimized function must have a closing brace");
    &ir[definition..end]
}

#[test]
#[should_panic(expected = "unexpectedly references allocator symbol")]
fn allocator_gate_rejects_allocating_fixture() {
    assert_no_allocator("define void @fixed() { call ptr @__rust_alloc(i64 64) }");
}

#[test]
fn call_parser_ignores_ir_comments() {
    assert!(!is_call("; call zeroize_bytes"));
    assert!(is_call("  invoke void @zeroize_bytes()"));
}
