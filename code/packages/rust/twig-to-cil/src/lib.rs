//! # twig-to-cil — end-to-end Twig → CLR CIL compiler.
//!
//! This crate is the CLR pipeline crate in the LANG compilation stack.
//! It connects four upstream crates into a single ergonomic function call
//! that compiles a Twig source string all the way to a [`CILProgramArtifact`].
//!
//! ```text
//! Twig source (str)
//!     │
//!     ▼  twig-ir-compiler  ::  compile_source
//! IIRModule   (type_hint = "any" everywhere)
//!     │
//!     ▼  iir-type-checker  ::  infer_and_check
//! IIRModule   (constants and arithmetic get concrete hints: "i64", "bool", …)
//!     │
//!     ▼  iir-builtin-lowering  ::  lower_builtins
//! IIRModule   (call_builtin "+" → add; call_builtin "<" → lt; etc.)
//!     │
//!     ▼  iir-to-cil-bytecode  ::  lower_iir_to_cil
//! CILProgramArtifact  (structured, multi-method CLR artifact)
//! ```
//!
//! ## Return type — `CILProgramArtifact`
//!
//! The return type is [`CILProgramArtifact`] (from `ir-to-cil-bytecode`).
//! Each method in the artifact carries a `body: Vec<u8>` with raw CIL
//! bytecode, plus a `name` and `descriptor` for CLR reflection.
//!
//! This matches the existing `iir-to-cil-bytecode` crate convention: the
//! artifact is *structured* — callers can inspect per-method bytecode and
//! feed it to a CLR simulator without parsing a PE file.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use twig_to_cil::compile_twig_to_cil;
//!
//! // The pipeline may succeed or produce a CLR-stage error depending on
//! // whether type inference produces fully-typed IIR.
//! let result = compile_twig_to_cil("(+ 1 2)", "demo");
//! match result {
//!     Ok(artifact) => println!("Compiled {} method(s)", artifact.methods.len()),
//!     Err(e) => println!("Pipeline error: {e}"),
//! }
//! ```
//!
//! ## Error handling
//!
//! Every stage reports errors through [`TwigToCilError`].  See the `error`
//! module for details.
//!
//! ## Module structure
//!
//! | Module       | Contents |
//! |--------------|----------|
//! | [`error`]    | `TwigToCilError` — unified error type |
//! | [`pipeline`] | `run_pipeline` — stage wiring |

pub mod error;
pub mod pipeline;

pub use error::TwigToCilError;
pub use iir_to_cil_bytecode::{CILProgramArtifact, CILMethodArtifact, IIRClrConfig};
// Re-export interpreter_ir so tests can use it without a separate dependency.
pub use interpreter_ir;

use pipeline::run_pipeline;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Compile a Twig source string to a [`CILProgramArtifact`].
///
/// This is the primary entry point.  It runs all four pipeline stages:
///
/// 1. Twig frontend (`twig-ir-compiler`) → `IIRModule`
/// 2. Type inference + checking (`iir-type-checker`) → enriched `IIRModule`
/// 3. Builtin lowering (`iir-builtin-lowering`) → lowered `IIRModule`
/// 4. CLR backend (`iir-to-cil-bytecode`) → `CILProgramArtifact`
///
/// The assembly name in the output artifact is set to `module_name`.
/// For a custom assembly name, use [`pipeline::run_pipeline`] with a
/// custom [`IIRClrConfig`].
///
/// # Arguments
///
/// * `source` — Twig source code.  May be empty (`""`), in which case a
///   minimal `main` function that returns `nil` is emitted.
/// * `module_name` — Name for the `IIRModule` and the CLR assembly.
///
/// # Returns
///
/// A [`CILProgramArtifact`] with one [`CILMethodArtifact`] per Twig function.
/// Each method artifact contains:
///
/// - `name: String` — the function name (e.g. `"main"`, `"fact"`).
/// - `body: Vec<u8>` — raw CIL bytecode bytes.
/// - `descriptor: String` — the CLR method descriptor.
///
/// # Errors
///
/// Returns [`TwigToCilError`] on any pipeline failure:
///
/// - [`TwigToCilError::Compile`] — parse / IIR compile error.
/// - [`TwigToCilError::TypeCheck`] — type-checker found fatal errors.
/// - [`TwigToCilError::ClrValidation`] — pre-flight CLR validation failed.
/// - [`TwigToCilError::ClrBackend`] — CLR lowering pass returned `IIRClrError`.
///
/// # Examples
///
/// ```rust,no_run
/// use twig_to_cil::compile_twig_to_cil;
///
/// // Run the pipeline and handle both outcomes:
/// let result = compile_twig_to_cil("(+ 1 2)", "demo");
/// match result {
///     Ok(artifact) => {
///         println!("Compiled to {} CLR method(s)", artifact.methods.len());
///     }
///     Err(e) => {
///         // Most Twig programs hit the CLR type-system gap and return an error.
///         println!("Pipeline error: {e}");
///     }
/// }
/// ```
pub fn compile_twig_to_cil(
    source: &str,
    module_name: &str,
) -> Result<CILProgramArtifact, TwigToCilError> {
    let config = IIRClrConfig::new(module_name);
    run_pipeline(source, module_name, config)
}
