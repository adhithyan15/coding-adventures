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
    /// McCarthy Lisp — the 1960 Lisp 1.0, compiled via
    /// `mccarthy-lisp-iir-compiler` over the `lispy-runtime` value model.
    McCarthyLisp,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Twig => write!(f, "twig"),
            Language::Nib => write!(f, "nib"),
            Language::Brainfuck => write!(f, "brainfuck"),
            Language::DartmouthBasic => write!(f, "dartmouth-basic"),
            Language::Oct => write!(f, "oct"),
            Language::McCarthyLisp => write!(f, "mccarthy-lisp"),
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
            "mccarthy-lisp" | "mccarthy" | "mcl" | "lisp" => Ok(Self::McCarthyLisp),
            other => Err(format!(
                "unknown language {other:?}; expected one of: twig, nib, \
                 brainfuck (or bf), dartmouth-basic (or basic / bas), oct, \
                 mccarthy-lisp (or mccarthy / mcl / lisp)")),
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
        "mcl" | "lisp" => Some(Language::McCarthyLisp),
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
    /// The RV32I backend rejected the IIR.
    ///
    /// Carries the human-readable string from `iir-to-riscv` (which
    /// already includes the failing function name and the unsupported
    /// op/type/operand).
    RiscvBackendError(String),
    /// The Intel 8008 backend rejected the IIR.
    ///
    /// Carries the human-readable string from `iir-to-intel8008`
    /// (which already includes the failing function name and the
    /// unsupported op/type/operand).
    Intel8008BackendError(String),
    /// The ARMv7 (A32) backend rejected the IIR.
    ///
    /// Carries the human-readable string from `iir-to-armv7` (which
    /// already includes the failing function name and the
    /// unsupported op/type/operand).
    Armv7BackendError(String),
    /// The Intel 4004 backend rejected the IIR.
    ///
    /// Carries the human-readable string from `iir-to-intel4004`
    /// (which already includes the failing function name and the
    /// unsupported op/type/operand).
    Intel4004BackendError(String),
    /// The GE-225 backend rejected the IIR.
    ///
    /// Carries the human-readable string from `iir-to-ge225` (which
    /// already includes the failing function name and the
    /// unsupported op/type/operand).  The GE-225 (1959) was the
    /// mainframe where Dartmouth BASIC was designed in 1964.
    Ge225BackendError(String),
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
            LangAotError::RiscvBackendError(m) => write!(f, "riscv32: {m}"),
            LangAotError::Intel8008BackendError(m) => write!(f, "intel8008: {m}"),
            LangAotError::Armv7BackendError(m) => write!(f, "armv7: {m}"),
            LangAotError::Intel4004BackendError(m) => write!(f, "intel4004: {m}"),
            LangAotError::Ge225BackendError(m) => write!(f, "ge225: {m}"),
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
        Language::McCarthyLisp => {
            mccarthy_lisp_iir_compiler::compile_source(source, module_name)
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

/// Cross-platform: source → IIR → RV32I machine code (`.bin`) on disk.
///
/// Unlike the native-executable pipelines, this one does **not** link or
/// run any toolchain — it just writes a flat `.bin` of little-endian
/// 32-bit RV32I instruction words.  Downstream consumers:
///
/// * [`riscv-simulator`](../riscv-simulator) — load + execute in-process.
/// * `qemu-riscv32` — `qemu-riscv32 -kernel out.bin`.
/// * A physical flash loader on a SiFive / ESP32-C3 / etc. board.
///
/// No `cfg(target_os = ...)` gating: emitting bytes is platform-agnostic.
///
/// # Wire format
///
/// Each emitted word is written as **little-endian** bytes per the RISC-V
/// spec (Volume I §1.4): bit `[7:0]` of the word goes to the lowest-
/// address byte.  `Vec<u32>::iter().flat_map(u32::to_le_bytes)` is the
/// canonical Rust expression of that encoding.
///
/// # Errors
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `RiscvBackendError` — the IIR contained an op or type the RV32I
///   backend does not yet handle (the message names the function and
///   op).
/// * `Io` — failed to read the input or write the output.
pub fn compile_file_to_riscv32_bin(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // Phase 7 (FINAL lane) of the historical-arch backend migration:
    // route through aot_core::infer + aot_core::specialise +
    // riscv_backend::compile per function, same pattern as Phases 3-6
    // for GE-225 / Intel 4004 / ARMv7 / Intel 8008.  riscv-backend
    // emits little-endian-flattened bytes directly, so concatenation
    // here is just `extend_from_slice`.
    let _ = stem;
    let mut bytes = Vec::new();
    let empty_params: Vec<(String, String)> = Vec::new();
    for f in &module.functions {
        let inferred = aot_core::infer::infer_types(f);
        let cir = aot_core::specialise::aot_specialise(f, Some(&inferred));
        let ctx = jit_core::backend::FunctionContext {
            name: f.name.as_str(),
            params: &empty_params,
            return_type: f.return_type.as_str(),
        };
        let fn_bytes = riscv_backend::compile(&ctx, &cir)
            .map_err(|e| LangAotError::RiscvBackendError(format!("{e}")))?;
        bytes.extend_from_slice(&fn_bytes);
    }
    if bytes.is_empty() {
        // Fallback: an empty module still needs at least the
        // canonical `ret` so consumers (qemu-riscv32, simulator)
        // see a well-formed `.bin`.
        bytes.extend_from_slice(&riscv_encoder::RET_WORD.to_le_bytes());
    }

    std::fs::write(out, &bytes)?;
    Ok(())
}

/// Cross-platform: source → IIR → Intel 8008 machine code (`.bin`) on disk.
///
/// Unlike the native-executable pipelines, this one does **not** link
/// or run any toolchain — it just writes a flat `.bin` of 8-bit Intel
/// 8008 opcode bytes.  Downstream consumers:
///
/// * [`intel8008-simulator`](../intel8008-simulator) — load + execute
///   in-process via `Simulator::run`.
/// * An external 8008 emulator that consumes raw byte streams.
/// * A 1702 EPROM burner — Oct's intended deployment path.  The 8008
///   is Oct's native target ISA.
///
/// No `cfg(target_os = ...)` gating: emitting bytes is platform-agnostic.
///
/// # Wire format
///
/// Intel 8008 instructions are 1, 2, or 3 bytes each, in the order
/// the silicon's instruction pointer walks them.  No endianness
/// conversion — each byte is written exactly as `iir-to-intel8008`
/// emits it.  Multi-byte instructions (MVI, JMP, CAL, conditional
/// branches) lay out as `<opcode> <low_byte> [<high_byte>]` per the
/// 8008 spec (the address bus is 14 bits wide, so the high byte's
/// top 2 bits are zero).
///
/// # Why no host gating?
///
/// The 8008 is a historical ISA with no modern host equivalent — we
/// never produce a "native" binary the host can execute.  The
/// downstream consumer is always an emulator (in-tree
/// `intel8008-simulator` or external), an EPROM burner, or actual
/// 8008 silicon.  All host OSes can write a flat byte file, so the
/// pipeline is universally available.
///
/// # Errors
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `Intel8008BackendError` — the IIR contained an op or type the
///   Intel 8008 backend does not yet handle (the message names the
///   function and op).
/// * `Io` — failed to read the input or write the output.
///
/// # Example downstream invocation
///
/// ```bash
/// lang-aot foo.oct --emit=intel8008 -o foo.bin
/// # Then load foo.bin into the simulator:
/// intel8008-simulator foo.bin
/// ```
pub fn compile_file_to_intel8008_bin(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // Phase 6 of the historical-arch backend migration: route
    // through aot_core::infer + aot_core::specialise +
    // intel8008_backend::compile per function, same pattern as
    // Phases 3-5.
    let _ = stem;
    let mut bytes = Vec::new();
    let empty_params: Vec<(String, String)> = Vec::new();
    for f in &module.functions {
        let inferred = aot_core::infer::infer_types(f);
        let cir = aot_core::specialise::aot_specialise(f, Some(&inferred));
        let ctx = jit_core::backend::FunctionContext {
            name: f.name.as_str(),
            params: &empty_params,
            return_type: f.return_type.as_str(),
        };
        let fn_bytes = intel8008_backend::compile(&ctx, &cir)
            .map_err(|e| LangAotError::Intel8008BackendError(format!("{e}")))?;
        bytes.extend_from_slice(&fn_bytes);
    }
    if bytes.is_empty() {
        bytes.push(intel8008_encoder::HLT);
    }

    std::fs::write(out, &bytes)?;
    Ok(())
}

/// Cross-platform: source → IIR → ARMv7 (A32) machine code (`.bin`) on disk.
///
/// Unlike the native-executable pipelines, this one does **not** link
/// or run any toolchain — it just writes a flat `.bin` of 32-bit
/// ARMv7-A instruction words encoded as little-endian bytes.
/// Downstream consumers:
///
/// * [`arm-simulator`](../arm-simulator) — load + execute in-process.
/// * `qemu-arm` — `qemu-arm -kernel out.bin`.
/// * `objcopy` + a phone-class Linux linker for an ELF executable on
///   a Cortex-A7/A8/A9-era SoC.
///
/// No `cfg(target_os = ...)` gating: emitting bytes is platform-
/// agnostic.
///
/// # Wire format
///
/// Each emitted A32 word is written as **little-endian** bytes.
/// ARMv7 is configurable-endian but defaults to little-endian on
/// every modern Linux / Android distribution, on QEMU, and on the
/// in-tree `arm-simulator`.  `Vec<u32>::iter().flat_map(u32::to_le_bytes)`
/// is the canonical Rust expression of that encoding.
///
/// # Why no host gating?
///
/// ARMv7 host execution would require an ARM Cortex-A class CPU.
/// Most LANG VM developers don't have one as their dev host, so we
/// never produce a "native" binary the host can execute — the
/// downstream consumer is always a simulator (in-tree
/// `arm-simulator`, `qemu-arm`), a phone-class Linux board, or a
/// flash loader.  All host OSes can write a flat byte file, so the
/// pipeline is universally available.
///
/// # Errors
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `Armv7BackendError` — the IIR contained an op or type the ARMv7
///   backend does not yet handle (the message names the function and
///   op).
/// * `Io` — failed to read the input or write the output.
///
/// # Example downstream invocation
///
/// ```bash
/// lang-aot foo.twig --emit=armv7 -o foo.bin
/// # Then load foo.bin into the simulator or QEMU:
/// qemu-arm -kernel foo.bin
/// ```
pub fn compile_file_to_armv7_bin(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // Phase 5 of the historical-arch backend migration: route
    // through `aot_core` + `armv7-backend` (which itself returns
    // little-endian-flattened bytes), same pattern as Phases 3+4
    // for GE-225 and Intel 4004.
    let _ = stem;
    let mut bytes = Vec::new();
    let empty_params: Vec<(String, String)> = Vec::new();
    for f in &module.functions {
        let inferred = aot_core::infer::infer_types(f);
        let cir = aot_core::specialise::aot_specialise(f, Some(&inferred));
        let ctx = jit_core::backend::FunctionContext {
            name: f.name.as_str(),
            params: &empty_params,
            return_type: f.return_type.as_str(),
        };
        let fn_bytes = armv7_backend::compile(&ctx, &cir)
            .map_err(|e| LangAotError::Armv7BackendError(format!("{e}")))?;
        bytes.extend_from_slice(&fn_bytes);
    }

    std::fs::write(out, &bytes)?;
    Ok(())
}

/// Cross-platform: source → IIR → Intel 4004 machine code (`.bin`) on disk.
///
/// Unlike the native-executable pipelines, this one does **not** link
/// or run any toolchain — it just writes a flat `.bin` of 1- or
/// 2-byte Intel 4004 opcodes.  Downstream consumers:
///
/// * Any 4004 simulator (in-tree, MAME, custom emulator).
/// * The in-tree `intel-4004-assembler` for round-trip
///   disassembly.
/// * An EPROM burner for a 4004 dev board (the 4004 was paired
///   with 1702 EPROMs).
///
/// No `cfg(target_os = ...)` gating: emitting bytes is platform-
/// agnostic.
///
/// # Wire format
///
/// 4004 instructions are 1 or 2 bytes each, in the order the
/// silicon's program counter walks them.  No endianness conversion
/// needed — every byte is written exactly as `iir-to-intel4004`
/// emits it.  This is the same byte-aligned format as
/// `iir-to-intel8008`'s output.
///
/// # Why no host gating?
///
/// The 4004 is a 1971-era 4-bit microprocessor with no modern
/// host equivalent.  Downstream is always a simulator, an
/// assembler round-trip tool, or an EPROM burner.  All host OSes
/// can write a flat byte file, so the pipeline is universally
/// available.
///
/// # Errors
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `Intel4004BackendError` — the IIR contained an op or type the
///   Intel 4004 backend does not yet handle (the message names the
///   function and op).
/// * `Io` — failed to read the input or write the output.
///
/// # Example downstream invocation
///
/// ```bash
/// lang-aot foo.twig --emit=intel4004 -o foo.bin
/// # Then disassemble or load into a simulator
/// ```
pub fn compile_file_to_intel4004_bin(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // Phase 4 of the historical-arch backend migration: route
    // through `aot_core` + `intel4004-backend` instead of the
    // legacy `iir_to_intel4004::lower_iir_to_intel4004`.  Same
    // pipeline as `compile_file_to_ge225_bin` (Phase 3).
    let _ = stem;
    let mut bytes = Vec::new();
    let empty_params: Vec<(String, String)> = Vec::new();
    for f in &module.functions {
        let inferred = aot_core::infer::infer_types(f);
        let cir = aot_core::specialise::aot_specialise(f, Some(&inferred));
        let ctx = jit_core::backend::FunctionContext {
            name: f.name.as_str(),
            params: &empty_params,
            return_type: f.return_type.as_str(),
        };
        let fn_bytes = intel4004_backend::compile(&ctx, &cir)
            .map_err(|e| LangAotError::Intel4004BackendError(format!("{e}")))?;
        bytes.extend_from_slice(&fn_bytes);
    }
    if bytes.is_empty() {
        bytes.extend_from_slice(&intel4004_encoder::HALT_LOOP);
    }

    std::fs::write(out, &bytes)?;
    Ok(())
}

/// Cross-platform: source → IIR → GE-225 machine code (`.bin`) on disk.
///
/// Unlike the native-executable pipelines, this one does **not** link
/// or run any toolchain — it just writes a flat `.bin` of 20-bit
/// GE-225 instruction words, each packed as 3 bytes (24 bits) big-
/// endian with the top 4 bits of byte 0 zero.  Downstream consumers:
///
/// * Any GE-225 simulator (historical software, the in-tree
///   `ge225-simulator` once it lands).
/// * A custom disassembler / decoder that reads 3 bytes per word
///   and masks off the top 4 bits.
///
/// No `cfg(target_os = ...)` gating: emitting bytes is platform-
/// agnostic.
///
/// # Wire format
///
/// GE-225 instructions are 20 bits each; `iir-to-ge225` packs them
/// as 3 bytes per word, big-endian, with the top 4 bits of byte 0
/// always zero (since 20 bits < 24 bits in 3 bytes).  This file
/// writes those bytes in order — no endianness conversion at the
/// file-format layer because we already chose big-endian inside the
/// word.
///
/// # Why no host gating?
///
/// The GE-225 is a 1959-era mainframe with no modern host equivalent.
/// Downstream is always a simulator or a custom decoder.  All host
/// OSes can write a flat byte file, so the pipeline is universally
/// available — same rationale as `compile_file_to_intel4004_bin`.
///
/// # Why is this Dartmouth BASIC's birthplace?
///
/// The GE-225 at Dartmouth College ran the very first BASIC program
/// in 1964.  Kemeny and Kurtz designed the language to fit this
/// machine's accumulator-anchored ISA and 20-bit word size — BASIC's
/// 16-bit integer defaults and single-letter variable names still
/// bear the imprint.  Compiling BASIC source through this pipeline
/// round-trips the language to the silicon it was designed for.
///
/// # Errors
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `Ge225BackendError` — the IIR contained an op or type the
///   GE-225 backend does not yet handle (the message names the
///   function and op).
/// * `Io` — failed to read the input or write the output.
///
/// # Example downstream invocation
///
/// ```bash
/// lang-aot foo.bas --emit=ge225 -o foo.bin
/// # Then load into a GE-225 simulator or decode 3 bytes at a time
/// ```
pub fn compile_file_to_ge225_bin(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // Phase 3 of the historical-arch backend migration: route
    // through `aot_core` + `ge225-backend` instead of the legacy
    // `iir_to_ge225::lower_iir_to_ge225`.
    //
    // Pipeline per function:
    //   1. `aot_core::infer::infer_types` — produce a name→type map.
    //   2. `aot_core::specialise::aot_specialise` — lift IIR to
    //      monomorphised CIR (`add_i64`, `cmp_lt_u32`, …).
    //   3. `ge225_backend::compile` — lower CIR to GE-225 bytes.
    //
    // Per-function byte streams are concatenated in declaration
    // order, mirroring `iir-to-ge225` v0.9.0's per-function layout
    // (every function's last instruction is a HLT or RTS, so
    // concatenation produces a well-formed program).  This matches
    // what the e2e smoke tests pin byte-for-byte.
    //
    // Cross-function `call` is currently not resolved here —
    // `ge225-backend` v0.1.0 returns `UnsupportedOp` for it and a
    // future increment will add module-level relocations.  All
    // existing tests (Twig literals, BASIC LET/PRINT/END,
    // Brainfuck empty programs) only exercise intra-function
    // control flow, so this is fine for v0.1.0.
    let mut bytes = Vec::new();
    let empty_params: Vec<(String, String)> = Vec::new();
    for f in &module.functions {
        let inferred = aot_core::infer::infer_types(f);
        let cir = aot_core::specialise::aot_specialise(f, Some(&inferred));
        let ctx = jit_core::backend::FunctionContext {
            name: f.name.as_str(),
            params: &empty_params,
            return_type: f.return_type.as_str(),
        };
        let fn_bytes = ge225_backend::compile(&ctx, &cir)
            .map_err(|e| LangAotError::Ge225BackendError(format!("{e}")))?;
        bytes.extend_from_slice(&fn_bytes);
    }

    // Empty-module guard — mirror `ge225_backend::compile` which
    // emits a HLT for empty CIR.
    if bytes.is_empty() {
        bytes.extend_from_slice(&ge225_encoder::HALT_WORD);
    }

    std::fs::write(out, &bytes)?;
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
        assert_eq!(Language::parse("mccarthy-lisp").unwrap(), Language::McCarthyLisp);
        assert_eq!(Language::parse("mccarthy").unwrap(), Language::McCarthyLisp);
        assert_eq!(Language::parse("mcl").unwrap(), Language::McCarthyLisp);
        assert_eq!(Language::parse("lisp").unwrap(), Language::McCarthyLisp);
        assert!(Language::parse("bogus").is_err());
    }

    #[test]
    fn mccarthy_language_displays_and_round_trips() {
        assert_eq!(Language::McCarthyLisp.to_string(), "mccarthy-lisp");
        assert_eq!(
            Language::parse(&Language::McCarthyLisp.to_string()).unwrap(),
            Language::McCarthyLisp
        );
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
        assert_eq!(detect_language_from_path(&p("foo.mcl")), Some(Language::McCarthyLisp));
        assert_eq!(detect_language_from_path(&p("foo.lisp")), Some(Language::McCarthyLisp));
        assert_eq!(detect_language_from_path(&p("foo.txt")), None);
        assert_eq!(detect_language_from_path(&p("README")), None);
    }

    #[test]
    fn mccarthy_lisp_compiles_to_iir() {
        // `lang-aot` routes McCarthy source through
        // `mccarthy-lisp-iir-compiler`.  We exercise a spread of the
        // language — a scalar literal, the symbol/cons-returning worked
        // example, and a closure — and check each yields a valid module
        // with a `main` entry point.  (Whether a *backend* can lower a
        // symbol/cons program is a separate, per-backend concern — L3b.)
        for src in ["42", "(CAR '(A B C))", "(CONS 'A 'B)", "((LAMBDA (X) X) 'Q)"] {
            let iir = compile_source_to_iir(Language::McCarthyLisp, src, "mcl")
                .unwrap_or_else(|e| panic!("McCarthy {src:?} must compile: {e:?}"));
            assert_eq!(iir.entry_point.as_deref(), Some("main"), "{src:?}");
            assert!(iir.validate().is_empty(), "{src:?} must validate");
        }
    }

    #[test]
    fn mccarthy_lisp_frontend_error_is_surfaced() {
        // A lex/parse error (lowercase is not a McCarthy symbol) comes back
        // as a `FrontendError` tagged with the language, not a panic.
        let err = compile_source_to_iir(Language::McCarthyLisp, "car", "mcl").unwrap_err();
        assert!(matches!(
            err,
            LangAotError::FrontendError { language: Language::McCarthyLisp, .. }
        ));
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
