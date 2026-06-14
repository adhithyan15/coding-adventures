//! # codegen-core — Universal IR-to-native compilation layer (LANG19/LANG20)
//!
//! `codegen-core` is the single shared layer that defines what code generation
//! means across every compilation path in this repository:
//!
//! ## Compilation paths
//!
//! ```text
//! JIT path (jit-core → codegen-core)
//!   IIRFunction
//!     → jit_core::specialise()   → Vec<CIRInstr>
//!     → CIROptimizer::run()      → Vec<CIRInstr>  (constant fold + DCE)
//!     → Backend::compile()       → bytes
//!
//! AOT path (aot-core → codegen-core)
//!   IIRFunction
//!     → aot_core::aot_specialise() → Vec<CIRInstr>
//!     → CIROptimizer::run()         → Vec<CIRInstr>
//!     → Backend::compile()          → bytes
//!
//! Compiled-language path (Nib, BF, Algol-60 → codegen-core)
//!   IrProgram
//!     → IrProgramOptimizer::optimize() → IrProgram
//!     → CodeGenerator::generate()      → Assembly  (LANG20)
//!     → Backend::compile()             → bytes
//! ```
//!
//! ## Module structure
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`optimizer`] | `Optimizer<IR>` trait, `CIROptimizer`, `IrProgramOptimizer` |
//! | [`codegen`]   | `CodeGenerator<IR, Assembly>` trait, `CodeGeneratorRegistry` |
//! | [`pipeline`]  | `CodegenPipeline<IR>`, `CodegenResult<IR>` |
//! | [`registry`]  | `BackendRegistry` |
//!
//! ## Re-exports from jit-core
//!
//! `CIRInstr`, `CIROperand`, `Backend` are already defined in `jit-core`
//! and re-exported here so callers can import everything from one place.

pub mod codegen;
pub mod optimizer;
pub mod pipeline;
pub mod registry;

// Re-export from jit-core so callers can use codegen-core as a one-stop shop.
pub use jit_core::backend::Backend;
pub use jit_core::cir::{CIRInstr, CIROperand};
pub use jit_core::optimizer::CIROptimizer;

// Re-export the new types.
pub use codegen::{CodeGenerator, CodeGeneratorRegistry};
pub use optimizer::IrProgramOptimizer;
pub use pipeline::{Compile, CodegenPipeline, CodegenResult, Optimizer};
pub use registry::BackendRegistry;
