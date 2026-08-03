//! Execution proof for the bracket-index bug fix on the C backend — lower
//! REAL Ruby source (`a[i]`, `a[i] = v`), emit C, compile with a real cc,
//! run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! `recv[k]` / `recv[k] = v` lower to `__method__(recv, "[]", k)` /
//! `__method__(recv, "[]=", k, v)` — the SAME narrow-waist dispatch every
//! other Collections built-in uses, so this proves the runtime dispatch in
//! `semantic-ir-to-c`'s `_sir_builtin_method_v` (the `"[]"`/`"[]="` arms)
//! genuinely branches on the RECEIVER's actual tag (Array vs Hash), not a
//! compile-time guess from the index's syntactic shape. The critical cases
//! are the non-string-key Hash writes (`h[2] = ...`, `h[:sym] = ...`) — the
//! rejected heuristic design (string-literal key -> Map, else -> Seq) would
//! have mis-routed these to the Array path and crashed.

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
    // Hash the full source, not just its length -- see the temp-file-collision
    // fix elsewhere in this test suite (equal-length sources run as parallel
    // threads in the SAME process and would collide on a length-keyed stem).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_idxbr_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
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
fn array_read_in_call_argument_and_bare_statement_position() {
    match run_ruby("a = [10, 20, 30]\nputs(a[1])\nputs a[2]\n") {
        Some(out) => assert_eq!(out, "20\n30\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_read_into_a_variable() {
    // The bug this fixes: `x = g[1]` used to silently split into two
    // statements (`x = g`, dangling `[1]`) instead of reading the element.
    match run_ruby("g = [5, 6, 7]\nx = g[1]\nputs x\n") {
        Some(out) => assert_eq!(out, "6\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_write_then_read_back() {
    match run_ruby("a = [1, 2, 3]\na[0] = 9\nputs a\n") {
        Some(out) => assert_eq!(out, "[9, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn array_read_out_of_bounds_is_nil() {
    match run_ruby("a = [1, 2, 3]\nputs a[10]\n") {
        Some(out) => assert_eq!(out, "nil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_read_and_write_with_string_key() {
    match run_ruby("h = {}\nh[\"b\"] = 2\nputs h[\"b\"]\nputs h\n") {
        Some(out) => assert_eq!(out, "2\n{b: 2}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_write_with_integer_key_does_not_crash() {
    // The critical regression case: the rejected string-literal-key
    // heuristic mis-routed this to Array's `_sir_seq_set` (the key isn't a
    // StrLit), which EXITS at runtime with "sir: []= on a non-sequence"
    // because `h` is actually a SirMap. The `__method__` dispatch instead
    // checks `h`'s ACTUAL runtime tag, so an int key on a Hash just works.
    match run_ruby("h = {1 => \"a\"}\nh[2] = \"b\"\nputs h\n") {
        Some(out) => assert_eq!(out, "{1: a, 2: b}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_write_with_symbol_key_does_not_crash() {
    match run_ruby("h = {}\nh[:sym] = 1\nputs h\n") {
        Some(out) => assert_eq!(out, "{sym: 1}\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_read_missing_key_is_nil() {
    match run_ruby("h = {\"a\" => 1}\nputs h[\"missing\"]\n") {
        Some(out) => assert_eq!(out, "nil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn hash_write_updates_an_existing_key_in_place() {
    match run_ruby("h = {\"a\" => 1}\nh[\"a\"] = 99\nputs h[\"a\"]\n") {
        Some(out) => assert_eq!(out, "99\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn chained_array_read() {
    match run_ruby("a = [[1, 2], [3, 4]]\nputs a[1][0]\n") {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn cyclic_write_does_not_crash() {
    // `a[0] = a` -- a self-referential array, constructible now that
    // bracket-index write is real. Proves the new dispatch path shares the
    // same aliasing-safe mutators as the rest of Collections (no special
    // casing needed) and that display/exit code stay sane on a cycle.
    match run_ruby("a = [0]\na[0] = a\nputs \"ok\"\n") {
        Some(out) => assert_eq!(out, "ok\n"),
        None => eprintln!("skip: no cc"),
    }
}
