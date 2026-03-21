//! GPUCore -- the generic, pluggable accelerator processing element.
//!
//! # What is a GPU Core?
//!
//! A GPU core is the smallest independently programmable compute unit on a GPU.
//! It's like a tiny, simplified CPU that does one thing well: floating-point math.
//!
//! ```text
//! CPU Core (complex):                    GPU Core (simple):
//! +------------------------+             +----------------------+
//! | Branch predictor       |             |                      |
//! | Out-of-order engine    |             | In-order execution   |
//! | Large cache hierarchy  |             | Small register file  |
//! | Integer + FP ALUs      |             | FP ALU only          |
//! | Complex decoder        |             | Simple fetch-execute  |
//! | Speculative execution  |             | No speculation       |
//! +------------------------+             +----------------------+
//! ```
//!
//! A single GPU core is MUCH simpler than a CPU core. GPUs achieve performance
//! not through per-core complexity, but through massive parallelism: thousands
//! of these simple cores running in parallel.
//!
//! # How This Core is Pluggable
//!
//! The GPUCore takes a boxed [`InstructionSet`] as a constructor parameter. This
//! ISA object handles all the vendor-specific decode and execute logic:
//!
//! ```text
//! // Generic educational ISA (this package)
//! let core = GPUCore::new(Box::new(GenericISA));
//!
//! // NVIDIA PTX (future package)
//! let core = GPUCore::with_config(Box::new(PtxIsa), FP32, 255, 4096);
//!
//! // AMD GCN (future package)
//! let core = GPUCore::with_config(Box::new(GcnIsa), FP32, 256, 4096);
//! ```
//!
//! The core itself (fetch loop, registers, memory, tracing) stays the same.
//! Only the ISA changes.
//!
//! # Execution Model
//!
//! The GPU core uses a simple fetch-execute loop (no separate decode stage):
//!
//! ```text
//! +-------------------------------------------+
//! |              GPU Core                      |
//! |                                            |
//! |  +---------+    +-----------------+        |
//! |  | Program |---→|   Fetch         |        |
//! |  | Memory  |    |   instruction   |        |
//! |  +---------+    |   at PC         |        |
//! |                 +-------+---------+        |
//! |                         |                  |
//! |                 +-------v---------+        |
//! |  +-----------+  |   ISA.execute() |        |
//! |  | Register  |<-|   (pluggable!)  |-->Trace|
//! |  | File      |->|                 |        |
//! |  +-----------+  +-------+---------+        |
//! |                         |                  |
//! |  +-----------+  +-------v---------+        |
//! |  |  Local   |<--|  Update PC      |        |
//! |  |  Memory  |   +-----------------+        |
//! |  +-----------+                             |
//! +-------------------------------------------+
//! ```
//!
//! Each `step()`:
//! 1. **Fetch**: read instruction at `program[PC]`
//! 2. **Execute**: call `isa.execute(instruction, registers, memory)`
//! 3. **Update PC**: advance based on ExecuteResult (branch or +1)
//! 4. **Return trace**: GPUCoreTrace with full execution details

use fp_arithmetic::{FloatFormat, FP32};

use crate::generic_isa::GenericISA;
use crate::memory::LocalMemory;
use crate::opcodes::Instruction;
use crate::protocols::{InstructionSet, ProcessingElement};
use crate::registers::FPRegisterFile;
use crate::trace::GPUCoreTrace;

/// A generic GPU processing element with a pluggable instruction set.
///
/// This is the central struct of the package. It simulates a single
/// processing element -- one CUDA core, one AMD stream processor, one
/// Intel vector engine, or one ARM Mali execution engine -- depending
/// on which [`InstructionSet`] you plug in.
///
/// # Example
///
/// ```
/// use gpu_core::{GPUCore, GenericISA};
/// use gpu_core::opcodes::{limm, fmul, halt};
///
/// let mut core = GPUCore::new(Box::new(GenericISA));
/// core.load_program(vec![
///     limm(0, 3.0),
///     limm(1, 4.0),
///     fmul(2, 0, 1),
///     halt(),
/// ]);
/// let traces = core.run(10000).unwrap();
/// assert_eq!(core.registers.read_float(2), 12.0);
/// ```
pub struct GPUCore {
    /// The pluggable instruction set.
    isa: Box<dyn InstructionSet>,
    /// The floating-point format for registers.
    pub fmt: FloatFormat,
    /// The core's floating-point register file.
    pub registers: FPRegisterFile,
    /// The core's local scratchpad memory.
    pub memory: LocalMemory,
    /// Program counter: index into the loaded program.
    pub pc: usize,
    /// Clock cycle counter (1-indexed after first step).
    pub cycle: u64,
    /// Whether the core has executed a HALT instruction.
    halted: bool,
    /// The loaded program (list of instructions).
    program: Vec<Instruction>,
    /// Configuration: number of registers (stored for reset).
    num_registers: usize,
    /// Configuration: memory size (stored for reset).
    memory_size: usize,
}

impl GPUCore {
    /// Create a new GPU core with default configuration.
    ///
    /// Uses FP32 format, 32 registers, and 4096 bytes of local memory.
    pub fn new(isa: Box<dyn InstructionSet>) -> Self {
        Self::with_config(isa, FP32, 32, 4096)
    }

    /// Create a new GPU core with custom configuration.
    ///
    /// # Arguments
    ///
    /// - `isa`: The instruction set to use.
    /// - `fmt`: Floating-point format for registers (FP32, FP16, BF16).
    /// - `num_registers`: Number of FP registers (1-256).
    /// - `memory_size`: Local memory size in bytes.
    pub fn with_config(
        isa: Box<dyn InstructionSet>,
        fmt: FloatFormat,
        num_registers: usize,
        memory_size: usize,
    ) -> Self {
        Self {
            isa,
            fmt,
            registers: FPRegisterFile::new(num_registers, fmt),
            memory: LocalMemory::with_format(memory_size, fmt),
            pc: 0,
            cycle: 0,
            halted: false,
            program: Vec::new(),
            num_registers,
            memory_size,
        }
    }

    /// Load a program (list of instructions) into the core.
    ///
    /// This replaces any previously loaded program and resets the PC to 0,
    /// but does NOT reset registers or memory. Call `reset()` for a full reset.
    pub fn load_program(&mut self, program: Vec<Instruction>) {
        self.program = program;
        self.pc = 0;
        self.halted = false;
        self.cycle = 0;
    }

    /// Execute the program until HALT or max_steps reached.
    ///
    /// This repeatedly calls `step()` until the core halts or the step
    /// limit is reached (preventing infinite loops from hanging).
    ///
    /// # Errors
    ///
    /// Returns an error if `max_steps` is exceeded (likely an infinite loop)
    /// or if `step()` fails.
    pub fn run(&mut self, max_steps: usize) -> Result<Vec<GPUCoreTrace>, String> {
        let mut traces = Vec::new();
        let mut steps = 0;

        while !self.halted && steps < max_steps {
            traces.push(self.step()?);
            steps += 1;
        }

        if !self.halted && steps >= max_steps {
            return Err(format!(
                "Execution limit reached ({} steps). Possible infinite loop. Last PC={}",
                max_steps, self.pc
            ));
        }

        Ok(traces)
    }

    /// Get the ISA name.
    pub fn isa_name(&self) -> &str {
        self.isa.name()
    }
}

impl ProcessingElement for GPUCore {
    /// Execute one instruction and return a trace of what happened.
    ///
    /// This is the core fetch-execute loop:
    /// 1. Check if halted or PC out of range
    /// 2. Fetch instruction at PC
    /// 3. Call ISA.execute() to perform the operation
    /// 4. Update PC based on the result
    /// 5. Build and return a trace record
    fn step(&mut self) -> Result<GPUCoreTrace, String> {
        if self.halted {
            return Err("Cannot step: core is halted".to_string());
        }

        if self.pc >= self.program.len() {
            return Err(format!(
                "PC={} out of program range [0, {})",
                self.pc,
                self.program.len()
            ));
        }

        // Fetch
        let instruction = self.program[self.pc];
        let current_pc = self.pc;
        self.cycle += 1;

        // Execute (delegated to the pluggable ISA)
        let result = self.isa.execute(
            &instruction,
            &mut self.registers,
            &mut self.memory,
        );

        // Update PC
        let next_pc = if result.halted {
            self.halted = true;
            current_pc // PC doesn't advance on halt
        } else if result.absolute_jump {
            result.next_pc_offset as usize
        } else {
            (current_pc as i64 + result.next_pc_offset) as usize
        };
        self.pc = next_pc;

        // Build trace
        Ok(GPUCoreTrace {
            cycle: self.cycle,
            pc: current_pc,
            instruction,
            description: result.description,
            next_pc,
            halted: result.halted,
            registers_changed: result.registers_changed.unwrap_or_default(),
            memory_changed: result.memory_changed.unwrap_or_default(),
        })
    }

    fn halted(&self) -> bool {
        self.halted
    }

    /// Reset the core to its initial state.
    ///
    /// Clears registers, memory, PC, and cycle count. The loaded program
    /// is preserved -- call `load_program()` to change it.
    fn reset(&mut self) {
        self.registers = FPRegisterFile::new(self.num_registers, self.fmt);
        self.memory = LocalMemory::with_format(self.memory_size, self.fmt);
        self.pc = 0;
        self.cycle = 0;
        self.halted = false;
    }
}

impl std::fmt::Debug for GPUCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.halted {
            "halted".to_string()
        } else {
            format!("running at PC={}", self.pc)
        };
        write!(
            f,
            "GPUCore(isa={}, regs={}, fmt={}, {})",
            self.isa.name(),
            self.num_registers,
            self.fmt.name,
            status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::*;

    #[test]
    fn test_new_core() {
        let core = GPUCore::new(Box::new(GenericISA));
        assert_eq!(core.pc, 0);
        assert_eq!(core.cycle, 0);
        assert!(!core.halted());
    }

    #[test]
    fn test_simple_program() {
        let mut core = GPUCore::new(Box::new(GenericISA));
        core.load_program(vec![limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()]);
        let traces = core.run(100).unwrap();
        assert_eq!(traces.len(), 4);
        assert_eq!(core.registers.read_float(2), 12.0);
        assert!(core.halted());
    }

    #[test]
    fn test_step_when_halted() {
        let mut core = GPUCore::new(Box::new(GenericISA));
        core.load_program(vec![halt()]);
        core.step().unwrap();
        assert!(core.halted());
        assert!(core.step().is_err());
    }

    #[test]
    fn test_pc_out_of_range() {
        let mut core = GPUCore::new(Box::new(GenericISA));
        core.load_program(vec![nop()]);
        core.step().unwrap(); // PC is now 1, program length is 1
        assert!(core.step().is_err());
    }

    #[test]
    fn test_reset() {
        let mut core = GPUCore::new(Box::new(GenericISA));
        core.load_program(vec![limm(0, 42.0), halt()]);
        core.run(100).unwrap();
        assert!(core.halted());
        assert_eq!(core.registers.read_float(0), 42.0);

        core.reset();
        assert!(!core.halted());
        assert_eq!(core.pc, 0);
        assert_eq!(core.cycle, 0);
        assert_eq!(core.registers.read_float(0), 0.0);
    }

    #[test]
    fn test_load_program_resets_pc() {
        let mut core = GPUCore::new(Box::new(GenericISA));
        core.load_program(vec![limm(0, 1.0), halt()]);
        core.run(100).unwrap();

        // Loading a new program resets PC but NOT registers
        core.load_program(vec![limm(1, 2.0), halt()]);
        assert_eq!(core.pc, 0);
        assert!(!core.halted());
        // Register 0 still has its old value
        assert_eq!(core.registers.read_float(0), 1.0);
    }

    #[test]
    fn test_max_steps_exceeded() {
        let mut core = GPUCore::new(Box::new(GenericISA));
        // Infinite loop: JMP 0
        core.load_program(vec![jmp(0)]);
        let result = core.run(100);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Execution limit"));
    }

    #[test]
    fn test_debug_format() {
        let core = GPUCore::new(Box::new(GenericISA));
        let debug = format!("{:?}", core);
        assert!(debug.contains("Generic"));
        assert!(debug.contains("running"));
    }

    #[test]
    fn test_isa_name() {
        let core = GPUCore::new(Box::new(GenericISA));
        assert_eq!(core.isa_name(), "Generic");
    }

    #[test]
    fn test_with_config() {
        let core = GPUCore::with_config(Box::new(GenericISA), FP32, 64, 8192);
        assert_eq!(core.registers.num_registers, 64);
        assert_eq!(core.memory.size, 8192);
    }
}
