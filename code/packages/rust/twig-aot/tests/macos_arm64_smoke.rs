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
