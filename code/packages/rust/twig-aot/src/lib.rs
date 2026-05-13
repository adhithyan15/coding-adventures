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

use aarch64_backend::{compile_with_relocs, Reloc};
use aot_core::infer::infer_types;
use aot_core::link::{entry_point_offset, link};
use aot_core::specialise::aot_specialise;
use code_packager::macho_object::pack_object;
use code_packager::{CodeArtifact, Target};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
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
//    `"u64"` — Twig's tagged-integer representation fits in a 64-bit register
//    and all arithmetic on function arguments is 64-bit.  `infer_types` (from
//    `aot-core`) seeds its environment from `func.params`, so this is the
//    right place to override the "any" sentinel.
//
//  Step 3 — Propagate and default types.
//    A lightweight fixed-point pass populates `type_hint` on every instruction
//    whose type can be determined from its operands and the seeded param types.
//    Any remaining `"any"` hints on arithmetic and `mov` instructions are
//    then defaulted to `"u64"` so that `aot_specialise` never sees an
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

/// Step 2: promote `"any"` / `"polymorphic"` param types to `"u64"`.
fn normalize_params_to_u64(func: &mut IIRFunction) {
    for (_, ty) in &mut func.params {
        if ty == "any" || ty == "polymorphic" {
            *ty = "u64".to_string();
        }
    }
}

/// Step 3a: fixed-point type propagation seeded from params.
///
/// Applies inference rules:
/// - `const` with `Int` → `"u64"`, `Bool` → `"bool"`
/// - `cmp_*` → `"bool"` (comparisons always produce a boolean)
/// - `add`, `sub`, `mul`, `div`, `mov`, `neg`, `not` → type of first operand
///
/// Runs in a loop until stable.  Function params (already "u64") seed the
/// environment together with already-typed instructions.
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
            Some(Operand::Int(_))  => Some("u64".into()),
            Some(Operand::Bool(_)) => Some("bool".into()),
            _                       => None,
        },
        // Comparisons always yield bool, regardless of operand types.
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => Some("bool".into()),
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
        Operand::Int(_)  => Some("u64".into()),
        Operand::Bool(_) => Some("bool".into()),
        _                => None,
    }
}

/// Step 3b: default any remaining `"any"` hints on arithmetic / move
/// instructions to `"u64"`.
///
/// This handles instructions whose sources are still `"any"` after the
/// propagation pass — most commonly the results of `call` instructions whose
/// return type is not tracked.  Defaulting to `"u64"` is safe for Twig:
/// integer values fit in 64 bits and all arithmetic operates on them uniformly.
/// The ARM64 backend generates correct code regardless of signed/unsigned
/// flavour for the non-negative values produced by programs like fibonacci.
fn default_any_to_u64(func: &mut IIRFunction) {
    for instr in &mut func.instructions {
        if instr.type_hint != "any" { continue; }
        match instr.op.as_str() {
            "add" | "sub" | "mul" | "div" | "mod" | "mov" | "neg" | "not" => {
                instr.type_hint = "u64".to_string();
            }
            _ => {}
        }
    }
}

/// Apply all three preparation steps to every function in `module`.
fn prepare_module_for_aot(module: &mut IIRModule) {
    for func in &mut module.functions {
        pre_lower_aot_builtins(func);
        normalize_params_to_u64(func);
        propagate_aot_types(func);
        default_any_to_u64(func);
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Run the per-function AOT pipeline and link into a single text section.
///
/// Two-pass strategy for cross-function calls:
///
/// Pass 1 — compile each function independently.  Cross-function `call`
///   instructions emit `BL #0` placeholder instructions and record
///   [`Reloc`] entries (callee name + word index in the per-function binary).
///
/// Link — concatenate all function binaries into one flat code section and
///   record each function's byte offset.
///
/// Pass 2 — patch every placeholder `BL` with the correct PC-relative
///   offset using the now-known function offsets.
///
/// ARM64 `BL` encoding: opcode `0x94000000`, 26-bit signed PC-relative
/// offset in units of 4 bytes (instruction words).
fn compile_module_to_text(
    module: &IIRModule,
) -> Result<(Vec<u8>, HashMap<String, usize>), AotError> {
    // ── Preparation: lower builtins, normalise types, default "any" → u64 ──
    //
    // We work on a clone so the caller's `IIRModule` is never mutated.
    // `prepare_module_for_aot` runs three passes:
    //   1. `call_builtin "+"` → `add`, etc.   (see `pre_lower_aot_builtins`)
    //   2. param types "any" → "u64"           (see `normalize_params_to_u64`)
    //   3. propagate + default remaining "any" (see `propagate_aot_types` /
    //                                            `default_any_to_u64`)
    let mut module = module.clone();
    prepare_module_for_aot(&mut module);

    // ── Pass 1: compile all functions, collecting cross-function relocations ──
    // Each entry: (fn_name, per-function bytes, list of ExternalReloc for that fn)
    let mut fn_results: Vec<(String, Vec<u8>, Vec<Reloc>)> =
        Vec::with_capacity(module.functions.len());

    for fn_ in &module.functions {
        let (bytes, relocs) = compile_one_with_relocs(fn_)
            .ok_or_else(|| AotError::BackendRefused { function: fn_.name.clone() })?;
        fn_results.push((fn_.name.clone(), bytes, relocs));
    }

    // ── Link: concatenate binaries and record byte offsets ──────────────────
    let plain_binaries: Vec<(String, Vec<u8>)> = fn_results
        .iter()
        .map(|(name, bytes, _)| (name.clone(), bytes.clone()))
        .collect();
    let (mut linked, offsets) = link(&plain_binaries);

    // ── Pass 2: patch cross-function BL placeholders ──────────────────────
    for (fn_name, _bytes, relocs) in &fn_results {
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
                // Callee not in this module — unresolved external symbol.
                // Return an explicit error rather than leaving a `BL #0`
                // placeholder (which would silently produce a self-recursive
                // call or a misaligned trap in the output binary).
                return Err(AotError::Linker {
                    status: None,
                    stderr: format!(
                        "twig-aot: unresolved external symbol '{}' called from '{}'; \
                         only within-module calls are supported in AOT mode",
                        reloc.symbol, fn_name
                    ),
                });
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

    Ok((linked, offsets))
}

/// Compile one `IIRFunction` to ARM64 machine code, returning the bytes and
/// any cross-function call relocations.  Returns `None` if the function
/// contains opcodes the backend doesn't support.
fn compile_one_with_relocs(fn_: &IIRFunction) -> Option<(Vec<u8>, Vec<Reloc>)> {
    let inferred = infer_types(fn_);
    let cir = aot_specialise(fn_, Some(&inferred));
    let ctx = FunctionContext {
        name:        &fn_.name,
        params:      &fn_.params,
        return_type: &fn_.return_type,
    };
    compile_with_relocs(&ctx, &cir).ok()
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
