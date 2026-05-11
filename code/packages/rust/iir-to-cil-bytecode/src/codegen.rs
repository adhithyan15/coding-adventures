//! `IIRClrCodeGenerator` — `CodeGenerator<IIRModule, CILProgramArtifact>` adapter.
//!
//! Wraps [`validate_iir_for_clr`] and [`lower_iir_to_cil`] in the shared
//! `codegen_core::CodeGenerator` protocol so callers can use any backend
//! interchangeably.
//!
//! ## Example
//!
//! ```
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
//! use codegen_core::codegen::CodeGenerator;
//! use iir_to_cil_bytecode::codegen::IIRClrCodeGenerator;
//!
//! let fn_ = IIRFunction::new("main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//! let mut module = IIRModule::new("test", "tetrad");
//! module.add_or_replace(fn_);
//!
//! let gen = IIRClrCodeGenerator::new("MyAssembly");
//! assert!(gen.validate(&module).is_empty());
//! let artifact = gen.generate(&module);
//! assert!(!artifact.methods[0].body.is_empty());
//! ```

use codegen_core::codegen::CodeGenerator;
use interpreter_ir::IIRModule;
use ir_to_cil_bytecode::backend::CILProgramArtifact;

use crate::lower::{IIRClrConfig, lower_iir_to_cil};
use crate::validate::validate_iir_for_clr;

// ===========================================================================
// IIRClrCodeGenerator
// ===========================================================================

/// `CodeGenerator<IIRModule, CILProgramArtifact>` adapter for the CLR backend.
///
/// Wraps `validate_iir_for_clr` and `lower_iir_to_cil` so the CLR IIR
/// backend participates in the shared code-generator protocol defined in
/// `codegen_core`.
///
/// Assembly is returned as a `CILProgramArtifact` — a structured multi-method
/// artifact ready for the CLR simulator or a PE-file packager.
#[derive(Debug, Clone)]
pub struct IIRClrCodeGenerator {
    config: IIRClrConfig,
}

impl IIRClrCodeGenerator {
    /// Create a new generator with the given assembly name.
    ///
    /// The assembly name is a CLR identifier, e.g. `"MyApp"`.
    pub fn new(assembly_name: impl Into<String>) -> Self {
        Self { config: IIRClrConfig::new(assembly_name) }
    }

    /// Create a generator with the default assembly name (`"IIRAssembly"`).
    pub fn default_name() -> Self {
        Self { config: IIRClrConfig::default() }
    }
}

impl Default for IIRClrCodeGenerator {
    fn default() -> Self {
        Self::default_name()
    }
}

impl CodeGenerator<IIRModule, CILProgramArtifact> for IIRClrCodeGenerator {
    /// Stable backend name used for registry lookups and debug output.
    fn name(&self) -> &str {
        "iir-clr"
    }

    /// Validate `ir` for the CLR CIL target.
    ///
    /// Returns a `Vec<String>` of error messages; empty = valid.
    fn validate(&self, ir: &IIRModule) -> Vec<String> {
        validate_iir_for_clr(ir)
    }

    /// Compile `ir` to a `CILProgramArtifact`.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would return errors.  Callers must validate
    /// first or ensure their IR is valid for the CLR target.
    fn generate(&self, ir: &IIRModule) -> CILProgramArtifact {
        lower_iir_to_cil(ir, &self.config)
            .unwrap_or_else(|e| {
                panic!(
                    "IIRClrCodeGenerator::generate called on invalid IR \
                     (call validate() first): {e}"
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
    use codegen_core::codegen::CodeGenerator;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    fn minimal_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        );
        let mut module = IIRModule::new("test", "tetrad");
        module.add_or_replace(fn_);
        module
    }

    #[test]
    fn name_is_iir_clr() {
        assert_eq!(IIRClrCodeGenerator::default_name().name(), "iir-clr");
    }

    #[test]
    fn validate_valid_module_is_empty() {
        let module = minimal_module();
        assert!(IIRClrCodeGenerator::default_name().validate(&module).is_empty());
    }

    #[test]
    fn validate_bad_type_returns_error() {
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any")],
        );
        let mut module = IIRModule::new("test", "tetrad");
        module.add_or_replace(fn_);
        let errors = IIRClrCodeGenerator::default_name().validate(&module);
        assert!(!errors.is_empty());
    }

    #[test]
    fn generate_valid_produces_artifact() {
        let module = minimal_module();
        let artifact = IIRClrCodeGenerator::default_name().generate(&module);
        assert!(!artifact.methods.is_empty());
        assert!(!artifact.methods[0].body.is_empty());
    }

    #[test]
    fn generate_body_contains_ret() {
        let module = minimal_module();
        let artifact = IIRClrCodeGenerator::default_name().generate(&module);
        assert!(artifact.methods[0].body.contains(&0x2A)); // ret
    }

    #[test]
    fn custom_assembly_name() {
        let gen = IIRClrCodeGenerator::new("TestLib");
        assert_eq!(gen.config.assembly_name, "TestLib");
        assert_eq!(gen.name(), "iir-clr");
    }

    #[test]
    fn round_trip_validate_then_generate() {
        let module = minimal_module();
        let gen = IIRClrCodeGenerator::default_name();
        let errors = gen.validate(&module);
        assert!(errors.is_empty());
        let artifact = gen.generate(&module);
        assert!(!artifact.methods[0].body.is_empty());
    }
}
