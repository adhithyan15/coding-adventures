//! # nib-clr-compiler
//!
//! End-to-end compiler from Nib source text to CLR CIL bytecode.
//!
//! ## Pipeline
//!
//! ```text
//! Nib source
//!   → nib_parser::parse_nib()              — lex + parse Nib tokens into a grammar AST
//!   → nib_type_checker::check()            — type inference + constraint checking
//!   → nib_ir_compiler::compile_nib()       — emit target-independent IrProgram
//!   → ir_optimizer::optimize_program()     — constant fold, dead-code elimination
//!   → ir_to_cil_bytecode::lower_ir_to_cil_bytecode()
//!                                          — emit CIL method body bytes
//!   → CILProgramArtifact                   — structured CIL artifact (name + body + locals)
//! ```
//!
//! The CIL output can be:
//! - Wrapped in a PE/CLI assembly by `cli-assembly-writer` (future — Python-only today)
//! - Validated with `validate_for_clr` from `ir-to-cil-bytecode`
//! - Executed on a CLR runtime (Mono, .NET) after PE wrapping
//!
//! ## Quick start
//!
//! ```no_run
//! use nib_clr_compiler::compile_source;
//!
//! let result = compile_source("fn main() { let x: u4 = 7; }").unwrap();
//! assert!(!result.cil_bytes.is_empty());
//! assert_eq!(result.assembly_name, "NibProgram");
//! ```
//!
//! ## Error handling
//!
//! Each pipeline stage is labelled so callers can identify where a failure occurred:
//!
//! ```no_run
//! use nib_clr_compiler::{compile_source, PackageError};
//!
//! let err = compile_source("fn").unwrap_err();
//! assert_eq!(err.stage, "parse");
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use coding_adventures_nib_parser::parse_nib;
use compiler_ir::IrProgram;
use ir_optimizer::optimize_program;
use ir_to_cil_bytecode::{lower_ir_to_cil_bytecode, CILBackendConfig, CILProgramArtifact};
use nib_ir_compiler::{compile_nib, release_config, BuildConfig};
use nib_type_checker::{check, TypedAst};
use parser::grammar_parser::GrammarASTNode;

// ===========================================================================
// Public types
// ===========================================================================

/// The result of compiling a Nib program to CIL bytecode.
///
/// Every intermediate artifact is retained so callers can inspect any stage
/// of the pipeline for debugging or visualisation.
///
/// Note: `Debug` is implemented manually because [`CILProgramArtifact`] contains
/// a `Box<dyn CILTokenProvider>` which does not implement `Debug`.
pub struct PackageResult {
    /// Original Nib source code.
    pub source: String,
    /// Assembly name embedded in CLR metadata (e.g., `"NibProgram"`).
    pub assembly_name: String,
    /// Type name for the CLR class containing the entry method.
    pub type_name: String,
    /// Parsed grammar AST from the Nib parser.
    pub ast: GrammarASTNode,
    /// Type-annotated AST produced by the type checker.
    pub typed_ast: TypedAst,
    /// IrProgram as emitted by `nib-ir-compiler`, before optimisation.
    pub raw_ir: IrProgram,
    /// IrProgram after the IR optimiser has run (constant fold + DCE).
    pub optimized_ir: IrProgram,
    /// Structured CIL artifact: entry label, method bodies, local variable types.
    pub cil_artifact: CILProgramArtifact,
    /// Raw CIL method body bytes for the entry method.
    ///
    /// This is a convenience copy of `cil_artifact.methods[0].body` — the
    /// bytes that the CLR simulator or PE writer can consume directly.
    pub cil_bytes: Vec<u8>,
    /// Path where the output file was written, if [`NibClrCompiler::write_cil_file`] was used.
    pub assembly_path: Option<PathBuf>,
}

// Manual Debug because CILProgramArtifact contains Box<dyn CILTokenProvider> which lacks Debug.
impl fmt::Debug for PackageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageResult")
            .field("source", &self.source)
            .field("assembly_name", &self.assembly_name)
            .field("type_name", &self.type_name)
            .field("cil_bytes_len", &self.cil_bytes.len())
            .field("assembly_path", &self.assembly_path)
            .finish_non_exhaustive()
    }
}

/// An error produced at any stage of the compilation pipeline.
///
/// The `stage` field identifies which step failed so callers can give targeted
/// diagnostics without parsing the message string.
///
/// | `stage`          | Cause |
/// |------------------|-------|
/// | `"parse"`        | Malformed Nib syntax |
/// | `"type-check"`   | Type errors in the Nib source |
/// | `"ir-compile"`   | IR emission error (should be rare) |
/// | `"lower-cil"`    | CIL lowering error |
/// | `"write"`        | I/O error writing the output file |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageError {
    /// Short identifier for the pipeline stage that failed.
    pub stage: String,
    /// Human-readable error message.
    pub message: String,
}

impl PackageError {
    fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.stage, self.message)
    }
}

impl std::error::Error for PackageError {}

// ===========================================================================
// NibClrCompiler — the pipeline orchestrator
// ===========================================================================

/// End-to-end Nib → CLR CIL compiler.
///
/// Chains the Nib parser, type checker, IR compiler, IR optimiser, and CIL
/// backend into a single composable object.  All configuration lives on the
/// struct; the same instance can compile many programs without re-creating
/// the pipeline.
///
/// # Default configuration
///
/// ```
/// use nib_clr_compiler::NibClrCompiler;
///
/// let compiler = NibClrCompiler::default();
/// assert_eq!(compiler.assembly_name, "NibProgram");
/// assert_eq!(compiler.type_name, "NibProgram");
/// assert!(compiler.optimize_ir);
/// ```
#[derive(Debug, Clone)]
pub struct NibClrCompiler {
    /// Assembly name emitted in the CIL metadata header.
    pub assembly_name: String,
    /// Type name for the CLR class containing the entry method.
    pub type_name: String,
    /// Build configuration for the IR compiler (defaults to `release_config()`).
    pub build_config: Option<BuildConfig>,
    /// Whether to run the IR optimiser (dead-code elimination + constant folding).
    pub optimize_ir: bool,
    /// CIL backend configuration; `None` uses default (`CILBackendConfig` with syscall arg reg 4).
    pub cil_config: Option<CILBackendConfig>,
}

impl Default for NibClrCompiler {
    /// Build a compiler with sensible defaults.
    ///
    /// - `assembly_name` = `"NibProgram"` — CLR assembly name.
    /// - `type_name` = `"NibProgram"` — CLR type name.
    /// - `optimize_ir` = `true` — enables constant folding + dead-code elimination.
    fn default() -> Self {
        Self {
            assembly_name: "NibProgram".to_string(),
            type_name: "NibProgram".to_string(),
            build_config: None,
            optimize_ir: true,
            cil_config: None,
        }
    }
}

impl NibClrCompiler {
    // -----------------------------------------------------------------------
    // compile_source
    // -----------------------------------------------------------------------

    /// Compile Nib source to a CIL artifact.
    ///
    /// Runs all five pipeline stages in order:
    ///
    /// 1. **Parse** — `nib_parser::parse_nib`
    /// 2. **Type-check** — `nib_type_checker::check`
    /// 3. **IR compile** — `nib_ir_compiler::compile_nib`
    /// 4. **Optimise** — `ir_optimizer::optimize_program` (if `self.optimize_ir`)
    /// 5. **CIL lower** — `ir_to_cil_bytecode::lower_ir_to_cil_bytecode`
    ///
    /// # Errors
    ///
    /// Returns a [`PackageError`] if any stage fails.  Check `err.stage` to
    /// identify the failing step.
    ///
    /// # Example
    ///
    /// ```
    /// use nib_clr_compiler::NibClrCompiler;
    ///
    /// let result = NibClrCompiler::default()
    ///     .compile_source("fn main() { let x: u4 = 7; }")
    ///     .unwrap();
    /// assert!(!result.cil_bytes.is_empty());
    /// ```
    pub fn compile_source(&self, source: &str) -> Result<PackageResult, PackageError> {
        // ── Stage 1: Parse ─────────────────────────────────────────────────
        //
        // Turn raw Nib source into a grammar AST.  Syntax errors (missing
        // tokens, unexpected EOF) are caught here.
        let ast = parse_nib(source)
            .map_err(|err| PackageError::new("parse", err.to_string()))?;

        // ── Stage 2: Type-check ────────────────────────────────────────────
        //
        // Run type inference and constraint checking over the AST.  The type
        // checker returns a `TypeCheckResult<TypedAst>` with an `ok` flag and
        // a list of `TypeErrorDiagnostic` entries.  We collect all diagnostics
        // into a single multi-line message rather than stopping on the first
        // error, giving the user a full picture of what needs fixing.
        let type_result = check(ast.clone());
        if !type_result.ok {
            let diagnostics = type_result
                .errors
                .iter()
                .map(|e| format!("Line {}, Col {}: {}", e.line, e.column, e.message))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(PackageError::new("type-check", diagnostics));
        }

        // ── Stage 3: IR compile ────────────────────────────────────────────
        //
        // Convert the type-annotated AST into a target-independent `IrProgram`.
        // `release_config()` enables all optimisations (inlining, loop unrolling)
        // for the most compact IR before the optimiser runs.
        let config = self.build_config.unwrap_or_else(release_config);
        let ir_result = compile_nib(type_result.typed_ast.clone(), config);
        let raw_ir = ir_result.program;

        // ── Stage 4: IR optimise ───────────────────────────────────────────
        //
        // Run constant folding and dead-code elimination over the IrProgram.
        // Skipped when `optimize_ir = false` (useful for inspecting raw IR output).
        let optimized_ir = if self.optimize_ir {
            optimize_program(&raw_ir)
        } else {
            raw_ir.clone()
        };

        // ── Stage 5: CIL lower ─────────────────────────────────────────────
        //
        // Translate the IrProgram into CIL method body bytes.  The CIL
        // backend uses syscall_arg_reg=4 by default (matching the Python
        // `CILBackendConfig(call_register_count=None)` default).
        let cil_config = self.cil_config.clone().unwrap_or(CILBackendConfig {
            syscall_arg_reg: 4,
            ..Default::default()
        });
        let cil_artifact = lower_ir_to_cil_bytecode(&optimized_ir, Some(cil_config), None)
            .map_err(|err| PackageError::new("lower-cil", err.to_string()))?;

        // Extract the entry method body bytes for the convenience field.
        let cil_bytes = cil_artifact
            .methods
            .first()
            .map(|m| m.body.clone())
            .unwrap_or_default();

        Ok(PackageResult {
            source: source.to_string(),
            assembly_name: self.assembly_name.clone(),
            type_name: self.type_name.clone(),
            ast,
            typed_ast: type_result.typed_ast,
            raw_ir,
            optimized_ir,
            cil_bytes,
            cil_artifact,
            assembly_path: None,
        })
    }

    // -----------------------------------------------------------------------
    // write_cil_file
    // -----------------------------------------------------------------------

    /// Compile and write the raw CIL method body bytes to a file.
    ///
    /// The output file contains the raw bytes from the entry method body.
    /// This is useful for offline inspection or for feeding to a PE/CLI
    /// wrapper in a downstream pipeline stage.
    ///
    /// # Errors
    ///
    /// Returns `PackageError` if compilation or the file write fails.
    pub fn write_cil_file(
        &self,
        source: &str,
        output_path: impl AsRef<Path>,
    ) -> Result<PackageResult, PackageError> {
        let mut result = self.compile_source(source)?;
        let path = output_path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PackageError::new("write", e.to_string()))?;
        }
        std::fs::write(path, &result.cil_bytes)
            .map_err(|e| PackageError::new("write", e.to_string()))?;
        result.assembly_path = Some(path.to_path_buf());
        Ok(result)
    }
}

// ===========================================================================
// Free-function convenience wrappers
// ===========================================================================

/// Compile Nib source to a CIL artifact using default settings.
///
/// Equivalent to `NibClrCompiler::default().compile_source(source)`.
///
/// # Example
///
/// ```
/// use nib_clr_compiler::compile_source;
///
/// let result = compile_source("fn main() { let x: u4 = 7; }").unwrap();
/// assert!(!result.cil_bytes.is_empty());
/// assert_eq!(result.assembly_name, "NibProgram");
/// ```
pub fn compile_source(source: &str) -> Result<PackageResult, PackageError> {
    NibClrCompiler::default().compile_source(source)
}

/// Alias for [`compile_source`] — mirrors the Python `pack_source` API.
pub fn pack_source(source: &str) -> Result<PackageResult, PackageError> {
    compile_source(source)
}

/// Compile and write raw CIL bytes to `output_path` using default settings.
pub fn write_cil_file(
    source: &str,
    output_path: impl AsRef<Path>,
) -> Result<PackageResult, PackageError> {
    NibClrCompiler::default().write_cil_file(source, output_path)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Standard Nib program used across tests: declares a u4 local and returns.
    const SIMPLE_NIB: &str = "fn main() { let x: u4 = 7; }";

    // ------------------------------------------------------------------
    // compile_source tests
    // ------------------------------------------------------------------

    /// Simple Nib program compiles without error and produces non-empty CIL.
    #[test]
    fn compile_source_returns_non_empty_cil() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert!(!result.cil_bytes.is_empty(), "CIL bytes should not be empty");
    }

    /// The raw and optimised IR must both have instructions.
    #[test]
    fn compile_source_produces_ir_stages() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert!(
            !result.raw_ir.instructions.is_empty(),
            "raw IR should have instructions"
        );
        assert!(
            !result.optimized_ir.instructions.is_empty(),
            "optimized IR should have instructions"
        );
    }

    /// The CIL artifact's entry label defaults to `"_start"`.
    #[test]
    fn compile_source_entry_label_is_start() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert_eq!(result.cil_artifact.entry_label, "_start");
    }

    /// Default assembly_name is `"NibProgram"`.
    #[test]
    fn compile_source_default_assembly_name() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert_eq!(result.assembly_name, "NibProgram");
    }

    /// Default type_name is `"NibProgram"`.
    #[test]
    fn compile_source_default_type_name() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert_eq!(result.type_name, "NibProgram");
    }

    /// The CIL artifact must have at least one method.
    #[test]
    fn compile_source_cil_artifact_has_methods() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert!(
            !result.cil_artifact.methods.is_empty(),
            "artifact should contain at least one method"
        );
    }

    /// The CIL body bytes end with `0x2A` (CIL `ret` opcode).
    ///
    /// Every well-formed CIL method ends with `ret` — the CLR verifier rejects
    /// any method body that can "fall off" the end.
    #[test]
    fn cil_body_ends_with_ret_opcode() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert_eq!(
            result.cil_bytes.last().copied(),
            Some(0x2A),
            "CIL body should end with ret (0x2A)"
        );
    }

    /// `pack_source` is an alias for `compile_source` and must produce identical output.
    #[test]
    fn pack_source_alias_matches_compile_source() {
        let compiled = compile_source(SIMPLE_NIB).unwrap();
        let packed = pack_source(SIMPLE_NIB).unwrap();
        assert_eq!(compiled.cil_bytes, packed.cil_bytes);
        assert_eq!(compiled.assembly_name, packed.assembly_name);
    }

    /// Custom assembly_name and type_name are propagated to the result.
    #[test]
    fn custom_names_are_propagated() {
        let result = NibClrCompiler {
            assembly_name: "MyNibAssembly".to_string(),
            type_name: "MyNibAssembly.Program".to_string(),
            ..Default::default()
        }
        .compile_source(SIMPLE_NIB)
        .unwrap();

        assert_eq!(result.assembly_name, "MyNibAssembly");
        assert_eq!(result.type_name, "MyNibAssembly.Program");
    }

    /// With `optimize_ir = false`, raw and optimised IR are identical.
    #[test]
    fn no_optimization_leaves_ir_unchanged() {
        let result = NibClrCompiler {
            optimize_ir: false,
            ..Default::default()
        }
        .compile_source(SIMPLE_NIB)
        .unwrap();

        assert_eq!(
            result.raw_ir.instructions.len(),
            result.optimized_ir.instructions.len(),
            "IR should be identical when optimization is disabled"
        );
    }

    /// A program with multiple variable bindings compiles successfully.
    #[test]
    fn multi_variable_program_compiles() {
        let result = compile_source("fn main() { let a: u4 = 3; let b: u4 = 4; }").unwrap();
        assert!(!result.cil_bytes.is_empty());
    }

    /// Source is captured verbatim in the result.
    #[test]
    fn source_field_is_captured_verbatim() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        assert_eq!(result.source, SIMPLE_NIB);
    }

    /// The typed_ast is populated (type checker ran successfully).
    #[test]
    fn typed_ast_is_populated() {
        let result = compile_source(SIMPLE_NIB).unwrap();
        // TypedAst always has a root GrammarASTNode — check it has children.
        assert!(!result.typed_ast.root.children.is_empty());
    }

    // ------------------------------------------------------------------
    // write_cil_file tests
    // ------------------------------------------------------------------

    /// `write_cil_file` creates the output file with the correct bytes.
    #[test]
    fn write_cil_file_creates_output() {
        let dir = std::env::temp_dir()
            .join(format!("nib-clr-test-{}", std::process::id()));
        let output_path = dir.join("program.cil");
        let result = write_cil_file(SIMPLE_NIB, &output_path).unwrap();

        assert!(output_path.exists(), "output file should exist");
        let on_disk = std::fs::read(&output_path).unwrap();
        assert_eq!(on_disk, result.cil_bytes, "file content should match cil_bytes");
        assert_eq!(result.assembly_path.as_deref(), Some(output_path.as_path()));

        let _ = std::fs::remove_dir_all(dir);
    }

    // ------------------------------------------------------------------
    // Error handling tests
    // ------------------------------------------------------------------

    /// Truncated `fn` keyword must produce a `"parse"` stage error.
    #[test]
    fn parse_errors_are_stage_labeled() {
        let err = compile_source("fn").unwrap_err();
        assert_eq!(err.stage, "parse", "stage should be 'parse'");
    }

    /// A type error (wrong operator on unsigned type) must produce a `"type-check"` error.
    #[test]
    fn type_errors_are_stage_labeled() {
        // `+%` is not a valid operator for `u4` in Nib — triggers a type error.
        let err = compile_source("fn main() { let x: bool = 1 +% 2; }").unwrap_err();
        assert_eq!(err.stage, "type-check");
    }

    /// `PackageError::Display` includes the stage label and message.
    #[test]
    fn package_error_display_includes_stage() {
        let err = PackageError::new("parse", "unexpected EOF");
        assert!(err.to_string().contains("parse"));
        assert!(err.to_string().contains("unexpected EOF"));
    }

    // ------------------------------------------------------------------
    // CLR validation + simulator integration
    // ------------------------------------------------------------------

    /// The optimised IR for a Nib program must pass CLR validation.
    ///
    /// `validate_for_clr` checks that all `IrOp`s are within the CLR
    /// backend's supported subset.  An empty error list means the IR is
    /// safe to lower to CIL bytecode.
    #[test]
    fn compile_source_cil_validates_for_clr() {
        use ir_to_cil_bytecode::validate_for_clr;

        let result = compile_source(SIMPLE_NIB).unwrap();
        let errors = validate_for_clr(&result.optimized_ir);
        assert!(
            errors.is_empty(),
            "CLR validation should pass for valid Nib program, got: {:?}",
            errors
        );
    }

    /// The CLR simulator can execute hand-assembled arithmetic CIL bytecode.
    ///
    /// Nib programs emit `call` instructions for some operations; the CLR
    /// simulator (a teaching tool) does not implement `call`.  This test
    /// verifies the integration between the `nib-clr-compiler` crate and
    /// `clr-simulator` by running a minimal arithmetic program directly.
    ///
    /// Program: `ldc.i4.s 10 ; ldc.i4.s 5 ; sub ; stloc.0 ; ldloc.0 ; ret`
    /// Expected: locals[0] = 5.
    #[test]
    fn clr_simulator_runs_direct_arithmetic_bytecode() {
        use clr_simulator::{
            CLRSimulator, OP_RET, OP_SUB, assemble_clr, encode_ldc_i4, encode_ldloc, encode_stloc,
        };

        let prog = assemble_clr(&[
            encode_ldc_i4(10),
            encode_ldc_i4(5),
            vec![OP_SUB],
            encode_stloc(0),
            encode_ldloc(0),
            vec![OP_RET],
        ]);

        let mut sim = CLRSimulator::new();
        sim.load(&prog, 1);
        let traces = sim.run(100);

        assert!(!traces.is_empty(), "simulator should produce trace steps");
        assert!(sim.halted, "simulator should halt on ret");
        assert_eq!(sim.locals[0], Some(5), "10 - 5 should equal 5");
    }
}
