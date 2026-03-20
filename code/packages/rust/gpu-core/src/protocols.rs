//! Protocols -- the pluggable interfaces that make this core vendor-agnostic.
//!
//! # Why traits?
//!
//! Every GPU vendor (NVIDIA, AMD, Intel, ARM) and every accelerator type (GPU,
//! TPU, NPU) has a processing element at its heart. They all do the same basic
//! thing: compute floating-point operations. But the details differ:
//!
//! ```text
//! NVIDIA CUDA Core:     FP32 ALU + 255 registers + PTX instructions
//! AMD Stream Processor: FP32 ALU + 256 VGPRs + GCN instructions
//! Intel Vector Engine:  SIMD8 ALU + GRF + Xe instructions
//! ARM Mali Exec Engine: FP32 ALU + register bank + Mali instructions
//! TPU Processing Element: MAC unit + weight register + accumulator
//! NPU MAC Unit:         MAC + activation function + buffer
//! ```
//!
//! Instead of building separate simulators for each, we define two traits:
//!
//! 1. **ProcessingElement** -- the generic "any compute unit" interface
//! 2. **InstructionSet** -- the pluggable "how to decode and execute instructions"
//!
//! Any vendor-specific implementation just needs to satisfy these traits.
//! The core simulation infrastructure (registers, memory, tracing) is reused.
//!
//! # What is a trait?
//!
//! In Rust, a trait is like an interface in Java or Go, or a Protocol in Python.
//! It says "any type that implements these methods can be used here."
//!
//! ```text
//! trait Flyable {
//!     fn fly(&self);
//! }
//!
//! struct Bird;
//! impl Flyable for Bird {
//!     fn fly(&self) { println!("flap flap"); }
//! }
//!
//! struct Airplane;
//! impl Flyable for Airplane {
//!     fn fly(&self) { println!("zoom"); }
//! }
//! // Both Bird and Airplane satisfy Flyable!
//! ```

use std::collections::HashMap;

use crate::memory::LocalMemory;
use crate::opcodes::Instruction;
use crate::registers::FPRegisterFile;
use crate::trace::GPUCoreTrace;

// ---------------------------------------------------------------------------
// ExecuteResult -- what an instruction execution produces
// ---------------------------------------------------------------------------

/// The outcome of executing a single instruction.
///
/// This is what the [`InstructionSet`]'s `execute()` method returns. It tells the
/// core what changed so the core can build a complete execution trace.
///
/// # Fields
///
/// - `description`: Human-readable summary, e.g. "R3 = R1 * R2 = 6.0"
/// - `next_pc_offset`: How to advance the program counter.
///    +1 for most instructions (next instruction).
///    Other values for branches/jumps.
/// - `absolute_jump`: If true, `next_pc_offset` is an absolute address,
///    not a relative offset.
/// - `registers_changed`: Map of register name to new float value.
/// - `memory_changed`: Map of memory address to new float value.
/// - `halted`: True if this instruction stops execution.
#[derive(Debug, Clone)]
pub struct ExecuteResult {
    pub description: String,
    pub next_pc_offset: i64,
    pub absolute_jump: bool,
    pub registers_changed: Option<HashMap<String, f64>>,
    pub memory_changed: Option<HashMap<usize, f64>>,
    pub halted: bool,
}

impl ExecuteResult {
    /// Create a new ExecuteResult with default values.
    ///
    /// By default, the PC advances by 1, no jump occurs, no registers or
    /// memory change, and execution is not halted.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            next_pc_offset: 1,
            absolute_jump: false,
            registers_changed: None,
            memory_changed: None,
            halted: false,
        }
    }

    /// Builder method: set the next PC offset.
    pub fn with_next_pc_offset(mut self, offset: i64) -> Self {
        self.next_pc_offset = offset;
        self
    }

    /// Builder method: mark this as an absolute jump.
    pub fn with_absolute_jump(mut self, absolute: bool) -> Self {
        self.absolute_jump = absolute;
        self
    }

    /// Builder method: set registers that changed.
    pub fn with_registers_changed(mut self, regs: HashMap<String, f64>) -> Self {
        self.registers_changed = Some(regs);
        self
    }

    /// Builder method: set memory addresses that changed.
    pub fn with_memory_changed(mut self, mem: HashMap<usize, f64>) -> Self {
        self.memory_changed = Some(mem);
        self
    }

    /// Builder method: mark execution as halted.
    pub fn with_halted(mut self, halted: bool) -> Self {
        self.halted = halted;
        self
    }
}

// ---------------------------------------------------------------------------
// InstructionSet -- pluggable ISA (the key to vendor-agnosticism)
// ---------------------------------------------------------------------------

/// A pluggable instruction set that can be swapped to simulate any vendor.
///
/// # How it works
///
/// The GPUCore calls `isa.execute(instruction, registers, memory)` for each
/// instruction. The ISA implementation:
///
/// 1. Reads the opcode to determine what operation to perform
/// 2. Reads source registers and/or memory
/// 3. Performs the computation (using `fp_add`, `fp_mul`, `fp_fma`, etc.)
/// 4. Writes the result to the destination register and/or memory
/// 5. Returns an [`ExecuteResult`] describing what happened
///
/// # Implementing a new ISA
///
/// To add support for a new vendor (e.g., NVIDIA PTX):
///
/// ```text
/// struct PtxIsa;
///
/// impl InstructionSet for PtxIsa {
///     fn name(&self) -> &str { "PTX" }
///     fn execute(&self, inst: &Instruction, regs: &mut FPRegisterFile,
///                mem: &mut LocalMemory) -> ExecuteResult {
///         match inst.opcode {
///             // vendor-specific dispatch...
///         }
///     }
/// }
///
/// let core = GPUCore::new(Box::new(PtxIsa));
/// ```
pub trait InstructionSet {
    /// The ISA name, e.g. "Generic", "PTX", "GCN", "Xe", "Mali".
    fn name(&self) -> &str;

    /// Decode and execute a single instruction.
    ///
    /// # Arguments
    ///
    /// - `instruction`: The instruction to execute.
    /// - `registers`: The core's floating-point register file.
    /// - `memory`: The core's local scratchpad memory.
    ///
    /// # Returns
    ///
    /// An [`ExecuteResult`] describing what happened.
    fn execute(
        &self,
        instruction: &Instruction,
        registers: &mut FPRegisterFile,
        memory: &mut LocalMemory,
    ) -> ExecuteResult;
}

// ---------------------------------------------------------------------------
// ProcessingElement -- the most generic abstraction
// ---------------------------------------------------------------------------

/// Any compute unit in any accelerator.
///
/// This is the most generic interface -- a GPU core, a TPU processing element,
/// and an NPU MAC unit all satisfy this trait. It provides just enough
/// structure for a higher-level component (like a warp scheduler or systolic
/// array controller) to drive the PE.
///
/// # Why so minimal?
///
/// Different accelerators have radically different execution models:
///
/// - **GPUs**: instruction-stream + register file (step = execute one instruction)
/// - **TPUs**: dataflow, no instructions (step = one MAC + pass data to neighbor)
/// - **NPUs**: scheduled MACs (step = one MAC from the scheduler's queue)
///
/// This trait captures only what they ALL share: the ability to advance
/// one cycle, check if done, and reset.
pub trait ProcessingElement {
    /// Execute one cycle. Returns a trace of what happened.
    fn step(&mut self) -> Result<GPUCoreTrace, String>;

    /// True if this PE has finished execution.
    fn halted(&self) -> bool;

    /// Reset to initial state.
    fn reset(&mut self);
}
