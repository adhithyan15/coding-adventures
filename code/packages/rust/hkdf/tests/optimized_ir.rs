//! Release-codegen regression gate for the fixed-memory Expand boundary.
//!
//! Source-level ownership tests cannot prove that an ABI return slot containing
//! `T(i)` was scrubbed after the tag was copied. This test asks the active Rust
//! compiler for optimized LLVM IR and verifies that both HMAC result payloads
//! and the previous-block owner reach the repository zeroize primitive, while
//! the public Expand function contains no allocator call.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn optimized_fixed_expand_scrubs_result_slots_without_allocating() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("HKDF package must be inside the Rust workspace");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "coding-adventures-hkdf-ir-{}-{unique}",
        std::process::id()
    ));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let compilation = Command::new(cargo)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "rustc",
            "-p",
            "coding_adventures_hkdf",
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
                        .starts_with("coding_adventures_hkdf-")
                })
        })
        .expect("Cargo must emit HKDF LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let expand = rust_function_body(&ir, "coding_adventures_hkdf::hkdf_expand_fixed");
    for forbidden in [
        "__rust_alloc",
        "__rust_realloc",
        "alloc::alloc",
        "exchange_malloc",
        "RawVec",
    ] {
        assert!(
            !expand.contains(forbidden),
            "fixed Expand unexpectedly references allocator symbol {forbidden}:\n{expand}"
        );
    }

    let sha256_payload = assert_result_slot_wiped(expand, 33, 32);
    let sha384_payload = assert_result_slot_wiped(expand, 49, 48);
    let previous = alloca_name(expand, 48, 1);
    assert_owner_wiped(
        expand,
        &previous,
        48,
        &[(&sha256_payload, 32), (&sha384_payload, 48)],
    );
}

fn assert_result_slot_wiped(function: &str, result_size: usize, tag_size: usize) -> String {
    let result = alloca_name(function, result_size, 1);
    let payload_marker = format!("ptr {result}, i64 1");
    let payload = function
        .lines()
        .find_map(|line| {
            let line = line.trim();
            (line.contains("getelementptr") && line.contains(&payload_marker)).then(|| {
                line.split(" = ")
                    .next()
                    .expect("GEP must define a value")
                    .to_owned()
            })
        })
        .unwrap_or_else(|| panic!("optimized Expand has no payload pointer for {result}"));

    assert_success_copies_are_wiped(function, &payload, tag_size);
    payload
}

fn alloca_name(function: &str, size: usize, alignment: usize) -> String {
    let alloca_suffix = format!(" = alloca [{size} x i8], align {alignment}");
    let matches: Vec<String> = function
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_suffix(&alloca_suffix).map(str::to_owned)
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "optimized Expand must have exactly one {size}-byte aligned-{alignment} slot, found {matches:?}"
    );
    matches.into_iter().next().expect("one slot was required")
}

fn assert_success_copies_are_wiped(function: &str, payload: &str, tag_size: usize) {
    let lines: Vec<&str> = function.lines().collect();
    let copies: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| is_full_tag_copy(line, payload, tag_size).then_some(index))
        .collect();
    assert!(
        !copies.is_empty(),
        "optimized Expand did not copy a {tag_size}-byte tag from {payload} into the previous-block buffer"
    );

    for copy_index in copies {
        let following = &lines[copy_index + 1..];
        let wipe_offset = following
            .iter()
            .position(|line| is_zeroize_call(line, payload, tag_size));
        let block_offset = following.iter().position(|line| is_basic_block_label(line));
        assert!(
            wipe_offset.is_some_and(|wipe| block_offset.is_none_or(|block| wipe < block)),
            "optimized Expand did not scrub {payload} directly after its successful {tag_size}-byte copy:\n{function}"
        );
    }
}

fn assert_owner_wiped(
    function: &str,
    owner: &str,
    size: usize,
    successful_payloads: &[(&str, usize)],
) {
    let blocks = basic_blocks(function);
    let by_label: HashMap<&str, &BasicBlock<'_>> = blocks
        .iter()
        .map(|block| (block.label.as_str(), block))
        .collect();
    let starts: Vec<&BasicBlock<'_>> = blocks
        .iter()
        .filter(|block| {
            block.lines.iter().any(|line| {
                successful_payloads
                    .iter()
                    .any(|(payload, tag_size)| is_full_tag_copy(line, payload, *tag_size))
            })
        })
        .collect();
    assert!(
        !starts.is_empty(),
        "optimized Expand has no successful tag-copy blocks"
    );

    for start in starts {
        let mut stack = vec![start.label.clone()];
        let mut visited = HashSet::new();
        let mut reached_wipe = false;
        while let Some(label) = stack.pop() {
            if !visited.insert(label.clone()) {
                continue;
            }
            let block = by_label
                .get(label.as_str())
                .unwrap_or_else(|| panic!("optimized Expand references missing block {label}"));
            if block
                .lines
                .iter()
                .any(|line| is_zeroize_call(line, owner, size))
            {
                reached_wipe = true;
                continue;
            }
            let successors = successor_labels(&block.lines);
            if successors.is_empty() {
                assert!(
                    !block
                        .lines
                        .iter()
                        .any(|line| line.trim_start().starts_with("ret ")),
                    "optimized Expand can return from successful copy block {} through terminal block {label} without scrubbing {owner}:\n{function}",
                    start.label
                );
            }
            stack.extend(successors);
        }
        assert!(
            reached_wipe,
            "optimized Expand has no reachable scrub of {owner} after successful copy block {}",
            start.label
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

fn successor_labels(lines: &[&str]) -> Vec<String> {
    let text = lines.join("\n");
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

fn is_full_tag_copy(line: &str, payload: &str, tag_size: usize) -> bool {
    if !line.contains("llvm.memcpy") {
        return false;
    }
    let Some(arguments) = call_arguments(line) else {
        return false;
    };
    let arguments: Vec<&str> = arguments.split(',').collect();
    arguments.len() >= 3
        && line_mentions_value(arguments[1], payload)
        && i64_argument_equals(arguments[2], tag_size)
}

fn is_zeroize_call(line: &str, pointer: &str, size: usize) -> bool {
    line.contains("zeroize_bytes")
        && line_mentions_value(line, pointer)
        && call_has_i64_constant(line, size)
}

fn call_has_i64_constant(line: &str, value: usize) -> bool {
    call_arguments(line).is_some_and(|arguments| {
        arguments
            .split(',')
            .any(|arg| i64_argument_equals(arg, value))
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

fn is_basic_block_label(line: &str) -> bool {
    basic_block_label(line).is_some()
}

fn basic_block_label(line: &str) -> Option<&str> {
    let code = line.split(';').next().unwrap_or_default().trim();
    code.strip_suffix(':').filter(|label| !label.is_empty())
}

#[test]
fn parser_does_not_require_lifetime_intrinsics() {
    let result_without_lifetime = "bb0:\n  call void @llvm.memcpy.p0.p0.i64(ptr %previous, ptr %tag, i64 32, i1 false)\n  invoke void @zeroize_bytes(ptr %tag, i64 32)\n";
    assert_success_copies_are_wiped(result_without_lifetime, "%tag", 32);

    let owner_without_lifetime = "define void @expand() {\ncopy:\n  call void @llvm.memcpy.p0.p0.i64(ptr %previous, ptr %tag, i64 32, i1 false)\n  invoke void @zeroize_bytes(ptr %tag, i64 32) to label %success unwind label %cleanup\nsuccess:\n  call void @zeroize_bytes(ptr %previous, i64 48)\n  ret void\ncleanup:\n  call void @zeroize_bytes(ptr %previous, i64 48)\n  resume { ptr, i32 } undef\n}";
    assert_owner_wiped(owner_without_lifetime, "%previous", 48, &[("%tag", 32)]);
}

#[test]
#[should_panic(expected = "did not scrub %tag directly after")]
fn parser_rejects_a_success_copy_without_a_same_block_wipe() {
    let function = "bb0:\n  call void @llvm.memcpy.p0.p0.i64(ptr %previous, ptr %tag, i64 32, i1 false)\nbb1:\n  invoke void @zeroize_bytes(ptr %tag, i64 32)\n";
    assert_success_copies_are_wiped(function, "%tag", 32);
}

#[test]
fn parser_distinguishes_overlapping_ir_value_names() {
    assert!(line_mentions_value("ptr %tag, i64 32", "%tag"));
    assert!(!line_mentions_value("ptr %tag1, i64 32", "%tag"));
}

#[test]
#[should_panic(expected = "return from successful copy block copy through terminal block success")]
fn parser_rejects_an_owner_wipe_only_on_an_error_path() {
    let function = "define void @expand() {\ncopy:\n  call void @llvm.memcpy.p0.p0.i64(ptr %previous, ptr %tag, i64 32, i1 false)\n  invoke void @zeroize_bytes(ptr %tag, i64 32) to label %success unwind label %error\nerror:\n  call void @zeroize_bytes(ptr %previous, i64 48)\n  ret void\nsuccess:\n  ret void\n}";
    assert_owner_wiped(function, "%previous", 48, &[("%tag", 32)]);
}

fn rust_function_body<'a>(ir: &'a str, rust_name: &str) -> &'a str {
    let marker = format!("; {rust_name}\n");
    let marker_start = ir
        .find(&marker)
        .unwrap_or_else(|| panic!("optimized IR did not identify {rust_name}"));
    let definition = marker_start
        + ir[marker_start..]
            .find("define ")
            .expect("optimized function must have a definition");
    let end = ir[definition..]
        .find("\n}")
        .map(|offset| definition + offset + 2)
        .expect("optimized function must have a closing brace");
    &ir[definition..end]
}
