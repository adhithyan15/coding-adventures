//! # `lang-aot` — multi-language AOT driver for the LANG VM chain.
//!
//! Compiles source code in several languages to native executables by
//! routing each language's frontend through the **same** AOT pipeline
//! the `twig-aot` crate ships for Twig:
//!
//! ```text
//! <lang> source
//!     │
//!     ▼ <lang>-iir-compiler
//! interpreter_ir::IIRModule        ← lingua franca
//!     │
//!     ▼ twig_aot::compile_module_to_{linux,windows,macos}_executable
//! native executable
//! ```
//!
//! ## Supported languages today
//!
//! | Language | Status | Frontend crate |
//! |---|---|---|
//! | Twig            | full | `twig-ir-compiler` |
//! | Nib             | full | `nib-iir-compiler` |
//! | Brainfuck       | full | `brainfuck-iir-compiler` |
//! | Dartmouth BASIC | full (integer subset) | `dartmouth-basic-iir-compiler` |
//! | Oct             | full (integer subset; 8008 intrinsics rejected) | `oct-iir-compiler` |
//!
//! ## How to add a language
//!
//! 1. Implement a new `*-iir-compiler` Rust crate whose `compile_source`
//!    returns `Result<interpreter_ir::IIRModule, _>`.  Mirror the shape
//!    of [`nib_iir_compiler::compile_source`] or
//!    [`brainfuck_iir_compiler::compile_source`].
//! 2. Add a variant to [`Language`] in this crate and wire it into
//!    [`compile_source_to_iir`].
//! 3. Add a file extension to [`detect_language_from_path`].
//! 4. Add a smoke test.
//!
//! No backend changes required — every frontend gets x86-64 Linux,
//! x86-64 Windows, and ARM64 macOS for free via the shared chain.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;
use std::path::Path;

use interpreter_ir::module::IIRModule;

/// Source language a `lang-aot` invocation is compiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Twig — original language the AOT pipeline was built for.
    Twig,
    /// Nib — typed expression language, multi-language implementation.
    Nib,
    /// Brainfuck — minimalist tape language.
    Brainfuck,
    /// Dartmouth BASIC — placeholder; no IIR-emitting frontend yet.
    DartmouthBasic,
    /// Oct — placeholder; no Rust frontend yet (Python only).
    Oct,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Twig => write!(f, "twig"),
            Language::Nib => write!(f, "nib"),
            Language::Brainfuck => write!(f, "brainfuck"),
            Language::DartmouthBasic => write!(f, "dartmouth-basic"),
            Language::Oct => write!(f, "oct"),
        }
    }
}

impl Language {
    /// Parse a `--lang` value or a short alias like `bf`.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "twig" => Ok(Self::Twig),
            "nib" => Ok(Self::Nib),
            "brainfuck" | "bf" => Ok(Self::Brainfuck),
            "dartmouth-basic" | "basic" | "bas" => Ok(Self::DartmouthBasic),
            "oct" => Ok(Self::Oct),
            other => Err(format!(
                "unknown language {other:?}; expected one of: twig, nib, \
                 brainfuck (or bf), dartmouth-basic (or basic / bas), oct")),
        }
    }
}

/// Detect a [`Language`] from a file extension.  Returns `None` if the
/// extension is missing or unrecognised — callers should fall back to
/// an explicit `--lang` flag in that case.
pub fn detect_language_from_path(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "twig" => Some(Language::Twig),
        "nib" => Some(Language::Nib),
        "bf" | "b" => Some(Language::Brainfuck),
        "bas" | "basic" => Some(Language::DartmouthBasic),
        "oct" => Some(Language::Oct),
        _ => None,
    }
}

/// Errors `lang-aot` surfaces to callers.
#[derive(Debug)]
pub enum LangAotError {
    /// The language doesn't have an IIR-emitting Rust frontend yet.
    UnsupportedLanguage {
        /// Which language was requested.
        language: Language,
        /// What the user/developer needs to do to unblock it.
        guidance: &'static str,
    },
    /// The frontend rejected the source (parse / type-check error).
    FrontendError {
        /// Which language's frontend failed.
        language: Language,
        /// Human-readable error from the frontend.
        message: String,
    },
    /// The shared AOT backend (twig-aot) rejected the IR.
    AotError(twig_aot::AotError),
    /// Filesystem I/O failure.
    Io(std::io::Error),
    /// The LLVM textual-IR backend rejected the IIR.
    ///
    /// Carries the human-readable string from `iir-to-llvm` (which already
    /// includes the failing function name and the unsupported op/type).
    LlvmBackendError(String),
}

impl fmt::Display for LangAotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LangAotError::UnsupportedLanguage { language, guidance } => write!(f,
                "lang-aot: no Rust IIR frontend for {language} yet — {guidance}"),
            LangAotError::FrontendError { language, message } => write!(f,
                "{language}: {message}"),
            LangAotError::AotError(e) => write!(f, "{e}"),
            LangAotError::Io(e) => write!(f, "io: {e}"),
            LangAotError::LlvmBackendError(m) => write!(f, "llvm: {m}"),
        }
    }
}

impl std::error::Error for LangAotError {}

impl From<twig_aot::AotError> for LangAotError {
    fn from(e: twig_aot::AotError) -> Self { LangAotError::AotError(e) }
}

impl From<std::io::Error> for LangAotError {
    fn from(e: std::io::Error) -> Self { LangAotError::Io(e) }
}

/// Compile source text to an [`IIRModule`] using the matching frontend.
///
/// `module_name` is used for diagnostics and as the IR module's
/// identifier; pick something descriptive (usually the input file's
/// stem).
///
/// All `Language` variants now have working Rust frontends — every
/// dispatch arm returns [`Ok`] on a well-formed program.  Errors
/// surface as [`LangAotError::FrontendError`] (parse / type / unsupported
/// construct).  `LangAotError::UnsupportedLanguage` is no longer
/// reachable from this function; it stays in the enum so adding a new
/// `Language` variant continues to be a single-arm change.
pub fn compile_source_to_iir(
    language: Language,
    source: &str,
    module_name: &str,
) -> Result<IIRModule, LangAotError> {
    match language {
        Language::Twig => {
            twig_ir_compiler::compile_source(source, module_name)
                .map_err(|e| LangAotError::FrontendError {
                    language,
                    message: format!("{e:?}"),
                })
        }
        Language::Nib => {
            nib_iir_compiler::compile_source(source, module_name)
                .map_err(|e| LangAotError::FrontendError {
                    language,
                    message: format!("{e:?}"),
                })
        }
        Language::Brainfuck => {
            let mut module = brainfuck_iir_compiler::compile_source(source, module_name)
                .map_err(|e| LangAotError::FrontendError {
                    language,
                    message: e,
                })?;
            lower_brainfuck_for_aot(&mut module);
            Ok(module)
        }
        Language::DartmouthBasic => {
            dartmouth_basic_iir_compiler::compile_source(source, module_name)
                .map_err(|e| LangAotError::FrontendError {
                    language,
                    message: format!("{e}"),
                })
        }
        Language::Oct => {
            oct_iir_compiler::compile_source(source, module_name)
                .map_err(|e| LangAotError::FrontendError {
                    language,
                    message: format!("{e}"),
                })
        }
    }
}

// ---------------------------------------------------------------------------
// End-to-end pipelines: source → IIR → executable
//
// Each function is cfg-gated to the host that can actually link for the
// target — same policy as twig-aot.  Cross-OS object emission goes
// through `compile_object_to_disk` instead.
// ---------------------------------------------------------------------------

/// Cross-platform: source → IIR → textual LLVM IR (`.ll`) on disk.
///
/// Unlike the native-executable pipelines below, this one does **not** link
/// or run the LLVM toolchain — it just writes a `.ll` file.  Downstream
/// `llc` / `opt` invocations are the caller's responsibility.
///
/// No `cfg(target_os = ...)` gating: emitting text is platform-agnostic.
/// Pair this with `--emit=llvm-ir` on the `lang-aot` CLI.
///
/// Errors:
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `LlvmBackendError` — the IIR contained an op or type the LLVM backend
///   does not yet handle (the error message names the function and op).
/// * `Io` — failed to read the input or write the output.
pub fn compile_file_to_llvm_ir(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // Use the stem as both the LLVM module-id and the default triple stays
    // at iir-to-llvm's deterministic `x86_64-unknown-linux-gnu` so test
    // output is reproducible across CI runners.
    let cfg = iir_to_llvm::IIRLlvmConfig::new(stem);
    let ll = iir_to_llvm::lower_iir_to_llvm(&module, &cfg)
        .map_err(|e| LangAotError::LlvmBackendError(format!("{e}")))?;

    std::fs::write(out, ll)?;
    Ok(())
}

/// Linux x86-64: source → IIR → ELF → executable (Linux host only).
#[cfg(target_os = "linux")]
pub fn compile_file_to_linux_executable(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;
    twig_aot::compile_module_to_linux_executable(&module, out)?;
    Ok(())
}

/// Windows x86-64: source → IIR → PE → executable (Windows host only).
#[cfg(target_os = "windows")]
pub fn compile_file_to_windows_executable(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;
    twig_aot::compile_module_to_windows_executable(&module, out)?;
    Ok(())
}

/// macOS ARM64: source → IIR → Mach-O → executable (Unix host only).
#[cfg(unix)]
pub fn compile_file_to_macos_executable(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;
    twig_aot::compile_module_to_macos_executable(&module, out)?;
    Ok(())
}

// ===========================================================================
// BF07 — Brainfuck → LANG76 lowering pass
// ===========================================================================
//
// The `brainfuck-iir-compiler` crate emits IIR that targets a hypothetical
// VM with an implicit byte tape: `load_mem v, ptr` reads `*ptr`,
// `store_mem ptr, v` writes `*ptr = v`, and `ptr` is just a cell index
// (0..30000).  That shape works for `vm-core`, `jit-core`, and
// `iir-to-wasm` — they each materialise the tape themselves.
//
// The AOT chain has no implicit tape: the backends only know about
// `alloc_bytes` + `load_byte` + `store_byte` (LANG76).  This pass
// rewrites a Brainfuck-shaped `IIRModule` into a LANG76-shaped one
// without touching the frontend (so existing consumers keep working):
//
// 1. Prepend `const __bf_tape_size = 30000` and `alloc_bytes
//    __bf_tape_size -> __bf_tape` to `main`.
// 2. Rewrite `load_mem v, ptr` (one src) → `load_byte __bf_tape, ptr -> v`.
// 3. Rewrite `store_mem ptr, v` (two srcs) → `store_byte __bf_tape, ptr, v`.
// 4. Change `main`'s return type from `void` to `i64` and replace the
//    trailing `ret_void` with `const r = 0; ret r` — Brainfuck has no
//    exit code so we always return 0 from main, which the LANG VM AOT
//    chain requires (the entry-point's return value is the process
//    exit code).

fn lower_brainfuck_for_aot(module: &mut IIRModule) {
    use interpreter_ir::instr::{IIRInstr, Operand};

    const TAPE: &str = "__bf_tape";
    const TAPE_SIZE_VAR: &str = "__bf_tape_size";

    for func in &mut module.functions {
        if func.name != "main" {
            // Defensive: BF compiler today only emits a single `main`,
            // but the pass is correct regardless.  Functions other than
            // `main` are passed through untouched.
            continue;
        }

        // Step 1 — preamble: const TAPE_SIZE = 30000; alloc_bytes -> TAPE.
        let mut new_instrs = Vec::with_capacity(func.instructions.len() + 2);
        new_instrs.push(IIRInstr::new(
            "const",
            Some(TAPE_SIZE_VAR.to_string()),
            vec![Operand::Int(30_000)],
            "i64",
        ));
        new_instrs.push(IIRInstr::new(
            "alloc_bytes",
            Some(TAPE.to_string()),
            vec![Operand::Var(TAPE_SIZE_VAR.to_string())],
            "i64",
        ));

        // Step 2 & 3 — rewrite load_mem / store_mem.
        for instr in std::mem::take(&mut func.instructions) {
            match instr.op.as_str() {
                // load_mem  v <- ptr   ⇒   load_byte v <- TAPE, ptr
                "load_mem" => {
                    let mut srcs = Vec::with_capacity(2);
                    srcs.push(Operand::Var(TAPE.to_string()));
                    if let Some(ptr) = instr.srcs.into_iter().next() {
                        srcs.push(ptr);
                    }
                    new_instrs.push(IIRInstr::new(
                        "load_byte",
                        instr.dest,
                        srcs,
                        instr.type_hint,
                    ));
                }
                // store_mem ptr, v   ⇒   store_byte TAPE, ptr, v
                "store_mem" => {
                    let mut srcs = Vec::with_capacity(3);
                    srcs.push(Operand::Var(TAPE.to_string()));
                    srcs.extend(instr.srcs.into_iter());
                    new_instrs.push(IIRInstr::new(
                        "store_byte",
                        None,
                        srcs,
                        instr.type_hint,
                    ));
                }
                // Step 4 — replace `ret_void` with `const r=0; ret r`.
                //
                // We synthesise a fresh register name (`__bf_ret`) to
                // avoid colliding with the BF compiler's fixed names
                // `ptr` / `v` / `c` / `k`.
                "ret_void" => {
                    new_instrs.push(IIRInstr::new(
                        "const",
                        Some("__bf_ret".to_string()),
                        vec![Operand::Int(0)],
                        "i64",
                    ));
                    new_instrs.push(IIRInstr::new(
                        "ret",
                        None,
                        vec![Operand::Var("__bf_ret".to_string())],
                        "i64",
                    ));
                }
                _ => new_instrs.push(instr),
            }
        }
        func.instructions = new_instrs;
        // Reflect the step-4 return-type change so downstream type
        // propagation in twig-aot sees i64 instead of void.
        func.return_type = "i64".to_string();
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_language_aliases() {
        assert_eq!(Language::parse("twig").unwrap(), Language::Twig);
        assert_eq!(Language::parse("nib").unwrap(), Language::Nib);
        assert_eq!(Language::parse("brainfuck").unwrap(), Language::Brainfuck);
        assert_eq!(Language::parse("bf").unwrap(), Language::Brainfuck);
        assert_eq!(Language::parse("basic").unwrap(), Language::DartmouthBasic);
        assert_eq!(Language::parse("oct").unwrap(), Language::Oct);
        assert!(Language::parse("bogus").is_err());
    }

    #[test]
    fn detect_language_from_extension() {
        let p = |s: &str| std::path::PathBuf::from(s);
        assert_eq!(detect_language_from_path(&p("foo.twig")), Some(Language::Twig));
        assert_eq!(detect_language_from_path(&p("foo.nib")), Some(Language::Nib));
        assert_eq!(detect_language_from_path(&p("foo.bf")),
                   Some(Language::Brainfuck));
        assert_eq!(detect_language_from_path(&p("foo.bas")),
                   Some(Language::DartmouthBasic));
        assert_eq!(detect_language_from_path(&p("foo.oct")), Some(Language::Oct));
        assert_eq!(detect_language_from_path(&p("foo.txt")), None);
        assert_eq!(detect_language_from_path(&p("README")), None);
    }

    #[test]
    fn brainfuck_compiles_to_iir() {
        // Smallest meaningful BF program — increment cell and output ASCII.
        // We don't check the exact IIR shape (that's brainfuck-iir-compiler's
        // job); just that `lang-aot` routes the call correctly and we get
        // back a module.
        let iir = compile_source_to_iir(
            Language::Brainfuck, "++++++++++++++++++++++++++++++++.", "bf"
        ).expect("brainfuck must compile");
        assert!(!iir.functions.is_empty(), "BF module must have at least main");
    }

    /// BF07: verify the Brainfuck lowering pass rewrites the module
    /// shape correctly — `load_mem` / `store_mem` become `load_byte` /
    /// `store_byte`, the `alloc_bytes` preamble is prepended, and the
    /// implicit `ret_void` becomes an `i64` return of 0.
    #[test]
    fn brainfuck_lowering_inserts_tape_and_byte_ops() {
        let iir = compile_source_to_iir(
            Language::Brainfuck, "+.", "bf"
        ).expect("brainfuck must compile");
        let main = iir.functions.iter().find(|f| f.name == "main")
            .expect("BF main must exist");

        // Step 1: first two instructions must be the tape preamble.
        let ops: Vec<&str> = main.instructions.iter()
            .map(|i| i.op.as_str()).collect();
        assert_eq!(ops[0], "const",
                   "first instr must be const for tape size; got {ops:?}");
        assert_eq!(ops[1], "alloc_bytes",
                   "second instr must be alloc_bytes; got {ops:?}");

        // Step 2/3: no `load_mem` or `store_mem` should remain.
        for op in &ops {
            assert_ne!(*op, "load_mem", "load_mem leaked through lowering");
            assert_ne!(*op, "store_mem", "store_mem leaked through lowering");
        }

        // At least one load_byte and one store_byte must be present
        // (the `+` command writes back, the `.` command loads).
        assert!(ops.contains(&"load_byte"),
                "lowered module must contain load_byte; got {ops:?}");
        assert!(ops.contains(&"store_byte"),
                "lowered module must contain store_byte; got {ops:?}");

        // Step 4: ret_void must be gone, replaced by `const __bf_ret = 0; ret`.
        assert!(!ops.iter().any(|o| *o == "ret_void"),
                "ret_void must be replaced by ret i64 0");
        assert_eq!(main.return_type, "i64",
                   "main return type must be i64 after lowering");
        // Last two instrs are `const __bf_ret = 0; ret __bf_ret`.
        let last_two = &ops[ops.len()-2..];
        assert_eq!(last_two, &["const", "ret"],
                   "epilogue must be `const; ret`; got {last_two:?}");
    }

    #[test]
    fn twig_compiles_to_iir() {
        let iir = compile_source_to_iir(Language::Twig, "42", "twig")
            .expect("Twig must compile");
        assert!(!iir.functions.is_empty());
    }

    /// PL05 — Dartmouth BASIC now compiles to IIR.  Previously this
    /// returned `UnsupportedLanguage`; with the new
    /// `dartmouth-basic-iir-compiler` crate we get a real module back.
    #[test]
    fn dartmouth_basic_compiles_to_iir() {
        let iir = compile_source_to_iir(
            Language::DartmouthBasic, "10 PRINT 42\n20 END\n", "basic"
        ).expect("dartmouth-basic must compile");
        assert!(!iir.functions.is_empty(),
                "BASIC module must have at least main");
        assert_eq!(iir.functions[0].name, "main");
    }

    /// OCT02 phase 4 — Oct now compiles to IIR (was UnsupportedLanguage).
    #[test]
    fn oct_compiles_to_iir() {
        let iir = compile_source_to_iir(
            Language::Oct, "fn main() { let x: u8 = 42; }", "oct"
        ).expect("oct should compile");
        assert!(!iir.functions.is_empty(),
                "Oct module must have at least main");
        assert_eq!(iir.functions[0].name, "main");
    }

    /// 8008 intrinsics still produce a clean error (Unsupported8008Intrinsic
    /// propagates through `FrontendError`, not `UnsupportedLanguage`).
    #[test]
    fn oct_8008_intrinsic_reports_frontend_error() {
        let err = compile_source_to_iir(
            Language::Oct, "fn main() { let x: u8 = in(1); }", "oct"
        ).unwrap_err();
        match err {
            LangAotError::FrontendError { language, message } => {
                assert_eq!(language, Language::Oct);
                assert!(message.contains("8008") || message.contains("intrinsic"),
                        "expected 8008 intrinsic mention; got: {message}");
            }
            other => panic!("expected FrontendError, got {other:?}"),
        }
    }
}
