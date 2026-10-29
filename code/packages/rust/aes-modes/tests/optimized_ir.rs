//! Release-codegen regression gate for the fixed-memory GHASH boundary.
//!
//! The source uses fixed loops, masked field choices, and wipe-on-drop owners.
//! This test asks the active compiler for optimized LLVM IR and verifies that
//! the exported multiplication retains only its public fixed-count loop branch,
//! neither fixed boundary reaches an allocator, caller output is ABI-owned, and
//! the product, multiplier, accumulator, and padded block reach the repository
//! zeroize primitive on normal and unwind paths.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MULTIPLY_SYMBOL: &str = "coding_adventures_ghash_multiply_fixed_ir_audit";
const GHASH_SYMBOL: &str = "coding_adventures_ghash_fixed_ir_audit";

#[test]
fn optimized_ghash_is_fixed_memory_masked_and_scrubbed() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("AES modes package must be inside the Rust workspace");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "coding-adventures-ghash-ir-{}-{unique}",
        std::process::id()
    ));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let compilation = Command::new(cargo)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "rustc",
            "-p",
            "coding_adventures_aes_modes",
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
                        .starts_with("coding_adventures_aes_modes-")
                })
        })
        .expect("Cargo must emit AES modes LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let multiply = exported_function_body(&ir, MULTIPLY_SYMBOL);
    let ghash = exported_function_body(&ir, GHASH_SYMBOL);
    let absorb = rust_function_body(&ir, "ghash_absorb");
    for function in [multiply, ghash, absorb] {
        assert_no_allocator(function);
    }

    assert!(
        multiply.lines().next().is_some_and(|line| {
            line.starts_with("define void ") && line.contains("%output")
        }),
        "multiplication must write to caller-owned output rather than return a product:\n{multiply}"
    );
    let conditional_branches: Vec<&str> = multiply
        .lines()
        .filter(|line| line.trim_start().starts_with("br i1 "))
        .collect();
    assert_eq!(
        conditional_branches.len(),
        1,
        "multiplication must retain only its fixed-count loop branch:\n{multiply}"
    );
    assert_fixed_128_round_loop(multiply, conditional_branches[0]);
    assert!(
        !multiply
            .lines()
            .any(|line| line.trim_start().starts_with("switch ")),
        "multiplication must not introduce secret-derived switch control flow:\n{multiply}"
    );
    assert_eq!(alloca_names(multiply, 16, 16), ["%v", "%z"]);
    assert_eq!(zeroize_calls(multiply, "%v", 16), 1);
    assert_eq!(zeroize_calls(multiply, "%z", 16), 2);
    assert_output_precedes_wipes(multiply, &["%v", "%z"]);

    assert_eq!(alloca_names(ghash, 16, 16), ["%block", "%accumulator"]);
    assert!(
        ghash.contains(MULTIPLY_SYMBOL),
        "complete GHASH must use the audited multiplication boundary:\n{ghash}"
    );
    assert!(zeroize_calls(ghash, "%block", 16) >= 3);
    assert!(zeroize_calls(ghash, "%accumulator", 16) >= 2);
    assert_output_precedes_wipes(ghash, &["%block", "%accumulator"]);
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

fn assert_fixed_128_round_loop(function: &str, branch: &str) {
    let condition = branch
        .trim()
        .strip_prefix("br i1 ")
        .and_then(|rest| rest.split(',').next())
        .expect("conditional branch must expose its condition");
    let comparison = definition_line(function, condition);
    assert!(
        comparison.contains("icmp eq i64") && comparison.ends_with(", 128"),
        "multiplication exit must compare its induction value with 128:\n{function}"
    );
    let induction_next = comparison
        .split("icmp eq i64 ")
        .nth(1)
        .and_then(|operands| operands.split(',').next())
        .expect("loop comparison must expose its induction value");
    let increment = definition_line(function, induction_next);
    assert!(
        increment.contains(" add ") && increment.ends_with(", 1"),
        "multiplication induction value must advance by exactly one:\n{function}"
    );
    let induction = increment
        .split(" i64 ")
        .nth(1)
        .and_then(|operands| operands.split(',').next())
        .expect("loop increment must expose its previous induction value");
    let phi = definition_line(function, induction);
    assert!(
        phi.contains(" phi i64 [ 0,") && phi.contains(&format!("[ {induction_next},")),
        "multiplication induction variable must start at zero and feed back only its increment:\n{function}"
    );
}

fn definition_line<'a>(function: &'a str, value: &str) -> &'a str {
    let prefix = format!("{value} = ");
    function
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("optimized function did not define {value}:\n{function}"))
}

fn assert_output_precedes_wipes(function: &str, owners: &[&str]) {
    let lines: Vec<&str> = function.lines().collect();
    let copy = lines
        .iter()
        .position(|line| line.contains("memcpy") && line.contains("%output"))
        .unwrap_or_else(|| panic!("fixed boundary must copy into caller output:\n{function}"));
    for owner in owners {
        assert!(
            lines[copy + 1..].iter().any(|line| is_call(line)
                && line.contains("zeroize_bytes")
                && line.contains(owner)),
            "fixed boundary must wipe {owner} after its output copy:\n{function}"
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
    assert_no_allocator("define void @fixed() { call ptr @__rust_alloc(i64 16) }");
}

#[test]
fn call_parser_ignores_ir_comments() {
    assert!(!is_call("; call zeroize_bytes"));
    assert!(is_call("  invoke void @zeroize_bytes()"));
}
