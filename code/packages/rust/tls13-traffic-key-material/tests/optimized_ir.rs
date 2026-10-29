//! Release-codegen regression gate for traffic key and IV staging ownership.
//!
//! Source-level ownership cannot prove that moved or copied stack bytes are
//! scrubbed after optimization. This test asks the active Rust compiler for
//! optimized LLVM IR and verifies that both secret staging arrays are copied
//! only into caller-owned storage, every successful copy reaches the
//! repository zeroize primitive before return, and the boundary contains no
//! allocator call.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const EXPORTED_SYMBOL: &str = "coding_adventures_tls13_derive_traffic_key_material_ir_audit";

#[test]
fn optimized_derivation_scrubs_staging_without_returning_or_allocating_it() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("traffic-key package must be inside the Rust workspace");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "coding-adventures-tls13-traffic-key-ir-{}-{unique}",
        std::process::id()
    ));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let compilation = Command::new(cargo)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "rustc",
            "-p",
            "tls13-traffic-key-material",
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
                        .starts_with("tls13_traffic_key_material-")
                })
        })
        .expect("Cargo must emit traffic-key LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let function = exported_function_body(&ir, EXPORTED_SYMBOL);
    assert_no_allocator(function);

    let key_staging = alloca_name(function, 32, 1);
    let iv_staging = alloca_name(function, 12, 1);
    assert_owner_copies_reach_wipe(function, &key_staging, 32);
    assert_owner_copies_reach_wipe(function, &iv_staging, 12);
    assert_not_copied_to_return_slot(function, &key_staging);
    assert_not_copied_to_return_slot(function, &iv_staging);
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

fn alloca_name(function: &str, size: usize, alignment: usize) -> String {
    let suffix = format!(" = alloca [{size} x i8], align {alignment}");
    let matches: Vec<String> = function
        .lines()
        .filter_map(|line| line.trim().strip_suffix(&suffix).map(str::to_owned))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "optimized derivation must have exactly one {size}-byte aligned-{alignment} staging slot, found {matches:?}:\n{function}"
    );
    matches.into_iter().next().expect("one slot was required")
}

fn assert_owner_copies_reach_wipe(function: &str, owner: &str, size: usize) {
    let blocks = basic_blocks(function);
    let by_label: HashMap<&str, &BasicBlock<'_>> = blocks
        .iter()
        .map(|block| (block.label.as_str(), block))
        .collect();
    let starts: Vec<(&BasicBlock<'_>, usize)> = blocks
        .iter()
        .flat_map(|block| {
            block
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| is_memcpy_from(line, owner))
                .map(move |(index, _)| (block, index))
        })
        .collect();
    assert!(
        !starts.is_empty(),
        "optimized derivation never copies staging owner {owner} into caller storage"
    );

    for (start, copy_index) in starts {
        let mut stack = vec![(start.label.clone(), copy_index + 1)];
        let mut visited = HashSet::new();
        let mut reached_wipe = false;
        while let Some((label, offset)) = stack.pop() {
            if !visited.insert(label.clone()) {
                continue;
            }
            let block = by_label
                .get(label.as_str())
                .unwrap_or_else(|| panic!("optimized derivation references missing block {label}"));
            let remaining = &block.lines[offset..];
            if remaining
                .iter()
                .any(|line| is_zeroize_call(line, owner, size))
            {
                reached_wipe = true;
                continue;
            }
            let successors = normal_successor_labels(remaining);
            if successors.is_empty() {
                assert!(
                    !remaining
                        .iter()
                        .any(|line| line.trim_start().starts_with("ret ")),
                    "optimized derivation can return after copying {owner} without scrubbing it:\n{function}"
                );
            }
            stack.extend(successors.into_iter().map(|successor| (successor, 0)));
        }
        assert!(
            reached_wipe,
            "optimized derivation has no reachable scrub of {owner} after its successful copy"
        );
    }
}

fn assert_not_copied_to_return_slot(function: &str, owner: &str) {
    for line in function.lines().filter(|line| is_memcpy_from(line, owner)) {
        let arguments = call_arguments(line).expect("memcpy call must have arguments");
        let destination = arguments
            .split(',')
            .next()
            .expect("memcpy must have a destination");
        assert!(
            !line_mentions_value(destination, "%_0"),
            "secret staging owner {owner} was copied into the ABI return slot:\n{function}"
        );
    }
}

struct BasicBlock<'a> {
    label: String,
    lines: Vec<&'a str>,
}

fn basic_blocks(function: &str) -> Vec<BasicBlock<'_>> {
    let mut blocks = Vec::new();
    let mut current: Option<BasicBlock<'_>> = None;
    for line in function.lines() {
        if let Some(label) = basic_block_label(line) {
            if let Some(block) = current.take() {
                blocks.push(block);
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

fn normal_successor_labels(lines: &[&str]) -> Vec<String> {
    let text = lines.join("\n");
    if text.contains("invoke ") {
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

fn is_memcpy_from(line: &str, source: &str) -> bool {
    if !line.contains("llvm.memcpy") {
        return false;
    }
    call_arguments(line)
        .and_then(|arguments| arguments.split(',').nth(1))
        .is_some_and(|argument| line_mentions_value(argument, source))
}

fn is_zeroize_call(line: &str, pointer: &str, size: usize) -> bool {
    line.contains("zeroize_bytes")
        && line_mentions_value(line, pointer)
        && call_arguments(line).is_some_and(|arguments| {
            arguments
                .split(',')
                .any(|argument| i64_argument_equals(argument, size))
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

fn is_ir_value_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-' | b'$')
}

fn basic_block_label(line: &str) -> Option<&str> {
    let code = line.split(';').next().unwrap_or_default().trim();
    code.strip_suffix(':').filter(|label| !label.is_empty())
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
    let end = ir[definition..]
        .find("\n}")
        .map(|offset| definition + offset + 2)
        .expect("optimized function must have a closing brace");
    &ir[definition..end]
}

#[test]
#[should_panic(expected = "copied into the ABI return slot")]
fn parser_rejects_secret_copy_into_return_slot() {
    let function = "define void @derive(ptr %_0) {\nstart:\n  call void @llvm.memcpy.p0.p0.i64(ptr %_0, ptr %key, i64 32, i1 false)\n  call void @zeroize_bytes(ptr %key, i64 32)\n  ret void\n}";
    assert_not_copied_to_return_slot(function, "%key");
}

#[test]
#[should_panic(expected = "can return after copying %key without scrubbing it")]
fn parser_rejects_success_path_without_wipe() {
    let function = "define void @derive() {\nstart:\n  %key = alloca [32 x i8], align 1\n  call void @llvm.memcpy.p0.p0.i64(ptr %storage, ptr %key, i64 32, i1 false)\n  ret void\n}";
    assert_owner_copies_reach_wipe(function, "%key", 32);
}
