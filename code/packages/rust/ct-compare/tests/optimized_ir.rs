//! Release-codegen regression gate for the constant-work equality boundary.
//!
//! Functional tests prove equality results, but cannot prove that an optimizer
//! retained every input read. This test asks the active Rust compiler for
//! optimization-level-3 LLVM IR and verifies the loop CFG, not just token
//! counts: every cycle passes through the volatile input/accumulator block and
//! no loop exit depends on a value derived from those volatile reads.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct BasicBlock {
    name: String,
    text: String,
    successors: Vec<String>,
}

#[test]
fn optimized_equal_length_scan_retains_full_volatile_loop() {
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
        .arg("-Csymbol-mangling-version=v0")
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
    let scan = rust_function_body(&ir, "ct_compare_codegen_fixture::ct_eq_equal_length");

    assert_full_scan_loop(scan).unwrap_or_else(|reason| {
        panic!("optimized equality scan lost its full volatile loop: {reason}\n\n{scan}")
    });
}

#[test]
fn rust_function_discovery_is_platform_independent() {
    let ir = concat!(
        "; ct_compare_codegen_fixture::ct_eq_equal_length\r\n",
        "; Function Attrs: noinline\r\n",
        "define i1 @__RNvCs_platform_specific_symbol(ptr %a.0, i64 %a.1, ptr %b.0, i64 %b.1) {\r\n",
        "start:\r\n",
        "  ret i1 true\r\n",
        "}\r\n",
    );

    let function = rust_function_body(ir, "ct_compare_codegen_fixture::ct_eq_equal_length");
    assert!(function.starts_with("define i1 @__RNvCs_platform_specific_symbol"));
    assert!(function.ends_with("\n}"));
}

#[test]
fn structural_gate_rejects_a_one_time_volatile_prelude() {
    let false_pass = r#"
define i1 @ct_eq_equal_length(ptr %a.0, i64 %a.1, ptr %b.0, i64 %b.1) {
start:
  %acc = alloca i8
  %left = load volatile i8, ptr %a.0
  %right = load volatile i8, ptr %b.0
  %old = load volatile i8, ptr %acc
  store volatile i8 %old, ptr %acc
  br label %loop
loop:
  %i = phi i64 [ 0, %start ], [ %next, %loop ]
  %next = add i64 %i, 1
  %done = icmp eq i64 %next, %a.1
  br i1 %done, label %exit, label %loop
exit:
  %result = load volatile i8, ptr %acc
  %equal = icmp eq i8 %result, 0
  ret i1 %equal
}
"#;

    assert!(assert_full_scan_loop(false_pass).is_err());
}

#[test]
fn structural_gate_rejects_a_data_dependent_loop_exit() {
    let false_pass = r#"
define i1 @ct_eq_equal_length(ptr %a.0, i64 %a.1, ptr %b.0, i64 %b.1) {
start:
  %acc = alloca i8
  br label %loop
loop:
  %i = phi i64 [ 0, %start ], [ %next, %loop ]
  %left_ptr = getelementptr i8, ptr %a.0, i64 %i
  %right_ptr = getelementptr i8, ptr %b.0, i64 %i
  %old = load volatile i8, ptr %acc
  %left = load volatile i8, ptr %left_ptr
  %right = load volatile i8, ptr %right_ptr
  %different = icmp ne i8 %left, %right
  store volatile i8 %old, ptr %acc
  %next = add i64 %i, 1
  br i1 %different, label %exit, label %loop
exit:
  %result = load volatile i8, ptr %acc
  %equal = icmp eq i8 %result, 0
  ret i1 %equal
}
"#;

    assert!(assert_full_scan_loop(false_pass).is_err());
}

#[test]
fn structural_gate_rejects_a_data_dependent_preheader_bypass() {
    let false_pass = r#"
define i1 @ct_eq_equal_length(ptr %a.0, i64 %a.1, ptr %b.0, i64 %b.1) {
start:
  %acc = alloca i8
  %first_a = load i8, ptr %a.0
  %first_b = load i8, ptr %b.0
  %different = icmp ne i8 %first_a, %first_b
  br i1 %different, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %start ], [ %next, %loop ]
  %left_ptr = getelementptr i8, ptr %a.0, i64 %i
  %right_ptr = getelementptr i8, ptr %b.0, i64 %i
  %old = load volatile i8, ptr %acc
  %left = load volatile i8, ptr %left_ptr
  %right = load volatile i8, ptr %right_ptr
  %combined = or i8 %old, %left
  store volatile i8 %combined, ptr %acc
  %next = add i64 %i, 1
  %done = icmp eq i64 %next, %a.1
  br i1 %done, label %exit, label %loop
exit:
  %result = load volatile i8, ptr %acc
  %equal = icmp eq i8 %result, 0
  ret i1 %equal
}
"#;

    assert!(assert_full_scan_loop(false_pass).is_err());
}

#[test]
fn structural_gate_rejects_a_wide_load_preheader_bypass() {
    let false_pass = r#"
define i1 @ct_eq_equal_length(ptr %a.0, i64 %a.1, ptr %b.0, i64 %b.1) {
start:
  %acc = alloca i8
  %first_a = load i64, ptr %a.0
  %first_b = load i64, ptr %b.0
  %different = icmp ne i64 %first_a, %first_b
  br i1 %different, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %start ], [ %next, %loop ]
  %left_ptr = getelementptr i8, ptr %a.0, i64 %i
  %right_ptr = getelementptr i8, ptr %b.0, i64 %i
  %old = load volatile i8, ptr %acc
  %left = load volatile i8, ptr %left_ptr
  %right = load volatile i8, ptr %right_ptr
  %combined = or i8 %old, %left
  store volatile i8 %combined, ptr %acc
  %next = add i64 %i, 1
  %done = icmp eq i64 %next, %a.1
  br i1 %done, label %exit, label %loop
exit:
  %result = load volatile i8, ptr %acc
  %equal = icmp eq i8 %result, 0
  ret i1 %equal
}
"#;

    assert!(assert_full_scan_loop(false_pass).is_err());
}

fn assert_full_scan_loop(scan: &str) -> Result<(), String> {
    if scan.matches("load volatile i8").count() != 4 {
        return Err("expected three loop reads and one final accumulator read".into());
    }
    if scan.matches("store volatile i8").count() != 1 {
        return Err("expected exactly one static volatile accumulator store".into());
    }
    if scan.contains("memcmp") || scan.contains("bcmp") {
        return Err("scan lowered to an early-exit comparison routine".into());
    }
    if scan
        .lines()
        .filter(|line| line.trim_start().starts_with("ret "))
        .count()
        != 1
    {
        return Err("scan must have exactly one post-loop return".into());
    }

    let blocks = parse_blocks(scan)?;
    let edges = blocks
        .iter()
        .map(|block| (block.name.clone(), block.successors.clone()))
        .collect::<HashMap<_, _>>();
    let store_blocks = blocks
        .iter()
        .filter(|block| block.text.contains("store volatile i8"))
        .collect::<Vec<_>>();
    if store_blocks.len() != 1 {
        return Err("volatile accumulator store must have one owning block".into());
    }
    let iteration = store_blocks[0];
    if iteration.text.matches("load volatile i8").count() != 3 {
        return Err("iteration block must read the accumulator and both inputs".into());
    }
    if iteration.text.matches("store volatile i8").count() != 1 {
        return Err("iteration block must write the accumulator exactly once".into());
    }

    let left_pointer = gep_from_input(&iteration.text, "%a.0")?;
    let right_pointer = gep_from_input(&iteration.text, "%b.0")?;
    let volatile_load_pointers = iteration
        .text
        .lines()
        .filter_map(|line| pointer_after(line, "load volatile i8"))
        .collect::<Vec<_>>();
    if !volatile_load_pointers.contains(&left_pointer.as_str())
        || !volatile_load_pointers.contains(&right_pointer.as_str())
    {
        return Err("volatile input reads must use both induction-indexed pointers".into());
    }
    let store_pointer = iteration
        .text
        .lines()
        .find_map(|line| pointer_after(line, "store volatile i8"))
        .ok_or("volatile store has no pointer operand")?;
    if !volatile_load_pointers.contains(&store_pointer) {
        return Err("volatile accumulator load and store must address the same byte".into());
    }

    let loop_component = blocks
        .iter()
        .filter(|block| {
            reachable(&edges, &iteration.name, &block.name, None)
                && reachable(&edges, &block.name, &iteration.name, None)
        })
        .map(|block| block.name.clone())
        .collect::<HashSet<_>>();
    if !has_cycle(&edges, &loop_component) {
        return Err("volatile iteration block is not part of a loop".into());
    }

    let without_iteration = loop_component
        .iter()
        .filter(|name| *name != &iteration.name)
        .cloned()
        .collect::<HashSet<_>>();
    if has_cycle(&edges, &without_iteration) {
        return Err("a loop cycle can bypass the volatile iteration block".into());
    }
    if !loop_component.iter().any(|name| {
        edges.get(name).is_some_and(|targets| {
            targets
                .iter()
                .any(|target| !loop_component.contains(target))
        })
    }) {
        return Err("loop has no public-bound exit".into());
    }

    let definitions = ssa_definitions(scan);
    let tainted = volatile_taint(&definitions);
    let block_names = blocks
        .iter()
        .map(|block| format!("%{}", block.name))
        .collect::<HashSet<_>>();
    let public_lengths = HashSet::from(["%a.1".to_owned(), "%b.1".to_owned()]);
    // Inspect every block, not only the loop SCC. A data-dependent preheader
    // must not be able to bypass a perfectly formed volatile loop and converge
    // on its single return.
    for block in &blocks {
        for line in block.text.lines().map(str::trim) {
            if line.starts_with("switch ")
                || line.starts_with("invoke ")
                || line.starts_with("indirectbr ")
            {
                return Err(format!(
                    "unsupported comparison control in {}: {line}",
                    block.name
                ));
            }
            if ordinary_load(line) {
                return Err(format!("ordinary load inside comparison: {line}"));
            }
            if (line.starts_with("call ") || line.contains(" call "))
                && !line.contains("@llvm.lifetime.")
                && !line.contains("panic_bounds_check")
            {
                return Err(format!("unexpected call inside comparison: {line}"));
            }
            let Some(condition) = branch_condition(line) else {
                continue;
            };
            if tainted.contains(condition) {
                return Err(format!(
                    "comparison control depends on volatile data: {line}"
                ));
            }
            let definition = definitions
                .get(condition)
                .ok_or_else(|| format!("branch condition has no SSA definition: {condition}"))?;
            if !definition.contains("icmp ") || !definition.contains(" i64 ") {
                return Err(format!(
                    "comparison branch is not an induction/length comparison: {line}"
                ));
            }
            if !public_control(
                condition,
                &definitions,
                &block_names,
                &public_lengths,
                &mut HashSet::new(),
            ) {
                return Err(format!(
                    "comparison branch is not derived only from public lengths: {line}"
                ));
            }
        }
    }

    Ok(())
}

fn rust_function_body<'a>(ir: &'a str, rust_name: &str) -> &'a str {
    // Rust emits a demangled source-level name immediately before each LLVM
    // function definition. Anchor to that comment instead of to the ABI symbol:
    // the legacy `_ZN...17h...E` and v0/platform-specific manglings differ.
    let marker = format!("; {rust_name}");
    let marker_start = ir
        .match_indices(&marker)
        .find_map(|(offset, _)| {
            let at_line_start = offset == 0 || ir.as_bytes().get(offset - 1) == Some(&b'\n');
            let after = offset + marker.len();
            let at_line_end = matches!(ir.as_bytes().get(after), Some(b'\r' | b'\n'));
            (at_line_start && at_line_end).then_some(offset)
        })
        .unwrap_or_else(|| panic!("optimized IR did not identify {rust_name}"));
    let after_marker = marker_start + marker.len();
    let definition_offset = ir[after_marker..]
        .find("define ")
        .unwrap_or_else(|| panic!("optimized IR did not define {rust_name}"));
    let definition = after_marker + definition_offset;
    let end = ir[definition..]
        .find("\n}")
        .map(|offset| definition + offset + 2)
        .expect("function definition must have a closing brace");
    &ir[definition..end]
}

fn parse_blocks(function: &str) -> Result<Vec<BasicBlock>, String> {
    let mut blocks = Vec::new();
    let mut current: Option<(String, String)> = None;

    for line in function.lines() {
        if let Some(name) = block_label(line) {
            if let Some((name, text)) = current.take() {
                blocks.push(BasicBlock {
                    successors: label_targets(&text),
                    name,
                    text,
                });
            }
            current = Some((name.to_owned(), String::new()));
        } else if let Some((_, text)) = current.as_mut() {
            text.push_str(line);
            text.push('\n');
        }
    }
    if let Some((name, text)) = current {
        blocks.push(BasicBlock {
            successors: label_targets(&text),
            name,
            text,
        });
    }
    if blocks.is_empty() {
        return Err("optimized function has no basic blocks".into());
    }
    Ok(blocks)
}

fn block_label(line: &str) -> Option<&str> {
    if line.starts_with(char::is_whitespace) || line.starts_with(';') {
        return None;
    }
    let (candidate, suffix) = line.split_once(':')?;
    if !suffix.trim().is_empty() && !suffix.trim_start().starts_with(';') {
        return None;
    }
    candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '$' | '.' | '_' | '-'))
        .then_some(candidate)
}

fn label_targets(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("br ") || line.starts_with("switch ")
        })
        .flat_map(|line| {
            let mut targets = Vec::new();
            let mut rest = line;
            while let Some(offset) = rest.find("label %") {
                let after = &rest[offset + "label %".len()..];
                let end = after
                    .find(|ch: char| {
                        !(ch.is_ascii_alphanumeric() || matches!(ch, '$' | '.' | '_' | '-'))
                    })
                    .unwrap_or(after.len());
                targets.push(after[..end].to_owned());
                rest = &after[end..];
            }
            targets
        })
        .collect()
}

fn reachable(
    edges: &HashMap<String, Vec<String>>,
    start: &str,
    target: &str,
    allowed: Option<&HashSet<String>>,
) -> bool {
    if start == target {
        return true;
    }
    let mut queue = VecDeque::from([start.to_owned()]);
    let mut seen = HashSet::from([start.to_owned()]);
    while let Some(node) = queue.pop_front() {
        for next in edges.get(&node).into_iter().flatten() {
            if allowed.is_some_and(|names| !names.contains(next)) {
                continue;
            }
            if next == target {
                return true;
            }
            if seen.insert(next.clone()) {
                queue.push_back(next.clone());
            }
        }
    }
    false
}

fn has_cycle(edges: &HashMap<String, Vec<String>>, nodes: &HashSet<String>) -> bool {
    nodes.iter().any(|start| {
        edges.get(start).into_iter().flatten().any(|next| {
            nodes.contains(next) && (next == start || reachable(edges, next, start, Some(nodes)))
        })
    })
}

fn gep_from_input(block: &str, input: &str) -> Result<String, String> {
    block
        .lines()
        .find(|line| line.contains("getelementptr") && line.contains(&format!("ptr {input}")))
        .and_then(assignment_name)
        .map(str::to_owned)
        .ok_or_else(|| format!("iteration block has no indexed pointer for {input}"))
}

fn pointer_after<'a>(line: &'a str, operation: &str) -> Option<&'a str> {
    let operation = line.find(operation)?;
    let pointer = line[operation..].find("ptr %")? + operation + "ptr ".len();
    let after = &line[pointer..];
    let end = after
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '%' | '$' | '.' | '_' | '-')))
        .unwrap_or(after.len());
    Some(&after[..end])
}

fn assignment_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let (name, _) = line.split_once(" = ")?;
    name.starts_with('%').then_some(name)
}

fn ssa_definitions(function: &str) -> HashMap<String, String> {
    function
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (name, expression) = line.split_once(" = ")?;
            name.starts_with('%')
                .then(|| (name.to_owned(), expression.to_owned()))
        })
        .collect()
}

fn volatile_taint(definitions: &HashMap<String, String>) -> HashSet<String> {
    let mut tainted = definitions
        .iter()
        .filter(|(_, expression)| expression.contains("load volatile i8"))
        .map(|(name, _)| name.clone())
        .collect::<HashSet<_>>();
    loop {
        let before = tainted.len();
        for (name, expression) in definitions {
            if ssa_values(expression).any(|value| tainted.contains(value)) {
                tainted.insert(name.clone());
            }
        }
        if tainted.len() == before {
            return tainted;
        }
    }
}

fn ssa_values(text: &str) -> impl Iterator<Item = &str> {
    text.split(|ch: char| {
        !(ch.is_ascii_alphanumeric() || matches!(ch, '%' | '$' | '.' | '_' | '-'))
    })
    .filter(|token| token.starts_with('%'))
}

fn branch_condition(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("br i1 ")?;
    Some(rest.split(',').next()?.trim())
}

fn ordinary_load(line: &str) -> bool {
    line.trim_start()
        .split_once(" = ")
        .is_some_and(|(_, expression)| {
            expression.starts_with("load ") && !expression.starts_with("load volatile ")
        })
}

fn public_control(
    value: &str,
    definitions: &HashMap<String, String>,
    block_names: &HashSet<String>,
    public_lengths: &HashSet<String>,
    visiting: &mut HashSet<String>,
) -> bool {
    if public_lengths.contains(value) {
        return true;
    }
    if !visiting.insert(value.to_owned()) {
        // Induction variables form a phi/add dependency cycle. The remaining
        // dependencies still have to terminate at public lengths or constants.
        return true;
    }
    let Some(expression) = definitions.get(value) else {
        visiting.remove(value);
        return false;
    };
    let opcode = expression.split_whitespace().next().unwrap_or_default();
    let allowed_operation =
        matches!(opcode, "icmp" | "phi" | "add" | "sub") && expression.contains(" i64 ");
    let allowed_dependencies = ssa_values(expression)
        .filter(|dependency| !block_names.contains(*dependency))
        .all(|dependency| {
            public_control(
                dependency,
                definitions,
                block_names,
                public_lengths,
                visiting,
            )
        });
    visiting.remove(value);
    allowed_operation && allowed_dependencies
}
