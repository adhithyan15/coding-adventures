//! `IIRBeamCodeGenerator` — adapter that wires the IIR BEAM backend behind the
//! `name()` / `validate()` / `generate()` API used across the LANG pipeline.
//!
//! # Why a code-generator adapter?
//!
//! The LANG20 pipeline defines a [`codegen_core::codegen::CodeGenerator`]
//! protocol: every backend exposes the same three methods so the pipeline
//! driver can treat all backends uniformly.  This thin adapter delegates to
//! [`validate::validate_for_beam`] and [`lower::lower_iir_to_beam`] and
//! handles the `Result → panic` translation expected by `generate()`.
//!
//! # Usage
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
//! use iir_to_beam::IIRBeamCodeGenerator;
//!
//! let fn_ = IIRFunction::new("main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("main".into()),
//!     language: "test".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! let gen = IIRBeamCodeGenerator::new("demo");
//! let errors = gen.validate(&module);
//! assert!(errors.is_empty());
//! let beam_module = gen.generate(&module);
//! assert_eq!(beam_module.name, "demo");
//! ```

use interpreter_ir::IIRModule;
use ir_to_beam::BEAMModule;

use crate::lower::{lower_iir_to_beam, IIRBeamConfig};
use crate::validate::validate_for_beam;

// ===========================================================================
// IIRBeamCodeGenerator
// ===========================================================================

/// BEAM code generator for `IIRModule` inputs.
///
/// Implements the LANG20 `name` / `validate` / `generate` protocol:
///
/// | Method | Delegates to |
/// |--------|-------------|
/// | `name()` | returns `"iir-beam"` (stable identifier) |
/// | `validate()` | [`validate_for_beam`] |
/// | `generate()` | [`lower_iir_to_beam`] (panics if validation would fail) |
#[derive(Debug, Clone)]
pub struct IIRBeamCodeGenerator {
    config: IIRBeamConfig,
}

impl IIRBeamCodeGenerator {
    /// Create a generator that will emit a BEAM module named `module_name`.
    ///
    /// # Example
    /// ```
    /// use iir_to_beam::IIRBeamCodeGenerator;
    /// let gen = IIRBeamCodeGenerator::new("myapp");
    /// ```
    pub fn new(module_name: impl Into<String>) -> Self {
        Self { config: IIRBeamConfig::new(module_name) }
    }

    /// Create a generator with the default module name `"iir_module"`.
    pub fn default_name() -> Self {
        Self { config: IIRBeamConfig::default() }
    }

    /// Stable backend identifier — always `"iir-beam"`.
    ///
    /// The hyphenated form distinguishes this backend from the `compiler-ir`
    /// based `"beam"` backend in `ir-to-beam`.
    pub fn name(&self) -> &str {
        "iir-beam"
    }

    /// Validate `ir` for BEAM lowering.
    ///
    /// Returns a list of human-readable error strings.
    /// An empty list means `ir` is safe to pass to [`generate`](Self::generate).
    pub fn validate(&self, ir: &IIRModule) -> Vec<String> {
        validate_for_beam(ir)
    }

    /// Lower `ir` to a [`BEAMModule`].
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would have returned errors.  Always call
    /// `validate` first in production code, or use
    /// [`lower_iir_to_beam`] directly to get a `Result`.
    pub fn generate(&self, ir: &IIRModule) -> BEAMModule {
        lower_iir_to_beam(ir, &self.config)
            .unwrap_or_else(|e| {
                panic!(
                    "IIRBeamCodeGenerator::generate called on invalid IIRModule: {}",
                    e
                )
            })
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};

    fn minimal_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        );
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    #[test]
    fn name_is_iir_beam() {
        let gen = IIRBeamCodeGenerator::new("test");
        assert_eq!(gen.name(), "iir-beam");
    }

    #[test]
    fn validate_valid_module() {
        let gen = IIRBeamCodeGenerator::new("test");
        assert!(gen.validate(&minimal_module()).is_empty());
    }

    #[test]
    fn generate_returns_correct_name() {
        let gen = IIRBeamCodeGenerator::new("mymod");
        let module = gen.generate(&minimal_module());
        assert_eq!(module.name, "mymod");
    }

    #[test]
    fn default_name_is_iir_module() {
        let gen = IIRBeamCodeGenerator::default_name();
        let module = gen.generate(&minimal_module());
        assert_eq!(module.name, "iir_module");
    }
}
