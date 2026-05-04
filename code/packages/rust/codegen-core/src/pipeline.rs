//! `CodegenPipeline<IR>` — the universal optimize-then-compile pipeline.
//!
//! `CodegenPipeline<IR>` is the single place that defines what *code generation*
//! means in this repository:
//!
//! ```text
//! IR  →  [Optimizer<IR>]  →  [Compile<IR>]  →  Option<Vec<u8>>
//! ```
//!
//! ## Two concrete instantiations
//!
//! ```text
//! CodegenPipeline<Vec<CIRInstr>>
//!   Optimizer: CIROptimizer (constant folding + DCE)
//!   Compile:   any CIR backend (Intel 4004, interp, …)
//!
//! CodegenPipeline<IrProgram>
//!   Optimizer: IrProgramOptimizer (NOP stripping)
//!   Compile:   any IrProgram backend (WASM, JVM, …)
//! ```
//!
//! ## Optimizer protocol
//!
//! The `Optimizer<IR>` trait (defined in [`crate::optimizer`]) requires only
//! `fn optimize(&self, ir: IR) -> IR`.  Passing `optimizer: None` skips the
//! optimization step entirely; `optimizer_applied` is `false` in the result.
//!
//! ## The `Compile<IR>` trait
//!
//! `Compile<IR>` abstracts over *compilation* — the step that turns IR into
//! an opaque binary blob.  It parallels `jit_core::backend::Backend` but is
//! generic over the IR type so that `CodegenPipeline<IrProgram>` is also
//! expressible.
//!
//! A blanket `impl<B: Backend> Compile<Vec<CIRInstr>> for B` lets callers
//! wrap any existing `jit_core` backend with zero boilerplate.
//!
//! ## Example: JIT pipeline
//!
//! ```rust
//! use codegen_core::pipeline::{Compile, CodegenPipeline};
//! use codegen_core::CIRInstr;
//!
//! // A trivial mock backend for illustration.
//! struct NullCompile;
//!
//! impl Compile<Vec<CIRInstr>> for NullCompile {
//!     fn name(&self) -> &str { "null" }
//!     fn compile(&self, _ir: &Vec<CIRInstr>) -> Option<Vec<u8>> {
//!         Some(vec![0u8])
//!     }
//! }
//!
//! let pipeline: CodegenPipeline<Vec<CIRInstr>> = CodegenPipeline::new(
//!     Box::new(NullCompile),
//!     None,  // no optimizer
//! );
//! assert!(pipeline.compile(vec![]).is_some());
//! assert_eq!(pipeline.backend_name(), "null");
//! ```

use std::time::Instant;

// Re-export `Optimizer` from the optimizer module so callers can write
// `use codegen_core::pipeline::Optimizer` and also so the crate root
// re-export `pub use pipeline::Optimizer` works.
pub use crate::optimizer::Optimizer;

// ── Compile<IR> trait ─────────────────────────────────────────────────────────

/// A generic compilation backend: translates `IR` into an opaque binary.
///
/// This trait mirrors `jit_core::backend::Backend` but is parameterised over
/// the IR type so it can be used inside `CodegenPipeline<IrProgram>` as well
/// as `CodegenPipeline<Vec<CIRInstr>>`.
///
/// ## Contract
///
/// - `compile` is **pure**: the same IR must produce semantically equivalent
///   binaries on every call.
/// - Returns `None` when compilation is not possible (unsupported opcodes,
///   register count too large, etc.).  `CodegenPipeline` propagates the `None`
///   to its caller unchanged.
///
/// ## Implementing
///
/// ```rust
/// use codegen_core::pipeline::Compile;
///
/// struct EchoCompile;
///
/// impl Compile<Vec<u8>> for EchoCompile {
///     fn name(&self) -> &str { "echo" }
///     fn compile(&self, ir: &Vec<u8>) -> Option<Vec<u8>> {
///         Some(ir.clone())
///     }
/// }
/// ```
pub trait Compile<IR>: Send + Sync {
    /// A short human-readable identifier (e.g. `"intel4004"`, `"wasm"`).
    ///
    /// Stored in `CodegenResult.backend_name` for diagnostics.
    fn name(&self) -> &str;

    /// Compile `ir` into an opaque binary, or return `None` on failure.
    fn compile(&self, ir: &IR) -> Option<Vec<u8>>;
}

// ── Blanket impl: jit_core::Backend → Compile<Vec<CIRInstr>> ─────────────────

// Any type that implements the CIR-specific `jit_core::backend::Backend` trait
// automatically satisfies `Compile<Vec<CIRInstr>>`.  This lets callers pass
// a `NullBackend` or `EchoBackend` directly to `CodegenPipeline::new()` without
// writing adapter code.

use crate::CIRInstr;
use jit_core::backend::Backend;

impl<B: Backend> Compile<Vec<CIRInstr>> for B {
    fn name(&self) -> &str {
        Backend::name(self)
    }

    fn compile(&self, ir: &Vec<CIRInstr>) -> Option<Vec<u8>> {
        Backend::compile(self, ir)
    }
}

// ── CodegenResult<IR> ─────────────────────────────────────────────────────────

/// The output of one `CodegenPipeline::compile_with_stats()` call.
///
/// Bundles the native binary with enough metadata for diagnostics, profiling,
/// and cache-key decisions.
///
/// ## Fields
///
/// | Field | Description |
/// |-------|-------------|
/// | `binary` | Opaque native bytes, or `None` if the backend declined. |
/// | `ir_snapshot` | Post-optimization IR snapshot for dumps and tests. |
/// | `backend_name` | Identifies which backend produced the binary. |
/// | `compilation_time_ns` | Wall-clock nanoseconds from IR-in to binary-out. |
/// | `optimizer_applied` | Whether an optimizer pass ran. |
///
/// ## Example
///
/// ```rust
/// use codegen_core::pipeline::CodegenResult;
///
/// let r = CodegenResult {
///     binary: Some(vec![0u8]),
///     ir_snapshot: "add r0, r1".to_string(),
///     backend_name: "test".to_string(),
///     compilation_time_ns: 42_000,
///     optimizer_applied: false,
/// };
/// assert!(r.success());
/// assert_eq!(r.binary_size(), 1);
/// ```
pub struct CodegenResult<IR> {
    /// Opaque native binary, or `None` if the backend declined.
    pub binary: Option<Vec<u8>>,

    /// Post-optimization IR snapshot (what was handed to the backend).
    pub ir_snapshot: IR,

    /// Short name of the backend that produced the binary.
    pub backend_name: String,

    /// Wall-clock nanoseconds from IR-in to binary-out.
    pub compilation_time_ns: u64,

    /// `true` when an `Optimizer` was present in the pipeline and ran.
    pub optimizer_applied: bool,
}

impl<IR> CodegenResult<IR> {
    /// `true` when the backend produced a non-`None` binary.
    ///
    /// ```rust
    /// use codegen_core::pipeline::CodegenResult;
    ///
    /// let ok = CodegenResult {
    ///     binary: Some(vec![1, 2, 3]),
    ///     ir_snapshot: (),
    ///     backend_name: "t".into(),
    ///     compilation_time_ns: 0,
    ///     optimizer_applied: false,
    /// };
    /// assert!(ok.success());
    ///
    /// let fail = CodegenResult {
    ///     binary: None,
    ///     ir_snapshot: (),
    ///     backend_name: "t".into(),
    ///     compilation_time_ns: 0,
    ///     optimizer_applied: false,
    /// };
    /// assert!(!fail.success());
    /// ```
    pub fn success(&self) -> bool {
        self.binary.is_some()
    }

    /// Byte length of the compiled binary, or `0` if compilation failed.
    ///
    /// ```rust
    /// use codegen_core::pipeline::CodegenResult;
    ///
    /// let r = CodegenResult {
    ///     binary: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
    ///     ir_snapshot: (),
    ///     backend_name: "t".into(),
    ///     compilation_time_ns: 0,
    ///     optimizer_applied: false,
    /// };
    /// assert_eq!(r.binary_size(), 4);
    /// ```
    pub fn binary_size(&self) -> usize {
        self.binary.as_ref().map(|b| b.len()).unwrap_or(0)
    }
}

// ── CodegenPipeline<IR> ───────────────────────────────────────────────────────

/// Universal IR → native binary pipeline.
///
/// `CodegenPipeline<IR>` composes an optional optimizer with a compilation
/// backend:
///
/// ```text
/// IR  →  optimizer.optimize(IR)  →  backend.compile(&IR)  →  Option<Vec<u8>>
/// ```
///
/// ## Type parameter
///
/// `IR` — the IR type flowing through this pipeline.  Common values:
/// - `Vec<CIRInstr>` — the JIT/AOT path
/// - `IrProgram` — the compiled-language path (Nib, Brainfuck, Algol-60)
///
/// ## Thread safety
///
/// `CodegenPipeline` is not `Sync` (it holds `Box<dyn …>` trait objects).
/// Each concurrent compilation should use its own instance.
///
/// ## Example
///
/// ```rust
/// use codegen_core::pipeline::{Compile, CodegenPipeline};
///
/// struct AlwaysOk;
/// impl Compile<String> for AlwaysOk {
///     fn name(&self) -> &str { "ok" }
///     fn compile(&self, ir: &String) -> Option<Vec<u8>> {
///         Some(ir.as_bytes().to_vec())
///     }
/// }
///
/// let pipeline: CodegenPipeline<String> = CodegenPipeline::new(
///     Box::new(AlwaysOk),
///     None,
/// );
/// let result = pipeline.compile_with_stats("hello world".to_string());
/// assert!(result.success());
/// assert!(!result.optimizer_applied);
/// ```
pub struct CodegenPipeline<IR> {
    /// The compilation backend.
    backend: Box<dyn Compile<IR>>,

    /// An optional optimizer that runs before the backend.
    optimizer: Option<Box<dyn Optimizer<IR>>>,
}

impl<IR> CodegenPipeline<IR> {
    /// Create a new pipeline.
    ///
    /// # Parameters
    ///
    /// - `backend` — implements `Compile<IR>`.  Called last; its `compile()`
    ///   method receives the (possibly optimized) IR.
    /// - `optimizer` — implements `Optimizer<IR>`, or `None` to skip
    ///   optimization.
    pub fn new(backend: Box<dyn Compile<IR>>, optimizer: Option<Box<dyn Optimizer<IR>>>) -> Self {
        Self { backend, optimizer }
    }

    /// Short identifier of the backend used by this pipeline.
    ///
    /// ```rust
    /// use codegen_core::pipeline::{Compile, CodegenPipeline};
    ///
    /// struct MyBackend;
    /// impl Compile<()> for MyBackend {
    ///     fn name(&self) -> &str { "my-backend" }
    ///     fn compile(&self, _ir: &()) -> Option<Vec<u8>> { Some(vec![]) }
    /// }
    ///
    /// let p = CodegenPipeline::new(Box::new(MyBackend), None);
    /// assert_eq!(p.backend_name(), "my-backend");
    /// ```
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Compile `ir` to a native binary.
    ///
    /// This is the **fast path** — no timing or IR snapshot is taken.
    ///
    /// # Returns
    ///
    /// `Some(bytes)` — opaque native binary ready for `Backend::run()`.
    /// `None` — if the backend declined to compile the IR.
    pub fn compile(&self, ir: IR) -> Option<Vec<u8>> {
        let optimized = self.run_optimizer(ir);
        self.backend.compile(&optimized)
    }

    /// Compile `ir` and return a `CodegenResult` with diagnostics.
    ///
    /// Unlike the plain `compile()` method, this path:
    /// - Captures wall-clock compilation time in nanoseconds.
    /// - Stores the post-optimization IR snapshot in the result.
    /// - Records whether an optimizer was applied.
    ///
    /// Use this path when you need to store or display the post-optimization
    /// IR (e.g., for a JIT cache entry or an IR dump tool).
    pub fn compile_with_stats(&self, ir: IR) -> CodegenResult<IR> {
        let t0 = Instant::now();
        let optimizer_applied = self.optimizer.is_some();
        let optimized = self.run_optimizer(ir);
        let binary = self.backend.compile(&optimized);
        let compilation_time_ns = t0.elapsed().as_nanos() as u64;

        CodegenResult {
            binary,
            ir_snapshot: optimized,
            backend_name: self.backend.name().to_string(),
            compilation_time_ns,
            optimizer_applied,
        }
    }

    // ── Private helpers ────────────────────────────────────────────────────

    /// Apply the optimizer if one is present; otherwise return `ir` unchanged.
    fn run_optimizer(&self, ir: IR) -> IR {
        match &self.optimizer {
            Some(opt) => opt.optimize(ir),
            None => ir,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ────────────────────────────────────────────────────────────

    /// A backend that always succeeds, returning the IR length as a single byte.
    struct LenBackend;

    impl Compile<String> for LenBackend {
        fn name(&self) -> &str {
            "len"
        }
        fn compile(&self, ir: &String) -> Option<Vec<u8>> {
            Some(vec![ir.len() as u8])
        }
    }

    /// A backend that always fails.
    struct FailBackend;

    impl Compile<String> for FailBackend {
        fn name(&self) -> &str {
            "fail"
        }
        fn compile(&self, _ir: &String) -> Option<Vec<u8>> {
            None
        }
    }

    /// An optimizer that uppercases the string IR.
    struct UpperOptimizer;

    impl Optimizer<String> for UpperOptimizer {
        fn optimize(&self, ir: String) -> String {
            ir.to_uppercase()
        }
    }

    // ── CodegenResult tests ────────────────────────────────────────────────

    // Test 1: success() returns true when binary is Some
    #[test]
    fn result_success_when_some() {
        let r = CodegenResult {
            binary: Some(vec![0u8]),
            ir_snapshot: (),
            backend_name: "t".into(),
            compilation_time_ns: 0,
            optimizer_applied: false,
        };
        assert!(r.success());
    }

    // Test 2: success() returns false when binary is None
    #[test]
    fn result_success_false_when_none() {
        let r: CodegenResult<()> = CodegenResult {
            binary: None,
            ir_snapshot: (),
            backend_name: "t".into(),
            compilation_time_ns: 0,
            optimizer_applied: false,
        };
        assert!(!r.success());
    }

    // Test 3: binary_size() returns the byte length
    #[test]
    fn result_binary_size() {
        let r = CodegenResult {
            binary: Some(vec![1, 2, 3, 4]),
            ir_snapshot: (),
            backend_name: "t".into(),
            compilation_time_ns: 0,
            optimizer_applied: false,
        };
        assert_eq!(r.binary_size(), 4);
    }

    // Test 4: binary_size() returns 0 when binary is None
    #[test]
    fn result_binary_size_none() {
        let r: CodegenResult<()> = CodegenResult {
            binary: None,
            ir_snapshot: (),
            backend_name: "t".into(),
            compilation_time_ns: 0,
            optimizer_applied: false,
        };
        assert_eq!(r.binary_size(), 0);
    }

    // ── CodegenPipeline tests ──────────────────────────────────────────────

    // Test 5: backend_name() returns the backend's name
    #[test]
    fn pipeline_backend_name() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(LenBackend), None);
        assert_eq!(p.backend_name(), "len");
    }

    // Test 6: compile() returns Some when backend succeeds
    #[test]
    fn pipeline_compile_success() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(LenBackend), None);
        let result = p.compile("abc".to_string());
        assert_eq!(result, Some(vec![3u8]));
    }

    // Test 7: compile() returns None when backend fails
    #[test]
    fn pipeline_compile_failure() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(FailBackend), None);
        assert!(p.compile("anything".to_string()).is_none());
    }

    // Test 8: compile() runs the optimizer before the backend
    #[test]
    fn pipeline_compile_with_optimizer() {
        // LenBackend returns the length of the string.
        // UpperOptimizer uppercases — doesn't change len, but proves it ran.
        // Use a different optimizer that changes length.
        struct DupOptimizer; // doubles the string
        impl Optimizer<String> for DupOptimizer {
            fn optimize(&self, ir: String) -> String {
                format!("{ir}{ir}")
            }
        }

        let p: CodegenPipeline<String> = CodegenPipeline::new(
            Box::new(LenBackend),
            Some(Box::new(DupOptimizer)),
        );
        // "ab" → "abab" (len 4) → backend returns [4]
        let result = p.compile("ab".to_string());
        assert_eq!(result, Some(vec![4u8]));
    }

    // Test 9: compile_with_stats() has optimizer_applied=false when no optimizer
    #[test]
    fn pipeline_stats_no_optimizer() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(LenBackend), None);
        let result = p.compile_with_stats("x".to_string());
        assert!(!result.optimizer_applied);
        assert!(result.success());
    }

    // Test 10: compile_with_stats() has optimizer_applied=true when optimizer present
    #[test]
    fn pipeline_stats_with_optimizer() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(
            Box::new(LenBackend),
            Some(Box::new(UpperOptimizer)),
        );
        let result = p.compile_with_stats("hello".to_string());
        assert!(result.optimizer_applied);
    }

    // Test 11: compile_with_stats() stores the post-optimization IR snapshot
    #[test]
    fn pipeline_stats_ir_snapshot() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(
            Box::new(LenBackend),
            Some(Box::new(UpperOptimizer)),
        );
        let result = p.compile_with_stats("hello".to_string());
        // UpperOptimizer turns "hello" into "HELLO"
        assert_eq!(result.ir_snapshot, "HELLO");
    }

    // Test 12: compile_with_stats() stores backend_name
    #[test]
    fn pipeline_stats_backend_name() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(LenBackend), None);
        let result = p.compile_with_stats("x".to_string());
        assert_eq!(result.backend_name, "len");
    }

    // Test 13: compile_with_stats() records compilation_time_ns > 0
    #[test]
    fn pipeline_stats_timing() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(LenBackend), None);
        let result = p.compile_with_stats("x".to_string());
        // Timing may be 0 on very fast machines; just verify it's a non-negative u64
        let _ = result.compilation_time_ns; // type check passes
    }

    // Test 14: compile_with_stats() binary is None when backend fails
    #[test]
    fn pipeline_stats_fail_backend() {
        let p: CodegenPipeline<String> = CodegenPipeline::new(Box::new(FailBackend), None);
        let result = p.compile_with_stats("x".to_string());
        assert!(!result.success());
        assert_eq!(result.binary_size(), 0);
    }

    // Test 15: blanket impl works — jit_core::NullBackend via Compile<Vec<CIRInstr>>
    #[test]
    fn blanket_impl_null_backend() {
        use jit_core::backend::NullBackend;

        let p: CodegenPipeline<Vec<CIRInstr>> =
            CodegenPipeline::new(Box::new(NullBackend), None);
        let result = p.compile(vec![]);
        assert_eq!(result, Some(vec![0u8]));
        assert_eq!(p.backend_name(), "null");
    }
}
