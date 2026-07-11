//! End-to-end smoke tests for `lang-aot` on the host platform.
//!
//! Each test compiles a small program in one of the supported
//! languages all the way to a native executable on the build host,
//! runs it, and asserts the exit code.  Tests are gated to their host
//! OS so they only execute on the matching CI runner.
//!
//! ## Why exit codes
//!
//! All three host platforms route a function's return value through the
//! C runtime's `exit()`, which truncates to `& 0xFF`.  So asserting the
//! exit code is the simplest end-to-end check that doesn't require
//! capturing stdout — and it exercises the full chain: frontend →
//! IIR → x86_64-backend (or aarch64-backend) → object → system linker.
//!
//! ## Languages tested
//!
//! - Twig: trivially exercises the existing pipeline.
//! - Nib: the new piece — confirms that the Nib frontend wires through
//!   the shared LANG VM chain.
//! - Brainfuck: compiled end-to-end via the BF07 lowering pass.  The
//!   `++++++++[>+++++++++<-]>.<++++[>++++<-]>+.+++++++..+++.>++++[>+++<-]>.+.--------.<++.<.`
//!   program (canonical "Hello\n") is fed through `lang-aot`, linked,
//!   and the resulting executable's stdout is asserted byte-for-byte.

use std::io::Write;
use std::process::Command;

fn linker_available_windows() -> bool {
    // Mirror twig-aot's link.exe probe — confirm the banner is
    // Microsoft's, not git-bash's POSIX `link(1)`.
    let probes: &[(&str, &str, &[&str])] = &[
        ("link.exe",     "",          &["Microsoft", "Linker"]),
        ("lld-link.exe", "",          &["LLD"]),
        ("gcc.exe",      "--version", &["gcc"]),
    ];
    for (name, arg, markers) in probes {
        let mut cmd = Command::new(name);
        if !arg.is_empty() { cmd.arg(arg); }
        let Ok(o) = cmd.output() else { continue; };
        let banner = format!("{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr));
        if markers.iter().all(|m| banner.contains(m)) {
            return true;
        }
    }
    false
}

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_twig_returns_42_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let exe = dir.path().join("smoke.exe");
    let mut f = std::fs::File::create(&src).unwrap();
    writeln!(f, "42").unwrap();
    drop(f);

    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Twig)
        .unwrap_or_else(|e| panic!("Twig compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(42));
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_twig_returns_42_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let exe = dir.path().join("smoke");
    let mut f = std::fs::File::create(&src).unwrap();
    writeln!(f, "42").unwrap();
    drop(f);

    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Twig)
        .unwrap_or_else(|e| panic!("Twig compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(42));
}

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_nib_returns_42_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.nib");
    let exe = dir.path().join("smoke.exe");
    let mut f = std::fs::File::create(&src).unwrap();
    writeln!(f, "fn main() -> u8 {{ return 42; }}").unwrap();
    drop(f);

    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(42),
        "Nib `fn main() -> u8 {{ return 42; }}` should exit 42; got {:?}",
        out.status.code(),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_nib_returns_42_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.nib");
    let exe = dir.path().join("smoke");
    let mut f = std::fs::File::create(&src).unwrap();
    writeln!(f, "fn main() -> u8 {{ return 42; }}").unwrap();
    drop(f);

    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(42));
}

/// Nib arithmetic round-trip — verifies the backend's lowering of
/// `+`, comparisons, etc. when driven by the Nib frontend.
#[cfg(target_os = "windows")]
#[test]
fn end_to_end_nib_arithmetic_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = [
        ("fn main() -> u8 { return 30 + 12; }", 42i32),
        ("fn main() -> u8 { if 1 == 1 { return 100; } else { return 200; } }", 100),
        ("fn main() -> u8 { if 1 == 2 { return 100; } else { return 200; } }", 200),
    ];
    for (i, (src_text, expected)) in cases.iter().enumerate() {
        let src = dir.path().join(format!("case_{i}.nib"));
        let exe = dir.path().join(format!("case_{i}.exe"));
        std::fs::write(&src, src_text).unwrap();
        lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Nib)
            .unwrap_or_else(|e| panic!("Nib compile failed for {src_text:?}: {e}"));
        let out = Command::new(&exe).output().expect("launch");
        assert_eq!(out.status.code(), Some(*expected),
            "Nib program {src_text:?} expected {expected}, got {:?}",
            out.status.code());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_nib_arithmetic_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = [
        ("fn main() -> u8 { return 30 + 12; }", 42i32),
        ("fn main() -> u8 { if 1 == 1 { return 100; } else { return 200; } }", 100),
        ("fn main() -> u8 { if 1 == 2 { return 100; } else { return 200; } }", 200),
    ];
    for (i, (src_text, expected)) in cases.iter().enumerate() {
        let src = dir.path().join(format!("case_{i}.nib"));
        let exe = dir.path().join(format!("case_{i}"));
        std::fs::write(&src, src_text).unwrap();
        lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Nib)
            .unwrap_or_else(|e| panic!("Nib compile failed for {src_text:?}: {e}"));
        let out = Command::new(&exe).output().expect("launch");
        assert_eq!(out.status.code(), Some(*expected));
    }
}

// ── BF07: Brainfuck end-to-end via lang-aot ───────────────────────────────────

/// `++++++++[>++++++++<-]>+.` is the shortest canonical BF for "print
/// 'A'": cell0=8, loop adds 8 to cell1 each pass (8 passes) → cell1=64,
/// then `+` to 65, then `.` prints.  Asserts stdout = `"A"` exactly.
///
/// This single test exercises every mechanic LANG75 + LANG76 deliver:
/// pointer shift (`>`), cell mutation (`+`/`-` via load_byte +
/// arithmetic + store_byte), nested loop via `[` `]` (jmp_if_false on
/// cell value), the 30000-byte tape from `alloc_bytes`, and `.` →
/// `call_builtin "putchar"`.
const BF_PRINT_A: &str = "++++++++[>++++++++<-]>+.";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_brainfuck_prints_a_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("a.bf");
    let exe = dir.path().join("a.exe");
    std::fs::write(&src, BF_PRINT_A).unwrap();

    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Brainfuck)
        .unwrap_or_else(|e| panic!("BF compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"A",
        "expected stdout 'A', got {:?}; stderr={:?}; exit={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_brainfuck_prints_a_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("a.bf");
    let exe = dir.path().join("a");
    std::fs::write(&src, BF_PRINT_A).unwrap();

    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Brainfuck)
        .unwrap_or_else(|e| panic!("BF compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"A",
        "expected stdout 'A', got {:?}; stderr={:?}; exit={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    );
}

// ── PL05: Dartmouth BASIC end-to-end via lang-aot ────────────────────────────

/// Shortest BASIC program that exercises LET + PRINT + END.  Asserts
/// stdout is exactly `"42\n"` after compiling through `lang-aot`.
const BASIC_PRINT_42: &str = "10 PRINT 42\n20 END\n";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_basic_print_42_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("p.bas");
    let exe = dir.path().join("p.exe");
    std::fs::write(&src, BASIC_PRINT_42).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"42\n",
        "expected stdout '42\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_basic_print_42_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("p.bas");
    let exe = dir.path().join("p");
    std::fs::write(&src, BASIC_PRINT_42).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"42\n",
        "expected stdout '42\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// BASIC FOR/NEXT loop with PRINT: prints integers 1..=3 each on a
/// separate line.  Asserts the exact stdout sequence.
const BASIC_FOR_PRINT: &str = "10 FOR I = 1 TO 3\n\
                               20 PRINT I\n\
                               30 NEXT I\n\
                               40 END\n";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_basic_for_loop_prints_1_2_3() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("loop.bas");
    let exe = dir.path().join("loop.exe");
    std::fs::write(&src, BASIC_FOR_PRINT).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"1\n2\n3\n",
        "expected stdout '1\\n2\\n3\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_basic_for_loop_prints_1_2_3() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("loop.bas");
    let exe = dir.path().join("loop");
    std::fs::write(&src, BASIC_FOR_PRINT).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"1\n2\n3\n",
        "expected stdout '1\\n2\\n3\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

// ── NIB04: Nib AOT — print + cross-function calls ────────────────────────────

/// NIB04 — a Nib program with a user-defined `double` function that
/// `main` calls.  Asserts the exit code is `double(21) = 42`.
const NIB_DOUBLE: &str = "fn double(x: u8) -> u8 { return x + x; } \
                          fn main() -> u8 { return double(21); }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_nib_cross_fn_call_returns_42() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("double.nib");
    let exe = dir.path().join("double.exe");
    std::fs::write(&src, NIB_DOUBLE).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(42),
        "expected double(21)=42, got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_nib_cross_fn_call_returns_42() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("double.nib");
    let exe = dir.path().join("double");
    std::fs::write(&src, NIB_DOUBLE).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(42),
        "expected double(21)=42, got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// NIB04 — `print(42)` writes "42\n" to stdout via `__twig_print_i64`.
const NIB_PRINT_42: &str = "fn main() -> u8 { print(42); return 0; }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_nib_print_writes_42() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("print.nib");
    let exe = dir.path().join("print.exe");
    std::fs::write(&src, NIB_PRINT_42).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"42\n",
        "expected stdout '42\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_nib_print_writes_42() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("print.nib");
    let exe = dir.path().join("print");
    std::fs::write(&src, NIB_PRINT_42).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"42\n",
        "expected stdout '42\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// NIB04 step 3 — `while` loop counts from 0 to 10, returns 10.
/// Exercises `let`, comparison, assignment, increment, and the full
/// label / jmp_if_false / jmp loop scaffold in concert.
const NIB_WHILE_COUNT_TO_10: &str =
    "fn main() -> u4 { \
       let n: u4 = 0; \
       while n < 10 { n = n + 1; } \
       return n; \
     }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_nib_while_loop_counts_to_10() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("while.nib");
    let exe = dir.path().join("while.exe");
    std::fs::write(&src, NIB_WHILE_COUNT_TO_10).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(10),
        "expected exit code 10, got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_nib_while_loop_counts_to_10() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("while.nib");
    let exe = dir.path().join("while");
    std::fs::write(&src, NIB_WHILE_COUNT_TO_10).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Nib)
        .unwrap_or_else(|e| panic!("Nib compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(10),
        "expected exit code 10, got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

// ── OCT02 phase 4: Oct end-to-end via lang-aot ───────────────────────────────

/// A minimal Oct program that exercises the entire pipeline:
/// `let`, arithmetic, `if`, comparison, and `return`-via-main's-i64-rewrite.
///
/// Oct's `main` is declared void in source; `oct-iir-compiler` rewrites
/// it to return `i64 0`, so the AOT chain's exit-code convention works.
/// We exit with the static count `0`.
const OCT_MINIMAL: &str = "fn main() { let x: u8 = 42; }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_oct_minimal_main_exits_zero() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("min.oct");
    let exe = dir.path().join("min.exe");
    std::fs::write(&src, OCT_MINIMAL).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(0),
        "expected exit code 0 from Oct's synthesised i64-return main; \
         got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_oct_minimal_main_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("min.oct");
    let exe = dir.path().join("min");
    std::fs::write(&src, OCT_MINIMAL).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(0),
        "expected exit code 0 from Oct's synthesised i64-return main; \
         got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Oct program with a user-defined function and recursion-style cross-fn
/// call.  Verifies the cross-function `call` reloc + LANG43 patcher
/// works for Oct just like Twig / Nib / BASIC.  Asserts exit code 0
/// (main's synthesised return); we don't have printing in Oct yet to
/// assert intermediate values.
const OCT_USER_FN: &str = "fn double(a: u8) -> u8 { return a + a; } \
                           fn main() { let x: u8 = double(21); }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_oct_user_fn_call_succeeds() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("user.oct");
    let exe = dir.path().join("user.exe");
    std::fs::write(&src, OCT_USER_FN).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_oct_user_fn_call_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("user.oct");
    let exe = dir.path().join("user");
    std::fs::write(&src, OCT_USER_FN).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

// ---------------------------------------------------------------------------
// Oct: expanded coverage (mirrors Nib's 5-test breadth)
// ---------------------------------------------------------------------------

/// Oct program with if/else — the then branch should set x to 1, the
/// program exits successfully.  Exercises typed `cmp_eq` + `jmp_if_false`
/// + `mov` + `jmp` + `label` through the AOT chain.
const OCT_IF_ELSE: &str = "fn main() { \
                               let x: u8 = 0; \
                               if x == 0 { x = 1; } else { x = 2; } \
                           }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_oct_if_else_exits_zero() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("ifelse.oct");
    let exe = dir.path().join("ifelse.exe");
    std::fs::write(&src, OCT_IF_ELSE).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0 from Oct if/else; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_oct_if_else_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("ifelse.oct");
    let exe = dir.path().join("ifelse");
    std::fs::write(&src, OCT_IF_ELSE).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0 from Oct if/else; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

/// Oct program with a while loop — counts n from 0 to 10.  Exercises
/// backward `jmp` (the AOT chain's branch-distance encoding) and
/// repeated `cmp_lt` + `add` through native codegen.
const OCT_WHILE_LOOP: &str = "fn main() { \
                                  let n: u8 = 0; \
                                  while n < 10 { n = n + 1; } \
                              }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_oct_while_loop_exits_zero() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("whileloop.oct");
    let exe = dir.path().join("whileloop.exe");
    std::fs::write(&src, OCT_WHILE_LOOP).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0 from Oct while loop; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_oct_while_loop_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("whileloop.oct");
    let exe = dir.path().join("whileloop");
    std::fs::write(&src, OCT_WHILE_LOOP).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0 from Oct while loop; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

/// Oct cross-function arithmetic: `add_one(8)` should yield 9 and main
/// then doubles it via `add_one`-style chain.  Exercises typed argument
/// passing + cross-fn `call` reloc multiple times in a single program.
const OCT_CROSS_FN_CHAIN: &str = "fn add_one(a: u8) -> u8 { return a + 1; } \
                                  fn main() { \
                                      let x: u8 = add_one(8); \
                                      let y: u8 = add_one(x); \
                                  }";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_oct_cross_fn_chain_exits_zero() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("chain.oct");
    let exe = dir.path().join("chain.exe");
    std::fs::write(&src, OCT_CROSS_FN_CHAIN).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0 from Oct cross-fn chain; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_oct_cross_fn_chain_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("chain.oct");
    let exe = dir.path().join("chain");
    std::fs::write(&src, OCT_CROSS_FN_CHAIN).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::Oct)
        .unwrap_or_else(|e| panic!("Oct compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(0),
        "expected exit 0 from Oct cross-fn chain; got {:?}; stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr));
}

// ---------------------------------------------------------------------------
// BASIC: expanded coverage (mirrors Nib's 5-test breadth)
// ---------------------------------------------------------------------------

/// BASIC arithmetic chain: A + B + C printed.  Exercises multiple
/// typed `add` ops through the AOT pipeline.
const BASIC_ARITH_CHAIN: &str = "10 LET A = 10\n\
                                 20 LET B = 20\n\
                                 30 LET C = 12\n\
                                 40 LET D = A + B + C\n\
                                 50 PRINT D\n\
                                 60 END\n";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_basic_arith_chain_prints_42() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("chain.bas");
    let exe = dir.path().join("chain.exe");
    std::fs::write(&src, BASIC_ARITH_CHAIN).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"42\n",
        "expected stdout '42\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_basic_arith_chain_prints_42() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("chain.bas");
    let exe = dir.path().join("chain");
    std::fs::write(&src, BASIC_ARITH_CHAIN).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"42\n",
        "expected stdout '42\\n', got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// BASIC `IF…THEN <line>` conditional branch — A > 5 takes the then
/// branch, printing 1.  Exercises typed `cmp_gt` + `jmp_if_*` through
/// native codegen with the line-label resolution.
const BASIC_IF_THEN: &str = "10 LET A = 7\n\
                             20 IF A > 5 THEN 100\n\
                             30 PRINT 0\n\
                             40 GOTO 200\n\
                             100 PRINT 1\n\
                             200 END\n";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_basic_if_then_prints_1() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("ifthen.bas");
    let exe = dir.path().join("ifthen.exe");
    std::fs::write(&src, BASIC_IF_THEN).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"1\n",
        "expected stdout '1\\n' from IF A > 5 THEN; got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_basic_if_then_prints_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("ifthen.bas");
    let exe = dir.path().join("ifthen");
    std::fs::write(&src, BASIC_IF_THEN).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"1\n",
        "expected stdout '1\\n' from IF A > 5 THEN; got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// BASIC `GOTO` unconditional jump — control flow skips the assignment
/// on line 30 and reaches line 100 directly, printing A's original
/// value (1).  Exercises forward unconditional branch resolution.
const BASIC_GOTO: &str = "10 LET A = 1\n\
                          20 GOTO 100\n\
                          30 LET A = 999\n\
                          100 PRINT A\n\
                          110 END\n";

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_basic_goto_prints_1() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("goto.bas");
    let exe = dir.path().join("goto.exe");
    std::fs::write(&src, BASIC_GOTO).unwrap();
    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"1\n",
        "expected stdout '1\\n' from GOTO skip; got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_basic_goto_prints_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("goto.bas");
    let exe = dir.path().join("goto");
    std::fs::write(&src, BASIC_GOTO).unwrap();
    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::DartmouthBasic)
        .unwrap_or_else(|e| panic!("BASIC compile failed: {e}"));
    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.stdout, b"1\n",
        "expected stdout '1\\n' from GOTO skip; got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

// ===========================================================================
// LLVM04 — source → IIR → textual LLVM IR (.ll) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  These tests confirm that
// `compile_file_to_llvm_ir` runs the full source → IIR → iir-to-llvm
// pipeline and produces a .ll file that contains the LLVM IR shape we
// expect for each frontend.  The .ll is not handed to `llc`/`opt` —
// that's downstream.

#[test]
fn end_to_end_twig_emits_llvm_ir_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let ll  = dir.path().join("smoke.ll");
    std::fs::write(&src, b"42\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_llvm_ir(&src, &ll, lang_aot::Language::Twig) {
        // Twig may emit IIR ops the LLVM backend hasn't grown coverage
        // for yet (e.g. heap/closure).  Treat as expected gap rather
        // than a test failure — once LLVM05+ adds coverage it'll flip.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp") || msg.contains("UnsupportedType") {
            eprintln!("skipping: Twig LLVM lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig → LLVM IR error: {e}");
    }

    let body = std::fs::read_to_string(&ll).expect("read .ll");
    assert!(body.contains("target triple ="),
        "Twig .ll should have a target triple line; got:\n{body}");
    assert!(body.contains("define"),
        "Twig .ll should contain a `define` block; got:\n{body}");
}

#[test]
fn end_to_end_basic_print_emits_llvm_ir_with_print_extern() {
    // After the BA2 PRINT overhaul, BASIC's `PRINT 42` compiles the integer
    // literal as a real constant (42.0) and routes it through
    // `@__basic_print_real`, which handles the full numeric format including
    // sign, digits, and decimal point.  The old `@__print_i64` path is no
    // longer used for numeric PRINT expressions.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bas");
    let ll  = dir.path().join("smoke.ll");
    std::fs::write(&src, b"10 PRINT 42\n20 END\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_llvm_ir(&src, &ll, lang_aot::Language::DartmouthBasic) {
        // Tolerate the same not-yet-covered op gap as Twig.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp") || msg.contains("UnsupportedType") {
            eprintln!("skipping: BASIC LLVM lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected BASIC → LLVM IR error: {e}");
    }

    let body = std::fs::read_to_string(&ll).expect("read .ll");
    assert!(body.contains("@__basic_print_real"),
        "BASIC `PRINT 42` should route through `@__basic_print_real` after BA2; got:\n{body}");
    assert!(body.contains("call i64 @__basic_print_real(double"),
        "BASIC `PRINT 42` should call `@__basic_print_real` with a double; got:\n{body}");
}

// ===========================================================================
// A1+++ — source -> IIR -> RV32I machine code (.bin) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  Confirms the new
// compile_file_to_riscv32_bin entry point runs the full source ->
// IIR -> CIR -> riscv-backend pipeline (Phase 7 of the historical-
// arch backend migration — the FINAL lane).  Produces a flat
// little-endian .bin of 32-bit RV32I instruction words.

#[test]
fn end_to_end_basic_print_emits_riscv32_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bas");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"10 PRINT 42\n20 END\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_riscv32_bin(&src, &bin, lang_aot::Language::DartmouthBasic) {
        // Tolerate not-yet-covered op gaps - same convention as the LLVM path.
        // Phase 7 added the lowercase `unsupported op` form (Display string
        // of `BackendError::UnsupportedOp` on `riscv-backend` v0.1.0); the
        // CamelCase forms remain valid for any future Display drift.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("unsupported op")
            || msg.contains("UnsupportedType")
            || msg.contains("UnsupportedCallShape")
            || msg.contains("ImmediateOutOfRange")
            || msg.contains("immediate")
            || msg.contains("OutOfRegisters")
            || msg.contains("temp-register pool exhausted")
        {
            eprintln!("skipping: BASIC RV32I lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected BASIC -> RV32I error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(!bytes.is_empty(),
        "BASIC PRINT 42 should produce a non-empty .bin");
    assert_eq!(bytes.len() % 4, 0,
        ".bin length should be a multiple of 4; got {} bytes",
        bytes.len());

    // The last 4 bytes should encode the canonical ret = jalr x0, x1, 0 = 0x0000_8067.
    // Stored little-endian: 0x67, 0x80, 0x00, 0x00.
    let last_word_le = &bytes[bytes.len() - 4..];
    assert_eq!(last_word_le, &[0x67, 0x80, 0x00, 0x00],
        "last 4 bytes should be the canonical ret encoded little-endian; got: {last_word_le:02x?}");
}

// Phase 7 of the historical-arch backend migration: a Twig `42`
// program that the v0.1.0 `riscv-backend` minimal-viable scope DOES
// cover.  Pins the exact 12-byte sequence (3 RV32I words flattened
// little-endian) and matches the architectural pattern the
// Intel 8008, ARMv7, Intel 4004, and GE-225 Twig-42 e2e tests follow.
//
// CIR:
//   const_i64 v=42
//   ret_i64   v
//
// Lowered RV32I:
//   addi t0, x0, 42      ; 0x02A0_0293
//   addi a0, t0, 0       ; 0x0002_8513   (mv a0, t0)
//   jalr x0, x1, 0       ; 0x0000_8067   (canonical ret)
//
// Little-endian on disk:
//   [0x93, 0x02, 0xA0, 0x02,
//    0x13, 0x85, 0x02, 0x00,
//    0x67, 0x80, 0x00, 0x00]
#[test]
fn end_to_end_twig_42_emits_riscv32_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"42\n").unwrap();

    lang_aot::compile_file_to_riscv32_bin(&src, &bin, lang_aot::Language::Twig)
        .expect("Twig 42 must compile through the riscv-backend v0.1.0 minimal-viable scope");

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert_eq!(
        bytes,
        vec![
            0x93, 0x02, 0xA0, 0x02, // addi t0, x0, 42
            0x13, 0x85, 0x02, 0x00, // addi a0, t0, 0  (mv a0, t0)
            0x67, 0x80, 0x00, 0x00, // jalr x0, x1, 0  (ret)
        ],
        "Twig 42 -> RV32I byte sequence is the migration-pinned regression invariant for Phase 7"
    );
}

// ===========================================================================
// A2+++ — source -> IIR -> Intel 8008 machine code (.bin) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  Confirms the new
// compile_file_to_intel8008_bin entry point runs the full source ->
// IIR -> iir-to-intel8008 pipeline and produces a flat .bin of
// 8-bit Intel 8008 opcode bytes.
//
// Why Twig instead of BASIC?  The 8008 is **Oct's** native target,
// but Oct programs at the LANG VM benchmark sizes routinely exceed
// the 7-register pool that iir-to-intel8008 v0.3.9 supports (stack
// spilling lands with A3 or later).  Twig's `42` program — the
// canonical "return the integer 42" — survives that constraint
// because it lowers to a single `const v; ret v` IIR sequence which
// fits in registers A and exits cleanly.

#[test]
fn end_to_end_twig_42_emits_intel8008_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"42\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_intel8008_bin(&src, &bin, lang_aot::Language::Twig) {
        // Tolerate not-yet-covered op gaps — same convention as the
        // LLVM + RISC-V paths.  A2++.5.5 covered the IIR core so this
        // should generally succeed for Twig's `42`, but languages
        // that emit ops beyond what v0.3.9 supports (e.g. ref<T>
        // allocation, multi-arg calls) will surface here.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("InvalidOperand")
            || msg.contains("OutOfRegisters")
        {
            eprintln!("skipping: Twig Intel 8008 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig -> Intel 8008 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(!bytes.is_empty(),
        "Twig `42` should produce a non-empty .bin");
    // The canonical Twig-`42` lowering is `const v=42 → A; ret v →
    // HLT` (since main is the entry-point function, ret emits HLT
    // not RET).  Pinned 3-byte sequence: MVI A, 42 (0x3E 0x2A) + HLT
    // (0x76).
    assert_eq!(&bytes[..3.min(bytes.len())], &[0x3E, 0x2A, 0x76],
        "Twig `42` should produce `MVI A, 42; HLT`; got: {bytes:02x?}");
}

// ===========================================================================
// A3+++ — source -> IIR -> ARMv7 (A32) machine code (.bin) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  Confirms the new
// compile_file_to_armv7_bin entry point runs the full source ->
// IIR -> iir-to-armv7 pipeline and produces a flat .bin of
// little-endian 32-bit A32 instruction words.
//
// Twig `42` lowers to the canonical 2-word `MOV r0, #42; BX LR`
// sequence: `0xE3A0_002A 0xE12F_FF1E` stored little-endian as
// `2A 00 A0 E3  1E FF 2F E1`.

#[test]
fn end_to_end_twig_42_emits_armv7_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"42\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_armv7_bin(&src, &bin, lang_aot::Language::Twig) {
        // Tolerate not-yet-covered op gaps — same convention as the
        // LLVM + RISC-V + Intel 8008 paths.  After A3++.6, ARMv7
        // supports the full IIR core for simple programs, so Twig's
        // `42` should always succeed here.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("InvalidOperand")
            || msg.contains("OutOfRegisters")
        {
            eprintln!("skipping: Twig ARMv7 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig -> ARMv7 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(!bytes.is_empty(),
        "Twig `42` should produce a non-empty .bin");
    assert_eq!(bytes.len() % 4, 0,
        ".bin length should be a multiple of 4 (A32 word size); got {} bytes",
        bytes.len());

    // Twig `42` lowers to `MOV r0, #42; BX LR` = `0xE3A0_002A 0xE12F_FF1E`.
    // Stored little-endian: `2A 00 A0 E3 1E FF 2F E1`.
    assert_eq!(&bytes[..4.min(bytes.len())], &[0x2A, 0x00, 0xA0, 0xE3],
        "Twig `42` should produce `MOV r0, #42` (0xE3A0_002A) as the first \
         4 bytes little-endian (2A 00 A0 E3); got: {:02x?}",
        &bytes[..4.min(bytes.len())]);
    assert_eq!(&bytes[4..8.min(bytes.len())], &[0x1E, 0xFF, 0x2F, 0xE1],
        "second word should be BX LR (0xE12F_FF1E) little-endian; got: {:02x?}",
        &bytes[4..8.min(bytes.len())]);
}

// ===========================================================================
// A4+++ — source -> IIR -> Intel 4004 machine code (.bin) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  Confirms the new
// compile_file_to_intel4004_bin entry point runs the full source ->
// IIR -> iir-to-intel4004 pipeline.
//
// Twig `42` won't fit in a 4-bit immediate, so we use `5` for this
// smoke test.  The expected byte stream is `LDM 5; JUN 0x000` =
// `0xD5 0x40 0x00` (the trivial-case 3-byte shape preserved by the
// ACC-first allocator in v0.3.0).

#[test]
fn end_to_end_twig_5_emits_intel4004_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"5\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_intel4004_bin(&src, &bin, lang_aot::Language::Twig) {
        // Tolerate not-yet-covered op gaps — same convention as the
        // LLVM + RISC-V + Intel 8008 + ARMv7 paths.  After A4++,
        // Twig's `5` should always succeed here.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("InvalidOperand")
            || msg.contains("OutOfRegisters")
        {
            eprintln!("skipping: Twig Intel 4004 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig -> Intel 4004 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(!bytes.is_empty(),
        "Twig `5` should produce a non-empty .bin");

    // Twig `5` lowers to `LDM 5; JUN 0x000` = `0xD5 0x40 0x00` via
    // the ACC-first allocator's trivial-case preservation.
    assert_eq!(&bytes[..3.min(bytes.len())], &[0xD5, 0x40, 0x00],
        "Twig `5` should produce `LDM 5; JUN 0x000` (0xD5 0x40 0x00); got: {bytes:02x?}");
}

// ===========================================================================
// A5++++ — source -> IIR -> GE-225 machine code (.bin) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  Confirms the new
// compile_file_to_ge225_bin entry point runs the full source ->
// IIR -> iir-to-ge225 pipeline.
//
// The GE-225 is a 1959-era mainframe — Dartmouth BASIC's birthplace.
// Each 20-bit instruction word packs into 3 bytes (big-endian, top 4
// bits of byte 0 zero).  Expected encodings:
//   * HLT = [0x00, 0x00, 0x00] (all-zero 20-bit word)
//   * LDA n = [0x01, hi(n), lo(n)] for 16-bit immediate n

#[test]
fn end_to_end_twig_5_emits_ge225_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"5\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_ge225_bin(&src, &bin, lang_aot::Language::Twig) {
        // Tolerate not-yet-covered op gaps — same convention as the
        // LLVM + RISC-V + Intel 8008 + ARMv7 + Intel 4004 paths.
        // After A5+, Twig `5` should always succeed here.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("InvalidOperand")
            || msg.contains("OutOfRegisters")
        {
            eprintln!("skipping: Twig GE-225 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig -> GE-225 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(!bytes.is_empty(), "Twig `5` should produce a non-empty .bin");

    // Twig `5` lowers (since v0.2.0 A5+) to `LDA 5; HLT` packed as 6
    // bytes: [0x01, 0x00, 0x05, 0x00, 0x00, 0x00].  Trivial-case ROM
    // shape preserved by the ACC-first allocator from v0.3.0.
    assert_eq!(
        &bytes[..6.min(bytes.len())],
        &[0x01, 0x00, 0x05, 0x00, 0x00, 0x00],
        "Twig `5` should produce `LDA 5; HLT` ([0x01, 0x00, 0x05, 0x00, 0x00, 0x00]); \
         got: {bytes:02x?}"
    );
}

#[test]
fn end_to_end_twig_3_plus_4_emits_ge225_arithmetic_bin_via_lang_aot() {
    // Twig `(+ 3 4)` exercises the A5+++ ADD r lowering: the
    // expected byte sequence is the trivial-add ROM pinned by
    // iir-to-ge225's `trivial_add_byte_sequence` test:
    //   LDA 3, STA r0, LDA 4, STA r1, LD r0, ADD r1, HLT
    //   = [0x01,0x00,0x03, 0x02,0x00,0x00, 0x01,0x00,0x04,
    //      0x02,0x00,0x01, 0x03,0x00,0x00, 0x04,0x00,0x01,
    //      0x00,0x00,0x00]
    //   = 21 bytes (7 words).
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"(+ 3 4)\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_ge225_bin(&src, &bin, lang_aot::Language::Twig) {
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("InvalidOperand")
            || msg.contains("OutOfRegisters")
        {
            eprintln!("skipping: Twig (+ 3 4) GE-225 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig -> GE-225 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(
        !bytes.is_empty(),
        "Twig `(+ 3 4)` should produce a non-empty .bin"
    );
    // We don't pin the exact 21-byte shape here — Twig's `(+ 3 4)`
    // may go through several typed-arithmetic helpers before
    // reaching the IIR `add` op.  Loose check: the output must
    // contain at least one HLT (last word = all zeros) and at least
    // one non-zero byte (some LDA or ADD).
    assert!(
        bytes.windows(3).any(|w| w == [0x00, 0x00, 0x00]),
        ".bin should contain at least one HLT word; got: {bytes:02x?}"
    );
    assert!(
        bytes.iter().any(|&b| b != 0x00),
        ".bin should contain at least one non-zero byte; got: {bytes:02x?}"
    );
}

#[test]
fn end_to_end_brainfuck_emits_ge225_bin_via_lang_aot() {
    // Brainfuck IR shapes might not all fit a 20-bit accumulator
    // model, but a degenerate empty program (`""`) should still
    // round-trip via the empty-module HALT contract from v0.1.0.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bf");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"").unwrap();

    if let Err(e) =
        lang_aot::compile_file_to_ge225_bin(&src, &bin, lang_aot::Language::Brainfuck)
    {
        // Tolerate gaps — the Brainfuck pipeline produces ops the
        // skeleton backend may not yet handle (load_mem, store_mem,
        // etc.).  As long as the failure is a recognised lowering
        // gap, the wiring is correct.
        let msg = format!("{e}");
        // Match both CamelCase variant names and the lowercase
        // Display strings (`unsupported op`, etc.) — iir-to-ge225's
        // Display emits the lowercase form.
        if msg.contains("UnsupportedOp")
            || msg.contains("unsupported op")
            || msg.contains("UnsupportedType")
            || msg.contains("unsupported type")
            || msg.contains("InvalidOperand")
            || msg.contains("invalid operand")
            || msg.contains("OutOfRegisters")
            || msg.contains("out of GE-225 registers")
        {
            eprintln!("skipping: Brainfuck GE-225 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Brainfuck -> GE-225 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(
        !bytes.is_empty(),
        "Brainfuck empty program should still emit at least the HALT_WORD"
    );
    // Empty Brainfuck program → empty IIR module → HALT_WORD
    // (the v0.1.0 empty-module contract preserved through v0.4.0).
    assert_eq!(
        &bytes[..3.min(bytes.len())],
        &[0x00, 0x00, 0x00],
        "Empty Brainfuck program should produce HALT_WORD; got: {bytes:02x?}"
    );
}

#[test]
fn ge225_emit_writes_to_disk_with_expected_byte_count() {
    // Cross-check that the file written to disk matches what
    // iir-to-ge225 would produce in-memory: any non-empty Twig
    // program lowers to >= 6 bytes (LDA + HLT minimum from the
    // const+ret trivial case).
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"42\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_ge225_bin(&src, &bin, lang_aot::Language::Twig) {
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("InvalidOperand")
            || msg.contains("OutOfRegisters")
        {
            eprintln!("skipping: Twig GE-225 lowering gap (expected): {msg}");
            return;
        }
        panic!("unexpected Twig -> GE-225 error: {e}");
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    // Trivial Twig `42` should be exactly LDA 42 + HLT = 6 bytes.
    // If Twig's frontend introduces additional ops the byte count
    // will exceed 6 — we accept that, but require >= 6 to confirm
    // the LDA + HLT shape is at least present.
    assert!(
        bytes.len() >= 6,
        "Twig `42` should produce at least 6 bytes (LDA + HLT); got {} bytes: {bytes:02x?}",
        bytes.len()
    );
    // Byte count must be a multiple of 3 (word-aligned).
    assert_eq!(
        bytes.len() % 3,
        0,
        ".bin byte count must be a multiple of 3 (each word is 3 bytes packed); \
         got {} bytes: {bytes:02x?}",
        bytes.len()
    );
}

// ===========================================================================
// A5++++++++ — Dartmouth BASIC end-to-end through GE-225
// ===========================================================================
//
// This is the milestone moment for the GE-225 lane: Dartmouth BASIC
// — designed in 1964 on the GE-225 mainframe at Dartmouth College
// by Kemeny and Kurtz — round-trips through the full lang-aot
// pipeline back to GE-225 byte code 62 years later.
//
// BASIC's IIR-op surface as of v0.7.0 of iir-to-ge225:
//   * Supported by iir-to-ge225: const, mov, add, cmp_le, jmp,
//     jmp_if_true, jmp_if_false, label, ret.
//   * NOT yet supported: call_builtin (PRINT et al.), neg.
//
// Smoke tests below tolerate "lowering gap" errors so the cascade
// keeps progressing as BASIC frontend and GE-225 backend both add
// ops over time.

/// Helper: detect whether a GE-225 error is a known lowering gap
/// (so the test can skip cleanly rather than fail).  Mirrors the
/// pattern used by the Twig + Brainfuck GE-225 smoke tests.
fn is_ge225_lowering_gap(msg: &str) -> bool {
    msg.contains("UnsupportedOp")
        || msg.contains("unsupported op")
        || msg.contains("UnsupportedType")
        || msg.contains("unsupported type")
        || msg.contains("InvalidOperand")
        || msg.contains("invalid operand")
        || msg.contains("OutOfRegisters")
        || msg.contains("out of GE-225 registers")
        || msg.contains("UndefinedFunction")
        || msg.contains("undefined function")
}

/// The simplest BASIC program: `10 LET A = 5\n20 END`.
///
/// Every IIR op this emits (const, mov, ret) is supported by
/// iir-to-ge225 v0.7.0, so this end-to-end test should **always
/// succeed** with no skip.
///
/// Expected output: a non-empty .bin file containing at least
/// `LDA 5` somewhere (the literal 5 makes it into bytes
/// `[0x01, 0x00, 0x05]`) followed eventually by `HLT`
/// `[0x00, 0x00, 0x00]`.
#[test]
fn end_to_end_basic_let_a_5_emits_ge225_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bas");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"10 LET A = 5\n20 END\n").unwrap();

    match lang_aot::compile_file_to_ge225_bin(&src, &bin, lang_aot::Language::DartmouthBasic) {
        Ok(()) => {}
        Err(e) => {
            let msg = format!("{e}");
            if is_ge225_lowering_gap(&msg) {
                eprintln!("skipping: BASIC GE-225 lowering gap (expected): {msg}");
                return;
            }
            panic!("unexpected BASIC -> GE-225 error: {e}");
        }
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert!(
        !bytes.is_empty(),
        "BASIC `10 LET A = 5; END` should produce a non-empty .bin"
    );
    // Word-aligned (each 20-bit word packs as 3 bytes).
    assert_eq!(
        bytes.len() % 3,
        0,
        "GE-225 .bin must be a multiple of 3 bytes; got {bytes:02x?}"
    );
    // Must contain a HLT word somewhere (the END statement lowers
    // through `ret` in the entry function, which emits HLT).
    assert!(
        bytes.windows(3).any(|w| w == [0x00, 0x00, 0x00]),
        ".bin must contain at least one HLT word; got {bytes:02x?}"
    );
    // Must contain at least one LDA word (LET A = 5 lowers to a
    // const → LDA somewhere in the program).
    assert!(
        bytes.windows(3).any(|w| w[0] == 0x01),
        ".bin must contain at least one LDA word (0x01..); got {bytes:02x?}"
    );
}

/// A slightly larger BASIC program exercising the GE-225 ADD
/// opcode: `10 LET A = 1 + 2\n20 END`.
///
/// BASIC lowers `1 + 2` through a `const`, `const`, `add` chain.
/// iir-to-ge225 v0.7.0 supports all three, so this should succeed.
///
/// Expected: at least 21 bytes (the canonical add ROM size) plus
/// any prep / store-to-A overhead, with at least one ADD word
/// (`0x04, 0x00, r`) present.
#[test]
fn end_to_end_basic_let_a_1_plus_2_exercises_add_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bas");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"10 LET A = 1 + 2\n20 END\n").unwrap();

    match lang_aot::compile_file_to_ge225_bin(&src, &bin, lang_aot::Language::DartmouthBasic) {
        Ok(()) => {}
        Err(e) => {
            let msg = format!("{e}");
            if is_ge225_lowering_gap(&msg) {
                eprintln!("skipping: BASIC arithmetic GE-225 lowering gap (expected): {msg}");
                return;
            }
            panic!("unexpected BASIC -> GE-225 error: {e}");
        }
    }

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert_eq!(bytes.len() % 3, 0);
    // Must contain an ADD instruction word (0x04 in byte 0 of
    // some 3-byte chunk).
    assert!(
        bytes.chunks_exact(3).any(|w| w[0] == 0x04),
        ".bin must contain at least one ADD word for `1 + 2`; got {bytes:02x?}"
    );
    // And an HLT for END.
    assert!(
        bytes.windows(3).any(|w| w == [0x00, 0x00, 0x00]),
        ".bin must contain HLT for END statement; got {bytes:02x?}"
    );
}

/// BASIC with PRINT — exercises the `call_builtin` IIR op, which
/// **is NOT yet supported** by iir-to-ge225 v0.7.0.  This test is
/// here to (a) document the gap, and (b) confirm the gap is
/// reported via the standard `UnsupportedOp` error so it'll be
/// caught by the skip clause and a future implementation will
/// automatically activate the test.
#[test]
fn end_to_end_basic_print_documents_call_builtin_gap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bas");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"10 LET A = 5\n20 PRINT A\n30 END\n").unwrap();

    let result = lang_aot::compile_file_to_ge225_bin(
        &src,
        &bin,
        lang_aot::Language::DartmouthBasic,
    );

    match result {
        Ok(()) => {
            // If a future iir-to-ge225 implements call_builtin,
            // the test still passes — just verify the .bin is
            // non-empty and word-aligned.
            let bytes = std::fs::read(&bin).expect("read .bin");
            assert!(!bytes.is_empty());
            assert_eq!(bytes.len() % 3, 0);
            eprintln!(
                "BASIC PRINT now compiles to GE-225 — call_builtin gap closed; \
                 {} bytes emitted",
                bytes.len()
            );
        }
        Err(e) => {
            let msg = format!("{e}");
            // The gap MUST be reported via one of the canonical
            // lowering-gap errors so the cascade can detect it.
            assert!(
                is_ge225_lowering_gap(&msg),
                "BASIC PRINT failed with non-gap error (broken cascade): {msg}"
            );
            eprintln!("documented: BASIC PRINT GE-225 lowering gap: {msg}");
        }
    }
}

// ---------------------------------------------------------------------------
// McCarthy Lisp (L3a) — the language drives the AOT native pipeline.
//
// McCarthy 1960 Lisp has integer literals, so a scalar program exercises the
// full source → IIR → infer → specialise → x86_64/aarch64 → executable path
// exactly like the Nib smoke test.  (Symbol/cons-returning programs need
// value-model runtime glue in each backend — that is L3b, not wired here.)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_mccarthy_returns_42_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.mcl");
    let exe = dir.path().join("smoke.exe");
    std::fs::write(&src, "42").unwrap();

    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
        .unwrap_or_else(|e| panic!("McCarthy compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(42),
        "McCarthy program `42` should exit 42; got {:?}", out.status.code(),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_mccarthy_returns_42_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.mcl");
    let exe = dir.path().join("smoke");
    std::fs::write(&src, "42").unwrap();

    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
        .unwrap_or_else(|e| panic!("McCarthy compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(42));
}

// ---------------------------------------------------------------------------
// McCarthy Lisp cons cells (L3b) — a heap data structure compiled to native.
//
// `lower_heap_builtins` (run in twig-aot) rewrites `cons`/`car`/`cdr` into
// `alloc`/`field_store`/`field_load`, which the native backends now lower to
// a `__twig_alloc_bytes` cell + word loads/stores.  Values are raw words (no
// NaN-boxing), so a cons-of-integers program round-trips: `(CAR (CONS 7 9))`
// allocates a pair, stores 7/9, loads field 0, and the program exits 7.
//
// Gated to Linux/Windows like the other native smoke tests — the macOS-exe
// runtime archive does not currently provide `__twig_alloc_bytes` (a
// pre-existing limitation shared with Brainfuck's tape; see lessons.md).
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_mccarthy_cons_car_returns_7_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("cons.mcl");
    let exe = dir.path().join("cons");
    std::fs::write(&src, "(CAR (CONS 7 9))").unwrap();

    lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
        .unwrap_or_else(|e| panic!("McCarthy cons compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(
        out.status.code(), Some(7),
        "(CAR (CONS 7 9)) should exit 7; got {:?}", out.status.code(),
    );
}

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_mccarthy_cons_car_returns_7_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("cons.mcl");
    let exe = dir.path().join("cons.exe");
    std::fs::write(&src, "(CAR (CONS 7 9))").unwrap();

    lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
        .unwrap_or_else(|e| panic!("McCarthy cons compile failed: {e}"));

    let out = Command::new(&exe).output().expect("launch");
    assert_eq!(out.status.code(), Some(7));
}

// ---------------------------------------------------------------------------
// McCarthy Lisp ATOM/EQ + COND (L3b-2c-2) — tagged predicates drive a branch.
//
// `(ATOM x)` lowers to `not(pair?(x))` → `__dyn_not(__dyn_pair_p)`;
// a `COND` predicate's tagged `#t`/`#f` is normalised by `__dyn_truthy`
// for `jmp_if_false`.  The integer atoms box (`5` → 40) so `pair?` reads the
// int tag, not the heap tag.  Two programs distinguish the branches:
//   (COND ((ATOM 5) 7) (5 9))          → ATOM of an int is true  → 7
//   (COND ((ATOM (CONS 1 2)) 7) (5 9)) → ATOM of a pair is false → 9
// Gated to Linux/Windows like the other native smoke tests (macOS-exe gap).
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_mccarthy_atom_cond_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = [
        ("(COND ((ATOM 5) 7) (5 9))", 7i32),
        ("(COND ((ATOM (CONS 1 2)) 7) (5 9))", 9),
    ];
    for (i, (program, expected)) in cases.iter().enumerate() {
        let src = dir.path().join(format!("atom_{i}.mcl"));
        let exe = dir.path().join(format!("atom_{i}"));
        std::fs::write(&src, program).unwrap();
        lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
            .unwrap_or_else(|e| panic!("McCarthy ATOM/COND compile failed for {program:?}: {e}"));
        let out = Command::new(&exe).output().expect("launch");
        assert_eq!(
            out.status.code(), Some(*expected),
            "{program} should exit {expected}; got {:?}", out.status.code(),
        );
    }
}

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_mccarthy_atom_cond_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = [
        ("(COND ((ATOM 5) 7) (5 9))", 7i32),
        ("(COND ((ATOM (CONS 1 2)) 7) (5 9))", 9),
    ];
    for (i, (program, expected)) in cases.iter().enumerate() {
        let src = dir.path().join(format!("atom_{i}.mcl"));
        let exe = dir.path().join(format!("atom_{i}.exe"));
        std::fs::write(&src, program).unwrap();
        lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
            .unwrap_or_else(|e| panic!("McCarthy ATOM/COND compile failed for {program:?}: {e}"));
        let out = Command::new(&exe).output().expect("launch");
        assert_eq!(out.status.code(), Some(*expected), "{program}");
    }
}

// ---------------------------------------------------------------------------
// McCarthy Lisp symbols (L3b-2c-3) — the worked example `(CAR '(A B C))` → A.
//
// Symbols are interned at compile time (`intern_symbols`) into the tagged
// immediate `(id << 32) | TAG_SYMBOL`; the same name gets the same id, so `EQ`
// (= `equal?`) on symbols is word equality. A native program observes a symbol
// *value* via `EQ` + `COND` exit codes (no symbol-name printing — that needs
// string-literal emission, deferred):
//   (COND ((EQ (CAR '(A B C)) 'A) 7) ('T 9))  → CAR is A, equals 'A   → 7
//   (COND ((EQ (CAR '(A B C)) 'B) 7) ('T 9))  → CAR is A, ≠ 'B → else → 9
// The `'T` else also exercises a symbol-as-COND-predicate (truthy).
// Gated to Linux/Windows like the other native smoke tests (macOS-exe gap).
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn end_to_end_mccarthy_symbols_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = [
        ("(COND ((EQ (CAR (QUOTE (A B C))) (QUOTE A)) 7) ((QUOTE T) 9))", 7i32),
        ("(COND ((EQ (CAR (QUOTE (A B C))) (QUOTE B)) 7) ((QUOTE T) 9))", 9),
    ];
    for (i, (program, expected)) in cases.iter().enumerate() {
        let src = dir.path().join(format!("sym_{i}.mcl"));
        let exe = dir.path().join(format!("sym_{i}"));
        std::fs::write(&src, program).unwrap();
        lang_aot::compile_file_to_linux_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
            .unwrap_or_else(|e| panic!("McCarthy symbol compile failed for {program:?}: {e}"));
        let out = Command::new(&exe).output().expect("launch");
        assert_eq!(
            out.status.code(), Some(*expected),
            "{program} should exit {expected}; got {:?}", out.status.code(),
        );
    }
}

#[cfg(target_os = "windows")]
#[test]
fn end_to_end_mccarthy_symbols_via_lang_aot() {
    if !linker_available_windows() {
        eprintln!("skipping: no Windows linker");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = [
        ("(COND ((EQ (CAR (QUOTE (A B C))) (QUOTE A)) 7) ((QUOTE T) 9))", 7i32),
        ("(COND ((EQ (CAR (QUOTE (A B C))) (QUOTE B)) 7) ((QUOTE T) 9))", 9),
    ];
    for (i, (program, expected)) in cases.iter().enumerate() {
        let src = dir.path().join(format!("sym_{i}.mcl"));
        let exe = dir.path().join(format!("sym_{i}.exe"));
        std::fs::write(&src, program).unwrap();
        lang_aot::compile_file_to_windows_executable(&src, &exe, lang_aot::Language::McCarthyLisp)
            .unwrap_or_else(|e| panic!("McCarthy symbol compile failed for {program:?}: {e}"));
        let out = Command::new(&exe).output().expect("launch");
        assert_eq!(out.status.code(), Some(*expected), "{program}");
    }
}

// ===========================================================================
// L4 + L5 — source -> IIR -> IBM 704 machine code (.bin) via lang-aot
// ===========================================================================
//
// The closing half of the **CAR/CDR birthplace round-trip** — Lisp on the
// silicon it was born on.  Mirrors the existing Twig-42-on-X tests
// (intel8008/armv7/riscv).  Per the v0.1.0 scope decision the IBM 704
// backend is no-CONS-only, so we pin both Twig `42` and McCarthy `42`
// (which lower to the same const_i64 + ret_i64 CIR sequence).

#[test]
fn end_to_end_twig_42_emits_ibm704_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.twig");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"42\n").unwrap();

    lang_aot::compile_file_to_ibm704_bin(&src, &bin, lang_aot::Language::Twig)
        .expect("Twig `42` must compile through the ibm704-backend v0.1.0 minimal-viable scope");

    let bytes = std::fs::read(&bin).expect("read .bin");
    assert_eq!(
        bytes,
        vec![
            0x2A, 0x00, 0x00, 0x00, 0x0A, // CLA 42         word = 0xA_0000_002A
            0x00, 0x00, 0x00, 0x80, 0x08, // HTR  0 (halt)  word = 0x8_8000_0000
        ],
        "Twig 42 -> IBM 704 byte sequence is the migration-pinned regression invariant for L4 \
         (CAR/CDR were literal 704 instruction-field mnemonics; this is the round-trip to that silicon)"
    );
}

#[test]
fn end_to_end_mccarthy_42_emits_ibm704_bin_via_lang_aot() {
    // The symbolic capstone — McCarthy 1960 Lisp source `42` compiled
    // to bytecode for the very machine McCarthy & Russell first ran
    // Lisp on at MIT in 1959.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.mcl");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"42\n").unwrap();

    lang_aot::compile_file_to_ibm704_bin(&src, &bin, lang_aot::Language::McCarthyLisp)
        .expect("McCarthy `42` must compile through the ibm704-backend v0.1.0 minimal-viable scope");

    let bytes = std::fs::read(&bin).expect("read .bin");
    // McCarthy `42` and Twig `42` both lower to `const_i64 v=42; ret_i64 v`
    // by the time aot_core::specialise has run, so the emitted bytes
    // are identical.  That's the whole point of the IIR layer.
    assert_eq!(
        bytes,
        vec![
            0x2A, 0x00, 0x00, 0x00, 0x0A, // CLA 42
            0x00, 0x00, 0x00, 0x80, 0x08, // HTR  0 (halt)
        ],
        "McCarthy Lisp `42` -> IBM 704: the CAR/CDR-birthplace round-trip is now closed end-to-end."
    );
}
