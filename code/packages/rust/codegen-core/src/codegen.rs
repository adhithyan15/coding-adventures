//! `CodeGenerator<IR, Assembly>` — the LANG20 protocol.
//!
//! A `CodeGenerator` separates the *code generation* concern from assembly,
//! packaging, and execution.  It takes a typed IR, validates it for the target
//! architecture, and produces "assembly" — whatever the backend naturally
//! emits before the packaging step.
//!
//! ## Why a separate protocol?
//!
//! LANG19's `Backend<IR>` bundled validate + generate + assemble + run into
//! one interface.  That works for single-stage backends but prevents:
//!
//! - Inspecting assembly before it is assembled into bytes.
//! - Sharing a validate-then-generate loop across the AOT and JIT pipelines.
//! - A clean seam between "IR → Assembly" and "Assembly → binary".
//!
//! LANG20 splits at the assembly boundary:
//!
//! ```text
//! [CodeGenerator] — validate + generate assembly    ← this module
//!     ↓ Assembly (str | Vec<u8> | WasmModule | …)
//!     ├──▶ [Assembler → Packager] → binary  (AOT path)
//!     ├──▶ [JIT runner]           → run     (JIT path)
//!     └──▶ [Simulator]            → run     (orthogonal)
//! ```
//!
//! ## Assembly type
//!
//! The `Assembly` type parameter is whatever the backend naturally produces
//! before packaging:
//!
//! - `String` — for text-assembly backends (Intel 4004, Intel 8008)
//! - `Vec<u8>` — for backends that emit binary directly (GE-225, JVM)
//! - Structured objects — WASM (`WasmModule`), CIL (`CILProgramArtifact`)
//!
//! ## Example
//!
//! ```rust
//! use codegen_core::codegen::{CodeGenerator, CodeGeneratorRegistry};
//!
//! // A mock generator for illustration.
//! struct EchoGenerator;
//!
//! impl CodeGenerator<String, String> for EchoGenerator {
//!     fn name(&self) -> &str { "echo" }
//!
//!     fn validate(&self, _ir: &String) -> Vec<String> { vec![] }
//!
//!     fn generate(&self, ir: &String) -> String {
//!         ir.clone()
//!     }
//! }
//!
//! let mut registry = CodeGeneratorRegistry::new();
//! registry.register("echo", Box::new(EchoGenerator) as Box<dyn std::any::Any + Send + Sync>);
//! // Registry stores type-erased generators keyed by name.
//! assert_eq!(registry.len(), 1);
//! ```

use std::any::Any;
use std::collections::HashMap;

// ── CodeGenerator trait ───────────────────────────────────────────────────────

/// Validates IR for a target architecture and generates target assembly.
///
/// Does NOT assemble (text → binary), package, link, or execute.
///
/// ## Type parameters
///
/// - `IR` — the typed intermediate representation consumed by this generator.
/// - `Assembly` — the output: text, bytes, or a structured object.
///
/// ## Contract
///
/// 1. `validate(ir)` returns an empty `Vec` if `ir` is valid for this target.
/// 2. `generate(ir)` MAY panic or return a wrong result if `validate(ir)` was
///    non-empty.  Well-behaved callers always validate first.
/// 3. `generate(ir)` implicitly calls `validate(ir)` and panics on the first
///    error — callers that do not need to inspect errors can skip the explicit
///    `validate` call.
pub trait CodeGenerator<IR, Assembly>: Send + Sync {
    /// A unique, stable name for this code generator (e.g. `"intel4004"`).
    fn name(&self) -> &str;

    /// Return validation errors for this target; empty list = valid.
    ///
    /// This is a *pre-flight check* — it reports issues before any code is
    /// generated, so the caller can produce a useful error message.
    fn validate(&self, ir: &IR) -> Vec<String>;

    /// Generate assembly from `ir`.
    ///
    /// # Panics
    ///
    /// May panic if `validate(ir)` would have returned errors.  Always call
    /// `validate` first in production code.
    fn generate(&self, ir: &IR) -> Assembly;
}

// ── CodeGeneratorRegistry ─────────────────────────────────────────────────────

/// Name-to-generator mapping.
///
/// The registry holds type-erased (`Box<dyn Any + Send + Sync>`) generators
/// because Rust does not support generic trait objects with multiple type
/// parameters directly.  Callers must downcast after retrieval.
///
/// ## Usage pattern
///
/// ```rust
/// use codegen_core::codegen::CodeGeneratorRegistry;
/// use std::any::Any;
///
/// let mut registry = CodeGeneratorRegistry::new();
/// assert!(registry.get("missing").is_none());
/// assert_eq!(registry.names(), Vec::<String>::new());
/// ```
pub struct CodeGeneratorRegistry {
    generators: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl CodeGeneratorRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            generators: HashMap::new(),
        }
    }

    /// Register a code generator.
    ///
    /// The generator is stored under its `name()`.  If a generator with the
    /// same name was previously registered, it is silently replaced.
    ///
    /// The generator must implement `Any + Send + Sync` so it can be stored
    /// and later downcast.  In practice, pass a `Box<dyn CodeGenerator<IR, Assembly>>`
    /// wrapped in a newtype that derives `Any`, or use `register_named`.
    pub fn register(&mut self, name: impl Into<String>, generator: Box<dyn Any + Send + Sync>) {
        self.generators.insert(name.into(), generator);
    }

    /// Retrieve a generator by name, as an opaque `Any` reference.
    ///
    /// The caller must downcast to the concrete `CodeGenerator` type.
    pub fn get(&self, name: &str) -> Option<&(dyn Any + Send + Sync)> {
        self.generators.get(name).map(|b| b.as_ref())
    }

    /// Return all registered generator names, sorted alphabetically.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.generators.keys().cloned().collect();
        names.sort();
        names
    }

    /// Return the number of registered generators.
    pub fn len(&self) -> usize {
        self.generators.len()
    }

    /// Return `true` if no generators are registered.
    pub fn is_empty(&self) -> bool {
        self.generators.is_empty()
    }
}

impl Default for CodeGeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal mock code generator for tests.
    struct MockGen {
        name: String,
        fail: bool,
    }

    impl MockGen {
        fn new(name: &str) -> Self {
            Self { name: name.into(), fail: false }
        }
        fn failing(name: &str) -> Self {
            Self { name: name.into(), fail: true }
        }
    }

    impl CodeGenerator<String, String> for MockGen {
        fn name(&self) -> &str { &self.name }

        fn validate(&self, _ir: &String) -> Vec<String> {
            if self.fail {
                vec!["mock validation error".into()]
            } else {
                vec![]
            }
        }

        fn generate(&self, ir: &String) -> String {
            format!("[{}] {}", self.name, ir)
        }
    }

    // Test 1: validate() returns empty on valid IR
    #[test]
    fn mock_validate_valid() {
        let gen = MockGen::new("echo");
        assert!(gen.validate(&"hello".to_string()).is_empty());
    }

    // Test 2: validate() returns errors when generator is set to fail
    #[test]
    fn mock_validate_failing() {
        let gen = MockGen::failing("bad");
        let errors = gen.validate(&"x".to_string());
        assert!(!errors.is_empty());
    }

    // Test 3: generate() produces output
    #[test]
    fn mock_generate() {
        let gen = MockGen::new("myarch");
        let out = gen.generate(&"add r0, r1".to_string());
        assert!(out.contains("myarch"));
        assert!(out.contains("add r0, r1"));
    }

    // Test 4: name() returns the generator name
    #[test]
    fn mock_name() {
        let gen = MockGen::new("intel4004");
        assert_eq!(gen.name(), "intel4004");
    }

    // Test 5: registry starts empty
    #[test]
    fn registry_starts_empty() {
        let reg = CodeGeneratorRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.names().is_empty());
    }

    // Test 6: register + get
    #[test]
    fn registry_register_and_get() {
        let mut reg = CodeGeneratorRegistry::new();
        reg.register("echo", Box::new(MockGen::new("echo")));
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        assert!(reg.get("echo").is_some());
        assert!(reg.get("missing").is_none());
    }

    // Test 7: names() returns sorted names
    #[test]
    fn registry_names_sorted() {
        let mut reg = CodeGeneratorRegistry::new();
        reg.register("zzz", Box::new(MockGen::new("zzz")));
        reg.register("aaa", Box::new(MockGen::new("aaa")));
        reg.register("mmm", Box::new(MockGen::new("mmm")));
        assert_eq!(reg.names(), vec!["aaa", "mmm", "zzz"]);
    }

    // Test 8: register replaces existing generator with same name
    #[test]
    fn registry_register_replaces() {
        let mut reg = CodeGeneratorRegistry::new();
        reg.register("gen", Box::new(MockGen::new("gen")));
        reg.register("gen", Box::new(MockGen::failing("gen")));
        assert_eq!(reg.len(), 1); // still one entry
        // The new entry should be the failing one.
        let stored = reg.get("gen").unwrap();
        let downcast = stored.downcast_ref::<MockGen>().unwrap();
        assert!(downcast.fail);
    }

    // Test 9: default() is the same as new()
    #[test]
    fn registry_default() {
        let reg: CodeGeneratorRegistry = Default::default();
        assert!(reg.is_empty());
    }

    // Test 10: downcast after retrieval
    #[test]
    fn registry_downcast() {
        let mut reg = CodeGeneratorRegistry::new();
        reg.register("echo", Box::new(MockGen::new("echo")));
        let any = reg.get("echo").unwrap();
        let gen = any.downcast_ref::<MockGen>().unwrap();
        assert_eq!(gen.name(), "echo");
    }
}
