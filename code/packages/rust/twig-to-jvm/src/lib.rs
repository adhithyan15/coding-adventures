//! # twig-to-jvm — end-to-end Twig → JVM class file compiler.
//!
//! This crate is the JVM pipeline crate in the LANG compilation stack.
//! It connects every upstream stage into a single ergonomic function call:
//!
//! ```text
//! Twig source (str)
//!     │
//!     ▼  twig_ir_compiler::compile_source
//! IIRModule   (type_hint = "any" everywhere)
//!     │
//!     ▼  iir_type_checker::infer_and_check
//! IIRModule   (type_hint filled in for constants and arithmetic)
//!     │
//!     ▼  iir_builtin_lowering::lower_builtins
//! IIRModule   (call_builtin "+" → add, etc.)
//!     │
//!     ▼  iir_to_jvm_class_file::lower_iir_to_jvm
//! JvmClassFile  (structured, multi-method JVM class)
//! ```
//!
//! ## Why is the return type `JvmClassFile` and not `Vec<u8>`?
//!
//! `JvmClassFile` is a *structured* representation — a Rust struct whose
//! fields mirror the JVM class-file spec (`constant_pool`, `methods`, …).
//! The `jvm-class-file` crate does not expose a `to_bytes` / `serialize`
//! method on `JvmClassFile`; the raw-byte builder (`build_minimal_class_file`)
//! is a separate low-level API.
//!
//! Returning a structured `JvmClassFile` lets callers:
//!
//! - Inspect methods, constant pool entries, and bytecode in tests without
//!   parsing raw bytes.
//! - Feed the result to a JVM simulator or an existing serializer of their
//!   choice.
//! - Avoid a lossy round-trip (structured → bytes → structured) when the
//!   downstream tool already works with the structured form.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use twig_to_jvm::compile_twig_to_jvm;
//!
//! // Note: Twig's dynamic typing means most programs reach but do not fully
//! // pass the JVM backend's strict type requirements.  Use compile_twig_to_jvm
//! // and handle both Ok and Err variants.
//! let result = compile_twig_to_jvm("(+ 1 2)", "demo");
//! // The result may be Ok (if the JVM backend accepts the typed IIR) or Err.
//! println!("{result:?}");
//! ```
//!
//! ## Error handling
//!
//! Every stage reports errors through [`TwigToJvmError`].  See the `error`
//! module documentation for details on each variant.
//!
//! ## Module structure
//!
//! | Module      | Contents |
//! |-------------|----------|
//! | [`error`]   | `TwigToJvmError` — unified error type |
//! | [`pipeline`] | `run_pipeline` — stage wiring |

pub mod error;
pub mod pipeline;

pub use error::TwigToJvmError;
pub use iir_to_jvm_class_file::JvmClassFile;
pub use iir_to_jvm_class_file::IIRJvmConfig;
// Re-export interpreter_ir types so tests can use them without a separate dep.
pub use interpreter_ir;

use pipeline::run_pipeline;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Compile a Twig source string to a [`JvmClassFile`].
///
/// This is the primary entry point.  It runs all four pipeline stages:
///
/// 1. Twig frontend (`twig-ir-compiler`) → `IIRModule`
/// 2. Type inference + checking (`iir-type-checker`) → enriched `IIRModule`
/// 3. Builtin lowering (`iir-builtin-lowering`) → lowered `IIRModule`
/// 4. JVM backend (`iir-to-jvm-class-file`) → `JvmClassFile`
///
/// The class name in the output `JvmClassFile` is derived from `module_name`
/// (capitalised to match JVM binary-name conventions).  For full control over
/// the class name, use [`pipeline::run_pipeline`] with a custom
/// [`IIRJvmConfig`].
///
/// # Arguments
///
/// * `source` — Twig source code.  May be empty (`""`), in which case a
///   minimal `main` that returns `nil` is emitted.
/// * `module_name` — Name for the `IIRModule` and the JVM class.  The name
///   should be a valid Java identifier (no `.`, no spaces).
///
/// # Returns
///
/// A [`JvmClassFile`] that contains one JVM method per Twig function plus
/// the synthetic `main` function.  Use the returned struct to inspect the
/// class, feed it to a simulator, or serialise it with `build_minimal_class_file`.
///
/// # Errors
///
/// Returns [`TwigToJvmError`] on any pipeline failure:
///
/// - [`TwigToJvmError::Compile`] — parse / IIR compile error.
/// - [`TwigToJvmError::TypeCheck`] — type-checker found fatal errors.
/// - [`TwigToJvmError::JvmValidation`] — pre-flight JVM validation failed.
/// - [`TwigToJvmError::JvmBackend`] — JVM lowering pass failed.
///
/// # Example
///
/// ```rust,no_run
/// use twig_to_jvm::compile_twig_to_jvm;
///
/// // The pipeline runs all stages; the result may be Ok or a JVM-stage error
/// // depending on whether type inference produces fully-typed IIR.
/// let result = compile_twig_to_jvm("(+ 1 2)", "demo");
/// match result {
///     Ok(cf) => println!("Compiled to JVM class with {} method(s)", cf.methods.len()),
///     Err(e) => println!("Pipeline error: {e}"),
/// }
/// ```
pub fn compile_twig_to_jvm(source: &str, module_name: &str) -> Result<JvmClassFile, TwigToJvmError> {
    // Use the module_name as the JVM class name directly.  Callers who need
    // a different class name can call pipeline::run_pipeline with a custom config.
    let config = IIRJvmConfig::new(module_name);
    run_pipeline(source, module_name, config)
}
