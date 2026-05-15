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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aarch64_backend::{compile_with_globals, GlobalWordReloc, Reloc};
use aot_core::infer::infer_types;
use aot_core::link::{entry_point_offset, link};
use aot_core::specialise::aot_specialise;
use code_packager::macho_object::{
    pack_object_with_globals_and_externals, ExternBranchReloc, GlobalByteReloc,
};
use code_packager::{
    pack_elf64_object_x86_64, pack_pe_object_x86_64,
    Target, X86RelocKind, X86RelocRecord,
};

// ── Embedded Twig AOT runtime archive ──────────────────────────────────────
//
// `build.rs` compiles `runtime/twig_runtime.c` (via the `cc` crate) into a
// static archive at `$OUT_DIR/libtwig_aot_runtime.a` and exports the path
// via `cargo:rustc-env=TWIG_RUNTIME_ARCHIVE`.  `include_bytes!` resolves
// the path at *compile time* (not runtime) and bakes the archive bytes into
// this binary.
//
// At AOT link time (when the user runs `twig-aot`), we write these bytes to
// a temp file and pass it to the system linker (`ld`).  `ld` extracts
// whatever symbols it needs (e.g. `__twig_print_i64`) from the archive and
// links them into the final executable.  This means:
//
// - No separate installation of a runtime library.
// - Portable: `twig_runtime.c` uses `<stdio.h>` (printf), which resolves
//   through `-lSystem` on macOS and `-lc` on Linux — no raw syscall numbers.
static RUNTIME_ARCHIVE: &[u8] = include_bytes!(env!("TWIG_RUNTIME_ARCHIVE"));

/// Linux x86-64 runtime archive embedded at compile time (LANG46).
///
/// On a Linux x86-64 build host, this is the real `.a` archive built by
/// `build.rs` via the `cc` crate.  On other hosts it is a 1-byte stub;
/// `compile_file_linux_x86_64` checks the length and refuses with a
/// clear error if invoked on a non-Linux host's build.
static RUNTIME_LINUX_X86_64: &[u8] =
    include_bytes!(env!("TWIG_RUNTIME_ARCHIVE_LINUX_X86_64"));

/// Windows x86-64 runtime archive embedded at compile time (LANG46).
///
/// On a Windows x86-64 build host, this is the real `.lib` archive
/// built by `build.rs` via the `cc` crate using MSVC's `cl.exe`.  On
/// other hosts it is a 1-byte stub.
static RUNTIME_WINDOWS_X86_64: &[u8] =
    include_bytes!(env!("TWIG_RUNTIME_ARCHIVE_WINDOWS_X86_64"));
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use iir_builtin_lowering::lower_global_io;
use iir_refinement_pass::{check_module as check_refinements, RefinementMode};
use jit_core::backend::FunctionContext;

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
    /// One or more refinement proof obligations were violated (LANG42).
    ///
    /// In `Lenient` mode this fires only for `ProvenUnsafe` outcomes.
    /// In `Strict` mode it also fires for `Unknown` outcomes.
    RefinementViolations(Vec<iir_refinement_pass::RefinementError>),
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
            AotError::RefinementViolations(errs) => {
                write!(f, "{} refinement violation(s):", errs.len())?;
                for e in errs {
                    write!(f, "\n  {e}")?;
                }
                Ok(())
            }
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
///
/// When the module references global variables (LANG39), the object file
/// is produced via [`pack_object_with_globals`] which adds a `__DATA/__data`
/// section and emits `ARM64_RELOC_PAGE21` / `ARM64_RELOC_PAGEOFF12`
/// relocation records so that the system linker (`ld`) can patch the
/// `ADRP + ADD` instruction pairs that address `_twig_globals`.
pub fn compile_module_macos_arm64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    compile_module_macos_arm64_object_with_mode(module, RefinementMode::Lenient)
}

/// Like [`compile_module_macos_arm64_object`] but with an explicit
/// [`RefinementMode`].
///
/// Use `RefinementMode::Strict` for `(typed strict)` modules (TW05-A) or any
/// pipeline that must treat `Unknown` solver outcomes as hard errors.
pub fn compile_module_macos_arm64_object_with_mode(
    module: &IIRModule,
    refinement_mode: RefinementMode,
) -> Result<Vec<u8>, AotError> {
    // ── LANG42: refinement obligation pass ──────────────────────────────────
    //
    // Run the pass on the *original* IIR before any lowering.  After
    // `prepare_module_for_aot` runs, variable names are rewritten and the
    // correspondence between names and annotations is lost — so we must check
    // before we mutate the module.
    //
    // `check_refinements` returns an empty Vec if there are no violations.
    let refinement_errors = check_refinements(module, refinement_mode);
    if !refinement_errors.is_empty() {
        return Err(AotError::RefinementViolations(refinement_errors));
    }

    let entry = module.entry_point.as_deref().ok_or(AotError::NoEntryPoint)?;
    let (text, offsets, n_global_slots, global_relocs, extern_relocs) =
        compile_module_to_text(module)?;
    let entry_off = entry_point_offset(&offsets, Some(entry));

    // Use pack_object_with_globals_and_externals for all cases (LANG41).
    //
    // This function always produces a two-section Mach-O (text + data).
    // When neither globals nor externals are present the data section is
    // empty and the reloc/symbol tables are minimal — functionally
    // equivalent to the old `pack_object` path.
    //
    // External BL targets (e.g. `__twig_print_i64`) become N_UNDF | N_EXT
    // symbol-table entries + ARM64_RELOC_BRANCH26 records so the system
    // linker can patch them from the embedded runtime archive.
    pack_object_with_globals_and_externals(
        &text,
        entry_off,
        n_global_slots,
        &global_relocs,
        &extern_relocs,
        &Target::macos_arm64(),
    )
    .map_err(|e| AotError::Packager(format!("{e}")))
}

/// Returns raw ARM64 machine code bytes and a function-name→byte-offset map.
///
/// Uses the standard untyped prep pipeline (pre-lower + i64 normalization +
/// type propagation + default-any-to-i64).  The resulting bytes are a flat
/// code section with no Mach-O wrapping — suitable for in-process execution
/// via [`call_arm64_function_in_process`].
///
/// Global-variable relocation metadata (`n_global_slots`, `global_byte_relocs`)
/// is intentionally discarded here; callers that need a full Mach-O object
/// should use [`compile_module_macos_arm64_object`] instead.
pub fn compile_module_to_arm64_bytes(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>), AotError> {
    // Extern relocs (5th element) are discarded for in-process execution:
    // the JIT path does not link against the runtime archive, so `io_out`
    // BL placeholders remain unpatched.  Programs using `io_out` must
    // use the AOT path (compile_module_macos_arm64_object) to run.
    let (text, offsets, _, _, _) = compile_module_to_text(module)?;
    Ok((text, offsets))
}

/// Like [`compile_module_to_arm64_bytes`] but uses the caller-supplied type
/// annotations instead of forcing everything to `u64`.
///
/// The caller is responsible for having:
/// 1. Run `pre_lower_aot_builtins_on_module` (or equivalent).
/// 2. Run a type-inference pass (e.g. `iir-type-checker::infer_and_check`).
/// 3. Set any remaining `"any"` params to a concrete type (e.g. `"i64"`).
///
/// This function then runs `propagate_aot_types` (seeded from the caller's
/// type annotations) and `default_any_to_u64` to fill in any gaps, without
/// ever running `normalize_params_to_u64`.  Because params stay at whatever
/// type the caller gave them (e.g. `i64`), arithmetic and comparisons are
/// emitted with signed `i64` semantics — correct for negative numbers.
///
/// ## Why propagation is still needed
///
/// `iir-type-checker::infer_function` seeds its SSA environment only from
/// instruction dests with existing type hints — it does **not** seed from
/// `func.params`.  As a result, instructions of the form
/// `sub dest, param_var, const_var` stay `"any"` because `param_var` is not
/// found in the environment.  `propagate_aot_types` seeds from `func.params`
/// and picks up these remaining `"any"` instructions.
pub fn compile_typed_module_to_arm64_bytes(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>), AotError> {
    let mut module = module.clone();
    for func in &mut module.functions {
        pre_lower_aot_builtins(func);
        // Promote any params still marked "any" or "polymorphic" to "i64".
        //
        // The caller may have already set some params to "i64" from source-level
        // annotations (e.g. `(x : int)`).  `normalize_params_to_i64` only touches
        // params that are STILL "any" — so caller-set types are never overwritten.
        // This ensures that unannotated functions (no param_refinements) also get
        // signed-integer semantics, matching Twig's semantic model.
        normalize_params_to_i64(func);
        // Propagate types seeded from the (now-concrete) params.
        // This fills in instructions like `sub dest, param, const` that
        // iir-type-checker missed because it doesn't seed from func.params.
        propagate_aot_types(func);
        // Default any still-unresolvable arithmetic/mov hints to i64.
        // (Should be rare after propagation with typed params.)
        default_any_to_i64(func);
    }
    // Discard global and extern reloc metadata — typed callers that need a full
    // Mach-O object should use `compile_module_macos_arm64_object` instead.
    let (text, offsets, _, _, _) = compile_module_to_text_raw(&module)?;
    Ok((text, offsets))
}

/// Apply the AOT builtin pre-lowering pass to every function in `module`.
///
/// This converts `call_builtin "+" a b` → `add a b`, `call_builtin "<" a b`
/// → `cmp_lt a b`, etc.  It is exposed so callers can pre-lower an IIR
/// module before running their own type-inference pass.
pub fn pre_lower_aot_builtins_on_module(module: &mut IIRModule) {
    for func in &mut module.functions {
        pre_lower_aot_builtins(func);
    }
}

/// Execute compiled ARM64 code in-process by mapping it into executable memory,
/// calling `fn_name(arg)`, and returning the result.
///
/// ## How it works
///
/// macOS does not allow `mmap` of file-backed pages with `PROT_EXEC` from
/// user code — that path requires the `com.apple.security.cs.allow-jit`
/// entitlement and fails with `EPERM` otherwise.  The simpler approach:
///
/// 1. Allocate an anonymous `PROT_READ | PROT_WRITE` page via `mmap`.
/// 2. `memcpy` the ARM64 bytes into the mapping.
/// 3. Call `mprotect` to change protection to `PROT_READ | PROT_EXEC`.
///    This is permitted on macOS without any special entitlement.
/// 4. Cast the byte at `fn_name`'s offset to `extern "C" fn(i64) -> i64`
///    and call it.
/// 5. `munmap` the region.
///
/// The two-step write-then-protect pattern is the standard JIT technique
/// on hardened macOS without `MAP_JIT`.
///
/// ## Calling convention
///
/// The compiled function must follow AAPCS64: argument in `x0`, result in `x0`.
/// This matches how the twig AOT backend generates function prologues.
///
/// ## Platform
///
/// macOS/ARM64 only.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn call_arm64_function_in_process(
    code: &[u8],
    offsets: &HashMap<String, usize>,
    fn_name: &str,
    arg: i64,
) -> Result<i64, AotError> {
    extern "C" {
        fn mmap(
            addr: *mut std::ffi::c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut std::ffi::c_void;
        fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
        fn mprotect(addr: *mut std::ffi::c_void, len: usize, prot: i32) -> i32;
    }
    const PROT_READ:  i32 = 0x01;
    const PROT_WRITE: i32 = 0x02;
    const PROT_EXEC:  i32 = 0x04;
    const MAP_PRIVATE: i32 = 0x0002;
    const MAP_ANON:    i32 = 0x1000; // macOS anonymous mapping flag

    let fn_offset = offsets.get(fn_name).copied().ok_or_else(|| AotError::BackendRefused {
        function: fn_name.to_string(),
    })?;

    // Round length up to a multiple of the page size (4096 on ARM64).
    let len = (code.len() + 4095) & !4095;

    // Step 1: allocate a writable anonymous mapping.
    let ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        )
    };
    // mmap returns MAP_FAILED = (void*)-1 = usize::MAX on failure.
    if ptr as usize == usize::MAX {
        return Err(AotError::Linker {
            status: None,
            stderr: "mmap(PROT_WRITE) failed for in-process ARM64 execution".into(),
        });
    }

    // Step 2: copy code bytes into the mapping.
    unsafe {
        std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());
    }

    // Step 3: make the mapping executable (read + exec, no write).
    // This is permitted on macOS without MAP_JIT or any entitlement.
    let rc = unsafe { mprotect(ptr, len, PROT_READ | PROT_EXEC) };
    if rc != 0 {
        unsafe { munmap(ptr, len) };
        return Err(AotError::Linker {
            status: None,
            stderr: "mprotect(PROT_EXEC) failed for in-process ARM64 execution".into(),
        });
    }

    // Step 4: validate fn_offset is within the code buffer before pointer
    // arithmetic, then jump to it and call the function.
    // AAPCS64: argument in x0, result in x0.
    if fn_offset >= code.len() {
        // Offset out of range — compiler bug or mismatched (code, offsets) pair.
        // munmap before returning to avoid leaking the executable mapping.
        unsafe { munmap(ptr, len) };
        return Err(AotError::BackendRefused {
            function: format!(
                "{fn_name}: offset {fn_offset} >= code length {}",
                code.len()
            ),
        });
    }
    let fn_ptr = (ptr as *const u8).wrapping_add(fn_offset);
    let f: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(fn_ptr) };
    let result = f(arg);

    unsafe { munmap(ptr, len) };
    Ok(result)
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
///
/// **Platform note:** this function is a no-op stub on non-Unix platforms
/// (Windows) — it always returns [`AotError::Linker`] with a helpful
/// message.  Cross-compiling for macOS from Windows is not yet supported.
#[cfg(not(unix))]
pub fn compile_file_macos_arm64(
    _src_path: &Path,
    _out_path: &Path,
) -> Result<(), AotError> {
    Err(AotError::Linker {
        status: None,
        stderr: "twig-aot: native macOS compilation requires a Unix host \
                 (macOS or Linux with a cross-toolchain)"
            .to_string(),
    })
}

/// Compile a Twig source file to a runnable ARM64 Mach-O executable on
/// disk (Unix-only implementation).
///
/// See the stub above for the non-Unix signature.
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

    // Object files go to a secure temp file (O_EXCL + random name) so that
    // concurrent `twig-aot` invocations don't collide and symlink attacks
    // against a predictable path are not possible.  `NamedTempFile` deletes
    // the file automatically when it drops.
    {
        use std::io::Write as _;
        let mut tmp_obj = tempfile::Builder::new()
            .prefix(&format!("twig-aot-{stem}-"))
            .suffix(".o")
            .tempfile()?;
        tmp_obj.write_all(&object_bytes)?;
        invoke_ld(tmp_obj.path(), out_path)?;
        // tmp_obj drops here — temp file deleted by NamedTempFile destructor.
    }

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
/// - `<runtime.a>`            — Twig AOT runtime archive (provides `__twig_print_i64` etc.)
/// - `-lSystem`               — macOS C runtime (printf, exit, …)
/// - `-o <out>`               — output path
///
/// We intentionally **do not** pass `-static`.  Modern macOS heavily
/// privileges binaries that link against `libSystem` (the standard C
/// runtime) — they get the trusted toolchain provenance and pass the
/// kernel's security policy.
///
/// The Twig AOT runtime archive (`libtwig_aot_runtime.a`, compiled from
/// `runtime/twig_runtime.c` by `build.rs`) is embedded in this binary as
/// [`RUNTIME_ARCHIVE`] and written to a temp file for each link invocation.
/// `ld` extracts whatever symbols it needs (e.g. `__twig_print_i64`) from
/// the archive; the static-archive format means only referenced object files
/// are pulled in, so programs that don't use `io_out` don't pay for the
/// print helper.
///
/// `ld` sets up: `LC_LOAD_DYLINKER`, `LC_LOAD_DYLIB libSystem`,
/// `LC_DYLD_CHAINED_FIXUPS`, ad-hoc code signature, etc.
fn invoke_ld(object_path: &Path, out_path: &Path) -> Result<(), AotError> {
    use std::io::Write as _;

    // Write the embedded runtime archive to a secure temp file.
    //
    // Security: we use `tempfile::Builder` (O_EXCL + random suffix) rather than
    // a PID-derived path.  A PID-based name is predictable and can be raced by
    // an attacker with write access to `$TMPDIR` (TOCTOU / symlink attack):
    //
    // - Symlink write-through: if a symlink at the predicted path already exists,
    //   `fs::write` would follow it and overwrite an arbitrary file.
    // - Replace-after-write: the attacker races to replace the just-written
    //   archive with a malicious one before `ld` opens it, injecting arbitrary
    //   machine code into the produced binary.
    //
    // `NamedTempFile` prevents both: the kernel creates the file atomically with
    // `O_EXCL`, and the random name is not guessable.  The file is deleted when
    // `runtime_tmp` drops (after `ld` exits below).
    let mut runtime_tmp = tempfile::Builder::new()
        .prefix("twig_aot_runtime_")
        .suffix(".a")
        .tempfile()?;
    runtime_tmp.write_all(RUNTIME_ARCHIVE)?;

    // `-lSystem` is non-negotiable on modern macOS: `ld` refuses to
    // produce a dynamic executable without linking the C runtime.
    // The runtime archive itself uses `printf` (from libSystem), so
    // linking both is required and correct.
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
        .arg(runtime_tmp.path()) // runtime archive: provides __twig_print_i64 etc.
        .output()
        .map_err(|e| AotError::Linker {
            status: None,
            stderr: format!("ld not found on PATH or could not be spawned: {e}"),
        })?;

    // `runtime_tmp` drops here — NamedTempFile deletes the temp archive file.
    drop(runtime_tmp);

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

// ===========================================================================
// x86-64 multi-target driver (LANG46 phase 2)
// ===========================================================================
//
// Mirrors the macOS ARM64 path above, but for Linux x86-64 (ELF + cc) and
// Windows x86-64 (PE/COFF + link.exe / lld-link / gcc).

use x86_64_backend::{compile_function_with_globals as x86_64_compile_with_globals, X86_64Abi};

/// Per-function compile for x86-64, then concatenate function bytes into a
/// single `.text` and lift each function's relocations into linked-text byte
/// offsets.
///
/// Cross-function calls *within the same module* end up as `PltRel32`
/// relocation records targeting another function in the module.  The linker
/// resolves those because every module function ends up as a symbol in the
/// emitted object (via the AOT runtime helper for `main` — multi-function
/// programs that need cross-function symbols are deferred to a follow-up).
///
/// Returns `(linked_text, fn_offsets, entry_byte_offset, relocs)`.
///
/// `entry_byte_offset` is the position of `main` in the linked text; the
/// caller passes it to `pack_*_object_x86_64`.
///
/// Pass strategy: this V1 driver performs **no in-place patching** of
/// cross-function call sites; all calls become `PltRel32` external
/// relocations.  For single-function programs (the smoke-test target),
/// that yields the correct linker output trivially.  Multi-function
/// programs that emit cross-function `call` records currently rely on
/// each function appearing as a global symbol in the emitted object —
/// a refinement deferred to a follow-up alongside multi-fn smoke tests.
fn compile_module_x86_64_to_text(
    module: &IIRModule,
    abi: X86_64Abi,
) -> Result<(Vec<u8>, HashMap<String, usize>, usize, Vec<X86RelocRecord>), AotError> {
    let global_slots = collect_global_slots(module);

    // Pass 1: compile each function with the x86_64 backend.
    let mut fn_results: Vec<(String, Vec<u8>, Vec<x86_64_encoder::ExternalReloc>)> =
        Vec::with_capacity(module.functions.len());

    for fn_ in &module.functions {
        let ctx = FunctionContext {
            name: &fn_.name,
            params: &fn_.params,
            return_type: &fn_.return_type,
        };
        let inferred = infer_types(fn_);
        let cir = aot_specialise(fn_, Some(&inferred));
        let (bytes, relocs) = x86_64_compile_with_globals(&ctx, &cir, abi, &global_slots)
            .map_err(|_| AotError::BackendRefused { function: fn_.name.clone() })?;
        fn_results.push((fn_.name.clone(), bytes, relocs));
    }

    // Concatenate function bytes and record per-function offsets.
    let plain: Vec<(String, Vec<u8>)> = fn_results.iter()
        .map(|(name, bytes, _)| (name.clone(), bytes.clone()))
        .collect();
    let (linked, offsets) = link(&plain);

    let entry = module.entry_point.as_deref().ok_or(AotError::NoEntryPoint)?;
    let entry_off = entry_point_offset(&offsets, Some(entry));

    // Pass 2: lift per-function reloc offsets into linked-text offsets.
    //
    // Reloc patch_offsets from each function are local to that function's
    // byte stream; add the function's start offset in the linked text so
    // the packager records the correct file-relative addresses.
    let mut all_relocs: Vec<X86RelocRecord> = Vec::new();
    for (fn_name, _bytes, fn_relocs) in &fn_results {
        let base = *offsets.get(fn_name.as_str()).unwrap_or(&0) as u32;
        for r in fn_relocs {
            all_relocs.push(X86RelocRecord {
                patch_offset: base + r.patch_offset as u32,
                symbol: r.symbol.clone(),
                kind: match r.kind {
                    x86_64_encoder::ExternalRelocKind::PltRel32   => X86RelocKind::PltRel32,
                    x86_64_encoder::ExternalRelocKind::PcRel32    => X86RelocKind::PcRel32,
                    x86_64_encoder::ExternalRelocKind::GotPcRel32 => X86RelocKind::GotPcRel32,
                },
                addend: r.addend,
            });
        }
    }

    Ok((linked, offsets, entry_off, all_relocs))
}

/// Compile an `IIRModule` to a Linux x86-64 ELF object file (`*.o`).
///
/// Returns the raw object bytes ready to hand to `cc` / `ld`.
pub fn compile_module_linux_x86_64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);
    let global_slots = collect_global_slots(&module);
    let (text, _, entry_off, relocs) =
        compile_module_x86_64_to_text(&module, X86_64Abi::SysV)?;
    pack_elf64_object_x86_64(
        &text, entry_off, global_slots.len(), &relocs, &Target::linux_x64(),
    ).map_err(|e| AotError::Packager(format!("{e}")))
}

/// Convenience: compile Twig source text directly to an ELF object.
pub fn compile_linux_x86_64_object(source: &str, module_name: &str) -> Result<Vec<u8>, AotError> {
    let module = twig_ir_compiler::compile_source(source, module_name)
        .map_err(|e| AotError::Compile(format!("{e}")))?;
    compile_module_linux_x86_64_object(&module)
}

/// Compile an `IIRModule` to a Windows x86-64 PE/COFF object file (`*.obj`).
pub fn compile_module_windows_x86_64_object(module: &IIRModule) -> Result<Vec<u8>, AotError> {
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);
    let global_slots = collect_global_slots(&module);
    let (text, _, entry_off, relocs) =
        compile_module_x86_64_to_text(&module, X86_64Abi::MsX64)?;
    pack_pe_object_x86_64(
        &text, entry_off, global_slots.len(), &relocs, &Target::windows_x64(),
    ).map_err(|e| AotError::Packager(format!("{e}")))
}

/// Convenience: compile Twig source text directly to a PE/COFF object.
pub fn compile_windows_x86_64_object(source: &str, module_name: &str) -> Result<Vec<u8>, AotError> {
    let module = twig_ir_compiler::compile_source(source, module_name)
        .map_err(|e| AotError::Compile(format!("{e}")))?;
    compile_module_windows_x86_64_object(&module)
}

// ---------------------------------------------------------------------------
// End-to-end pipelines: Twig source → object → system linker → executable
// ---------------------------------------------------------------------------

/// Compile a Twig source file to a runnable Linux x86-64 ELF executable.
///
/// Pipeline: source → IR → x86_64-backend (SysV ABI) → ELF object →
/// `cc` (the system C compiler) → executable.  Uses `cc` rather than
/// `ld` directly so that libc startup files (`crt1.o`, `crti.o`, etc.)
/// and dynamic-linker setup are handled correctly.
///
/// Requires a Linux build host (the embedded runtime archive is host-
/// specific per LANG46 phase 1).  Returns an error with a clear "no
/// Linux runtime archive on this host" message if invoked from a build
/// that wasn't done on Linux.
#[cfg(target_os = "linux")]
pub fn compile_file_linux_x86_64(src: &Path, out: &Path) -> Result<(), AotError> {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt;

    if RUNTIME_LINUX_X86_64.len() <= 4 {
        return Err(AotError::Linker {
            status: None,
            stderr: "twig-aot: no Linux x86-64 runtime archive on this host \
                     (build twig-aot on a Linux x86-64 host)".into(),
        });
    }

    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("twig");
    let obj_bytes = compile_linux_x86_64_object(&source, stem)?;

    let mut obj_tmp = tempfile::Builder::new()
        .prefix(&format!("twig-aot-{stem}-"))
        .suffix(".o")
        .tempfile()?;
    obj_tmp.write_all(&obj_bytes)?;

    let mut rt_tmp = tempfile::Builder::new()
        .prefix("twig_aot_runtime_")
        .suffix(".a")
        .tempfile()?;
    rt_tmp.write_all(RUNTIME_LINUX_X86_64)?;

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let output = std::process::Command::new(&cc)
        .arg("-o").arg(out)
        .arg(obj_tmp.path())
        .arg(rt_tmp.path())
        .arg("-lc").arg("-lm")
        .output()
        .map_err(|e| AotError::Linker {
            status: None,
            stderr: format!("twig-aot: {cc} not found on PATH: {e}"),
        })?;
    drop(obj_tmp);
    drop(rt_tmp);

    if !output.status.success() {
        return Err(AotError::Linker {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let mut perms = std::fs::metadata(out)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(out, perms)?;
    Ok(())
}

/// Compile a Twig source file to a runnable Windows x86-64 PE executable.
///
/// Pipeline: source → IR → x86_64-backend (MS x64 ABI) → PE/COFF object
/// → linker probe (`link.exe` → `lld-link.exe` → `gcc.exe`) → `.exe`.
///
/// Requires a Windows build host with at least one supported linker on
/// `PATH`.  Returns an error with a clear "no Windows linker found"
/// message if none is available.
#[cfg(target_os = "windows")]
pub fn compile_file_windows_x86_64(src: &Path, out: &Path) -> Result<(), AotError> {
    use std::io::Write as _;

    if RUNTIME_WINDOWS_X86_64.len() <= 4 {
        return Err(AotError::Linker {
            status: None,
            stderr: "twig-aot: no Windows x86-64 runtime archive on this host \
                     (build twig-aot on a Windows x86-64 host)".into(),
        });
    }

    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("twig");
    let obj_bytes = compile_windows_x86_64_object(&source, stem)?;

    let mut obj_tmp = tempfile::Builder::new()
        .prefix(&format!("twig-aot-{stem}-"))
        .suffix(".obj")
        .tempfile()?;
    obj_tmp.write_all(&obj_bytes)?;

    let mut rt_tmp = tempfile::Builder::new()
        .prefix("twig_aot_runtime_")
        .suffix(".lib")
        .tempfile()?;
    rt_tmp.write_all(RUNTIME_WINDOWS_X86_64)?;

    let linker = find_windows_linker().ok_or_else(|| AotError::Linker {
        status: None,
        stderr: "twig-aot: no Windows linker found on PATH \
                 (tried link.exe, lld-link.exe, gcc.exe)".into(),
    })?;

    let output = match linker.kind {
        WinLinkerKind::Link | WinLinkerKind::LldLink => {
            std::process::Command::new(&linker.path)
                .arg(format!("/OUT:{}", out.display()))
                .arg("/ENTRY:main")
                .arg("/SUBSYSTEM:CONSOLE")
                .arg(obj_tmp.path())
                .arg(rt_tmp.path())
                .arg("libcmt.lib")
                .arg("legacy_stdio_definitions.lib")
                .output()
        }
        WinLinkerKind::Gcc => {
            std::process::Command::new(&linker.path)
                .arg("-o").arg(out)
                .arg(obj_tmp.path())
                .arg(rt_tmp.path())
                .output()
        }
    }.map_err(|e| AotError::Linker {
        status: None,
        stderr: format!("twig-aot: linker spawn failed: {e}"),
    })?;
    drop(obj_tmp);
    drop(rt_tmp);

    if !output.status.success() {
        return Err(AotError::Linker {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

#[cfg(target_os = "windows")]
struct WinLinker {
    path: PathBuf,
    kind: WinLinkerKind,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
enum WinLinkerKind { Link, LldLink, Gcc }

/// Probe `PATH` for a supported Windows linker.  Returns the first one
/// found in priority order (link.exe → lld-link.exe → gcc.exe).
///
/// The probe checks the program's banner output rather than just
/// whether it can be spawned — git-bash environments ship a POSIX
/// `link(1)` utility on `PATH` that has the same name as MSVC's
/// `link.exe` but the wrong CLI grammar.
#[cfg(target_os = "windows")]
fn find_windows_linker() -> Option<WinLinker> {
    use std::process::Command;
    for (name, kind) in [
        ("link.exe", WinLinkerKind::Link),
        ("lld-link.exe", WinLinkerKind::LldLink),
        ("gcc.exe", WinLinkerKind::Gcc),
    ] {
        let probe = match kind {
            WinLinkerKind::Link | WinLinkerKind::LldLink => {
                // MSVC link.exe and lld-link.exe both print a banner
                // to stderr when invoked without args, then exit non-zero.
                Command::new(name).output()
            }
            WinLinkerKind::Gcc => Command::new(name).arg("--version").output(),
        };
        let Ok(o) = probe else { continue; };
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        let banner = format!("{stdout}{stderr}");
        let is_real = match kind {
            // MSVC link.exe banner contains "Microsoft (R) Incremental Linker".
            // POSIX `link(1)` (coreutils) prints "link: missing operand" instead.
            WinLinkerKind::Link    => banner.contains("Microsoft") && banner.contains("Linker"),
            WinLinkerKind::LldLink => banner.contains("LLD") || banner.contains("lld-link"),
            WinLinkerKind::Gcc     => banner.contains("gcc") || banner.contains("GCC"),
        };
        if is_real {
            return Some(WinLinker { path: PathBuf::from(name), kind });
        }
    }
    None
}


// ---------------------------------------------------------------------------
// AOT preparation pipeline
// ---------------------------------------------------------------------------
//
// The Twig IR compiler emits `call_builtin "<op>" arg…` for every primitive
// operation and leaves all `type_hint` fields as `"any"`.  The ARM64 backend
// requires typed CIR instructions (`add_u64`, `cmp_lt_u64`, `mov_u64`, …).
//
// This preparation pipeline bridges the gap in three steps:
//
//  Step 1 — Pre-lower builtins.
//    Converts `call_builtin "+" a b` → `add a b`, `call_builtin "_move" n` →
//    `mov n`, etc.  This happens at the IIR level so that type propagation can
//    see the named ops and infer concrete types.
//
//  Step 2 — Normalise param types.
//    The Twig IR compiler declares every param as `"any"`.  We promote them to
//    `"i64"` — Twig integers are semantically signed 64-bit values, so using
//    the signed default ensures that comparisons (e.g. `cmp_lt`) emit signed
//    ARM64 condition codes (`B.LT`) rather than unsigned ones (`B.CC`), which
//    matters for programs that test against negative numbers.  `infer_types`
//    (from `aot-core`) seeds its environment from `func.params`, so this is
//    the right place to override the "any" sentinel.
//
//  Step 3 — Propagate and default types.
//    A lightweight fixed-point pass populates `type_hint` on every instruction
//    whose type can be determined from its operands and the seeded param types.
//    Any remaining `"any"` hints on arithmetic and `mov` instructions are
//    then defaulted to `"i64"` so that `aot_specialise` never sees an
//    untyped instruction and never emits `type_assert` guards (which the ARM64
//    backend lowers to `udf` hard-traps).

/// Builtin-name → IIR-op for AOT.
const AOT_BUILTIN_MAP: &[(&str, &str)] = &[
    ("+",  "add"),  ("-",  "sub"),   ("*",  "mul"),   ("/",  "div"),
    ("=",  "cmp_eq"), ("<", "cmp_lt"), (">", "cmp_gt"),
    ("<=", "cmp_le"), (">=", "cmp_ge"),
    ("not", "not"),  ("_move", "mov"),
];

/// Step 1: lower `call_builtin "<op>"` instructions to named IIR ops.
fn pre_lower_aot_builtins(func: &mut IIRFunction) {
    let old = std::mem::take(&mut func.instructions);
    func.instructions = old.into_iter().map(|instr| {
        if instr.op != "call_builtin" { return instr; }
        let name = match instr.srcs.first() {
            Some(Operand::Var(n)) => n.as_str(),
            _ => return instr,
        };
        let Some((_, op)) = AOT_BUILTIN_MAP.iter().find(|(b, _)| *b == name) else {
            return instr;
        };
        let args: Vec<Operand> = instr.srcs[1..].to_vec();
        IIRInstr::new(*op, instr.dest.clone(), args, &instr.type_hint)
    }).collect();
}

/// Step 2: promote `"any"` / `"polymorphic"` param types to `"i64"`.
///
/// Twig integers are semantically signed 64-bit values.  Using `i64` as the
/// default ensures that comparison instructions like `cmp_lt` emit signed ARM64
/// condition codes (`B.LT` / `B.GE`) rather than unsigned ones (`B.CC` / `B.CS`).
/// This matters for any program that compares against negative numbers — e.g.
/// `(if (< x 0) …)` — where unsigned semantics would treat `-5` as a huge
/// positive number and take the wrong branch.
fn normalize_params_to_i64(func: &mut IIRFunction) {
    for (_, ty) in &mut func.params {
        if ty == "any" || ty == "polymorphic" {
            *ty = "i64".to_string();
        }
    }
}

/// Step 3a: fixed-point type propagation seeded from params.
///
/// Applies inference rules:
/// - `const` with `Int` → `"i64"`, `Bool` → `"bool"`
/// - `cmp_*` → **operand type** (not `"bool"`)
/// - `add`, `sub`, `mul`, `div`, `mov`, `neg`, `not` → type of first operand
///
/// ## Why `cmp_*` uses operand type, not `"bool"`
///
/// The ARM64 backend's `emit_cmp` uses the CIR mnemonic suffix to choose
/// between signed (`i64`, `i32`, …) and unsigned (`u64`, `u32`, …) condition
/// codes.  `cmp_lt_bool` produces an unsigned comparison — correct when all
/// values are non-negative, but wrong for negative numbers.
///
/// Returning the operand type lets the specialiser emit `cmp_lt_u64` for the
/// untyped path (u64 params) and `cmp_lt_i64` for the typed path (i64 params),
/// selecting the right condition code in each case.  The boolean result value
/// (0 or 1 stored to the dest register) is the same either way.
///
/// Runs in a loop until stable.  Function params (already "i64" or a typed hint)
/// seed the environment together with already-typed instructions.
fn propagate_aot_types(func: &mut IIRFunction) {
    // Build initial env from params + any already-typed instructions.
    let mut env: HashMap<String, String> = HashMap::new();
    for (name, ty) in &func.params {
        if ty != "any" && ty != "polymorphic" {
            env.insert(name.clone(), ty.clone());
        }
    }
    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            if instr.type_hint != "any" && instr.type_hint != "polymorphic" {
                env.insert(dest.clone(), instr.type_hint.clone());
            }
        }
    }

    // Fixed-point: keep iterating until no new types are inferred.
    loop {
        let mut changed = false;

        for instr in func.instructions.iter_mut() {
            if instr.type_hint != "any" { continue; }
            let Some(dest) = instr.dest.as_ref() else { continue; };

            let inferred = infer_aot_type(instr, &env);
            if let Some(ty) = inferred {
                instr.type_hint = ty.clone();
                if env.insert(dest.clone(), ty).as_deref() != Some(&instr.type_hint) {
                    changed = true;
                }
            }
        }

        if !changed { break; }
    }
}

/// Try to infer a concrete type for `instr` given the current SSA `env`.
///
/// Returns `None` when the instruction's type cannot yet be determined
/// (e.g. because a source variable is still `"any"`).
fn infer_aot_type(instr: &IIRInstr, env: &HashMap<String, String>) -> Option<String> {
    match instr.op.as_str() {
        "const" => match instr.srcs.first() {
            Some(Operand::Int(_))  => Some("i64".into()),
            Some(Operand::Bool(_)) => Some("bool".into()),
            _                       => None,
        },
        // Comparisons: use the operand type so the ARM64 backend can choose
        // signed vs unsigned condition codes.  `cmp_lt_u64` → unsigned CMP,
        // `cmp_lt_i64` → signed CMP.  Both store a bool 0/1 result.
        // Fall back to "bool" only when operand types are still unresolved —
        // that way the untyped path (which resolves to u64) still works.
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
            resolve_src_aot_type(instr.srcs.first(), env)
                .or_else(|| Some("bool".into()))
        }
        // Arithmetic + move: use the type of the first source operand.
        "add" | "sub" | "mul" | "div" | "mov" | "neg" | "not" => {
            resolve_src_aot_type(instr.srcs.first(), env)
        }
        _ => None,
    }
}

/// Resolve the type of a source `Operand` for AOT purposes.
fn resolve_src_aot_type(src: Option<&Operand>, env: &HashMap<String, String>) -> Option<String> {
    match src? {
        Operand::Var(name) => {
            let ty = env.get(name)?;
            if ty != "any" && ty != "polymorphic" {
                Some(ty.clone())
            } else {
                None
            }
        }
        Operand::Int(_)  => Some("i64".into()),
        Operand::Bool(_) => Some("bool".into()),
        _                => None,
    }
}

/// Step 0b: remove dead string-literal `const` instructions.
///
/// The twig-ir-compiler emits a pattern like:
/// ```text
/// const  %n1 = Var("x")            -- name register (string literal)
/// call_builtin "global_set" %n1 %v -- will be lowered by lower_global_io
/// ```
///
/// After `lower_global_io` runs, `call_builtin "global_set"` becomes
/// `global_store Str("x") Var("%v")` — the global name is now inline.
/// The `const %n1` instruction becomes dead code (its register is never
/// read again).  If left in place, `aot_specialise` converts it to
/// `const_str`, which the ARM64 backend cannot lower.
///
/// This pass removes `const` instructions whose source is `Operand::Var(_)`
/// (the string-literal-as-Var encoding) AND whose dest register never
/// appears in any other instruction's `srcs`.  Instructions that are
/// still referenced (e.g. as arg to an un-lowered `call_builtin`, or as
/// a `make_closure` name register) are retained.
fn strip_dead_string_consts(func: &mut IIRFunction) {
    use std::collections::HashSet;

    // Build a set of every Var-register name that is read in any src position.
    let used: HashSet<String> = func.instructions
        .iter()
        .flat_map(|instr| instr.srcs.iter())
        .filter_map(|op| {
            if let Operand::Var(n) = op { Some(n.clone()) } else { None }
        })
        .collect();

    // Remove const instructions of the form `%dest = Var("text")` whose
    // dest is not in `used`.  These are dead name-register loads.
    func.instructions.retain(|instr| {
        if instr.op == "const" {
            if let Some(Operand::Var(_)) = instr.srcs.first() {
                if let Some(dest) = &instr.dest {
                    return used.contains(dest);
                }
            }
        }
        true // keep everything else
    });
}

/// Step 3b: default any remaining `"any"` hints on arithmetic / move
/// instructions to `"i64"`.
///
/// This handles instructions whose sources are still `"any"` after the
/// propagation pass — most commonly the results of `call` instructions whose
/// return type is not tracked.  Defaulting to `"i64"` is correct for Twig:
/// all Twig integer literals and arithmetic results are semantically signed
/// 64-bit values.  Using `i64` here ensures that comparisons downstream
/// (e.g. `cmp_lt`) emit signed ARM64 condition codes and produce correct
/// results for negative numbers, regardless of whether the source was
/// annotated or not.
fn default_any_to_i64(func: &mut IIRFunction) {
    for instr in &mut func.instructions {
        if instr.type_hint != "any" { continue; }
        match instr.op.as_str() {
            "add" | "sub" | "mul" | "div" | "mod" | "mov" | "neg" | "not" => {
                instr.type_hint = "i64".to_string();
            }
            _ => {}
        }
    }
}

/// Apply all preparation steps to every function in `module`.
///
/// Steps:
///  0. `lower_global_io` — converts `call_builtin "global_set"` /
///     `"global_get"` to `global_store` / `global_load` (LANG39).
///     Must run first so the const-string look-back can see all instructions.
///  0b. `strip_dead_string_consts` — removes `const %n = Var("name")`
///     instructions that are now dead after step 0.  Without this pass,
///     `aot_specialise` converts them to `const_str` which the ARM64 backend
///     cannot lower (there is no stack-slot for a string pointer).
///  1. `pre_lower_aot_builtins` — lowers `call_builtin "+"` → `add`, etc.
///  2. `normalize_params_to_i64` — promotes untyped params to `i64`.
///  3. `propagate_aot_types` — fixed-point type propagation.
///  4. `default_any_to_i64` — defaults unresolved arithmetic types to `i64`.
fn prepare_module_for_aot(module: &mut IIRModule) {
    // Phase 0: lower global_set / global_get → global_store / global_load.
    lower_global_io(module);

    for func in &mut module.functions {
        // Phase 0b: remove dead name-register `const` instructions.
        // `lower_global_io` leaves `const %n = Var("x")` in place even after
        // the `global_set`/`global_get` that consumed it is rewritten.
        // These become `const_str` in CIR which the ARM64 backend rejects.
        strip_dead_string_consts(func);
        pre_lower_aot_builtins(func);
        normalize_params_to_i64(func);
        propagate_aot_types(func);
        default_any_to_i64(func);
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Run the per-function AOT pipeline and link into a single text section.
///
/// Clones `module`, runs the full AOT preparation pipeline (builtin
/// pre-lowering + u64 type normalisation + type propagation + default-any),
/// and then delegates to [`compile_module_to_text_raw`] for the actual
/// two-pass compile + link.
///
/// This is the "untyped u64" path: all params and arithmetic are treated as
/// `u64`.  For signed `i64` semantics use [`compile_typed_module_to_arm64_bytes`].
fn compile_module_to_text(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>, usize, Vec<GlobalByteReloc>, Vec<ExternBranchReloc>), AotError> {
    // We work on a clone so the caller's `IIRModule` is never mutated.
    // `prepare_module_for_aot` runs four passes (LANG39 adds step 0):
    //   0. `lower_global_io` — `call_builtin "global_set/get"` → `global_store/load`
    //   1. `call_builtin "+"` → `add`, etc.   (see `pre_lower_aot_builtins`)
    //   2. param types "any" → "i64"           (see `normalize_params_to_i64`)
    //   3. propagate + default remaining "any" (see `propagate_aot_types` /
    //                                            `default_any_to_i64`)
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);
    compile_module_to_text_raw(&module)
}

// ---------------------------------------------------------------------------
// Global-slot scanning (LANG39)
// ---------------------------------------------------------------------------

/// Scan all functions in a (post-`lower_global_io`) IIR module for
/// `global_load` / `global_store` instructions and assign each unique global
/// name a consecutive slot index (0, 1, 2, …).
///
/// Returns a map from global name → slot index.  If no globals are used the
/// map is empty.
fn collect_global_slots(module: &IIRModule) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for fn_ in &module.functions {
        for instr in &fn_.instructions {
            if instr.op == "global_load" || instr.op == "global_store" {
                if let Some(Operand::Str(name)) = instr.srcs.first() {
                    let next = map.len();
                    map.entry(name.clone()).or_insert(next);
                }
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Inner two-pass compile + link
// ---------------------------------------------------------------------------

/// Inner two-pass compile + link, operating on an already-prepared `IIRModule`.
///
/// Unlike [`compile_module_to_text`] this function **does not** clone or
/// prepare the module — it trusts that the caller has already run the
/// appropriate preparation (either [`prepare_module_for_aot`] for the untyped
/// u64 path, or just [`pre_lower_aot_builtins`] for the typed i64 path).
///
/// Two-pass strategy for cross-function calls:
///
/// Pass 1 — compile each function independently.  Cross-function `call`
///   instructions emit `BL #0` placeholder instructions and record
///   [`Reloc`] entries (callee name + word index in the per-function binary).
///   `global_load`/`global_store` instructions emit `ADRP + ADD` placeholder
///   pairs and record [`GlobalWordReloc`] entries (word indices for Mach-O
///   ARM64 relocations).
///
/// Link — concatenate all function binaries into one flat code section and
///   record each function's byte offset.
///
/// Pass 2 — patch every placeholder `BL` with the correct PC-relative
///   offset using the now-known function offsets.  `GlobalWordReloc` entries
///   are converted to [`GlobalByteReloc`] byte offsets (for `pack_object_with_globals`).
///
/// ARM64 `BL` encoding: opcode `0x94000000`, 26-bit signed PC-relative
/// offset in units of 4 bytes (instruction words).
///
/// Returns `(linked_text, fn_offsets, n_global_slots, global_byte_relocs, extern_branch_relocs)`.
///
/// `extern_branch_relocs` — BL instructions in the linked text that target
/// symbols not defined in this module (e.g. `__twig_print_i64`).  The
/// packager ([`pack_object_with_globals_and_externals`]) converts them into
/// `N_UNDF | N_EXT` symbol-table entries and `ARM64_RELOC_BRANCH26` records
/// so the system linker can patch them from the Twig AOT runtime archive.
fn compile_module_to_text_raw(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>, usize, Vec<GlobalByteReloc>, Vec<ExternBranchReloc>), AotError> {
    // ── Collect global names (LANG39) ─────────────────────────────────────────
    let global_slots = collect_global_slots(module);
    let n_global_slots = global_slots.len();

    // ── Pass 1: compile all functions, collecting cross-function + global relocs ─
    // Each entry: (fn_name, per-function bytes, ExternalRelocs, GlobalWordRelocs)
    let mut fn_results: Vec<(String, Vec<u8>, Vec<Reloc>, Vec<GlobalWordReloc>)> =
        Vec::with_capacity(module.functions.len());

    for fn_ in &module.functions {
        let (bytes, ext_relocs, glob_relocs) = compile_one_with_globals(fn_, &global_slots)
            .ok_or_else(|| AotError::BackendRefused { function: fn_.name.clone() })?;
        fn_results.push((fn_.name.clone(), bytes, ext_relocs, glob_relocs));
    }

    // ── LANG41: external symbols are resolved by the system linker ───────────
    //
    // Unlike the LANG40 approach (which injected a macOS-specific helper with
    // raw `SVC #0x80` syscalls), LANG41 lets BL instructions that target
    // symbols outside this module remain unpatched.  Pass 2 below collects
    // them as `ExternBranchReloc` entries; `pack_object_with_globals_and_externals`
    // emits them as `N_UNDF | N_EXT` symbol-table entries and
    // `ARM64_RELOC_BRANCH26` records so `ld` can patch them from the embedded
    // Twig AOT runtime archive (`libtwig_aot_runtime.a`).
    //
    // The runtime archive (compiled from `runtime/twig_runtime.c` by `build.rs`)
    // provides `__twig_print_i64` using `printf` from libc — portable across
    // macOS (`-lSystem`) and Linux (`-lc`).
    let mut extern_branch_relocs: Vec<ExternBranchReloc> = Vec::new();

    // ── Link: concatenate binaries and record byte offsets ──────────────────
    let plain_binaries: Vec<(String, Vec<u8>)> = fn_results
        .iter()
        .map(|(name, bytes, _, _)| (name.clone(), bytes.clone()))
        .collect();
    let (mut linked, offsets) = link(&plain_binaries);

    // ── Collect global byte relocs (LANG39) ───────────────────────────────
    //
    // Convert GlobalWordReloc (word indices relative to each function) to
    // GlobalByteReloc (byte offsets relative to the linked text section) by
    // adding the function's byte offset and multiplying word index by 4.
    let mut global_byte_relocs: Vec<GlobalByteReloc> = Vec::new();
    for (fn_name, _bytes, _, glob_relocs) in &fn_results {
        // Use an explicit error rather than `unwrap_or(&0)`.  A missing entry
        // in `offsets` here would mean a function was compiled but not linked,
        // which indicates a linker bug — silently using offset 0 would produce
        // relocation records pointing to the wrong instructions.
        let fn_byte_offset = *offsets.get(fn_name.as_str())
            .ok_or_else(|| AotError::Linker {
                status: None,
                stderr: format!(
                    "twig-aot: internal error: function '{}' missing from link offsets \
                     during global reloc collection",
                    fn_name
                ),
            })?;
        for gr in glob_relocs {
            global_byte_relocs.push(GlobalByteReloc {
                adrp_byte_offset: (fn_byte_offset + gr.adrp_word * 4) as u32,
                add_byte_offset:  (fn_byte_offset + gr.add_word  * 4) as u32,
            });
        }
    }

    // ── Pass 2: patch cross-function BL placeholders ──────────────────────
    for (fn_name, _bytes, relocs, _) in &fn_results {
        let fn_offset = *offsets.get(fn_name.as_str()).unwrap_or(&0);

        for reloc in relocs {
            // Byte offset of the placeholder `BL` instruction in the linked buffer.
            // Use checked arithmetic: a pathologically large function could make
            // word_idx * 4 or the addition overflow on 32-bit targets (or in
            // release builds on 64-bit targets where wrapping arithmetic is silent).
            let call_byte_offset = reloc.word_idx
                .checked_mul(4)
                .and_then(|off| fn_offset.checked_add(off))
                .ok_or_else(|| AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: BL relocation offset arithmetic overflow for '{}' in '{}'",
                        reloc.symbol, fn_name
                    ),
                })?;
            // Byte offset of the callee's first instruction.
            let Some(&callee_offset) = offsets.get(reloc.symbol.as_str()) else {
                // Callee not in this module — record as an external branch
                // relocation.  The system linker will patch this BL from
                // the Twig AOT runtime archive (or any dylib that defines
                // the symbol).  The `BL #0` placeholder (`0x94000000`)
                // already has the correct opcode; `ld` overwrites only the
                // 26-bit immediate.
                extern_branch_relocs.push(ExternBranchReloc {
                    byte_offset: call_byte_offset as u32,
                    symbol: reloc.symbol.clone(),
                });
                continue; // leave the BL #0 placeholder in place
            };

            // PC-relative offset in instruction words (4 bytes each).
            // The PC of `BL` is `call_byte_offset`.
            let delta_bytes = callee_offset as i64 - call_byte_offset as i64;
            let delta_words = delta_bytes / 4;

            // ARM64 BL has a 26-bit signed immediate → range ±128 MiB.
            // Guard against modules that exceed this limit so we never silently
            // patch a BL with a wrong offset (which would produce a binary that
            // jumps to an arbitrary address).
            const BL_MAX: i64 =  (1i64 << 25) - 1; //  33_554_431 words ≈ +128 MiB
            const BL_MIN: i64 = -(1i64 << 25);      // -33_554_432 words ≈ -128 MiB
            if delta_words < BL_MIN || delta_words > BL_MAX {
                // The call target is >128 MiB away — this should never happen
                // for programs that fit in a single flat binary, but if it does
                // we surface it as a linker error rather than patching garbage.
                return Err(AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: BL relocation out of range for '{sym}' \
                         called from '{fn_name}' (delta={delta_words} words, \
                         allowed range [{BL_MIN}, {BL_MAX}]); \
                         module exceeds 128 MiB text size limit",
                        sym = reloc.symbol,
                    ),
                });
            }
            let imm26 = (delta_words as u32) & 0x03FFFFFF;
            let bl_word: u32 = 0x94000000 | imm26;

            // Patch the 4 bytes at call_byte_offset (little-endian).
            linked[call_byte_offset..call_byte_offset + 4]
                .copy_from_slice(&bl_word.to_le_bytes());
        }
    }

    Ok((linked, offsets, n_global_slots, global_byte_relocs, extern_branch_relocs))
}

/// Compile one `IIRFunction` to ARM64 machine code, returning the bytes,
/// any cross-function call relocations, and global-access relocations.
/// Returns `None` if the function contains opcodes the backend doesn't support.
fn compile_one_with_globals(
    fn_: &IIRFunction,
    global_slots: &HashMap<String, usize>,
) -> Option<(Vec<u8>, Vec<Reloc>, Vec<GlobalWordReloc>)> {
    let inferred = infer_types(fn_);
    let cir = aot_specialise(fn_, Some(&inferred));
    let ctx = FunctionContext {
        name:        &fn_.name,
        params:      &fn_.params,
        return_type: &fn_.return_type,
    };
    compile_with_globals(&ctx, &cir, global_slots).ok()
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
    fn global_define_compiles_ok() {
        // `(define x 5)` is a top-level value define — the ir compiler emits a
        // `global_set` call_builtin.  LANG39 (lower_global_io + ARM64 global
        // load/store + pack_object_with_globals) now handles this end-to-end,
        // so the program must compile to a valid Mach-O object file.
        let src = "(define x 5) x";
        let result = compile_macos_arm64_object(src, "globals_test");
        assert!(result.is_ok(), "global define should compile with LANG39; got: {:?}", result.err());

        // The output must be a Mach-O MH_OBJECT (filetype == 1).
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O magic");
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }

    #[test]
    fn fib_compiles_ok() {
        // End-to-end: the classic fibonacci program must compile to an ARM64
        // Mach-O object file without error.  This exercises the full AOT
        // preparation pipeline (builtin pre-lowering + type normalisation +
        // propagation) and the two-pass linker (cross-function BL patching
        // for the recursive fib → fib calls).
        let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";
        let result = compile_macos_arm64_object(src, "fib");
        assert!(result.is_ok(), "fib should compile; got: {:?}", result.err());
    }

    // ---- LANG41: io_out / __twig_print_i64 via runtime library ----

    #[test]
    fn print_program_compiles_ok() {
        // A Twig program that calls `io_out` must compile end-to-end to a valid
        // Mach-O MH_OBJECT without error.  The `print` builtin lowers to
        // `io_out` in the IIR compiler, which becomes `BL __twig_print_i64` in
        // the ARM64 backend.
        //
        // Under LANG41, `__twig_print_i64` is NOT injected as a synthetic
        // function; instead, the object file contains an N_UNDF | N_EXT
        // symbol-table entry and an ARM64_RELOC_BRANCH26 record so the system
        // linker (`ld`) can resolve the symbol from the embedded runtime archive
        // (`libtwig_aot_runtime.a`).
        //
        // NOTE: (print "hello") may fail type-checking; use (print 42) which
        // produces a typed i64 value.
        let src = "(print 42)";
        let result = compile_macos_arm64_object(src, "print_test");
        assert!(
            result.is_ok(),
            "io_out program should compile with LANG41; got: {:?}",
            result.err()
        );

        // Verify Mach-O magic and file type.
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O magic");
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1, "MH_OBJECT");
    }

    #[test]
    fn print_program_is_valid_macho() {
        // The compiled object should be a reasonable size — header (312 bytes)
        // plus user code, symbol table, and reloc records.  Under LANG41 the
        // 208-byte injected helper is gone; the object is smaller but still
        // well above 200 bytes.
        //
        // This test catches silent compilation failures (e.g. the packager
        // returning an empty buffer) rather than verifying exact byte counts.
        let src = "(print 99)";
        let result = compile_macos_arm64_object(src, "print_size_test");
        if let Ok(bytes) = result {
            // Minimum: 312 (header) + ~60 (user fn) + 8 (1 reloc) + 48 (3 syms)
            //        + 39 (strtab) ≈ 467 bytes.  Use 300 as the floor.
            assert!(
                bytes.len() >= 300,
                "Mach-O should be ≥ 300 bytes (LANG41: no helper injection); \
                 got {} bytes",
                bytes.len()
            );
        }
    }

    #[test]
    fn empty_main_compiles_to_object_bytes() {
        use interpreter_ir::function::IIRFunction;
        use interpreter_ir::instr::{IIRInstr, Operand};

        let main = IIRFunction::new(
            "main", vec![], "i64",
            vec![
                IIRInstr::new("const", Some("v0".into()),
                              vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None,
                              vec![Operand::Var("v0".into())], "i64"),
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

    // ── LANG42: refinement checker wired into twig-aot ────────────────────
    //
    // The Twig IR compiler already parses `(param : (Int lo hi))` syntax and
    // wires the annotation into `IIRFunction::param_refinements`.  LANG42 adds
    // the pass that *checks* those annotations before codegen.
    //
    // These tests use the source-string path (`compile_macos_arm64_object`) so
    // we don't need to add `lang-refined-types` to twig-aot's dev-dependencies.

    #[test]
    fn refinement_violation_becomes_aot_error() {
        // The worked example from the LANG42 spec (top-level expression form,
        // same pattern as `fib_compiles_ok` which avoids BackendRefused):
        //
        //   (define (ascii-info (codepoint : (Int 0 128))) codepoint)
        //   (ascii-info 200)        ← 200 violates [0, 128)
        //
        // The refinement pass fires BEFORE prepare_module_for_aot so we catch
        // the violation even though the module might fail codegen later.
        //
        // 200 violates (Int 0 128) → ProvenUnsafe → AotError::RefinementViolations.
        let src = "(define (ascii-info (codepoint : (Int 0 128))) codepoint) \
                   (ascii-info 200)";
        let result = compile_macos_arm64_object(src, "refinement_test");
        assert!(
            matches!(result, Err(AotError::RefinementViolations(_))),
            "expected RefinementViolations, got: {:?}",
            result.as_ref().err(),
        );
        if let Err(AotError::RefinementViolations(errs)) = result {
            assert!(!errs.is_empty(), "at least one error expected");
            assert_eq!(errs[0].counter_example, 200, "counter-example should be 200");
        }
    }

    #[test]
    fn safe_annotated_program_compiles_ok() {
        // (ascii-info 42) — 42 ∈ [0, 128) → ProvenSafe → no refinement error.
        // Same top-level expression form as fib_compiles_ok.
        let src = "(define (ascii-info (codepoint : (Int 0 128))) codepoint) \
                   (ascii-info 42)";
        let result = compile_macos_arm64_object(src, "refinement_safe");
        assert!(
            result.is_ok(),
            "safe annotated program should compile; got: {:?}",
            result.err(),
        );
    }
}
