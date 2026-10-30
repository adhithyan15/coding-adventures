//! Release-codegen regression gate for the fixed-memory AES-128 boundary.
//!
//! Source tests establish FIPS correctness, but they cannot show what the
//! optimizer does with ownership, branches, or the legacy S-box global. This
//! test asks the active compiler for release LLVM IR and checks the admitted
//! boundary itself: caller-owned output, no allocator, no S-box table path,
//! secret-independent control and addresses, no hidden helper calls, and a
//! CFG-ordered wipe for every copied-secret owner.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const AUDIT_SYMBOL: &str = "coding_adventures_aes128_encrypt_fixed_ir_audit";

#[test]
fn optimized_aes128_boundary_is_fixed_memory_table_free_and_scrubbed() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("AES package must be inside the Rust workspace");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "coding-adventures-aes-ir-{}-{unique}",
        std::process::id()
    ));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let compilation = Command::new(cargo)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "rustc",
            "-p",
            "coding_adventures_aes",
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
                    name.to_string_lossy().starts_with("coding_adventures_aes-")
                })
        })
        .expect("Cargo must emit AES LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let boundary = exported_function_body(&ir, AUDIT_SYMBOL);

    assert!(
        boundary.starts_with(&format!("define void @{AUDIT_SYMBOL}(")),
        "fixed AES must return through caller-owned output, not an ABI payload"
    );
    assert!(
        boundary
            .lines()
            .next()
            .is_some_and(|signature| signature.contains("%output")),
        "fixed AES must retain an explicit caller-owned output parameter"
    );
    assert!(
        boundary.contains("%expanded = alloca [176 x i8]")
            && boundary.contains("%state = alloca [16 x i8]")
            && boundary.contains("%key_temp = alloca [4 x i8]")
            && boundary.contains("%row_scratch = alloca [16 x i8]")
            && boundary.contains("%column = alloca [4 x i8]"),
        "fixed AES must retain its bounded copied-secret owners"
    );

    for forbidden in [
        "__rust_alloc",
        "__rust_realloc",
        "__rust_dealloc",
        "alloc::alloc",
        "SBOXES",
        "get_sboxes",
        "build_sboxes",
    ] {
        assert!(
            !boundary.contains(forbidden),
            "fixed AES boundary unexpectedly references {forbidden}"
        );
    }

    let owners = [
        ("%column", 4),
        ("%row_scratch", 16),
        ("%key_temp", 4),
        ("%state", 16),
        ("%expanded", 176),
    ];
    assert_expected_callees(boundary);
    assert_public_control_and_addresses(boundary, &owners);
    assert_output_commit_then_wipes(boundary, &owners);
}

fn exported_function_body<'a>(ir: &'a str, symbol: &str) -> &'a str {
    let signature = format!("define void @{symbol}(");
    let start = ir
        .find(&signature)
        .unwrap_or_else(|| panic!("optimized IR must contain {symbol}"));
    let tail = &ir[start..];
    let end = tail
        .find("\n}\n")
        .expect("exported function must have a closing brace")
        + 3;
    &tail[..end]
}

/// Track secret provenance through optimized SSA values. This deliberately
/// does not equate an `i64` width with public data: a secret `i8` widened with
/// `zext` remains secret. Conditional branches and variable GEP indexes must
/// both be independent of the borrowed key/plaintext and every protected local
/// owner.
fn assert_public_control_and_addresses(function: &str, owners: &[(&str, usize)]) {
    let definitions = ssa_definitions(function);
    let secret_memory: HashSet<String> = ["%plaintext", "%key"]
        .into_iter()
        .chain(owners.iter().map(|(owner, _)| *owner))
        .map(str::to_owned)
        .collect();
    let secret_values = secret_value_closure(&definitions, &secret_memory);
    let secret_addresses = secret_address_closure(&definitions, &secret_values);
    assert!(
        secret_addresses.is_empty(),
        "fixed AES constructs secret-derived pointer addresses: {secret_addresses:?}"
    );
    let mut control_count = 0usize;

    for line in function.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("br i1 ") {
            control_count += 1;
            let condition = rest
                .split(',')
                .next()
                .expect("conditional branch must expose its predicate");
            assert!(
                !secret_values.contains(condition),
                "secret-derived branch predicate {condition}: {line}"
            );
        } else if let Some(rest) = line.strip_prefix("switch ") {
            control_count += 1;
            let condition = rest
                .split_ascii_whitespace()
                .nth(1)
                .map(|value| value.trim_end_matches(','))
                .expect("switch must expose its predicate");
            assert!(
                !secret_values.contains(condition),
                "secret-derived switch predicate {condition}: {line}"
            );
        }
    }
    assert!(
        control_count > 0,
        "fixed AES should retain public bounded control flow"
    );

    let mut variable_index_count = 0usize;
    for (value, definition) in &definitions {
        let index_fields: Vec<&str> = if definition.contains("getelementptr") {
            definition.split(',').skip(2).collect()
        } else if definition.contains("extractelement") || definition.contains("insertelement") {
            definition.rsplit(',').next().into_iter().collect()
        } else {
            Vec::new()
        };
        for index_field in index_fields {
            for index in ir_values(index_field) {
                variable_index_count += 1;
                assert!(
                    !secret_values.contains(&index),
                    "secret-derived address index {index} in {value} = {definition}"
                );
            }
        }
    }
    assert!(
        variable_index_count > 0,
        "fixed AES should retain public variable address indexes"
    );
}

/// The audited boundary must be closed over code generation. Any ordinary
/// helper call could hide a secret-dependent branch or lookup outside the
/// function body inspected here, so only compiler intrinsics, the repository
/// zeroize primitive, and Rust's cleanup-abort routine are admitted.
fn assert_expected_callees(function: &str) {
    for line in function.lines().map(str::trim) {
        if line.starts_with(';') {
            continue;
        }
        let is_call = line.starts_with("call ")
            || line.starts_with("tail call ")
            || line.starts_with("invoke ")
            || line.contains(" call ");
        if !is_call {
            continue;
        }
        let before_arguments = line.split_once('(').map_or(line, |(before, _)| before);
        let Some(at) = before_arguments.rfind('@') else {
            panic!("fixed AES contains unexpected indirect callee: {line}");
        };
        let callee = &before_arguments[at + 1..];
        assert!(
            callee.starts_with("llvm.") || is_expected_rust_helper(callee),
            "fixed AES contains unexpected callee @{callee}: {line}"
        );
    }
}

/// Admit exactly the two reviewed Rust paths under either mangling scheme.
/// Crate hashes distinguish builds; they must never widen the allowed path.
fn is_expected_rust_helper(callee: &str) -> bool {
    [
        (
            "_ZN25coding_adventures_zeroize13zeroize_bytes17h",
            "_RNvC",
            "25coding_adventures_zeroize13zeroize_bytes",
        ),
        (
            "_ZN4core9panicking16panic_in_cleanup17h",
            "_RNvNtC",
            "4core9panicking16panic_in_cleanup",
        ),
    ]
    .iter()
    .any(|(legacy_prefix, v0_prefix, path)| {
        let legacy = callee
            .strip_prefix(legacy_prefix)
            .and_then(|tail| tail.strip_suffix('E'))
            .is_some_and(|hash| hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()));
        let v0 = callee.strip_prefix(v0_prefix).is_some_and(|tail| {
            let path_tail = if let Some(disambiguated) = tail.strip_prefix('s') {
                let Some((hash, path_tail)) = disambiguated.split_once('_') else {
                    return false;
                };
                if !hash.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                    return false;
                }
                path_tail
            } else {
                tail
            };
            path_tail == *path
        });
        legacy || v0
    })
}

#[test]
fn helper_allowlist_accepts_legacy_and_v0_paths_only() {
    for callee in [
        "_ZN25coding_adventures_zeroize13zeroize_bytes17h0123456789abcdefE",
        "_ZN4core9panicking16panic_in_cleanup17hfedcba9876543210E",
        "_RNvCs2xMqgm6MVc5_25coding_adventures_zeroize13zeroize_bytes",
        "_RNvNtCskADBYZwJmSU_4core9panicking16panic_in_cleanup",
        "_RNvC25coding_adventures_zeroize13zeroize_bytes",
    ] {
        assert_expected_callees(&format!("call void @{callee}(ptr %state)"));
    }
    for callee in [
        "_RNvCs2xMqgm6MVc5_25coding_adventures_zeroize13zeroize_bytes_extra",
        "_RNvCs2xMqgm6MVc5_13foreign_crate13zeroize_bytes",
        "_RNvCsbad$hash_25coding_adventures_zeroize13zeroize_bytes",
        "_RNvNtCskADBYZwJmSU_4core9panicking16panic_in_cleanup_extra",
        "_ZN25coding_adventures_zeroize13zeroize_bytes17hnot_a_hex_hash__E",
    ] {
        assert!(std::panic::catch_unwind(|| {
            assert_expected_callees(&format!("call void @{callee}(ptr %state)"));
        })
        .is_err());
    }
}

fn ssa_definitions(function: &str) -> HashMap<String, String> {
    function
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (value, definition) = line.split_once(" = ")?;
            value
                .starts_with('%')
                .then(|| (value.to_owned(), definition.to_owned()))
        })
        .collect()
}

/// Compute secret provenance to a fixed point so cyclic phi graphs cannot hide
/// a key-derived value behind source order or DFS cycle handling.
fn secret_value_closure(
    definitions: &HashMap<String, String>,
    secret_memory: &HashSet<String>,
) -> HashSet<String> {
    let mut secret_pointers = secret_memory.clone();
    loop {
        let mut changed = false;
        for (value, definition) in definitions {
            let produces_pointer = definition.contains("getelementptr")
                || definition.starts_with("bitcast ")
                || definition.starts_with("addrspacecast ")
                || definition.starts_with("phi ptr ")
                || definition.contains("@llvm.ptrmask")
                || (definition.starts_with("select ") && definition.contains(" ptr "));
            if produces_pointer
                && ir_values(definition)
                    .iter()
                    .any(|operand| secret_pointers.contains(operand))
            {
                changed |= secret_pointers.insert(value.clone());
            }
        }
        if !changed {
            break;
        }
    }

    let mut secret_values = secret_memory.clone();
    loop {
        let mut changed = false;
        for (value, definition) in definitions {
            let operands = ir_values(definition);
            let secret = if definition.starts_with("load ") {
                operands
                    .iter()
                    .any(|operand| secret_pointers.contains(operand))
            } else {
                operands
                    .iter()
                    .any(|operand| secret_values.contains(operand))
            };
            if secret {
                changed |= secret_values.insert(value.clone());
            }
        }
        if !changed {
            break;
        }
    }
    secret_values
}

/// Track whether the *choice of address* depends on secret data. Pointers to
/// secret memory are intentionally not tainted merely because their contents
/// are secret; only secret-selected or secret-constructed addresses are. This
/// keeps content provenance separate from address provenance.
fn secret_address_closure(
    definitions: &HashMap<String, String>,
    secret_values: &HashSet<String>,
) -> HashSet<String> {
    let mut secret_addresses = HashSet::new();
    loop {
        let mut changed = false;
        for (value, definition) in definitions {
            let operands = ir_values(definition);
            let secret_address =
                if definition.starts_with("select ") && definition.contains(" ptr ") {
                    operands.first().is_some_and(|condition| {
                        secret_values.contains(condition) || secret_addresses.contains(condition)
                    }) || operands
                        .iter()
                        .skip(1)
                        .any(|pointer| secret_addresses.contains(pointer))
                } else if definition.starts_with("inttoptr ") {
                    operands.iter().any(|operand| {
                        secret_values.contains(operand) || secret_addresses.contains(operand)
                    })
                } else if definition.contains("@llvm.ptrmask") {
                    operands
                        .iter()
                        .skip(1)
                        .any(|mask| secret_values.contains(mask) || secret_addresses.contains(mask))
                        || operands
                            .first()
                            .is_some_and(|pointer| secret_addresses.contains(pointer))
                } else if definition.contains("getelementptr") {
                    operands
                        .first()
                        .is_some_and(|base| secret_addresses.contains(base))
                } else if definition.starts_with("bitcast ")
                    || definition.starts_with("addrspacecast ")
                    || definition.starts_with("phi ptr ")
                {
                    operands
                        .iter()
                        .any(|operand| secret_addresses.contains(operand))
                } else {
                    false
                };
            if secret_address {
                changed |= secret_addresses.insert(value.clone());
            }
        }
        if !changed {
            break;
        }
    }
    secret_addresses
}

fn ir_values(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && is_ir_value_byte(bytes[index]) {
            index += 1;
        }
        if index > start + 1 {
            values.push(text[start..index].to_owned());
        }
    }
    values
}

/// Starting from the real ciphertext commit, follow every normal and unwind
/// CFG edge. Each owner must cross its exact wipe before any terminal block.
/// A second forward traversal proves no owner was wiped on a path before the
/// commit and then used to produce the ciphertext.
fn assert_output_commit_then_wipes(function: &str, owners: &[(&str, usize)]) {
    let blocks = basic_blocks(function);
    let by_label: HashMap<&str, &BasicBlock<'_>> = blocks
        .iter()
        .map(|block| (block.label.as_str(), block))
        .collect();
    let commits: Vec<(&BasicBlock<'_>, usize)> = blocks
        .iter()
        .filter_map(|block| {
            block
                .lines
                .iter()
                .position(|line| is_output_commit(line))
                .map(|index| (block, index))
        })
        .collect();
    assert_eq!(
        commits.len(),
        1,
        "fixed AES must have exactly one state-to-output ciphertext commit"
    );
    let (commit_block, commit_index) = commits[0];

    assert_no_wipe_before_commit(&blocks, &by_label, commit_block, commit_index, owners);
    for &(owner, length) in owners {
        assert_owner_wiped_after_commit(&by_label, commit_block, commit_index + 1, owner, length);
    }
}

fn assert_no_wipe_before_commit(
    blocks: &[BasicBlock<'_>],
    by_label: &HashMap<&str, &BasicBlock<'_>>,
    commit_block: &BasicBlock<'_>,
    commit_index: usize,
    owners: &[(&str, usize)],
) {
    let entry = blocks
        .first()
        .expect("optimized function must have an entry block");
    let mut stack = vec![(entry.label.clone(), 0usize)];
    let mut visited = HashSet::new();
    let mut reached_commit = false;

    while let Some((label, offset)) = stack.pop() {
        if !visited.insert(label.clone()) {
            continue;
        }
        let block = by_label
            .get(label.as_str())
            .unwrap_or_else(|| panic!("optimized AES references missing block {label}"));
        let end = if block.label == commit_block.label {
            commit_index
        } else {
            block.lines.len()
        };
        let remaining = &block.lines[offset..end];
        for &(owner, length) in owners {
            assert!(
                !remaining
                    .iter()
                    .any(|line| is_zeroize_call(line, owner, length)),
                "optimized AES scrubs {owner} before ciphertext commit"
            );
        }
        if block.label == commit_block.label {
            reached_commit = true;
            continue;
        }
        let successors = successor_labels(&block.lines[offset..]);
        assert!(
            !successors.is_empty(),
            "optimized AES has a terminal path before ciphertext commit in {label}"
        );
        stack.extend(successors.into_iter().map(|next| (next, 0)));
    }
    assert!(
        reached_commit,
        "optimized AES ciphertext commit is unreachable"
    );
}

fn assert_owner_wiped_after_commit(
    by_label: &HashMap<&str, &BasicBlock<'_>>,
    commit_block: &BasicBlock<'_>,
    commit_offset: usize,
    owner: &str,
    length: usize,
) {
    let mut stack = vec![(commit_block.label.clone(), commit_offset)];
    let mut visited = HashSet::new();
    let mut reached_wipe = false;

    while let Some((label, offset)) = stack.pop() {
        if !visited.insert(label.clone()) {
            continue;
        }
        let block = by_label
            .get(label.as_str())
            .unwrap_or_else(|| panic!("optimized AES references missing block {label}"));
        let remaining = &block.lines[offset..];
        if remaining
            .iter()
            .any(|line| is_zeroize_call(line, owner, length))
        {
            reached_wipe = true;
            continue;
        }
        assert!(
            !remaining.iter().any(|line| is_terminal(line)),
            "optimized AES reaches terminal path {label} before scrubbing {owner}"
        );
        let successors = successor_labels(remaining);
        assert!(
            !successors.is_empty(),
            "optimized AES path {label} ends before scrubbing {owner}"
        );
        stack.extend(successors.into_iter().map(|next| (next, 0)));
    }
    assert!(
        reached_wipe,
        "optimized AES has no reachable scrub of {owner} after ciphertext commit"
    );
}

struct BasicBlock<'a> {
    label: String,
    lines: Vec<&'a str>,
}

fn basic_blocks(function: &str) -> Vec<BasicBlock<'_>> {
    let mut blocks = Vec::new();
    let mut current = Some(BasicBlock {
        label: "__entry".to_owned(),
        lines: Vec::new(),
    });
    for line in function.lines().skip(1) {
        if let Some(label) = basic_block_label(line) {
            if let Some(block) = current.take() {
                if !block.lines.is_empty() {
                    blocks.push(block);
                }
            }
            current = Some(BasicBlock {
                label: label.to_owned(),
                lines: Vec::new(),
            });
        } else if let Some(block) = &mut current {
            block.lines.push(line);
        }
    }
    if let Some(block) = current {
        blocks.push(block);
    }
    blocks
}

fn basic_block_label(line: &str) -> Option<&str> {
    let code = line.split(';').next().unwrap_or_default().trim();
    code.strip_suffix(':').filter(|label| !label.is_empty())
}

fn successor_labels(lines: &[&str]) -> Vec<String> {
    let text = lines.join("\n");
    // The repository zeroize primitive has no panic path. Rust still emits an
    // unwind edge for its non-`nounwind` ABI; following that edge models a
    // destructor panicking while it is already cleaning up, which terminates
    // the process and cannot provide further ordered drops. For a zeroize
    // invoke, continue through its normal edge after the exact wipe. Other
    // invokes retain both normal and unwind successors.
    if text.contains("invoke ") && text.contains("zeroize_bytes") {
        return label_after(&text, "to label %").into_iter().collect();
    }
    let mut successors = Vec::new();
    let mut remaining = text.as_str();
    while let Some(offset) = remaining.find("label %") {
        let after = &remaining[offset + "label %".len()..];
        let length = after
            .bytes()
            .take_while(|byte| is_ir_value_byte(*byte))
            .count();
        if length > 0 {
            successors.push(after[..length].to_owned());
        }
        remaining = &after[length..];
    }
    successors.sort();
    successors.dedup();
    successors
}

fn label_after(text: &str, marker: &str) -> Option<String> {
    let after = text.split_once(marker)?.1;
    let length = after
        .bytes()
        .take_while(|byte| is_ir_value_byte(*byte))
        .count();
    (length > 0).then(|| after[..length].to_owned())
}

fn is_output_commit(line: &str) -> bool {
    if !line.contains("llvm.memcpy") {
        return false;
    }
    let Some(arguments) = call_arguments(line) else {
        return false;
    };
    let arguments: Vec<&str> = arguments.split(',').collect();
    arguments.len() >= 3
        && line_mentions_value(arguments[0], "%output")
        && line_mentions_value(arguments[1], "%state")
        && i64_argument_equals(arguments[2], 16)
}

fn is_zeroize_call(line: &str, owner: &str, length: usize) -> bool {
    line.contains("zeroize_bytes")
        && line_mentions_value(line, owner)
        && call_has_i64_constant(line, length)
}

fn call_has_i64_constant(line: &str, value: usize) -> bool {
    call_arguments(line).is_some_and(|arguments| {
        arguments
            .split(',')
            .any(|argument| i64_argument_equals(argument, value))
    })
}

fn i64_argument_equals(argument: &str, value: usize) -> bool {
    let tokens: Vec<&str> = argument.split_ascii_whitespace().collect();
    tokens.first() == Some(&"i64")
        && tokens
            .last()
            .and_then(|token| token.trim_end_matches(')').parse::<usize>().ok())
            == Some(value)
}

fn call_arguments(line: &str) -> Option<&str> {
    let start = line.find('(')? + 1;
    let mut depth = 1usize;
    for (offset, byte) in line.as_bytes()[start..].iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&line[start..start + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

fn line_mentions_value(line: &str, value: &str) -> bool {
    line.match_indices(value).any(|(start, _)| {
        let end = start + value.len();
        let before_is_value = start
            .checked_sub(1)
            .and_then(|index| line.as_bytes().get(index))
            .is_some_and(|byte| is_ir_value_byte(*byte));
        let after_is_value = line
            .as_bytes()
            .get(end)
            .is_some_and(|byte| is_ir_value_byte(*byte));
        !before_is_value && !after_is_value
    })
}

fn is_terminal(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("ret ") || line.starts_with("resume ") || line == "unreachable"
}

fn is_ir_value_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-' | b'$')
}

#[test]
#[should_panic(expected = "secret-derived branch predicate")]
fn parser_rejects_secret_widened_before_a_branch() {
    let function = "define void @aes(ptr %key) {\nstart:\n  %secret8 = load i8, ptr %key\n  %secret64 = zext i8 %secret8 to i64\n  %condition = icmp eq i64 %secret64, 0\n  br i1 %condition, label %left, label %right\nleft:\n  ret void\nright:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived address index")]
fn parser_rejects_secret_indexing_into_an_unnamed_local_table() {
    let function = "define void @aes(ptr %key) {\nstart:\n  br i1 true, label %indexed, label %done\nindexed:\n  %table = alloca [256 x i8]\n  %secret8 = load i8, ptr %key\n  %index = zext i8 %secret8 to i64\n  %slot = getelementptr i8, ptr %table, i64 %index\n  %value = load i8, ptr %slot\n  br label %done\ndone:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived switch predicate")]
fn parser_rejects_a_secret_switch() {
    let function = "define void @aes(ptr %key) {\nstart:\n  %secret = load i8, ptr %key\n  switch i8 %secret, label %default [ i8 0, label %zero ]\nzero:\n  ret void\ndefault:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived address index")]
fn parser_rejects_a_secret_vector_index() {
    let function = "define void @aes(ptr %key) {\nstart:\n  br i1 true, label %indexed, label %done\nindexed:\n  %table = load <256 x i8>, ptr @table\n  %secret8 = load i8, ptr %key\n  %index = zext i8 %secret8 to i64\n  %value = extractelement <256 x i8> %table, i64 %index\n  br label %done\ndone:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived pointer addresses")]
fn parser_rejects_a_secret_selected_pointer() {
    let function = "define void @aes(ptr %key, ptr %left, ptr %right) {\nstart:\n  br i1 true, label %selected, label %done\nselected:\n  %secret = load i8, ptr %key\n  %condition = icmp eq i8 %secret, 0\n  %address = select i1 %condition, ptr %left, ptr %right\n  %value = load i8, ptr %address\n  br label %done\ndone:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived pointer addresses")]
fn parser_rejects_a_secret_integer_to_pointer() {
    let function = "define void @aes(ptr %key) {\nstart:\n  br i1 true, label %constructed, label %done\nconstructed:\n  %secret = load i8, ptr %key\n  %wide = zext i8 %secret to i64\n  %address = inttoptr i64 %wide to ptr\n  %value = load i8, ptr %address\n  br label %done\ndone:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived pointer addresses")]
fn parser_rejects_a_secret_pointer_mask() {
    let function = "define void @aes(ptr %key, ptr %base) {\nstart:\n  br i1 true, label %masked, label %done\nmasked:\n  %secret = load i8, ptr %key\n  %mask = zext i8 %secret to i64\n  %address = call ptr @llvm.ptrmask.p0.i64(ptr %base, i64 %mask)\n  %value = load i8, ptr %address\n  br label %done\ndone:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "secret-derived branch predicate")]
fn parser_tracks_secret_memory_through_a_public_pointer_mask() {
    let function = "define void @aes(ptr %key) {\nstart:\n  %masked = call ptr @llvm.ptrmask.p0.i64(ptr %key, i64 -16)\n  %secret = load i8, ptr %masked\n  %condition = icmp eq i8 %secret, 0\n  br i1 %condition, label %left, label %right\nleft:\n  ret void\nright:\n  ret void\n}";
    assert_public_control_and_addresses(function, &[]);
}

#[test]
#[should_panic(expected = "unexpected callee @new_sbox")]
fn parser_rejects_an_unexpected_helper_call() {
    let function = "define void @aes(ptr %key) {\nstart:\n  %secret = load i8, ptr %key\n  %value = call i8 @new_sbox(i8 %secret)\n  ret void\n}";
    assert_expected_callees(function);
}

#[test]
#[should_panic(expected = "terminal path success before scrubbing %state")]
fn parser_rejects_a_cleanup_only_wipe() {
    let function = "define void @aes(ptr %output) {\nstart:\n  %state = alloca [16 x i8]\n  call void @llvm.memcpy(ptr %output, ptr %state, i64 16)\n  invoke void @may_fail() to label %success unwind label %cleanup\nsuccess:\n  ret void\ncleanup:\n  call void @zeroize_bytes(ptr %state, i64 16)\n  resume { ptr, i32 } undef\n}";
    assert_output_commit_then_wipes(function, &[("%state", 16)]);
}

#[test]
#[should_panic(expected = "terminal path returned before scrubbing %state")]
fn parser_rejects_a_return_path_before_wipe() {
    let function = "define void @aes(ptr %output) {\nstart:\n  %state = alloca [16 x i8]\n  call void @llvm.memcpy(ptr %output, ptr %state, i64 16)\n  br i1 true, label %wiped, label %returned\nwiped:\n  call void @zeroize_bytes(ptr %state, i64 16)\n  ret void\nreturned:\n  ret void\n}";
    assert_output_commit_then_wipes(function, &[("%state", 16)]);
}

#[test]
#[should_panic(expected = "scrubs %state before ciphertext commit")]
fn parser_rejects_a_wipe_before_final_output_use() {
    let function = "define void @aes(ptr %output) {\nstart:\n  %state = alloca [16 x i8]\n  call void @zeroize_bytes(ptr %state, i64 16)\n  call void @llvm.memcpy(ptr %output, ptr %state, i64 16)\n  ret void\n}";
    assert_output_commit_then_wipes(function, &[("%state", 16)]);
}

#[test]
#[should_panic(expected = "scrubs %state before ciphertext commit")]
fn parser_checks_an_unlabelled_entry_block() {
    let function = "define void @aes(ptr %output) {\n  %state = alloca [16 x i8]\n  call void @zeroize_bytes(ptr %state, i64 16)\n  call void @llvm.memcpy(ptr %output, ptr %state, i64 16)\n  ret void\n}";
    assert_output_commit_then_wipes(function, &[("%state", 16)]);
}
