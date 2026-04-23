//! # Intel 8008 Simulator — the world's first 8-bit microprocessor.
//!
//! ## Historical context
//!
//! The Intel 8008 was released in April 1972 as the world's first 8-bit
//! microprocessor. It was designed by Ted Hoff, Stanley Mazor, and Hal Feeney
//! at Intel, originally at the request of Computer Terminal Corporation (CTC)
//! for their Datapoint 2200 terminal. CTC ultimately rejected the chip for
//! being too slow, but Intel retained rights and released it commercially.
//!
//! The 8008 went on to inspire the Intel 8080 (1974), which inspired the Z80
//! and the entire x86 architecture. Every modern PC, server, and smartphone
//! running x86 or x86-64 code traces its lineage to this 3,500-transistor chip.
//!
//! ## Architecture at a glance
//!
//! | Feature          | Value                                                |
//! |------------------|------------------------------------------------------|
//! | Data width       | 8 bits                                               |
//! | Registers        | A (accumulator), B, C, D, E, H, L + M (memory ptr)  |
//! | Flags            | Carry (CY), Zero (Z), Sign (S), Parity (P)           |
//! | Program counter  | 14 bits (addresses 16 KiB)                           |
//! | Stack            | 8-level internal push-down stack                     |
//! | Memory           | 16,384 bytes (14-bit address space)                  |
//! | I/O              | 8 input ports, 24 output ports                       |
//! | Transistors      | ~3,500 (PMOS, 10 μm)                                 |
//!
//! ## Register encoding
//!
//! Instructions use 3-bit register fields:
//!
//! ```text
//! 000 = B    001 = C    010 = D    011 = E
//! 100 = H    101 = L    110 = M    111 = A
//! ```
//!
//! M (code 110) is a pseudo-register: it aliases the byte in memory at address
//! `(H & 0x3F) << 8 | L`. Only the low 6 bits of H are used for addressing.
//!
//! ## Instruction encoding
//!
//! ```text
//!  7   6   5   4   3   2   1   0
//! ┌───┬───┬───┬───┬───┬───┬───┬───┐
//! │ b7  b6 │  DDD  │  SSS/data     │
//! └───┴───┴───┴───┴───┴───┴───┴───┘
//!
//! bits 7–6 (group):
//!   00 = Register ops (INR, DCR, Rotates) + 2-byte MVI
//!   01 = MOV + HLT + Jumps + Calls + IN
//!   10 = ALU register source (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
//!   11 = ALU immediate + returns (ADI, SUI, etc.) + OUT
//! ```
//!
//! ## The push-down stack
//!
//! The 8008 stack is unique: entry[0] IS the program counter. On CALL, the
//! stack rotates down (entry[0]→entry[1], …) and the target is loaded into
//! entry[0]. On RETURN, the stack rotates up, restoring the saved return address
//! into entry[0]. Programs can nest calls at most 7 levels deep before the
//! oldest saved address is silently overwritten.
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_intel8008_simulator::Simulator;
//!
//! let mut sim = Simulator::new();
//! // MVI B,1  (0x06 0x01)
//! // MVI A,2  (0x3E 0x02)
//! // ADD B    (0x80)
//! // HLT      (0x76)
//! let program = &[0x06u8, 0x01, 0x3E, 0x02, 0x80, 0x76];
//! let traces = sim.run(program, 100);
//! assert_eq!(sim.a(), 3);
//! assert!(!sim.flags().carry);
//! assert!(!sim.flags().zero);
//! ```

// ============================================================================
// Public types
// ============================================================================

/// Condition flags for the Intel 8008.
///
/// These four flags are updated by most ALU operations and tested by
/// conditional jump, call, and return instructions.
///
/// ```text
/// Bit 7  Bit 6  Bit 5  Bit 4  Bit 3  Bit 2  Bit 1  Bit 0
///   S      -      -      -      -      P      Z     CY
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    /// CY — set on addition overflow or subtraction borrow; updated by rotates.
    pub carry: bool,
    /// Z — set when the result is exactly 0x00.
    pub zero: bool,
    /// S — set when bit 7 of the result is 1 (negative in signed 8-bit).
    pub sign: bool,
    /// P — set when the result has an **even** number of 1-bits (even parity).
    pub parity: bool,
}

impl Flags {
    fn zero_state() -> Self {
        Flags { carry: false, zero: false, sign: false, parity: false }
    }
}

/// Record of a single instruction execution.
///
/// Produced by `step()` and collected by `run()`. Useful for debugging,
/// testing, and cross-validating against the gate-level simulator.
#[derive(Debug, Clone)]
pub struct Trace {
    /// Address in memory where this instruction was fetched (14-bit).
    pub address: u16,
    /// Raw instruction bytes (1, 2, or 3 bytes).
    pub raw: Vec<u8>,
    /// Human-readable disassembly, e.g. `"ADD B"`, `"MVI A, 0x05"`.
    pub mnemonic: String,
    /// Accumulator value before this instruction executed.
    pub a_before: u8,
    /// Accumulator value after this instruction executed.
    pub a_after: u8,
    /// Flags before this instruction.
    pub flags_before: Flags,
    /// Flags after this instruction.
    pub flags_after: Flags,
    /// Memory address accessed, if this instruction touched memory via M register.
    pub mem_address: Option<u16>,
    /// Memory value read or written, if applicable.
    pub mem_value: Option<u8>,
}

// ============================================================================
// Register index constants (the 3-bit SSS/DDD encoding)
// ============================================================================

const REG_B: usize = 0;
const REG_C: usize = 1;
const REG_D: usize = 2;
const REG_E: usize = 3;
const REG_H: usize = 4;
const REG_L: usize = 5;
const REG_M: usize = 6; // pseudo-register — memory at [H:L]
const REG_A: usize = 7;

// Register names for disassembly
const REG_NAMES: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];

// ALU operation names for disassembly
const ALU_MNEMONICS: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];
const ALU_IMM_MNEMONICS: [&str; 8] = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];

// ============================================================================
// Intel 8008 Simulator
// ============================================================================

/// Intel 8008 behavioral simulator.
///
/// Executes 8008 machine code directly, producing correct results without
/// modeling internal hardware gates. For gate-level simulation, see
/// `coding-adventures-intel8008-gatelevel`.
///
/// # Register layout
///
/// Registers are stored as `regs[0..=7]`:
///
/// ```text
/// regs[0] = B    regs[1] = C    regs[2] = D    regs[3] = E
/// regs[4] = H    regs[5] = L    regs[6] = (unused — M is a pseudo-reg)
/// regs[7] = A
/// ```
///
/// # Stack layout
///
/// `stack[0]` is ALWAYS the current program counter. `stack[1..=7]` hold
/// saved return addresses in LIFO order. `stack_depth` tracks how many
/// CALL levels are currently active (0 = no active calls).
pub struct Simulator {
    /// General-purpose registers indexed by 3-bit opcode field.
    /// Index 6 (M) is unused here; M accesses go through hl_address().
    regs: [u8; 8],
    /// 16 KiB unified address space (program + data).
    /// Boxed to avoid placing 16 KiB on the stack at construction.
    memory: Box<[u8; 16384]>,
    /// 8-level push-down stack. stack[0] is always the live PC (14-bit).
    stack: [u16; 8],
    /// Number of active CALL frames (0–7). Does not count entry[0] (current PC).
    stack_depth: usize,
    /// CPU condition flags.
    flags: Flags,
    /// True after a HLT instruction executes.
    halted: bool,
    /// 8 input ports (read by IN instruction, set externally).
    input_ports: [u8; 8],
    /// 24 output ports (written by OUT instruction, read externally).
    output_ports: [u8; 24],
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    /// Create a new simulator with all state zeroed.
    pub fn new() -> Self {
        Simulator {
            regs: [0u8; 8],
            memory: Box::new([0u8; 16384]),
            stack: [0u16; 8],
            stack_depth: 0,
            flags: Flags::zero_state(),
            halted: false,
            input_ports: [0u8; 8],
            output_ports: [0u8; 24],
        }
    }

    // -------------------------------------------------------------------------
    // Register accessors
    // -------------------------------------------------------------------------

    /// Accumulator (register A).
    pub fn a(&self) -> u8 { self.regs[REG_A] }
    /// Register B.
    pub fn b(&self) -> u8 { self.regs[REG_B] }
    /// Register C.
    pub fn c(&self) -> u8 { self.regs[REG_C] }
    /// Register D.
    pub fn d(&self) -> u8 { self.regs[REG_D] }
    /// Register E.
    pub fn e(&self) -> u8 { self.regs[REG_E] }
    /// Register H (high byte of address pair).
    pub fn h(&self) -> u8 { self.regs[REG_H] }
    /// Register L (low byte of address pair).
    pub fn l(&self) -> u8 { self.regs[REG_L] }

    /// Current 14-bit program counter (always `stack[0] & 0x3FFF`).
    pub fn pc(&self) -> u16 { self.stack[0] & 0x3FFF }

    /// 14-bit effective address for the M pseudo-register: `(H & 0x3F) << 8 | L`.
    ///
    /// Only the low 6 bits of H contribute to the address — the top 2 bits
    /// of H are "don't care" on the real chip.
    pub fn hl_address(&self) -> u16 {
        ((self.regs[REG_H] as u16 & 0x3F) << 8) | self.regs[REG_L] as u16
    }

    /// Current condition flags.
    pub fn flags(&self) -> Flags { self.flags }

    /// Number of active CALL frames (0 = no calls outstanding).
    pub fn stack_depth(&self) -> usize { self.stack_depth }

    /// True if a HLT instruction has executed.
    pub fn halted(&self) -> bool { self.halted }

    // -------------------------------------------------------------------------
    // I/O ports
    // -------------------------------------------------------------------------

    /// Set the value of an input port (port 0–7).
    /// The value will be read by the IN instruction.
    pub fn set_input_port(&mut self, port: usize, value: u8) {
        assert!(port < 8, "input port number must be 0–7, got {port}");
        self.input_ports[port] = value;
    }

    /// Read the current value of an output port (port 0–23).
    /// Updated whenever an OUT instruction executes.
    pub fn get_output_port(&self, port: usize) -> u8 {
        assert!(port < 24, "output port number must be 0–23, got {port}");
        self.output_ports[port]
    }

    // -------------------------------------------------------------------------
    // Memory access
    // -------------------------------------------------------------------------

    /// Read from memory (14-bit address).
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[(addr & 0x3FFF) as usize]
    }

    /// Write to memory (14-bit address).
    fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[(addr & 0x3FFF) as usize] = value;
    }

    // -------------------------------------------------------------------------
    // Register read/write (handles M pseudo-register transparently)
    // -------------------------------------------------------------------------

    /// Read a register by its 3-bit index. For M (index 6), reads memory[H:L].
    fn reg_read(&self, idx: usize) -> u8 {
        if idx == REG_M {
            self.mem_read(self.hl_address())
        } else {
            self.regs[idx]
        }
    }

    /// Write a register by its 3-bit index. For M (index 6), writes memory[H:L].
    #[allow(dead_code)]
    fn reg_write(&mut self, idx: usize, value: u8) {
        if idx == REG_M {
            let addr = self.hl_address();
            self.mem_write(addr, value);
        } else {
            self.regs[idx] = value;
        }
    }

    // -------------------------------------------------------------------------
    // Stack operations
    // -------------------------------------------------------------------------

    /// CALL — save current PC (already in stack[0] beyond the full instruction),
    /// rotate stack down, and load the jump target into stack[0].
    ///
    /// ```text
    /// Before: stack = [PC, r1, r2, r3, r4, r5, r6, r7]
    /// After:  stack = [target, PC, r1, r2, r3, r4, r5, r6]
    ///                           ^^^  (next instruction after CALL)
    /// ```
    ///
    /// If already at maximum depth (7 saved frames), the oldest frame (stack[7])
    /// is silently overwritten — matching real 8008 behaviour.
    fn push_and_jump(&mut self, target: u16) {
        // Rotate down: entry[7] is lost, each entry shifts one position deeper.
        for i in (1..8).rev() {
            self.stack[i] = self.stack[i - 1];
        }
        // Load target as the new PC.
        self.stack[0] = target & 0x3FFF;
        if self.stack_depth < 7 {
            self.stack_depth += 1;
        }
    }

    /// RETURN — rotate stack up, restoring the saved return address into stack[0].
    ///
    /// ```text
    /// Before: stack = [target, saved_pc, r2, r3, ...]
    /// After:  stack = [saved_pc, r2, r3, ..., 0]
    /// ```
    fn pop_return(&mut self) {
        for i in 0..7 {
            self.stack[i] = self.stack[i + 1];
        }
        self.stack[7] = 0;
        if self.stack_depth > 0 {
            self.stack_depth -= 1;
        }
    }

    // -------------------------------------------------------------------------
    // Flag computation
    // -------------------------------------------------------------------------

    /// Compute flags from an 8-bit result.
    ///
    /// - `carry` — the carry/borrow bit from the operation
    /// - `update_carry` — if false, the carry flag is preserved from `prev`
    /// - `prev` — previous flags (used when `update_carry` is false)
    ///
    /// The Parity flag is true (even parity) when the number of 1-bits in
    /// `result` is even. This uses `count_ones()` for simplicity; the
    /// gate-level simulator implements this via XOR reduction.
    fn compute_flags(result: u8, carry: bool, update_carry: bool, prev: Flags) -> Flags {
        let ones = result.count_ones();
        Flags {
            carry: if update_carry { carry } else { prev.carry },
            zero: result == 0,
            sign: (result & 0x80) != 0,
            parity: ones % 2 == 0, // true = even parity
        }
    }

    // -------------------------------------------------------------------------
    // Condition evaluation (for conditional jumps/calls/returns)
    // -------------------------------------------------------------------------

    /// Evaluate a conditional instruction's guard.
    ///
    /// The 8008 encodes conditions as a 3-bit code (CCC) plus a sense bit (T):
    /// - T=0: jump/call/return if the flag is **false** (not set)
    /// - T=1: jump/call/return if the flag is **true** (set)
    ///
    /// CCC codes:
    /// ```text
    /// 000 = CY    001 = Z    010 = S    011 = P
    /// ```
    fn condition_met(&self, ccc: u8, sense: bool) -> bool {
        let flag_val = match ccc & 0x03 {
            0 => self.flags.carry,
            1 => self.flags.zero,
            2 => self.flags.sign,
            3 => self.flags.parity,
            _ => unreachable!(),
        };
        if sense { flag_val } else { !flag_val }
    }

    // -------------------------------------------------------------------------
    // ALU operations
    // -------------------------------------------------------------------------

    /// Execute one of the 8 ALU operations.
    ///
    /// Returns `(result, carry_out, clear_carry)`:
    /// - `result` — 8-bit result written back to A (or discarded for CMP)
    /// - `carry_out` — new carry flag value (for ADD/ADC/SUB/SBB/CMP/rotates)
    /// - `clear_carry` — true for ANA/XRA/ORA which always clear carry
    fn alu_op(alu_code: u8, a: u8, b: u8, carry_in: bool) -> (u8, bool, bool) {
        match alu_code {
            0 => { // ADD
                let (r, c) = a.overflowing_add(b);
                (r, c, false)
            }
            1 => { // ADC — add with carry
                let ci = if carry_in { 1u8 } else { 0u8 };
                let (r1, c1) = a.overflowing_add(b);
                let (r2, c2) = r1.overflowing_add(ci);
                (r2, c1 || c2, false)
            }
            2 => { // SUB — CY=1 means borrow occurred
                let (r, c) = a.overflowing_sub(b);
                (r, c, false)
            }
            3 => { // SBB — subtract with borrow
                let bi = if carry_in { 1u8 } else { 0u8 };
                let (r1, c1) = a.overflowing_sub(b);
                let (r2, c2) = r1.overflowing_sub(bi);
                (r2, c1 || c2, false)
            }
            4 => { // ANA — AND, always clears carry
                (a & b, false, true)
            }
            5 => { // XRA — XOR, always clears carry
                (a ^ b, false, true)
            }
            6 => { // ORA — OR, always clears carry
                (a | b, false, true)
            }
            7 => { // CMP — compare (set flags for A-B, A unchanged)
                let (r, c) = a.overflowing_sub(b);
                (r, c, false) // result used for flags only
            }
            _ => unreachable!(),
        }
    }

    // -------------------------------------------------------------------------
    // Rotate operations
    // -------------------------------------------------------------------------

    /// RLC — rotate A left circular. Bit 7 wraps to bit 0 and is copied to CY.
    ///
    /// ```text
    /// CY ← A[7];  A ← (A << 1) | A[7]
    /// ```
    fn rlc(a: u8) -> (u8, bool) {
        let bit7 = (a >> 7) & 1;
        let result = (a << 1) | bit7;
        (result, bit7 == 1)
    }

    /// RRC — rotate A right circular. Bit 0 wraps to bit 7 and is copied to CY.
    ///
    /// ```text
    /// CY ← A[0];  A ← (A >> 1) | (A[0] << 7)
    /// ```
    fn rrc(a: u8) -> (u8, bool) {
        let bit0 = a & 1;
        let result = (a >> 1) | (bit0 << 7);
        (result, bit0 == 1)
    }

    /// RAL — rotate A left through carry (9-bit rotation).
    ///
    /// ```text
    /// new_CY ← A[7];  A ← (A << 1) | old_CY
    /// ```
    fn ral(a: u8, carry_in: bool) -> (u8, bool) {
        let new_carry = (a >> 7) & 1 == 1;
        let result = (a << 1) | (if carry_in { 1 } else { 0 });
        (result, new_carry)
    }

    /// RAR — rotate A right through carry (9-bit rotation).
    ///
    /// ```text
    /// new_CY ← A[0];  A ← (old_CY << 7) | (A >> 1)
    /// ```
    fn rar(a: u8, carry_in: bool) -> (u8, bool) {
        let new_carry = a & 1 == 1;
        let result = (if carry_in { 0x80u8 } else { 0u8 }) | (a >> 1);
        (result, new_carry)
    }

    // -------------------------------------------------------------------------
    // Fetch helpers
    // -------------------------------------------------------------------------

    /// Fetch one byte from memory at the current PC, then advance PC.
    fn fetch_byte(&mut self) -> u8 {
        let pc = self.stack[0] & 0x3FFF;
        let byte = self.memory[pc as usize];
        self.stack[0] = (pc + 1) & 0x3FFF;
        byte
    }

    // -------------------------------------------------------------------------
    // Load / run
    // -------------------------------------------------------------------------

    /// Load a program into memory starting at `start`.
    pub fn load_program(&mut self, program: &[u8], start: usize) {
        let end = (start + program.len()).min(16384);
        self.memory[start..end].copy_from_slice(&program[..end - start]);
    }

    /// Reset all CPU state to power-on defaults (memory is NOT cleared).
    pub fn reset(&mut self) {
        self.regs = [0u8; 8];
        self.stack = [0u16; 8];
        self.stack_depth = 0;
        self.flags = Flags::zero_state();
        self.halted = false;
    }

    /// Execute one instruction and return a trace record.
    ///
    /// Returns `Err` if the CPU is already halted or if an unrecognized opcode
    /// is encountered. On success, the trace captures full before/after state.
    pub fn step(&mut self) -> Result<Trace, String> {
        if self.halted {
            return Err("CPU is halted".to_string());
        }

        let fetch_pc = self.stack[0] & 0x3FFF;
        let a_before = self.regs[REG_A];
        let flags_before = self.flags;

        let opcode = self.fetch_byte();
        let mut raw = vec![opcode];
        let mut mem_address: Option<u16> = None;
        let mut mem_value: Option<u8> = None;

        // Decode top 2 bits for group selection
        let group = (opcode >> 6) & 0x03;
        let ddd = (opcode >> 3) & 0x07; // bits 5–3
        let sss = opcode & 0x07;        // bits 2–0

        let mnemonic: String;

        match group {
            // =================================================================
            // GROUP 00: INR, DCR, Rotates/OUT, MVI, Returns, RST
            //
            // Instruction family is fully determined by the low 3 bits (sss):
            //   000 → INR DDD       (1 byte)
            //   001 → DCR DDD       (1 byte)
            //   010 → Rotate (ddd 0-3) or OUT (ddd 4-7)  (1 byte)
            //   011 → Return if flag false  (1 byte)
            //   100 → (undefined in standard 8008)
            //   101 → RST AAA       (1 byte, CALL to AAA×8)
            //   110 → MVI DDD, d    (2 bytes)
            //   111 → Return if flag true   (1 byte)
            // =================================================================
            0 => {
                match sss {
                    // -------------------------------------------------------
                    // INR DDD  (00 DDD 000) — increment register by 1.
                    //
                    // Wraps 0xFF → 0x00. Updates Z, S, P but NOT CY.
                    // INR B (0x00) is the closest thing the 8008 has to NOP.
                    // -------------------------------------------------------
                    0 => {
                        let prev = self.reg_read(ddd as usize);
                        let result = prev.wrapping_add(1);
                        if ddd as usize == REG_M {
                            let addr = self.hl_address();
                            self.mem_write(addr, result);
                            mem_address = Some(addr);
                            mem_value = Some(result);
                        } else {
                            self.regs[ddd as usize] = result;
                        }
                        self.flags = Self::compute_flags(result, false, false, self.flags);
                        mnemonic = format!("INR {}", REG_NAMES[ddd as usize]);
                    }

                    // -------------------------------------------------------
                    // DCR DDD  (00 DDD 001) — decrement register by 1.
                    //
                    // Wraps 0x00 → 0xFF. Updates Z, S, P but NOT CY.
                    // -------------------------------------------------------
                    1 => {
                        let prev = self.reg_read(ddd as usize);
                        let result = prev.wrapping_sub(1);
                        if ddd as usize == REG_M {
                            let addr = self.hl_address();
                            self.mem_write(addr, result);
                            mem_address = Some(addr);
                            mem_value = Some(result);
                        } else {
                            self.regs[ddd as usize] = result;
                        }
                        self.flags = Self::compute_flags(result, false, false, self.flags);
                        mnemonic = format!("DCR {}", REG_NAMES[ddd as usize]);
                    }

                    // -------------------------------------------------------
                    // Rotates (00 RR_ 010, ddd=0-3) and OUT (ddd=4-7)
                    //
                    // The 8008 shares sss=010 between rotate and OUT opcodes.
                    // The four rotate instructions use ddd=0,1,2,3:
                    //   RLC (0x02), RRC (0x0A), RAL (0x12), RAR (0x1A)
                    //
                    // OUT uses ddd=4-7 (and the upper-group bits for ports 0-8):
                    //   Port = (opcode >> 1) & 0x1F
                    //
                    // Rotates only update CY; Z, S, P are not affected.
                    // OUT never affects any flags.
                    // -------------------------------------------------------
                    2 => {
                        match ddd {
                            0 => { // RLC — rotate A left circular (0x02)
                                let (r, c) = Self::rlc(self.regs[REG_A]);
                                self.regs[REG_A] = r;
                                self.flags.carry = c;
                                mnemonic = "RLC".to_string();
                            }
                            1 => { // RRC — rotate A right circular (0x0A)
                                let (r, c) = Self::rrc(self.regs[REG_A]);
                                self.regs[REG_A] = r;
                                self.flags.carry = c;
                                mnemonic = "RRC".to_string();
                            }
                            2 => { // RAL — rotate left through carry (0x12)
                                let (r, c) = Self::ral(self.regs[REG_A], self.flags.carry);
                                self.regs[REG_A] = r;
                                self.flags.carry = c;
                                mnemonic = "RAL".to_string();
                            }
                            3 => { // RAR — rotate right through carry (0x1A)
                                let (r, c) = Self::rar(self.regs[REG_A], self.flags.carry);
                                self.regs[REG_A] = r;
                                self.flags.carry = c;
                                mnemonic = "RAR".to_string();
                            }
                            _ => { // OUT — output A to port (ddd=4,5,6,7)
                                let port = ((opcode >> 1) & 0x1F) as usize;
                                if port < 24 {
                                    self.output_ports[port] = self.regs[REG_A];
                                }
                                mnemonic = format!("OUT {}", port);
                            }
                        }
                    }

                    // -------------------------------------------------------
                    // Return false  (00 CCC 011) — pop stack if flag is clear.
                    //
                    // CCC (condition code) is in bits 5-3 (ddd field):
                    //   000=CY  001=Z  010=S  011=P
                    // RET (0x3F = 00 111 111) is caught by sss=7, not here.
                    // -------------------------------------------------------
                    3 => {
                        let ccc = ddd as u8 & 0x03;
                        let (cond_name, _) = Self::cond_name(ccc, false);
                        mnemonic = format!("R{}", cond_name);
                        if self.condition_met(ccc, false) {
                            self.pop_return();
                        }
                    }

                    // -------------------------------------------------------
                    // RST AAA  (00 AAA 101) — restart: 1-byte CALL to AAA×8.
                    //
                    // The 3-bit AAA field (ddd) encodes a target address as
                    // AAA × 8. This puts 8 fixed interrupt vectors in the
                    // first 64 bytes of memory (0x00, 0x08, 0x10, …, 0x38).
                    //
                    // RST is functionally equivalent to CAL (AAA×8) but
                    // encoded in a single byte, making it efficient for
                    // interrupt dispatch.
                    // -------------------------------------------------------
                    5 => {
                        let target = (ddd as u16) << 3;
                        mnemonic = format!("RST {}", ddd);
                        self.push_and_jump(target);
                    }

                    // -------------------------------------------------------
                    // MVI DDD, d  (00 DDD 110) — move immediate byte into register.
                    //
                    // Loads the 8-bit constant that follows the opcode into
                    // register DDD. If DDD=M (110), writes to memory at [H:L].
                    // Flags: not affected.
                    // -------------------------------------------------------
                    6 => {
                        let data = self.fetch_byte();
                        raw.push(data);
                        if ddd == 6 {
                            let addr = self.hl_address();
                            self.mem_write(addr, data);
                            mem_address = Some(addr);
                            mem_value = Some(data);
                            mnemonic = format!("MVI M, 0x{:02X}", data);
                        } else {
                            self.regs[ddd as usize] = data;
                            mnemonic = format!("MVI {}, 0x{:02X}", REG_NAMES[ddd as usize], data);
                        }
                    }

                    // -------------------------------------------------------
                    // Return true  (00 CCC 111) — pop stack if flag is set.
                    //
                    // RET (unconditional) = 00 111 111 = 0x3F → ddd=7 CCC=3=P,
                    // but it's treated as "always return". In the 8008, the
                    // unconditional RET is the "return if P is true" opcode with
                    // the parity flag always being... no, 0x3F is just the opcode
                    // for RFP (return if parity false from sss=3), RTP (sss=7).
                    // Wait: 0x3F = 00 111 111 → ddd=7, sss=7. This is sss=7
                    // (return if true) with ddd=7 (condition=P... but 7&3=3=P).
                    // However, the hardware convention is that 0x3F means
                    // unconditional return. We special-case it.
                    // -------------------------------------------------------
                    7 => {
                        if opcode == 0x3F {
                            // RET — unconditional return
                            self.pop_return();
                            mnemonic = "RET".to_string();
                        } else {
                            let ccc = ddd as u8 & 0x03;
                            let (cond_name, _) = Self::cond_name(ccc, true);
                            mnemonic = format!("R{}", cond_name);
                            if self.condition_met(ccc, true) {
                                self.pop_return();
                            }
                        }
                    }

                    // sss=4 is undefined in the standard 8008 instruction set
                    _ => {
                        return Err(format!("Unknown opcode 0x{:02X} at 0x{:04X}", opcode, fetch_pc));
                    }
                }
            }

            // =================================================================
            // GROUP 01: MOV, HLT, Jumps, Calls, IN
            // =================================================================
            //
            // The 8008 group-01 space is shared among four instruction families.
            // The disambiguation requires examining both ddd (bits 5-3) and sss
            // (bits 2-0) — not just the low bits of sss alone.
            //
            // Instruction families in group 01:
            //
            //   HLT  = 0x76 (01 110 110) — special: MOV M,M as halt sentinel
            //
            //   IN   = 01 PPP 001  (sss == 001 exactly, port = ddd)
            //     IN 0 = 0x41, IN 1 = 0x49, ..., IN 7 = 0x79
            //
            //   Jump = 01 CCC T 00  (sss ∈ {000,100} AND ddd ≤ 3 = valid CCC)
            //     T=0: JFC(0x40), JFZ(0x48), JFS(0x50), JFP(0x58)
            //     T=1: JTC(0x44), JTZ(0x4C), JTS(0x54), JTP(0x5C)
            //   JMP  = 0x7C (unconditional)
            //
            //   Call = 01 CCC T 10  (sss ∈ {010,110} AND ddd ≤ 3)
            //     T=0: CFC(0x42), CFZ(0x4A), CFS(0x52), CFP(0x5A)
            //     T=1: CTC(0x46), CTZ(0x4E), CTS(0x56), CTP(0x5E)
            //   CAL  = 0x7E (unconditional)
            //
            //   MOV  = all remaining group-01 opcodes
            //     Examples: MOV A,B (0x78), MOV A,M (0x7E)... wait 0x7E=CAL!
            //     The spec says MOV A,M = 0x7E but that's also CAL. The 8008
            //     resolves this because M is not a valid destination for group-01
            //     when the same opcode is used for CAL. Actually, looking at the
            //     spec example: "MOV A, M = 01 111 110 (0x7E)" — but 0x7E IS CAL!
            //     This is an error in how I read the spec. MOV A,M = 0x7E shares
            //     opcode with CAL. In the real 8008, MOV A,M doesn't exist as a
            //     separate opcode — you use the MOV encoding to read memory, but
            //     0x7E is indeed CAL. The alternative is MOV A,M exists only when
            //     H:L points somewhere safe. In practice, 0x7E always means CAL.
            //     For our simulator, we follow hardware: 0x7E = CAL. MOV to A from
            //     memory is accomplished with other methods.
            //
            //     Real MOV uses opcodes where sss is NOT 0,1,2,4,6 for ddd≤3.
            //     All remaining opcodes (not matched by above rules) are MOV.
            1 => {
                if opcode == 0x76 {
                    // HLT — processor halts (encoded as MOV M,M = 01 110 110)
                    self.halted = true;
                    mnemonic = "HLT".to_string();
                } else if opcode == 0x7C {
                    // JMP — unconditional 3-byte jump
                    let addr_lo = self.fetch_byte();
                    let addr_hi = self.fetch_byte();
                    raw.push(addr_lo);
                    raw.push(addr_hi);
                    let target = ((addr_hi as u16 & 0x3F) << 8) | addr_lo as u16;
                    self.stack[0] = target;
                    mnemonic = format!("JMP 0x{:04X}", target);
                } else if opcode == 0x7E {
                    // CAL — unconditional 3-byte call
                    let addr_lo = self.fetch_byte();
                    let addr_hi = self.fetch_byte();
                    raw.push(addr_lo);
                    raw.push(addr_hi);
                    let target = ((addr_hi as u16 & 0x3F) << 8) | addr_lo as u16;
                    self.push_and_jump(target);
                    mnemonic = format!("CAL 0x{:04X}", target);
                } else if sss == 1 {
                    // IN PPP — read input port into A (port 0-7, encoded in ddd)
                    // Opcodes: 0x41, 0x49, 0x51, 0x59, 0x61, 0x69, 0x71, 0x79
                    let port = ddd as usize;
                    self.regs[REG_A] = self.input_ports[port.min(7)];
                    mnemonic = format!("IN {}", port);
                } else if (sss == 0 || sss == 4) && ddd <= 3 {
                    // Conditional jump — 01 CCC T00 addr_lo addr_hi
                    //   CCC = ddd (bits 5-3), T = bit 2 of opcode (= bit 2 of sss)
                    //   T=0 (sss=0): jump if flag is clear
                    //   T=1 (sss=4): jump if flag is set
                    let addr_lo = self.fetch_byte();
                    let addr_hi = self.fetch_byte();
                    raw.push(addr_lo);
                    raw.push(addr_hi);
                    let target = ((addr_hi as u16 & 0x3F) << 8) | addr_lo as u16;
                    let sense = sss == 4; // T=1 means "jump if true"
                    let ccc = ddd as u8;
                    let (cond_name, _) = Self::cond_name(ccc, sense);
                    mnemonic = format!("J{} 0x{:04X}", cond_name, target);
                    if self.condition_met(ccc, sense) {
                        self.stack[0] = target;
                    }
                } else if (sss == 2 || sss == 6) && ddd <= 3 {
                    // Conditional call — 01 CCC T10 addr_lo addr_hi
                    //   T=0 (sss=2): call if flag is clear
                    //   T=1 (sss=6): call if flag is set
                    let addr_lo = self.fetch_byte();
                    let addr_hi = self.fetch_byte();
                    raw.push(addr_lo);
                    raw.push(addr_hi);
                    let target = ((addr_hi as u16 & 0x3F) << 8) | addr_lo as u16;
                    let sense = sss == 6; // T=1 means "call if true"
                    let ccc = ddd as u8;
                    let (cond_name, _) = Self::cond_name(ccc, sense);
                    mnemonic = format!("C{} 0x{:04X}", cond_name, target);
                    if self.condition_met(ccc, sense) {
                        self.push_and_jump(target);
                    }
                } else {
                    // MOV DDD, SSS — register-to-register copy (1 byte).
                    //
                    // M pseudo-register (code 6) redirects to memory at [H:L].
                    // Flags: not affected.
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
                        self.regs[ddd as usize] = src_val;
                    }
                    mnemonic = format!(
                        "MOV {}, {}",
                        REG_NAMES[ddd as usize],
                        REG_NAMES[sss as usize]
                    );
                }
            }

            // =================================================================
            // GROUP 10: ALU register-source operations
            //   10 OOO SSS
            //   OOO = operation, SSS = source register
            // =================================================================
            2 => {
                let alu_code = ddd; // operation (3 bits)
                let src = sss;     // source register
                let src_val = self.reg_read(src as usize);
                if src as usize == REG_M {
                    mem_address = Some(self.hl_address());
                    mem_value = Some(src_val);
                }
                let a = self.regs[REG_A];
                let (result, carry, clear_carry) = Self::alu_op(alu_code as u8, a, src_val, self.flags.carry);
                if clear_carry {
                    self.flags = Self::compute_flags(result, false, true, self.flags);
                } else {
                    self.flags = Self::compute_flags(result, carry, true, self.flags);
                }
                // CMP doesn't write result back to A
                if alu_code != 7 {
                    self.regs[REG_A] = result;
                }
                mnemonic = format!("{} {}", ALU_MNEMONICS[alu_code as usize], REG_NAMES[src as usize]);
            }

            // =================================================================
            // GROUP 11: ALU immediate operations + unconditional returns (RET in group 00)
            //   11 OOO 100  — ALU immediate (2 bytes)
            // =================================================================
            3 => {
                // ALU immediate: 11 OOO 100
                // sss must be 4 (100) for standard ALU immediate
                // But 0xFF is also HLT
                if opcode == 0xFF {
                    self.halted = true;
                    mnemonic = "HLT".to_string();
                } else if sss == 4 {
                    // ALU immediate
                    let data = self.fetch_byte();
                    raw.push(data);
                    let alu_code = ddd;
                    let a = self.regs[REG_A];
                    let (result, carry, clear_carry) = Self::alu_op(alu_code as u8, a, data, self.flags.carry);
                    if clear_carry {
                        self.flags = Self::compute_flags(result, false, true, self.flags);
                    } else {
                        self.flags = Self::compute_flags(result, carry, true, self.flags);
                    }
                    if alu_code != 7 {
                        self.regs[REG_A] = result;
                    }
                    mnemonic = format!("{} 0x{:02X}", ALU_IMM_MNEMONICS[alu_code as usize], data);
                } else {
                    return Err(format!("Unknown opcode 0x{:02X} at 0x{:04X}", opcode, fetch_pc));
                }
            }

            _ => unreachable!(),
        }

        Ok(Trace {
            address: fetch_pc,
            raw,
            mnemonic,
            a_before,
            a_after: self.regs[REG_A],
            flags_before,
            flags_after: self.flags,
            mem_address,
            mem_value,
        })
    }

    /// Load a program and run it for up to `max_steps` instructions.
    ///
    /// Stops early if the CPU halts (HLT) or an error occurs. Returns all
    /// executed instruction traces in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// use coding_adventures_intel8008_simulator::Simulator;
    /// let mut sim = Simulator::new();
    /// // ADI 42 (0xC4 0x2A), HLT (0x76)
    /// let traces = sim.run(&[0xC4, 0x2A, 0x76], 100);
    /// assert_eq!(sim.a(), 42);
    /// ```
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

    // -------------------------------------------------------------------------
    // Helpers for disassembly
    // -------------------------------------------------------------------------

    /// Produce the condition mnemonic suffix for conditional instructions.
    ///
    /// Returns `(suffix, prefix)` where suffix is the 2-letter code used in
    /// the mnemonic (e.g. "TC" for "True Carry") and prefix indicates T or F.
    fn cond_name(ccc: u8, sense: bool) -> (String, &'static str) {
        let flag_letter = match ccc & 0x03 {
            0 => "C",
            1 => "Z",
            2 => "S",
            3 => "P",
            _ => "?",
        };
        if sense {
            (format!("T{}", flag_letter), "T")
        } else {
            (format!("F{}", flag_letter), "F")
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Basic arithmetic
    // -------------------------------------------------------------------------

    #[test]
    fn test_add_basic() {
        // MVI B,1; MVI A,2; ADD B; HLT
        // 0x06 0x01  0x3E 0x02  0x80  0x76
        let mut sim = Simulator::new();
        let program = &[0x06u8, 0x01, 0x3E, 0x02, 0x80, 0x76];
        let traces = sim.run(program, 100);
        assert_eq!(traces.len(), 4, "expected 4 instructions");
        assert_eq!(sim.a(), 3);
        assert!(!sim.flags().carry);
        assert!(!sim.flags().zero);
        assert!(!sim.flags().sign);
        // 0x03 = 0b00000011 → 2 ones → even parity → P=true
        assert!(sim.flags().parity, "0x03 has even parity");
    }

    #[test]
    fn test_add_overflow_carry() {
        // MVI A, 0xFF; ADI 1 → A=0x00, CY=1, Z=1
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0xFF, 0xC4, 0x01, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x00);
        assert!(sim.flags().carry, "0xFF+1 should set carry");
        assert!(sim.flags().zero, "result is 0");
        assert!(!sim.flags().sign);
        // 0x00 → 0 ones → even parity → P=true
        assert!(sim.flags().parity);
    }

    #[test]
    fn test_subtract_borrow() {
        // MVI A, 0x00; SUI 1 → A=0xFF, CY=1 (borrow), S=1, Z=0, P=1
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x00, 0xD4, 0x01, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0xFF);
        assert!(sim.flags().carry, "0x00-1 borrows → CY=1");
        assert!(sim.flags().sign, "0xFF has bit7 set");
        assert!(!sim.flags().zero);
        // 0xFF = 0b11111111 → 8 ones → even parity
        assert!(sim.flags().parity);
    }

    #[test]
    fn test_adc_with_carry() {
        // MVI A, 0xFE; ADI 1 → A=0xFF, CY=0
        // then ACI 1 → A=0xFF+1=0x00, CY=1 (but carry was 0, so A=0x00? No:
        // First: A=0xFE+1=0xFF, CY=0, then ACI 1 = 0xFF+1+0 = 0x00, CY=1
        // Let me just test ACI directly with preset carry.
        // MVI A, 0xFE; ADI 0x01 (A=0xFF, CY=0); ACI 0x01 (A=0xFF+1+0=0x00, CY=1)
        let mut sim = Simulator::new();
        // 0x3E 0xFE  0xC4 0x01  0xCC 0x01  0x76
        let program = &[0x3Eu8, 0xFE, 0xC4, 0x01, 0xCC, 0x01, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x00);
        assert!(sim.flags().carry);
        assert!(sim.flags().zero);
    }

    // -------------------------------------------------------------------------
    // Logical operations
    // -------------------------------------------------------------------------

    #[test]
    fn test_ana_clears_carry() {
        // Set carry first via ADD overflow, then ANA should clear it.
        // MVI A, 0xFF; ADI 1 → CY=1; ANA A → CY=0
        // ANA A opcode: 10 100 111 = 0xA7
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0xFF, 0xC4, 0x01, 0xA7, 0x76];
        sim.run(program, 100);
        assert!(!sim.flags().carry, "ANA always clears carry");
        assert_eq!(sim.a(), 0x00, "ANA A with A=0 gives 0");
    }

    #[test]
    fn test_xra_self_clears() {
        // XRA A clears A to 0 and sets Z=1, P=1 (even parity of 0)
        // MVI A, 0xAB; XRA A (0xAF); HLT
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0xAB, 0xAF, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x00);
        assert!(sim.flags().zero);
        assert!(!sim.flags().carry);
        assert!(sim.flags().parity);
    }

    #[test]
    fn test_ora_basic() {
        // MVI A, 0x0F; MVI B, 0xF0; ORA B → A=0xFF
        // ORA B = 10 110 000 = 0xB0
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x0F, 0x06, 0xF0, 0xB0, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0xFF);
        assert!(!sim.flags().carry);
        assert!(sim.flags().parity, "0xFF has even parity");
    }

    // -------------------------------------------------------------------------
    // INR / DCR
    // -------------------------------------------------------------------------

    #[test]
    fn test_inr_wrap() {
        // MVI B, 0xFF; INR B → B=0x00, Z=1, CY unchanged
        // INR B = 00 000 000 = 0x00
        let mut sim = Simulator::new();
        // Set carry first so we can verify it's preserved
        let program = &[0x3Eu8, 0xFF, 0xC4, 0x01, // A=0xFF+1=0x00, CY=1
                         0x06, 0xFF,                // MVI B, 0xFF
                         0x00,                      // INR B
                         0x76];
        sim.run(program, 100);
        assert_eq!(sim.b(), 0x00);
        assert!(sim.flags().zero);
        assert!(sim.flags().carry, "INR must not change carry");
    }

    #[test]
    fn test_dcr_wrap() {
        // MVI A, 0x00; DCR A → A=0xFF, S=1, Z=0, P=1, CY unchanged
        // DCR A = 00 111 001 = 0x39
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x00, 0x39, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0xFF);
        assert!(sim.flags().sign);
        assert!(!sim.flags().zero);
        assert!(sim.flags().parity);
    }

    // -------------------------------------------------------------------------
    // Rotate instructions
    // -------------------------------------------------------------------------

    #[test]
    fn test_rlc() {
        // MVI A, 0x80; RLC → A=0x01, CY=1
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x80, 0x02, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x01);
        assert!(sim.flags().carry);
    }

    #[test]
    fn test_rrc() {
        // MVI A, 0x01; RRC → A=0x80, CY=1
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x01, 0x0A, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x80);
        assert!(sim.flags().carry);
    }

    #[test]
    fn test_ral_through_carry() {
        // MVI A, 0xFF; RAL with CY=0 → A=0xFE, CY=1
        // Start with A=0xFF, CY should be 0 initially
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0xFF, 0x12, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0xFE);
        assert!(sim.flags().carry, "bit 7 of 0xFF shifts into carry");
    }

    #[test]
    fn test_rar_through_carry() {
        // MVI A, 0x01; with CY=1: RAR → A=0x80, CY=1
        // First set CY=1 via ADD overflow, then load 0x01, then RAR
        // MVI A, 0xFF; ADI 1 (CY=1); MVI A, 0x01; RAR
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0xFF, 0xC4, 0x01, // set CY=1
                         0x3E, 0x01,               // MVI A, 0x01
                         0x1A, 0x76];              // RAR; HLT
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x80);
        assert!(sim.flags().carry, "bit 0 of 0x01 shifts into carry");
    }

    // -------------------------------------------------------------------------
    // Stack — CALL and RETURN
    // -------------------------------------------------------------------------

    #[test]
    fn test_stack_push_pop() {
        // CAL subroutine at 0x0010, subroutine does MVI A,42, RET
        // Main: MVI A,0 (0x3E 0x00), CAL 0x0010 (0x7E 0x10 0x00), HLT (0x76)
        // Sub at 0x10: MVI A,42 (0x3E 0x2A), RET (0x3F)
        let mut sim = Simulator::new();
        let mut program = vec![0u8; 0x14];
        // main at 0x00: MVI A,0; CAL 0x0010; HLT
        program[0] = 0x3E; program[1] = 0x00;
        program[2] = 0x7E; program[3] = 0x10; program[4] = 0x00;
        program[5] = 0x76;
        // sub at 0x10: MVI A,42; RET
        program[0x10] = 0x3E; program[0x11] = 0x2A;
        program[0x12] = 0x3F;
        sim.run(&program, 100);
        assert_eq!(sim.a(), 42);
    }

    #[test]
    fn test_nested_calls() {
        // Two levels of nesting: main calls f1 at 0x20, f1 calls f2 at 0x40
        // f2 sets A=99, returns; f1 returns; main HLTs
        let mut program = vec![0u8; 0x50];
        // main at 0x00: MVI A,0; CAL 0x0020; HLT
        program[0] = 0x3E; program[1] = 0x00;
        program[2] = 0x7E; program[3] = 0x20; program[4] = 0x00;
        program[5] = 0x76;
        // f1 at 0x20: CAL 0x0040; RET
        program[0x20] = 0x7E; program[0x21] = 0x40; program[0x22] = 0x00;
        program[0x23] = 0x3F;
        // f2 at 0x40: MVI A, 99; RET
        program[0x40] = 0x3E; program[0x41] = 99;
        program[0x42] = 0x3F;
        let mut sim = Simulator::new();
        sim.run(&program, 200);
        assert_eq!(sim.a(), 99);
    }

    // -------------------------------------------------------------------------
    // Memory via M pseudo-register
    // -------------------------------------------------------------------------

    #[test]
    fn test_m_register_memory_access() {
        // Test reading from memory via the M pseudo-register.
        //
        // Note: MOV A,M (01 111 110 = 0x7E) is the SAME opcode as CAL (unconditional
        // call). The 8008 deliberately repurposes this slot for control flow.
        // Instead we read memory into L via MOV L,M, then copy L into A via MOV A,L.
        //
        // MOV L,M = 01 101 110 = 0x6E  (ddd=5=L, sss=6=M — not a call since ddd>3)
        // MOV A,L = 01 111 101 = 0x7D  (ddd=7=A, sss=5=L — plain register move)
        //
        // MVI H = 00 100 110 = 0x26
        // MVI L = 00 101 110 = 0x2E
        // MVI M = 00 110 110 = 0x36
        let mut sim = Simulator::new();
        let program = &[0x26u8, 0x00,  // MVI H, 0       → H=0
                         0x2E, 0x20,   // MVI L, 0x20    → L=0x20; HL=0x0020
                         0x36, 0x55,   // MVI M, 0x55    → mem[0x0020]=0x55
                         0x6E,         // MOV L, M       → L = mem[HL] = 0x55
                         0x7D,         // MOV A, L       → A = L = 0x55
                         0x76];        // HLT
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x55, "should read 0x55 from memory via M register");
    }

    // -------------------------------------------------------------------------
    // Conditional jumps
    // -------------------------------------------------------------------------

    #[test]
    fn test_conditional_jump_taken() {
        // MVI A,0; ADI 0 (A=0, Z=1); JTZ target; MVI A,99 (skipped); HLT
        // target: MVI A,42; HLT
        // JTZ = 01 001 100 = 0x4C; jump to 0x0010
        let mut program = vec![0u8; 0x14];
        program[0] = 0x3E; program[1] = 0x00;   // MVI A,0
        program[2] = 0xC4; program[3] = 0x00;   // ADI 0 → Z=1
        program[4] = 0x4C; program[5] = 0x10; program[6] = 0x00; // JTZ 0x10
        program[7] = 0x3E; program[8] = 99;      // MVI A,99 (should be skipped)
        program[9] = 0x76;                        // HLT (should be skipped)
        program[0x10] = 0x3E; program[0x11] = 42; // MVI A,42
        program[0x12] = 0x76;                      // HLT
        let mut sim = Simulator::new();
        sim.run(&program, 100);
        assert_eq!(sim.a(), 42, "JTZ should have jumped");
    }

    #[test]
    fn test_conditional_jump_not_taken() {
        // MVI A,1; JTZ target (Z=0, not taken); MVI A,99; HLT
        // JTZ = 0x4C
        let mut program = vec![0u8; 0x14];
        program[0] = 0x3E; program[1] = 0x01;   // MVI A,1
        program[2] = 0x4C; program[3] = 0x10; program[4] = 0x00; // JTZ 0x10 (not taken)
        program[5] = 0x3E; program[6] = 99;      // MVI A,99
        program[7] = 0x76;                        // HLT
        program[0x10] = 0x3E; program[0x11] = 42; // MVI A,42 (not reached)
        program[0x12] = 0x76;
        let mut sim = Simulator::new();
        sim.run(&program, 100);
        assert_eq!(sim.a(), 99, "JTZ should NOT have jumped");
    }

    // -------------------------------------------------------------------------
    // Parity flag
    // -------------------------------------------------------------------------

    #[test]
    fn test_parity_flag_even() {
        // 0x03 = 0b00000011 → 2 ones → even parity → P=true
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x03, 0xF4, 0x00, 0x76]; // MVI A,3; ORI 0 (set flags); HLT
        sim.run(program, 100);
        assert!(sim.flags().parity, "0x03 has even parity");
    }

    #[test]
    fn test_parity_flag_odd() {
        // 0x01 = 0b00000001 → 1 one → odd parity → P=false
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x01, 0xF4, 0x00, 0x76]; // MVI A,1; ORI 0; HLT
        sim.run(program, 100);
        assert!(!sim.flags().parity, "0x01 has odd parity");
    }

    #[test]
    fn test_parity_flag_ff() {
        // 0xFF → 8 ones → even parity → P=true
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0xFF, 0xF4, 0x00, 0x76];
        sim.run(program, 100);
        assert!(sim.flags().parity, "0xFF has even parity (8 ones)");
    }

    // -------------------------------------------------------------------------
    // CMP instruction
    // -------------------------------------------------------------------------

    #[test]
    fn test_cmp_equal() {
        // MVI A, 5; CPI 5 → A unchanged=5, Z=1, CY=0
        // CPI = 11 111 100 = 0xFC
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x05, 0xFC, 0x05, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 5, "CMP must not change A");
        assert!(sim.flags().zero);
        assert!(!sim.flags().carry);
    }

    #[test]
    fn test_cmp_less_than() {
        // MVI A, 3; CPI 5 → A=3, CY=1 (borrow: 3<5)
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x03, 0xFC, 0x05, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 3);
        assert!(sim.flags().carry, "3 < 5 means borrow");
        assert!(!sim.flags().zero);
    }

    // -------------------------------------------------------------------------
    // I/O ports
    // -------------------------------------------------------------------------

    #[test]
    fn test_input_port() {
        // Set port 3 to 0xAB externally; IN 3 → A=0xAB
        // IN 3 = 01 011 001 = 0x59 (port in bits 4-1: 3<<1 | 0x41 = 0x41 + 0x18 = 0x59)
        // Actually: IN port → opcode = 0x40 | (port << 1) | 0x01
        // IN 3 = 0x41 | (3 << 1) = 0x41 | 0x06 = 0x47? Let me recalculate.
        // Encoding: 01 PPP P01 where PPP P = port bits, bits 4-1 = port, bits 0 = 1, bits 7-6=01
        // opcode = 0x40 | (port << 1) | 0x01
        // IN 0 = 0x41, IN 1 = 0x43, IN 2 = 0x45, IN 3 = 0x47... wait
        // bits 7-6=01, bits 4-1=port, bits 5=0 (for ports 0-3), bit0=1
        // For IN 0: 01 000 001 = 0x41
        // For IN 1: 01 001 001 = 0x49... but that uses bit 3
        // Actually the spec says:
        // IN 0: 0x41, IN 1: 0x49, IN 2: 0x51, IN 3: 0x59
        // So the port is encoded in bits 4-1: (opcode >> 1) & 0x0F
        // IN 3: (0x59 >> 1) & 0x0F = 0x2C & 0x0F = 0x0C... that's 12, not 3.
        // The spec says port = bits 4-1 but with step 8.
        // IN 0=0x41, IN 1=0x49 → difference is 0x08 = 8
        // So port = (opcode - 0x41) / 8 = (opcode >> 3) - 8... not clean.
        // Let me use: port = (opcode >> 3) & 0x07 (bits 5-3)
        // IN 0=0x41: bits 5-3 = (0x41>>3)&7 = 0x08&7 = 0 ✓
        // IN 1=0x49: bits 5-3 = (0x49>>3)&7 = 0x09&7 = 1 ✓
        // IN 2=0x51: bits 5-3 = (0x51>>3)&7 = 0x0A&7 = 2 ✓
        // IN 3=0x59: bits 5-3 = (0x59>>3)&7 = 0x0B&7 = 3 ✓
        // Good! So port = ddd (bits 5-3). That's what the decoder uses.
        let mut sim = Simulator::new();
        sim.set_input_port(3, 0xAB);
        let program = &[0x59u8, 0x76]; // IN 3; HLT
        sim.run(program, 10);
        assert_eq!(sim.a(), 0xAB);
    }

    #[test]
    fn test_output_port() {
        // OUT instruction — write accumulator to an output port.
        //
        // The 8008 OUT encoding: 00 PPPPP 10 (group=00, bits 2-0=010, sss=2)
        // with the 5-bit port field in bits 5-1. However, ddd=0-3 with sss=2
        // conflicts with the four rotate instructions (RLC, RRC, RAL, RAR).
        // Our simulator treats those specific opcodes as rotates; the remaining
        // sss=2 opcodes (ddd=4-7) are OUT.
        //
        //   0x22 (00 100 010, ddd=4, sss=2): port = (0x22 >> 1) & 0x1F = 17
        //   0x2A (00 101 010, ddd=5, sss=2): port = 21
        //
        // Test port 17 via opcode 0x22:
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x77, // MVI A, 0x77
                         0x22,        // OUT 17 (00 100 010, ddd=4>3, sss=2)
                         0x76];       // HLT
        sim.run(program, 10);
        assert_eq!(sim.get_output_port(17), 0x77, "OUT 17 should write A=0x77 to port 17");
    }

    // -------------------------------------------------------------------------
    // Subroutine — absolute value
    // -------------------------------------------------------------------------

    #[test]
    fn test_abs_value_subroutine() {
        // Absolute value of A using a subroutine.
        //
        // The 8008 Sign flag is only updated by ALU operations, not by MVI.
        // We use `ORI 0x00` (the canonical 8008 idiom for "set flags from A")
        // before the CAL so that S reflects the sign of A=0xF6.
        //
        // ORI 0x00: 0xF4 0x00 (11 110 100 = group3, ddd=6=ORA, sss=4)
        // JFS = jump if sign false (S=0): 01 010 000 = 0x50
        // XRI = 11 101 100 = 0xEC
        // ADI = 11 000 100 = 0xC4
        // RET = 0x3F
        let mut program = vec![0u8; 0x40];
        // main at 0x00
        program[0] = 0x3E; program[1] = 0xF6; // MVI A, 0xF6 (-10)
        program[2] = 0xF4; program[3] = 0x00; // ORI 0 — sets S=1 because bit7 of 0xF6 is set
        program[4] = 0x7E; program[5] = 0x20; program[6] = 0x00; // CAL 0x20
        program[7] = 0x76; // HLT

        // ABS_VAL at 0x20: if S=0, jump to DONE at 0x30
        program[0x20] = 0x50; program[0x21] = 0x30; program[0x22] = 0x00; // JFS 0x30
        program[0x23] = 0xEC; program[0x24] = 0xFF; // XRI 0xFF
        program[0x25] = 0xC4; program[0x26] = 0x01; // ADI 1
        // DONE at 0x30
        program[0x30] = 0x3F; // RET

        let mut sim = Simulator::new();
        sim.run(&program, 200);
        assert_eq!(sim.a(), 10, "abs(-10) should be 10");
    }

    // -------------------------------------------------------------------------
    // Multiply via loop
    // -------------------------------------------------------------------------

    #[test]
    fn test_multiply_4x5_loop() {
        // A = 4 × 5 = 20 using repeated addition
        // MVI B,5; MVI C,4; MVI A,0
        // LOOP: ADD B; DCR C; JFZ LOOP; HLT
        // MVI B = 0x06 0x05
        // MVI C = 0x0E 0x04
        // MVI A = 0x3E 0x00
        // ADD B = 0x80
        // DCR C = 00 001 001 = 0x09
        // JFZ = 01 001 000 = 0x48 (jump if Z=0, i.e. carry false on zero)
        // LOOP is at offset 6
        let mut program = vec![0u8; 20];
        program[0] = 0x06; program[1] = 0x05;  // MVI B,5
        program[2] = 0x0E; program[3] = 0x04;  // MVI C,4
        program[4] = 0x3E; program[5] = 0x00;  // MVI A,0
        // LOOP at 6:
        program[6] = 0x80;                      // ADD B
        program[7] = 0x09;                      // DCR C
        program[8] = 0x48; program[9] = 0x06; program[10] = 0x00; // JFZ 6
        program[11] = 0x76;                     // HLT

        let mut sim = Simulator::new();
        sim.run(&program, 200);
        assert_eq!(sim.a(), 20, "4 × 5 = 20");
    }

    // -------------------------------------------------------------------------
    // RST instruction
    // -------------------------------------------------------------------------

    #[test]
    fn test_rst_instruction() {
        // RST 3 jumps to 0x18; put MVI A,77 there, then RET back
        // RST 3 = 00 011 101 = 0x1D
        let mut program = vec![0u8; 0x20];
        program[0] = 0x1D;   // RST 3 → jump to 0x18
        program[1] = 0x76;   // HLT (after return)
        program[0x18] = 0x3E; program[0x19] = 77; // MVI A, 77
        program[0x1A] = 0x3F; // RET

        let mut sim = Simulator::new();
        sim.run(&program, 100);
        assert_eq!(sim.a(), 77, "RST 3 should jump to 0x18");
    }

    // -------------------------------------------------------------------------
    // SBB instruction
    // -------------------------------------------------------------------------

    #[test]
    fn test_sbb_with_borrow() {
        // MVI A, 5; ADI 0xFF (A=4, CY=1); SBI 1 (A=4-1-1=2)
        // SBI = 11 011 100 = 0xDC
        let mut sim = Simulator::new();
        let program = &[0x3Eu8, 0x05,   // MVI A, 5
                         0xC4, 0xFF,    // ADI 0xFF → A=4, CY=1
                         0xDC, 0x01,    // SBI 1 → A = 4-1-1 = 2
                         0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 2);
    }

    // -------------------------------------------------------------------------
    // MOV instruction variants
    // -------------------------------------------------------------------------

    #[test]
    fn test_mov_reg_to_reg() {
        // MVI B, 0x42; MOV A, B → A=0x42
        // MOV A,B = 01 111 000 = 0x78
        let mut sim = Simulator::new();
        let program = &[0x06u8, 0x42, 0x78, 0x76];
        sim.run(program, 100);
        assert_eq!(sim.a(), 0x42);
    }

    // -------------------------------------------------------------------------
    // Trace verification
    // -------------------------------------------------------------------------

    #[test]
    fn test_trace_contains_correct_info() {
        let mut sim = Simulator::new();
        // MVI A, 5; HLT
        let program = &[0x3Eu8, 0x05, 0x76];
        let traces = sim.run(program, 100);
        assert_eq!(traces.len(), 2);
        // First trace: MVI A, 5
        assert_eq!(traces[0].address, 0);
        assert_eq!(traces[0].a_before, 0);
        assert_eq!(traces[0].a_after, 5);
        assert_eq!(traces[0].raw, vec![0x3E, 0x05]);
        // Second trace: HLT
        assert_eq!(traces[1].address, 2);
        assert!(traces[1].mnemonic.contains("HLT"));
    }
}
