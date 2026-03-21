//! # CPU -- the central processing unit that ties everything together.
//!
//! ## What is a CPU?
//!
//! The CPU (Central Processing Unit) is the "brain" of a computer. But unlike
//! a human brain, it's extremely simple -- it can only do one thing:
//!
//! > Read an instruction, figure out what it means, do what it says. Repeat.
//!
//! That's it. That's all a CPU does. The power of a computer comes not from
//! the complexity of individual operations (they're trivial -- add two numbers,
//! copy a value, compare two things) but from doing billions of them per second.
//!
//! ## CPU components
//!
//! ```text
//!     +------------------------------------------------------+
//!     |                        CPU                           |
//!     |                                                      |
//!     |  +----------+  +----------------+  +-------------+   |
//!     |  | Program  |  |  Register File |  |    ALU      |   |
//!     |  | Counter  |  | R0  R1  R2 ... |  |  (add, sub, |   |
//!     |  |  (PC)    |  | [0] [0] [0]    |  |   and, or)  |   |
//!     |  +----------+  +----------------+  +-------------+   |
//!     |                                                      |
//!     |  +------------------------------------------+        |
//!     |  |  Control Unit (fetch-decode-execute)      |        |
//!     |  +------------------------------------------+        |
//!     |                                                      |
//!     +---------------------+--------------------------------+
//!                           | (memory bus)
//!                           v
//!     +------------------------------------------------------+
//!     |                      Memory                          |
//!     |  [instruction 0] [instruction 1] [data] [data] ...   |
//!     +------------------------------------------------------+
//! ```
//!
//! - **Program Counter (PC):** A special register that holds the address of
//!   the next instruction to execute. It's like a bookmark in a book.
//!
//! - **Register File:** A small set of fast storage slots (see [`registers`]).
//!
//! - **ALU:** The arithmetic/logic unit that does actual computation
//!   (see the `arithmetic` crate).
//!
//! - **Control Unit:** The logic that orchestrates the fetch-decode-execute
//!   cycle -- reading instructions, decoding them, and dispatching
//!   operations to the ALU and registers.
//!
//! - **Memory:** External storage connected via a "bus" (see [`memory`]).
//!
//! ## How this module works
//!
//! This module provides the CPU shell -- registers, memory, PC, and the
//! pipeline framework. It does NOT know how to decode specific instructions
//! (that's ISA-specific). Instead, it accepts a `decoder` and `executor`
//! that are provided by the ISA simulator (RISC-V, ARM, etc.).
//!
//! This separation means the same CPU can run RISC-V, ARM, WASM, or 4004
//! instructions -- you just plug in a different decoder.

use std::collections::HashMap;

use crate::memory::Memory;
use crate::pipeline::{DecodeResult, ExecuteResult, FetchResult, PipelineTrace};
use crate::registers::RegisterFile;

// ---------------------------------------------------------------------------
// Decoder and Executor traits
// ---------------------------------------------------------------------------
// These define the interface that ISA simulators must implement.
// The CPU calls decode() and execute() -- the ISA provides the implementation.

/// Trait for ISA-specific instruction decoding.
///
/// The CPU fetches raw bits from memory and passes them to the decoder.
/// The decoder figures out what those bits mean in the context of a
/// specific instruction set (RISC-V, ARM, etc.).
///
/// # Implementing a decoder
///
/// ```text
///     impl InstructionDecoder for MyRiscvDecoder {
///         fn decode(&self, raw: u32, pc: usize) -> DecodeResult {
///             // Extract opcode, registers, immediate from raw bits
///             // Return a DecodeResult with mnemonic and fields
///         }
///     }
/// ```
pub trait InstructionDecoder {
    fn decode(&self, raw_instruction: u32, pc: usize) -> DecodeResult;
}

/// Trait for ISA-specific instruction execution.
///
/// The CPU passes the decoded instruction to the executor, along with
/// mutable references to the register file and memory. The executor
/// performs the operation and returns what changed.
///
/// # Implementing an executor
///
/// ```text
///     impl InstructionExecutor for MyRiscvExecutor {
///         fn execute(&self, decoded: &DecodeResult, regs: &mut RegisterFile,
///                    mem: &mut Memory, pc: usize) -> ExecuteResult {
///             // Read registers, compute result, write back
///             // Return an ExecuteResult describing what happened
///         }
///     }
/// ```
pub trait InstructionExecutor {
    fn execute(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        memory: &mut Memory,
        pc: usize,
    ) -> ExecuteResult;
}

// ---------------------------------------------------------------------------
// CPU State
// ---------------------------------------------------------------------------

/// A snapshot of the entire CPU state at a point in time.
///
/// This is useful for debugging and visualization -- you can capture
/// the state before and after each instruction to see what changed.
///
/// ```text
///     CPUState {
///         pc: 8,
///         registers: { "R0": 0, "R1": 1, "R2": 2, "R3": 3 },
///         halted: false,
///         cycle: 2,
///     }
/// ```
#[derive(Debug, Clone)]
pub struct CPUState {
    pub pc: usize,
    pub registers: HashMap<String, u32>,
    pub halted: bool,
    pub cycle: usize,
}

// ---------------------------------------------------------------------------
// CPU
// ---------------------------------------------------------------------------

/// A generic CPU that executes instructions through a visible pipeline.
///
/// The CPU doesn't know what instruction set it's running -- that's
/// determined by the decoder and executor you provide. This makes it
/// reusable across RISC-V, ARM, WASM, and Intel 4004.
///
/// # Usage
///
/// 1. Create a CPU with a decoder and executor
/// 2. Load a program into memory
/// 3. Call `step()` to execute one instruction (visible pipeline)
/// 4. Or call `run()` to execute until halt
///
/// # Example
///
/// ```text
///     let cpu = CPU::new(decoder, executor, 32, 32, 65536);
///     cpu.load_program(&machine_code, 0);
///     let trace = cpu.step();
///     println!("{}", format_pipeline(&trace));
/// ```
pub struct CPU {
    /// The CPU's register file -- fast internal storage.
    pub registers: RegisterFile,

    /// The CPU's main memory -- program instructions and data live here.
    pub memory: Memory,

    /// Program counter -- address of the next instruction to fetch.
    pub pc: usize,

    /// Whether the CPU has halted (received a halt instruction).
    pub halted: bool,

    /// How many instructions have been executed so far.
    pub cycle: usize,

    /// The ISA-specific instruction decoder.
    decoder: Box<dyn InstructionDecoder>,

    /// The ISA-specific instruction executor.
    executor: Box<dyn InstructionExecutor>,
}

impl CPU {
    /// Create a new CPU with the given decoder, executor, and hardware parameters.
    ///
    /// - `num_registers`: How many general-purpose registers (e.g., 32 for RISC-V)
    /// - `bit_width`: Bits per register (e.g., 32 for a 32-bit CPU)
    /// - `memory_size`: Total bytes of memory
    pub fn new(
        decoder: Box<dyn InstructionDecoder>,
        executor: Box<dyn InstructionExecutor>,
        num_registers: usize,
        bit_width: usize,
        memory_size: usize,
    ) -> Self {
        CPU {
            registers: RegisterFile::new(num_registers, bit_width),
            memory: Memory::new(memory_size),
            pc: 0,
            halted: false,
            cycle: 0,
            decoder,
            executor,
        }
    }

    /// Capture the current CPU state as a snapshot.
    ///
    /// This copies all register values and flags into a standalone struct
    /// that can be compared, printed, or stored for later inspection.
    pub fn state(&self) -> CPUState {
        CPUState {
            pc: self.pc,
            registers: self.registers.dump(),
            halted: self.halted,
            cycle: self.cycle,
        }
    }

    /// Load machine code bytes into memory.
    ///
    /// This is how programs get into the computer -- the bytes are copied
    /// into memory starting at `start_address`. The PC is set to point
    /// at the first instruction.
    ///
    /// # Example
    ///
    /// ```text
    ///     // Load a 3-instruction program at address 0:
    ///     cpu.load_program(&[0x93, 0x00, 0x10, 0x00, ...], 0);
    /// ```
    pub fn load_program(&mut self, program: &[u8], start_address: usize) {
        self.memory.load_bytes(start_address, program);
        self.pc = start_address;
    }

    /// Execute ONE instruction through the full pipeline.
    ///
    /// This is the core of the CPU -- the fetch-decode-execute cycle
    /// made visible. Each call to `step()` processes one instruction
    /// and returns a [`PipelineTrace`] showing what happened at each stage.
    ///
    /// The three stages:
    ///
    /// ```text
    ///     +-----------+    +-----------+    +-----------+
    ///     |   FETCH   |--->|  DECODE   |--->|  EXECUTE  |
    ///     |           |    |           |    |           |
    ///     | Read 4    |    | What does |    | Do the    |
    ///     | bytes at  |    | this      |    | operation,|
    ///     | PC from   |    | binary    |    | update    |
    ///     | memory    |    | mean?     |    | registers |
    ///     +-----------+    +-----------+    +-----------+
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the CPU has already halted.
    pub fn step(&mut self) -> PipelineTrace {
        assert!(!self.halted, "CPU has halted -- no more instructions to execute");

        // === STAGE 1: FETCH ===
        // Read 4 bytes from memory at the current PC.
        // These 4 bytes form one 32-bit instruction.
        let raw_instruction = self.memory.read_word(self.pc);
        let fetch_result = FetchResult {
            pc: self.pc,
            raw_instruction,
        };

        // === STAGE 2: DECODE ===
        // Pass the raw bits to the ISA-specific decoder.
        // The decoder extracts the opcode, register numbers, and immediate values.
        let decode_result = self.decoder.decode(raw_instruction, self.pc);

        // === STAGE 3: EXECUTE ===
        // Pass the decoded instruction to the ISA-specific executor.
        // The executor reads registers, uses the ALU, writes results back.
        let execute_result = self.executor.execute(
            &decode_result,
            &mut self.registers,
            &mut self.memory,
            self.pc,
        );

        // === UPDATE CPU STATE ===
        // After execution, update the PC and check if we should halt.
        self.pc = execute_result.next_pc;
        self.halted = execute_result.halted;

        // Build the complete pipeline trace for this instruction.
        let trace = PipelineTrace {
            cycle: self.cycle,
            fetch: fetch_result,
            decode: decode_result,
            execute: execute_result,
            register_snapshot: self.registers.dump(),
        };

        self.cycle += 1;
        trace
    }

    /// Run the CPU until it halts or hits the step limit.
    ///
    /// Returns a vector of [`PipelineTrace`] objects -- one for each instruction
    /// executed. This gives you the complete execution history.
    ///
    /// The `max_steps` parameter is a safety limit to prevent infinite loops.
    /// If the program doesn't halt within `max_steps` instructions, execution
    /// stops and the traces collected so far are returned.
    ///
    /// # Example
    ///
    /// ```text
    ///     let traces = cpu.run(10000);
    ///     for trace in &traces {
    ///         println!("{}", format_pipeline(trace));
    ///     }
    /// ```
    pub fn run(&mut self, max_steps: usize) -> Vec<PipelineTrace> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step());
        }
        traces
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Mock decoder/executor for testing
    // -----------------------------------------------------------------------
    //
    // To test the CPU without a real ISA, we create a trivial instruction set:
    //
    //   Opcode (lowest byte):
    //     0x01 = Write 42 to register 1
    //     0x02 = Write 255 to memory address 10
    //     0x00 = Halt
    //     anything else = NOP (advance PC by 4)

    struct MockDecoder;

    impl InstructionDecoder for MockDecoder {
        fn decode(&self, raw_instruction: u32, _pc: usize) -> DecodeResult {
            DecodeResult {
                mnemonic: "mock".to_string(),
                fields: HashMap::from([("op".to_string(), (raw_instruction & 0xFF) as i32)]),
                raw_instruction,
            }
        }
    }

    struct MockExecutor;

    impl InstructionExecutor for MockExecutor {
        fn execute(
            &self,
            decoded: &DecodeResult,
            registers: &mut RegisterFile,
            memory: &mut Memory,
            pc: usize,
        ) -> ExecuteResult {
            let op = decoded.fields["op"];
            match op {
                1 => {
                    registers.write(1, 42);
                    ExecuteResult {
                        description: "write reg 1".to_string(),
                        registers_changed: HashMap::from([("R1".to_string(), 42)]),
                        memory_changed: HashMap::new(),
                        next_pc: pc + 4,
                        halted: false,
                    }
                }
                2 => {
                    memory.write_byte(10, 255);
                    ExecuteResult {
                        description: "write mem 10".to_string(),
                        registers_changed: HashMap::new(),
                        memory_changed: HashMap::from([(10, 255)]),
                        next_pc: pc + 4,
                        halted: false,
                    }
                }
                0 => ExecuteResult {
                    description: "halt".to_string(),
                    registers_changed: HashMap::new(),
                    memory_changed: HashMap::new(),
                    next_pc: pc,
                    halted: true,
                },
                _ => ExecuteResult {
                    description: "nop".to_string(),
                    registers_changed: HashMap::new(),
                    memory_changed: HashMap::new(),
                    next_pc: pc + 4,
                    halted: false,
                },
            }
        }
    }

    // -----------------------------------------------------------------------
    // CPU tests
    // -----------------------------------------------------------------------

    #[test]
    fn cpu_executes_three_instruction_program() {
        // Program: OP=1 (writes reg), OP=2 (writes mem), OP=0 (halts)
        //
        // Each instruction is 4 bytes (one 32-bit word):
        //   [0x01, 0x00, 0x00, 0x00]  -> opcode 1
        //   [0x02, 0x00, 0x00, 0x00]  -> opcode 2
        //   [0x00, 0x00, 0x00, 0x00]  -> opcode 0 (halt)
        let program: Vec<u8> = vec![
            1, 0, 0, 0, // OP=1: write reg
            2, 0, 0, 0, // OP=2: write mem
            0, 0, 0, 0, // OP=0: halt
        ];

        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4,  // 4 registers
            32, // 32-bit
            128, // 128 bytes of memory
        );
        cpu.load_program(&program, 0);

        let traces = cpu.run(10);
        assert_eq!(traces.len(), 3, "Should execute 3 instructions before halt");

        // Verify side effects: register 1 should be 42.
        assert_eq!(cpu.registers.read(1), 42);

        // Verify side effects: memory byte 10 should be 255.
        assert_eq!(cpu.memory.read_byte(10), 255);

        // Verify PC: halt instruction sets PC to its own address (8).
        assert_eq!(cpu.pc, 8);

        // Verify halted state.
        assert!(cpu.halted);
    }

    #[test]
    fn cpu_state_snapshot() {
        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4, 32, 128,
        );
        cpu.load_program(&[1, 0, 0, 0], 0);
        cpu.step();

        let state = cpu.state();
        assert_eq!(state.pc, 4);
        assert_eq!(state.cycle, 1);
        assert!(!state.halted);
        assert_eq!(state.registers["R1"], 42);
    }

    #[test]
    #[should_panic(expected = "CPU has halted")]
    fn step_after_halt_panics() {
        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4, 32, 128,
        );
        // Load a single halt instruction.
        cpu.load_program(&[0, 0, 0, 0], 0);
        cpu.step(); // Executes halt
        cpu.step(); // Should panic
    }

    #[test]
    fn run_respects_max_steps() {
        // A program of all NOPs (opcode 0xFF) that never halts.
        let program: Vec<u8> = vec![0xFF; 128];
        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4, 32, 128,
        );
        cpu.load_program(&program, 0);

        let traces = cpu.run(5);
        assert_eq!(traces.len(), 5, "Should stop after max_steps");
        assert!(!cpu.halted, "CPU should not be halted");
    }

    #[test]
    fn load_program_sets_pc() {
        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4, 32, 128,
        );
        cpu.load_program(&[1, 0, 0, 0], 16);
        assert_eq!(cpu.pc, 16, "PC should be set to start_address");
    }

    #[test]
    fn pipeline_trace_records_cycle_numbers() {
        let program: Vec<u8> = vec![
            1, 0, 0, 0,
            1, 0, 0, 0,
            0, 0, 0, 0,
        ];
        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4, 32, 128,
        );
        cpu.load_program(&program, 0);

        let traces = cpu.run(10);
        for (i, trace) in traces.iter().enumerate() {
            assert_eq!(trace.cycle, i, "Trace {} should have cycle {}", i, i);
        }
    }

    #[test]
    fn pipeline_trace_captures_register_snapshot() {
        let mut cpu = CPU::new(
            Box::new(MockDecoder),
            Box::new(MockExecutor),
            4, 32, 128,
        );
        cpu.load_program(&[1, 0, 0, 0, 0, 0, 0, 0], 0);

        let trace = cpu.step();
        // After OP=1, register 1 should be 42 in the snapshot.
        assert_eq!(trace.register_snapshot["R1"], 42);
    }
}
