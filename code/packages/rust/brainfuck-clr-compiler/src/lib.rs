//! # brainfuck-clr-compiler
//!
//! End-to-end compiler from Brainfuck source text to CLR CIL bytecode.
//!
//! ## Pipeline
//!
//! ```text
//! Brainfuck source
//!   → brainfuck::parse_brainfuck()         — lex + parse BF tokens
//!   → brainfuck_ir_compiler::compile()     — emit target-independent IrProgram
//!   → ir_optimizer::optimize_program()     — constant fold, dead-code elimination
//!   → ir_to_cil_bytecode::lower_ir_to_cil_bytecode()
//!                                          — emit CIL method body bytes
//!   → CILProgramArtifact                   — structured CIL artifact (name + body + locals)
//! ```
//!
//! The CIL output can be:
//! - Wrapped in a PE/CLI assembly by `cli-assembly-writer` (future)
//! - Executed directly by [`clr_simulator::CLRSimulator`] for testing
//!
//! ## Quick start
//!
//! ```no_run
//! use brainfuck_clr_compiler::compile_source;
//!
//! let result = compile_source("+.").unwrap();
//! assert!(!result.cil_bytes.is_empty());
//! assert_eq!(result.assembly_name, "BrainfuckProgram");
//! ```
//!
//! ## Error handling
//!
//! Every stage is labelled so callers know which part of the pipeline failed:
//!
//! ```no_run
//! use brainfuck_clr_compiler::{compile_source, PackageError};
//!
//! let err = compile_source("[").unwrap_err();
//! assert_eq!(err.stage, "parse");
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use brainfuck::parser::parse_brainfuck;
use brainfuck_ir_compiler::{compile, release_config, BuildConfig};
use compiler_ir::IrProgram;
use ir_optimizer::optimize_program;
use ir_to_cil_bytecode::{lower_ir_to_cil_bytecode, CILBackendConfig, CILProgramArtifact};
use parser::grammar_parser::GrammarASTNode;

// ===========================================================================
// Public types
// ===========================================================================

/// The result of compiling a Brainfuck program to CIL bytecode.
///
/// Every intermediate artifact is retained so callers can inspect any stage
/// of the pipeline for debugging or visualisation.
///
/// Note: `Debug` is implemented manually because [`CILProgramArtifact`] contains
/// a `Box<dyn CILTokenProvider>` which does not implement `Debug`.
pub struct PackageResult {
    /// Original Brainfuck source code.
    pub source: String,
    /// Logical filename used in error messages (e.g., `"program.bf"`).
    pub filename: String,
    /// Assembly name embedded in metadata (e.g., `"BrainfuckProgram"`).
    pub assembly_name: String,
    /// Parsed grammar AST node from the Brainfuck parser.
    pub ast: GrammarASTNode,
    /// IrProgram as emitted by `brainfuck-ir-compiler`, before optimisation.
    pub raw_ir: IrProgram,
    /// IrProgram after the IR optimiser has run (constant fold + DCE).
    pub optimized_ir: IrProgram,
    /// Structured CIL artifact: entry label, method bodies, local variable types.
    pub cil_artifact: CILProgramArtifact,
    /// Raw CIL method body bytes for the entry method.
    ///
    /// This is a convenience copy of `cil_artifact.methods[0].body` — the
    /// bytes that can be fed directly to `clr_simulator::CLRSimulator::load`.
    pub cil_bytes: Vec<u8>,
    /// Path where the assembly was written, if [`BrainfuckClrCompiler::write_cil_file`] was used.
    pub assembly_path: Option<PathBuf>,
}

// Manual Debug because CILProgramArtifact contains Box<dyn CILTokenProvider> which lacks Debug.
impl fmt::Debug for PackageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageResult")
            .field("source", &self.source)
            .field("filename", &self.filename)
            .field("assembly_name", &self.assembly_name)
            .field("cil_bytes_len", &self.cil_bytes.len())
            .field("assembly_path", &self.assembly_path)
            .finish_non_exhaustive()
    }
}

/// Result of compiling *and* executing a Brainfuck program on the CLR simulator.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Full compilation artifact.
    pub compilation: PackageResult,
    /// The value left on the CLR stack after the entry method returns, if any.
    ///
    /// For arithmetic Brainfuck programs (`+`, `-`, `*`, `/`) this is the
    /// computed integer result.  For programs that use only I/O (`.`, `,`)
    /// the result is typically `None` or `Some(0)`.
    pub return_value: Option<i32>,
}

/// An error produced during the compilation pipeline.
///
/// The `stage` field identifies which step failed so callers can give
/// targeted diagnostics without parsing the message string.
///
/// | `stage`         | Cause |
/// |-----------------|-------|
/// | `"parse"`       | Malformed Brainfuck syntax (e.g., unmatched `[`) |
/// | `"ir-compile"`  | IR emission failed (should be unreachable for valid BF) |
/// | `"lower-cil"`   | CIL lowering error |
/// | `"write"`       | I/O error writing the output file |
/// | `"execute"`     | CLR simulator error |
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
// BrainfuckClrCompiler — the pipeline orchestrator
// ===========================================================================

/// End-to-end Brainfuck → CLR CIL compiler.
///
/// Chains the BF parser, IR compiler, IR optimiser, and CIL backend into a
/// single composable object.  All configuration lives on the struct; the
/// same `BrainfuckClrCompiler` instance can compile many programs without
/// re-creating the pipeline.
///
/// # Default configuration
///
/// ```
/// use brainfuck_clr_compiler::BrainfuckClrCompiler;
///
/// let compiler = BrainfuckClrCompiler::default();
/// assert_eq!(compiler.filename, "program.bf");
/// assert_eq!(compiler.assembly_name, "BrainfuckProgram");
/// assert!(compiler.optimize_ir);
/// ```
#[derive(Debug, Clone)]
pub struct BrainfuckClrCompiler {
    /// Filename used in parse error messages (does not need to exist on disk).
    pub filename: String,
    /// Assembly / class name emitted in the CIL metadata header.
    pub assembly_name: String,
    /// Type name for the CLR class that contains the entry method.
    pub type_name: String,
    /// Build configuration for the IR compiler (defaults to `release_config()`).
    pub build_config: Option<BuildConfig>,
    /// Whether to run the IR optimiser (dead-code elimination + constant folding).
    pub optimize_ir: bool,
    /// CIL backend configuration; `None` uses the default (`CILBackendConfig` with syscall arg reg 4).
    pub cil_config: Option<CILBackendConfig>,
}

impl Default for BrainfuckClrCompiler {
    /// Build a compiler with sensible defaults.
    ///
    /// - `filename` = `"program.bf"` — appears in parse error messages.
    /// - `assembly_name` = `"BrainfuckProgram"` — CLR assembly name.
    /// - `type_name` = `"BrainfuckProgram"` — CLR type name.
    /// - `optimize_ir` = `true` — enables constant folding + dead-code elimination.
    fn default() -> Self {
        Self {
            filename: "program.bf".to_string(),
            assembly_name: "BrainfuckProgram".to_string(),
            type_name: "BrainfuckProgram".to_string(),
            build_config: None,
            optimize_ir: true,
            cil_config: None,
        }
    }
}

impl BrainfuckClrCompiler {
    // -----------------------------------------------------------------------
    // compile_source
    // -----------------------------------------------------------------------

    /// Compile Brainfuck source to a CIL artifact.
    ///
    /// Runs all four pipeline stages in order:
    ///
    /// 1. **Parse** — `brainfuck::parse_brainfuck`
    /// 2. **IR compile** — `brainfuck_ir_compiler::compile`
    /// 3. **Optimise** — `ir_optimizer::optimize_program` (if `self.optimize_ir`)
    /// 4. **CIL lower** — `ir_to_cil_bytecode::lower_ir_to_cil_bytecode`
    ///
    /// # Errors
    ///
    /// Returns a [`PackageError`] if any stage fails.  Check `err.stage` to
    /// identify the failing step.
    ///
    /// # Example
    ///
    /// ```
    /// use brainfuck_clr_compiler::BrainfuckClrCompiler;
    ///
    /// let result = BrainfuckClrCompiler::default().compile_source("+.").unwrap();
    /// assert!(!result.cil_bytes.is_empty());
    /// ```
    pub fn compile_source(&self, source: &str) -> Result<PackageResult, PackageError> {
        // ── Stage 1: Parse ─────────────────────────────────────────────────
        //
        // The Brainfuck parser turns the raw characters into a grammar AST.
        // Only `[` `]` `+` `-` `<` `>` `.` `,` are meaningful; all other
        // characters are treated as comments and silently ignored.
        let ast = parse_brainfuck(source)
            .map_err(|err| PackageError::new("parse", err.to_string()))?;

        // ── Stage 2: IR compile ────────────────────────────────────────────
        //
        // Convert the BF AST to a target-independent `IrProgram`.  The
        // `release_config()` enables all optimisations (loop unrolling, bulk
        // clear detection) for compact IR output.
        let config = self.build_config.clone().unwrap_or_else(release_config);
        let ir_result = compile(&ast, &self.filename, config)
            .map_err(|err| PackageError::new("ir-compile", err))?;
        let raw_ir = ir_result.program;

        // ── Stage 3: IR optimise ───────────────────────────────────────────
        //
        // Run constant folding and dead-code elimination over the IrProgram.
        // Skipped when `optimize_ir = false` (useful for debugging to see
        // the raw IR output).
        let optimized_ir = if self.optimize_ir {
            optimize_program(&raw_ir)
        } else {
            raw_ir.clone()
        };

        // ── Stage 4: CIL lower ─────────────────────────────────────────────
        //
        // Translate the IrProgram into CIL method body bytes.  The CIL
        // backend uses syscall_arg_reg=4 by default (matching the Python
        // `CILBackendConfig(syscall_arg_reg=4)` default).
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
            filename: self.filename.clone(),
            assembly_name: self.assembly_name.clone(),
            ast,
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
    /// The output file contains the raw method body bytes from the entry
    /// method.  This is useful for offline inspection (e.g. with `ildasm`)
    /// or for feeding to a PE wrapper in a downstream pipeline stage.
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

/// Compile Brainfuck source to a CIL artifact using default settings.
///
/// Equivalent to `BrainfuckClrCompiler::default().compile_source(source)`.
///
/// # Example
///
/// ```
/// use brainfuck_clr_compiler::compile_source;
///
/// let result = compile_source("+.").unwrap();
/// assert!(!result.cil_bytes.is_empty());
/// assert_eq!(result.assembly_name, "BrainfuckProgram");
/// ```
pub fn compile_source(source: &str) -> Result<PackageResult, PackageError> {
    BrainfuckClrCompiler::default().compile_source(source)
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
    BrainfuckClrCompiler::default().write_cil_file(source, output_path)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // compile_source tests
    // ------------------------------------------------------------------

    /// Simple BF program compiles without error and produces non-empty CIL.
    #[test]
    fn compile_source_returns_non_empty_cil() {
        let result = compile_source("+.").unwrap();
        assert!(!result.cil_bytes.is_empty(), "CIL bytes should not be empty");
    }

    /// The raw IR must have instructions before and after optimisation.
    #[test]
    fn compile_source_produces_ir_stages() {
        let result = compile_source("+.").unwrap();
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
        let result = compile_source("+.").unwrap();
        assert_eq!(result.cil_artifact.entry_label, "_start");
    }

    /// Default assembly_name is `"BrainfuckProgram"`.
    #[test]
    fn compile_source_default_assembly_name() {
        let result = compile_source("+.").unwrap();
        assert_eq!(result.assembly_name, "BrainfuckProgram");
    }

    /// Default filename is `"program.bf"`.
    #[test]
    fn compile_source_default_filename() {
        let result = compile_source("+.").unwrap();
        assert_eq!(result.filename, "program.bf");
    }

    /// The CIL artifact must have at least one method.
    #[test]
    fn compile_source_cil_artifact_has_methods() {
        let result = compile_source("+.").unwrap();
        assert!(
            !result.cil_artifact.methods.is_empty(),
            "artifact should contain at least one method"
        );
    }

    /// The CIL body bytes end with `0x2A` (CIL `ret` opcode).
    ///
    /// Every well-formed CIL method ends with `ret` — the CLR rejects any
    /// method that can "fall off" the end of its body.
    #[test]
    fn cil_body_ends_with_ret_opcode() {
        let result = compile_source("+.").unwrap();
        assert_eq!(
            result.cil_bytes.last().copied(),
            Some(0x2A),
            "CIL body should end with ret (0x2A)"
        );
    }

    /// pack_source is an alias for compile_source and must produce identical output.
    #[test]
    fn pack_source_matches_compile_source() {
        let compiled = compile_source("+.").unwrap();
        let packed = pack_source("+.").unwrap();
        assert_eq!(compiled.cil_bytes, packed.cil_bytes);
        assert_eq!(compiled.assembly_name, packed.assembly_name);
    }

    /// Custom assembly_name and type_name are propagated.
    #[test]
    fn custom_assembly_and_type_names_are_propagated() {
        let result = BrainfuckClrCompiler {
            filename: "hello.bf".to_string(),
            assembly_name: "HelloBrainfuck".to_string(),
            type_name: "demo.HelloBrainfuck".to_string(),
            ..Default::default()
        }
        .compile_source("+.")
        .unwrap();

        assert_eq!(result.filename, "hello.bf");
        assert_eq!(result.assembly_name, "HelloBrainfuck");
    }

    /// With `optimize_ir = false`, raw and optimised IR are identical.
    #[test]
    fn no_optimization_leaves_ir_unchanged() {
        let result = BrainfuckClrCompiler {
            optimize_ir: false,
            ..Default::default()
        }
        .compile_source("+.")
        .unwrap();

        assert_eq!(
            result.raw_ir.instructions.len(),
            result.optimized_ir.instructions.len(),
            "IR should be identical when optimization is disabled"
        );
    }

    /// A loop `[+]` compiles successfully — tests forward-reference resolution.
    #[test]
    fn loop_program_compiles_successfully() {
        let result = compile_source("[+]").unwrap();
        assert!(!result.cil_bytes.is_empty());
    }

    /// A larger program (65 increments + output) compiles correctly.
    #[test]
    fn larger_program_compiles_correctly() {
        let source = "+".repeat(65) + ".";
        let result = compile_source(&source).unwrap();
        assert!(!result.cil_bytes.is_empty());
    }

    /// Source is captured verbatim on the result.
    #[test]
    fn source_field_is_captured_verbatim() {
        let source = "+++.";
        let result = compile_source(source).unwrap();
        assert_eq!(result.source, source);
    }

    // ------------------------------------------------------------------
    // write_cil_file tests
    // ------------------------------------------------------------------

    /// `write_cil_file` creates the output file with the correct bytes.
    #[test]
    fn write_cil_file_creates_output() {
        let dir = std::env::temp_dir()
            .join(format!("brainfuck-clr-test-{}", std::process::id()));
        let output_path = dir.join("program.cil");
        let result = write_cil_file("+.", &output_path).unwrap();

        assert!(output_path.exists(), "output file should exist");
        let on_disk = std::fs::read(&output_path).unwrap();
        assert_eq!(on_disk, result.cil_bytes, "file content should match cil_bytes");
        assert_eq!(result.assembly_path.as_deref(), Some(output_path.as_path()));

        let _ = std::fs::remove_dir_all(dir);
    }

    // ------------------------------------------------------------------
    // Error handling tests
    // ------------------------------------------------------------------

    /// Unmatched `[` must produce a `"parse"` stage error.
    #[test]
    fn unmatched_open_bracket_is_parse_error() {
        let err = compile_source("[").unwrap_err();
        assert_eq!(err.stage, "parse", "stage should be 'parse'");
    }

    /// Unmatched `]` must also produce a `"parse"` stage error.
    #[test]
    fn unmatched_close_bracket_is_parse_error() {
        let err = compile_source("]").unwrap_err();
        assert_eq!(err.stage, "parse");
    }

    /// `PackageError::Display` includes the stage label.
    #[test]
    fn package_error_display_includes_stage() {
        let err = PackageError::new("parse", "unexpected token");
        assert!(err.to_string().contains("parse"));
        assert!(err.to_string().contains("unexpected token"));
    }

    // ------------------------------------------------------------------
    // CLR validation + simulator integration
    // ------------------------------------------------------------------

    /// The optimised IR for a Brainfuck program must pass CLR validation.
    ///
    /// `validate_for_clr` checks that all `IrOp`s are within the CLR
    /// backend's supported subset.  An empty validation error list means
    /// the IR is safe to lower to CIL bytecode.
    #[test]
    fn compile_source_cil_validates_for_clr() {
        use ir_to_cil_bytecode::validate_for_clr;

        let result = compile_source("+++.").unwrap();
        let errors = validate_for_clr(&result.optimized_ir);
        assert!(
            errors.is_empty(),
            "CLR validation should pass for valid BF program, got: {:?}",
            errors
        );
    }

    /// The CLR simulator can execute hand-assembled arithmetic CIL bytecode.
    ///
    /// The BF-to-CIL pipeline emits `call` instructions for tape memory
    /// operations; the CLR simulator (a teaching tool) does not implement
    /// `call`.  This test verifies the integration layer between the
    /// `brainfuck-clr-compiler` crate and the `clr-simulator` crate by
    /// loading a minimal arithmetic program directly.
    ///
    /// Program: `ldc.i4.s 3 ; ldc.i4.s 4 ; add ; stloc.0 ; ldloc.0 ; ret`
    /// Expected: stack result = 7, locals[0] = 7.
    #[test]
    fn clr_simulator_runs_direct_arithmetic_bytecode() {
        use clr_simulator::{CLRSimulator, assemble_clr, encode_ldc_i4, encode_stloc, encode_ldloc, OP_ADD, OP_RET};

        let prog = assemble_clr(&[
            encode_ldc_i4(3),
            encode_ldc_i4(4),
            vec![OP_ADD],
            encode_stloc(0),
            encode_ldloc(0),
            vec![OP_RET],
        ]);

        let mut sim = CLRSimulator::new();
        sim.load(&prog, 1);
        let traces = sim.run(100);

        assert!(!traces.is_empty(), "simulator should produce trace steps");
        assert!(sim.halted, "simulator should halt on ret");
        assert_eq!(sim.locals[0], Some(7), "3 + 4 should equal 7");
    }

    /// An empty BF program (all comments) compiles to a trivial CIL body.
    #[test]
    fn empty_program_compiles_to_minimal_cil() {
        // All characters except +-<>.,[] are BF comments.
        let result = compile_source("this is a comment").unwrap();
        assert!(!result.cil_bytes.is_empty(), "even empty programs produce a CIL body (just ret)");
    }
}
