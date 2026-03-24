//! # ARM1 Gate-Level Simulator
//!
//! Every arithmetic operation routes through actual logic gate functions -- AND,
//! OR, XOR, NOT -- chained into adders, then into a 32-bit ALU. The barrel
//! shifter is built from multiplexer trees. This is NOT the same as the
//! behavioral simulator (arm1-simulator crate).
//!
//! Both produce identical results for any program. The difference is the
//! execution path:
//!
//! ```text
//! Behavioral:  opcode -> match -> host arithmetic -> result
//! Gate-level:  opcode -> decoder gates -> barrel shifter muxes -> ALU gates -> result
//! ```
//!
//! # Architecture
//!
//! The gate-level simulator composes packages from layers below:
//! - logic-gates: AND, OR, XOR, NOT, MUX, D flip-flop, register
//! - arithmetic: half_adder, full_adder, ripple_carry_adder, ALU
//! - arm1-simulator: types, condition codes, instruction encoding helpers

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate, xnor_gate};
use logic_gates::combinational::mux2;
use arithmetic::adders::ripple_carry_adder_with_carry;

use arm1_simulator::{
    Flags, Trace, MemoryAccess, DecodedInstruction, InstType,
    FLAG_I, FLAG_F, MODE_SVC, MODE_FIQ, MODE_IRQ, MODE_MASK, PC_MASK, HALT_SWI,
    OP_MOV, OP_MVN,
    COND_EQ, COND_NE, COND_CS, COND_CC, COND_MI, COND_PL,
    COND_VS, COND_VC, COND_HI, COND_LS, COND_GE, COND_LT,
    COND_GT, COND_LE, COND_AL, COND_NV,
    decode, cond_string, is_test_op,
};

// =========================================================================
// Bit Conversion Helpers
// =========================================================================
//
// Converts between integer values and bit slices (LSB-first). The ARM1
// uses 32-bit data paths, so most conversions use width=32.
//
// LSB-first ordering matches how ripple-carry adders process data:
// bit 0 feeds the first full adder, bit 1 feeds the second, etc.
//
//   int_to_bits(5, 32) -> [1, 0, 1, 0, 0, ..., 0]  (32 elements)
//   bits_to_int(...)    -> 5

/// Converts a u32 to a Vec of 32 bits (LSB first), using u8 per bit.
///
/// This is the bridge between the integer world (test programs, external API)
/// and the gate-level world (slices of 0s and 1s flowing through gates).
pub fn int_to_bits(value: u32, width: usize) -> Vec<u8> {
    let mut bits = vec![0u8; width];
    for i in 0..width {
        bits[i] = ((value >> i) & 1) as u8;
    }
    bits
}

/// Converts a slice of bits (LSB first) to a u32.
pub fn bits_to_int(bits: &[u8]) -> u32 {
    let mut result: u32 = 0;
    for (i, &bit) in bits.iter().enumerate() {
        if i >= 32 { break; }
        result |= (bit as u32) << i;
    }
    result
}

// =========================================================================
// Gate-Level ALU
// =========================================================================
//
// This ALU wraps the arithmetic crate's ripple-carry adder and uses
// logic gate functions from the logic-gates crate. Every ADD instruction
// traverses a chain of 32 full adders, each built from XOR, AND, and OR
// gates. Total: ~160 gate calls per addition.

/// Holds the output of a gate-level ALU operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateALUResult {
    pub result: Vec<u8>,
    pub n: u8,
    pub z: u8,
    pub c: u8,
    pub v: u8,
}

/// Performs one of the 16 ALU operations using gate-level logic.
///
/// Every operation routes through actual gate function calls:
/// - Arithmetic: ripple_carry_adder (32 full adders -> 160+ gate calls)
/// - Logical: AND/OR/XOR/NOT applied to each of 32 bits (32-64 gate calls)
pub fn gate_alu_execute(opcode: u32, a: &[u8], b: &[u8], carry_in: u8, shifter_carry: u8, old_v: u8) -> GateALUResult {
    let (result, carry, overflow) = match opcode {
        // -- Logical operations --
        0x0 | 0x8 => { // AND, TST
            (bitwise_gate(a, b, and_gate), shifter_carry, old_v)
        }
        0x1 | 0x9 => { // EOR, TEQ
            (bitwise_gate(a, b, xor_gate), shifter_carry, old_v)
        }
        0xC => { // ORR
            (bitwise_gate(a, b, or_gate), shifter_carry, old_v)
        }
        0xD => { // MOV
            (b.to_vec(), shifter_carry, old_v)
        }
        0xE => { // BIC = AND(a, NOT(b))
            let not_b = bitwise_not(b);
            (bitwise_gate(a, &not_b, and_gate), shifter_carry, old_v)
        }
        0xF => { // MVN = NOT(b)
            (bitwise_not(b), shifter_carry, old_v)
        }

        // -- Arithmetic operations --
        // All route through the ripple-carry adder (32 full adders chained).
        0x4 | 0xB => { // ADD, CMN: A + B
            let r = ripple_carry_adder_with_carry(a, b, 0);
            let ov = compute_overflow(a, b, &r.sum);
            (r.sum, r.carry_out, ov)
        }
        0x5 => { // ADC: A + B + C
            let r = ripple_carry_adder_with_carry(a, b, carry_in);
            let ov = compute_overflow(a, b, &r.sum);
            (r.sum, r.carry_out, ov)
        }
        0x2 | 0xA => { // SUB, CMP: A - B = A + NOT(B) + 1
            let not_b = bitwise_not(b);
            let r = ripple_carry_adder_with_carry(a, &not_b, 1);
            let ov = compute_overflow(a, &not_b, &r.sum);
            (r.sum, r.carry_out, ov)
        }
        0x6 => { // SBC: A - B - !C = A + NOT(B) + C
            let not_b = bitwise_not(b);
            let r = ripple_carry_adder_with_carry(a, &not_b, carry_in);
            let ov = compute_overflow(a, &not_b, &r.sum);
            (r.sum, r.carry_out, ov)
        }
        0x3 => { // RSB: B - A = B + NOT(A) + 1
            let not_a = bitwise_not(a);
            let r = ripple_carry_adder_with_carry(b, &not_a, 1);
            let ov = compute_overflow(b, &not_a, &r.sum);
            (r.sum, r.carry_out, ov)
        }
        0x7 => { // RSC: B - A - !C = B + NOT(A) + C
            let not_a = bitwise_not(a);
            let r = ripple_carry_adder_with_carry(b, &not_a, carry_in);
            let ov = compute_overflow(b, &not_a, &r.sum);
            (r.sum, r.carry_out, ov)
        }
        _ => (vec![0u8; 32], 0, 0),
    };

    // N flag = MSB of result
    let n = result[31];

    // Z flag: NOR of all 32 result bits
    let z = compute_zero(&result);

    GateALUResult { result, n, z, c: carry, v: overflow }
}

/// Applies a 2-input gate function to each bit pair.
/// This is how the real ARM1 does AND, OR, XOR -- 32 gate instances in parallel.
fn bitwise_gate(a: &[u8], b: &[u8], gate: fn(u8, u8) -> u8) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&ai, &bi)| gate(ai, bi)).collect()
}

/// Applies NOT to each bit.
fn bitwise_not(bits: &[u8]) -> Vec<u8> {
    bits.iter().map(|&b| not_gate(b)).collect()
}

/// Checks if all 32 bits are zero using OR gates followed by NOT.
/// In hardware, this is a tree of OR gates reducing 32 bits to 1.
fn compute_zero(bits: &[u8]) -> u8 {
    let mut combined = bits[0];
    for &b in &bits[1..] {
        combined = or_gate(combined, b);
    }
    not_gate(combined)
}

/// Detects signed overflow using XOR gates.
/// V = (a[31] XOR result[31]) AND (b[31] XOR result[31])
fn compute_overflow(a: &[u8], b: &[u8], result: &[u8]) -> u8 {
    let xor1 = xor_gate(a[31], result[31]);
    let xor2 = xor_gate(b[31], result[31]);
    and_gate(xor1, xor2)
}

// =========================================================================
// Gate-Level Barrel Shifter
// =========================================================================
//
// On the real ARM1, the barrel shifter was a 32x32 crossbar network of pass
// transistors. We model it with a 5-level tree of Mux2 gates from the
// logic-gates crate.
//
// Each level handles one bit of the shift amount:
//   Level 0: shift by 0 or 1   (controlled by amount bit 0)
//   Level 1: shift by 0 or 2   (controlled by amount bit 1)
//   Level 2: shift by 0 or 4   (controlled by amount bit 2)
//   Level 3: shift by 0 or 8   (controlled by amount bit 3)
//   Level 4: shift by 0 or 16  (controlled by amount bit 4)
//
// Total: 5 x 32 = 160 Mux2 gates, each ~4 primitive gates = ~640 gate calls.

/// Performs a shift operation on a 32-bit value using a tree of multiplexer gates.
///
/// Returns (shifted_value, carry_out) where both are bit arrays.
pub fn gate_barrel_shift(value: &[u8], shift_type: u32, amount: u32, carry_in: u8, by_register: bool) -> (Vec<u8>, u8) {
    if by_register && amount == 0 {
        return (value.to_vec(), carry_in);
    }

    match shift_type {
        0 => gate_lsl(value, amount, carry_in, by_register),
        1 => gate_lsr(value, amount, carry_in, by_register),
        2 => gate_asr(value, amount, carry_in, by_register),
        3 => gate_ror(value, amount, carry_in, by_register),
        _ => (value.to_vec(), carry_in),
    }
}

/// Logical Shift Left using a 5-level multiplexer tree.
fn gate_lsl(value: &[u8], amount: u32, carry_in: u8, _by_register: bool) -> (Vec<u8>, u8) {
    if amount == 0 {
        return (value.to_vec(), carry_in);
    }
    if amount >= 32 {
        let result = vec![0u8; 32];
        if amount == 32 {
            return (result, value[0]);
        }
        return (result, 0);
    }

    let mut current = value.to_vec();
    for level in 0..5u32 {
        let shift = 1usize << level;
        let sel = ((amount >> level) & 1) as u8;
        let mut next = vec![0u8; 32];
        for i in 0..32 {
            let shifted = if i >= shift { current[i - shift] } else { 0 };
            next[i] = mux2(current[i], shifted, sel);
        }
        current = next;
    }

    // Carry = last bit shifted out = bit (32 - amount) of original
    let carry = if amount > 0 && amount <= 32 {
        value[(32 - amount) as usize]
    } else {
        carry_in
    };
    (current, carry)
}

/// Logical Shift Right using mux tree.
fn gate_lsr(value: &[u8], amount: u32, carry_in: u8, by_register: bool) -> (Vec<u8>, u8) {
    if amount == 0 && !by_register {
        // Immediate LSR #0 encodes LSR #32
        return (vec![0u8; 32], value[31]);
    }
    if amount == 0 {
        return (value.to_vec(), carry_in);
    }
    if amount >= 32 {
        let result = vec![0u8; 32];
        if amount == 32 {
            return (result, value[31]);
        }
        return (result, 0);
    }

    let mut current = value.to_vec();
    for level in 0..5u32 {
        let shift = 1usize << level;
        let sel = ((amount >> level) & 1) as u8;
        let mut next = vec![0u8; 32];
        for i in 0..32 {
            let shifted = if i + shift < 32 { current[i + shift] } else { 0 };
            next[i] = mux2(current[i], shifted, sel);
        }
        current = next;
    }

    let carry = value[(amount - 1) as usize];
    (current, carry)
}

/// Arithmetic Shift Right (sign-extending) using mux tree.
fn gate_asr(value: &[u8], amount: u32, carry_in: u8, by_register: bool) -> (Vec<u8>, u8) {
    let sign_bit = value[31];

    if amount == 0 && !by_register {
        // Immediate ASR #0 encodes ASR #32
        return (vec![sign_bit; 32], sign_bit);
    }
    if amount == 0 {
        return (value.to_vec(), carry_in);
    }
    if amount >= 32 {
        return (vec![sign_bit; 32], sign_bit);
    }

    let mut current = value.to_vec();
    for level in 0..5u32 {
        let shift = 1usize << level;
        let sel = ((amount >> level) & 1) as u8;
        let mut next = vec![0u8; 32];
        for i in 0..32 {
            let shifted = if i + shift < 32 { current[i + shift] } else { sign_bit };
            next[i] = mux2(current[i], shifted, sel);
        }
        current = next;
    }

    let carry = value[(amount - 1) as usize];
    (current, carry)
}

/// Rotate Right using mux tree.
fn gate_ror(value: &[u8], amount: u32, carry_in: u8, by_register: bool) -> (Vec<u8>, u8) {
    if amount == 0 && !by_register {
        // RRX: 33-bit rotate through carry
        let mut result = vec![0u8; 32];
        for i in 0..31 {
            result[i] = value[i + 1];
        }
        result[31] = carry_in; // Old carry becomes MSB
        let carry = value[0];  // Old LSB becomes new carry
        return (result, carry);
    }
    if amount == 0 {
        return (value.to_vec(), carry_in);
    }

    // Normalize to 0-31
    let amount = amount & 31;
    if amount == 0 {
        return (value.to_vec(), value[31]);
    }

    let mut current = value.to_vec();
    for level in 0..5u32 {
        let shift = 1usize << level;
        let sel = ((amount >> level) & 1) as u8;
        let mut next = vec![0u8; 32];
        for i in 0..32 {
            let shifted = current[(i + shift) % 32];
            next[i] = mux2(current[i], shifted, sel);
        }
        current = next;
    }

    // Carry = MSB of result
    let carry = current[31];
    (current, carry)
}

/// Decodes a rotated immediate using gate-level rotation.
pub fn gate_decode_immediate(imm8: u32, rotate: u32) -> (Vec<u8>, u8) {
    let bits = int_to_bits(imm8, 32);
    let rotate_amount = rotate * 2;
    if rotate_amount == 0 {
        return (bits, 0);
    }
    gate_ror(&bits, rotate_amount, 0, false)
}

// =========================================================================
// ARM1 Gate-Level CPU
// =========================================================================
//
// This is the top-level gate-level ARM1 CPU. It uses the same public API as
// the behavioral simulator but routes every operation through logic gates.
//
// The execution flow for a single ADD instruction:
//   1. FETCH:   Read 32-bit instruction from memory
//   2. DECODE:  Extract bit fields (combinational logic)
//   3. CONDITION: Evaluate 4-bit condition code (gate tree)
//   4. BARREL SHIFT: Process Operand2 (5-level mux tree, ~640 gates)
//   5. ALU:     32-bit ripple-carry add (32 full adders, ~160 gates)
//   6. FLAGS:   Compute N/Z/C/V from result bits (NOR tree, XOR gates)
//   7. WRITE:   Store result in register file (32 flip-flops)
//
// Total per instruction: ~1,000-1,500 gate function calls.

/// The gate-level ARM1 simulator.
///
/// Has the same external behavior as the behavioral `ARM1`,
/// but routes all DATA PATH operations (ALU, barrel shifter, register
/// read/write, flag computation) through gates.
pub struct ARM1GateLevel {
    /// Register file: stored as bit arrays (27 x 32 flip-flop states).
    regs: [[u8; 32]; 27],

    /// Memory (not gate-level -- would need millions of flip-flops).
    memory: Vec<u8>,

    halted: bool,

    /// Gate count tracking.
    gate_ops: usize,
}

impl ARM1GateLevel {
    /// Creates a new gate-level ARM1 simulator.
    pub fn new(memory_size: usize) -> Self {
        let memory_size = if memory_size == 0 { 1024 * 1024 } else { memory_size };
        let mut cpu = Self {
            regs: [[0u8; 32]; 27],
            memory: vec![0u8; memory_size],
            halted: false,
            gate_ops: 0,
        };
        cpu.reset();
        cpu
    }

    /// Restores the CPU to power-on state.
    pub fn reset(&mut self) {
        self.regs = [[0u8; 32]; 27];
        // Set R15: SVC mode, IRQ/FIQ disabled
        let r15val = FLAG_I | FLAG_F | MODE_SVC;
        let bits = int_to_bits(r15val, 32);
        self.regs[15].copy_from_slice(&bits);
        self.halted = false;
        self.gate_ops = 0;
    }

    // =====================================================================
    // Register access (gate-level)
    // =====================================================================

    fn read_reg(&self, index: usize) -> u32 {
        let phys = self.physical_reg(index);
        bits_to_int(&self.regs[phys])
    }

    fn write_reg(&mut self, index: usize, value: u32) {
        let phys = self.physical_reg(index);
        let bits = int_to_bits(value, 32);
        self.regs[phys].copy_from_slice(&bits);
    }

    fn physical_reg(&self, index: usize) -> usize {
        let mode = bits_to_int(&self.regs[15]) & MODE_MASK;
        match (mode, index) {
            (MODE_FIQ, 8..=14) => 16 + (index - 8),
            (MODE_IRQ, 13..=14) => 23 + (index - 13),
            (MODE_SVC, 13..=14) => 25 + (index - 13),
            _ => index,
        }
    }

    fn read_reg_bits(&self, index: usize) -> Vec<u8> {
        let phys = self.physical_reg(index);
        self.regs[phys].to_vec()
    }

    /// Returns the current program counter.
    pub fn pc(&self) -> u32 {
        bits_to_int(&self.regs[15]) & PC_MASK
    }

    /// Sets the PC portion of R15.
    pub fn set_pc(&mut self, addr: u32) {
        let r15 = bits_to_int(&self.regs[15]);
        let new_r15 = (r15 & !PC_MASK) | (addr & PC_MASK);
        let bits = int_to_bits(new_r15, 32);
        self.regs[15].copy_from_slice(&bits);
    }

    /// Returns the current condition flags.
    pub fn flags(&self) -> Flags {
        Flags {
            n: self.regs[15][31] == 1,
            z: self.regs[15][30] == 1,
            c: self.regs[15][29] == 1,
            v: self.regs[15][28] == 1,
        }
    }

    fn set_flags_bits(&mut self, n: u8, z: u8, c: u8, v: u8) {
        self.regs[15][31] = n;
        self.regs[15][30] = z;
        self.regs[15][29] = c;
        self.regs[15][28] = v;
    }

    /// Returns the current processor mode.
    pub fn mode(&self) -> u32 {
        bits_to_int(&self.regs[15]) & MODE_MASK
    }

    /// Returns true if the CPU has been halted.
    pub fn halted(&self) -> bool {
        self.halted
    }

    /// Returns the total number of gate operations performed.
    pub fn gate_ops(&self) -> usize {
        self.gate_ops
    }

    // =====================================================================
    // Memory (same as behavioral -- not gate-level)
    // =====================================================================

    /// Reads a 32-bit word from memory (little-endian, word-aligned).
    pub fn read_word(&self, addr: u32) -> u32 {
        let addr = (addr & PC_MASK) as usize;
        let a = addr & !3;
        if a + 3 >= self.memory.len() {
            return 0;
        }
        u32::from_le_bytes([
            self.memory[a],
            self.memory[a + 1],
            self.memory[a + 2],
            self.memory[a + 3],
        ])
    }

    /// Writes a 32-bit word to memory (little-endian, word-aligned).
    pub fn write_word(&mut self, addr: u32, value: u32) {
        let addr = (addr & PC_MASK) as usize;
        let a = addr & !3;
        if a + 3 >= self.memory.len() {
            return;
        }
        let bytes = value.to_le_bytes();
        self.memory[a..a + 4].copy_from_slice(&bytes);
    }

    /// Reads a single byte from memory.
    pub fn read_byte(&self, addr: u32) -> u8 {
        let a = (addr & PC_MASK) as usize;
        if a >= self.memory.len() { return 0; }
        self.memory[a]
    }

    /// Writes a single byte to memory.
    pub fn write_byte(&mut self, addr: u32, value: u8) {
        let a = (addr & PC_MASK) as usize;
        if a >= self.memory.len() { return; }
        self.memory[a] = value;
    }

    /// Loads raw bytes into memory.
    pub fn load_program(&mut self, code: &[u8], start_addr: u32) {
        for (i, &b) in code.iter().enumerate() {
            let addr = start_addr as usize + i;
            if addr < self.memory.len() {
                self.memory[addr] = b;
            }
        }
    }

    /// Loads a program from u32 instruction words.
    pub fn load_program_words(&mut self, instructions: &[u32], start_addr: u32) {
        let mut code = Vec::with_capacity(instructions.len() * 4);
        for &inst in instructions {
            code.extend_from_slice(&inst.to_le_bytes());
        }
        self.load_program(&code, start_addr);
    }

    // =====================================================================
    // Condition evaluation (gate-level)
    // =====================================================================

    fn evaluate_condition(&mut self, cond: u32, flags: Flags) -> bool {
        let n: u8 = if flags.n { 1 } else { 0 };
        let z: u8 = if flags.z { 1 } else { 0 };
        let c: u8 = if flags.c { 1 } else { 0 };
        let v: u8 = if flags.v { 1 } else { 0 };

        self.gate_ops += 4;

        match cond {
            COND_EQ => z == 1,
            COND_NE => not_gate(z) == 1,
            COND_CS => c == 1,
            COND_CC => not_gate(c) == 1,
            COND_MI => n == 1,
            COND_PL => not_gate(n) == 1,
            COND_VS => v == 1,
            COND_VC => not_gate(v) == 1,
            COND_HI => and_gate(c, not_gate(z)) == 1,
            COND_LS => or_gate(not_gate(c), z) == 1,
            COND_GE => xnor_gate(n, v) == 1,
            COND_LT => xor_gate(n, v) == 1,
            COND_GT => and_gate(not_gate(z), xnor_gate(n, v)) == 1,
            COND_LE => or_gate(z, xor_gate(n, v)) == 1,
            COND_AL => true,
            COND_NV => false,
            _ => false,
        }
    }

    // =====================================================================
    // Execution
    // =====================================================================

    /// Executes one instruction and returns a trace.
    pub fn step(&mut self) -> Trace {
        let pc = self.pc();
        let mut regs_before = [0u32; 16];
        for i in 0..16 {
            regs_before[i] = self.read_reg(i);
        }
        let flags_before = self.flags();

        let instruction = self.read_word(pc);
        let decoded = decode(instruction);
        let cond_met = self.evaluate_condition(decoded.cond, flags_before);

        let mut trace = Trace {
            address: pc,
            raw: instruction,
            mnemonic: decoded.disassemble(),
            condition: cond_string(decoded.cond).to_string(),
            condition_met: cond_met,
            regs_before,
            regs_after: [0u32; 16],
            flags_before,
            flags_after: Flags::default(),
            memory_reads: Vec::new(),
            memory_writes: Vec::new(),
        };

        self.set_pc(pc + 4);

        if cond_met {
            match decoded.inst_type {
                InstType::DataProcessing => self.execute_data_processing(&decoded, &mut trace),
                InstType::LoadStore => self.execute_load_store(&decoded, &mut trace),
                InstType::BlockTransfer => self.execute_block_transfer(&decoded, &mut trace),
                InstType::Branch => self.execute_branch(&decoded, &mut trace),
                InstType::SWI => self.execute_swi(&decoded),
                InstType::Coprocessor | InstType::Undefined => self.trap_undefined(),
            }
        }

        for i in 0..16 {
            trace.regs_after[i] = self.read_reg(i);
        }
        trace.flags_after = self.flags();
        trace
    }

    /// Executes instructions until halted or max_steps reached.
    pub fn run(&mut self, max_steps: usize) -> Vec<Trace> {
        let mut traces = Vec::with_capacity(max_steps.min(1024));
        for _ in 0..max_steps {
            if self.halted { break; }
            traces.push(self.step());
        }
        traces
    }

    // =====================================================================
    // Data Processing (gate-level)
    // =====================================================================

    fn execute_data_processing(&mut self, d: &DecodedInstruction, _trace: &mut Trace) {
        // Read Rn as bits
        let a_bits = if d.opcode != OP_MOV && d.opcode != OP_MVN {
            self.read_reg_bits_for_exec(d.rn)
        } else {
            vec![0u8; 32]
        };

        // Get Operand2 through gate-level barrel shifter
        let flags = self.flags();
        let flag_c: u8 = if flags.c { 1 } else { 0 };
        let flag_v: u8 = if flags.v { 1 } else { 0 };

        let (b_bits, shifter_carry) = if d.immediate {
            let (bits, sc) = gate_decode_immediate(d.imm8, d.rotate);
            if d.rotate == 0 { (bits, flag_c) } else { (bits, sc) }
        } else {
            let rm_bits = self.read_reg_bits_for_exec(d.rm);
            let shift_amount = if d.shift_by_reg {
                self.read_reg(d.rs) & 0xFF
            } else {
                d.shift_imm
            };
            gate_barrel_shift(&rm_bits, d.shift_type, shift_amount, flag_c, d.shift_by_reg)
        };

        // Execute ALU operation through gate-level ALU
        let result = gate_alu_execute(d.opcode, &a_bits, &b_bits, flag_c, shifter_carry, flag_v);
        self.gate_ops += 200;

        let result_val = bits_to_int(&result.result);

        // Write result
        if !is_test_op(d.opcode) {
            if d.rd == 15 {
                if d.s {
                    let r15bits = int_to_bits(result_val, 32);
                    self.regs[15].copy_from_slice(&r15bits);
                } else {
                    self.set_pc(result_val & PC_MASK);
                }
            } else {
                self.write_reg(d.rd, result_val);
            }
        }

        // Update flags
        if d.s && d.rd != 15 {
            self.set_flags_bits(result.n, result.z, result.c, result.v);
        }
        if is_test_op(d.opcode) {
            self.set_flags_bits(result.n, result.z, result.c, result.v);
        }
    }

    fn read_reg_bits_for_exec(&self, index: usize) -> Vec<u8> {
        if index == 15 {
            let val = bits_to_int(&self.regs[15]).wrapping_add(4);
            int_to_bits(val, 32)
        } else {
            self.read_reg_bits(index)
        }
    }

    fn read_reg_for_exec(&self, index: usize) -> u32 {
        if index == 15 {
            bits_to_int(&self.regs[15]).wrapping_add(4)
        } else {
            self.read_reg(index)
        }
    }

    // =====================================================================
    // Load/Store, Block Transfer, Branch, SWI
    // =====================================================================

    fn execute_load_store(&mut self, d: &DecodedInstruction, trace: &mut Trace) {
        let offset = if d.immediate {
            let mut rm_val = self.read_reg_for_exec(d.rm);
            if d.shift_imm != 0 {
                let rm_bits = int_to_bits(rm_val, 32);
                let flag_c: u8 = if self.flags().c { 1 } else { 0 };
                let (shifted, _) = gate_barrel_shift(&rm_bits, d.shift_type, d.shift_imm, flag_c, false);
                rm_val = bits_to_int(&shifted);
            }
            rm_val
        } else {
            d.offset12
        };

        let base = self.read_reg_for_exec(d.rn);
        let addr = if d.up { base.wrapping_add(offset) } else { base.wrapping_sub(offset) };
        let transfer_addr = if d.pre_index { addr } else { base };

        if d.load {
            let value = if d.byte {
                self.read_byte(transfer_addr) as u32
            } else {
                let mut v = self.read_word(transfer_addr);
                let rotation = (transfer_addr & 3) * 8;
                if rotation != 0 {
                    v = (v >> rotation) | (v << (32 - rotation));
                }
                v
            };
            trace.memory_reads.push(MemoryAccess { address: transfer_addr, value });
            if d.rd == 15 {
                let bits = int_to_bits(value, 32);
                self.regs[15].copy_from_slice(&bits);
            } else {
                self.write_reg(d.rd, value);
            }
        } else {
            let value = self.read_reg_for_exec(d.rd);
            if d.byte {
                self.write_byte(transfer_addr, (value & 0xFF) as u8);
            } else {
                self.write_word(transfer_addr, value);
            }
            trace.memory_writes.push(MemoryAccess { address: transfer_addr, value });
        }

        if d.write_back || !d.pre_index {
            if d.rn != 15 {
                self.write_reg(d.rn, addr);
            }
        }
    }

    fn execute_block_transfer(&mut self, d: &DecodedInstruction, trace: &mut Trace) {
        let base = self.read_reg(d.rn);
        let count: u32 = (0..16).filter(|i| (d.register_list >> i) & 1 == 1).count() as u32;
        if count == 0 { return; }

        let start_addr = match (d.pre_index, d.up) {
            (false, true) => base,
            (true, true) => base + 4,
            (false, false) => base - (count * 4) + 4,
            (true, false) => base - (count * 4),
        };

        let mut addr = start_addr;
        for i in 0..16usize {
            if (d.register_list >> i) & 1 == 0 { continue; }

            if d.load {
                let value = self.read_word(addr);
                trace.memory_reads.push(MemoryAccess { address: addr, value });
                if i == 15 {
                    let bits = int_to_bits(value, 32);
                    self.regs[15].copy_from_slice(&bits);
                } else {
                    self.write_reg(i, value);
                }
            } else {
                let value = if i == 15 {
                    bits_to_int(&self.regs[15]).wrapping_add(4)
                } else {
                    self.read_reg(i)
                };
                self.write_word(addr, value);
                trace.memory_writes.push(MemoryAccess { address: addr, value });
            }
            addr += 4;
        }

        if d.write_back {
            let new_base = if d.up { base + (count * 4) } else { base - (count * 4) };
            self.write_reg(d.rn, new_base);
        }
    }

    fn execute_branch(&mut self, d: &DecodedInstruction, _trace: &mut Trace) {
        let branch_base = self.pc().wrapping_add(4);
        if d.link {
            let return_addr = bits_to_int(&self.regs[15]);
            self.write_reg(14, return_addr);
        }
        let target = (branch_base as i32).wrapping_add(d.branch_offset) as u32;
        self.set_pc(target & PC_MASK);
    }

    fn execute_swi(&mut self, d: &DecodedInstruction) {
        if d.swi_comment == HALT_SWI {
            self.halted = true;
            return;
        }
        let r15val = bits_to_int(&self.regs[15]);
        self.regs[25] = self.regs[15];
        self.regs[26] = self.regs[15];

        let new_r15 = (r15val & !MODE_MASK) | MODE_SVC | FLAG_I;
        let bits = int_to_bits(new_r15, 32);
        self.regs[15].copy_from_slice(&bits);
        self.set_pc(0x08);
    }

    fn trap_undefined(&mut self) {
        self.regs[26] = self.regs[15];
        let r15val = bits_to_int(&self.regs[15]);
        let new_r15 = (r15val & !MODE_MASK) | MODE_SVC | FLAG_I;
        let bits = int_to_bits(new_r15, 32);
        self.regs[15].copy_from_slice(&bits);
        self.set_pc(0x04);
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use arm1_simulator::{
        ARM1, encode_mov_imm, encode_alu_reg, encode_halt, encode_branch,
        encode_data_processing, encode_ldr, encode_str, encode_ldm, encode_stm,
        OP_ADD, OP_SUB, OP_AND, OP_EOR, OP_ORR, OP_CMP, SHIFT_LSL,
    };

    // =====================================================================
    // Bit conversion
    // =====================================================================

    #[test]
    fn test_int_to_bits() {
        let bits = int_to_bits(5, 32);
        assert_eq!(bits[0], 1);
        assert_eq!(bits[2], 1);
        assert_eq!(bits_to_int(&bits), 5);
    }

    #[test]
    fn test_bits_round_trip() {
        for v in [0u32, 1, 42, 0xFF, 0xDEAD_BEEF, 0xFFFF_FFFF] {
            let bits = int_to_bits(v, 32);
            assert_eq!(bits_to_int(&bits), v, "round-trip failed for {v}");
        }
    }

    // =====================================================================
    // Gate-level ALU
    // =====================================================================

    #[test]
    fn test_gate_alu_add() {
        let a = int_to_bits(1, 32);
        let b = int_to_bits(2, 32);
        let r = gate_alu_execute(OP_ADD, &a, &b, 0, 0, 0);
        assert_eq!(bits_to_int(&r.result), 3);
        assert_eq!(r.n, 0);
        assert_eq!(r.z, 0);
        assert_eq!(r.c, 0);
        assert_eq!(r.v, 0);
    }

    #[test]
    fn test_gate_alu_sub_zero() {
        let a = int_to_bits(5, 32);
        let b = int_to_bits(5, 32);
        let r = gate_alu_execute(OP_SUB, &a, &b, 0, 0, 0);
        assert_eq!(bits_to_int(&r.result), 0);
        assert_eq!(r.z, 1, "Z should be set for 5-5=0");
        assert_eq!(r.c, 1, "C should be set (no borrow)");
    }

    #[test]
    fn test_gate_alu_logical() {
        let a = int_to_bits(0xFF00_FF00, 32);
        let b = int_to_bits(0x0FF0_0FF0, 32);

        let r = gate_alu_execute(OP_AND, &a, &b, 0, 0, 0);
        assert_eq!(bits_to_int(&r.result), 0x0F00_0F00);

        let r = gate_alu_execute(OP_EOR, &a, &b, 0, 0, 0);
        assert_eq!(bits_to_int(&r.result), 0xF0F0_F0F0);

        let r = gate_alu_execute(OP_ORR, &a, &b, 0, 0, 0);
        assert_eq!(bits_to_int(&r.result), 0xFFF0_FFF0);
    }

    // =====================================================================
    // Gate-level barrel shifter
    // =====================================================================

    #[test]
    fn test_gate_barrel_shift_lsl() {
        let value = int_to_bits(0xFF, 32);
        let (result, _) = gate_barrel_shift(&value, 0, 4, 0, false);
        assert_eq!(bits_to_int(&result), 0xFF0);
    }

    #[test]
    fn test_gate_barrel_shift_lsr() {
        let value = int_to_bits(0xFF00, 32);
        let (result, _) = gate_barrel_shift(&value, 1, 8, 0, false);
        assert_eq!(bits_to_int(&result), 0xFF);
    }

    #[test]
    fn test_gate_barrel_shift_ror() {
        let value = int_to_bits(0x0000_000F, 32);
        let (result, _) = gate_barrel_shift(&value, 3, 4, 0, false);
        assert_eq!(bits_to_int(&result), 0xF000_0000);
    }

    #[test]
    fn test_gate_barrel_shift_rrx() {
        let value = int_to_bits(0x0000_0001, 32);
        let (result, carry) = gate_barrel_shift(&value, 3, 0, 1, false);
        assert_eq!(bits_to_int(&result), 0x8000_0000);
        assert_eq!(carry, 1, "RRX carry should be 1 (old bit 0 was 1)");
    }

    // =====================================================================
    // Gate-level CPU basic tests
    // =====================================================================

    #[test]
    fn test_gate_level_new_and_reset() {
        let cpu = ARM1GateLevel::new(1024);
        assert_eq!(cpu.mode(), MODE_SVC);
        assert_eq!(cpu.pc(), 0);
    }

    #[test]
    fn test_gate_level_halt() {
        let mut cpu = ARM1GateLevel::new(1024);
        cpu.load_program_words(&[encode_halt()], 0);
        let traces = cpu.run(10);
        assert!(cpu.halted());
        assert_eq!(traces.len(), 1);
    }

    #[test]
    fn test_gate_level_gate_ops_tracking() {
        let mut cpu = ARM1GateLevel::new(1024);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 42),
            encode_halt(),
        ], 0);
        cpu.run(10);
        assert!(cpu.gate_ops() > 0, "gate ops should be non-zero after execution");
    }

    // =====================================================================
    // Cross-validation: Gate-level vs Behavioral
    // =====================================================================
    //
    // This is the ultimate correctness guarantee. We run the same program on
    // both simulators and verify they produce identical results.

    fn cross_validate(name: &str, instructions: &[u32]) {
        let mut behavioral = ARM1::new(4096);
        let mut gate_lev = ARM1GateLevel::new(4096);

        behavioral.load_program_words(instructions, 0);
        gate_lev.load_program_words(instructions, 0);

        let b_traces = behavioral.run(200);
        let g_traces = gate_lev.run(200);

        assert_eq!(
            b_traces.len(), g_traces.len(),
            "{name}: trace count mismatch: behavioral={} gate-level={}",
            b_traces.len(), g_traces.len()
        );

        for i in 0..b_traces.len() {
            let bt = &b_traces[i];
            let gt = &g_traces[i];

            assert_eq!(bt.address, gt.address, "{name} step {i}: address mismatch");
            assert_eq!(bt.condition_met, gt.condition_met, "{name} step {i}: condition mismatch");

            for r in 0..16 {
                assert_eq!(
                    bt.regs_after[r], gt.regs_after[r],
                    "{name} step {i}: R{r} mismatch: B=0x{:X} G=0x{:X}",
                    bt.regs_after[r], gt.regs_after[r]
                );
            }

            assert_eq!(
                bt.flags_after, gt.flags_after,
                "{name} step {i}: flags mismatch: B={:?} G={:?}",
                bt.flags_after, gt.flags_after
            );
        }
    }

    #[test]
    fn test_cross_validate_one_plus_two() {
        cross_validate("1+2", &[
            encode_mov_imm(COND_AL, 0, 1),
            encode_mov_imm(COND_AL, 1, 2),
            encode_alu_reg(COND_AL, OP_ADD, 0, 2, 0, 1),
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_subs_with_flags() {
        cross_validate("SUBS", &[
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_conditional() {
        cross_validate("conditional", &[
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1),
            encode_mov_imm(COND_NE, 3, 99),
            encode_mov_imm(COND_EQ, 4, 42),
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_barrel_shifter() {
        // ADD R1, R0, R0, LSL #2 (multiply by 5)
        let add_with_shift = (COND_AL << 28) |
            (OP_ADD << 21) |
            (0 << 16) |
            (1 << 12) |
            (2 << 7) |
            (SHIFT_LSL << 5) |
            0;

        cross_validate("barrel_shifter", &[
            encode_mov_imm(COND_AL, 0, 7),
            add_with_shift,
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_loop() {
        cross_validate("loop_sum_1_to_10", &[
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 10),
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 1),
            encode_data_processing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1),
            encode_branch(COND_NE, false, -16),
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_ldr_str() {
        cross_validate("ldr_str", &[
            encode_mov_imm(COND_AL, 0, 42),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
            encode_str(COND_AL, 0, 1, 0, true),
            encode_mov_imm(COND_AL, 0, 0),
            encode_ldr(COND_AL, 0, 1, 0, true),
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_stm_ldm() {
        cross_validate("stm_ldm", &[
            encode_mov_imm(COND_AL, 0, 10),
            encode_mov_imm(COND_AL, 1, 20),
            encode_mov_imm(COND_AL, 2, 30),
            encode_mov_imm(COND_AL, 3, 40),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
            encode_stm(COND_AL, 5, 0x000F, true, "IA"),
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 0),
            encode_mov_imm(COND_AL, 2, 0),
            encode_mov_imm(COND_AL, 3, 0),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
            encode_ldm(COND_AL, 5, 0x000F, true, "IA"),
            encode_halt(),
        ]);
    }

    #[test]
    fn test_cross_validate_branch_and_link() {
        cross_validate("branch_and_link", &[
            encode_mov_imm(COND_AL, 0, 7),
            encode_branch(COND_AL, true, 4),
            encode_halt(),
            0,
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 0),
            encode_data_processing(COND_AL, OP_MOV, 1, 0, 15, 14),
        ]);
    }

    #[test]
    fn test_cross_validate_cmp() {
        cross_validate("cmp", &[
            encode_mov_imm(COND_AL, 0, 10),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_CMP, 1, 0, 0, 1),
            encode_mov_imm(COND_GT, 2, 1),
            encode_mov_imm(COND_LE, 2, 0),
            encode_halt(),
        ]);
    }
}
