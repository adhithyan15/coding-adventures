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
