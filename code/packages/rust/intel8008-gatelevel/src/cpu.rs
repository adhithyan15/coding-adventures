//! Intel 8008 gate-level CPU — all operations route through real logic gates.
//!
//! # What makes this gate-level?
//!
//! Every computation flows through the gate chain:
//!
//! ```text
//! NOT/AND/OR/XOR → half_adder → full_adder → ripple_carry_adder → ALU
//! D flip-flop → register → register file / program counter / stack
//! ```
//!
//! When you execute ADD B:
//! 1. Register B is read from its flip-flop array
//! 2. The accumulator is read from its flip-flop array
//! 3. Both are fed into `GateAlu8::add()` (which calls the arithmetic crate's
//!    `ripple_carry_adder`, which chains 8 full-adders, each built from 5 gates)
//! 4. The 8-bit result is clocked into the accumulator's flip-flop array
//! 5. The parity is computed by `xor_n` (7 XOR gates in a chain) + NOT
//!
//! # Cross-validation
//!
//! The gate-level CPU produces identical results to the behavioral simulator
//! for any program. The `run()` method returns `Vec<Trace>` (from the
//! `coding-adventures-intel8008-simulator` crate) so both simulators can be
//! directly compared trace-by-trace.
//!
//! # Gate count estimate
//!
//! ```text
//! Component                Gates    Notes
//! ─────────────────────    ──────   ──────────────────────────────────────
//! 8-bit ALU (add/sub)      40+7     8 full-adders + 7-gate parity XOR tree
//! Register file (7×8)      336      7 × 8 × 6 gates per flip-flop
//! Accumulator (8-bit)      48       8 flip-flops × 6 gates
//! Stack (8×14-bit)         672      8 × 14 flip-flops × 6 gates
//! PC (14-bit)              84       14 flip-flops × 6 gates
//! PC increment chain       28       14 half-adder stages × 2 gates
//! Instruction decoder      ~80      AND/OR/NOT gate tree
//! Control + wiring         ~200     mux selections, bus logic
//! ─────────────────────    ──────   ──────────────────────────────────────
//! Estimated total          ~1,495   (real 8008: ~3,500 transistors ≈ ~875 gates)
//! ```
//!
//! Our simulation is conservative — we count logical gates, not transistors.
//! The real chip's PMOS implementation required ~4 transistors per gate.

use std::collections::HashMap;

use coding_adventures_intel8008_simulator::{Flags, Trace};

use crate::alu::{AluFlags, GateAlu8};
use crate::bits::{compute_parity, int_to_bits};
use crate::decoder::decode;
use crate::pc::ProgramCounter;
use crate::registers::RegisterFile;
use crate::stack::PushDownStack;

const REG_A: usize = 7;
const REG_M: usize = 6;
const REG_NAMES: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];
const ALU_MNEMONICS: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];
const ALU_IMM_MNEMONICS: [&str; 8] = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];

/// Intel 8008 CPU where every operation routes through real logic gates.
///
/// Implements the same instruction set as `coding_adventures_intel8008_simulator::Simulator`
/// but all arithmetic and flag computation goes through gate-level functions.
pub struct GateLevelCpu {
    regs: RegisterFile,
    stack: PushDownStack,
    memory: Box<[u8; 16384]>,
    flags: Flags,
    halted: bool,
    input_ports: [u8; 8],
    output_ports: [u8; 24],
}

impl Default for GateLevelCpu {
    fn default() -> Self {
        Self::new()
    }
}

impl GateLevelCpu {
    /// Create a new gate-level CPU with all state zeroed.
    pub fn new() -> Self {
        GateLevelCpu {
            regs: RegisterFile::new(),
            stack: PushDownStack::new(),
            memory: Box::new([0u8; 16384]),
            flags: Flags { carry: false, zero: false, sign: false, parity: false },
            halted: false,
            input_ports: [0u8; 8],
            output_ports: [0u8; 24],
        }
    }

    // -------------------------------------------------------------------------
    // Register / memory accessors
    // -------------------------------------------------------------------------

    /// Accumulator value.
    pub fn a(&self) -> u8 { self.regs.read(REG_A) }
    /// Register B.
    pub fn b(&self) -> u8 { self.regs.read(0) }
    /// Register C.
    pub fn c(&self) -> u8 { self.regs.read(1) }
    /// Register D.
    pub fn d(&self) -> u8 { self.regs.read(2) }
    /// Register E.
    pub fn e(&self) -> u8 { self.regs.read(3) }
    /// Register H.
    pub fn h(&self) -> u8 { self.regs.read(4) }
    /// Register L.
    pub fn l(&self) -> u8 { self.regs.read(5) }
    /// Current 14-bit program counter.
    pub fn pc(&self) -> u16 { self.stack.pc() }
    /// Current condition flags.
    pub fn flags(&self) -> Flags { self.flags }
    /// Stack nesting depth (0-7).
    pub fn stack_depth(&self) -> usize { self.stack.depth() }
    /// True if HLT has executed.
    pub fn halted(&self) -> bool { self.halted }

    /// 14-bit effective address for M: (H & 0x3F) << 8 | L
    fn hl_address(&self) -> u16 {
        ((self.regs.read(4) as u16 & 0x3F) << 8) | self.regs.read(5) as u16
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[(addr & 0x3FFF) as usize]
    }

    fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[(addr & 0x3FFF) as usize] = value;
    }

    fn reg_read(&self, idx: usize) -> u8 {
        if idx == REG_M {
            self.mem_read(self.hl_address())
        } else {
            self.regs.read(idx)
        }
    }

    fn reg_write_val(&mut self, idx: usize, value: u8) {
        if idx == REG_M {
            let addr = self.hl_address();
            self.mem_write(addr, value);
        } else {
            self.regs.write(idx, value);
        }
    }

    // -------------------------------------------------------------------------
    // I/O ports
    // -------------------------------------------------------------------------

    /// Set the value of an input port (0-7).
    pub fn set_input_port(&mut self, port: usize, value: u8) {
        self.input_ports[port.min(7)] = value;
    }

    /// Read the current value of an output port (0-23).
    pub fn get_output_port(&self, port: usize) -> u8 {
        self.output_ports[port.min(23)]
    }

    // -------------------------------------------------------------------------
    // Flag helpers
    // -------------------------------------------------------------------------

    /// Convert gate-level AluFlags into the shared Flags type, optionally
    /// preserving the carry flag (for INR/DCR which don't update carry).
    fn to_flags(&self, alu_flags: AluFlags, preserve_carry: bool) -> Flags {
        Flags {
            carry: if preserve_carry { self.flags.carry } else { alu_flags.carry },
            zero: alu_flags.zero,
            sign: alu_flags.sign,
            parity: alu_flags.parity,
        }
    }

    /// Compute flags from a raw result byte using the gate-level parity tree.
    fn flags_from_result(&self, result: u8, carry: bool, preserve_carry: bool) -> Flags {
        let bits = int_to_bits(result, 8);
        let any_set = bits.iter().any(|&b| b == 1);
        Flags {
            carry: if preserve_carry { self.flags.carry } else { carry },
            zero: !any_set,
            sign: bits[7] == 1,
            parity: compute_parity(&bits) == 1,
        }
    }

    /// Evaluate a condition (CCC + sense bit) against current flags.
    fn condition_met(&self, ccc: u8, sense: bool) -> bool {
        let flag_val = match ccc & 0x03 {
            0 => self.flags.carry,
            1 => self.flags.zero,
            2 => self.flags.sign,
            3 => self.flags.parity,
            _ => false,
        };
        if sense { flag_val } else { !flag_val }
    }

    // -------------------------------------------------------------------------
    // Execution
    // -------------------------------------------------------------------------

    /// Load a program into memory starting at `start`.
    pub fn load_program(&mut self, program: &[u8], start: usize) {
        let end = (start + program.len()).min(16384);
        self.memory[start..end].copy_from_slice(&program[..end - start]);
    }

    /// Reset all CPU state (memory is not cleared).
    pub fn reset(&mut self) {
        self.regs = RegisterFile::new();
        self.stack = PushDownStack::new();
        self.flags = Flags { carry: false, zero: false, sign: false, parity: false };
        self.halted = false;
    }

    /// Fetch one byte from memory at the current PC and advance the PC.
    fn fetch_byte(&mut self) -> u8 {
        let pc = self.stack.pc();
        let byte = self.memory[pc as usize];
        let mut pc_reg = ProgramCounter::new();
        pc_reg.load(pc);
        pc_reg.increment();
        self.stack.set_pc(pc_reg.read());
        byte
    }

    /// Execute one instruction and return a trace.
    pub fn step(&mut self) -> Result<Trace, String> {
        if self.halted {
            return Err("CPU is halted".to_string());
        }

        let fetch_pc = self.stack.pc();
        let a_before = self.regs.read(REG_A);
        let flags_before = self.flags;

        let opcode = self.fetch_byte();
        let mut raw = vec![opcode];
        let mut mem_address: Option<u16> = None;
        let mut mem_value: Option<u8> = None;

        let decoded = decode(opcode);
        let ddd = (opcode >> 3) & 0x07;
        let sss = opcode & 0x07;

        let mnemonic: String;

        if decoded.is_halt {
            self.halted = true;
            mnemonic = "HLT".to_string();
        } else if decoded.is_mvi {
            let data = self.fetch_byte();
            raw.push(data);
            if ddd as usize == REG_M {
                let addr = self.hl_address();
                self.mem_write(addr, data);
                mem_address = Some(addr);
                mem_value = Some(data);
                mnemonic = format!("MVI M, 0x{:02X}", data);
            } else {
                self.regs.write(ddd as usize, data);
                mnemonic = format!("MVI {}, 0x{:02X}", REG_NAMES[ddd as usize], data);
            }
        } else if decoded.is_inr {
            let prev = self.reg_read(ddd as usize);
            let result = GateAlu8::increment(prev);
            self.reg_write_val(ddd as usize, result);
            if ddd as usize == REG_M {
                mem_address = Some(self.hl_address());
                mem_value = Some(result);
            }
            self.flags = self.flags_from_result(result, self.flags.carry, true);
            mnemonic = format!("INR {}", REG_NAMES[ddd as usize]);
        } else if decoded.is_dcr {
            let prev = self.reg_read(ddd as usize);
            let result = GateAlu8::decrement(prev);
            self.reg_write_val(ddd as usize, result);
            if ddd as usize == REG_M {
                mem_address = Some(self.hl_address());
                mem_value = Some(result);
            }
            self.flags = self.flags_from_result(result, self.flags.carry, true);
            mnemonic = format!("DCR {}", REG_NAMES[ddd as usize]);
        } else if decoded.is_rotate {
            let a = self.regs.read(REG_A);
            let (result, new_carry) = match decoded.rotate_type {
                0 => GateAlu8::rotate_left_circular(a),
                1 => GateAlu8::rotate_right_circular(a),
                2 => GateAlu8::rotate_left_carry(a, self.flags.carry),
                3 => GateAlu8::rotate_right_carry(a, self.flags.carry),
                _ => unreachable!(),
            };
            self.regs.write(REG_A, result);
            self.flags.carry = new_carry;
            let names = ["RLC", "RRC", "RAL", "RAR"];
            mnemonic = names[decoded.rotate_type as usize].to_string();
        } else if decoded.is_output {
            let port = decoded.port as usize;
            let a = self.regs.read(REG_A);
            if port < 24 {
                self.output_ports[port] = a;
            }
            mnemonic = format!("OUT {}", port);
        } else if decoded.is_rst {
            let target = decoded.rst_target;
            mnemonic = format!("RST {}", ddd);
            self.stack.push_and_jump(target);
        } else if decoded.is_return {
            if opcode == 0x3F {
                self.stack.pop_return();
                mnemonic = "RET".to_string();
            } else {
                let sense = sss == 7; // sss=7 → T=1 (return if true)
                let ccc = ddd as u8 & 0x03;
                let prefix = if sense { "T" } else { "F" };
                let flag_name = ["C", "Z", "S", "P"][ccc as usize];
                mnemonic = format!("R{}{}", prefix, flag_name);
                if self.condition_met(ccc, sense) {
                    self.stack.pop_return();
                }
            }
        } else if decoded.is_jump {
            let addr_lo = self.fetch_byte();
            let addr_hi = self.fetch_byte();
            raw.push(addr_lo);
            raw.push(addr_hi);
            let target = ((addr_hi as u16 & 0x3F) << 8) | addr_lo as u16;
            if opcode == 0x7C {
                self.stack.set_pc(target);
                mnemonic = format!("JMP 0x{:04X}", target);
            } else {
                let sense = sss == 4;
                let ccc = ddd as u8 & 0x03;
                let prefix = if sense { "T" } else { "F" };
                let flag_name = ["C", "Z", "S", "P"][ccc as usize];
                mnemonic = format!("J{}{} 0x{:04X}", prefix, flag_name, target);
                if self.condition_met(ccc, sense) {
                    self.stack.set_pc(target);
                }
            }
        } else if decoded.is_call {
            let addr_lo = self.fetch_byte();
            let addr_hi = self.fetch_byte();
            raw.push(addr_lo);
            raw.push(addr_hi);
            let target = ((addr_hi as u16 & 0x3F) << 8) | addr_lo as u16;
            if opcode == 0x7E {
                self.stack.push_and_jump(target);
                mnemonic = format!("CAL 0x{:04X}", target);
            } else {
                let sense = sss == 6;
                let ccc = ddd as u8 & 0x03;
                let prefix = if sense { "T" } else { "F" };
                let flag_name = ["C", "Z", "S", "P"][ccc as usize];
                mnemonic = format!("C{}{} 0x{:04X}", prefix, flag_name, target);
                if self.condition_met(ccc, sense) {
                    self.stack.push_and_jump(target);
                }
            }
        } else if decoded.is_input {
            let port = decoded.port as usize;
            let val = self.input_ports[port.min(7)];
            self.regs.write(REG_A, val);
            mnemonic = format!("IN {}", port);
        } else if decoded.is_mov {
            let src_val = self.reg_read(sss as usize);
            if sss as usize == REG_M {
                mem_address = Some(self.hl_address());
                mem_value = Some(src_val);
            }
            if ddd as usize == REG_M {
                let addr = self.hl_address();
                self.mem_write(addr, src_val);
                mem_address = Some(addr);
                mem_value = Some(src_val);
            } else {
                self.regs.write(ddd as usize, src_val);
            }
            mnemonic = format!("MOV {}, {}", REG_NAMES[ddd as usize], REG_NAMES[sss as usize]);
        } else if decoded.is_alu {
            let alu_code = ddd as usize;
            let a = self.regs.read(REG_A);
            let src_val = if decoded.is_alu_immediate {
                let data = self.fetch_byte();
                raw.push(data);
                data
            } else {
                let v = self.reg_read(sss as usize);
                if sss as usize == REG_M {
                    mem_address = Some(self.hl_address());
                    mem_value = Some(v);
                }
                v
            };

            let (result, alu_flags) = match alu_code {
                0 => GateAlu8::add(a, src_val),
                1 => GateAlu8::add_with_carry(a, src_val, self.flags.carry),
                2 => GateAlu8::subtract(a, src_val),
                3 => GateAlu8::subtract_with_borrow(a, src_val, self.flags.carry),
                4 => GateAlu8::and(a, src_val),
                5 => GateAlu8::xor(a, src_val),
                6 => GateAlu8::or(a, src_val),
                7 => GateAlu8::subtract(a, src_val), // CMP: compute flags but don't write
                _ => unreachable!(),
            };

            let new_flags = self.to_flags(alu_flags, false);
            self.flags = new_flags;

            // CMP (alu_code=7) does not write result back to A
            if alu_code != 7 {
                self.regs.write(REG_A, result);
            }

            let mnems = if decoded.is_alu_immediate { ALU_IMM_MNEMONICS } else { ALU_MNEMONICS };
            let src_name = if decoded.is_alu_immediate {
                format!("0x{:02X}", src_val)
            } else {
                REG_NAMES[sss as usize].to_string()
            };
            mnemonic = format!("{} {}", mnems[alu_code], src_name);
        } else {
            return Err(format!("Unknown opcode 0x{:02X} at 0x{:04X}", opcode, fetch_pc));
        }

        Ok(Trace {
            address: fetch_pc,
            raw,
            mnemonic,
            a_before,
            a_after: self.regs.read(REG_A),
            flags_before,
            flags_after: self.flags,
            mem_address,
            mem_value,
        })
    }

    /// Load a program and run for up to `max_steps` instructions.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Vec<Trace> {
        self.reset();
        self.load_program(program, 0);
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step() {
                Ok(t) => {
                    let halted = self.halted;
                    traces.push(t);
                    if halted { break; }
                }
                Err(_) => break,
            }
        }
        traces
    }

    /// Return a map of component name → estimated gate count.
    ///
    /// This gives insight into the chip area breakdown — how many gates
    /// each subsystem requires. Matches the block diagram in the spec.
    pub fn gate_count(&self) -> HashMap<&'static str, usize> {
        let mut m = HashMap::new();
        m.insert("alu_8bit_adder",   40);  // 8 full-adders × 5 gates
        m.insert("alu_parity_tree",   7);  // 7-gate XOR chain for Parity flag
        m.insert("register_file",   336);  // 7 × 8 bits × 6 gates/flip-flop
        m.insert("accumulator",      48);  // 8 bits × 6 gates/flip-flop
        m.insert("stack_8x14",      672);  // 8 × 14 bits × 6 gates/flip-flop
        m.insert("program_counter",  84);  // 14 bits × 6 gates/flip-flop
        m.insert("pc_incrementer",   28);  // 14 half-adders × 2 gates
        m.insert("decoder",          80);  // combinational AND/OR/NOT gate tree
        m.insert("control_wiring",  200);  // bus mux, control logic estimates
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_intel8008_simulator::Simulator;

    #[test]
    fn test_add_basic() {
        let mut cpu = GateLevelCpu::new();
        // MVI B,1; MVI A,2; ADD B; HLT
        let program = &[0x06u8, 0x01, 0x3E, 0x02, 0x80, 0x76];
        cpu.run(program, 100);
        assert_eq!(cpu.a(), 3);
        assert!(!cpu.flags().carry);
        assert!(!cpu.flags().zero);
        assert!(cpu.flags().parity); // 3 = 0b11 → even parity
    }

    #[test]
    fn test_multiply_loop() {
        let mut cpu = GateLevelCpu::new();
        let mut program = vec![0u8; 20];
        program[0] = 0x06; program[1] = 0x05;  // MVI B, 5
        program[2] = 0x0E; program[3] = 0x04;  // MVI C, 4
        program[4] = 0x3E; program[5] = 0x00;  // MVI A, 0
        program[6] = 0x80;                      // ADD B
        program[7] = 0x09;                      // DCR C
        program[8] = 0x48; program[9] = 0x06; program[10] = 0x00; // JFZ 6
        program[11] = 0x76;                     // HLT
        cpu.run(&program, 200);
        assert_eq!(cpu.a(), 20);
    }

    #[test]
    fn test_call_return() {
        let mut program = vec![0u8; 0x14];
        program[0] = 0x3E; program[1] = 0x00;
        program[2] = 0x7E; program[3] = 0x10; program[4] = 0x00;
        program[5] = 0x76;
        program[0x10] = 0x3E; program[0x11] = 0x2A;
        program[0x12] = 0x3F;
        let mut cpu = GateLevelCpu::new();
        cpu.run(&program, 100);
        assert_eq!(cpu.a(), 42);
    }

    #[test]
    fn test_cross_validate() {
        // Cross-validate gate-level CPU against behavioral simulator.
        // Both should produce identical trace results for every instruction.
        let program: &[u8] = &[
            0x06, 0x05,  // MVI B, 5
            0x0E, 0x04,  // MVI C, 4
            0x3E, 0x00,  // MVI A, 0
            0x80,        // ADD B (LOOP)
            0x09,        // DCR C
            0x48, 0x06, 0x00, // JFZ LOOP
            0x76,        // HLT
        ];

        let mut bsim = Simulator::new();
        let mut gsim = GateLevelCpu::new();

        let b_traces = bsim.run(program, 200);
        let g_traces = gsim.run(program, 200);

        assert_eq!(
            b_traces.len(),
            g_traces.len(),
            "trace length mismatch: behavioral={}, gate-level={}",
            b_traces.len(),
            g_traces.len()
        );

        for (i, (bt, gt)) in b_traces.iter().zip(g_traces.iter()).enumerate() {
            assert_eq!(
                bt.a_after, gt.a_after,
                "A mismatch at step {i}: behavioral={}, gate-level={}",
                bt.a_after, gt.a_after
            );
            assert_eq!(
                bt.flags_after, gt.flags_after,
                "flags mismatch at step {i}: behavioral={:?}, gate-level={:?}",
                bt.flags_after, gt.flags_after
            );
        }
    }

    #[test]
    fn test_cross_validate_subtract() {
        let program: &[u8] = &[
            0x3E, 0xFF,  // MVI A, 0xFF
            0xD4, 0x01,  // SUI 1
            0x76,        // HLT
        ];
        let mut bsim = Simulator::new();
        let mut gsim = GateLevelCpu::new();
        let b_traces = bsim.run(program, 100);
        let g_traces = gsim.run(program, 100);
        for (bt, gt) in b_traces.iter().zip(g_traces.iter()) {
            assert_eq!(bt.a_after, gt.a_after);
            assert_eq!(bt.flags_after, gt.flags_after);
        }
    }

    #[test]
    fn test_gate_count_available() {
        let cpu = GateLevelCpu::new();
        let counts = cpu.gate_count();
        assert!(counts.contains_key("alu_8bit_adder"));
        let total: usize = counts.values().sum();
        assert!(total > 1000, "total gate count should exceed 1000");
    }
}
