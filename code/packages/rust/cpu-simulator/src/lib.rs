//! # CPU Simulator -- types and traits for ISA simulators
//!
//! This crate provides the foundational types that ISA-specific simulators
//! (like RISC-V and ARM) build upon:
//!
//! - `RegisterFile`: A fixed-size array of 32-bit registers
//! - `Memory`: Byte-addressable memory with word read/write support
//! - `SparseMemory`: Address-range-mapped memory for full 32-bit address spaces
//! - `CPU`: Generic fetch-decode-execute loop driven by pluggable decoder/executor
//! - `InstructionDecoder` / `InstructionExecutor`: Traits that ISAs implement
//! - `DecodeResult` / `ExecuteResult` / `PipelineTrace`: Pipeline stage outputs

use std::collections::HashMap;

pub mod sparse_memory;

// ===========================================================================
// Register File
// ===========================================================================

/// A fixed-size array of 32-bit registers.
///
/// Register 0 can optionally be hardwired to zero (RISC-V convention).
/// Reads from x0 always return 0; writes to x0 are silently discarded.
pub struct RegisterFile {
    regs: Vec<u32>,
    zero_register: bool,
}

impl RegisterFile {
    /// Create a new register file with `count` registers.
    /// If `zero_register` is true, register 0 is hardwired to zero.
    pub fn new(count: usize, zero_register: bool) -> Self {
        Self {
            regs: vec![0u32; count],
            zero_register,
        }
    }

    /// Read a register value. Returns 0 for out-of-range or if index is 0
    /// and zero_register is enabled.
    pub fn read(&self, index: usize) -> u32 {
        if self.zero_register && index == 0 {
            return 0;
        }
        self.regs.get(index).copied().unwrap_or(0)
    }

    /// Write a value to a register. Writes to x0 are silently ignored
    /// when zero_register is enabled.
    pub fn write(&mut self, index: usize, value: u32) {
        if self.zero_register && index == 0 {
            return;
        }
        if index < self.regs.len() {
            self.regs[index] = value;
        }
    }

    /// Dump all register values as a name->value map (for pipeline traces).
    pub fn dump(&self) -> HashMap<String, u32> {
        let mut map = HashMap::new();
        for (i, &val) in self.regs.iter().enumerate() {
            map.insert(format!("R{}", i), val);
        }
        map
    }
}

// ===========================================================================
// Memory
// ===========================================================================

/// Byte-addressable memory with little-endian word operations.
///
/// Memory is a flat array of bytes, like real RAM. Multi-byte values
/// are stored in little-endian order (least significant byte first),
/// matching RISC-V and x86 conventions.
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    /// Create memory of the specified size (in bytes), initialized to zero.
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
        }
    }

    /// Read a single byte from the given address.
    pub fn read_byte(&self, addr: usize) -> u8 {
        self.data.get(addr).copied().unwrap_or(0)
    }

    /// Write a single byte to the given address.
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if addr < self.data.len() {
            self.data[addr] = value;
        }
    }

    /// Read a 32-bit word (little-endian) from the given address.
    pub fn read_word(&self, addr: usize) -> u32 {
        let b0 = self.read_byte(addr) as u32;
        let b1 = self.read_byte(addr + 1) as u32;
        let b2 = self.read_byte(addr + 2) as u32;
        let b3 = self.read_byte(addr + 3) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    /// Write a 32-bit word (little-endian) to the given address.
    pub fn write_word(&mut self, addr: usize, value: u32) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr + 1, ((value >> 8) & 0xFF) as u8);
        self.write_byte(addr + 2, ((value >> 16) & 0xFF) as u8);
        self.write_byte(addr + 3, ((value >> 24) & 0xFF) as u8);
    }

    /// Load a byte slice into memory starting at the given address.
    pub fn load_bytes(&mut self, addr: usize, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write_byte(addr + i, byte);
        }
    }

    /// Return the size of memory in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

// ===========================================================================
// Pipeline types -- DecodeResult, ExecuteResult, PipelineTrace
// ===========================================================================

/// The result of decoding a raw instruction word.
///
/// The decoder extracts the mnemonic (e.g. "add", "mov") and a map of
/// named fields (e.g. rd=3, rs1=1, imm=42). The raw instruction is
/// preserved for debugging.
pub struct DecodeResult {
    /// Human-readable instruction name (e.g. "add", "sub", "hlt").
    pub mnemonic: String,
    /// Named fields extracted from the instruction bits.
    pub fields: HashMap<String, i32>,
    /// The original 32-bit instruction word.
    pub raw_instruction: u32,
}

/// The result of executing a decoded instruction.
///
/// Captures what changed: which registers were written, which memory
/// addresses were modified, the next PC value, and whether the CPU halted.
pub struct ExecuteResult {
    /// Human-readable description of what happened (e.g. "R3 = R1 + R2 = 30").
    pub description: String,
    /// Registers that were modified: name -> new value.
    pub registers_changed: HashMap<String, u32>,
    /// Memory addresses that were modified: address -> new byte value.
    pub memory_changed: HashMap<usize, u8>,
    /// The program counter after this instruction.
    pub next_pc: usize,
    /// True if this instruction halts the CPU.
    pub halted: bool,
}

/// A complete record of one instruction's journey through the pipeline.
///
/// Each step of the CPU produces a PipelineTrace capturing the fetch,
/// decode, and execute results along with a register snapshot. This is
/// invaluable for debugging and for educational visualization.
pub struct PipelineTrace {
    /// Which cycle this trace belongs to.
    pub cycle: usize,
    /// The fetch stage result (PC and raw instruction word).
    pub fetch: FetchResult,
    /// The decode stage result (mnemonic and fields).
    pub decode: DecodeResult,
    /// The execute stage result (description, changes, next PC).
    pub execute: ExecuteResult,
    /// Snapshot of all register values after execution.
    pub register_snapshot: HashMap<String, u32>,
}

/// The result of the fetch stage: reading the instruction word from memory.
pub struct FetchResult {
    /// The program counter at which the instruction was fetched.
    pub pc: usize,
    /// The raw 32-bit instruction word read from memory.
    pub raw_instruction: u32,
}

// ===========================================================================
// Decoder and Executor traits
// ===========================================================================

/// Trait that ISA-specific decoders must implement.
///
/// Given a raw 32-bit instruction word and the current PC, the decoder
/// parses bit fields and produces a DecodeResult with the mnemonic and
/// extracted operands.
pub trait InstructionDecoder {
    fn decode(&self, raw: u32, pc: usize) -> DecodeResult;
}

/// Trait that ISA-specific executors must implement.
///
/// Given a decoded instruction, mutable access to registers and memory,
/// and the current PC, the executor performs the operation and returns
/// an ExecuteResult describing what changed.
pub trait InstructionExecutor {
    fn execute(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        memory: &mut Memory,
        pc: usize,
    ) -> ExecuteResult;
}

// ===========================================================================
// CPU -- the generic fetch-decode-execute engine
// ===========================================================================

/// A generic CPU that drives the fetch-decode-execute cycle.
///
/// The CPU owns registers and memory, and delegates instruction semantics
/// to pluggable decoder and executor trait objects. This lets us reuse
/// the same CPU core for ARM, RISC-V, or any other ISA.
pub struct CPU {
    /// The register file (publicly accessible for ISA simulators).
    pub registers: RegisterFile,
    /// Main memory (publicly accessible for ISA simulators).
    pub memory: Memory,
    /// Program counter.
    pub pc: usize,
    /// True when a halt instruction has been executed.
    pub halted: bool,
    /// Number of cycles executed so far.
    pub cycle: usize,
    /// ISA-specific instruction decoder.
    decoder: Box<dyn InstructionDecoder>,
    /// ISA-specific instruction executor.
    executor: Box<dyn InstructionExecutor>,
}

impl CPU {
    /// Create a new CPU with the given decoder, executor, register count,
    /// bit width (currently only 32-bit is supported), and memory size.
    pub fn new(
        decoder: Box<dyn InstructionDecoder>,
        executor: Box<dyn InstructionExecutor>,
        num_registers: usize,
        _bit_width: usize,
        memory_size: usize,
    ) -> Self {
        Self {
            registers: RegisterFile::new(num_registers, false),
            memory: Memory::new(memory_size),
            pc: 0,
            halted: false,
            cycle: 0,
            decoder,
            executor,
        }
    }

    /// Load a program (byte slice) into memory at the given start address
    /// and set the PC to that address.
    pub fn load_program(&mut self, program: &[u8], start_address: usize) {
        self.memory.load_bytes(start_address, program);
        self.pc = start_address;
    }

    /// Execute one fetch-decode-execute cycle and return a PipelineTrace.
    ///
    /// Panics if the CPU has already halted.
    pub fn step(&mut self) -> PipelineTrace {
        assert!(!self.halted, "CPU has halted -- no more instructions to execute");

        // 1: FETCH
        let raw_instruction = self.memory.read_word(self.pc);
        let fetch = FetchResult {
            pc: self.pc,
            raw_instruction,
        };

        // 2: DECODE
        let decode = self.decoder.decode(raw_instruction, self.pc);

        // 3: EXECUTE
        let execute = self.executor.execute(
            &decode,
            &mut self.registers,
            &mut self.memory,
            self.pc,
        );

        self.pc = execute.next_pc;
        self.halted = execute.halted;

        let trace = PipelineTrace {
            cycle: self.cycle,
            fetch,
            decode,
            execute,
            register_snapshot: self.registers.dump(),
        };

        self.cycle += 1;
        trace
    }

    /// Run instructions until the CPU halts or `max_steps` is reached.
    /// Returns a vector of PipelineTrace records.
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_file_basic() {
        let mut regs = RegisterFile::new(32, false);
        regs.write(5, 42);
        assert_eq!(regs.read(5), 42);
    }

    #[test]
    fn test_register_file_zero_register() {
        let mut regs = RegisterFile::new(32, true);
        regs.write(0, 42);
        assert_eq!(regs.read(0), 0);
        regs.write(1, 100);
        assert_eq!(regs.read(1), 100);
    }

    #[test]
    fn test_register_file_dump() {
        let mut regs = RegisterFile::new(4, false);
        regs.write(1, 42);
        let dump = regs.dump();
        assert_eq!(dump.len(), 4);
        assert_eq!(*dump.get("R1").unwrap(), 42);
    }

    #[test]
    fn test_memory_byte() {
        let mut mem = Memory::new(256);
        mem.write_byte(10, 0xAB);
        assert_eq!(mem.read_byte(10), 0xAB);
    }

    #[test]
    fn test_memory_word() {
        let mut mem = Memory::new(256);
        mem.write_word(0, 0x12345678);
        assert_eq!(mem.read_word(0), 0x12345678);
        // Little-endian check
        assert_eq!(mem.read_byte(0), 0x78);
        assert_eq!(mem.read_byte(1), 0x56);
        assert_eq!(mem.read_byte(2), 0x34);
        assert_eq!(mem.read_byte(3), 0x12);
    }

    #[test]
    fn test_memory_load_bytes() {
        let mut mem = Memory::new(256);
        mem.load_bytes(0, &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(mem.read_byte(0), 0xDE);
        assert_eq!(mem.read_byte(3), 0xEF);
    }

    // CPU tests using a trivial "halt immediately" ISA.

    struct HaltDecoder;
    impl InstructionDecoder for HaltDecoder {
        fn decode(&self, raw: u32, _pc: usize) -> DecodeResult {
            DecodeResult {
                mnemonic: if raw == 0xFFFFFFFF { "hlt" } else { "nop" }.to_string(),
                fields: HashMap::new(),
                raw_instruction: raw,
            }
        }
    }

    struct HaltExecutor;
    impl InstructionExecutor for HaltExecutor {
        fn execute(
            &self,
            decoded: &DecodeResult,
            _registers: &mut RegisterFile,
            _memory: &mut Memory,
            pc: usize,
        ) -> ExecuteResult {
            let halted = decoded.mnemonic == "hlt";
            ExecuteResult {
                description: decoded.mnemonic.clone(),
                registers_changed: HashMap::new(),
                memory_changed: HashMap::new(),
                next_pc: if halted { pc } else { pc + 4 },
                halted,
            }
        }
    }

    #[test]
    fn test_cpu_halt() {
        let mut cpu = CPU::new(
            Box::new(HaltDecoder),
            Box::new(HaltExecutor),
            16, 32, 1024,
        );
        // Write HLT instruction (0xFFFFFFFF) at address 0
        cpu.memory.write_word(0, 0xFFFFFFFF);
        let traces = cpu.run(10);
        assert_eq!(traces.len(), 1);
        assert!(cpu.halted);
    }

    #[test]
    fn test_cpu_run_max_steps() {
        let mut cpu = CPU::new(
            Box::new(HaltDecoder),
            Box::new(HaltExecutor),
            16, 32, 1024,
        );
        // Memory is all zeros = NOP, so CPU runs until max_steps
        let traces = cpu.run(5);
        assert_eq!(traces.len(), 5);
        assert!(!cpu.halted);
    }
}
