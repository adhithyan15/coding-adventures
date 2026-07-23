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

/// LANG75 — end-to-end `call_builtin "putchar"` on Linux x86-64.
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
/// Compiles to an executable, runs it, and asserts stdout is exactly
/// `"Hi\n"`.  Proves the entire chain — frontend IIR → CIR specialiser
/// → x86_64 backend → ELF packager → `cc` linker → runtime archive —
/// wires `__twig_putchar` correctly through the System V AMD64 ABI
/// (`int` arg passed in `EDI`/`RDI`).
#[test]
fn end_to_end_call_builtin_putchar_writes_hi() {
    let module = build_putchar_hi_module();
    let dir = tempfile::tempdir().expect("tempdir");
    let exe_path = dir.path().join("putchar_hi");

    twig_aot::compile_module_to_linux_executable(&module, &exe_path)
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

/// LANG76 — end-to-end heap byte I/O on Linux x86-64.
///
/// Mirrors the Windows test: alloc 4 bytes, write `'H','i','\n'` at
/// offsets 0/1/2, call `print_string(buf, 3)`, return 0.  Asserts
/// stdout = `"Hi\n"`.
#[test]
fn end_to_end_lang76_heap_byte_io_writes_hi() {
    let module = build_heap_byte_io_module();
    let dir = tempfile::tempdir().expect("tempdir");
    let exe_path = dir.path().join("heap_byte_io");

    twig_aot::compile_module_to_linux_executable(&module, &exe_path)
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

    for (slot, off, byte) in [
        ("o0", 0_i64, 72_i64),
        ("o1", 1,     105),
        ("o2", 2,     10),
    ] {
        ins.push(con(slot, off));
        let v = format!("v_{slot}");
        ins.push(con(&v, byte));
        ins.push(IIRInstr::new("store_byte", None,
            vec![Operand::Var("buf".into()),
                 Operand::Var(slot.into()),
                 Operand::Var(v)], "void"));
    }

    ins.push(con("len3", 3));
    ins.push(IIRInstr::new("call_builtin", None,
        vec![Operand::Var("print_string".into()),
             Operand::Var("buf".into()),
             Operand::Var("len3".into())], "void"));

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

// ── AOT00-T1 x86_64 PR-x3: precise-roots registration, end to end ──────────
//
// These run on the native x86-64 `ubuntu-latest` runner — the authoritative
// validator for the x86-64 GC path (the dev host is aarch64 macOS). They prove the
// SysV `__gc_init_stackmaps` registration codegen runs correctly and is load-bearing.

/// The start-up registration actually ran in the linked image: a program that returns
/// `__gc_stackmap_count()` exits > 0.
#[test]
fn gc_stackmap_registration_ran_on_linux() {
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
    let exe = dir.path().join("gc_count");
    twig_aot::compile_module_to_linux_executable(&m, &exe).expect("count compiles+links");
    let code = Command::new(&exe).output().expect("count runs").status.code().unwrap();
    assert!(code > 0, "registration must have run at start-up (count={code})");
}

/// The GC-stress `live_bytes` differential on x86-64: a 64-byte allocation whose address
/// lives only in an `i64` (non-reference) slot is reclaimed by a precise collect but
/// pinned by a conservative one. Precise → `live_bytes == 0`, conservative → `== 64`.
/// This is the headline proof that precise roots are load-bearing on native x86-64.
#[test]
fn gc_stress_live_bytes_differential_on_linux() {
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
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_linux_executable(&build(collect, ret), &exe)
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

/// AOT00-T1 x86_64 PR-x4 / PR-x5 — precise roots reach through a **self-recursive** frame.
///
/// The differential above proves precise roots work for `main`'s own frame. This one
/// proves they work for an *intermediate* frame in a recursion chain — the case that, on
/// x86-64, is precise ONLY because a self-recursive `call` is now a registered safepoint
/// (PR-x4). A self-recursive `call <fn>` lowers to `call rel32` with an internal label
/// fixup and no `PltRel32` relocation, so before PR-x4 its return address was not in the
/// stack map and the collector conservatively re-scanned that frame.
///
/// ```text
///   rec(stop) -> i64:
///       a = gc_alloc(64)          ; i64 look-alike, one per active frame
///       if stop != 0 goto base
///       r = rec(1)                ; SELF-RECURSIVE call — a0 sits in this frame across it
///       ret r
///     base:
///       <collect>                 ; gc_collect | gc_collect_precise (fires here)
///       ret gc_live_bytes()
///   main() -> i64: ret rec(0)
/// ```
///
/// `main → rec(0) → rec(1)`. The collect fires in `rec(1)`; unwinding passes through
/// `rec(0)`, an intermediate self-recursive frame holding a 64-byte look-alike (`a0`).
/// **Conservative** pins both look-alikes → `128`. **Precise** maps every frame — `rec(1)`
/// via its builtin calls, `rec(0)` via the self-recursive-call safepoint PR-x4 added — so
/// both `i64` look-alikes are unrooted and reclaimed → `0`. A `precise` of `64` would mean
/// `rec(0)` fell back to a conservative scan: the exact regression PR-x4 prevents. Runs on
/// the native x86-64 `ubuntu-latest` runner (dev host is aarch64 macOS); the identical
/// module is validated locally on aarch64 in `macos_arm64_smoke`.
#[test]
fn gc_recursive_frame_live_bytes_differential_on_linux() {
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    fn build(collect: &str, collect_returns: bool) -> IIRModule {
        // rec(stop: i64) -> i64
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
            IIRInstr::new(
                "call_builtin",
                Some("freed".into()),
                vec![Operand::Var(collect.into())],
                "i64",
            )
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
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_linux_executable(&build(collect, ret), &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        Command::new(&exe)
            .output()
            .unwrap_or_else(|e| panic!("{tag} runs: {e}"))
            .status
            .code()
            .unwrap_or_else(|| panic!("{tag} exited by signal"))
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
         frame — requires that frame be precisely mapped at the recursive-call return \
         address (PR-x4) (got {precise}, conservative={conservative})",
    );
}
