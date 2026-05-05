//! # `twig-aot` — Twig ahead-of-time compiler.
//!
//! Compiles a Twig source file to a native ARM64 Mach-O executable that
//! macOS can launch directly.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │  twig_ir_compiler::compile_source
//!     ▼
//! IIRModule  (interpreter-ir)
//!     │  for each fn: aot_core::{infer, specialise} → CIR
//!     │  aarch64_backend::compile_function → ARM64 machine code
//!     ▼
//! Vec<(fn_name, Vec<u8>)>
//!     │  aot_core::link::link → (text_bytes, offsets)
//!     │  aot_core::link::entry_point_offset(entry="main")
//!     ▼
//! (text_bytes, entry_off)
//!     │  code_packager::macho_object::pack_object
//!     ▼
//! object.o  (Mach-O MH_OBJECT with `_main` symbol)
//!     │  ld -arch arm64 -platform_version macos 15.0 15.0 -e _main -o exe
//!     ▼
//! native ARM64 executable that exec()s without ENOEXEC
//! ```
//!
//! ## Why we shell out to `ld`
//!
//! On macOS 15+ (Sequoia / Tahoe) the kernel attaches a "provenance"
//! tag to every executable file recording which process wrote it.
//! Files written by the system linker (`/usr/bin/ld`, Apple-signed)
//! inherit a trusted provenance and run normally; files written by
//! random user code (e.g. our crate) are SIGKILL'd by
//! `AppleSystemPolicy` regardless of how well-formed the Mach-O is.
//!
//! Delegating the final link to `ld` solves the provenance problem.
//! As a bonus, `ld` also handles dyld setup, code signing, and the
//! various subtleties of producing a runnable Apple Silicon binary —
//! we just supply the `__text` bytes and the entry symbol.
//!
//! ## Execution model
//!
//! Every Twig program's `main` function returns a `u64`.  The linker
//! produces a binary that calls `_main` and routes the `x0` return
//! through `exit()`, so the process exit code equals `main()`'s return
//! value modulo 256.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::path::{Path, PathBuf};

use aarch64_backend::AArch64Backend;
use aot_core::infer::infer_types;
use aot_core::link::{entry_point_offset, link};
use aot_core::specialise::aot_specialise;
use code_packager::macho_object::pack_object;
use code_packager::{CodeArtifact, Target};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::module::IIRModule;
use jit_core::backend::{Backend, FunctionContext};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the AOT compiler.
#[derive(Debug)]
#[allow(dead_code)] // diagnostic carried via Debug formatting
pub enum AotError {
    /// `twig-ir-compiler` failed to parse / compile the source.
    Compile(String),
    /// The ARM64 backend rejected one of the functions (untyped or
    /// unsupported opcode).
    BackendRefused {
        /// Function name that the backend declined to compile.
        function: String,
    },
    /// The IIR module has no `entry_point` set.
    NoEntryPoint,
    /// `code-packager` rejected the artifact (malformed or unsupported
    /// target).
    Packager(String),
    /// Filesystem error while writing the output binary.
    Io(std::io::Error),
    /// The system linker (`ld`) returned a non-zero exit code or could
    /// not be located on `PATH`.
    Linker {
        /// `ld`'s exit code, if it ran at all.
        status: Option<i32>,
        /// `ld`'s stderr, captured for diagnostics.
        stderr: String,
    },
}

impl std::fmt::Display for AotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AotError::Compile(s)            => write!(f, "twig compile: {s}"),
            AotError::BackendRefused { function } =>
                write!(f, "backend refused function '{function}' (untyped or unsupported op)"),
            AotError::NoEntryPoint          => write!(f, "module has no entry point"),
            AotError::Packager(s)           => write!(f, "packager: {s}"),
            AotError::Io(e)                 => write!(f, "io: {e}"),
            AotError::Linker { status, stderr } =>
                write!(f, "linker (ld) failed: status={status:?}: {stderr}"),
        }
    }
}

impl std::error::Error for AotError {}

impl From<std::io::Error> for AotError {
    fn from(e: std::io::Error) -> Self { AotError::Io(e) }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a Twig source string to a Mach-O **object file** (`MH_OBJECT`),
/// suitable for feeding to `ld`.
///
/// `module_name` is used in profile dumps and error messages; pick
/// something descriptive of the source file (e.g. its stem).
///
/// Returns the raw object-file bytes; the caller is responsible for
/// running `ld` to produce an executable.  See
/// [`compile_file_macos_arm64`] for the end-to-end flow.
pub fn compile_macos_arm64_object(source: &str, module_name: &str) -> Result<Vec<u8>, AotError> {
    let module = twig_ir_compiler::compile_source(source, module_name)
        .map_err(|e| AotError::Compile(format!("{e}")))?;
    compile_module_macos_arm64_object(&module)
}

/// Compile an already-built `IIRModule` to Mach-O object-file bytes.
pub fn compile_module_macos_arm64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    let entry = module.entry_point.as_deref().ok_or(AotError::NoEntryPoint)?;
    let (text, offsets) = compile_module_to_text(module)?;
    let entry_off = entry_point_offset(&offsets, Some(entry));

    let artifact = CodeArtifact::new(text, entry_off, Target::macos_arm64());
    pack_object(&artifact).map_err(|e| AotError::Packager(format!("{e}")))
}

/// Compile a Twig source file to a runnable ARM64 Mach-O executable on
/// disk by:
///
/// 1. Generating the `.o` object file (`compile_macos_arm64_object`).
/// 2. Writing it to a temp file.
/// 3. Invoking `ld` to produce the final executable at `out_path`.
/// 4. Marking the output `0o755`.
///
/// See [`AotError::Linker`] for `ld` invocation failures.
#[cfg(unix)]
pub fn compile_file_macos_arm64(
    src_path: &Path,
    out_path: &Path,
) -> Result<(), AotError> {
    use std::os::unix::fs::PermissionsExt;
    let source = std::fs::read_to_string(src_path)?;
    let stem = src_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("twig");
    let object_bytes = compile_macos_arm64_object(&source, stem)?;

    // Object files go to a temp file the linker reads.  We can't
    // deterministically name it (concurrent invocations would collide)
    // so use the OS tempdir.
    let tmp_dir  = std::env::temp_dir();
    let tmp_obj  = tmp_dir.join(format!("twig-aot-{}-{}.o", stem, std::process::id()));
    std::fs::write(&tmp_obj, &object_bytes)?;

    let link_result = invoke_ld(&tmp_obj, out_path);
    let _ = std::fs::remove_file(&tmp_obj); // best-effort cleanup
    link_result?;

    let mut perms = std::fs::metadata(out_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(out_path, perms)?;
    Ok(())
}

/// Run Apple's system linker on `object_path`, producing `out_path`.
///
/// The arguments are deliberately conservative:
/// - `-arch arm64`            — explicit target arch
/// - `-platform_version macos 15.0 15.0` — minOS + SDK declaration
/// - `-e _main`               — entry symbol (matches our object's symtab)
/// - `-o <out>`               — output path
///
/// We intentionally **do not** pass `-static`.  Modern macOS heavily
/// privileges binaries that link against `libSystem` (the standard C
/// runtime) — they get the trusted toolchain provenance and pass the
/// kernel's security policy.  Our compiled `_main` doesn't actually
/// call any libSystem function (it makes raw `svc` syscalls), so the
/// link is "free" in the sense that no library code ends up reachable
/// from `main`, but the LC_LOAD_DYLIB stub makes the kernel happy.
///
/// `ld` itself sets up: `LC_LOAD_DYLINKER`, `LC_LOAD_DYLIB libSystem`,
/// `LC_DYLD_CHAINED_FIXUPS`, ad-hoc code signature, etc.
fn invoke_ld(object_path: &Path, out_path: &Path) -> Result<(), AotError> {
    // `-lSystem` is non-negotiable on modern macOS: `ld` refuses to
    // produce a dynamic executable without linking the C runtime.
    // Our compiled `_main` doesn't actually call any libSystem
    // function (it makes raw `svc` syscalls), so the link is "free"
    // in terms of reachability — but the LC_LOAD_DYLIB stub is what
    // makes the kernel accept the binary.
    //
    // `-L<sdk>/usr/lib` tells ld where to find `libSystem.tbd`.  We
    // probe `xcrun --sdk macosx --show-sdk-path` first, falling back
    // to the conventional `/usr/lib` if Xcode CLT isn't installed.
    let sdk_lib = sdk_lib_path();

    let output = std::process::Command::new("ld")
        .arg("-arch").arg("arm64")
        .arg("-platform_version").arg("macos").arg("15.0").arg("15.0")
        .arg("-e").arg("_main")
        .arg("-L").arg(&sdk_lib)
        .arg("-lSystem")
        .arg("-o").arg(out_path)
        .arg(object_path)
        .output()
        .map_err(|e| AotError::Linker {
            status: None,
            stderr: format!("ld not found on PATH or could not be spawned: {e}"),
        })?;

    if !output.status.success() {
        return Err(AotError::Linker {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

/// Locate `<sdk>/usr/lib` so `ld` can find `libSystem.tbd`.
///
/// First tries `xcrun --sdk macosx --show-sdk-path`, which works on any
/// machine with the Xcode Command Line Tools installed.  Falls back to
/// `/usr/lib` for machines where `xcrun` is missing or fails.
fn sdk_lib_path() -> PathBuf {
    if let Ok(o) = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
    {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() {
                return PathBuf::from(s).join("usr").join("lib");
            }
        }
    }
    PathBuf::from("/usr/lib")
}


// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Run the per-function AOT pipeline and link into a single text section.
fn compile_module_to_text(
    module: &IIRModule,
) -> Result<(Vec<u8>, std::collections::HashMap<String, usize>), AotError> {
    let backend = AArch64Backend;

    let mut fn_binaries: Vec<(String, Vec<u8>)> = Vec::with_capacity(module.functions.len());
    for fn_ in &module.functions {
        let bytes = compile_one(&backend, fn_)
            .ok_or_else(|| AotError::BackendRefused { function: fn_.name.clone() })?;
        fn_binaries.push((fn_.name.clone(), bytes));
    }

    Ok(link(&fn_binaries))
}

fn compile_one<B: Backend>(backend: &B, fn_: &IIRFunction) -> Option<Vec<u8>> {
    let inferred = infer_types(fn_);
    let cir = aot_specialise(fn_, Some(&inferred));
    let ctx = FunctionContext {
        name:        &fn_.name,
        params:      &fn_.params,
        return_type: &fn_.return_type,
    };
    backend.compile_function(&ctx, &cir)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_with_no_entry_point_errors() {
        let mut m = IIRModule::new("noent", "twig");
        m.entry_point = None;
        assert!(matches!(
            compile_module_macos_arm64_object(&m),
            Err(AotError::NoEntryPoint)
        ));
    }

    #[test]
    fn untyped_twig_returns_backend_refused() {
        // `(define x 5)` is a top-level value define — the ir compiler
        // emits a `global_set` call_builtin which the V1 backend doesn't
        // support → BackendRefused.
        let src = "(define x 5) x";
        let err = compile_macos_arm64_object(src, "untyped").unwrap_err();
        assert!(matches!(err, AotError::BackendRefused { .. }), "got: {err:?}");
    }

    #[test]
    fn empty_main_compiles_to_object_bytes() {
        use interpreter_ir::function::IIRFunction;
        use interpreter_ir::instr::{IIRInstr, Operand};

        let main = IIRFunction::new(
            "main", vec![], "u64",
            vec![
                IIRInstr::new("const", Some("v0".into()),
                              vec![Operand::Int(0)], "u64"),
                IIRInstr::new("ret", None,
                              vec![Operand::Var("v0".into())], "u64"),
            ],
        );
        let mut m = IIRModule::new("hello", "twig");
        m.add_or_replace(main);
        m.entry_point = Some("main".into());

        let bytes = compile_module_macos_arm64_object(&m).expect("ok");
        // Mach-O magic for 64-bit LE is 0xCFFAEDFE.  This is an MH_OBJECT.
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }
}
