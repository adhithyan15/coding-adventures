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
//! | ALGOL 60        | scalar integer/boolean subset | `algol-iir-compiler` |
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

/// McCarthy Lisp on the universal JIT backend (W15).
pub mod jit_lisp;
pub use jit_lisp::run_mccarthy_on_jit;

/// Source language a `lang-aot` invocation is compiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Twig — original language the AOT pipeline was built for.
    Twig,
    /// Nib — typed expression language, multi-language implementation.
    Nib,
    /// Brainfuck — minimalist tape language; `brainfuck-iir-compiler` frontend
    /// lowered for AOT by `lower_brainfuck_for_aot`.
    Brainfuck,
    /// Dartmouth BASIC — integer subset (PRINT/LET/FOR/GOTO/IF) via the
    /// `dartmouth-basic-iir-compiler` Rust frontend over the shared IIR.
    DartmouthBasic,
    /// Oct — integer subset (let/if/while/calls) via the `oct-iir-compiler`
    /// Rust frontend over the shared IIR; `main` is void (exits 0).
    Oct,
    /// McCarthy Lisp — the 1960 Lisp 1.0, compiled via
    /// `mccarthy-lisp-iir-compiler` over the `lispy-runtime` value model.
    McCarthyLisp,
    /// ALGOL 60 — scalar integer/boolean subset over the shared IIR.
    Algol60,
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
            Language::Algol60 => write!(f, "algol60"),
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
            "algol" | "algol60" | "algol-60" | "a60" => Ok(Self::Algol60),
            other => Err(format!(
                "unknown language {other:?}; expected one of: twig, nib, \
                 brainfuck (or bf), dartmouth-basic (or basic / bas), oct, \
                 mccarthy-lisp (or mccarthy / mcl / lisp), algol60 \
                 (or algol / algol-60 / a60)")),
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
        "algol" | "alg" | "a60" => Some(Language::Algol60),
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
    /// The universal JIT (`jit-core`) rejected or trapped while running the IIR
    /// (McCarthy W15). Carries the JIT/VM error string.
    JitBackendError(String),
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
    /// The IBM 704 backend rejected the IIR.
    ///
    /// Carries the human-readable string from `ibm704-backend`.
    /// L4 of the McCarthy Lisp implementation — closes the
    /// round-trip to the silicon Lisp was born on.
    Ibm704BackendError(String),
    /// The WebAssembly backend rejected the IIR.
    ///
    /// Carries the string from `iir-to-wasm` (a validation failure, or an
    /// op/type its WasmGC lowering does not yet handle).  WASM is the first of
    /// the modern *managed* targets the worked example reaches (LANG77 /
    /// McCarthy L3b-3).
    WasmBackendError(String),
    /// The JVM (Java class-file) backend rejected the IIR.
    ///
    /// Carries the string from `iir-to-jvm-class-file` (a validation failure, or
    /// an op/type its lowering does not yet handle).  The JVM is the second of
    /// the modern *managed* targets, replicating the WASM uniform-reference value
    /// model with `Object`/`Integer` boxing (LANG77 / McCarthy W3).
    JvmBackendError(String),
    /// The CLR (.NET CIL) backend rejected the IIR.
    ///
    /// Carries the string from `iir-to-cil-bytecode`.  The CLR is the third of
    /// the modern *managed* targets, replicating the WASM/JVM uniform-reference
    /// value model with `object`/boxing (LANG77 / McCarthy W6).
    ClrBackendError(String),
    /// The BEAM (Erlang VM) backend rejected the IIR.
    ///
    /// Carries the string from `iir-to-beam`.  BEAM uses the native **Erlang
    /// terms** value model (integers, atoms, list cells) rather than the
    /// structural uniform-reference model of WASM/JVM/CLR (LANG77 / McCarthy W9).
    BeamBackendError(String),
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
            LangAotError::JitBackendError(m) => write!(f, "jit: {m}"),
            LangAotError::WasmBackendError(m) => write!(f, "wasm: {m}"),
            LangAotError::JvmBackendError(m) => write!(f, "jvm: {m}"),
            LangAotError::ClrBackendError(m) => write!(f, "clr: {m}"),
            LangAotError::BeamBackendError(m) => write!(f, "beam: {m}"),
            LangAotError::RiscvBackendError(m) => write!(f, "riscv32: {m}"),
            LangAotError::Intel8008BackendError(m) => write!(f, "intel8008: {m}"),
            LangAotError::Armv7BackendError(m) => write!(f, "armv7: {m}"),
            LangAotError::Intel4004BackendError(m) => write!(f, "intel4004: {m}"),
            LangAotError::Ge225BackendError(m) => write!(f, "ge225: {m}"),
            LangAotError::Ibm704BackendError(m) => write!(f, "ibm704: {m}"),
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
        Language::Algol60 => {
            algol_iir_compiler::compile_source(source, module_name)
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

/// Concretise a **scalar** module's `any`/`polymorphic` values to `i64` for the
/// LLVM backend (LANG77 / McCarthy W12a). The LLVM backend is a typed SSA IR, so
/// — like wasm/JVM/CLR/BEAM — a polymorphic scalar value must be given a concrete
/// type before lowering; for a pure-integer McCarthy program that type is `i64`.
/// Heap/reference functions (cons/symbols/lambda — the tagged-word value model
/// routed through `dynval_runtime.c`, W12b+) are left alone.
fn concretize_scalar_any_for_llvm(module: &mut IIRModule) {
    const HEAP_OPS: &[&str] = &["alloc", "field_load", "field_store", "is_null"];
    const LISP_BUILTINS: &[&str] = &[
        "cons", "car", "cdr", "pair?", "not", "equal?", "make_symbol", "make_nil", "null?",
    ];
    for func in &mut module.functions {
        let uses_lisp = func.params.iter().any(|(_, t)| {
            t == "any" || t == "symbol" || t.starts_with("ref<")
        }) || func.instructions.iter().any(|i| {
            HEAP_OPS.contains(&i.op.as_str())
                || (i.op == "call_builtin"
                    && matches!(i.srcs.first(),
                        Some(interpreter_ir::Operand::Var(n)) if LISP_BUILTINS.contains(&n.as_str())))
                || i.type_hint.starts_with("ref<")
        });
        if uses_lisp {
            continue; // tagged-word C-runtime value model — W12b+.
        }
        if func.return_type == "any" || func.return_type == "polymorphic" {
            func.return_type = "i64".to_string();
        }
        for instr in &mut func.instructions {
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                instr.type_hint = "i64".to_string();
            }
        }
    }
}

/// Cross-platform: source → IIR → **LLVM IR text** (`.ll`) for the default,
/// reproducible target (`x86_64-unknown-linux-gnu`) — McCarthy W12a.
///
/// The fifth `--emit` value model and the first **tagged-word** target (the
/// LLVM/AOT/JIT family that links the shared `dynval_runtime.c`, as opposed to the
/// managed object models of wasm/JVM/CLR or BEAM's native terms). This scalar
/// run-foundation concretises `any`→`i64` and lowers to LLVM IR; the cons /
/// predicate / symbol / lambda lowering (`call __dyn_*`) is W12b+.
pub fn compile_source_to_llvm(
    language: Language,
    source: &str,
    module_name: &str,
) -> Result<String, LangAotError> {
    compile_source_to_llvm_with_target(language, source, module_name, "x86_64-unknown-linux-gnu")
}

/// As [`compile_source_to_llvm`], but with a caller-chosen target triple. The
/// **verify-by-running** harness passes the *host* triple (`clang -dumpmachine`)
/// so `clang -x ir <out>.ll` produces a native executable that runs on the test
/// machine — the LLVM analogue of `wasm-runtime` / the `clr-simulator` / real
/// `erl`, but using the real `clang` already on the box.
pub fn compile_source_to_llvm_with_target(
    language: Language,
    source: &str,
    module_name: &str,
    target_triple: &str,
) -> Result<String, LangAotError> {
    let mut module = compile_source_to_iir(language, source, module_name)?;
    // The TAGGED-WORD lisp pipeline (McCarthy W12b) — the SAME passes the native
    // AOT path runs, NOT the managed structural pass. `lower_heap_builtins_runtime`
    // turns cons/car/cdr/pair?/equal?/not into `call_builtin "dyn_*"`;
    // `intern_symbols` assigns each symbol a tagged immediate; `lower_dyn_repr`
    // boxes integer literals to tagged words and inserts the final `dyn_unbox_int`
    // so the result is a plain `i64`. `iir-to-llvm` then lowers each `dyn_*` to a
    // `call @__dyn_*` into `dynval_runtime.c`. A no-op for a scalar program.
    iir_builtin_lowering::lower_heap_builtins_runtime(&mut module);
    iir_builtin_lowering::lower_dynamic_arith(&mut module);
    iir_builtin_lowering::intern_symbols(&mut module);
    iir_builtin_lowering::lower_dyn_repr(&mut module);
    // E6d-2b: rewrite the generic `box`/`unbox` ops that `lower_dynamic_arith`
    // emitted into `dyn_box_int`/`dyn_unbox_int` runtime calls — the tagged-i64
    // (LLVM) representation, which `iir-to-llvm`'s `DYN_BUILTINS` table lowers to
    // `call @__dyn_box_int` / `__dyn_unbox_int`. (The structural backends keep
    // the generic ops; this runs only on the native/LLVM pipeline.)
    iir_builtin_lowering::lower_box_unbox_to_runtime_calls(&mut module);
    // Concretise any residual scalar `any` (a pure-integer program never enters
    // the lisp passes above) to `i64`.
    concretize_scalar_any_for_llvm(&mut module);
    let cfg = iir_to_llvm::IIRLlvmConfig::new(module_name).with_target(target_triple);
    iir_to_llvm::lower_iir_to_llvm(&module, &cfg)
        .map_err(|e| LangAotError::LlvmBackendError(format!("{e}")))
}

/// Concretise the polymorphic `"any"`/`"polymorphic"` type hints of a **purely
/// scalar** function to `"i64"`, so it can flow through the typed WASM backend
/// (LANG77 / McCarthy L3b-3a-2).
///
/// `iir-to-wasm` requires concrete types — it rejects `"any"` (a lisp
/// `LispyValue`, which on the native path is just a tagged machine word but on
/// WasmGC is `anyref`). For a function with **no heap / reference ops**
/// (`alloc`/`field_*`/`is_null` or any `dyn_*`/`cons`/`car`/`cdr` builtin),
/// every value is a machine integer, so `"any"` safely means `"i64"`. We do
/// **not** touch functions that use the heap (cons cells, symbols) — those
/// need the boxed-`anyref` value model, a follow-up slice (L3b-3a-3).
fn concretize_scalar_any_for_wasm(module: &mut IIRModule) {
    const HEAP_OPS: &[&str] = &["alloc", "field_load", "field_store", "is_null"];
    const LISP_BUILTINS: &[&str] = &[
        "cons", "car", "cdr", "pair?", "not", "equal?", "make_symbol", "make_nil", "null?",
    ];

    for func in &mut module.functions {
        // Does this function touch the lisp heap / reference model? A function
        // with **lisp parameters** (a `LAMBDA`/`LABEL` — params typed `any` /
        // `symbol` / `ref<…>`) participates in the uniform-anyref boundary and is
        // owned by `lower_dyn_repr_structural`, so skip it here too (it has
        // already retyped them to `ref<…>` by the time this runs).
        let uses_lisp = func.params.iter().any(|(_, t)| {
            t == "any" || t == "symbol" || t.starts_with("ref<")
        }) || func.instructions.iter().any(|i| {
            HEAP_OPS.contains(&i.op.as_str())
                || (i.op == "call_builtin"
                    && matches!(i.srcs.first(),
                        Some(interpreter_ir::Operand::Var(n)) if LISP_BUILTINS.contains(&n.as_str())))
                || i.type_hint.starts_with("ref<")
        });
        if uses_lisp {
            continue; // boxed-anyref value model — out of scope for the scalar slice.
        }
        // Pure scalar function: every `any`/`polymorphic` value is an i64.
        if func.return_type == "any" || func.return_type == "polymorphic" {
            func.return_type = "i64".to_string();
        }
        for instr in &mut func.instructions {
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                instr.type_hint = "i64".to_string();
            }
        }
    }
}

/// Cross-platform: source → IIR → **WebAssembly** module bytes (LANG77 / L3b-3a).
///
/// The first of the modern *managed* `--emit` targets. Unlike the native
/// pipeline — which routes the lisp value model through the linked C runtime
/// (tagged `LispyValue`s) — the managed backends have their own typed object
/// model, so we run the **structural** heap lowering
/// (`iir_builtin_lowering::lower_heap_builtins`: `cons`/`car`/`cdr`/`null?` →
/// `alloc`/`field_*`/`is_null`, materialised by `iir-to-wasm` as WasmGC
/// `$LispyPair` structs), then concretise scalar `"any"` values to `i64`.
///
/// **As of L3b-3a-2, scalar McCarthy programs emit a runnable `.wasm`** (e.g.
/// `42` → a module whose `main` returns `i64 42`, verified by running it on the
/// in-repo `wasm-runtime`). Cons/symbol programs need the boxed-`anyref` value
/// model (a follow-up slice) and are not yet supported here.
///
/// Emitting bytes is platform-agnostic — no `cfg(target_os = ...)` gate.
///
/// # Errors
/// * `FrontendError` — the frontend rejected the source.
/// * `WasmBackendError` — `iir-to-wasm` rejected the (lowered) IIR.
pub fn compile_source_to_wasm(
    language: Language,
    source: &str,
    module_name: &str,
) -> Result<Vec<u8>, LangAotError> {
    let mut module = compile_source_to_iir(language, source, module_name)?;
    // Managed backends consume the structural cons form (not the native
    // runtime-call form). A no-op for a module without cons builtins.
    iir_builtin_lowering::lower_heap_builtins(&mut module);
    iir_builtin_lowering::lower_dynamic_arith(&mut module);
    // Intern symbol literals to distinct integers in a reserved range, so each
    // distinct symbol is a unique value (boxed as `i31ref`) and `EQ` compares
    // them with `i32.eq` — `(EQ 'A 'A)` true, `(EQ 'A 'B)` false (LANG77 / W1).
    // A no-op for a module with no symbol literals. Before the repr pass.
    iir_builtin_lowering::intern_symbols_structural(&mut module);
    // The two representation passes partition the module's functions:
    //   • heap-using functions → the structural pass boxes their integer atoms
    //     as `i31ref` and unboxes the entry result (uniform-anyref value model);
    //   • pure-scalar functions → `concretize_scalar_any_for_wasm` retypes their
    //     `any` to `i64`.
    // Together they leave every value concretely typed (LANG77 / L3b-3a-3c).
    iir_builtin_lowering::lower_dyn_repr_structural(&mut module);
    concretize_scalar_any_for_wasm(&mut module);

    let config = iir_to_wasm::IIRWasmConfig::default();
    let wasm = iir_to_wasm::lower_iir_to_wasm(&module, &config)
        .map_err(|e| LangAotError::WasmBackendError(format!("{e:?}")))?;
    iir_to_wasm::encode_module(&wasm)
        .map_err(|e| LangAotError::WasmBackendError(format!("{e:?}")))
}

/// Retype a **scalar** module's `any`/`polymorphic`/`i64` values to JVM `i32`,
/// for the JVM run-foundation (LANG77 / McCarthy W3a). Like
/// `concretize_scalar_any_for_wasm`, but the managed target here is the JVM, and
/// the in-repo `jvm-simulator` (used to verify) is a 32-bit integer machine — so
/// a scalar program's entry returns `int` (`ireturn`), not `long`. We leave
/// heap/reference functions alone (cons/symbols/lambda are W3b+, where the
/// uniform-`Object` value model lands).
fn concretize_scalar_any_for_jvm(module: &mut IIRModule) {
    const HEAP_OPS: &[&str] = &["alloc", "field_load", "field_store", "is_null"];
    const LISP_BUILTINS: &[&str] = &[
        "cons", "car", "cdr", "pair?", "not", "equal?", "make_symbol", "make_nil", "null?",
    ];
    // Concretization is a **whole-module** decision, not a per-function one,
    // because a `call` couples a caller and callee's value models: the caller
    // pushes the argument and consumes the result at the callee's *declared*
    // width, so if the two functions disagree the emitted bytecode is invalid.
    //
    // Concretely (LANG-FULL BA5 — Dartmouth BASIC `DEF FN`): a program like
    // `DEF FNS(X) = X * X : PRINT FNS(7)` lowers to a printing `main` plus a
    // non-printing helper `FNS`. `main` keeps the wide i64 model (its
    // `print_i64` needs a `long` — see the per-function note below), but if we
    // narrowed `FNS` to `(I)I` independently, `main` would `invokestatic` it
    // with a `long` argument and `lstore` an `int` result → real `java` rejects
    // it (`VerifyError`) and the program prints nothing. So: if **any** function
    // in the module prints, the whole scalar module stays at i64, keeping every
    // cross-function call signature consistent. (A module with no printing
    // function — Nib/Twig/ALGOL, which return an exit code — concretizes to i32
    // uniformly, exactly as before; this changes only printing/input modules.)
    //
    // `input_i64` (BASIC `INPUT X`) also forces the wide i64 model: the host's
    // `readLong()J` returns a JVM `long`, so the receiving slot must be `Long`
    // (two slots wide). Concretizing `"i64"` → `"i32"` would give the slot type
    // `Int`, but `lstore` would tag it as `Long` in the verifier's type state →
    // `VerifyError: type mismatch` when the subsequent `iload` reads an `Int`
    // slot that the verifier sees as `Long`. (BA-JVM-INPUT)
    const WIDE_I64_BUILTINS: &[&str] = &["print_i64", "input_i64"];
    let module_prints = module.functions.iter().any(|f| {
        f.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(i.srcs.first(),
                    Some(interpreter_ir::Operand::Var(n)) if WIDE_I64_BUILTINS.contains(&n.as_str()))
        })
    });
    for func in &mut module.functions {
        let uses_lisp = func.params.iter().any(|(_, t)| t == "any" || t == "symbol")
            || func.instructions.iter().any(|i| {
                HEAP_OPS.contains(&i.op.as_str())
                    || (i.op == "call_builtin"
                        && matches!(i.srcs.first(),
                            Some(interpreter_ir::Operand::Var(n)) if LISP_BUILTINS.contains(&n.as_str())))
                    || i.type_hint.starts_with("ref<")
            });
        if uses_lisp {
            continue; // uniform-Object value model — JVM W3b+.
        }
        // A function that prints (Dartmouth BASIC's `PRINT`) or reads input
        // (Dartmouth BASIC's `INPUT`) needs the **wide** i64 value model:
        //   • `print_i64` lowers to `lload val; invokestatic
        //     env/BasicRuntime.println(J)V`, i.e. the value is loaded as a `long`.
        //   • `input_i64` lowers to `invokestatic env/BasicRuntime.readLong()J;
        //     lstore dest`, i.e. the return value is stored as a `long`.
        // If we concretized either to `i32`, slot types would mismatch — a real
        // `java` rejects it with `VerifyError: Accessing value from uninitialized
        // register pair` / `type mismatch`. So, exactly like the lisp/heap functions
        // above, we leave any function using a wide builtin at its native i64 width.
        // (Concretization exists only because the in-repo `jvm-simulator` is a 32-bit
        // machine; BASIC runs on real `java`, where `long` is fine.)
        let uses_wide_builtin = func.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(i.srcs.first(),
                    Some(interpreter_ir::Operand::Var(n)) if WIDE_I64_BUILTINS.contains(&n.as_str()))
        });
        if uses_wide_builtin || module_prints {
            // Wide i64 value model: this function uses a wide builtin directly
            // or shares a module with one, so it must keep i64 to stay
            // call-signature-consistent with its callers/callees.
            continue;
        }
        let to_i32 = |t: &str| t == "any" || t == "polymorphic" || t == "i64";
        if to_i32(&func.return_type) {
            func.return_type = "i32".to_string();
        }
        // Concretize the **parameters** too, not just the return type and the
        // instruction hints. A scalar helper such as Nib's `double(x: u8)` widens
        // its parameter to `i64`; if we retype the body to `i32` but leave the
        // parameter `i64`, the emitted method's signature is the inconsistent
        // `(J)I` and its body does `iadd`/`ireturn` on a `long` parameter — which
        // a real `java` rejects with `VerifyError: Expecting to find integer on
        // stack`. (The in-repo `jvm-simulator` is laxer and didn't catch it, so
        // this only surfaced once a parameterized scalar program ran on real
        // `java` in the LANG-MATRIX JVM column.) The lisp/`any`-param functions
        // were already skipped by the `uses_lisp` guard above, so every parameter
        // reaching here is a concrete scalar — safe to bring down to `i32`.
        for (_, ty) in &mut func.params {
            if to_i32(ty) {
                *ty = "i32".to_string();
            }
        }
        for instr in &mut func.instructions {
            if to_i32(&instr.type_hint) {
                instr.type_hint = "i32".to_string();
            } else if let Some(elem) = interpreter_ir::opcodes::array_elem_type(&instr.type_hint) {
                // LANG-FULL E5: narrow an `array<i64>` handle to `array<i32>` in
                // lockstep with the scalar narrowing above. Otherwise `alloc_array`
                // would build a `long[]` while the (now-`i32`) `array_get`/
                // `array_set` element hints emit `iaload`/`iastore` — a `long[]`
                // with `iaload` fails real `java`'s verifier. Aligning the element
                // type across alloc/get/set makes the whole array `int[]`.
                if to_i32(&elem) {
                    instr.type_hint = interpreter_ir::opcodes::make_array_type("i32");
                }
            }
        }
    }
}

/// Cross-platform: source → IIR → **JVM class file** bytes (LANG77 / McCarthy W3).
///
/// The second of the modern *managed* `--emit` targets. The JVM has its own
/// uniform-reference value model (`Object` references, `Integer` boxing) — the
/// analogue of the WASM `anyref`/`i31ref` model. **W3a (this slice)** wires the
/// pipeline and runs **scalar** programs: source → IIR → `concretize_scalar_any_for_jvm`
/// → `iir-to-jvm-class-file` → a serialized `.class`. The cons/symbol/lambda value
/// model (the uniform-`Object` replication of the WASM passes) lands in W3b+.
///
/// Verified end-to-end by *running* the emitted class's entry method on the
/// in-repo `jvm-simulator` (see the `jvm_emit` tests) — no external `java`.
///
/// # Errors
/// * `FrontendError` — the frontend rejected the source.
/// * `JvmBackendError` — `iir-to-jvm-class-file` rejected the (lowered) IIR.
pub fn compile_source_to_jvm(
    language: Language,
    source: &str,
    class_name: &str,
) -> Result<Vec<u8>, LangAotError> {
    let class = compile_source_to_jvm_class(language, source, class_name)?;
    Ok(iir_to_jvm_class_file::serialize_jvm_class_file(&class))
}

/// Cross-platform: source → IIR → a **`JvmClassFile`** (the structured class,
/// pre-serialization).
///
/// The shared core of [`compile_source_to_jvm`]; exposed so a caller can inspect
/// or augment the class before serializing — e.g. a test that injects a
/// `main([Ljava/lang/String;)V` launcher to run the entry method on a real JVM.
///
/// Runs the **managed value-model pipeline**, identical to the wasm path: the
/// structural passes emit *backend-agnostic* `box`/`unbox`/`alloc`/`field_*`
/// ops, and the JVM backend lowers them to `Integer.valueOf`/`intValue` +
/// `Object[]` cons cells (where wasm uses `i31ref`/`$LispyPair`). That shared
/// representation is exactly the reusable primitive a future lisp-family language
/// inherits for free.
pub fn compile_source_to_jvm_class(
    language: Language,
    source: &str,
    class_name: &str,
) -> Result<iir_to_jvm_class_file::JvmClassFile, LangAotError> {
    let mut module = compile_source_to_iir(language, source, class_name)?;
    iir_builtin_lowering::lower_heap_builtins(&mut module);
    iir_builtin_lowering::lower_dynamic_arith(&mut module);
    iir_builtin_lowering::intern_symbols_structural(&mut module);
    iir_builtin_lowering::lower_dyn_repr_structural(&mut module);
    concretize_scalar_any_for_jvm(&mut module);

    let config = iir_to_jvm_class_file::IIRJvmConfig::new(class_name);
    iir_to_jvm_class_file::lower_iir_to_jvm(&module, &config)
        .map_err(|e| LangAotError::JvmBackendError(format!("{e:?}")))
}

/// Retype a **scalar** module's `any`/`polymorphic`/`i64` values to CLR `i32`,
/// for the CLR run-foundation (LANG77 / McCarthy W6a). The CLR twin of
/// `concretize_scalar_any_for_jvm`: the in-repo `clr-simulator` (used to verify)
/// is a 32-bit integer machine and `iir-to-cil-bytecode`'s entry returns
/// `int32`, so a scalar program's result is an `int`. Heap/reference functions
/// (cons/symbols/lambda — W6b+) are left for the uniform-`object` value model.
fn concretize_scalar_any_for_cil(module: &mut IIRModule) {
    const HEAP_OPS: &[&str] = &["alloc", "field_load", "field_store", "is_null"];
    const LISP_BUILTINS: &[&str] = &[
        "cons", "car", "cdr", "pair?", "not", "equal?", "make_symbol", "make_nil", "null?",
    ];
    for func in &mut module.functions {
        let uses_lisp = func.params.iter().any(|(_, t)| t == "any" || t == "symbol")
            || func.instructions.iter().any(|i| {
                HEAP_OPS.contains(&i.op.as_str())
                    || (i.op == "call_builtin"
                        && matches!(i.srcs.first(),
                            Some(interpreter_ir::Operand::Var(n)) if LISP_BUILTINS.contains(&n.as_str())))
                    || i.type_hint.starts_with("ref<")
            });
        if uses_lisp {
            continue; // uniform-object value model — CLR W6b+.
        }
        let to_i32 = |t: &str| t == "any" || t == "polymorphic" || t == "i64";
        if to_i32(&func.return_type) {
            func.return_type = "i32".to_string();
        }
        // Concretize the **parameters** too — the same fix the JVM path needed.
        // A scalar helper such as Nib's `double(x: u8)` widens its parameter to
        // `i64`; if the body is retyped to `i32` but the parameter is left `i64`,
        // the emitted CIL method signature is the inconsistent `int32(int64)` and
        // its body does `int32` arithmetic on an `int64` argument — CoreCLR's
        // verifier rejects the mismatch. The lisp/`any`-param functions were
        // already skipped by the `uses_lisp` guard, so every parameter reaching
        // here is a concrete scalar — safe to bring down to `i32`.
        for (_, ty) in &mut func.params {
            if to_i32(ty) {
                *ty = "i32".to_string();
            }
        }
        for instr in &mut func.instructions {
            if to_i32(&instr.type_hint) {
                instr.type_hint = "i32".to_string();
            }
        }
    }
}

/// Cross-platform: source → IIR → a **CLR CIL artifact** (LANG77 / McCarthy W6).
///
/// The third of the modern *managed* `--emit` targets. The CLR has its own
/// uniform-reference value model (`object` references, value-type boxing) — the
/// analogue of the WASM `anyref` / JVM `Object` models. **W6a (this slice)** wires
/// the pipeline and runs **scalar** programs: source → IIR →
/// `concretize_scalar_any_for_cil` → `iir-to-cil-bytecode`. The cons/symbol/lambda
/// value model (the uniform-`object` replication of the shared structural passes)
/// lands in W6b+.
///
/// Verified end-to-end by *running* the emitted entry method's CIL on the in-repo
/// `clr-simulator` (see the `cil_emit` tests) — no external `dotnet`.
///
/// # Errors
/// * `FrontendError` — the frontend rejected the source.
/// * `ClrBackendError` — `iir-to-cil-bytecode` rejected the (lowered) IIR.
pub fn compile_source_to_cil_artifact(
    language: Language,
    source: &str,
    name: &str,
) -> Result<iir_to_cil_bytecode::CILProgramArtifact, LangAotError> {
    let mut module = compile_source_to_iir(language, source, name)?;
    // The managed value-model pipeline — the same backend-agnostic structural
    // passes the wasm/JVM paths use. The CLR backend lowers `box`/`unbox`/
    // `alloc`/`field_*` to `box [int32]`/`unbox.any` + `object[]` cons cells
    // (where wasm uses `i31ref`/`$LispyPair` and the JVM `Integer`/`Object[]`).
    // A no-op for a module without cons/symbols (W6a scalar still flows through).
    iir_builtin_lowering::lower_heap_builtins(&mut module);
    iir_builtin_lowering::lower_dynamic_arith(&mut module);
    iir_builtin_lowering::intern_symbols_structural(&mut module);
    iir_builtin_lowering::lower_dyn_repr_structural(&mut module);
    concretize_scalar_any_for_cil(&mut module);

    let config = iir_to_cil_bytecode::IIRClrConfig::new(name);
    iir_to_cil_bytecode::lower_iir_to_cil(&module, &config)
        .map_err(|e| LangAotError::ClrBackendError(format!("{e:?}")))
}

/// Compile `source` to **textual CIL** (`.il`) for the **real CoreCLR** path
/// (CLR-real C1). Where [`compile_source_to_cil_artifact`] yields raw method bodies
/// for the in-repo `clr-simulator`, this emits `.il` source that real `ilasm`
/// assembles into a loadable PE assembly which runs on real `dotnet` — the CLR
/// analog of [`compile_source_to_llvm`] (textual LLVM IR → real `clang`).
///
/// C1 covers scalar McCarthy; later slices grow the `iir-to-cil-bytecode::emit_il`
/// op match (cons, predicates, `COND`, symbols, lambda).
pub fn compile_source_to_cil_text(
    language: Language,
    source: &str,
    name: &str,
) -> Result<String, LangAotError> {
    let mut module = compile_source_to_iir(language, source, name)?;
    // The same managed value-model pipeline the binary CIL path uses, so the
    // textual and binary emitters lower an identical program.
    iir_builtin_lowering::lower_heap_builtins(&mut module);
    iir_builtin_lowering::lower_dynamic_arith(&mut module);
    iir_builtin_lowering::intern_symbols_structural(&mut module);
    iir_builtin_lowering::lower_dyn_repr_structural(&mut module);
    concretize_scalar_any_for_cil(&mut module);

    let config = iir_to_cil_bytecode::IIRClrConfig::new(name);
    iir_to_cil_bytecode::emit_il(&module, &config)
        .map_err(|e| LangAotError::ClrBackendError(format!("{e:?}")))
}

/// Concretise a **scalar** module's `any`/`polymorphic` values to `i64` for the
/// BEAM run-foundation (LANG77 / McCarthy W9a). Unlike the WASM/JVM/CLR
/// simulators (32-bit), the BEAM has **native arbitrary-precision integers**, so
/// the natural concrete type is `i64` (the `iir-to-beam` backend's integer
/// width). The `iir-to-beam` validator rejects `any`/`polymorphic`, so a scalar
/// program must be concretised before lowering. Heap/reference functions
/// (cons/symbols/lambda — W9+, the native Erlang-terms model) are left alone.
fn concretize_scalar_any_for_beam(module: &mut IIRModule) {
    // The BEAM is **dynamically typed** — every value is an Erlang *term* — so the
    // natural concrete type for an `any`/`polymorphic` lisp value is `i64` (a
    // native Erlang integer; the term carries its real runtime shape regardless).
    // We concretize **per instruction**, not per function (W9b): a cons program's
    // scalar results — e.g. the `car`/`cdr` of a cell, or the final `ret` of an
    // integer — become `i64`, while the cons cells themselves keep their
    // `ref<LispyPair>` type for `iir-to-beam`'s `put_list`/`get_hd`/`get_tl`
    // lowering. We never rewrite a `ref<…>` type: those ARE the native list cells.
    // (`get_hd` returning a sub-list is still sound — the `i64` hint is a lowering
    // placeholder, never an unboxing op; BEAM resolves the real term at runtime.)
    let to_i64 = |t: &str| t == "any" || t == "polymorphic";
    for func in &mut module.functions {
        if to_i64(&func.return_type) {
            func.return_type = "i64".to_string();
        }
        for instr in &mut func.instructions {
            if to_i64(&instr.type_hint) {
                instr.type_hint = "i64".to_string();
            }
        }
    }
}

/// Cross-platform: source → IIR → a **BEAM module** (`.beam` bytes) (LANG77 / W9).
///
/// The fourth managed `--emit` target — and the first on the **Erlang VM**, whose
/// native value model (integers, atoms, list cells) replaces the structural
/// uniform-reference model of WASM/JVM/CLR. **W9a (this slice)** wires the
/// pipeline and runs **scalar** programs: source → IIR →
/// `concretize_scalar_any_for_beam` → `iir-to-beam` → `encode_beam`. The cons/
/// symbol/lambda Erlang-terms lowering lands in W9+.
///
/// `module_name` must be a valid Erlang atom (lowercase, `[a-z][a-z0-9_]*`); the
/// emitted module exports `main/0`, so a runner loads it and calls
/// `<module_name>:main()`. Verified end-to-end by running on a real `erl` (OTP).
///
/// # Errors
/// * `FrontendError` — the frontend rejected the source.
/// * `BeamBackendError` — `iir-to-beam` rejected the (lowered) IIR.
pub fn compile_source_to_beam(
    language: Language,
    source: &str,
    module_name: &str,
) -> Result<Vec<u8>, LangAotError> {
    let mut module = compile_source_to_iir(language, source, module_name)?;
    // BEAM uses the NATIVE Erlang-terms value model, not the managed structural
    // pass: `lower_heap_builtins` turns McCarthy `cons`/`car`/`cdr` into
    // `alloc ref<LispyPair>` + `field_store`/`field_load`, which `iir-to-beam`
    // maps directly to BEAM list ops — `put_list` (a cons cell `[H|T]`) and
    // `get_hd`/`get_tl` (`hd`/`tl`). Integers stay native Erlang integers; there
    // is NO boxing (unlike wasm/JVM/CLR). A no-op for a scalar-only module.
    iir_builtin_lowering::lower_heap_builtins(&mut module);
    iir_builtin_lowering::lower_dynamic_arith(&mut module);
    // McCarthy symbols (F6): intern each distinct symbol to a stable `i32` id
    // (`SYMBOL_ID_BASE = 1<<29`). The BEAM carries it as a native Erlang integer,
    // and `EQ` on symbols becomes integer equality (`is_eq_exact`). We use the
    // SAME structural interning the wasm/JVM/CLR backends use, so a given symbol
    // gets the SAME id on every "intern-to-integer" backend (it matters for the
    // cross-backend conformance suite). Lambda (F7) needs nothing extra — it is
    // already a method `call`, which `iir-to-beam` lowers natively (a BEAM fun).
    iir_builtin_lowering::intern_symbols_structural(&mut module);
    concretize_scalar_any_for_beam(&mut module);

    let config = iir_to_beam::IIRBeamConfig::new(module_name);
    let beam = iir_to_beam::lower_iir_to_beam(&module, &config)
        .map_err(|e| LangAotError::BeamBackendError(format!("{e:?}")))?;
    Ok(iir_to_beam::encode_beam(&beam))
}

/// Cross-platform: source file → IIR → JVM class file (`.class`) on disk.
///
/// Thin wrapper over [`compile_source_to_jvm`]. Pair with `--emit=jvm`.
pub fn compile_file_to_jvm(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("Main");
    let bytes = compile_source_to_jvm(language, &source, stem)?;
    std::fs::write(out, bytes)?;
    Ok(())
}

/// Cross-platform: source file → IIR → WebAssembly binary (`.wasm`) on disk.
///
/// Thin wrapper over [`compile_source_to_wasm`]. Pair with `--emit=wasm`. No
/// `cfg` gate (emitting bytes is platform-agnostic).
pub fn compile_file_to_wasm(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let bytes = compile_source_to_wasm(language, &source, stem)?;
    std::fs::write(out, bytes)?;
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

/// Cross-platform: source → IIR → IBM 704 machine code (`.bin`) on disk.
///
/// L4 of the McCarthy Lisp implementation.  Unlike the native-
/// executable pipelines, this one does **not** link or run any
/// toolchain — it just writes a flat `.bin` of 36-bit IBM 704
/// instruction words, packed 5 bytes per word (low byte first,
/// high 4 bits of the top byte zeroed).  Downstream consumers:
///
/// * A future in-tree `ibm704-simulator` (not yet shipped).
/// * Any IBM 704 emulator that consumes 5-byte-per-word streams.
/// * Period scholarship / replica hardware.
///
/// No `cfg(target_os = ...)` gating: emitting bytes is platform-
/// agnostic.
///
/// # Why the IBM 704?
///
/// The IBM 704 is the vacuum-tube mainframe John McCarthy and his
/// MIT students first ran Lisp on, in 1959.  `CAR` and `CDR` —
/// the two universal Lisp accessors — were literally IBM 704
/// instruction-word field names (**C**ontents of the
/// **A**ddress / **D**ecrement part of **R**egister).  Compiling
/// McCarthy Lisp source through this pipeline round-trips the
/// language to the silicon it was born on — the symmetric
/// counterpart of the Dartmouth BASIC → GE-225 round-trip.
///
/// # Wire format
///
/// One 36-bit word per instruction, packed as 5 bytes per word
/// (40 bits — 4 wasted padding bits zeroed in the top nibble of
/// the high byte), low byte first.  Same convention `ge225-encoder`
/// uses (20-bit words → 3 bytes) extended to 36 bits.  Per-function
/// byte streams are concatenated directly.
///
/// # Errors
///
/// * `FrontendError` — the language-specific frontend rejected the source.
/// * `Ibm704BackendError` — the IIR contained an op or type the
///   IBM 704 backend does not yet handle (the message names the
///   function and op).  Per the v0.1.0 scope decision, CONS-using
///   programs are out of scope for every historical-arch backend.
/// * `Io` — failed to read the input or write the output.
///
/// # Example downstream invocation
///
/// ```bash
/// lang-aot foo.lisp --emit=ibm704 -o foo.bin
/// # Each 5-byte chunk decodes to a 36-bit IBM 704 word.
/// ```
pub fn compile_file_to_ibm704_bin(
    src: &Path,
    out: &Path,
    language: Language,
) -> Result<(), LangAotError> {
    let source = std::fs::read_to_string(src)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("lang");
    let module = compile_source_to_iir(language, &source, stem)?;

    // L4: route through aot_core::infer + aot_core::specialise +
    // ibm704_backend::compile per function, same pattern as the
    // historical-arch migration's Phases 3-7.  ibm704-backend emits
    // 5-byte-per-word output directly, so concatenation is just
    // `extend_from_slice`.
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
        let fn_bytes = ibm704_backend::compile(&ctx, &cir)
            .map_err(|e| LangAotError::Ibm704BackendError(format!("{e}")))?;
        bytes.extend_from_slice(&fn_bytes);
    }

    // Empty-module guard — mirror `ibm704_backend::compile` which
    // emits HTR 0 for empty CIR.
    if bytes.is_empty() {
        bytes.extend_from_slice(&ibm704_encoder::HTR_HALT_BYTES);
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
// 5. Widen every narrow-integer `type_hint` (`u8`/`u32` — the BF frontend's
//    cell and pointer widths) to `i64`, so the AOT/LLVM value model is a
//    uniform machine word. Byte width survives **only at the tape boundary**:
//    `load_byte` zero-extends the 8-bit cell to `i64` and `store_byte`
//    truncates back, so cell wrap-around (`255 + 1 == 0`) is still correct.
//    This is the LANG-MATRIX LM-L-Brainfuck fix: `iir-to-llvm` promotes any
//    reassigned variable (BF's `ptr`/`v`/`c`/`k`) to an `alloca i64` stack
//    slot, so a narrow `add i32`/`add i8` reading an `i64` slot-load would be
//    a type error (`'%__ld' defined with type 'i64' but expected 'i8'`).
//    Widening here makes every register `i64`, matching the slot model
//    without touching `iir-to-llvm`'s (McCarthy-critical) slot allocator.
//    We do it in this BF-specific pass rather than the frontend so the
//    frontend's `u8`/`u32` hints still reach `vm-core`/`jit-core`, whose
//    `specialise` step keys CIR opcode widths (`add_u8`/`add_u32`) off them.
//    Native AOT is unaffected: its byte ops already ignore the hint (they
//    zero-extend / truncate at the asm level) and its arithmetic runs in
//    64-bit registers regardless.

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
                    srcs.extend(instr.srcs);
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
        // Step 5 — widen narrow-integer hints to i64 (see the function-level
        // comment). `void`/`i64`/`u64`/`bool`/floats/lisp refs are left as-is;
        // only the BF cell/pointer widths (`u8`/`u32`, plus their signed and
        // 16-bit cousins for completeness) become `i64`.
        for instr in &mut new_instrs {
            if is_narrow_int_hint(&instr.type_hint) {
                instr.type_hint = "i64".to_string();
            }
        }

        func.instructions = new_instrs;
        // Reflect the step-4 return-type change so downstream type
        // propagation in twig-aot sees i64 instead of void.
        func.return_type = "i64".to_string();
    }
}

/// True when `hint` is a narrow (< 64-bit) machine integer type that the
/// Brainfuck-for-AOT pass widens to `i64`. Used by [`lower_brainfuck_for_aot`]
/// Step 5. We deliberately do **not** widen `u64`/`i64` (already a word),
/// `void`, `bool`, floats, `any`, `symbol`, or any `ref<…>` lisp type.
fn is_narrow_int_hint(hint: &str) -> bool {
    matches!(hint, "i8" | "u8" | "i16" | "u16" | "i32" | "u32")
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
        assert_eq!(Language::parse("algol60").unwrap(), Language::Algol60);
        assert_eq!(Language::parse("algol").unwrap(), Language::Algol60);
        assert_eq!(Language::parse("a60").unwrap(), Language::Algol60);
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
        assert_eq!(detect_language_from_path(&p("foo.algol")), Some(Language::Algol60));
        assert_eq!(detect_language_from_path(&p("foo.alg")), Some(Language::Algol60));
        assert_eq!(detect_language_from_path(&p("foo.a60")), Some(Language::Algol60));
        assert_eq!(detect_language_from_path(&p("foo.txt")), None);
        assert_eq!(detect_language_from_path(&p("README")), None);
    }

    #[test]
    fn algol_compiles_to_iir() {
        let src = "begin integer result; result := 42 end";
        let iir = compile_source_to_iir(Language::Algol60, src, "algol")
            .expect("ALGOL scalar program must compile");
        assert_eq!(iir.entry_point.as_deref(), Some("main"));
        assert!(iir.validate().is_empty());
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
        assert!(!ops.contains(&"ret_void"),
                "ret_void must be replaced by ret i64 0");
        assert_eq!(main.return_type, "i64",
                   "main return type must be i64 after lowering");
        // Last two instrs are `const __bf_ret = 0; ret __bf_ret`.
        let last_two = &ops[ops.len()-2..];
        assert_eq!(last_two, &["const", "ret"],
                   "epilogue must be `const; ret`; got {last_two:?}");
    }

    /// Step 5: every narrow-integer `type_hint` the BF frontend emits (`u8` for
    /// cells, `u32` for the pointer) must be widened to `i64` after lowering, so
    /// the AOT/LLVM value model is a uniform machine word. Byte width survives
    /// only inside `load_byte`/`store_byte` (the backend zero-extends/truncates).
    /// `void` stays `void`. This is the LANG-MATRIX LM-L-Brainfuck fix that makes
    /// `iir-to-llvm`'s i64-only slot model accept Brainfuck.
    #[test]
    fn brainfuck_lowering_widens_narrow_hints_to_i64() {
        // A program that exercises cells (`+`/`-`), the pointer (`>`/`<`),
        // a loop guard, and output (`.`).
        let iir = compile_source_to_iir(
            Language::Brainfuck, "+>+<[->+<].", "bf"
        ).expect("brainfuck must compile");
        let main = iir.functions.iter().find(|f| f.name == "main")
            .expect("BF main must exist");

        for instr in &main.instructions {
            // No narrow integer hint may survive.
            assert!(
                !matches!(instr.type_hint.as_str(),
                          "u8" | "u32" | "u16" | "i8" | "i16" | "i32"),
                "instr {:?} kept a narrow hint {:?} — must be widened to i64",
                instr.op, instr.type_hint,
            );
            // Every hint is now either i64 (registers + tape ops) or void
            // (control flow / store_byte / putchar).
            assert!(
                instr.type_hint == "i64" || instr.type_hint == "void",
                "instr {:?} has unexpected hint {:?}; expected i64 or void",
                instr.op, instr.type_hint,
            );
        }
        // The tape ops carry the widened i64 hint specifically.
        let load_byte = main.instructions.iter().find(|i| i.op == "load_byte").unwrap();
        assert_eq!(load_byte.type_hint, "i64", "load_byte hint widened to i64");
    }

    /// `is_narrow_int_hint` widens only sub-64-bit machine integers; it leaves
    /// `i64`/`u64`, `void`, `bool`, floats, and lisp/`any` types alone.
    #[test]
    fn is_narrow_int_hint_classifies_widths() {
        for narrow in ["i8", "u8", "i16", "u16", "i32", "u32"] {
            assert!(is_narrow_int_hint(narrow), "{narrow} should widen");
        }
        for wide in ["i64", "u64", "void", "bool", "f32", "f64", "any", "symbol", "ref<LispyPair>"] {
            assert!(!is_narrow_int_hint(wide), "{wide} must NOT widen");
        }
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
