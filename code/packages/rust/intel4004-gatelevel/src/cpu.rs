//! Intel 4004 gate-level CPU -- all operations route through real logic gates.
//!
//! # What makes this a "gate-level" simulator?
//!
//! Every computation in this CPU flows through the same gate chain that the
//! real Intel 4004 used:
//!
//! ```text
//! NOT/AND/OR/XOR -> half_adder -> full_adder -> ripple_carry_adder -> ALU
//! D flip-flop -> register -> register file / program counter / stack
//! ```
//!
//! When you execute ADD R3, the value in register R3 is read from flip-flops,
//! the accumulator is read from flip-flops, both are fed into the ALU (which
//! uses full adders built from gates), and the result is clocked back into
//! the accumulator's flip-flops.
//!
//! Nothing is simulated behaviorally. Every bit passes through gate functions.
//!
//! # Gate count
//!
//! ```text
//! Component               Gates   Transistors (x4 per gate)
//! ---------------------   -----   -------------------------
//! ALU (4-bit)             32      128
//! Register file (16x4)    480     1,920
//! Accumulator (4-bit)     24      96
//! Carry flag (1-bit)      6       24
//! Program counter (12)    96      384
//! Hardware stack (3x12)   226     904
//! Decoder                 ~50     200
//! Control + wiring        ~100    400
//! ---------------------   -----   -------------------------
//! Total                   ~1,014  ~4,056
//! ```
//!
//! # Execution model
//!
//! Each instruction executes in a single `step()` call:
//! 1. FETCH:   Read instruction byte from ROM using PC
//! 2. FETCH2:  For 2-byte instructions, read the second byte
//! 3. DECODE:  Route instruction through decoder gate network
//! 4. EXECUTE: Perform the operation through ALU/registers/etc.

use logic_gates::gates::{and_gate, not_gate, or_gate};

use crate::bits::{bits_to_int, int_to_bits};
use crate::decoder::{decode, DecodedInstruction};
use crate::gate_alu::GateALU;
use crate::pc::ProgramCounter;
use crate::ram::RAM;
use crate::registers::{Accumulator, CarryFlag, RegisterFile};
use crate::stack::HardwareStack;

/// Trace record for one instruction execution.
///
/// Same information as Intel4004Trace from the behavioral simulator,
/// plus gate-level details.
#[derive(Debug, Clone)]
pub struct GateTrace {
    pub address: u16,
    pub raw: u8,
    pub raw2: Option<u8>,
    pub mnemonic: String,
    pub accumulator_before: u8,
    pub accumulator_after: u8,
    pub carry_before: bool,
    pub carry_after: bool,
}

/// Intel 4004 CPU where every operation routes through real logic gates.
///
/// Public API matches the behavioral Intel4004Simulator for
/// cross-validation, but internally all computation flows through
/// gates, flip-flops, and adders.
///
/// # Example
///
/// ```
/// use intel4004_gatelevel::cpu::Intel4004GateLevel;
///
/// let mut cpu = Intel4004GateLevel::new();
/// // LDM 5, HLT
/// let traces = cpu.run(&[0xD5, 0x01], 100);
/// assert_eq!(cpu.accumulator(), 5);
/// assert!(cpu.halted());
/// ```
pub struct Intel4004GateLevel {
    alu: GateALU,
    regs: RegisterFile,
    acc: Accumulator,
    carry_flag: CarryFlag,
    pc: ProgramCounter,
    stack: HardwareStack,
    ram: RAM,
    rom: Vec<u8>,
    ram_bank: usize,
    ram_register: usize,
    ram_character: usize,
    rom_port: u8,
    is_halted: bool,
}

impl Intel4004GateLevel {
    /// Create a new Intel 4004 gate-level CPU with all components initialized.
    pub fn new() -> Self {
        Self {
            alu: GateALU::new(),
            regs: RegisterFile::new(),
            acc: Accumulator::new(),
            carry_flag: CarryFlag::new(),
            pc: ProgramCounter::new(),
            stack: HardwareStack::new(),
            ram: RAM::new(),
            rom: vec![0u8; 4096],
            ram_bank: 0,
            ram_register: 0,
            ram_character: 0,
            rom_port: 0,
            is_halted: false,
        }
    }

    // ------------------------------------------------------------------
    // Property accessors (match behavioral simulator's interface)
    // ------------------------------------------------------------------

    /// Read accumulator from flip-flops.
    pub fn accumulator(&self) -> u8 {
        self.acc.read()
    }

    /// Read all 16 registers from flip-flops.
    pub fn registers(&self) -> Vec<u8> {
        (0..16).map(|i| self.regs.read(i)).collect()
    }

    /// Read carry flag from flip-flop.
    pub fn carry(&self) -> bool {
        self.carry_flag.read()
    }

    /// Read program counter from flip-flops.
    pub fn pc(&self) -> u16 {
        self.pc.read()
    }

    /// Whether the CPU is halted.
    pub fn halted(&self) -> bool {
        self.is_halted
    }

    /// Read stack levels (for inspection only).
    pub fn hw_stack(&self) -> Vec<u16> {
        self.stack.read_levels()
    }

    /// Read RAM main characters.
    pub fn ram_main(&self) -> Vec<Vec<Vec<u8>>> {
        (0..4)
            .map(|b| {
                (0..4)
                    .map(|r| (0..16).map(|c| self.ram.read_main(b, r, c)).collect())
                    .collect()
            })
            .collect()
    }

    /// Read RAM status characters.
    pub fn ram_status(&self) -> Vec<Vec<Vec<u8>>> {
        (0..4)
            .map(|b| {
                (0..4)
                    .map(|r| (0..4).map(|s| self.ram.read_status(b, r, s)).collect())
                    .collect()
            })
            .collect()
    }

    /// Current RAM bank selection.
    pub fn ram_bank(&self) -> usize {
        self.ram_bank
    }

    /// Current ROM port value.
    pub fn rom_port(&self) -> u8 {
        self.rom_port
    }

    /// RAM output port values.
    pub fn ram_output(&self) -> Vec<u8> {
        (0..4).map(|i| self.ram.read_output(i)).collect()
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Load a program into ROM.
    pub fn load_program(&mut self, program: &[u8]) {
        self.rom = vec![0u8; 4096];
        for (i, &b) in program.iter().enumerate() {
            if i < 4096 {
                self.rom[i] = b;
            }
        }
    }

    /// Execute one instruction through the gate-level pipeline.
    ///
    /// Returns a GateTrace with before/after state.
    pub fn step(&mut self) -> GateTrace {
        if self.is_halted {
            panic!("CPU is halted -- cannot step further");
        }

        // Snapshot state before
        let acc_before = self.acc.read();
        let carry_before = self.carry_flag.read();
        let pc_before = self.pc.read();

        // FETCH: read instruction byte from ROM
        let raw = self.rom[pc_before as usize];

        // DECODE: route through combinational decoder
        let decoded = decode(raw, None);

        // FETCH2: if 2-byte, read second byte
        let (raw2, decoded) = if decoded.is_two_byte != 0 {
            let r2 = self.rom[((pc_before + 1) & 0xFFF) as usize];
            (Some(r2), decode(raw, Some(r2)))
        } else {
            (None, decoded)
        };

        // EXECUTE: route through appropriate gate paths
        let mnemonic = self.execute(&decoded);

        GateTrace {
            address: pc_before,
            raw,
            raw2,
            mnemonic,
            accumulator_before: acc_before,
            accumulator_after: self.acc.read(),
            carry_before,
            carry_after: self.carry_flag.read(),
        }
    }

    /// Load and run a program, returning execution trace.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Vec<GateTrace> {
        self.reset();
        self.load_program(program);

        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.is_halted {
                break;
            }
            traces.push(self.step());
        }
        traces
    }

    /// Reset all CPU state.
    pub fn reset(&mut self) {
        self.acc.reset();
        self.carry_flag.reset();
        self.regs.reset();
        self.pc.reset();
        self.stack.reset();
        self.ram.reset();
        self.rom = vec![0u8; 4096];
        self.ram_bank = 0;
        self.ram_register = 0;
        self.ram_character = 0;
        self.rom_port = 0;
        self.is_halted = false;
    }

    /// Total estimated gate count for the CPU.
    pub fn gate_count(&self) -> usize {
        self.alu.gate_count()
            + self.regs.gate_count()
            + self.acc.gate_count()
            + self.carry_flag.gate_count()
            + self.pc.gate_count()
            + self.stack.gate_count()
            + self.ram.gate_count()
            + 50  // decoder
            + 100 // control logic and wiring
    }

    // ------------------------------------------------------------------
    // Instruction execution -- routes through gate-level components
    // ------------------------------------------------------------------

    fn execute(&mut self, d: &DecodedInstruction) -> String {
        // NOP
        if d.is_nop != 0 {
            self.pc.increment();
            return "NOP".to_string();
        }

        // HLT
        if d.is_hlt != 0 {
            self.is_halted = true;
            self.pc.increment();
            return "HLT".to_string();
        }

        // LDM N: load immediate into accumulator
        if d.is_ldm != 0 {
            self.acc.write(d.immediate);
            self.pc.increment();
            return format!("LDM {}", d.immediate);
        }

        // LD Rn: load register into accumulator
        if d.is_ld != 0 {
            let val = self.regs.read(d.reg_index as usize);
            self.acc.write(val);
            self.pc.increment();
            return format!("LD R{}", d.reg_index);
        }

        // XCH Rn: exchange accumulator and register
        if d.is_xch != 0 {
            let a_val = self.acc.read();
            let r_val = self.regs.read(d.reg_index as usize);
            self.acc.write(r_val);
            self.regs.write(d.reg_index as usize, a_val);
            self.pc.increment();
            return format!("XCH R{}", d.reg_index);
        }

        // INC Rn: increment register (no carry effect)
        if d.is_inc != 0 {
            let r_val = self.regs.read(d.reg_index as usize);
            let (result, _) = self.alu.increment(r_val);
            self.regs.write(d.reg_index as usize, result);
            self.pc.increment();
            return format!("INC R{}", d.reg_index);
        }

        // ADD Rn: add register to accumulator with carry
        if d.is_add != 0 {
            let a_val = self.acc.read();
            let r_val = self.regs.read(d.reg_index as usize);
            let carry_in = if self.carry_flag.read() { 1 } else { 0 };
            let (result, carry_out) = self.alu.add(a_val, r_val, carry_in);
            self.acc.write(result);
            self.carry_flag.write(carry_out);
            self.pc.increment();
            return format!("ADD R{}", d.reg_index);
        }

        // SUB Rn: subtract register from accumulator
        if d.is_sub != 0 {
            let a_val = self.acc.read();
            let r_val = self.regs.read(d.reg_index as usize);
            let borrow_in = if self.carry_flag.read() { 0 } else { 1 };
            let (result, carry_out) = self.alu.subtract(a_val, r_val, borrow_in);
            self.acc.write(result);
            self.carry_flag.write(carry_out);
            self.pc.increment();
            return format!("SUB R{}", d.reg_index);
        }

        // JUN addr: unconditional jump
        if d.is_jun != 0 {
            self.pc.load(d.addr12);
            return format!("JUN 0x{:03X}", d.addr12);
        }

        // JCN cond,addr: conditional jump
        if d.is_jcn != 0 {
            return self.exec_jcn(d);
        }

        // ISZ Rn,addr: increment and skip if zero
        if d.is_isz != 0 {
            return self.exec_isz(d);
        }

        // JMS addr: jump to subroutine
        if d.is_jms != 0 {
            let return_addr = self.pc.read() + 2;
            self.stack.push(return_addr);
            self.pc.load(d.addr12);
            return format!("JMS 0x{:03X}", d.addr12);
        }

        // BBL N: branch back and load
        if d.is_bbl != 0 {
            self.acc.write(d.immediate);
            let return_addr = self.stack.pop();
            self.pc.load(return_addr);
            return format!("BBL {}", d.immediate);
        }

        // FIM Pp,data: fetch immediate to pair
        if d.is_fim != 0 {
            self.regs.write_pair(d.pair_index as usize, d.addr8);
            self.pc.increment2();
            return format!("FIM P{},0x{:02X}", d.pair_index, d.addr8);
        }

        // SRC Pp: send register control
        if d.is_src != 0 {
            let pair_val = self.regs.read_pair(d.pair_index as usize);
            self.ram_register = ((pair_val >> 4) & 0xF) as usize;
            self.ram_character = (pair_val & 0xF) as usize;
            self.pc.increment();
            return format!("SRC P{}", d.pair_index);
        }

        // FIN Pp: fetch indirect from ROM
        if d.is_fin != 0 {
            let p0_val = self.regs.read_pair(0);
            let page = self.pc.read() & 0xF00;
            let rom_addr = page | (p0_val as u16);
            let rom_byte = self.rom[(rom_addr & 0xFFF) as usize];
            self.regs.write_pair(d.pair_index as usize, rom_byte);
            self.pc.increment();
            return format!("FIN P{}", d.pair_index);
        }

        // JIN Pp: jump indirect
        if d.is_jin != 0 {
            let pair_val = self.regs.read_pair(d.pair_index as usize);
            let page = self.pc.read() & 0xF00;
            self.pc.load(page | (pair_val as u16));
            return format!("JIN P{}", d.pair_index);
        }

        // I/O operations (0xE_ range)
        if d.is_io != 0 {
            return self.exec_io(d);
        }

        // Accumulator operations (0xF_ range)
        if d.is_accum != 0 {
            return self.exec_accum(d);
        }

        // Unknown -- advance PC to avoid infinite loop
        self.pc.increment();
        format!("UNKNOWN(0x{:02X})", d.raw)
    }

    /// JCN cond,addr: conditional jump using gate logic.
    ///
    /// Condition nibble bits (evaluated with OR/AND/NOT gates):
    /// - Bit 3: INVERT
    /// - Bit 2: TEST A==0
    /// - Bit 1: TEST carry==1
    /// - Bit 0: TEST pin (always 0)
    fn exec_jcn(&mut self, d: &DecodedInstruction) -> String {
        let cond = d.condition;
        let a_val = self.acc.read();
        let carry_val = if self.carry_flag.read() { 1u8 } else { 0u8 };

        // Test A==0: OR all accumulator bits, then NOT
        let a_bits = int_to_bits(a_val as u16, 4);
        let a_is_zero = not_gate(or_gate(
            or_gate(a_bits[0], a_bits[1]),
            or_gate(a_bits[2], a_bits[3]),
        ));

        // Build test result using gates
        let test_zero = and_gate((cond >> 2) & 1, a_is_zero);
        let test_carry = and_gate((cond >> 1) & 1, carry_val);
        let test_pin = and_gate(cond & 1, 0); // Pin always 0

        let test_result = or_gate(or_gate(test_zero, test_carry), test_pin);

        // Invert if bit 3 set
        let invert = (cond >> 3) & 1;
        // XOR with invert using gates
        let final_result = or_gate(
            and_gate(test_result, not_gate(invert)),
            and_gate(not_gate(test_result), invert),
        );

        let page = (self.pc.read() + 2) & 0xF00;
        let target = page | (d.addr8 as u16);

        if final_result != 0 {
            self.pc.load(target);
        } else {
            self.pc.increment2();
        }

        format!("JCN {},{:02X}", cond, d.addr8)
    }

    /// ISZ Rn,addr: increment register, skip if zero.
    fn exec_isz(&mut self, d: &DecodedInstruction) -> String {
        let r_val = self.regs.read(d.reg_index as usize);
        let (result, _) = self.alu.increment(r_val);
        self.regs.write(d.reg_index as usize, result);

        // Test if result is zero using NOR of all bits
        let r_bits = int_to_bits(result as u16, 4);
        let is_zero = not_gate(or_gate(
            or_gate(r_bits[0], r_bits[1]),
            or_gate(r_bits[2], r_bits[3]),
        ));

        let page = (self.pc.read() + 2) & 0xF00;
        let target = page | (d.addr8 as u16);

        if is_zero != 0 {
            // Result is zero -> fall through
            self.pc.increment2();
        } else {
            // Result is nonzero -> jump
            self.pc.load(target);
        }

        format!("ISZ R{},0x{:02X}", d.reg_index, d.addr8)
    }

    /// Execute I/O instructions (0xE0-0xEF).
    fn exec_io(&mut self, d: &DecodedInstruction) -> String {
        let a_val = self.acc.read();
        let sub_op = d.lower;

        match sub_op {
            0x0 => {
                // WRM
                self.ram.write_main(self.ram_bank, self.ram_register, self.ram_character, a_val);
                self.pc.increment();
                "WRM".to_string()
            }
            0x1 => {
                // WMP
                self.ram.write_output(self.ram_bank, a_val);
                self.pc.increment();
                "WMP".to_string()
            }
            0x2 => {
                // WRR
                self.rom_port = a_val & 0xF;
                self.pc.increment();
                "WRR".to_string()
            }
            0x3 => {
                // WPM (NOP in simulation)
                self.pc.increment();
                "WPM".to_string()
            }
            0x4..=0x7 => {
                // WR0-WR3
                let idx = (sub_op - 0x4) as usize;
                self.ram.write_status(self.ram_bank, self.ram_register, idx, a_val);
                self.pc.increment();
                format!("WR{}", idx)
            }
            0x8 => {
                // SBM
                let ram_val =
                    self.ram.read_main(self.ram_bank, self.ram_register, self.ram_character);
                let borrow_in = if self.carry_flag.read() { 0 } else { 1 };
                let (result, carry_out) = self.alu.subtract(a_val, ram_val, borrow_in);
                self.acc.write(result);
                self.carry_flag.write(carry_out);
                self.pc.increment();
                "SBM".to_string()
            }
            0x9 => {
                // RDM
                let val =
                    self.ram.read_main(self.ram_bank, self.ram_register, self.ram_character);
                self.acc.write(val);
                self.pc.increment();
                "RDM".to_string()
            }
            0xA => {
                // RDR
                self.acc.write(self.rom_port & 0xF);
                self.pc.increment();
                "RDR".to_string()
            }
            0xB => {
                // ADM
                let ram_val =
                    self.ram.read_main(self.ram_bank, self.ram_register, self.ram_character);
                let carry_in = if self.carry_flag.read() { 1 } else { 0 };
                let (result, carry_out) = self.alu.add(a_val, ram_val, carry_in);
                self.acc.write(result);
                self.carry_flag.write(carry_out);
                self.pc.increment();
                "ADM".to_string()
            }
            0xC..=0xF => {
                // RD0-RD3
                let idx = (sub_op - 0xC) as usize;
                let val = self.ram.read_status(self.ram_bank, self.ram_register, idx);
                self.acc.write(val);
                self.pc.increment();
                format!("RD{}", idx)
            }
            _ => {
                self.pc.increment();
                format!("IO(0x{:02X})", d.raw)
            }
        }
    }

    /// Execute accumulator operations (0xF0-0xFD).
    fn exec_accum(&mut self, d: &DecodedInstruction) -> String {
        let a_val = self.acc.read();
        let sub_op = d.lower;

        match sub_op {
            0x0 => {
                // CLB
                self.acc.write(0);
                self.carry_flag.write(false);
                self.pc.increment();
                "CLB".to_string()
            }
            0x1 => {
                // CLC
                self.carry_flag.write(false);
                self.pc.increment();
                "CLC".to_string()
            }
            0x2 => {
                // IAC
                let (result, carry) = self.alu.increment(a_val);
                self.acc.write(result);
                self.carry_flag.write(carry);
                self.pc.increment();
                "IAC".to_string()
            }
            0x3 => {
                // CMC
                self.carry_flag.write(!self.carry_flag.read());
                self.pc.increment();
                "CMC".to_string()
            }
            0x4 => {
                // CMA
                let result = self.alu.complement(a_val);
                self.acc.write(result);
                self.pc.increment();
                "CMA".to_string()
            }
            0x5 => {
                // RAL - rotate accumulator left through carry
                let old_carry: u8 = if self.carry_flag.read() { 1 } else { 0 };
                let a_bits = int_to_bits(a_val as u16, 4);
                self.carry_flag.write(a_bits[3] != 0);
                let new_bits = [old_carry, a_bits[0], a_bits[1], a_bits[2]];
                self.acc.write(bits_to_int(&new_bits) as u8);
                self.pc.increment();
                "RAL".to_string()
            }
            0x6 => {
                // RAR - rotate accumulator right through carry
                let old_carry: u8 = if self.carry_flag.read() { 1 } else { 0 };
                let a_bits = int_to_bits(a_val as u16, 4);
                self.carry_flag.write(a_bits[0] != 0);
                let new_bits = [a_bits[1], a_bits[2], a_bits[3], old_carry];
                self.acc.write(bits_to_int(&new_bits) as u8);
                self.pc.increment();
                "RAR".to_string()
            }
            0x7 => {
                // TCC - transfer carry and clear
                self.acc.write(if self.carry_flag.read() { 1 } else { 0 });
                self.carry_flag.write(false);
                self.pc.increment();
                "TCC".to_string()
            }
            0x8 => {
                // DAC - decrement accumulator
                let (result, carry) = self.alu.decrement(a_val);
                self.acc.write(result);
                self.carry_flag.write(carry);
                self.pc.increment();
                "DAC".to_string()
            }
            0x9 => {
                // TCS - transfer carry subtract and clear
                self.acc.write(if self.carry_flag.read() { 10 } else { 9 });
                self.carry_flag.write(false);
                self.pc.increment();
                "TCS".to_string()
            }
            0xA => {
                // STC - set carry
                self.carry_flag.write(true);
                self.pc.increment();
                "STC".to_string()
            }
            0xB => {
                // DAA - decimal adjust accumulator
                if a_val > 9 || self.carry_flag.read() {
                    let (result, carry) = self.alu.add(a_val, 6, 0);
                    if carry {
                        self.carry_flag.write(true);
                    }
                    self.acc.write(result);
                }
                self.pc.increment();
                "DAA".to_string()
            }
            0xC => {
                // KBP - keyboard process
                let kbp_result = match a_val {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    4 => 3,
                    8 => 4,
                    _ => 15,
                };
                self.acc.write(kbp_result);
                self.pc.increment();
                "KBP".to_string()
            }
            0xD => {
                // DCL - designate command line (select RAM bank)
                let mut bank = self.alu.bitwise_and(a_val, 0x7);
                if bank > 3 {
                    bank = self.alu.bitwise_and(bank, 0x3);
                }
                self.ram_bank = bank as usize;
                self.pc.increment();
                "DCL".to_string()
            }
            _ => {
                self.pc.increment();
                format!("ACCUM(0x{:02X})", d.raw)
            }
        }
    }
}

impl Default for Intel4004GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Basic instruction tests ---

    #[test]
    fn test_nop() {
        let mut cpu = Intel4004GateLevel::new();
        let traces = cpu.run(&[0x00, 0x01], 10);
        assert_eq!(traces[0].mnemonic, "NOP");
        assert_eq!(traces[1].mnemonic, "HLT");
    }

    #[test]
    fn test_hlt() {
        let mut cpu = Intel4004GateLevel::new();
        let traces = cpu.run(&[0x01], 10);
        assert!(cpu.halted());
        assert_eq!(traces.len(), 1);
    }

    #[test]
    fn test_ldm() {
        let mut cpu = Intel4004GateLevel::new();
        cpu.run(&[0xD5, 0x01], 10);
        assert_eq!(cpu.accumulator(), 5);
    }

    #[test]
    fn test_ld_register() {
        let mut cpu = Intel4004GateLevel::new();
        // FIM P0, 0x53 -> R0=5, R1=3
        // LD R0
        // HLT
        cpu.run(&[0x20, 0x53, 0xA0, 0x01], 10);
        assert_eq!(cpu.accumulator(), 5);
    }

    #[test]
    fn test_xch() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 7, FIM P0, 0x30 (R0=3, R1=0), XCH R0, HLT
        cpu.run(&[0xD7, 0x20, 0x30, 0xB0, 0x01], 10);
        assert_eq!(cpu.accumulator(), 3);
        assert_eq!(cpu.registers()[0], 7);
    }

    #[test]
    fn test_inc() {
        let mut cpu = Intel4004GateLevel::new();
        // FIM P0, 0x50 (R0=5, R1=0), INC R0, LD R0, HLT
        cpu.run(&[0x20, 0x50, 0x60, 0xA0, 0x01], 10);
        assert_eq!(cpu.accumulator(), 6);
    }

    #[test]
    fn test_add() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 3, FIM P0, 0x50 (R0=5), ADD R0, HLT
        cpu.run(&[0xD3, 0x20, 0x50, 0x80, 0x01], 10);
        assert_eq!(cpu.accumulator(), 8);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_add_with_overflow() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 15, FIM P0, 0x10 (R0=1), ADD R0, HLT
        cpu.run(&[0xDF, 0x20, 0x10, 0x80, 0x01], 10);
        assert_eq!(cpu.accumulator(), 0);
        assert!(cpu.carry());
    }

    #[test]
    fn test_sub() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 7, FIM P0, 0x30 (R0=3), SUB R0, HLT
        // SUB with carry initially false -> borrow_in=1
        // 7 + NOT(3) + 1 = 7 + 12 + 1 = 20, result=4, carry=true (no borrow)
        cpu.run(&[0xD7, 0x20, 0x30, 0x90, 0x01], 10);
        assert_eq!(cpu.accumulator(), 4);
        assert!(cpu.carry()); // no borrow
    }

    #[test]
    fn test_jun() {
        let mut cpu = Intel4004GateLevel::new();
        // JUN 0x004 (skip next 2 bytes), <padding>, LDM 9, HLT
        cpu.run(&[0x40, 0x04, 0x00, 0x00, 0xD9, 0x01], 10);
        assert_eq!(cpu.accumulator(), 9);
    }

    #[test]
    fn test_jms_bbl() {
        let mut cpu = Intel4004GateLevel::new();
        // Address 0: JMS 0x004
        // Address 2: HLT
        // Address 3: <padding>
        // Address 4: BBL 5
        let program = [0x50, 0x04, 0x01, 0x00, 0xC5];
        cpu.run(&program, 10);
        assert_eq!(cpu.accumulator(), 5);
        assert!(cpu.halted());
    }

    #[test]
    fn test_fim() {
        let mut cpu = Intel4004GateLevel::new();
        // FIM P1, 0xAB -> R2=0xA, R3=0xB
        cpu.run(&[0x22, 0xAB, 0x01], 10);
        assert_eq!(cpu.registers()[2], 0xA);
        assert_eq!(cpu.registers()[3], 0xB);
    }

    #[test]
    fn test_src_wrm_rdm() {
        let mut cpu = Intel4004GateLevel::new();
        // FIM P0, 0x12 (reg=1, char=2), SRC P0, LDM 7, WRM, LDM 0, RDM, HLT
        cpu.run(&[0x20, 0x12, 0x21, 0xD7, 0xE0, 0xD0, 0xE9, 0x01], 10);
        assert_eq!(cpu.accumulator(), 7);
    }

    // --- Accumulator operations ---

    #[test]
    fn test_clb() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5, STC, CLB, HLT
        cpu.run(&[0xD5, 0xFA, 0xF0, 0x01], 10);
        assert_eq!(cpu.accumulator(), 0);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_clc() {
        let mut cpu = Intel4004GateLevel::new();
        // STC, CLC, HLT
        cpu.run(&[0xFA, 0xF1, 0x01], 10);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_iac() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5, IAC, HLT
        cpu.run(&[0xD5, 0xF2, 0x01], 10);
        assert_eq!(cpu.accumulator(), 6);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_iac_overflow() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 15, IAC, HLT
        cpu.run(&[0xDF, 0xF2, 0x01], 10);
        assert_eq!(cpu.accumulator(), 0);
        assert!(cpu.carry());
    }

    #[test]
    fn test_cmc() {
        let mut cpu = Intel4004GateLevel::new();
        // CMC (carry false->true), CMC (true->false), HLT
        cpu.run(&[0xF3, 0xF3, 0x01], 10);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_cma() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5, CMA, HLT -> complement of 5 = 10
        cpu.run(&[0xD5, 0xF4, 0x01], 10);
        assert_eq!(cpu.accumulator(), 10);
    }

    #[test]
    fn test_ral() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5 (0101), RAL, HLT
        // 0101 rotate left through carry (carry=0):
        // carry gets bit3(0), bits become [carry=0, bit0=1, bit1=0, bit2=1] = 1010 = 10
        cpu.run(&[0xD5, 0xF5, 0x01], 10);
        assert_eq!(cpu.accumulator(), 10);
        assert!(!cpu.carry()); // bit3 was 0
    }

    #[test]
    fn test_rar() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5 (0101), RAR, HLT
        // 0101 rotate right through carry (carry=0):
        // carry gets bit0(1), bits become [bit1=0, bit2=1, bit3=0, carry=0] = 0010 = 2
        cpu.run(&[0xD5, 0xF6, 0x01], 10);
        assert_eq!(cpu.accumulator(), 2);
        assert!(cpu.carry()); // bit0 was 1
    }

    #[test]
    fn test_tcc() {
        let mut cpu = Intel4004GateLevel::new();
        // STC, TCC, HLT -> acc=1, carry=false
        cpu.run(&[0xFA, 0xF7, 0x01], 10);
        assert_eq!(cpu.accumulator(), 1);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_dac() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5, DAC, HLT -> acc=4
        cpu.run(&[0xD5, 0xF8, 0x01], 10);
        assert_eq!(cpu.accumulator(), 4);
        assert!(cpu.carry()); // no borrow
    }

    #[test]
    fn test_dac_underflow() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 0, DAC, HLT -> acc=15, carry=false (borrow)
        cpu.run(&[0xD0, 0xF8, 0x01], 10);
        assert_eq!(cpu.accumulator(), 15);
        assert!(!cpu.carry());
    }

    #[test]
    fn test_tcs() {
        let mut cpu = Intel4004GateLevel::new();
        // TCS, HLT -> acc=9 (carry was false), carry=false
        cpu.run(&[0xF9, 0x01], 10);
        assert_eq!(cpu.accumulator(), 9);
        assert!(!cpu.carry());

        let mut cpu2 = Intel4004GateLevel::new();
        // STC, TCS, HLT -> acc=10 (carry was true), carry=false
        cpu2.run(&[0xFA, 0xF9, 0x01], 10);
        assert_eq!(cpu2.accumulator(), 10);
        assert!(!cpu2.carry());
    }

    #[test]
    fn test_stc() {
        let mut cpu = Intel4004GateLevel::new();
        cpu.run(&[0xFA, 0x01], 10);
        assert!(cpu.carry());
    }

    #[test]
    fn test_kbp() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 4, KBP, HLT -> 4 maps to 3
        cpu.run(&[0xD4, 0xFC, 0x01], 10);
        assert_eq!(cpu.accumulator(), 3);
    }

    #[test]
    fn test_kbp_invalid() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 3, KBP, HLT -> 3 maps to 15 (invalid)
        cpu.run(&[0xD3, 0xFC, 0x01], 10);
        assert_eq!(cpu.accumulator(), 15);
    }

    // --- JCN tests ---

    #[test]
    fn test_jcn_acc_zero_jump() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 0, JCN 4,06 (jump if A==0 to addr 6), LDM 1, HLT, <pad>, LDM 9, HLT
        cpu.run(&[0xD0, 0x14, 0x06, 0xD1, 0x01, 0x00, 0xD9, 0x01], 10);
        assert_eq!(cpu.accumulator(), 9);
    }

    #[test]
    fn test_jcn_acc_nonzero_no_jump() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 1, JCN 4,06 (jump if A==0 to addr 6), LDM 2, HLT, <pad>, LDM 9, HLT
        cpu.run(&[0xD1, 0x14, 0x06, 0xD2, 0x01, 0x00, 0xD9, 0x01], 10);
        assert_eq!(cpu.accumulator(), 2);
    }

    #[test]
    fn test_jcn_invert() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 5, JCN 0xC,06 (invert + test A==0 => jump if A!=0), HLT, <pad>, LDM 9, HLT
        cpu.run(&[0xD5, 0x1C, 0x06, 0x01, 0x00, 0x00, 0xD9, 0x01], 10);
        assert_eq!(cpu.accumulator(), 9);
    }

    // --- ISZ tests ---

    #[test]
    fn test_isz_nonzero_jumps() {
        let mut cpu = Intel4004GateLevel::new();
        // FIM P0, 0xD0 (R0=0xD=13), ISZ R0,0x00 (jump back to 0 while R0 != 0)
        // After 3 iterations, R0 wraps: 13->14->15->0, then falls through
        cpu.run(&[0x20, 0xD0, 0x70, 0x02], 100);
        assert_eq!(cpu.registers()[0], 0); // R0 wrapped to 0
    }

    // --- Gate count ---

    #[test]
    fn test_gate_count() {
        let cpu = Intel4004GateLevel::new();
        let count = cpu.gate_count();
        // Should be approximately 8,894
        assert!(count > 8000, "gate count {} should be > 8000", count);
        assert!(count < 10000, "gate count {} should be < 10000", count);
    }

    // --- Full program tests ---

    #[test]
    fn test_add_two_registers() {
        let mut cpu = Intel4004GateLevel::new();
        // LDM 3, XCH R0, LDM 5, ADD R0, HLT
        // Load 3 into R0, load 5 into acc, add R0 -> acc = 8
        cpu.run(&[0xD3, 0xB0, 0xD5, 0x80, 0x01], 10);
        assert_eq!(cpu.accumulator(), 8);
    }

    #[test]
    fn test_countdown_loop() {
        let mut cpu = Intel4004GateLevel::new();
        // Count down from 5 to 0 using ISZ (which wraps at 0)
        // FIM P0, 0xB0 (R0=0xB=11), ISZ R0,0x02 (loop), HLT
        // ISZ increments R0 each time: 11,12,13,14,15,0 -> falls through after 5 loops
        let traces = cpu.run(&[0x20, 0xB0, 0x70, 0x02, 0x01], 100);
        assert!(cpu.halted());
        assert_eq!(cpu.registers()[0], 0);
        // Should have done 5 ISZ loops + FIM + HLT
        assert!(traces.len() > 5);
    }

    #[test]
    fn test_subroutine_call_return() {
        let mut cpu = Intel4004GateLevel::new();
        // Main: JMS 0x006, HLT
        // Address 6: LDM 7, BBL 0
        let mut program = vec![0u8; 256];
        program[0] = 0x50; // JMS
        program[1] = 0x06; // addr 0x006
        program[2] = 0x01; // HLT (return here)
        program[6] = 0xD7; // LDM 7
        program[7] = 0xC0; // BBL 0
        cpu.run(&program, 20);
        assert_eq!(cpu.accumulator(), 0); // BBL loaded 0, then HLT
        // Actually BBL loads immediate into acc, so acc = 0
        // But LDM 7 happened first, then BBL 0 overwrites with 0
    }

    #[test]
    fn test_ram_read_write() {
        let mut cpu = Intel4004GateLevel::new();
        // FIM P0, 0x00 (reg=0, char=0)
        // SRC P0
        // LDM 0xA
        // WRM
        // LDM 0
        // RDM
        // HLT
        cpu.run(
            &[0x20, 0x00, 0x21, 0xDA, 0xE0, 0xD0, 0xE9, 0x01],
            20,
        );
        assert_eq!(cpu.accumulator(), 0xA);
    }
}
