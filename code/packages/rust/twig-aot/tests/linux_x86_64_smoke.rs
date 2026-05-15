//! End-to-end smoke test on Linux x86-64 (LANG46).
//!
//! Compiles a small typed Twig program through the entire AOT pipeline
//! (source → IR → x86_64-backend → ELF object → `cc` → executable),
//! runs it, and asserts the exit code.
//!
//! The entire file is gated to `#[cfg(target_os = "linux")]` so it
//! only compiles + runs on Linux CI runners (`ubuntu-latest`).  On
//! macOS and Windows the file is a no-op.
//!
//! ## Pipeline exercised
//!
//! 1. `twig-ir-compiler` parses the Twig source to IIR.
//! 2. `aot-core::specialise` lowers IIR to CIR with concrete integer
//!    types.
//! 3. `x86_64-backend` (System V AMD64 ABI) emits x86-64 machine code
//!    for `main`.
//! 4. `code-packager::elf_object` wraps the bytes in an ELF64 `ET_REL`
//!    relocatable object file.
//! 5. `cc` is invoked to link the object against the embedded Twig
//!    runtime archive, producing an ELF64 executable.
//! 6. The executable is exec'd; its exit code is the value `main`
//!    returned (modulo 256, per POSIX).

#![cfg(target_os = "linux")]

use std::io::Write;
use std::process::Command;

#[test]
fn end_to_end_twig_returns_42_on_linux() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src_path = dir.path().join("smoke.twig");
    let exe_path = dir.path().join("smoke");

    // Tiny Twig program: `(define (main) 42)` → exit code 42.
    let mut f = std::fs::File::create(&src_path).unwrap();
    writeln!(f, "42").unwrap();
    drop(f);

    twig_aot::compile_file_linux_x86_64(&src_path, &exe_path)
        .unwrap_or_else(|e| panic!("compile failed: {e}"));

    let out = Command::new(&exe_path).output()
        .unwrap_or_else(|e| panic!("launch failed: {e}"));
    assert_eq!(
        out.status.code(), Some(42),
        "expected exit 42, got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn end_to_end_twig_arithmetic_on_linux() {
    let cases = [
        ("42",                    42i32),
        ("(+ 30 12)",             42),
        ("(- 100 58)",            42),
        ("(* 6 7)",               42),
        ("(if (= 1 1) 100 200)",  100),
        ("(if (< 5 10) 7 13)",    7),
    ];

    let dir = tempfile::tempdir().expect("tempdir");
    for (i, (src, expected)) in cases.iter().enumerate() {
        let twig_path = dir.path().join(format!("case_{i}.twig"));
        let exe_path  = dir.path().join(format!("case_{i}"));
        let mut f = std::fs::File::create(&twig_path).unwrap();
        writeln!(f, "{src}").unwrap();
        drop(f);

        twig_aot::compile_file_linux_x86_64(&twig_path, &exe_path)
            .unwrap_or_else(|e| panic!("compile {src}: {e}"));

        let out = Command::new(&exe_path).output()
            .unwrap_or_else(|e| panic!("launch {src}: {e}"));
        assert_eq!(
            out.status.code(), Some(*expected),
            "src={src} expected={expected} got={:?} stderr={:?}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

#[test]
fn elf_object_has_correct_machine_field() {
    // Byte-level sanity check: produce an object for `42` and assert
    // the ELF header carries EM_X86_64 (62).  This test doesn't need
    // `cc` to be available — it only exercises the object emitter.
    let obj = twig_aot::compile_linux_x86_64_object("42", "smoke")
        .expect("compile to object");
    // Bytes 0..4 = ELF magic (\x7fELF)
    assert_eq!(&obj[0..4], &[0x7F, b'E', b'L', b'F']);
    // Byte 16..18 = e_type LE; ET_REL = 1.
    let e_type = u16::from_le_bytes([obj[16], obj[17]]);
    assert_eq!(e_type, 1);
    // Byte 18..20 = e_machine LE; EM_X86_64 = 62.
    let e_machine = u16::from_le_bytes([obj[18], obj[19]]);
    assert_eq!(e_machine, 62);
}
