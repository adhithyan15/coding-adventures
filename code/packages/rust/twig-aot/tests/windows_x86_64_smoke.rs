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

/// LANG76 — end-to-end heap byte I/O on Windows x86-64.
///
/// Builds a tiny `IIRModule`:
///
/// ```text
/// fn main() -> i64 {
///   const c4   = 4
///   alloc_bytes c4 -> buf       ; calloc(1, 4)
///   const c0   = 0
///   const cH   = 72
///   store_byte buf, c0, cH
///   const c1   = 1
///   const ci   = 105
///   store_byte buf, c1, ci
///   const c2_  = 2
///   const cnl  = 10
///   store_byte buf, c2_, cnl
///   const c3   = 3
///   call_builtin "print_string", buf, c3
///   const r    = 0
///   ret r
/// }
/// ```
///
/// Compiles, links, runs, asserts stdout = `"Hi\n"`.  This exercises
/// the full LANG76 chain: `alloc_bytes` → `__twig_alloc_bytes`,
/// `store_byte` (low-byte write), and `call_builtin "print_string"`
/// reading those bytes back.
#[test]
fn end_to_end_lang76_heap_byte_io_writes_hi() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }

    let module = build_heap_byte_io_module();
    let dir = tempfile::tempdir().expect("tempdir");
    let exe_path = dir.path().join("heap_byte_io.exe");

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

/// Build a module that allocates a 4-byte buffer, writes `'H','i','\n'`
/// at offsets 0/1/2, calls `print_string(buf, 3)`, and returns 0.
fn build_heap_byte_io_module() -> interpreter_ir::module::IIRModule {
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    let mut ins = Vec::new();
    let mut con = |slot: &str, n: i64| {
        IIRInstr::new("const", Some(slot.to_string()),
                      vec![Operand::Int(n)], "i64")
    };

    ins.push(con("c4", 4));
    ins.push(IIRInstr::new("alloc_bytes", Some("buf".into()),
        vec![Operand::Var("c4".into())], "i64"));

    // store bytes 'H','i','\n' at offsets 0, 1, 2
    for (slot, off, byte) in [
        ("o0", 0_i64, 72_i64),   // 'H'
        ("o1", 1,     105),      // 'i'
        ("o2", 2,     10),       // '\n'
    ] {
        ins.push(con(slot, off));
        let v = format!("v_{slot}");
        ins.push(con(&v, byte));
        ins.push(IIRInstr::new("store_byte", None,
            vec![Operand::Var("buf".into()),
                 Operand::Var(slot.into()),
                 Operand::Var(v)], "void"));
    }

    // print_string(buf, 3)
    ins.push(con("len3", 3));
    ins.push(IIRInstr::new("call_builtin", None,
        vec![Operand::Var("print_string".into()),
             Operand::Var("buf".into()),
             Operand::Var("len3".into())], "void"));

    // ret 0
    ins.push(con("r", 0));
    ins.push(IIRInstr::new("ret", None,
        vec![Operand::Var("r".into())], "i64"));

    let main = IIRFunction::new("main", vec![], "i64", ins);
    let mut module = IIRModule::new("heap_byte_io", "lang");
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

// ── AOT00-T1 x86_64 PR-x6: precise-roots registration on the Microsoft x64 ABI ─────
//
// These execute on the native `windows-latest` runner — the authoritative validator for
// the MsX64 GC path (the dev host is aarch64 macOS). They prove the MsX64
// `__gc_init_stackmaps` marshalling (args 1–4 in rcx/rdx/r8/r9, args 5–8 above a 32-byte
// shadow space) registers correctly and that precise roots are load-bearing on Windows —
// closing the "Windows degrades to conservative" gap left by PR-x3.

/// The start-up registration actually ran in the linked `.exe`: a program returning
/// `__gc_stackmap_count()` exits `> 0`. (Also proves the MsX64 init links against
/// `__gc_register_stackmap` — previously it was a no-op that referenced nothing.)
#[test]
fn gc_stackmap_registration_ran_on_windows() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    let main = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            IIRInstr::new(
                "call_builtin",
                Some("c".into()),
                vec![Operand::Var("gc_stackmap_count".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
        ],
    );
    let mut m = IIRModule::new("gc_count", "twig");
    m.add_or_replace(main);
    m.entry_point = Some("main".into());

    let dir = tempfile::tempdir().expect("tempdir");
    let exe = dir.path().join("gc_count.exe");
    twig_aot::compile_module_to_windows_executable(&m, &exe).expect("count compiles+links");
    let code = Command::new(&exe).output().expect("count runs").status.code().unwrap();
    assert!(code > 0, "MsX64 registration must have run at start-up (count={code})");
}

/// The GC-stress `live_bytes` differential on Windows: a 64-byte allocation whose address
/// lives only in an `i64` (non-reference) slot is reclaimed by a precise collect but pinned
/// by a conservative one. Precise → `live_bytes == 0`, conservative → `== 64`. The headline
/// proof that MsX64 precise roots are load-bearing (mirrors the Linux/SysV differential).
#[test]
fn gc_stress_live_bytes_differential_on_windows() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    fn build(collect: &str, collect_returns: bool) -> IIRModule {
        let mut body = vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("a".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
        ];
        body.push(if collect_returns {
            IIRInstr::new("call_builtin", Some("f".into()), vec![Operand::Var(collect.into())], "i64")
        } else {
            IIRInstr::new("call_builtin", None, vec![Operand::Var(collect.into())], "void")
        });
        body.push(IIRInstr::new(
            "call_builtin",
            Some("lb".into()),
            vec![Operand::Var("gc_live_bytes".into())],
            "i64",
        ));
        body.push(IIRInstr::new("ret", None, vec![Operand::Var("lb".into())], "i64"));
        let mut m = IIRModule::new("gc_stress", "twig");
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: &str, ret: bool| -> i32 {
        let exe = dir.path().join(format!("{tag}.exe"));
        twig_aot::compile_module_to_windows_executable(&build(collect, ret), &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"))
            .status.code().unwrap_or_else(|| panic!("{tag} exited by signal"))
    };

    let conservative = run("gc_stress_cons", "gc_collect", false);
    let precise = run("gc_stress_prec", "gc_collect_precise", true);
    assert_eq!(conservative, 64, "conservative retains the 64-byte look-alike-pinned object");
    assert_eq!(
        precise, 0,
        "precise reclaims the object reachable only via a non-reference i64 slot \
         (conservative kept {conservative})",
    );
}

/// The recursion differential on Windows — precise roots reach through an intermediate
/// **self-recursive** frame (identical module to the Linux/aarch64 twins). `rec(stop)`
/// allocates an `i64` look-alike per frame and recurses once (`main → rec(0) → rec(1)`);
/// the collect fires in `rec(1)`, so the collector unwinds through `rec(0)`, an
/// intermediate self-recursive frame. Conservative pins both look-alikes (`128`); precise
/// reclaims both (`0`). Exercises the MsX64 registration of the self-recursive-call
/// safepoint (PR-x4) end to end.
#[test]
fn gc_recursive_frame_live_bytes_differential_on_windows() {
    if !linker_available() {
        eprintln!("skipping: no Windows linker on PATH");
        return;
    }
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    fn build(collect: &str, collect_returns: bool) -> IIRModule {
        let mut rec_body = vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("a".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("stop".into()), Operand::Var("base".into())],
                "i64",
            ),
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "call",
                Some("r".into()),
                vec![Operand::Var("rec".into()), Operand::Var("one".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            IIRInstr::new("label", None, vec![Operand::Var("base".into())], "i64"),
        ];
        rec_body.push(if collect_returns {
            IIRInstr::new("call_builtin", Some("freed".into()), vec![Operand::Var(collect.into())], "i64")
        } else {
            IIRInstr::new("call_builtin", None, vec![Operand::Var(collect.into())], "void")
        });
        rec_body.push(IIRInstr::new(
            "call_builtin",
            Some("lb".into()),
            vec![Operand::Var("gc_live_bytes".into())],
            "i64",
        ));
        rec_body.push(IIRInstr::new("ret", None, vec![Operand::Var("lb".into())], "i64"));

        let main_body = vec![
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "call",
                Some("r".into()),
                vec![Operand::Var("rec".into()), Operand::Var("z".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ];

        let mut m = IIRModule::new("gc_recur", "twig");
        m.add_or_replace(IIRFunction::new(
            "rec",
            vec![("stop".to_string(), "i64".to_string())],
            "i64",
            rec_body,
        ));
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", main_body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: &str, ret: bool| -> i32 {
        let exe = dir.path().join(format!("{tag}.exe"));
        twig_aot::compile_module_to_windows_executable(&build(collect, ret), &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"))
            .status.code().unwrap_or_else(|| panic!("{tag} exited by signal"))
    };

    let conservative = run("gc_recur_cons", "gc_collect", false);
    let precise = run("gc_recur_prec", "gc_collect_precise", true);
    assert_eq!(
        conservative, 128,
        "conservative pins both recursive frames' 64-byte look-alikes (got {conservative})",
    );
    assert_eq!(
        precise, 0,
        "precise reclaims both look-alikes, incl. a0 in the intermediate self-recursive \
         frame — MsX64 must map it at the recursive-call return address (got {precise}, \
         conservative={conservative})",
    );
}
