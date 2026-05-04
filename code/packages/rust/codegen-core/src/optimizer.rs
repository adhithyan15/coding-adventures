//! Optimizer traits and concrete implementations.
//!
//! An `Optimizer<IR>` transforms an IR value into a (hopefully smaller or
//! faster) equivalent IR value without changing the program's observable
//! semantics.
//!
//! ## Two concrete optimizers
//!
//! ```text
//! CIROptimizer        — re-exported from jit-core
//!   Input:  Vec<CIRInstr>
//!   Output: Vec<CIRInstr>
//!   Passes: constant folding + dead-code elimination
//!
//! IrProgramOptimizer  — defined here, wraps ir-optimizer::IrOptimizer
//!   Input:  IrProgram
//!   Output: IrProgram
//!   Passes: NOP stripping (IrOp::Nop removal)
//! ```
//!
//! ## The `Optimizer<IR>` trait
//!
//! The trait is generic over IR so that `CodegenPipeline<IR>` can hold
//! an `Option<Box<dyn Optimizer<IR>>>` without knowing the concrete type.
//!
//! ```rust
//! use codegen_core::optimizer::{Optimizer, IrProgramOptimizer};
//! use compiler_ir::{IrProgram, IrInstruction};
//! use compiler_ir::opcodes::IrOp;
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(IrOp::Nop, vec![], 0));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
//!
//! let opt = IrProgramOptimizer;
//! let result = opt.optimize(prog);
//! // Nop was removed; only Halt remains.
//! assert_eq!(result.instructions.len(), 1);
//! ```

use compiler_ir::IrProgram;
use ir_optimizer::IrOptimizer;

// ── Optimizer trait ───────────────────────────────────────────────────────────

/// A pass that transforms `IR` → `IR` without changing observable semantics.
///
/// Implementors must be `Send + Sync` so they can be used inside
/// `Arc<dyn Optimizer<IR>>` in a multi-threaded compilation pipeline.
pub trait Optimizer<IR>: Send + Sync {
    /// Transform `ir` and return the optimized result.
    ///
    /// The input is taken by value because optimization often rewrites the
    /// entire IR rather than mutating it in place.  The output has the same
    /// type as the input.
    fn optimize(&self, ir: IR) -> IR;
}

// ── CIROptimizer is already in jit-core ──────────────────────────────────────
//
// We re-export it at the crate root (see lib.rs) so callers can write
// `use codegen_core::CIROptimizer` instead of `use jit_core::optimizer::CIROptimizer`.
//
// We also implement the Optimizer<Vec<CIRInstr>> trait for it here.

use jit_core::cir::CIRInstr;
use jit_core::optimizer::CIROptimizer;

/// `CIROptimizer` implements `Optimizer<Vec<CIRInstr>>` so it can be used
/// inside a `CodegenPipeline<Vec<CIRInstr>>`.
///
/// The implementation delegates to `CIROptimizer::run()`, which performs
/// constant folding and dead-code elimination.
///
/// ```rust
/// use codegen_core::optimizer::Optimizer;
/// use codegen_core::{CIRInstr, CIROptimizer};
///
/// let opt = CIROptimizer;
/// // Use ret_void — it has no destination so DCE never eliminates it.
/// let instrs = vec![CIRInstr::new("ret_void", None::<&str>, vec![], "void")];
/// let out = opt.optimize(instrs);
/// assert!(!out.is_empty());
/// ```
impl Optimizer<Vec<CIRInstr>> for CIROptimizer {
    fn optimize(&self, ir: Vec<CIRInstr>) -> Vec<CIRInstr> {
        self.run(ir)
    }
}

// ── IrProgramOptimizer ────────────────────────────────────────────────────────

/// A thin wrapper around `ir_optimizer::IrOptimizer` that implements
/// `Optimizer<IrProgram>`.
///
/// This is the compiled-language path optimizer (Nib, Brainfuck, Algol-60).
/// It currently strips `IrOp::Nop` instructions; future passes will add
/// constant folding and peephole optimizations.
///
/// ## Why wrap instead of using `IrOptimizer` directly?
///
/// `IrOptimizer` does not implement the `Optimizer<IrProgram>` trait from
/// this crate.  The wrapper provides the glue without modifying the
/// `ir-optimizer` crate.
pub struct IrProgramOptimizer;

impl Optimizer<IrProgram> for IrProgramOptimizer {
    /// Optimize `ir` by removing `Nop` instructions.
    ///
    /// ```rust
    /// use codegen_core::optimizer::{Optimizer, IrProgramOptimizer};
    /// use compiler_ir::{IrProgram, IrInstruction};
    /// use compiler_ir::opcodes::IrOp;
    ///
    /// let mut prog = IrProgram::new("_start");
    /// prog.add_instruction(IrInstruction::new(IrOp::Nop, vec![], 0));
    /// prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
    ///
    /// let result = IrProgramOptimizer.optimize(prog);
    /// assert_eq!(result.instructions.len(), 1);
    /// assert_eq!(result.instructions[0].opcode, IrOp::Halt);
    /// ```
    fn optimize(&self, ir: IrProgram) -> IrProgram {
        IrOptimizer.optimize(&ir)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrInstruction, IrProgram};
    use compiler_ir::opcodes::IrOp;
    use jit_core::cir::CIROperand;

    // Test 1: CIROptimizer implements Optimizer<Vec<CIRInstr>>
    // Use ret_void so DCE doesn't eliminate it (void instructions are always "live").
    #[test]
    fn cir_optimizer_trait() {
        let opt = CIROptimizer;
        let instrs = vec![CIRInstr::new("ret_void", None::<&str>, vec![], "void")];
        let out: Vec<CIRInstr> = opt.optimize(instrs);
        assert!(!out.is_empty());
    }

    // Test 2: CIROptimizer is identity on non-reducible instructions
    #[test]
    fn cir_optimizer_passthrough() {
        let opt = CIROptimizer;
        let instrs = vec![
            CIRInstr::new("label", None::<&str>, vec![CIROperand::Var("loop".into())], "void"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let out = opt.optimize(instrs);
        assert_eq!(out.len(), 2);
    }

    // Test 3: IrProgramOptimizer strips NOPs
    #[test]
    fn ir_optimizer_strips_nops() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(IrOp::Nop, vec![], 0));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let result = IrProgramOptimizer.optimize(prog);
        assert_eq!(result.instructions.len(), 1);
        assert_eq!(result.instructions[0].opcode, IrOp::Halt);
    }

    // Test 4: IrProgramOptimizer is identity on programs with no NOPs
    #[test]
    fn ir_optimizer_passthrough_no_nops() {
        let mut prog = IrProgram::new("main");
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        let result = IrProgramOptimizer.optimize(prog);
        assert_eq!(result.instructions.len(), 1);
    }

    // Test 5: IrProgramOptimizer preserves entry_label
    #[test]
    fn ir_optimizer_preserves_entry() {
        let prog = IrProgram::new("my_entry");
        let result = IrProgramOptimizer.optimize(prog);
        assert_eq!(result.entry_label, "my_entry");
    }
}
