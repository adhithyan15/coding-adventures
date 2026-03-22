//! # Intel 4004 Simulator — the world's first commercial microprocessor.
//!
//! ## What is the Intel 4004?
//!
//! The Intel 4004 was the world's first commercial single-chip microprocessor,
//! released by Intel in 1971. The entire processor contained just 2,300
//! transistors -- compared to billions in modern CPUs.
//!
//! ## Why 4-bit?
//!
//! The 4004 is natively 4-bit. Every data value is 4 bits wide (0-15).
//! All computations are forcefully masked to 4 bits (`& 0xF`). There is
//! no native support for 8-bit, 16-bit, or 32-bit math.
//!
//! This makes perfect sense for its original purpose: calculators.
//! A single decimal digit (0-9) fits in 4 bits, and BCD (Binary-Coded
//! Decimal) arithmetic operates on one digit at a time.
//!
//! ## Accumulator architecture
//!
//! Unlike register-to-register machines (RISC-V, ARM), the 4004 funnels
//! all operations through a single special register: the **Accumulator**.
//!
//! To add two values, you must:
//! 1. Load value A into the Accumulator
//! 2. Swap Accumulator with a general register
//! 3. Load value B into the Accumulator
//! 4. Add the general register to the Accumulator
//!
//! This is more steps than a register machine, but requires simpler hardware.
//!
//! ## Instruction encoding
//!
//! Each instruction is 8 bits (1 byte):
//!
//! ```text
//!     7    4  3    0
//!     +------+------+
//!     | opcode| operand|
//!     | 4 bits| 4 bits |
//!     +------+------+
//! ```
//!
//! ## Supported instructions
//!
//! | Opcode | Mnemonic | Description                           |
//! |--------|----------|---------------------------------------|
//! | 0xD    | LDM N   | Load immediate N into Accumulator     |
//! | 0xB    | XCH Rn  | Exchange Accumulator with register Rn |
//! | 0x8    | ADD Rn  | Add register Rn to Accumulator        |
//! | 0x9    | SUB Rn  | Subtract register Rn from Accumulator |
//! | 0x01   | HLT     | Halt (custom testing instruction)     |

// ===========================================================================
// Trace type
// ===========================================================================

/// A record of one Intel 4004 instruction's execution.
///
/// Because the 4004 is accumulator-based, we track the accumulator
/// and carry flag before and after each instruction.
#[derive(Debug, Clone)]
pub struct Intel4004Trace {
    pub address: usize,
    pub raw: u8,
    pub mnemonic: String,
    pub accumulator_before: u8,
    pub accumulator_after: u8,
    pub carry_before: bool,
    pub carry_after: bool,
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The Intel 4004 simulator.
///
/// Stands on its own rather than using the generic cpu-simulator framework,
/// because the 32-bit-oriented CPU base doesn't fit a 4-bit architecture.
///
/// ```text
///     +----------------------------+
///     |       Intel 4004           |
///     |  Accumulator: 4 bits       |
///     |  Registers: 16 x 4 bits   |
///     |  Carry: 1 bit              |
///     |  Memory: byte-addressable  |
///     +----------------------------+
/// ```
pub struct Intel4004Simulator {
    /// The 4-bit accumulator (0-15).
    pub accumulator: u8,
    /// 16 general-purpose 4-bit registers.
    pub registers: Vec<u8>,
    /// The carry flag -- set when arithmetic overflows or underflows.
    pub carry: bool,
    /// Byte-addressable program memory.
    pub memory: Vec<u8>,
    /// Program counter.
    pub pc: usize,
    /// Whether the CPU has halted.
    pub halted: bool,
}

impl Intel4004Simulator {
    /// Create a new Intel 4004 simulator with the given memory size.
    pub fn new(memory_size: usize) -> Self {
        Intel4004Simulator {
            accumulator: 0,
            registers: vec![0; 16],
            carry: false,
            memory: vec![0; memory_size],
            pc: 0,
            halted: false,
        }
    }

    /// Load a program into memory starting at address 0.
    pub fn load_program(&mut self, program: &[u8]) {
        for (i, &b) in program.iter().enumerate() {
            self.memory[i] = b;
        }
        self.pc = 0;
        self.halted = false;
    }

    /// Execute one instruction and return a trace.
    ///
    /// # Panics
    ///
    /// Panics if the CPU has already halted.
    pub fn step(&mut self) -> Intel4004Trace {
        assert!(!self.halted, "CPU is halted");

        let address = self.pc;
        let raw = self.memory[self.pc];
        self.pc += 1;

        let acc_before = self.accumulator;
        let carry_before = self.carry;

        // Decode: split 8-bit instruction into 4-bit opcode and operand.
        let opcode = (raw >> 4) & 0xF;
        let operand = raw & 0xF;

        let mnemonic = self.execute(opcode, operand, raw);

        Intel4004Trace {
            address,
            raw,
            mnemonic,
            accumulator_before: acc_before,
            accumulator_after: self.accumulator,
            carry_before,
            carry_after: self.carry,
        }
    }

    /// Execute the decoded instruction, updating accumulator/registers/carry.
    ///
    /// Returns the mnemonic string for the trace.
    fn execute(&mut self, opcode: u8, operand: u8, raw: u8) -> String {
        match opcode {
            0xD => {
                // LDM N -- Load immediate value N into the Accumulator.
                // The value is masked to 4 bits (0-15).
                self.accumulator = operand & 0xF;
                format!("LDM {}", operand)
            }
            0xB => {
                // XCH Rn -- Exchange Accumulator with register Rn.
                // Both values are masked to 4 bits.
                let reg = (operand & 0xF) as usize;
                let old_a = self.accumulator;
                self.accumulator = self.registers[reg] & 0xF;
                self.registers[reg] = old_a & 0xF;
                format!("XCH R{}", reg)
            }
            0x8 => {
                // ADD Rn -- Add register Rn to the Accumulator.
                //
                // If the result exceeds 15 (4-bit max), carry is set.
                // The result is masked to 4 bits.
                //
                // Example: A=12, R0=5 => result=17, carry=true, A=1 (17 & 0xF)
                let reg = (operand & 0xF) as usize;
                let result = self.accumulator as u16 + self.registers[reg] as u16;
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                format!("ADD R{}", reg)
            }
            0x9 => {
                // SUB Rn -- Subtract register Rn from the Accumulator.
                //
                // If the result is negative (borrow needed), carry is set.
                // The result wraps around via 4-bit masking.
                //
                // Example: A=0, R0=1 => result=-1, carry=true, A=15 ((-1) & 0xF)
                let reg = (operand & 0xF) as usize;
                let result = self.accumulator as i16 - self.registers[reg] as i16;
                self.carry = result < 0;
                self.accumulator = (result & 0xF) as u8;
                format!("SUB R{}", reg)
            }
            _ if raw == 0x01 => {
                // HLT -- Halt (custom instruction for testing).
                self.halted = true;
                "HLT".to_string()
            }
            _ => {
                format!("UNKNOWN(0x{:02X})", raw)
            }
        }
    }

    /// Run a program to completion or until max_steps is reached.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Vec<Intel4004Trace> {
        self.load_program(program);
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
// Encoding helpers
// ===========================================================================

/// Encode `LDM N` (load immediate N into accumulator).
pub fn encode_ldm(n: u8) -> u8 {
    (0xD << 4) | (n & 0xF)
}

/// Encode `XCH Rn` (exchange accumulator with register n).
pub fn encode_xch(r: u8) -> u8 {
    (0xB << 4) | (r & 0xF)
}

/// Encode `ADD Rn` (add register n to accumulator).
pub fn encode_add(r: u8) -> u8 {
    (0x8 << 4) | (r & 0xF)
}

/// Encode `SUB Rn` (subtract register n from accumulator).
pub fn encode_sub(r: u8) -> u8 {
    (0x9 << 4) | (r & 0xF)
}

/// Encode `HLT` (halt).
pub fn encode_hlt() -> u8 {
    0x01
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test program: compute x = 1 + 2 using accumulator architecture.
    ///
    /// Steps: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
    /// This takes 6 instructions vs 4 on a register machine (RISC-V),
    /// illustrating the accumulator architecture's verbosity.
    #[test]
    fn intel4004_basic_program() {
        let mut sim = Intel4004Simulator::new(4096);
        let program = vec![
            encode_ldm(1),
            encode_xch(0),
            encode_ldm(2),
            encode_add(0),
            encode_xch(1),
            encode_hlt(),
        ];
        let traces = sim.run(&program, 1000);
        assert_eq!(traces.len(), 6);
        assert_eq!(sim.registers[1], 3);
    }

    /// Test subtraction with borrow: 0 - 1 = 15 (wraps), carry = true.
    #[test]
    fn sub_borrow() {
        let mut sim = Intel4004Simulator::new(4096);
        let program = vec![
            encode_ldm(1),
            encode_xch(0),  // R0 = 1
            encode_ldm(0),  // A = 0
            encode_sub(0),  // A = 0 - 1 = -1 => 15 with carry
            encode_hlt(),
        ];
        sim.run(&program, 10);
        assert_eq!(sim.accumulator, 15);
        assert!(sim.carry, "Carry (borrow) should be true");
    }

    /// Unknown instructions should produce a non-empty mnemonic.
    #[test]
    fn unknown_instruction() {
        let mut sim = Intel4004Simulator::new(4096);
        let program = vec![0xFF, encode_hlt()];
        let traces = sim.run(&program, 10);
        assert!(!traces[0].mnemonic.is_empty());
    }

    /// Stepping a halted CPU should panic.
    #[test]
    #[should_panic(expected = "CPU is halted")]
    fn halted_step_panics() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.halted = true;
        sim.step();
    }

    /// Verify carry is set on addition overflow.
    #[test]
    fn add_overflow_carry() {
        let mut sim = Intel4004Simulator::new(4096);
        let program = vec![
            encode_ldm(15),  // A = 15
            encode_xch(0),   // R0 = 15
            encode_ldm(15),  // A = 15
            encode_add(0),   // A = 15 + 15 = 30, carry=true, A = 30 & 0xF = 14
            encode_hlt(),
        ];
        sim.run(&program, 10);
        assert_eq!(sim.accumulator, 14);
        assert!(sim.carry, "Carry should be true on overflow");
    }
}
