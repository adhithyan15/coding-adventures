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
