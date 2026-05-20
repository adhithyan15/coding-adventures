//! End-to-end smoke test on Windows x86-64 (LANG46).
//!
//! Compiles a small typed Twig program through the entire AOT pipeline
//! (source → IR → x86_64-backend → PE/COFF object → `link.exe` → `.exe`),
//! runs it, and asserts the exit code.
//!
//! The entire file is gated to `#[cfg(target_os = "windows")]` so it
//! only compiles + runs on Windows CI runners (`windows-latest`).  On
//! Linux and macOS the file is a no-op.
//!
//! ## Pipeline exercised
//!
//! 1. `twig-ir-compiler` parses the Twig source to IIR.
//! 2. `aot-core::specialise` lowers IIR to CIR.
//! 3. `x86_64-backend` (Microsoft x64 ABI) emits x86-64 machine code
//!    for `main`.
//! 4. `code-packager::pe_object` wraps the bytes in an AMD64 PE/COFF
//!    relocatable object file.
//! 5. The Windows linker (`link.exe`, falling back to `lld-link.exe`
//!    then MinGW `gcc.exe`) links the object against the embedded
//!    runtime archive, producing a `.exe`.
//! 6. The exe is run; its exit code is the value `main` returned
//!    (modulo `& 0xFFFFFFFF` for a 32-bit DWORD).
//!
//! ## Skipping behaviour
//!
//! Each test probes for a Windows linker on `PATH`.  If none is found
//! (e.g. MSVC environment not activated, no MinGW), the test prints a
//! skip message and exits cleanly — CI runners normally have the
//! Visual Studio Build Tools installed which provides `link.exe`.

#![cfg(target_os = "windows")]

use std::io::Write;
use std::process::Command;

/// Probe `PATH` for a real Windows linker (MSVC link.exe, LLVM lld-link.exe,
/// or MinGW gcc.exe).
///
/// Checks the banner output rather than just whether the program is
/// spawnable — git-bash hosts may ship a POSIX `link(1)` utility on
/// `PATH` that has the same name as MSVC's `link.exe` but isn't a
/// real linker.
fn linker_available() -> bool {
    let probes: &[(&str, &str, &[&str])] = &[
        // (name, arg, required-substrings-in-banner)
        ("link.exe",     "",           &["Microsoft", "Linker"]),
        ("lld-link.exe", "",           &["LLD"]),
        ("gcc.exe",      "--version",  &["gcc"]),
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

#[test]
fn end_to_end_twig_returns_42_on_windows() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let src_path = dir.path().join("smoke.twig");
    let exe_path = dir.path().join("smoke.exe");

    let mut f = std::fs::File::create(&src_path).unwrap();
    writeln!(f, "42").unwrap();
    drop(f);

    twig_aot::compile_file_windows_x86_64(&src_path, &exe_path)
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
fn end_to_end_twig_arithmetic_on_windows() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }

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
        let exe_path  = dir.path().join(format!("case_{i}.exe"));
        let mut f = std::fs::File::create(&twig_path).unwrap();
        writeln!(f, "{src}").unwrap();
        drop(f);

        twig_aot::compile_file_windows_x86_64(&twig_path, &exe_path)
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

/// LANG75 — end-to-end `call_builtin "putchar"` on Windows x86-64.
///
/// Builds a tiny `IIRModule` by hand:
///
/// ```text
/// fn main() -> i64 {
///   const c0 = 72   ; 'H'
///   call_builtin "putchar", c0
///   const c1 = 105  ; 'i'
///   call_builtin "putchar", c1
///   const c2 = 10   ; '\n'
///   call_builtin "putchar", c2
///   ret 0
/// }
/// ```
///
/// Compiles to an `.exe`, runs it, and asserts stdout is exactly `"Hi\n"`.
/// Proves that the entire chain — frontend IIR → CIR specialiser → x86_64
/// backend → PE/COFF packager → MS link.exe → runtime archive — wires
/// `__twig_putchar` correctly through the Microsoft x64 ABI (`int` arg
/// passed in `ECX`/`RCX`).
#[test]
fn end_to_end_call_builtin_putchar_writes_hi() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }

    let module = build_putchar_hi_module();
    let dir = tempfile::tempdir().expect("tempdir");
    let exe_path = dir.path().join("putchar_hi.exe");

    twig_aot::compile_module_to_windows_executable(&module, &exe_path)
        .unwrap_or_else(|e| panic!("compile failed: {e}"));

    let out = Command::new(&exe_path).output()
        .unwrap_or_else(|e| panic!("launch failed: {e}"));
    assert_eq!(
        out.stdout, b"Hi\n",
        "expected stdout == \"Hi\\n\", got {:?}; stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Construct an `IIRModule` that emits `"Hi\n"` via three `call_builtin
/// "putchar"` instructions, then returns 0 from main.
fn build_putchar_hi_module() -> interpreter_ir::module::IIRModule {
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    let mut instructions = Vec::new();

    for (slot, code) in [("c0", 72_i64), ("c1", 105), ("c2", 10)] {
        instructions.push(IIRInstr::new(
            "const",
            Some(slot.to_string()),
            vec![Operand::Int(code)],
            "i64",
        ));
        instructions.push(IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("putchar".to_string()),
                 Operand::Var(slot.to_string())],
            "void",
        ));
    }
    // Return 0 from main.
    instructions.push(IIRInstr::new(
        "const",
        Some("r".to_string()),
        vec![Operand::Int(0)],
        "i64",
    ));
    instructions.push(IIRInstr::new(
        "ret",
        None,
        vec![Operand::Var("r".to_string())],
        "i64",
    ));

    let main = IIRFunction::new("main", vec![], "i64", instructions);
    let mut module = IIRModule::new("putchar_hi", "lang");
    module.functions.push(main);
    module.entry_point = Some("main".to_string());
    module
}

#[test]
fn pe_object_has_correct_machine_field() {
    // Byte-level sanity check: produce an object for `42` and assert
    // the COFF header carries IMAGE_FILE_MACHINE_AMD64 (0x8664).  This
    // test doesn't need a linker to be available — it only exercises
    // the object emitter.
    let obj = twig_aot::compile_windows_x86_64_object("42", "smoke")
        .expect("compile to object");
    // First 2 bytes: IMAGE_FILE_MACHINE_AMD64 LE = 0x64 0x86.
    assert_eq!(&obj[0..2], &[0x64, 0x86]);
}
