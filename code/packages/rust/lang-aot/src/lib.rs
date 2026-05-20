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
//! | Dartmouth BASIC | **TODO** — needs a `dartmouth-basic-iir-compiler` crate (the existing `-ir-compiler` emits `compiler_ir::IrProgram`, not `interpreter_ir::IIRModule`) |
//! | Oct             | **TODO** — Python-only frontend; needs a Rust port or a bridge |
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
/// Returns [`LangAotError::UnsupportedLanguage`] for `DartmouthBasic`
/// and `Oct` — those frontends exist elsewhere (Python for Oct, and a
/// non-IIR Rust crate for BASIC) but haven't been ported to the shared
/// IIR shape yet.  The error carries a one-line guidance string
/// pointing at the work needed.
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
            brainfuck_iir_compiler::compile_source(source, module_name)
                .map_err(|e| LangAotError::FrontendError {
                    language,
                    message: e,
                })
        }
        Language::DartmouthBasic => Err(LangAotError::UnsupportedLanguage {
            language,
            guidance: "create code/packages/rust/dartmouth-basic-iir-compiler \
                       that emits `interpreter_ir::IIRModule` (the existing \
                       dartmouth-basic-ir-compiler emits a different IR shape \
                       and is not pluggable into the LANG VM AOT chain)",
        }),
        Language::Oct => Err(LangAotError::UnsupportedLanguage {
            language,
            guidance: "port the Python `oct-ir-compiler` to Rust (or bridge \
                       it via subprocess); the Python IR also uses a custom \
                       `IrProgram` shape that needs converting to \
                       `interpreter_ir::IIRModule`",
        }),
    }
}

// ---------------------------------------------------------------------------
// End-to-end pipelines: source → IIR → executable
//
// Each function is cfg-gated to the host that can actually link for the
// target — same policy as twig-aot.  Cross-OS object emission goes
// through `compile_object_to_disk` instead.
// ---------------------------------------------------------------------------

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

    #[test]
    fn twig_compiles_to_iir() {
        let iir = compile_source_to_iir(Language::Twig, "42", "twig")
            .expect("Twig must compile");
        assert!(!iir.functions.is_empty());
    }

    #[test]
    fn dartmouth_basic_returns_clean_unsupported_error() {
        let err = compile_source_to_iir(
            Language::DartmouthBasic, "10 PRINT 42", "basic"
        ).unwrap_err();
        match err {
            LangAotError::UnsupportedLanguage { language, .. } => {
                assert_eq!(language, Language::DartmouthBasic);
            }
            other => panic!("expected UnsupportedLanguage, got {other:?}"),
        }
    }

    #[test]
    fn oct_returns_clean_unsupported_error() {
        let err = compile_source_to_iir(Language::Oct, "1 + 1", "oct")
            .unwrap_err();
        match err {
            LangAotError::UnsupportedLanguage { language, .. } => {
                assert_eq!(language, Language::Oct);
            }
            other => panic!("expected UnsupportedLanguage, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_error_message_lists_guidance() {
        let err = compile_source_to_iir(Language::Oct, "", "oct").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("oct"), "msg should name the language: {msg}");
        assert!(msg.contains("Python") || msg.contains("port"),
                "msg should mention next-step guidance: {msg}");
    }
}
