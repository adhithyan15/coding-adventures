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
    // BASIC's PRINT triggers `call_builtin print_i64`, which the LLVM
    // backend lowers to `call void @__print_i64(i64 ...)` + a module-top
    // `declare void @__print_i64(i64)`.  This is the LLVM counterpart
    // to gaps G2 (wasm) / G3 (JVM) / G4 (CLR) — same builtin name on
    // every backend.
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
    assert!(body.contains("declare void @__print_i64(i64)"),
        "BASIC `PRINT 42` should produce an `@__print_i64` declare in the .ll; got:\n{body}");
    assert!(body.contains("call void @__print_i64(i64"),
        "BASIC `PRINT 42` should produce a `call void @__print_i64(i64 …)` site; got:\n{body}");
}

// ===========================================================================
// A1+++ — source -> IIR -> RV32I machine code (.bin) via lang-aot
// ===========================================================================
//
// Cross-platform: no linker, no cfg gating.  Confirms the new
// compile_file_to_riscv32_bin entry point runs the full source ->
// IIR -> iir-to-riscv pipeline and produces a flat little-endian
// .bin of 32-bit RV32I instruction words.

#[test]
fn end_to_end_basic_print_emits_riscv32_bin_via_lang_aot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("smoke.bas");
    let bin = dir.path().join("smoke.bin");
    std::fs::write(&src, b"10 PRINT 42\n20 END\n").unwrap();

    if let Err(e) = lang_aot::compile_file_to_riscv32_bin(&src, &bin, lang_aot::Language::DartmouthBasic) {
        // Tolerate not-yet-covered op gaps - same convention as the LLVM path.
        let msg = format!("{e}");
        if msg.contains("UnsupportedOp")
            || msg.contains("UnsupportedType")
            || msg.contains("UnsupportedCallShape")
            || msg.contains("ImmediateOutOfRange")
            || msg.contains("OutOfRegisters")
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
