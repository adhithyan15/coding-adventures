//! Execution proof that `puts` on an Array UNPACKS it one element per line
//! (real Ruby's `Kernel#puts` rule), rather than bracket-displaying it like
//! every other display path (`print`, a nested array, `Hash`). Lower REAL
//! Ruby source, emit C, compile with a real cc, run, assert stdout. Skips
//! gracefully when no `cc` is present.

use std::process::Command;

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    ["cc", "clang", "gcc"]
        .iter()
        .find(|c| {
            Command::new(c)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
}

fn run_ruby(src: &str) -> Option<String> {
    let cc = find_cc()?;
    let module = ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering");
    let art = semantic_ir_to_c::compile(&module).expect("C compile (no panic)");
    let dir = std::env::temp_dir();
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_putsarr_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm") // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        art.source
    );
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "program exited non-zero");
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

#[test]
fn puts_on_a_flat_array_unpacks_one_element_per_line() {
    match run_ruby("puts [1, 2, 3]\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn puts_on_an_empty_array_prints_nothing_at_all() {
    // Not even a blank line -- distinct from `puts nil` (one blank line) and
    // `puts []` bracket-displaying (`"[]\n"`, the OLD wrong behavior).
    match run_ruby("puts []\nputs \"after\"\n") {
        Some(out) => assert_eq!(out, "after\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn puts_recursively_flattens_every_level_of_nested_arrays() {
    // `puts [1, [2, 3], 4]` -> "1\n2\n3\n4\n", not one line per TOP-LEVEL
    // element (which would print the nested `[2, 3]` bracketed).
    match run_ruby("puts [1, [2, 3], 4]\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n4\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn puts_on_a_nested_empty_array_contributes_zero_lines() {
    // `[[]]` flattens to nothing (the outer array's only element is an
    // empty array, which itself contributes nothing) -- not an empty line
    // for the outer array and not a bracketed inner-empty marker.
    match run_ruby("puts [[]]\nputs \"after\"\n") {
        Some(out) => assert_eq!(out, "after\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn puts_does_not_unpack_a_hash_argument() {
    // Only Array gets the unpacking treatment; a Hash argument to `puts`
    // still prints as one brace-wrapped line, matching real Ruby (`puts
    // {"a"=>1}` prints the Hash's `to_s`/inspect form, not its entries).
    match run_ruby("puts({\"a\" => 1, \"b\" => 2})\n") {
        Some(out) => assert_eq!(out, "{a: 1, b: 2}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn print_still_bracket_displays_an_array() {
    // `print` (unlike `puts`) never unpacks -- it always uses the general
    // display path, matching Ruby's `Kernel#print`.
    match run_ruby("print [1, 2, 3]\n") {
        Some(out) => assert_eq!(out, "[1, 2, 3]"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn puts_on_a_self_referential_array_terminates_instead_of_recursing_forever() {
    // `a[0] = a` -- a self-referential array, constructible via bracket-
    // index write. `_sir_puts_one` shares `_sir_fmt`'s depth cap, so this
    // must terminate (not stack-overflow) rather than actually proving
    // anything about the printed content.
    match run_ruby("a = [1]\na[0] = a\nputs \"ok\"\n") {
        Some(out) => assert_eq!(out, "ok\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn puts_on_multiple_array_arguments_unpacks_each_in_order() {
    match run_ruby("puts [1, 2], [3, 4]\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n4\n"),
        None => eprintln!("skip: no cc"),
    }
}
