//! Execution proof for Collections slice 8 (remaining String methods) on the
//! C backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.
//!
//! Semantics are matched against the Python/TS `sir-runtime-oop` reference
//! catalog (the cross-backend golden source this cascade's runtimes agree
//! against) rather than always byte-for-byte true Ruby — e.g. `split(sep)`
//! keeps trailing empty fields like Python's `str.split`, not Ruby's
//! drop-trailing-empties rule; see the runtime.rs file-header comment on the
//! slice-8 helpers for the full rationale.

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
    // Hash the full source, not just its length -- two tests with equal-length
    // sources run as parallel threads in the SAME process and would collide on
    // a length-keyed stem (see the temp-file-collision fix elsewhere in this
    // test suite).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_str8_{}_{}", std::process::id(), hasher.finish());
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
fn capitalize_and_swapcase() {
    match run_ruby("puts \"heLLo WORLD\".capitalize\nputs \"heLLo\".swapcase\n") {
        Some(out) => assert_eq!(out, "Hello world\nHEllO\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn strip_family() {
    match run_ruby("puts \"  hi  \".strip\nputs \"  hi  \".lstrip\nputs \"  hi  \".rstrip\n") {
        Some(out) => assert_eq!(out, "hi\nhi  \n  hi\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn chomp_default_and_with_separator() {
    match run_ruby("puts \"hi\\n\".chomp\nputs \"hi\\r\\n\".chomp\nputs \"hi!!\".chomp(\"!!\")\n") {
        Some(out) => assert_eq!(out, "hi\nhi\nhi\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn chars_and_bytes() {
    match run_ruby("puts \"ab\".chars\nputs \"ab\".bytes\n") {
        Some(out) => assert_eq!(out, "a\nb\n97\n98\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn each_char_yields_every_character_in_order() {
    match run_ruby("\"abc\".each_char { |c| puts c }\n") {
        Some(out) => assert_eq!(out, "a\nb\nc\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn split_no_arg_splits_on_whitespace_runs() {
    match run_ruby("puts \"  a  b c  \".split\n") {
        Some(out) => assert_eq!(out, "a\nb\nc\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn split_with_separator() {
    match run_ruby("puts \"a,b,c\".split(\",\")\n") {
        Some(out) => assert_eq!(out, "a\nb\nc\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn replace_overwrites_the_whole_string() {
    match run_ruby("puts \"old\".replace(\"new\")\n") {
        Some(out) => assert_eq!(out, "new\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn sub_replaces_only_the_first_occurrence() {
    match run_ruby("puts \"aaa\".sub(\"a\", \"b\")\n") {
        Some(out) => assert_eq!(out, "baa\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn gsub_replaces_every_occurrence() {
    match run_ruby("puts \"aaa\".gsub(\"a\", \"b\")\n") {
        Some(out) => assert_eq!(out, "bbb\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn to_i_and_to_f_parse_leading_numeric_prefix() {
    match run_ruby(
        "puts \"42abc\".to_i\nputs \"abc\".to_i\nputs \"3.5xyz\".to_f\nputs \"nope\".to_f\n",
    ) {
        Some(out) => assert_eq!(out, "42\n0\n3.5\n0.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn to_sym_interns_the_string() {
    match run_ruby("x = \"hello\".to_sym\nputs x\n") {
        Some(out) => assert_eq!(out, "hello\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn tr_translates_matching_characters() {
    match run_ruby("puts \"hello\".tr(\"el\", \"ip\")\n") {
        Some(out) => assert_eq!(out, "hippo\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn tr_with_empty_to_deletes_matching_characters() {
    match run_ruby("puts \"hello\".tr(\"l\", \"\")\n") {
        Some(out) => assert_eq!(out, "heo\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn tr_shorter_to_repeats_its_last_character() {
    match run_ruby("puts \"hello\".tr(\"lo\", \"p\")\n") {
        Some(out) => assert_eq!(out, "heppp\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn multibyte_utf8_chars_is_codepoint_aware_not_byte_naive() {
    // "café" -- the 'é' is a 2-byte UTF-8 sequence. `.chars` must yield 4
    // elements (one per character), not 5 (one per byte); `.bytes` must
    // yield 5 (the raw byte count), proving the two are genuinely distinct.
    match run_ruby("puts \"café\".chars.length\nputs \"café\".bytes.length\n") {
        Some(out) => assert_eq!(out, "4\n5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn empty_pattern_sub_gsub_are_no_ops_not_infinite_loops() {
    // The documented degenerate-case guard: an empty search pattern returns
    // the receiver unchanged (never hangs). Also exercises the process exit
    // code (a hang would timeout `run`, not return non-zero).
    match run_ruby("puts \"abc\".sub(\"\", \"X\")\nputs \"abc\".gsub(\"\", \"X\")\n") {
        Some(out) => assert_eq!(out, "abc\nabc\n"),
        None => eprintln!("skip: no cc"),
    }
}
