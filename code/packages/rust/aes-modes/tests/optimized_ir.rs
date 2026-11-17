//! Release-codegen regression gate for the fixed-memory GHASH and AES-128-GCM
//! boundaries.
//!
//! The source uses fixed loops, masked field choices, and wipe-on-drop owners.
//! This test asks the active compiler for optimized LLVM IR and verifies that
//! the exported multiplication retains only its public fixed-count loop branch,
//! no fixed boundary reaches an allocator, caller output is ABI-owned, tag
//! comparison has no conditional control flow, plaintext is written only after
//! authentication, and every key-derived scratch owner reaches the repository
//! zeroize primitive on normal and unwind paths.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MULTIPLY_SYMBOL: &str = "coding_adventures_ghash_multiply_fixed_ir_audit";
const GHASH_SYMBOL: &str = "coding_adventures_ghash_fixed_ir_audit";
const AES128_BLOCK_SYMBOL: &str = "coding_adventures_aes128_encrypt_fixed_ir_audit";
const AES128_GCM_ENCRYPT_SYMBOL: &str = "coding_adventures_aes128_gcm_encrypt_fixed_ir_audit";
const AES128_GCM_DECRYPT_SYMBOL: &str = "coding_adventures_aes128_gcm_decrypt_fixed_ir_audit";
const AES128_GCM_TAG_EQUAL_SYMBOL: &str = "coding_adventures_aes128_gcm_tag_equal_ir_audit";

/// Finds the optimized LLVM IR file Cargo emitted for the library under test.
///
/// Where Cargo parks `--emit=llvm-ir` output depends on its build-dir layout.
/// Through Cargo 1.97 it was `<target>/release/deps/<crate>-<hash>.ll`; Cargo
/// 1.100 uses the new layout, `<target>/release/build/<crate>/<hash>/out/
/// <crate>-<hash>.ll`, and `release/deps` no longer exists. The gate cares
/// only about the IR itself, so search the whole release tree (skipping the
/// incremental cache, which holds no `.ll` files) rather than hard-coding
/// either layout. Candidates are sorted so the choice is deterministic.
fn find_llvm_ir(release_dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut pending = vec![release_dir.to_path_buf()];
    let mut found = Vec::new();
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "incremental") {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "ll")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found.into_iter().next()
}

#[test]
fn optimized_ghash_and_aes128_gcm_are_fixed_memory_and_scrubbed() {
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

    let ir_path = find_llvm_ir(&target_dir.join("release"), "coding_adventures_aes_modes-")
        .expect("Cargo must emit AES modes LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let multiply = exported_function_body(&ir, MULTIPLY_SYMBOL);
    let ghash = exported_function_body(&ir, GHASH_SYMBOL);
    let absorb = rust_function_body(&ir, "ghash_absorb");
    let ghash_validated = rust_function_body(&ir, "ghash_fixed_validated");
    let gcm_encrypt = exported_function_body(&ir, AES128_GCM_ENCRYPT_SYMBOL);
    let gcm_decrypt = exported_function_body(&ir, AES128_GCM_DECRYPT_SYMBOL);
    let gcm_ctr = rust_function_body(&ir, "aes128_gcm_ctr_xor_fixed");
    let gcm_tag = rust_function_body(&ir, "aes128_gcm_tag_fixed");
    let gcm_tag_equal = exported_function_body(&ir, AES128_GCM_TAG_EQUAL_SYMBOL);
    for function in [
        multiply,
        ghash,
        absorb,
        ghash_validated,
        gcm_encrypt,
        gcm_decrypt,
        gcm_ctr,
        gcm_tag,
        gcm_tag_equal,
    ] {
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
    assert_eq!(alloca_names(multiply, 16), ["%v", "%z"]);
    // Unwind lowering can duplicate cleanup calls. Require the established
    // wipes without rejecting an additional cleanup wipe on another target.
    assert!(zeroize_calls(multiply, "%v", 16) >= 1);
    assert!(zeroize_calls(multiply, "%z", 16) >= 2);
    assert_eq!(alloca_names(multiply, 1), ["%bit_mask"]);
    assert!(zeroize_calls(multiply, "%bit_mask", 1) >= 1);
    assert_output_precedes_wipes(multiply, &["%v", "%z", "%bit_mask"]);

    assert!(alloca_names(ghash, 16).is_empty());
    assert!(
        ghash.contains("ghash_fixed_validated"),
        "public GHASH must delegate to its prevalidated fixed-memory core:\n{ghash}"
    );

    assert_eq!(
        alloca_names(ghash_validated, 16),
        ["%accumulator", "%block"]
    );
    assert!(zeroize_calls(ghash_validated, "%block", 16) >= 3);
    assert!(zeroize_calls(ghash_validated, "%accumulator", 16) >= 2);
    assert_output_precedes_wipes(ghash_validated, &["%block", "%accumulator"]);

    for (name, function) in [("encrypt", gcm_encrypt), ("decrypt", gcm_decrypt)] {
        assert_eq!(
            alloca_names(function, 16),
            ["%computed_tag", "%hash_subkey"],
            "fixed AES-128-GCM {name} must own only its hash subkey and computed tag:\n{function}"
        );
        assert!(
            zeroize_calls(function, "%hash_subkey", 16) >= 2,
            "fixed AES-128-GCM {name} must scrub the hash subkey on success and unwind:\n{function}"
        );
        assert!(
            zeroize_calls(function, "%computed_tag", 16) >= 2,
            "fixed AES-128-GCM {name} must scrub the computed tag on success and unwind:\n{function}"
        );
        assert!(
            function.contains(AES128_BLOCK_SYMBOL),
            "fixed AES-128-GCM {name} must derive H through audited AES-128:\n{function}"
        );
        assert!(
            function.contains("aes128_gcm_tag_fixed"),
            "fixed AES-128-GCM {name} must use the audited tag composition:\n{function}"
        );
    }

    let compare_position = gcm_decrypt
        .find(AES128_GCM_TAG_EQUAL_SYMBOL)
        .expect("fixed AES-128-GCM decryption must compare the complete tag");
    let plaintext_position = gcm_decrypt
        .find("aes128_gcm_ctr_xor_fixed")
        .expect("fixed AES-128-GCM decryption must invoke the CTR helper");
    assert!(
        compare_position < plaintext_position,
        "fixed AES-128-GCM decryption must authenticate before writing plaintext:\n{gcm_decrypt}"
    );

    assert_eq!(alloca_names(gcm_ctr, 16), ["%counter_block", "%keystream"]);
    assert!(gcm_ctr.contains(AES128_BLOCK_SYMBOL));
    assert!(zeroize_calls(gcm_ctr, "%counter_block", 16) >= 2);
    assert!(zeroize_calls(gcm_ctr, "%keystream", 16) >= 2);

    assert_eq!(
        alloca_names(gcm_tag, 16),
        ["%encrypted_j0", "%ghash_result", "%j0"]
    );
    assert!(gcm_tag.contains(AES128_BLOCK_SYMBOL));
    assert!(gcm_tag.contains("ghash_fixed_validated"));
    for owner in ["%encrypted_j0", "%ghash_result", "%j0"] {
        assert!(
            zeroize_calls(gcm_tag, owner, 16) >= 2,
            "AES-128-GCM tag helper must scrub {owner} on success and unwind:\n{gcm_tag}"
        );
    }
    assert_output_precedes_wipes(gcm_tag, &["%encrypted_j0", "%ghash_result", "%j0"]);

    assert_no_conditional_control_flow(gcm_tag_equal);
    for forbidden in ["memcmp", "bcmp"] {
        assert!(
            !gcm_tag_equal.contains(forbidden),
            "fixed tag comparison must not lower to early-exit {forbidden}:\n{gcm_tag_equal}"
        );
    }
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

fn assert_no_conditional_control_flow(function: &str) {
    for forbidden in ["br i1", "switch "] {
        assert!(
            !function.contains(forbidden),
            "fixed-schedule core unexpectedly contains conditional control flow {forbidden}:\n{function}"
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
        .position(|line| {
            (line.contains("memcpy") && line.contains("%output"))
                || (line.trim_start().starts_with("store ") && line.contains("ptr %output"))
        })
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

fn alloca_names(function: &str, size: usize) -> Vec<&str> {
    // Alignment and declaration order vary by target. The bounded byte size
    // and identities of the copied-secret owners are the security invariant.
    let prefix = format!("[{size} x i8], align ");
    let mut names: Vec<_> = function
        .lines()
        .filter_map(|line| {
            let (name, allocation) = line.trim().split_once(" = alloca ")?;
            allocation.starts_with(&prefix).then_some(name)
        })
        .collect();
    names.sort_unstable();
    names
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
