//! `IIRJvmCodeGenerator` — a thin adapter that wires the JVM backend into
//! whatever code-generator registry the host uses.
//!
//! # Role
//!
//! The `codegen-core` crate defines a `CodeGenerator<IR, Assembly>` trait that
//! every backend must implement.  This module implements it for:
//!
//! ```text
//! IR       = interpreter_ir::IIRModule
//! Assembly = jvm_class_file::JvmClassFile
//! ```
//!
//! The struct carries only the lowering configuration ([`IIRJvmConfig`]) — no
//! mutable state.  Multiple threads can share an `IIRJvmCodeGenerator`
//! (`Send + Sync`).
//!
//! # Usage
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_jvm_class_file::codegen::IIRJvmCodeGenerator;
//!
//! let fn_ = IIRFunction::new(
//!     "main",
//!     vec![],
//!     "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! );
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("main".into()),
//!     language: "test".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! // Create a generator with a custom class name.
//! let gen = IIRJvmCodeGenerator::new("MyApp");
//! assert_eq!(gen.name(), "iir-jvm");
//!
//! // Validate before generating.
//! let errors = gen.validate(&module);
//! assert!(errors.is_empty(), "{:?}", errors);
//!
//! // Generate the JvmClassFile.
//! let class_file = gen.generate(&module);
//! assert_eq!(class_file.methods.len(), 1);
//! ```

use interpreter_ir::IIRModule;
use jvm_class_file::JvmClassFile;

use crate::lower::{lower_iir_to_jvm, IIRJvmConfig};
use crate::validate::validate_for_jvm;

// ── IIRJvmCodeGenerator ───────────────────────────────────────────────────────

/// Code generator adapter for the IIR → JVM class file backend.
///
/// Implements a `name() / validate() / generate()` API matching the
/// `codegen-core::CodeGenerator` trait pattern.  This makes the JVM backend
/// interchangeable with all other LANG pipeline backends (BEAM, WASM, x86, …).
///
/// ## Instantiation
///
/// ```rust
/// use iir_to_jvm_class_file::codegen::IIRJvmCodeGenerator;
///
/// // Default class name "IIRModule":
/// let gen = IIRJvmCodeGenerator::default_class();
///
/// // Custom class name:
/// let gen2 = IIRJvmCodeGenerator::new("com/example/App");
/// assert_eq!(gen2.name(), "iir-jvm");
/// ```
pub struct IIRJvmCodeGenerator {
    /// Internal configuration forwarded to `lower_iir_to_jvm`.
    config: IIRJvmConfig,
}

impl IIRJvmCodeGenerator {
    /// Create a new generator that will emit a JVM class with the given name.
    ///
    /// # Arguments
    ///
    /// * `class_name` — The JVM binary class name (e.g. `"Main"`,
    ///   `"com/example/Foo"`).  Uses `/` as the package separator per the JVM
    ///   binary name spec.
    ///
    /// # Example
    ///
    /// ```
    /// use iir_to_jvm_class_file::codegen::IIRJvmCodeGenerator;
    /// let gen = IIRJvmCodeGenerator::new("Calculator");
    /// assert_eq!(gen.name(), "iir-jvm");
    /// ```
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            config: IIRJvmConfig::new(class_name),
        }
    }

    /// Create a generator with the default class name `"IIRModule"`.
    ///
    /// Equivalent to `IIRJvmCodeGenerator::new("IIRModule")`.
    pub fn default_class() -> Self {
        Self {
            config: IIRJvmConfig::default(),
        }
    }

    /// Return the canonical backend name: `"iir-jvm"`.
    ///
    /// This string is used as the key in the backend registry and in
    /// pipeline-selection logic.
    pub fn name(&self) -> &str {
        "iir-jvm"
    }

    /// Validate `ir` for JVM lowering.
    ///
    /// Calls [`validate_for_jvm`] and returns the error strings.  An empty
    /// `Vec` means the module can be safely lowered.
    ///
    /// # When to call
    ///
    /// Always call `validate` before `generate` in production code.
    /// `generate` panics on invalid input.
    pub fn validate(&self, ir: &IIRModule) -> Vec<String> {
        validate_for_jvm(ir)
    }

    /// Lower `ir` to a `JvmClassFile`.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would have returned errors.  Always call
    /// `validate` first in production code.
    ///
    /// # Returns
    ///
    /// A `JvmClassFile` whose `methods` correspond one-to-one with the
    /// functions in `ir`.  Each method has a `Code` attribute containing
    /// raw JVM bytecode.
    pub fn generate(&self, ir: &IIRModule) -> JvmClassFile {
        lower_iir_to_jvm(ir, &self.config)
            .expect("IIRJvmCodeGenerator::generate called on invalid IIRModule; call validate() first")
    }
}

impl Default for IIRJvmCodeGenerator {
    fn default() -> Self {
        Self::default_class()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    use super::*;

    fn void_module() -> IIRModule {
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

    fn add_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
            "i32",
            vec![
                IIRInstr::new(
                    "add",
                    Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())],
                    "i32",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
            ],
        );
        IIRModule {
            name: "add_test".into(),
            functions: vec![fn_],
            entry_point: Some("add".into()),
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    #[test]
    fn name_is_iir_jvm() {
        let gen = IIRJvmCodeGenerator::default_class();
        assert_eq!(gen.name(), "iir-jvm");
    }

    #[test]
    fn default_class_name() {
        let gen = IIRJvmCodeGenerator::default_class();
        assert_eq!(gen.config.class_name, "IIRModule");
    }

    #[test]
    fn custom_class_name() {
        let gen = IIRJvmCodeGenerator::new("MyApp");
        assert_eq!(gen.config.class_name, "MyApp");
    }

    #[test]
    fn validate_empty_on_valid_module() {
        let gen = IIRJvmCodeGenerator::default_class();
        let errors = gen.validate(&void_module());
        assert!(errors.is_empty(), "{:?}", errors);
    }

    #[test]
    fn validate_errors_on_empty_module() {
        let gen = IIRJvmCodeGenerator::default_class();
        let empty = IIRModule {
            name: "empty".into(),
            functions: vec![],
            entry_point: None,
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        };
        let errors = gen.validate(&empty);
        assert!(!errors.is_empty());
    }

    #[test]
    fn generate_void_module_ok() {
        let gen = IIRJvmCodeGenerator::new("TestClass");
        let class = gen.generate(&void_module());
        assert_eq!(class.methods.len(), 1);
    }

    #[test]
    fn generate_class_name_from_config() {
        let gen = IIRJvmCodeGenerator::new("CustomClass");
        let class = gen.generate(&void_module());
        assert_eq!(class.this_class_name, "CustomClass");
    }

    #[test]
    fn generate_add_module_ok() {
        let gen = IIRJvmCodeGenerator::new("Math");
        let class = gen.generate(&add_module());
        assert_eq!(class.methods.len(), 1);
        let method = &class.methods[0];
        assert_eq!(method.name, "add");
    }

    #[test]
    fn generated_method_has_code() {
        let gen = IIRJvmCodeGenerator::default_class();
        let class = gen.generate(&void_module());
        let code = class.methods[0].code_attribute().unwrap();
        assert!(!code.code.is_empty());
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<IIRJvmCodeGenerator>();
    }
}
