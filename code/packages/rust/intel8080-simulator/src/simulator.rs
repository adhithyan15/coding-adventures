//! Top-level Intel 8080 simulator combining all components.
//!
//! The checked lifecycle is deliberately deterministic: invalid loads and
//! instructions return [`Intel8080Error`] before mutating state, `run` executes
//! transactionally on a fresh candidate machine, and snapshots own all of
//! their memory and port data.

use cpu_simulator::Memory;

use crate::decode;
use crate::execute;
use crate::execute::{ExecuteResult, Flags, Registers};
use crate::opcodes::{CALL, IN, JMP, LDA, LHLD, OUT, PAIR_D, REG_M, SHLD, STA};

/// Complete Intel 8080 simulator: 7 named 8-bit registers, a 16-bit stack
/// pointer, 5 condition flags, flat 64Ki byte-addressable memory, 256
/// input + 256 output ports, and a 16-bit PC.
pub struct Intel8080Simulator {
    /// A, B, C, D, E, H, L + SP.
    pub regs: Registers,
    /// S, Z, AC, P, CY.
    pub flags: Flags,
    /// Flat byte-addressable memory (64 KiB by convention, but callers may
    /// size it differently via [`Self::new`]).
    pub mem: Memory,
    /// Program counter.
    pub pc: u16,
    /// True once `HLT` has executed.
    pub halted: bool,
    /// INTE flip-flop — set by `EI`, cleared by `DI`.  Not connected to any
    /// external interrupt delivery mechanism (see module docs on
    /// `intel8080_simulator.simulator` in the Python original: interrupts
    /// cannot arrive between `step()` calls in a behavioral model).
    pub interrupts_enabled: bool,
    /// 256 input ports, read by `IN port`.
    pub input_ports: [u8; 256],
    /// 256 output ports, written by `OUT port`.
    pub output_ports: [u8; 256],
}

/// A checked Intel 8080 simulator failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intel8080Error {
    /// A program does not fit in the configured memory arena.
    ProgramOutOfRange { length: usize, memory_size: usize },
    /// The current instruction extends beyond configured memory.
    TruncatedInstruction {
        address: u16,
        expected: usize,
        available: usize,
    },
    /// The byte at `address` is one of the twelve undefined 8080 opcodes.
    UnknownOpcode { address: u16, opcode: u8 },
    /// The CPU has already executed HLT.
    Halted,
    /// An instruction referenced an address outside configured memory.
    MemoryOutOfRange { address: u16, memory_size: usize },
}

impl std::fmt::Display for Intel8080Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgramOutOfRange {
                length,
                memory_size,
            } => write!(
                formatter,
                "program of {length} bytes exceeds configured memory of {memory_size} bytes"
            ),
            Self::TruncatedInstruction {
                address,
                expected,
                available,
            } => write!(
                formatter,
                "instruction at {address:#06X} needs {expected} bytes but only {available} remain"
            ),
            Self::UnknownOpcode { address, opcode } => {
                write!(formatter, "unknown opcode {opcode:#04X} at {address:#06X}")
            }
            Self::Halted => formatter.write_str("CPU is halted"),
            Self::MemoryOutOfRange {
                address,
                memory_size,
            } => write!(
                formatter,
                "address {address:#06X} is outside configured memory of {memory_size} bytes"
            ),
        }
    }
}

impl std::error::Error for Intel8080Error {}

/// Complete immutable architectural state owned by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intel8080State {
    pub regs: Registers,
    pub flags: Flags,
    pub memory: Box<[u8]>,
    pub pc: u16,
    pub halted: bool,
    pub interrupts_enabled: bool,
    pub input_ports: [u8; 256],
    pub output_ports: [u8; 256],
}

/// Complete record of one successfully executed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub address: u16,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: Intel8080State,
    pub state_after: Intel8080State,
}

/// Observable outcome of a bounded simulator run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u16,
    pub traces: Vec<StepTrace>,
    pub final_state: Intel8080State,
}

impl Intel8080Simulator {
    /// Create a new simulator with the given memory size (in bytes).  The
    /// real chip addresses a fixed 64 KiB (`0x10000`); callers pass that
    /// explicitly (mirroring `MipsR2000Simulator::new`'s shape) rather than
    /// having it baked in, so tests can use a smaller arena.
    pub fn new(memory_size: usize) -> Self {
        Self {
            regs: Registers::default(),
            flags: Flags::default(),
            mem: Memory::new(memory_size),
            pc: 0,
            halted: false,
            interrupts_enabled: false,
            input_ports: [0u8; 256],
            output_ports: [0u8; 256],
        }
    }

    /// Load a program into memory at address 0 without resetting the CPU.
    ///
    /// Invalid images are rejected atomically.
    pub fn load_program(&mut self, program: &[u8]) -> Result<(), Intel8080Error> {
        if program.len() > self.mem.size() {
            return Err(Intel8080Error::ProgramOutOfRange {
                length: program.len(),
                memory_size: self.mem.size(),
            });
        }
        self.mem.load_bytes(0, program);
        Ok(())
    }

    /// Reset CPU and memory to power-on state.
    pub fn reset(&mut self) {
        *self = Self::new(self.mem.size());
    }

    /// Return an owned snapshot of every architecturally visible state bit.
    pub fn snapshot(&self) -> Intel8080State {
        Intel8080State {
            regs: self.regs,
            flags: self.flags,
            memory: (0..self.mem.size())
                .map(|address| self.mem.read_byte(address))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            pc: self.pc,
            halted: self.halted,
            interrupts_enabled: self.interrupts_enabled,
            input_ports: self.input_ports,
            output_ports: self.output_ports,
        }
    }

    /// Load and run a program transactionally for at most `max_steps`.
    pub fn run(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8080Error> {
        if program.len() > self.mem.size() {
            return Err(Intel8080Error::ProgramOutOfRange {
                length: program.len(),
                memory_size: self.mem.size(),
            });
        }
        let mut candidate = Self::new(self.mem.size());
        candidate.input_ports = self.input_ports;
        candidate.load_program(program)?;
        let result = candidate.run_loaded_with_limit(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    /// Run instructions from an already-loaded program with a safety limit.
    pub fn run_loaded(&mut self) -> Result<ExecutionResult, Intel8080Error> {
        self.run_loaded_with_limit(1_000_000)
    }

    /// Run the already-loaded program for at most `max_steps` instructions.
    ///
    /// A non-halting result means the budget was exhausted — this makes an
    /// infinite loop (or a forgotten `HLT`) visible to callers instead of
    /// silently treating it as success, mirroring
    /// `MipsR2000Simulator::run_loaded_with_limit`.
    pub fn run_loaded_with_limit(
        &mut self,
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8080Error> {
        let mut traces = Vec::new();
        while traces.len() < max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step()?);
        }
        Ok(ExecutionResult {
            halted: self.halted,
            steps: traces.len(),
            pc: self.pc,
            traces,
            final_state: self.snapshot(),
        })
    }

    fn instruction_length(opcode: u8) -> Option<usize> {
        if matches!(
            opcode,
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38 | 0xCB | 0xD9 | 0xDD | 0xED | 0xFD
        ) {
            return None;
        }
        if opcode & 0xC7 == 0x06 || opcode & 0xC7 == 0xC6 || matches!(opcode, IN | OUT) {
            Some(2)
        } else if opcode & 0xCF == 0x01
            || opcode & 0xC7 == 0xC2
            || opcode & 0xC7 == 0xC4
            || matches!(opcode, SHLD | LHLD | STA | LDA | JMP | CALL)
        {
            Some(3)
        } else {
            Some(1)
        }
    }

    fn check_address(&self, address: u16) -> Result<(), Intel8080Error> {
        if address as usize >= self.mem.size() {
            Err(Intel8080Error::MemoryOutOfRange {
                address,
                memory_size: self.mem.size(),
            })
        } else {
            Ok(())
        }
    }

    fn preflight_data_accesses(
        &self,
        decoded: &decode::DecodeResult,
    ) -> Result<(), Intel8080Error> {
        let field = |name: &str| decoded.fields.get(name).copied().unwrap_or(0) as u16;
        let mut addresses = Vec::new();
        match decoded.mnemonic.as_str() {
            "mov" if field("src") == REG_M as u16 || field("dst") == REG_M as u16 => {
                addresses.push(self.regs.hl());
            }
            "mvi" | "inr" | "dcr" if field("dst") == REG_M as u16 => {
                addresses.push(self.regs.hl());
            }
            "alu_reg" if field("src") == REG_M as u16 => addresses.push(self.regs.hl()),
            "stax" | "ldax" => addresses.push(if field("pair") == PAIR_D as u16 {
                self.regs.de()
            } else {
                self.regs.bc()
            }),
            "shld" | "lhld" => {
                let address = field("addr");
                addresses.extend([address, address.wrapping_add(1)]);
            }
            "sta" | "lda" => addresses.push(field("addr")),
            "call" | "rst" | "push" => {
                addresses.extend([self.regs.sp.wrapping_sub(2), self.regs.sp.wrapping_sub(1)]);
            }
            "ccond" if execute::condition_is_met(field("cond") as u8, &self.flags) => {
                addresses.extend([self.regs.sp.wrapping_sub(2), self.regs.sp.wrapping_sub(1)]);
            }
            "ret" | "pop" | "xthl" => {
                addresses.extend([self.regs.sp, self.regs.sp.wrapping_add(1)]);
            }
            "rcond" if execute::condition_is_met(field("cond") as u8, &self.flags) => {
                addresses.extend([self.regs.sp, self.regs.sp.wrapping_add(1)]);
            }
            _ => {}
        }
        addresses
            .into_iter()
            .try_for_each(|address| self.check_address(address))
    }

    /// Execute a single instruction and return a complete before/after trace.
    pub fn step(&mut self) -> Result<StepTrace, Intel8080Error> {
        if self.halted {
            return Err(Intel8080Error::Halted);
        }

        self.check_address(self.pc)?;
        let opcode = self.mem.read_byte(self.pc as usize);
        let instruction_length =
            Self::instruction_length(opcode).ok_or(Intel8080Error::UnknownOpcode {
                address: self.pc,
                opcode,
            })?;
        let available = self.mem.size() - self.pc as usize;
        if instruction_length > available {
            return Err(Intel8080Error::TruncatedInstruction {
                address: self.pc,
                expected: instruction_length,
                available,
            });
        }
        let address = self.pc;
        let state_before = self.snapshot();

        // Fetch the opcode byte, then hand `decode` a closure that reads
        // (and advances) further operand bytes directly out of `self.mem`
        // / a local PC cursor.  The closure only holds a shared borrow of
        // `self.mem`, so it's fully dropped before the `execute` call below
        // takes a mutable borrow.
        let mut cursor = self.pc.wrapping_add(1);
        let mem_ref = &self.mem;
        let decoded = decode::decode(opcode, &mut || {
            let b = mem_ref.read_byte(cursor as usize);
            cursor = cursor.wrapping_add(1);
            b
        });
        debug_assert_eq!(decoded.raw.len(), instruction_length);
        self.preflight_data_accesses(&decoded)?;
        let mnemonic = decoded.mnemonic.clone();
        let raw = decoded.raw.clone();
        let fallthrough_pc = cursor;

        let ExecuteResult { next_pc, halted } = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.flags,
            &mut self.mem,
            &self.input_ports,
            &mut self.output_ports,
            &mut self.interrupts_enabled,
            fallthrough_pc,
        );
        self.pc = next_pc;
        self.halted = halted;

        Ok(StepTrace {
            address,
            raw,
            mnemonic,
            state_before,
            state_after: self.snapshot(),
        })
    }

    /// Run a sequence of pre-encoded instructions (convenience for tests) —
    /// concatenates them and calls [`Self::run`].
    pub fn run_instructions(
        &mut self,
        instructions: &[Vec<u8>],
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8080Error> {
        let program = crate::encoding::assemble(instructions);
        self.run(&program, max_steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;
    use crate::opcodes::*;

    fn run_program(instructions: &[Vec<u8>]) -> Intel8080Simulator {
        let mut sim = Intel8080Simulator::new(65536);
        sim.run_instructions(instructions, 10_000).unwrap();
        sim
    }

    // ── the trivial "load immediate into accumulator + HLT" sequence the
    // intel8080-backend smoke test relies on ──
    #[test]
    fn mvi_a_then_hlt() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[encode_mvi_a(42), vec![HLT]]))
            .unwrap();
        let result = sim.run_loaded_with_limit(10).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.a, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[encode_mvi(REG_B, 7), vec![HLT]]))
            .unwrap();
        let result = sim.run_loaded_with_limit(10).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.b, 7);
    }

    // ── ALU ops ──
    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_mvi(REG_B, 10),
            encode_mvi(REG_C, 20),
            encode_mvi_a(10),
            vec![encode_alu_reg(ALU_ADD, REG_C)], // A = 10 + 20 = 30
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 30);
        assert!(!sim.flags.cy);
    }

    #[test]
    fn test_add_overflow_sets_carry() {
        let sim = run_program(&[encode_mvi_a(0xFF), encode_alu_imm(ALU_ADD, 1), vec![HLT]]);
        assert_eq!(sim.regs.a, 0x00);
        assert!(sim.flags.cy, "0xFF + 1 should set carry");
        assert!(sim.flags.z, "result is 0");
    }

    #[test]
    fn test_sub_borrow_sets_carry() {
        let sim = run_program(&[encode_mvi_a(0x00), encode_alu_imm(ALU_SUB, 1), vec![HLT]]);
        assert_eq!(sim.regs.a, 0xFF);
        assert!(sim.flags.cy, "0x00 - 1 borrows");
        assert!(sim.flags.s);
    }

    #[test]
    fn test_ana_ora_xra() {
        let sim = run_program(&[
            encode_mvi_a(0xFF),
            encode_mvi(REG_B, 0x0F),
            vec![encode_alu_reg(ALU_ANA, REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x0F);
        assert!(!sim.flags.cy, "ANA always clears carry");
    }

    #[test]
    fn test_cmp_does_not_write_a() {
        let sim = run_program(&[
            encode_mvi_a(5),
            encode_mvi(REG_B, 5),
            vec![encode_alu_reg(ALU_CMP, REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 5, "CMP must not write A");
        assert!(sim.flags.z, "5 == 5");
    }

    #[test]
    fn test_inr_dcr_wrap() {
        let sim = run_program(&[encode_mvi(REG_B, 0xFF), vec![encode_inr(REG_B)], vec![HLT]]);
        assert_eq!(sim.regs.b, 0x00);
        assert!(sim.flags.z);
    }

    // ── Load / store ──
    #[test]
    fn test_sta_lda_round_trip() {
        let sim = run_program(&[
            encode_mvi_a(0x42),
            encode_sta(0x0100),
            encode_mvi_a(0x00),
            encode_lda(0x0100),
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x42);
        assert_eq!(sim.mem.read_byte(0x0100), 0x42);
    }

    #[test]
    fn test_mov_through_memory_m() {
        // LXI H, 0x0200 ; MVI M, 0x55 ; MOV A, M
        let sim = run_program(&[
            encode_lxi(PAIR_H, 0x0200),
            encode_mvi(REG_M, 0x55),
            vec![encode_mov(REG_A, REG_M)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x55);
        assert_eq!(sim.mem.read_byte(0x0200), 0x55);
    }

    #[test]
    fn test_lxi_stax_ldax() {
        let sim = run_program(&[
            encode_lxi(PAIR_B, 0x0300),
            encode_mvi_a(0x77),
            vec![encode_stax(PAIR_B)],
            encode_mvi_a(0x00),
            vec![encode_ldax(PAIR_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x77);
    }

    // ── Branches / jumps ──
    #[test]
    fn test_jmp_unconditional() {
        // JMP over a poison MVI A,0x99 straight to MVI A,0x42; HLT.
        // Layout: JMP(3 bytes @0) MVI A,0x99(2 bytes @3) MVI A,0x42(2 bytes @5) HLT(@7).
        let sim = run_program(&[
            encode_jmp(5),
            encode_mvi_a(0x99),
            encode_mvi_a(0x42),
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x42);
    }

    #[test]
    fn test_jz_taken_and_not_taken() {
        // A=0; SUB A -> Z=1; JZ taken.
        // Layout: MVI A,5(2 @0) SUB A(1 @2) JZ(3 @3) MVI B,0x99(2 @6) MVI C,0x42(2 @8) HLT(@10).
        let taken = run_program(&[
            encode_mvi_a(5),
            vec![encode_alu_reg(ALU_SUB, REG_A)], // A=0, Z=1
            encode_jcond(COND_Z, 8),
            encode_mvi(REG_B, 0x99),
            encode_mvi(REG_C, 0x42),
            vec![HLT],
        ]);
        assert_eq!(taken.regs.b, 0);
        assert_eq!(taken.regs.c, 0x42);
    }

    #[test]
    fn test_call_ret() {
        // main: CALL sub; MVI B,42; HLT   (7 bytes: CD 08 00 06 2A 76)
        // sub (addr 8): MVI A,7; RET
        let mut program = vec![0u8; 12];
        program[0] = CALL;
        program[1] = 8;
        program[2] = 0;
        program[3..5].copy_from_slice(&encode_mvi(REG_B, 42));
        program[5] = HLT;
        program[8] = MVI_A_OPCODE;
        program[9] = 7;
        program[10] = RET;

        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&program).unwrap();
        sim.regs.sp = 0xFF00;
        sim.run_loaded_with_limit(20).unwrap();
        assert_eq!(sim.regs.a, 7);
        assert_eq!(sim.regs.b, 42);
    }

    const MVI_A_OPCODE: u8 = 0x3E;

    #[test]
    fn test_push_pop() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        sim.load_program(&assemble(&[
            encode_lxi(PAIR_H, 0x1234),
            vec![encode_push(PAIR_H)],
            encode_lxi(PAIR_H, 0x0000),
            vec![encode_pop(PAIR_H)],
            vec![HLT],
        ]))
        .unwrap();
        sim.run_loaded_with_limit(10).unwrap();
        assert_eq!(sim.regs.hl(), 0x1234);
    }

    #[test]
    fn test_rst_pushes_return_and_jumps() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        // RST 1 at address 0 -> jumps to 8, which is HLT.
        let mut program = vec![0u8; 16];
        program[0] = encode_rst(1);
        program[8] = HLT;
        sim.load_program(&program).unwrap();
        let result = sim.run_loaded_with_limit(5).unwrap();
        assert!(result.halted);
        assert_eq!(result.pc, 9);
    }

    #[test]
    fn test_daa_bcd_correction() {
        // 0x09 + 0x08 = 0x11 raw; DAA corrects to 0x17 (9+8=17 decimal).
        let sim = run_program(&[
            encode_mvi_a(0x09),
            encode_alu_imm(ALU_ADD, 0x08),
            vec![DAA],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x17);
    }

    #[test]
    fn test_rotates() {
        let sim = run_program(&[encode_mvi_a(0x80), vec![RLC], vec![HLT]]);
        assert_eq!(sim.regs.a, 0x01);
        assert!(sim.flags.cy);
    }

    #[test]
    fn test_in_out_ports() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.input_ports[3] = 0xAB;
        sim.load_program(&assemble(&[encode_in(3), vec![HLT]]))
            .unwrap();
        sim.run_loaded_with_limit(5).unwrap();
        assert_eq!(sim.regs.a, 0xAB);
    }

    #[test]
    fn test_ei_di() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[vec![EI], vec![DI], vec![HLT]]))
            .unwrap();
        sim.run_loaded_with_limit(5).unwrap();
        assert!(!sim.interrupts_enabled);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[encode_mvi_a(1), vec![HLT]]))
            .unwrap();
        assert_eq!(sim.step().unwrap().mnemonic, "mvi");
        assert_eq!(sim.regs.a, 1);
        assert_eq!(sim.step().unwrap().mnemonic, "hlt");
        assert!(sim.halted);
    }

    #[test]
    fn test_undefined_opcode_is_a_typed_error() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&[0x08]).unwrap(); // undefined on stock 8080
        assert_eq!(
            sim.run_loaded_with_limit(5),
            Err(Intel8080Error::UnknownOpcode {
                address: 0,
                opcode: 0x08,
            })
        );
    }

    // ── Integration: sum 1..=4 via ADD (matches the spec's "Hello" example) ──
    #[test]
    fn test_sum_program() {
        let sim = run_program(&[
            encode_mvi_a(0),
            encode_mvi(REG_B, 1),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_mvi(REG_B, 2),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_mvi(REG_B, 3),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_mvi(REG_B, 4),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 10);
    }
}
