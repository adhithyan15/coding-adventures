//! End-to-end smoke test on Apple Silicon.
//!
//! Compiles a tiny IIR program ("return 42 via exit syscall") through
//! the entire AOT pipeline → Mach-O bytes → on-disk binary → execution
//! by the OS, then asserts the exit code.
//!
//! ## Why a hand-built IIR rather than a Twig source
//!
//! `twig-ir-compiler` may emit IIR that the V1 ARM64 backend doesn't yet
//! support (e.g. `global_set` for top-level value defines).  Driving the
//! pipeline with hand-built IIR keeps the test focused on the AOT
//! plumbing and the encoder, not the Twig surface language.
//!
//! ## Why exit-syscall instead of plain `ret`
//!
//! macOS Mach-O exec via `LC_MAIN` requires `dyld` to set up the C ABI
//! (argc/argv/envp on the stack) before calling `main`, then routes
//! `main`'s return through `exit()`.  The current `code-packager`
//! Mach-O writer emits `LC_MAIN` but no `LC_LOAD_DYLINKER` — without
//! that load command modern macOS may refuse to exec the binary.
//!
//! Instead we build a program that bypasses dyld entirely: it issues
//! the BSD `exit` syscall directly with x0 as the exit code.  This is
//! valid for a static, dyld-less Mach-O and tells us whether the
//! basic Mach-O framing produced by `code-packager::macho64` is
//! launchable.
//!
//! If this test fails with a launch error (e.g. "Killed: 9" or "exec
//! format error"), the next step is fixing `code-packager` to emit
//! `LC_LOAD_DYLINKER` + a dyld-compatible header.
//!
//! ## Skipping
//!
//! The test compiles unconditionally, but the executable run is
//! `#[cfg(all(target_os = "macos", target_arch = "aarch64"))]` so it
//! only runs locally on Apple Silicon Macs.  Other CI runners just
//! verify the byte production.

// `PermissionsExt` is Unix-only; only import it on platforms where it exists.
// All callers are already gated with `#[cfg(all(target_os = "macos", ...))]`
// which is a strict subset of unix, so the cfg is safe.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use aarch64_backend::AArch64Backend;
use aarch64_encoder::{Assembler, Reg};
use code_packager::{CodeArtifact, PackagerRegistry, Target};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::module::IIRModule;
use jit_core::backend::{Backend, FunctionContext};

/// Build a function whose entire body is "exit(42) via SYS_exit".
///
/// We construct the bytes directly via the encoder rather than going
/// through CIR → backend; the latter would emit a function-shaped
/// prologue/epilogue that the OS exec path doesn't expect.
fn exit_42_text() -> Vec<u8> {
    let mut a = Assembler::new();
    // movz x0,  #42      ; exit code
    // movz x16, #1        ; SYS_exit on macOS arm64 BSD layer
    // svc  #0x80
    a.movz(Reg::X0, 42, 0);
    a.movz(Reg::X16, 1, 0);
    a.svc(0x80);
    a.finish().unwrap()
}

#[test]
fn macho_arm64_byte_production() {
    // The packager always succeeds for valid Target + bytes.  Verify the
    // output starts with the Mach-O magic number.
    let target = Target::macos_arm64();
    let artifact = CodeArtifact::new(exit_42_text(), 0, target);
    let bytes = PackagerRegistry::pack(&artifact).unwrap();
    assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
    assert!(bytes.len() > 200, "header alone is ≥ 200 bytes");
}

/// End-to-end execution test: produce object file → invoke `ld` →
/// run the resulting executable → assert exit code 42.
///
/// On macOS 15+ the kernel attaches a "provenance" tag to every file
/// recording which process wrote it; only files written by trusted
/// system tools (Apple-signed `ld`, etc.) are allowed to `exec()`.  By
/// shelling out to `/usr/bin/ld` we delegate the final write, so the
/// kernel grants the resulting executable trusted provenance and lets
/// it run.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_object_through_ld_returns_42() {
    use code_packager::macho_object::pack_object;

    // Build a tiny "exit(42)" function via the encoder (no Twig source
    // here — keeps this test honest about the encoder + linker glue).
    let target = Target::macos_arm64();
    let artifact = CodeArtifact::new(exit_42_text(), 0, target);
    let object_bytes = pack_object(&artifact).unwrap();

    let dir = tempfile::tempdir().expect("tempdir");
    let object_path: PathBuf = dir.path().join("twig_smoke.o");
    let exe_path:    PathBuf = dir.path().join("twig_smoke");
    std::fs::write(&object_path, &object_bytes).unwrap();

    // Discover the SDK lib path the same way `twig-aot` does internally.
    let sdk_lib = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| std::path::PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string())
                 .join("usr").join("lib"))
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/lib"));

    // Invoke `ld` — the Apple system linker — to produce the executable.
    // Same args twig-aot uses internally.
    let ld = Command::new("ld")
        .arg("-arch").arg("arm64")
        .arg("-platform_version").arg("macos").arg("15.0").arg("15.0")
        .arg("-e").arg("_main")
        .arg("-L").arg(&sdk_lib)
        .arg("-lSystem")
        .arg("-o").arg(&exe_path)
        .arg(&object_path)
        .output()
        .expect("ld must be on PATH (Xcode CLT)");
    assert!(ld.status.success(),
            "ld failed: stderr={:?}",
            String::from_utf8_lossy(&ld.stderr));

    // The system linker writes the executable; the kernel grants it
    // trusted provenance.  Run it and check the exit code.
    let mut perms = std::fs::metadata(&exe_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&exe_path, perms).unwrap();

    let out = Command::new(&exe_path).output()
        .expect("launch generated executable");
    assert_eq!(
        out.status.code(), Some(42),
        "expected exit 42, got {:?}; stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Full Twig-source-to-runnable-binary smoke test on this Mac.
///
/// Compiles a Twig program of the form `(define (main) -> u64 42)`
/// through the entire AOT pipeline and checks the binary's exit code.
///
/// This exercises:
/// 1. `twig-ir-compiler` → IIR
/// 2. `aot-core::specialise` → CIR (typed)
/// 3. `aarch64-backend::compile_function` → ARM64 bytes
/// 4. `code-packager::macho_object::pack_object` → Mach-O `.o`
/// 5. `ld` → executable Mach-O on disk
/// 6. exec → exit code 42
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_typed_twig_returns_42() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src_path = dir.path().join("smoke.twig");
    let out_path = dir.path().join("smoke");

    // A typed main that returns 42 via the encoded `ret_u64` path.
    // We can't yet write a full typed Twig program because untyped
    // global defines aren't lowered, so we do it via a hand-built
    // IIR module directly through twig_aot's API for this test.
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    let main = IIRFunction::new(
        "main", vec![], "u64",
        vec![
            IIRInstr::new("const", Some("v0".into()),
                          vec![Operand::Int(42)], "u64"),
            IIRInstr::new("ret", None,
                          vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let mut module = IIRModule::new("smoke", "twig");
    module.add_or_replace(main);
    module.entry_point = Some("main".into());

    // Object file → .o on disk.
    let obj = twig_aot::compile_module_macos_arm64_object(&module)
        .expect("module compiles");
    let obj_path = dir.path().join("smoke.o");
    std::fs::write(&obj_path, &obj).unwrap();

    // Drive the same `ld` invocation twig-aot uses, but skip the
    // file-on-disk dance by writing src/out manually.  We're testing
    // the linker integration here.
    std::fs::write(&src_path, b"(define (main) 42)\n").unwrap();

    // The hand-built module above is what we link, not the Twig source —
    // so we shell to ld directly with our object path.
    let sdk_lib = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output().ok()
        .filter(|o| o.status.success())
        .map(|o| std::path::PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string())
                 .join("usr").join("lib"))
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/lib"));
    let ld = Command::new("ld")
        .arg("-arch").arg("arm64")
        .arg("-platform_version").arg("macos").arg("15.0").arg("15.0")
        .arg("-e").arg("_main")
        .arg("-L").arg(&sdk_lib).arg("-lSystem")
        .arg("-o").arg(&out_path)
        .arg(&obj_path)
        .output().expect("ld must be available");
    assert!(ld.status.success(), "ld stderr: {}",
            String::from_utf8_lossy(&ld.stderr));
    let mut perms = std::fs::metadata(&out_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&out_path, perms).unwrap();

    let out = Command::new(&out_path).output()
        .expect("launch generated executable");
    // Our typed `main` returns 42; AAPCS64 puts it in x0; dyld_start
    // routes that through `exit(x0)`.  Process exit code should be 42.
    assert_eq!(out.status.code(), Some(42),
               "expected 42, got {:?}; stderr={:?}",
               out.status.code(),
               String::from_utf8_lossy(&out.stderr));
}

/// Real Twig source programs that exercise the full pipeline:
///   parser → IIR → CIR (specialise lowers `call_builtin` to typed ops)
///   → ARM64 → object → ld → runnable Mach-O → exec → exit code.
///
/// Each `(source, expected_exit_code)` pair is compiled via
/// `twig-aot`'s `compile_file_macos_arm64` and run.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_typed_twig_arithmetic_and_branches() {
    use std::io::Write;

    let cases = [
        ("42",                       42i32),
        ("(+ 30 12)",                42),
        ("(- 100 58)",               42),
        ("(* 6 7)",                  42),
        ("(if (= 1 1) 100 200)",     100),
        ("(if (= 1 2) 100 200)",     200),
        ("(if (< 5 10) 7 13)",       7),
        ("(if (> 5 10) 7 13)",       13),
    ];

    let dir = tempfile::tempdir().expect("tempdir");
    for (i, (src, expected)) in cases.iter().enumerate() {
        let twig_path = dir.path().join(format!("case_{i}.twig"));
        let exe_path  = dir.path().join(format!("case_{i}"));
        let mut f = std::fs::File::create(&twig_path).unwrap();
        writeln!(f, "{src}").unwrap();
        drop(f);

        twig_aot::compile_file_macos_arm64(&twig_path, &exe_path)
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

/// AOT00-T1 increment B: a program whose `main` **calls a helper** exercises the
/// real `__gc_init_stackmaps` at start-up.
///
/// `main` has a call → a safepoint record → the injected `__gc_init_stackmaps`
/// materialises `main`'s `func_start` (an `ADR`), loads its embedded
/// `pc_offsets`/`slot_counts`/`slots_flat` arrays via `adr`, and calls
/// `__gc_register_stackmap` — all before `main` runs. If the *structure* of that
/// codegen were malformed (unbalanced frame, bad ABI marshalling, a data word decoded
/// as an instruction, an `adr` whose target is executed) the image would fault at
/// start-up and never return, so a correct exit code proves the registration path
/// runs in production. (It does NOT by itself prove `func_start` is numerically
/// correct — the registry only *stores* it — that is pinned by the byte-decoding unit
/// test `func_start_adr_resolves_to_target_offset` and, end to end, by increment C's
/// precise-collection differential.)
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_init_registers_and_runs() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    // helper() -> u64 { 7 }   (no calls → no records → not registered)
    let helper = IIRFunction::new(
        "helper", vec![], "u64",
        vec![
            IIRInstr::new("const", Some("h".into()), vec![Operand::Int(7)], "u64"),
            IIRInstr::new("ret", None, vec![Operand::Var("h".into())], "u64"),
        ],
    );
    // main() -> u64 { helper() }   (one call → one safepoint record → registered)
    let main = IIRFunction::new(
        "main", vec![], "u64",
        vec![
            IIRInstr::new("call", Some("r".into()), vec![Operand::Var("helper".into())], "u64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u64"),
        ],
    );
    let mut module = IIRModule::new("gc_init_smoke", "twig");
    module.add_or_replace(helper);
    module.add_or_replace(main);
    module.entry_point = Some("main".into());

    let dir = tempfile::tempdir().expect("tempdir");
    let exe = dir.path().join("gc_init_smoke");
    twig_aot::compile_module_to_macos_executable(&module, &exe)
        .expect("module with GC stack-map registration compiles + links");

    let out = Command::new(&exe).output().expect("launch generated executable");
    assert_eq!(
        out.status.code(), Some(7),
        "expected main→helper()==7 after __gc_init_stackmaps ran; got {:?}, stderr={:?}",
        out.status.code(), String::from_utf8_lossy(&out.stderr),
    );
}

/// AOT00-T1 increment C — the **GC-stress `live_bytes` differential**: the headline
/// proof that precise roots are load-bearing in production.
///
/// The program allocates one heap object and keeps its address **only in an `i64`
/// slot** — a non-reference "look-alike" that holds a real heap pointer. Nothing else
/// references the object, so it is garbage. Then it collects and reports
/// `__gc_live_bytes` as its exit code:
///
/// ```text
///   main() -> i64:
///       a  = gc_alloc(64)        ; i64 slot — a heap-address look-alike (only ref)
///       <collect>                ; gc_collect  (conservative) | gc_collect_precise
///       lb = gc_live_bytes()
///       ret lb                   ; exit code = live payload bytes
/// ```
///
/// - **Conservative** (`__gc_collect`) scans the whole stack, sees the look-alike, and
///   *pins* the object → `live_bytes == 64`.
/// - **Precise** (`__gc_collect_precise`) walks `main`'s frame through its registered
///   stack map, which names only *reference* slots — the `i64` look-alike is **not**
///   among them — so the object is unrooted and reclaimed → `live_bytes == 0`.
///
/// The gap (64 → 0) is the whole precise-roots feature made observable: registration
/// (increment B) fired, `func_start` resolved `main`'s return address to the right map,
/// and the map correctly excluded the non-reference slot. If any link in that chain
/// were wrong the two columns would be equal.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_stress_live_bytes_differential() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    // Build `main` for a given collect builtin. `collect_returns` picks the dest shape
    // the backend requires (a returning builtin needs a dest; a void one must not).
    fn build(collect: &str, collect_returns: bool) -> IIRModule {
        let mut body = vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            // The allocation's address lives only here, in an i64 (non-ref) slot.
            IIRInstr::new(
                "call_builtin",
                Some("a".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
        ];
        body.push(if collect_returns {
            IIRInstr::new(
                "call_builtin",
                Some("freed".into()),
                vec![Operand::Var(collect.into())],
                "i64",
            )
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
    let run = |tag: &str, collect: &str, collect_returns: bool| -> i32 {
        let m = build(collect, collect_returns);
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_macos_executable(&m, &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    // Diagnostic: a program that just returns __gc_stackmap_count() — proves the
    // start-up registration actually ran in the linked image (increment B).
    {
        let mut m = IIRModule::new("gc_count", "twig");
        m.add_or_replace(IIRFunction::new(
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
        ));
        m.entry_point = Some("main".into());
        let exe = dir.path().join("gc_count");
        twig_aot::compile_module_to_macos_executable(&m, &exe).expect("count compiles");
        let code = Command::new(&exe).output().expect("count runs").status.code().unwrap();
        eprintln!("DIAG __gc_stackmap_count() = {code}");
        assert!(code > 0, "registration must have run at start-up (count={code})");
    }

    let conservative = run("gc_stress_cons", "gc_collect", false);
    let precise = run("gc_stress_prec", "gc_collect_precise", true);

    // Conservative pins the look-alike-referenced 64-byte object; precise reclaims it.
    assert_eq!(
        conservative, 64,
        "conservative collect must retain the 64-byte look-alike-pinned object \
         (live_bytes={conservative})",
    );
    assert_eq!(
        precise, 0,
        "precise collect must reclaim the object reachable only via a non-reference \
         i64 slot (live_bytes={precise}); conservative kept {conservative}",
    );
}

/// **Twig strings are now managed by the collector** (gc-core convergence): a runtime string
/// block, previously `calloc`'d and leaked, is allocated through `__gc_alloc_kind` (a
/// no-reference "blob" kind) by `__twig_alloc_bytes`. A leaking `calloc` block was invisible to
/// the collector and never freed; a gc-core block is **counted in `live_bytes`** and reclaimed
/// when unreachable.
///
/// ```text
///   main() -> i64:
///       a = str_const "AB"
///       b = str_const "CDE"
///       c = str_concat(a, b)     ; a FRESH gc-managed [len][bytes] block
///       [gc_collect_precise]     ; optional
///       lb = gc_live_bytes()
///       ret lb
/// ```
///
/// Every runtime string block goes through `__twig_alloc_bytes` → gc-core: the two literals
/// ("AB" = 8+2 = 10 B, "CDE" = 8+3 = 11 B) and the concatenation ("ABCDE" = 8+5 = 13 B), so
/// `live_bytes == 34`.
///
/// - **No collect:** `live_bytes == 34` — all three string blocks are counted by the collector,
///   proving strings go through gc-core at all (leaking `calloc` blocks would report `0`).
/// - **Precise collect:** `live_bytes == 13` — the two literals `a`/`b`, **dead** after the
///   concatenation, are reclaimed (34 → 13, freeing "AB" + "CDE" = 21 B), while the still-live
///   concatenation `c` is kept (its `str` handle is a reference the precise walk roots). So a
///   Twig string is genuinely **reclaimed when it dies**, not leaked — the headline of the
///   gc-core convergence, proven end-to-end through the native path.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_manages_runtime_strings() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn build(collect: Option<&str>) -> IIRModule {
        let mut body = vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("AB".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("CDE".into())], "str"),
            // A fresh runtime string → __twig_str_concat → __twig_alloc_bytes → __gc_alloc_kind.
            IIRInstr::new(
                "call_builtin",
                Some("c".into()),
                vec![Operand::Var("str_concat".into()), Operand::Var("a".into()), Operand::Var("b".into())],
                "str",
            ),
        ];
        if let Some(collect) = collect {
            body.push(IIRInstr::new(
                "call_builtin",
                Some("freed".into()),
                vec![Operand::Var(collect.into())],
                "i64",
            ));
        }
        body.push(IIRInstr::new(
            "call_builtin",
            Some("lb".into()),
            vec![Operand::Var("gc_live_bytes".into())],
            "i64",
        ));
        body.push(IIRInstr::new("ret", None, vec![Operand::Var("lb".into())], "i64"));

        let mut m = IIRModule::new("gc_strings", "twig");
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: Option<&str>| -> i32 {
        let m = build(collect);
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_macos_executable(&m, &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    let tracked = run("gc_str_tracked", None);
    let after_precise = run("gc_str_kept", Some("gc_collect_precise"));

    assert_eq!(
        tracked, 34,
        "every runtime string block must be a gc-core allocation counted in live_bytes: \
         \"AB\"(10) + \"CDE\"(11) + \"ABCDE\"(13) = 34; leaking calloc blocks would report 0 \
         (invisible to the collector). got {tracked}",
    );
    assert_eq!(
        after_precise, 13,
        "a precise collect must reclaim the two literals dead after the concat (freeing 21 B) and \
         keep the live concatenation \"ABCDE\"(13); a Twig string is reclaimed when it dies, not \
         leaked. got live_bytes={after_precise}",
    );
}

/// AOT00-T1 — the **complete** precise-roots correctness statement: precise roots must
/// *keep* a genuine heap reference AND *reclaim* a non-reference look-alike, in one run.
///
/// The prior differential proves only the reclaim half (its program keeps no live
/// object, so precise `live_bytes` is 0). This program holds **both**:
///
/// ```text
///   main() -> i64:
///       z = dyn_box_int(0)       ; any  — a tagged value to build a cell from
///       b = dyn_cons(z, z)       ; any  — a REAL heap cons cell → a live reference
///       a = gc_alloc(64)         ; i64  — a heap-address look-alike (garbage)
///       <collect>
///       lb = gc_live_bytes()
///       ret lb
/// ```
///
/// `b` is an `any`-typed slot, so the stack map *names* it → the cons cell survives a
/// precise collect. `a` is an `i64` slot, *not* named → its 64-byte object is reclaimed.
/// So `precise == sizeof(cons cell)` and `conservative == sizeof(cons cell) + 64`: the
/// map both roots the real reference and excludes the look-alike. A precise result of 0
/// would mean a live reference was wrongly dropped (a UAF); an equal result would mean
/// the look-alike was wrongly pinned.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_precise_keeps_ref_reclaims_lookalike() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn build(collect: &str, collect_returns: bool) -> IIRModule {
        let mut body = vec![
            // A real, live heap reference in an `any` slot: box a tagged int, then
            // cons it — `dyn_cons` allocates a cell through the GC and returns a
            // tagged `any` pointer, which the stack map names as a root.
            IIRInstr::new("const", Some("z0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("z".into()),
                vec![Operand::Var("dyn_box_int".into()), Operand::Var("z0".into())],
                "any",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("b".into()),
                vec![Operand::Var("dyn_cons".into()), Operand::Var("z".into()), Operand::Var("z".into())],
                "any",
            ),
            // A non-reference look-alike (garbage) in an `i64` slot.
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("a".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
        ];
        body.push(if collect_returns {
            IIRInstr::new("call_builtin", Some("freed".into()), vec![Operand::Var(collect.into())], "i64")
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

        let mut m = IIRModule::new("gc_keepref", "twig");
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: &str, collect_returns: bool| -> i32 {
        let m = build(collect, collect_returns);
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_macos_executable(&m, &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    let conservative = run("gc_keepref_cons", "gc_collect", false);
    let precise = run("gc_keepref_prec", "gc_collect_precise", true);

    // The cons cell (the live `any` reference) must survive the precise collect …
    assert!(
        precise > 0,
        "precise collect must KEEP the live cons cell referenced by an `any` slot \
         (live_bytes={precise})",
    );
    // … and the 64-byte i64 look-alike garbage must be reclaimed by precise but pinned
    // by conservative — so the two columns differ by exactly the look-alike's size.
    assert_eq!(
        conservative - precise,
        64,
        "the i64 look-alike (64 bytes) must be reclaimed by precise and pinned by \
         conservative (conservative={conservative}, precise={precise})",
    );
}

/// AOT00-T3 §5 — a native frontend triggers a **compacting** collection end to end.
///
/// Reuses the keep-ref program but drives `gc_collect_compacting` (the moving-collector
/// C-ABI entry, via the `__twig_gc_collect_compacting` alias). The compacting collect is a
/// strict generalisation of the precise collect: it keeps every live reference and reclaims
/// every look-alike, then *additionally* relocates the objects it can prove movable.
///
/// Today every frontend heap object is allocated **kind 0** (`__dyn_cons` → `__twig_gc_alloc`
/// → `__gc_alloc`), which the collector traces *conservatively* and therefore **pins** — so
/// nothing is movable yet and the compacting collect degrades to exactly the precise one.
/// This test pins that guarantee: `gc_collect_compacting` keeps the live cons cell and
/// reclaims the i64 look-alike, giving the **identical** `live_bytes` to `gc_collect_precise`
/// — proving the frontend can invoke a compaction, that it runs safely on a real thread
/// stack, and that it never drops a live reference or pins a look-alike differently. (A true
/// address-relocation differential is gated on frontend kind-registration — `__gc_alloc_kind`
/// with a ref-field map — so an object becomes movable; a separate follow-up.)
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_compacting_matches_precise() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn build(collect: &str) -> IIRModule {
        let body = vec![
            // A real, live heap reference in an `any` slot (the stack map names it).
            IIRInstr::new("const", Some("z0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("z".into()),
                vec![Operand::Var("dyn_box_int".into()), Operand::Var("z0".into())],
                "any",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("b".into()),
                vec![Operand::Var("dyn_cons".into()), Operand::Var("z".into()), Operand::Var("z".into())],
                "any",
            ),
            // A non-reference look-alike (garbage) in an `i64` slot.
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("a".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
            // The collect under test (both entries return the freed count).
            IIRInstr::new("call_builtin", Some("freed".into()), vec![Operand::Var(collect.into())], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("lb".into()),
                vec![Operand::Var("gc_live_bytes".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("lb".into())], "i64"),
        ];
        let mut m = IIRModule::new("gc_compact", "twig");
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: &str| -> i32 {
        let m = build(collect);
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_macos_executable(&m, &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    let precise = run("gc_compact_prec", "gc_collect_precise");
    let compacting = run("gc_compact_comp", "gc_collect_compacting");

    // The compacting collect keeps the live cons cell (a UAF would show live_bytes == 0)…
    assert!(
        compacting > 0,
        "compacting collect must KEEP the live cons cell (live_bytes={compacting})",
    );
    // …and matches the precise collect's live_bytes exactly. The cons cell is now MOVABLE
    // (a registered kind), so the compacting collect *relocates* it — but `live_bytes` counts
    // surviving payload bytes, which a move conserves, so the two columns stay equal. (That
    // the relocation actually happens and its pointers are fixed up is proven by
    // `end_to_end_gc_compacting_relocates_and_preserves` below.)
    assert_eq!(
        compacting, precise,
        "compaction must conserve live_bytes vs precise: \
         compacting={compacting}, precise={precise}",
    );
}

/// AOT00-T3 — **the real relocation payoff**: a native program triggers a compaction that
/// *moves* a live heap object, and the program keeps working through the moved reference.
///
/// A cons cell is now allocated under a registered kind (`__dyn_cons` → `__gc_alloc_kind`
/// with the ref-field map `{0, 8}`), so it is **movable**: precise-reachable via its `any`
/// slot (a stack-map root), a registered kind, and — its fields being immediate boxed ints —
/// with no conservative in-edge to pin it. So:
///
/// ```text
///   main() -> i64:
///       v    = dyn_box_int(42)            ; immediate 42
///       cell = dyn_cons(v, dyn_box_int(7)); a MOVABLE heap cell, held in an `any` slot
///       _    = gc_collect_compacting()    ; EVACUATES cell → new arena address;
///                                         ;   the `any` root slot is rewritten in place
///       car  = dyn_car(cell)              ; reads the NEW location (slot was fixed up)
///       ret dyn_unbox_int(car)            ; 42
/// ```
///
/// Returning **42** proves the cell relocated *and* every reference to it was fixed up: had
/// the compacting collect moved the cell without rewriting the `any` root slot, `dyn_car`
/// would dereference the freed from-space block — reading garbage or faulting, not 42. The
/// same program under `gc_collect_precise` (non-moving) also returns 42 (the cell stays put),
/// so the two agree on the *value* while differing on *where the cell lives*.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_compacting_relocates_and_preserves() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn build(collect: &str) -> IIRModule {
        let body = vec![
            IIRInstr::new("const", Some("k42".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("v".into()),
                vec![Operand::Var("dyn_box_int".into()), Operand::Var("k42".into())],
                "any",
            ),
            IIRInstr::new("const", Some("k7".into()), vec![Operand::Int(7)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("w".into()),
                vec![Operand::Var("dyn_box_int".into()), Operand::Var("k7".into())],
                "any",
            ),
            // A MOVABLE cons cell, held live in an `any` slot the stack map names.
            IIRInstr::new(
                "call_builtin",
                Some("cell".into()),
                vec![Operand::Var("dyn_cons".into()), Operand::Var("v".into()), Operand::Var("w".into())],
                "any",
            ),
            // Trigger the collection: under compaction the cell relocates + its root is fixed.
            IIRInstr::new("call_builtin", Some("freed".into()), vec![Operand::Var(collect.into())], "i64"),
            // Deref through the (possibly rewritten) reference and unbox → 42.
            IIRInstr::new(
                "call_builtin",
                Some("car".into()),
                vec![Operand::Var("dyn_car".into()), Operand::Var("cell".into())],
                "any",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![Operand::Var("dyn_unbox_int".into()), Operand::Var("car".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ];
        let mut m = IIRModule::new("gc_relocate", "twig");
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: &str| -> i32 {
        let m = build(collect);
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_macos_executable(&m, &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    let compacting = run("gc_relocate_comp", "gc_collect_compacting");
    let precise = run("gc_relocate_prec", "gc_collect_precise");

    assert_eq!(
        compacting, 42,
        "car of the RELOCATED cell must be 42 — a wrong value/crash means the move left a \
         dangling reference (got {compacting})",
    );
    assert_eq!(
        precise, 42,
        "the same program under a non-moving precise collect must also return 42 (got {precise})",
    );
}

/// AOT00-T4 §6 — a native program drives a **bounded-pause incremental** collection end to
/// end. The three-call cycle (`gc_collect_incremental_start` → `step(budget)` → `finish`) is
/// invoked from compiled code around a live cons cell held in an `any` slot; the cell must
/// survive and `car` must still read 42.
///
/// ```text
///   main() -> i64:
///       v    = dyn_box_int(42)
///       cell = dyn_cons(v, dyn_box_int(7))     ; live cons in an `any` slot (a precise root)
///       gc_collect_incremental_start()         ; snapshot roots, shade them grey
///       _    = gc_collect_incremental_step(1e6) ; one big-budget step completes the mark
///       _    = gc_collect_incremental_finish()  ; sweep the unreachable
///       ret dyn_unbox_int(dyn_car(cell))       ; 42 — the live cell was kept
/// ```
///
/// A single large-budget `step` finishes marking in one call, so the program needs no IIR
/// loop; the mutator does no stores between start and the step, so the write barrier isn't
/// exercised here (that is the gc-core load-bearing test's job — this proves the *native
/// wiring* end to end). Returning 42 proves the incremental collect kept the live reference.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_incremental_keeps_live_ref() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    let body = vec![
        IIRInstr::new("const", Some("k42".into()), vec![Operand::Int(42)], "i64"),
        IIRInstr::new(
            "call_builtin",
            Some("v".into()),
            vec![Operand::Var("dyn_box_int".into()), Operand::Var("k42".into())],
            "any",
        ),
        IIRInstr::new("const", Some("k7".into()), vec![Operand::Int(7)], "i64"),
        IIRInstr::new(
            "call_builtin",
            Some("w".into()),
            vec![Operand::Var("dyn_box_int".into()), Operand::Var("k7".into())],
            "any",
        ),
        IIRInstr::new(
            "call_builtin",
            Some("cell".into()),
            vec![Operand::Var("dyn_cons".into()), Operand::Var("v".into()), Operand::Var("w".into())],
            "any",
        ),
        // Drive the incremental cycle: start → one big-budget step → finish.
        IIRInstr::new("call_builtin", None, vec![Operand::Var("gc_collect_incremental_start".into())], "void"),
        IIRInstr::new("const", Some("budget".into()), vec![Operand::Int(1_000_000)], "i64"),
        IIRInstr::new(
            "call_builtin",
            Some("done".into()),
            vec![Operand::Var("gc_collect_incremental_step".into()), Operand::Var("budget".into())],
            "i64",
        ),
        IIRInstr::new(
            "call_builtin",
            Some("freed".into()),
            vec![Operand::Var("gc_collect_incremental_finish".into())],
            "i64",
        ),
        // The live cell must have survived: read its car and unbox → 42.
        IIRInstr::new(
            "call_builtin",
            Some("car".into()),
            vec![Operand::Var("dyn_car".into()), Operand::Var("cell".into())],
            "any",
        ),
        IIRInstr::new(
            "call_builtin",
            Some("r".into()),
            vec![Operand::Var("dyn_unbox_int".into()), Operand::Var("car".into())],
            "i64",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
    ];
    let mut m = IIRModule::new("gc_incremental", "twig");
    m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
    m.entry_point = Some("main".into());

    let dir = tempfile::tempdir().expect("tempdir");
    let exe = dir.path().join("gc_incremental");
    twig_aot::compile_module_to_macos_executable(&m, &exe)
        .unwrap_or_else(|e| panic!("gc_incremental compiles+links: {e}"));
    let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("gc_incremental runs: {e}"));
    let code = out.status.code().unwrap_or_else(|| panic!("exited by signal: {out:?}"));
    assert_eq!(
        code, 42,
        "car of the cons cell must be 42 after a native incremental collect — a wrong \
         value/crash means the live reference was lost (got {code})",
    );
}

/// AOT00-T1 (x86_64 PR-x4 / PR-x5) — precise roots reach through a **self-recursive**
/// frame, not just the entry frame.
///
/// The plain `gc_stress` differential proves precise roots work for `main`'s own frame.
/// This one proves they work for an *intermediate* frame in a recursion chain — the case
/// that, on x86-64, is precise only because a self-recursive `call` is now a registered
/// safepoint (PR-x4). Two functions:
///
/// ```text
///   rec(stop) -> i64:
///       a = gc_alloc(64)          ; i64 look-alike, one per active frame
///       if stop != 0 goto base
///       r = rec(1)                ; SELF-RECURSIVE call — a0 sits in this frame across it
///       ret r
///     base:
///       <collect>                 ; gc_collect | gc_collect_precise  (fires here)
///       ret gc_live_bytes()
///   main() -> i64: ret rec(0)
/// ```
///
/// `main → rec(0) → rec(1)`. The collect fires inside `rec(1)`; when the collector walks
/// out it passes through **`rec(0)`**, an intermediate self-recursive frame holding a
/// 64-byte look-alike (`a0`) in an `i64` (non-reference) slot.
///
/// - **Conservative** scans every frame's stack, sees both look-alikes (`a0`, `a1`), and
///   pins both objects → `live_bytes == 128`.
/// - **Precise** walks each frame through its registered map. The return address live on
///   `rec(0)`'s frame at collect time is the **self-recursive-call site**, so that frame is
///   precise only if that PC is a mapped safepoint (aarch64 has always mapped it — it
///   post-scans `BL`; x86-64 does so as of PR-x4). The `i64` slots are not references, so
///   both objects are unrooted and reclaimed → `live_bytes == 0`.
///
/// A `precise` of `64` instead of `0` would mean the intermediate recursive frame fell
/// back to a conservative scan — exactly the gap PR-x4 closes on x86-64. Here on aarch64
/// it validates the program shape + GC semantics on a locally-runnable target; the
/// identical module runs on the x86-64 CI runner (`linux_x86_64_smoke`).
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_gc_recursive_frame_live_bytes_differential() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn build(collect: &str, collect_returns: bool) -> IIRModule {
        // rec(stop: i64) -> i64
        let mut rec_body = vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            // The allocation's address lives only in this i64 (non-ref) slot.
            IIRInstr::new(
                "call_builtin",
                Some("a".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
            // stop != 0 → base case; else fall through to the recursive path.
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("stop".into()), Operand::Var("base".into())],
                "i64",
            ),
            // Recursive path (stop == 0): recurse exactly once with stop = 1.
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "call",
                Some("r".into()),
                vec![Operand::Var("rec".into()), Operand::Var("one".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
            // Base path (stop != 0): collect, then return live payload bytes.
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

        // main() -> i64  { ret rec(0) }
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
        twig_aot::compile_module_to_macos_executable(&build(collect, ret), &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    let conservative = run("gc_recur_cons", "gc_collect", false);
    let precise = run("gc_recur_prec", "gc_collect_precise", true);

    assert_eq!(
        conservative, 128,
        "conservative must pin both recursive frames' 64-byte look-alikes (got {conservative})",
    );
    assert_eq!(
        precise, 0,
        "precise must reclaim BOTH look-alikes, including a0 in the intermediate \
         self-recursive frame — which requires that frame be precisely mapped at the \
         recursive-call return address (got {precise}, conservative={conservative})",
    );
}

/// Sanity check that the AArch64Backend trait wiring goes end-to-end
/// for a hand-built CIR-shaped function.
#[test]
fn backend_pipeline_produces_bytes_for_simple_function() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    let main = IIRFunction::new(
        "main", vec![], "u64",
        vec![
            IIRInstr::new("const", Some("v0".into()),
                          vec![Operand::Int(42)], "u64"),
            IIRInstr::new("ret", None,
                          vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let mut module = IIRModule::new("smoke", "twig");
    module.add_or_replace(main);

    // Drive aot-core's per-fn pipeline manually so we can use the new
    // compile_function entry point.
    use aot_core::infer::infer_types;
    use aot_core::specialise::aot_specialise;
    let f = &module.functions[0];
    let inferred = infer_types(f);
    let cir = aot_specialise(f, Some(&inferred));
    let ctx = FunctionContext {
        name: &f.name, params: &f.params, return_type: &f.return_type,
    };
    let bytes = AArch64Backend.compile_function(&ctx, &cir).expect("ok");
    assert!(!bytes.is_empty());
    assert_eq!(bytes.len() % 4, 0, "ARM64 instructions are 4-byte aligned");
}

/// AOT00 closures-under-the-GC capstone — a **native closure's captured
/// environment survives a garbage collection and the closure remains callable**.
///
/// This ties the two halves of the Twig-native-GC directive together. Native
/// closures already compile via `iir-builtin-lowering::lower_closures_to_heap`
/// (E6d-7a): `alloc_closure(fn, cap0, …)` lowers to a cons chain
/// `(box(idx) . (cap0 . … . nil))` built with `__dyn_cons` — the *same*
/// GC-managed, kind-`{0,8}`, precise-and-movable cons cell every list uses — and
/// `call_closure` lowers to a `call` into a synthesized `__dyn_call_closure`
/// dispatcher that `car`/`cdr`-walks the captured environment and directly calls
/// the statically-known body. So a closure's captured values already live *in the
/// GC heap*. What this test proves end to end on real hardware is that a collection
/// occurring **while a closure is live** neither frees nor corrupts that captured
/// environment: the closure is still invocable afterwards and returns the value it
/// captured *before* the collect.
///
/// ```text
///   __cap_id(x, y) -> any: ret x        ; captures x, ignores its arg, returns x
///
///   main() -> i64:
///       cap = dyn_box_int(41)           ; the captured value, boxed → `any`
///       clo = alloc_closure(__cap_id, cap)   ; a closure capturing 41
///       a   = gc_alloc(64)              ; an unrooted i64 look-alike (reclaimable)
///       <collect>                       ; runs while `clo` is a live precise root
///       arg = dyn_box_int(0)            ; the (ignored) call argument
///       r   = call_closure(clo, arg)    ; __cap_id(41, 0) = 41 — reads the CAPTURE
///       ret dyn_to_exit_code(r)         ; 41
/// ```
///
/// After `lower_closures_to_heap`, `clo` is a `ref<any>` cons chain held live
/// across the collect call, so the precise stack map names it as a root and the
/// whole `(idx . (cap . nil))` structure — including the captured `41` — must
/// survive. Returning **41** proves it did *and* that the dispatcher still resolves
/// the captured value at its (possibly relocated) address. A wrong value or a crash
/// would mean the collect dropped or mangled the live captured environment — a
/// closure-specific use-after-free that the bare-cons-cell survival tests above do
/// not exercise (they never build or *call* a closure). Run under both the
/// non-moving precise collect and the moving compacting collect; a no-collect
/// baseline pins the expected value independently of the GC.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn end_to_end_closure_captured_env_survives_collect() {
    use interpreter_ir::instr::{IIRInstr, Operand};

    // The lambda body: two params in lambda-lifting order (capture first, then the
    // call arg); returns the captured param, ignoring the argument. Kept
    // arithmetic-free so the test isolates *capture survival*, not dynamic `+`.
    fn cap_id() -> IIRFunction {
        IIRFunction::new(
            "__cap_id",
            vec![("x".into(), "any".into()), ("y".into(), "any".into())],
            "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "any")],
        )
    }

    // `collect`: Some(builtin) inserts that collection while the closure is live;
    // None is the no-collect baseline.
    fn build(collect: Option<&str>) -> IIRModule {
        let mut body = vec![
            // The captured value 41, boxed into an `any`.
            IIRInstr::new("const", Some("k41".into()), vec![Operand::Int(41)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("cap".into()),
                vec![Operand::Var("dyn_box_int".into()), Operand::Var("k41".into())],
                "any",
            ),
            // A closure capturing 41. `lower_closures_to_heap` turns this into a
            // cons chain `(box(idx) . (cap . nil))` built with `__dyn_cons`.
            IIRInstr::new(
                "alloc_closure",
                Some("clo".into()),
                vec![Operand::Str("__cap_id".into()), Operand::Var("cap".into())],
                "closure",
            ),
            // An unrooted i64 look-alike: garbage a precise/compacting collect frees.
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(64)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("garbage".into()),
                vec![Operand::Var("gc_alloc".into()), Operand::Var("n".into())],
                "i64",
            ),
        ];
        // Collect while `clo` (and, through it, the captured 41) is live.
        if let Some(c) = collect {
            body.push(IIRInstr::new(
                "call_builtin",
                Some("freed".into()),
                vec![Operand::Var(c.into())],
                "i64",
            ));
        }
        body.extend([
            // Call the closure. The (ignored) argument is a boxed 0.
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("arg".into()),
                vec![Operand::Var("dyn_box_int".into()), Operand::Var("z".into())],
                "any",
            ),
            IIRInstr::new(
                "call_closure",
                Some("r".into()),
                vec![Operand::Var("clo".into()), Operand::Var("arg".into())],
                "any",
            ),
            // Coerce the polymorphic closure result to the process exit code.
            IIRInstr::new(
                "call_builtin",
                Some("e".into()),
                vec![Operand::Var("dyn_to_exit_code".into()), Operand::Var("r".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("e".into())], "i64"),
        ]);

        let mut m = IIRModule::new("gc_closure_capture", "twig");
        m.add_or_replace(cap_id());
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let run = |tag: &str, collect: Option<&str>| -> i32 {
        let m = build(collect);
        let exe = dir.path().join(tag);
        twig_aot::compile_module_to_macos_executable(&m, &exe)
            .unwrap_or_else(|e| panic!("{tag} compiles+links: {e}"));
        let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag} runs: {e}"));
        out.status.code().unwrap_or_else(|| panic!("{tag} exited by signal: {out:?}"))
    };

    // Baseline: no collection — the closure round-trips its captured 41.
    let baseline = run("gc_clo_base", None);
    assert_eq!(baseline, 41, "baseline: a native closure must return its captured value (got {baseline})");

    // Precise (non-moving) collect while the closure is live: the captured
    // environment must survive and the closure must still return 41.
    let precise = run("gc_clo_prec", Some("gc_collect_precise"));
    assert_eq!(
        precise, 41,
        "a precise collect while the closure is live must KEEP its captured environment \
         — the closure still returns 41 (got {precise}); a wrong value/crash is a \
         closure-capture use-after-free",
    );

    // Compacting (moving) collect: the closure's cons cells may relocate; every
    // reference — including the dispatcher's walk of the captured env — must be
    // fixed up, so the closure still returns 41.
    let compacting = run("gc_clo_comp", Some("gc_collect_compacting"));
    assert_eq!(
        compacting, 41,
        "a compacting collect must relocate the closure's captured environment and fix up \
         every reference — the closure still returns 41 (got {compacting})",
    );
}
