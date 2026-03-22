//! # Intel 4004 Simulator — the world's first commercial microprocessor.
//!
//! ## What is the Intel 4004?
//!
//! The Intel 4004 was the world's first commercial single-chip microprocessor,
//! released by Intel in 1971. It was designed by Federico Faggin, Ted Hoff, and
//! Stanley Mazor for the Busicom 141-PF calculator — a Japanese desktop printing
//! calculator. Intel negotiated to retain the rights to the chip design, which
//! turned out to be one of the most consequential business decisions in history.
//!
//! The entire processor contained just 2,300 transistors. For perspective, a
//! modern CPU has billions. The 4004 ran at 740 kHz — about a million times
//! slower than today's processors. Yet it proved that a general-purpose processor
//! could be built on a single chip, launching the microprocessor revolution.
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
//! Each instruction is 8 bits (1 byte), though some instructions are 2 bytes:
//!
//! ```text
//!     7    4  3    0
//!     +------+------+
//!     | opcode| operand|
//!     | 4 bits| 4 bits |
//!     +------+------+
//! ```
//!
//! ## Complete Instruction Set (46 instructions)
//!
//! The 4004 has 46 instructions organized by encoding:
//!
//! | Byte     | Mnemonic     | Description                              |
//! |----------|--------------|------------------------------------------|
//! | 0x00     | NOP          | No operation                             |
//! | 0x01     | HLT          | Halt (simulator-only)                    |
//! | 0x1C AA  | JCN c,addr   | Conditional jump                         |
//! | 0x2P DD  | FIM Pp,data  | Fetch immediate to register pair (even)  |
//! | 0x2P+1   | SRC Pp       | Send register control (odd)              |
//! | 0x3P     | FIN Pp       | Fetch indirect from ROM (even)           |
//! | 0x3P+1   | JIN Pp       | Jump indirect (odd)                      |
//! | 0x4H LL  | JUN addr     | Unconditional jump (12-bit)              |
//! | 0x5H LL  | JMS addr     | Jump to subroutine                       |
//! | 0x6R     | INC Rn       | Increment register                       |
//! | 0x7R AA  | ISZ Rn,addr  | Increment and skip if zero               |
//! | 0x8R     | ADD Rn       | Add register to accumulator with carry    |
//! | 0x9R     | SUB Rn       | Subtract register from accumulator       |
//! | 0xAR     | LD Rn        | Load register into accumulator           |
//! | 0xBR     | XCH Rn       | Exchange accumulator and register         |
//! | 0xCN     | BBL n        | Branch back and load                     |
//! | 0xDN     | LDM n        | Load immediate into accumulator          |
//! | 0xE0     | WRM          | Write RAM main character                 |
//! | 0xE1     | WMP          | Write RAM output port                    |
//! | 0xE2     | WRR          | Write ROM I/O port                       |
//! | 0xE3     | WPM          | Write program RAM (NOP in simulator)     |
//! | 0xE4-E7  | WR0-WR3      | Write RAM status characters 0-3          |
//! | 0xE8     | SBM          | Subtract RAM from accumulator            |
//! | 0xE9     | RDM          | Read RAM main character                  |
//! | 0xEA     | RDR          | Read ROM I/O port                        |
//! | 0xEB     | ADM          | Add RAM to accumulator                   |
//! | 0xEC-EF  | RD0-RD3      | Read RAM status characters 0-3           |
//! | 0xF0     | CLB          | Clear both (A=0, carry=0)                |
//! | 0xF1     | CLC          | Clear carry                              |
//! | 0xF2     | IAC          | Increment accumulator                    |
//! | 0xF3     | CMC          | Complement carry                         |
//! | 0xF4     | CMA          | Complement accumulator                   |
//! | 0xF5     | RAL          | Rotate left through carry                |
//! | 0xF6     | RAR          | Rotate right through carry               |
//! | 0xF7     | TCC          | Transfer carry to accumulator            |
//! | 0xF8     | DAC          | Decrement accumulator                    |
//! | 0xF9     | TCS          | Transfer carry subtract                  |
//! | 0xFA     | STC          | Set carry                                |
//! | 0xFB     | DAA          | Decimal adjust accumulator               |
//! | 0xFC     | KBP          | Keyboard process                         |
//! | 0xFD     | DCL          | Designate command line                   |

// ===========================================================================
// Trace type
// ===========================================================================

/// A record of one Intel 4004 instruction's execution.
///
/// Because the 4004 is accumulator-based, we track the accumulator
/// and carry flag before and after each instruction. For 2-byte
/// instructions, `raw2` holds the second byte.
#[derive(Debug, Clone)]
pub struct Intel4004Trace {
    /// ROM address where this instruction was fetched.
    pub address: usize,
    /// The first (or only) byte of the instruction.
    pub raw: u8,
    /// The second byte for 2-byte instructions, or `None`.
    pub raw2: Option<u8>,
    /// Human-readable disassembly (e.g., "LDM 5", "JUN 0x100").
    pub mnemonic: String,
    /// Accumulator value before execution.
    pub accumulator_before: u8,
    /// Accumulator value after execution.
    pub accumulator_after: u8,
    /// Carry flag before execution.
    pub carry_before: bool,
    /// Carry flag after execution.
    pub carry_after: bool,
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The Intel 4004 simulator — all 46 real instructions plus HLT.
///
/// Stands on its own rather than using the generic cpu-simulator framework,
/// because the 32-bit-oriented CPU base doesn't fit a 4-bit architecture.
///
/// ```text
///     +-------------------------------------------+
///     |           Intel 4004                       |
///     |  Accumulator:   4 bits                     |
///     |  Registers:     16 × 4 bits (R0-R15)      |
///     |  Carry:         1 bit                      |
///     |  PC:            12 bits (0-4095)           |
///     |  Stack:         3-level × 12 bits          |
///     |  ROM:           4096 × 8-bit               |
///     |  RAM:           4 banks × 4 regs × 16 nib  |
///     |  RAM status:    4 banks × 4 regs × 4 nib   |
///     +-------------------------------------------+
/// ```
pub struct Intel4004Simulator {
    // --- CPU Registers ---

    /// The 4-bit accumulator (0-15). The heart of computation.
    pub accumulator: u8,

    /// 16 general-purpose 4-bit registers (R0-R15).
    ///
    /// Organized as 8 pairs: P0=(R0,R1), P1=(R2,R3), ..., P7=(R14,R15).
    /// The even register is the high nibble, odd is the low nibble when
    /// reading a pair as an 8-bit value.
    pub registers: [u8; 16],

    /// The carry flag — set when arithmetic overflows or underflows.
    ///
    /// For SUB, the carry semantics are INVERTED from what you'd expect:
    /// - carry=true means NO borrow occurred (result >= 0)
    /// - carry=false means borrow occurred (result was negative)
    pub carry: bool,

    // --- Memory ---

    /// Byte-addressable program memory (ROM). Up to 4096 bytes.
    pub memory: Vec<u8>,

    /// Program counter (12-bit, addresses 0-4095).
    pub pc: usize,

    // --- Hardware Call Stack ---

    /// 3-level hardware call stack storing 12-bit return addresses.
    ///
    /// The real 4004 has exactly 3 stack levels — no more. If you call
    /// a 4th subroutine without returning, the oldest return address is
    /// silently overwritten. There is no stack overflow exception.
    pub hw_stack: [u16; 3],

    /// Stack pointer (0-2), wraps modulo 3.
    pub stack_pointer: usize,

    // --- RAM ---

    /// Main RAM: 4 banks × 4 registers × 16 nibbles.
    ///
    /// Addressed by: `ram[ram_bank][ram_register][ram_character]`
    /// Each entry is a 4-bit value (0-15).
    pub ram: [[[u8; 16]; 4]; 4],

    /// RAM status characters: 4 banks × 4 registers × 4 status nibbles.
    ///
    /// Each RAM register has 4 status nibbles in addition to the 16 main
    /// nibbles. These were used for flags, signs, etc. in BCD calculators.
    pub ram_status: [[[u8; 4]; 4]; 4],

    /// RAM output port — one per bank, written by WMP.
    pub ram_output: [u8; 4],

    /// Selected RAM bank (0-3), set by DCL instruction.
    pub ram_bank: usize,

    /// Selected RAM register (0-3), set by SRC instruction (high nibble of pair).
    pub ram_register: usize,

    /// Selected RAM character (0-15), set by SRC instruction (low nibble of pair).
    pub ram_character: usize,

    // --- ROM I/O ---

    /// ROM I/O port value, written by WRR, read by RDR.
    pub rom_port: u8,

    // --- Control ---

    /// Whether the CPU has halted (HLT instruction executed).
    pub halted: bool,
}

// ===========================================================================
// Helper: detect 2-byte instructions
// ===========================================================================

/// Returns true if the raw byte starts a 2-byte instruction.
///
/// The 4004 has five 2-byte instruction families:
///
/// | Upper nibble | Instruction | Condition           |
/// |--------------|-------------|---------------------|
/// | 0x1          | JCN         | always 2-byte       |
/// | 0x2          | FIM         | even lower nibble   |
/// | 0x4          | JUN         | always 2-byte       |
/// | 0x5          | JMS         | always 2-byte       |
/// | 0x7          | ISZ         | always 2-byte       |
///
/// Note: 0x2 with odd lower nibble is SRC, which is 1-byte.
fn is_two_byte(raw: u8) -> bool {
    let upper = (raw >> 4) & 0xF;
    match upper {
        0x1 | 0x4 | 0x5 | 0x7 => true,
        0x2 => (raw & 0x1) == 0, // FIM (even) is 2-byte, SRC (odd) is 1-byte
        _ => false,
    }
}

impl Intel4004Simulator {
    /// Create a new Intel 4004 simulator with the given memory size.
    ///
    /// The real 4004 addresses 4096 bytes of ROM. Pass `4096` for
    /// authentic behavior.
    pub fn new(memory_size: usize) -> Self {
        Intel4004Simulator {
            accumulator: 0,
            registers: [0; 16],
            carry: false,
            memory: vec![0; memory_size],
            pc: 0,
            hw_stack: [0; 3],
            stack_pointer: 0,
            ram: [[[0; 16]; 4]; 4],
            ram_status: [[[0; 4]; 4]; 4],
            ram_output: [0; 4],
            ram_bank: 0,
            ram_register: 0,
            ram_character: 0,
            rom_port: 0,
            halted: false,
        }
    }

    /// Load a program into memory starting at address 0.
    ///
    /// Resets the program counter and halted flag, but preserves
    /// registers, accumulator, carry, RAM, and stack state.
    pub fn load_program(&mut self, program: &[u8]) {
        for (i, &b) in program.iter().enumerate() {
            self.memory[i] = b;
        }
        self.pc = 0;
        self.halted = false;
    }

    /// Reset all CPU state to initial values.
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.registers = [0; 16];
        self.carry = false;
        for byte in self.memory.iter_mut() {
            *byte = 0;
        }
        self.pc = 0;
        self.hw_stack = [0; 3];
        self.stack_pointer = 0;
        self.ram = [[[0; 16]; 4]; 4];
        self.ram_status = [[[0; 4]; 4]; 4];
        self.ram_output = [0; 4];
        self.ram_bank = 0;
        self.ram_register = 0;
        self.ram_character = 0;
        self.rom_port = 0;
        self.halted = false;
    }

    // ===================================================================
    // Register pair helpers
    // ===================================================================

    /// Read an 8-bit value from a register pair.
    ///
    /// Pair 0 = (R0, R1), Pair 1 = (R2, R3), ..., Pair 7 = (R14, R15).
    /// The even register is the high nibble, the odd register is the low nibble.
    ///
    /// ```text
    ///     Pair P = (R[2*P], R[2*P+1])
    ///     Value  = (R[2*P] << 4) | R[2*P+1]
    /// ```
    fn read_pair(&self, pair_idx: usize) -> u8 {
        let high_reg = pair_idx * 2;
        let low_reg = high_reg + 1;
        (self.registers[high_reg] << 4) | self.registers[low_reg]
    }

    /// Write an 8-bit value to a register pair.
    ///
    /// High nibble goes to the even register, low nibble to the odd register.
    fn write_pair(&mut self, pair_idx: usize, value: u8) {
        let high_reg = pair_idx * 2;
        let low_reg = high_reg + 1;
        self.registers[high_reg] = (value >> 4) & 0xF;
        self.registers[low_reg] = value & 0xF;
    }

    // ===================================================================
    // Stack helpers
    // ===================================================================

    /// Push a return address onto the 3-level hardware stack.
    ///
    /// The real 4004 wraps silently on overflow — the 4th push overwrites
    /// the oldest entry. There is no stack overflow exception.
    fn stack_push(&mut self, address: u16) {
        self.hw_stack[self.stack_pointer] = address & 0xFFF;
        self.stack_pointer = (self.stack_pointer + 1) % 3;
    }

    /// Pop a return address from the hardware stack.
    fn stack_pop(&mut self) -> u16 {
        // (sp - 1) mod 3, using +2 to avoid underflow on unsigned
        self.stack_pointer = (self.stack_pointer + 2) % 3;
        self.hw_stack[self.stack_pointer]
    }

    // ===================================================================
    // RAM helpers
    // ===================================================================

    /// Read the current RAM main character (addressed by SRC + DCL).
    fn ram_read_main(&self) -> u8 {
        self.ram[self.ram_bank][self.ram_register][self.ram_character]
    }

    /// Write to the current RAM main character.
    fn ram_write_main(&mut self, value: u8) {
        self.ram[self.ram_bank][self.ram_register][self.ram_character] = value & 0xF;
    }

    /// Read a RAM status character (0-3) for the current register.
    fn ram_read_status(&self, index: usize) -> u8 {
        self.ram_status[self.ram_bank][self.ram_register][index]
    }

    /// Write a RAM status character (0-3).
    fn ram_write_status(&mut self, index: usize, value: u8) {
        self.ram_status[self.ram_bank][self.ram_register][index] = value & 0xF;
    }

    // ===================================================================
    // Fetch-decode-execute
    // ===================================================================

    /// Execute one instruction and return a trace.
    ///
    /// # Panics
    ///
    /// Panics if the CPU has already halted.
    pub fn step(&mut self) -> Intel4004Trace {
        assert!(!self.halted, "CPU is halted");

        let address = self.pc;
        let raw = self.memory[self.pc];
        let acc_before = self.accumulator;
        let carry_before = self.carry;

        // Check if this is a 2-byte instruction and fetch the second byte.
        let raw2 = if is_two_byte(raw) {
            let b2 = self.memory[self.pc + 1];
            self.pc += 2;
            Some(b2)
        } else {
            self.pc += 1;
            None
        };

        let mnemonic = self.execute(raw, raw2, address);

        Intel4004Trace {
            address,
            raw,
            raw2,
            mnemonic,
            accumulator_before: acc_before,
            accumulator_after: self.accumulator,
            carry_before,
            carry_after: self.carry,
        }
    }

    /// Execute the decoded instruction, updating all CPU state.
    ///
    /// Dispatches based on the upper nibble (and full byte for 0xE_/0xF_ range).
    /// Returns the mnemonic string for the trace.
    ///
    /// # Instruction encoding overview
    ///
    /// ```text
    ///     Upper nibble    Instruction(s)
    ///     0x0             NOP (0x00), HLT (0x01)
    ///     0x1             JCN (2-byte conditional jump)
    ///     0x2             FIM (even, 2-byte) / SRC (odd, 1-byte)
    ///     0x3             FIN (even) / JIN (odd)
    ///     0x4             JUN (2-byte unconditional jump)
    ///     0x5             JMS (2-byte jump to subroutine)
    ///     0x6             INC (increment register)
    ///     0x7             ISZ (2-byte increment and skip if zero)
    ///     0x8             ADD (add register with carry)
    ///     0x9             SUB (subtract register with borrow)
    ///     0xA             LD  (load register into accumulator)
    ///     0xB             XCH (exchange accumulator and register)
    ///     0xC             BBL (branch back and load)
    ///     0xD             LDM (load immediate)
    ///     0xE             I/O operations (full byte dispatch)
    ///     0xF             Accumulator operations (full byte dispatch)
    /// ```
    fn execute(&mut self, raw: u8, raw2: Option<u8>, address: usize) -> String {
        let upper = (raw >> 4) & 0xF;
        let lower = raw & 0xF;

        match upper {
            // =============================================================
            // 0x0: NOP and HLT
            // =============================================================
            0x0 => {
                match raw {
                    0x00 => {
                        // NOP — No operation. The simplest instruction: do nothing,
                        // just advance to the next byte. Used for timing delays or
                        // alignment padding.
                        "NOP".to_string()
                    }
                    0x01 => {
                        // HLT — Halt execution. This is a simulator-only instruction
                        // that doesn't exist on the real 4004 hardware. We use it to
                        // stop execution cleanly in tests.
                        self.halted = true;
                        "HLT".to_string()
                    }
                    _ => format!("UNKNOWN(0x{raw:02X})"),
                }
            }

            // =============================================================
            // 0x1: JCN — Conditional jump (2-byte)
            // =============================================================
            //
            // Format: 0x1C AA
            //   C = condition nibble (4 bits of test flags)
            //   AA = target address within the same 256-byte page
            //
            // The condition nibble C has 4 bits:
            //   Bit 3 (0x8): INVERT — if set, invert the final test result
            //   Bit 2 (0x4): TEST_ZERO — test if accumulator == 0
            //   Bit 1 (0x2): TEST_CARRY — test if carry == 1
            //   Bit 0 (0x1): TEST_PIN — test input pin (always 0 in simulator)
            //
            // Multiple test bits can be set — they are OR'd together.
            // If the (possibly inverted) result is True, the jump is taken.
            0x1 => {
                let cond = lower;
                let addr = raw2.unwrap_or(0);

                // Evaluate condition tests (OR'd together)
                let mut test_result = false;
                if cond & 0x4 != 0 {
                    // Test accumulator == 0
                    test_result = test_result || (self.accumulator == 0);
                }
                if cond & 0x2 != 0 {
                    // Test carry == 1
                    test_result = test_result || self.carry;
                }
                if cond & 0x1 != 0 {
                    // Test input pin (always 0 = not asserted in simulator)
                    // test_result = test_result || false;
                }

                // Invert if bit 3 is set
                if cond & 0x8 != 0 {
                    test_result = !test_result;
                }

                if test_result {
                    // Jump: target is within the same 256-byte page
                    let page = self.pc & 0xF00;
                    self.pc = page | (addr as usize);
                }

                format!("JCN {cond},{addr:02X}")
            }

            // =============================================================
            // 0x2: FIM (even) / SRC (odd)
            // =============================================================
            0x2 => {
                let pair = (lower >> 1) as usize;

                if lower & 0x1 == 0 {
                    // FIM Pp,data (0x2P 0xDD): Fetch immediate to register pair.
                    //
                    // Load the 8-bit immediate data byte into register pair Pp.
                    // High nibble → R(2*p), low nibble → R(2*p+1).
                    // This is the only way to load an 8-bit value in one instruction.
                    let data = raw2.unwrap_or(0);
                    self.write_pair(pair, data);
                    format!("FIM P{pair},0x{data:02X}")
                } else {
                    // SRC Pp (0x2P+1): Send Register Control.
                    //
                    // Send the 8-bit value in register pair Pp as an address for
                    // subsequent RAM/ROM I/O operations:
                    //   High nibble → ram_register (0-3)
                    //   Low nibble  → ram_character (0-15)
                    //
                    // This is like setting a pointer before a memory read/write.
                    let pair_val = self.read_pair(pair);
                    self.ram_register = ((pair_val >> 4) & 0xF) as usize;
                    self.ram_character = (pair_val & 0xF) as usize;
                    format!("SRC P{pair}")
                }
            }

            // =============================================================
            // 0x3: FIN (even) / JIN (odd)
            // =============================================================
            0x3 => {
                let pair = (lower >> 1) as usize;

                if lower & 0x1 == 0 {
                    // FIN Pp (0x3P): Fetch Indirect from ROM.
                    //
                    // Read the ROM byte at the address formed by:
                    //   - High bits: current page (bits 11-8 of the instruction's PC)
                    //   - Low bits:  register pair P0 (R0:R1)
                    //
                    // Store the result into register pair Pp.
                    // Note: the address comes from P0 regardless of which pair
                    // is the destination.
                    let p0_val = self.read_pair(0) as usize;
                    let current_page = address & 0xF00;
                    let rom_addr = current_page | p0_val;
                    let rom_byte = if rom_addr < self.memory.len() {
                        self.memory[rom_addr]
                    } else {
                        0
                    };
                    self.write_pair(pair, rom_byte);
                    format!("FIN P{pair}")
                } else {
                    // JIN Pp (0x3P+1): Jump Indirect.
                    //
                    // Jump to the address formed by:
                    //   - High bits: current page (bits 11-8 of this instruction's PC)
                    //   - Low bits:  register pair Pp (8-bit value)
                    let pair_val = self.read_pair(pair) as usize;
                    let current_page = address & 0xF00;
                    self.pc = current_page | pair_val;
                    format!("JIN P{pair}")
                }
            }

            // =============================================================
            // 0x4: JUN — Unconditional jump (2-byte)
            // =============================================================
            //
            // Format: 0x4H LL
            //   H = high 4 bits of 12-bit address (from lower nibble of byte 1)
            //   LL = low 8 bits of 12-bit address (byte 2)
            //   Full address = (H << 8) | LL
            //
            // This is the 4004's "goto" — jumps anywhere in the 4KB ROM space.
            0x4 => {
                let addr = ((lower as u16) << 8) | (raw2.unwrap_or(0) as u16);
                self.pc = addr as usize;
                format!("JUN 0x{addr:03X}")
            }

            // =============================================================
            // 0x5: JMS — Jump to subroutine (2-byte)
            // =============================================================
            //
            // Format: 0x5H LL
            //   Same address encoding as JUN.
            //
            // Pushes the return address (address of next instruction after JMS)
            // onto the 3-level hardware stack, then jumps to the target.
            // The return address is `address + 2` since JMS is 2 bytes.
            0x5 => {
                let addr = ((lower as u16) << 8) | (raw2.unwrap_or(0) as u16);
                // Push return address (the instruction AFTER this 2-byte JMS)
                let return_addr = (address + 2) as u16;
                self.stack_push(return_addr);
                self.pc = addr as usize;
                format!("JMS 0x{addr:03X}")
            }

            // =============================================================
            // 0x6: INC — Increment register
            // =============================================================
            //
            // INC Rn: Rn = (Rn + 1) & 0xF
            //
            // Note: INC does NOT affect the carry flag. It wraps from 15 to 0
            // silently. This is different from IAC (increment accumulator),
            // which does set carry.
            0x6 => {
                let reg = lower as usize;
                self.registers[reg] = (self.registers[reg] + 1) & 0xF;
                format!("INC R{reg}")
            }

            // =============================================================
            // 0x7: ISZ — Increment and Skip if Zero (2-byte)
            // =============================================================
            //
            // Format: 0x7R AA
            //   R = register number
            //   AA = target address within the same 256-byte page
            //
            // Increment Rn. If Rn != 0 after increment, jump to addr.
            // If Rn == 0 (wrapped from 15), fall through to next instruction.
            //
            // This is the 4004's loop counter instruction. Load a register with
            // a negative count in 4-bit (e.g., -4 = 12), then ISZ loops until
            // the register wraps to 0.
            0x7 => {
                let reg = lower as usize;
                let addr = raw2.unwrap_or(0);

                self.registers[reg] = (self.registers[reg] + 1) & 0xF;

                if self.registers[reg] != 0 {
                    // Not zero yet — jump back to continue loop
                    let page = self.pc & 0xF00;
                    self.pc = page | (addr as usize);
                }
                // If zero, fall through (pc already advanced past 2-byte instruction)

                format!("ISZ R{reg},0x{addr:02X}")
            }

            // =============================================================
            // 0x8: ADD — Add register to accumulator with carry
            // =============================================================
            //
            // A = A + Rn + carry
            // Carry is set if result > 15.
            //
            // The carry flag participates in the addition — this is how
            // multi-digit BCD arithmetic works. After adding two BCD digits,
            // the carry propagates to the next digit pair.
            0x8 => {
                let reg = lower as usize;
                let result = self.accumulator as u16
                    + self.registers[reg] as u16
                    + if self.carry { 1 } else { 0 };
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                format!("ADD R{reg}")
            }

            // =============================================================
            // 0x9: SUB — Subtract register from accumulator with borrow
            // =============================================================
            //
            // The 4004 uses complement-add for subtraction:
            //   A = A + (~Rn & 0xF) + borrow_in
            //   where borrow_in = 0 if carry (previous borrow), 1 if no carry
            //
            // The carry flag is INVERTED from what you'd expect:
            //   - carry=true  means NO borrow occurred (result >= 0)
            //   - carry=false means borrow occurred (result was negative)
            //
            // This matches the MCS-4 manual. The initial carry state acts as
            // an inverse borrow-in: carry=false means "there was a previous borrow",
            // so we subtract an extra 1.
            0x9 => {
                let reg = lower as usize;
                let complement = (!self.registers[reg]) & 0xF;
                let borrow_in: u16 = if self.carry { 0 } else { 1 };
                let result = self.accumulator as u16 + complement as u16 + borrow_in;
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                format!("SUB R{reg}")
            }

            // =============================================================
            // 0xA: LD — Load register into accumulator
            // =============================================================
            //
            // A = Rn (4-bit value)
            //
            // This is a simple register-to-accumulator copy. The register is
            // not modified.
            0xA => {
                let reg = lower as usize;
                self.accumulator = self.registers[reg] & 0xF;
                format!("LD R{reg}")
            }

            // =============================================================
            // 0xB: XCH — Exchange accumulator with register
            // =============================================================
            //
            // Swap A and Rn (both 4-bit values).
            0xB => {
                let reg = lower as usize;
                let old_a = self.accumulator;
                self.accumulator = self.registers[reg] & 0xF;
                self.registers[reg] = old_a & 0xF;
                format!("XCH R{reg}")
            }

            // =============================================================
            // 0xC: BBL — Branch Back and Load
            // =============================================================
            //
            // Pop the top of the hardware stack, load N into the accumulator,
            // and jump to the popped address.
            //
            // This is the 4004's "return from subroutine" instruction with a
            // twist — it also loads an immediate value into A. This lets a
            // subroutine return a simple status code (0 = success, 1 = error, etc.)
            0xC => {
                let n = lower;
                self.accumulator = n & 0xF;
                let return_addr = self.stack_pop();
                self.pc = return_addr as usize;
                format!("BBL {n}")
            }

            // =============================================================
            // 0xD: LDM — Load immediate into accumulator
            // =============================================================
            //
            // A = N (the lower nibble of the instruction byte).
            // The simplest way to get a constant value into the accumulator.
            0xD => {
                self.accumulator = lower & 0xF;
                format!("LDM {lower}")
            }

            // =============================================================
            // 0xE: I/O operations (full byte dispatch)
            // =============================================================
            //
            // These instructions interact with the 4004's RAM and ROM I/O
            // subsystem. The full byte determines the operation.
            0xE => self.execute_io(raw),

            // =============================================================
            // 0xF: Accumulator operations (full byte dispatch)
            // =============================================================
            //
            // These instructions manipulate the accumulator and carry flag
            // in various ways. The full byte determines the operation.
            0xF => self.execute_accumulator(raw),

            _ => format!("UNKNOWN(0x{raw:02X})"),
        }
    }

    /// Execute I/O instructions (0xE0-0xEF).
    ///
    /// These instructions read/write the 4004's RAM subsystem (main characters,
    /// status characters, output ports) and ROM I/O port. The RAM address is
    /// set by a prior SRC instruction; the RAM bank is set by DCL.
    fn execute_io(&mut self, raw: u8) -> String {
        match raw {
            // --- RAM/ROM Write Operations ---

            0xE0 => {
                // WRM: Write accumulator to RAM main character.
                // Writes A to ram[bank][register][character].
                self.ram_write_main(self.accumulator);
                "WRM".to_string()
            }
            0xE1 => {
                // WMP: Write accumulator to RAM output port.
                // Each bank has one output port nibble.
                self.ram_output[self.ram_bank] = self.accumulator & 0xF;
                "WMP".to_string()
            }
            0xE2 => {
                // WRR: Write accumulator to ROM I/O port.
                self.rom_port = self.accumulator & 0xF;
                "WRR".to_string()
            }
            0xE3 => {
                // WPM: Write Program Memory. On the real 4004, this was used
                // for EPROM programming — not applicable in simulation.
                // We treat it as a NOP.
                "WPM".to_string()
            }
            0xE4 => {
                // WR0: Write accumulator to RAM status character 0.
                self.ram_write_status(0, self.accumulator);
                "WR0".to_string()
            }
            0xE5 => {
                // WR1: Write accumulator to RAM status character 1.
                self.ram_write_status(1, self.accumulator);
                "WR1".to_string()
            }
            0xE6 => {
                // WR2: Write accumulator to RAM status character 2.
                self.ram_write_status(2, self.accumulator);
                "WR2".to_string()
            }
            0xE7 => {
                // WR3: Write accumulator to RAM status character 3.
                self.ram_write_status(3, self.accumulator);
                "WR3".to_string()
            }

            // --- RAM Arithmetic ---

            0xE8 => {
                // SBM: Subtract RAM main character from accumulator.
                //
                // Same complement-add logic as SUB:
                //   A = A + (~ram_val & 0xF) + borrow_in
                let ram_val = self.ram_read_main();
                let complement = (!ram_val) & 0xF;
                let borrow_in: u16 = if self.carry { 0 } else { 1 };
                let result = self.accumulator as u16 + complement as u16 + borrow_in;
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                "SBM".to_string()
            }

            // --- RAM/ROM Read Operations ---

            0xE9 => {
                // RDM: Read RAM main character into accumulator.
                self.accumulator = self.ram_read_main();
                "RDM".to_string()
            }
            0xEA => {
                // RDR: Read ROM I/O port into accumulator.
                self.accumulator = self.rom_port & 0xF;
                "RDR".to_string()
            }
            0xEB => {
                // ADM: Add RAM main character to accumulator with carry.
                //
                // Same logic as ADD but reads from RAM instead of a register.
                let ram_val = self.ram_read_main();
                let result =
                    self.accumulator as u16 + ram_val as u16 + if self.carry { 1 } else { 0 };
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                "ADM".to_string()
            }
            0xEC => {
                // RD0: Read RAM status character 0 into accumulator.
                self.accumulator = self.ram_read_status(0);
                "RD0".to_string()
            }
            0xED => {
                // RD1: Read RAM status character 1 into accumulator.
                self.accumulator = self.ram_read_status(1);
                "RD1".to_string()
            }
            0xEE => {
                // RD2: Read RAM status character 2 into accumulator.
                self.accumulator = self.ram_read_status(2);
                "RD2".to_string()
            }
            0xEF => {
                // RD3: Read RAM status character 3 into accumulator.
                self.accumulator = self.ram_read_status(3);
                "RD3".to_string()
            }

            _ => format!("UNKNOWN(0x{raw:02X})"),
        }
    }

    /// Execute accumulator manipulation instructions (0xF0-0xFD).
    ///
    /// These are single-byte instructions that operate on the accumulator
    /// and/or carry flag. They form the core of the 4004's arithmetic and
    /// logic capability beyond simple add/subtract.
    fn execute_accumulator(&mut self, raw: u8) -> String {
        match raw {
            0xF0 => {
                // CLB: Clear Both. A = 0, carry = 0.
                // The nuclear option: zero everything.
                self.accumulator = 0;
                self.carry = false;
                "CLB".to_string()
            }
            0xF1 => {
                // CLC: Clear Carry. carry = 0.
                // Useful before an ADD chain when you don't want a stale carry
                // from a previous operation affecting the first addition.
                self.carry = false;
                "CLC".to_string()
            }
            0xF2 => {
                // IAC: Increment Accumulator. A = (A + 1) & 0xF.
                // Carry is set if A was 15 (wraps to 0).
                let result = self.accumulator as u16 + 1;
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                "IAC".to_string()
            }
            0xF3 => {
                // CMC: Complement Carry. carry = !carry.
                self.carry = !self.carry;
                "CMC".to_string()
            }
            0xF4 => {
                // CMA: Complement Accumulator. A = ~A & 0xF (4-bit NOT).
                //
                // Flips all 4 bits: 0b0101 → 0b1010, 0b1111 → 0b0000.
                // Used in BCD subtraction (complement + add + 1 = negate).
                self.accumulator = (!self.accumulator) & 0xF;
                "CMA".to_string()
            }
            0xF5 => {
                // RAL: Rotate Accumulator Left through carry.
                //
                // This is a 5-bit rotation: the carry acts as bit 4.
                //
                //   Before: [carry | A3 A2 A1 A0]
                //   After:  [A3   | A2 A1 A0 carry_old]
                //
                // The highest bit (A3) shifts into carry, and the old carry
                // shifts into the lowest bit (A0).
                let old_carry = u8::from(self.carry);
                self.carry = (self.accumulator & 0x8) != 0;
                self.accumulator = ((self.accumulator << 1) | old_carry) & 0xF;
                "RAL".to_string()
            }
            0xF6 => {
                // RAR: Rotate Accumulator Right through carry.
                //
                //   Before: [carry | A3 A2 A1 A0]
                //   After:  [A0   | carry_old A3 A2 A1]
                //
                // The lowest bit (A0) shifts into carry, and the old carry
                // shifts into the highest bit (A3).
                let old_carry = u8::from(self.carry);
                self.carry = (self.accumulator & 0x1) != 0;
                self.accumulator = ((self.accumulator >> 1) | (old_carry << 3)) & 0xF;
                "RAR".to_string()
            }
            0xF7 => {
                // TCC: Transfer Carry to accumulator and Clear carry.
                //
                // A = 1 if carry was set, else 0. Carry is always cleared.
                // Useful for extracting the carry flag as a data value.
                self.accumulator = if self.carry { 1 } else { 0 };
                self.carry = false;
                "TCC".to_string()
            }
            0xF8 => {
                // DAC: Decrement Accumulator. A = (A - 1) & 0xF.
                //
                // Carry is SET if no borrow (A > 0), CLEARED if borrow (A was 0).
                // When A=0: result = -1, carry=false, A=15.
                // When A=5: result = 4, carry=true, A=4.
                let result = self.accumulator as i16 - 1;
                self.carry = result >= 0;
                self.accumulator = (result & 0xF) as u8;
                "DAC".to_string()
            }
            0xF9 => {
                // TCS: Transfer Carry Subtract.
                //
                // A = 10 if carry was set, else 9. Carry is always cleared.
                //
                // This is used in BCD subtraction: it provides the
                // tens-complement correction factor.
                self.accumulator = if self.carry { 10 } else { 9 };
                self.carry = false;
                "TCS".to_string()
            }
            0xFA => {
                // STC: Set Carry. carry = 1.
                self.carry = true;
                "STC".to_string()
            }
            0xFB => {
                // DAA: Decimal Adjust Accumulator (BCD correction).
                //
                // If A > 9 or carry is set, add 6 to A.
                // If the addition causes overflow past 15, set carry.
                //
                // This instruction exists because the 4004 was built for BCD
                // calculators. When you add two BCD digits (0-9 each), the
                // result might be > 9 (e.g., 7 + 8 = 15). DAA corrects this
                // by adding 6, wrapping to the correct BCD digit (15 + 6 = 21,
                // keep lower nibble 5, set carry for the tens digit).
                if self.accumulator > 9 || self.carry {
                    let result = self.accumulator as u16 + 6;
                    if result > 0xF {
                        self.carry = true;
                    }
                    self.accumulator = (result & 0xF) as u8;
                }
                "DAA".to_string()
            }
            0xFC => {
                // KBP: Keyboard Process.
                //
                // Converts a 1-hot encoded input to a binary position number:
                //   0b0000 (0) → 0  (no key pressed)
                //   0b0001 (1) → 1  (key 1)
                //   0b0010 (2) → 2  (key 2)
                //   0b0100 (4) → 3  (key 3)
                //   0b1000 (8) → 4  (key 4)
                //   anything else → 15 (error: multiple keys)
                //
                // This was designed for the Busicom calculator's keyboard scanning.
                self.accumulator = match self.accumulator {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    4 => 3,
                    8 => 4,
                    _ => 15,
                };
                "KBP".to_string()
            }
            0xFD => {
                // DCL: Designate Command Line (select RAM bank).
                //
                // The lower 3 bits of A select the RAM bank (0-7, but only 0-3
                // are used since the 4004 has 4 RAM banks). We clamp to 0-3.
                let mut bank = (self.accumulator & 0x7) as usize;
                if bank > 3 {
                    bank &= 0x3;
                }
                self.ram_bank = bank;
                "DCL".to_string()
            }

            _ => format!("UNKNOWN(0x{raw:02X})"),
        }
    }

    /// Run a program to completion or until max_steps is reached.
    ///
    /// Resets all CPU state, loads the program, then executes step by step.
    /// Returns a trace of every instruction executed.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Vec<Intel4004Trace> {
        self.reset();
        self.load_program(program);
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted || self.pc >= self.memory.len() {
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
//
// These functions produce the raw bytes for each instruction, making it
// easy to assemble programs in tests without memorizing hex opcodes.

/// Encode `NOP` (no operation).
pub fn encode_nop() -> u8 {
    0x00
}

/// Encode `HLT` (halt — simulator only).
pub fn encode_hlt() -> u8 {
    0x01
}

/// Encode `LDM N` (load immediate N into accumulator).
pub fn encode_ldm(n: u8) -> u8 {
    (0xD << 4) | (n & 0xF)
}

/// Encode `LD Rn` (load register into accumulator).
pub fn encode_ld(r: u8) -> u8 {
    (0xA << 4) | (r & 0xF)
}

/// Encode `XCH Rn` (exchange accumulator with register n).
pub fn encode_xch(r: u8) -> u8 {
    (0xB << 4) | (r & 0xF)
}

/// Encode `ADD Rn` (add register n to accumulator with carry).
pub fn encode_add(r: u8) -> u8 {
    (0x8 << 4) | (r & 0xF)
}

/// Encode `SUB Rn` (subtract register n from accumulator with borrow).
pub fn encode_sub(r: u8) -> u8 {
    (0x9 << 4) | (r & 0xF)
}

/// Encode `INC Rn` (increment register n).
pub fn encode_inc(r: u8) -> u8 {
    (0x6 << 4) | (r & 0xF)
}

/// Encode `BBL N` (branch back and load N).
pub fn encode_bbl(n: u8) -> u8 {
    (0xC << 4) | (n & 0xF)
}

/// Encode `JCN cond, addr` (conditional jump — returns 2 bytes).
pub fn encode_jcn(cond: u8, addr: u8) -> [u8; 2] {
    [(0x1 << 4) | (cond & 0xF), addr]
}

/// Encode `FIM Pp, data` (fetch immediate to register pair — returns 2 bytes).
pub fn encode_fim(pair: u8, data: u8) -> [u8; 2] {
    [(0x2 << 4) | ((pair & 0x7) << 1), data]
}

/// Encode `SRC Pp` (send register control).
pub fn encode_src(pair: u8) -> u8 {
    (0x2 << 4) | ((pair & 0x7) << 1) | 0x1
}

/// Encode `FIN Pp` (fetch indirect from ROM).
pub fn encode_fin(pair: u8) -> u8 {
    (0x3 << 4) | ((pair & 0x7) << 1)
}

/// Encode `JIN Pp` (jump indirect).
pub fn encode_jin(pair: u8) -> u8 {
    (0x3 << 4) | ((pair & 0x7) << 1) | 0x1
}

/// Encode `JUN addr` (unconditional jump — returns 2 bytes).
pub fn encode_jun(addr: u16) -> [u8; 2] {
    [
        (0x4 << 4) | (((addr >> 8) & 0xF) as u8),
        (addr & 0xFF) as u8,
    ]
}

/// Encode `JMS addr` (jump to subroutine — returns 2 bytes).
pub fn encode_jms(addr: u16) -> [u8; 2] {
    [
        (0x5 << 4) | (((addr >> 8) & 0xF) as u8),
        (addr & 0xFF) as u8,
    ]
}

/// Encode `ISZ Rn, addr` (increment and skip if zero — returns 2 bytes).
pub fn encode_isz(r: u8, addr: u8) -> [u8; 2] {
    [(0x7 << 4) | (r & 0xF), addr]
}

// I/O instruction encoders — these are single-byte, full-byte opcodes.

/// Encode `WRM` (write accumulator to RAM main character).
pub fn encode_wrm() -> u8 { 0xE0 }
/// Encode `WMP` (write accumulator to RAM output port).
pub fn encode_wmp() -> u8 { 0xE1 }
/// Encode `WRR` (write accumulator to ROM I/O port).
pub fn encode_wrr() -> u8 { 0xE2 }
/// Encode `WPM` (write program RAM — NOP in simulator).
pub fn encode_wpm() -> u8 { 0xE3 }
/// Encode `WR0` (write RAM status character 0).
pub fn encode_wr0() -> u8 { 0xE4 }
/// Encode `WR1` (write RAM status character 1).
pub fn encode_wr1() -> u8 { 0xE5 }
/// Encode `WR2` (write RAM status character 2).
pub fn encode_wr2() -> u8 { 0xE6 }
/// Encode `WR3` (write RAM status character 3).
pub fn encode_wr3() -> u8 { 0xE7 }
/// Encode `SBM` (subtract RAM from accumulator).
pub fn encode_sbm() -> u8 { 0xE8 }
/// Encode `RDM` (read RAM main character).
pub fn encode_rdm() -> u8 { 0xE9 }
/// Encode `RDR` (read ROM I/O port).
pub fn encode_rdr() -> u8 { 0xEA }
/// Encode `ADM` (add RAM to accumulator).
pub fn encode_adm() -> u8 { 0xEB }
/// Encode `RD0` (read RAM status character 0).
pub fn encode_rd0() -> u8 { 0xEC }
/// Encode `RD1` (read RAM status character 1).
pub fn encode_rd1() -> u8 { 0xED }
/// Encode `RD2` (read RAM status character 2).
pub fn encode_rd2() -> u8 { 0xEE }
/// Encode `RD3` (read RAM status character 3).
pub fn encode_rd3() -> u8 { 0xEF }

// Accumulator instruction encoders — single-byte, full-byte opcodes.

/// Encode `CLB` (clear both A and carry).
pub fn encode_clb() -> u8 { 0xF0 }
/// Encode `CLC` (clear carry).
pub fn encode_clc() -> u8 { 0xF1 }
/// Encode `IAC` (increment accumulator).
pub fn encode_iac() -> u8 { 0xF2 }
/// Encode `CMC` (complement carry).
pub fn encode_cmc() -> u8 { 0xF3 }
/// Encode `CMA` (complement accumulator).
pub fn encode_cma() -> u8 { 0xF4 }
/// Encode `RAL` (rotate accumulator left through carry).
pub fn encode_ral() -> u8 { 0xF5 }
/// Encode `RAR` (rotate accumulator right through carry).
pub fn encode_rar() -> u8 { 0xF6 }
/// Encode `TCC` (transfer carry to accumulator).
pub fn encode_tcc() -> u8 { 0xF7 }
/// Encode `DAC` (decrement accumulator).
pub fn encode_dac() -> u8 { 0xF8 }
/// Encode `TCS` (transfer carry subtract).
pub fn encode_tcs() -> u8 { 0xF9 }
/// Encode `STC` (set carry).
pub fn encode_stc() -> u8 { 0xFA }
/// Encode `DAA` (decimal adjust accumulator).
pub fn encode_daa() -> u8 { 0xFB }
/// Encode `KBP` (keyboard process).
pub fn encode_kbp() -> u8 { 0xFC }
/// Encode `DCL` (designate command line / select RAM bank).
pub fn encode_dcl() -> u8 { 0xFD }

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===================================================================
    // Helper: run a program and return the simulator
    // ===================================================================

    fn run_program(program: &[u8]) -> Intel4004Simulator {
        let mut sim = Intel4004Simulator::new(4096);
        sim.run(program, 10000);
        sim
    }

    // ===================================================================
    // NOP
    // ===================================================================

    #[test]
    fn nop_does_nothing() {
        let sim = run_program(&[encode_nop(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }

    #[test]
    fn nop_mnemonic() {
        let mut sim = Intel4004Simulator::new(4096);
        let traces = sim.run(&[encode_nop(), encode_hlt()], 10);
        assert_eq!(traces[0].mnemonic, "NOP");
    }

    // ===================================================================
    // HLT
    // ===================================================================

    #[test]
    fn hlt_stops_execution() {
        let sim = run_program(&[encode_hlt(), encode_ldm(5)]);
        assert!(sim.halted);
        assert_eq!(sim.accumulator, 0); // LDM 5 never executed
    }

    #[test]
    #[should_panic(expected = "CPU is halted")]
    fn halted_step_panics() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.halted = true;
        sim.step();
    }

    // ===================================================================
    // LDM — Load Immediate
    // ===================================================================

    #[test]
    fn ldm_loads_value() {
        let sim = run_program(&[encode_ldm(7), encode_hlt()]);
        assert_eq!(sim.accumulator, 7);
    }

    #[test]
    fn ldm_all_values() {
        for n in 0..16u8 {
            let sim = run_program(&[encode_ldm(n), encode_hlt()]);
            assert_eq!(sim.accumulator, n, "LDM {n} should set A={n}");
        }
    }

    // ===================================================================
    // LD — Load Register
    // ===================================================================

    #[test]
    fn ld_loads_register() {
        let sim = run_program(&[
            encode_ldm(9),
            encode_xch(3), // R3 = 9
            encode_ldm(0), // A = 0
            encode_ld(3),  // A = R3 = 9
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 9);
    }

    // ===================================================================
    // XCH — Exchange
    // ===================================================================

    #[test]
    fn xch_swaps_values() {
        let sim = run_program(&[
            encode_ldm(5),
            encode_xch(0), // R0=5, A=0
            encode_ldm(3), // A=3
            encode_xch(0), // R0=3, A=5
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 5);
        assert_eq!(sim.registers[0], 3);
    }

    // ===================================================================
    // ADD — Add with carry
    // ===================================================================

    #[test]
    fn add_basic() {
        // 1 + 2 = 3 (the classic test from the original)
        let sim = run_program(&[
            encode_ldm(1),
            encode_xch(0),
            encode_ldm(2),
            encode_add(0),
            encode_xch(1),
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[1], 3);
    }

    #[test]
    fn add_with_carry_input() {
        // ADD includes the carry flag in the sum: A = A + Rn + carry
        let sim = run_program(&[
            encode_stc(),        // carry = 1
            encode_ldm(3),       // A = 3
            encode_xch(0),       // R0 = 3, A = 0
            encode_ldm(4),       // A = 4
            encode_add(0),       // A = 4 + 3 + 1(carry) = 8
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 8);
        assert!(!sim.carry);
    }

    #[test]
    fn add_overflow_sets_carry() {
        let sim = run_program(&[
            encode_ldm(15),
            encode_xch(0),   // R0 = 15
            encode_ldm(15),  // A = 15
            encode_clc(),    // clear carry
            encode_add(0),   // A = 15 + 15 + 0 = 30, carry=true, A=14
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 14);
        assert!(sim.carry);
    }

    // ===================================================================
    // SUB — Subtract with borrow (complement-add)
    // ===================================================================

    #[test]
    fn sub_no_borrow() {
        // 5 - 3 = 2, carry=true (no borrow)
        // Using complement-add: A + (~3 & 0xF) + 1 = 5 + 12 + 1 = 18
        // 18 > 15 → carry=true, A = 18 & 0xF = 2
        let sim = run_program(&[
            encode_ldm(3),
            encode_xch(0),  // R0 = 3
            encode_ldm(5),  // A = 5
            encode_clc(),   // carry = false (meaning "no previous borrow"; borrow_in = 1)
            encode_sub(0),  // A = 5 + ~3 + 1 = 5 + 12 + 1 = 18 → A=2, carry=true
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 2);
        assert!(sim.carry, "carry=true means no borrow");
    }

    #[test]
    fn sub_with_borrow() {
        // 0 - 1 with no previous borrow (carry=false → borrow_in=1)
        // A + (~1 & 0xF) + 1 = 0 + 14 + 1 = 15
        // 15 <= 15 → carry=false (borrow), A=15
        let sim = run_program(&[
            encode_ldm(1),
            encode_xch(0),  // R0 = 1
            encode_ldm(0),  // A = 0
            encode_clc(),   // carry = false
            encode_sub(0),  // A = 0 + 14 + 1 = 15, carry=false (borrow)
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 15);
        assert!(!sim.carry, "carry=false means borrow occurred");
    }

    // ===================================================================
    // INC — Increment register
    // ===================================================================

    #[test]
    fn inc_register() {
        let sim = run_program(&[
            encode_ldm(7),
            encode_xch(2), // R2 = 7
            encode_inc(2), // R2 = 8
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[2], 8);
    }

    #[test]
    fn inc_wraps_at_15() {
        let sim = run_program(&[
            encode_ldm(15),
            encode_xch(0),  // R0 = 15
            encode_inc(0),  // R0 = 0 (wraps)
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[0], 0);
        // INC does NOT affect carry
        assert!(!sim.carry);
    }

    // ===================================================================
    // JUN — Unconditional jump
    // ===================================================================

    #[test]
    fn jun_jumps() {
        // JUN to address 6, skipping the LDM 1 at address 2
        let jun = encode_jun(0x006);
        let program = vec![
            jun[0], jun[1],    // 0-1: JUN 0x006
            encode_ldm(1),     // 2: (skipped)
            encode_hlt(),      // 3: (skipped)
            0x00, 0x00,        // 4-5: padding
            encode_ldm(9),     // 6: A = 9
            encode_hlt(),      // 7: halt
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 9);
    }

    // ===================================================================
    // JMS / BBL — Subroutine call and return
    // ===================================================================

    #[test]
    fn jms_bbl_subroutine() {
        // Main: JMS 0x006, HLT
        // Sub at 0x006: BBL 5
        let jms = encode_jms(0x006);
        let program = vec![
            jms[0], jms[1],    // 0-1: JMS 0x006
            encode_hlt(),      // 2: halt (after return)
            0x00, 0x00, 0x00,  // 3-5: padding
            encode_bbl(5),     // 6: BBL 5 (return, A=5)
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 5);
        assert!(sim.halted);
    }

    #[test]
    fn nested_subroutines() {
        // Main calls sub1, sub1 calls sub2, sub2 returns 3, sub1 returns
        let jms1 = encode_jms(0x004);
        let jms2 = encode_jms(0x007);
        let program = vec![
            jms1[0], jms1[1],  // 0-1: JMS 0x004
            encode_hlt(),      // 2: halt
            0x00,              // 3: padding
            jms2[0], jms2[1],  // 4-5: JMS 0x007
            encode_bbl(0),     // 6: BBL 0 (return from sub1)
            encode_bbl(3),     // 7: BBL 3 (return from sub2, A=3)
        ];
        let sim = run_program(&program);
        // sub2 sets A=3, then sub1 sets A=0 via BBL 0
        assert_eq!(sim.accumulator, 0);
    }

    // ===================================================================
    // JCN — Conditional jump
    // ===================================================================

    #[test]
    fn jcn_test_zero_taken() {
        // JCN 4 (test A==0), A is 0, so jump should be taken
        let jcn = encode_jcn(0x4, 0x04);
        let program = vec![
            jcn[0], jcn[1],    // 0-1: JCN 4, 0x04
            encode_ldm(1),     // 2: skipped
            encode_hlt(),      // 3: skipped
            encode_ldm(9),     // 4: A = 9
            encode_hlt(),      // 5: halt
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 9);
    }

    #[test]
    fn jcn_test_zero_not_taken() {
        // JCN 4 (test A==0), but A is 5, so jump not taken
        let jcn = encode_jcn(0x4, 0x06);
        let program = vec![
            encode_ldm(5),     // 0: A = 5
            jcn[0], jcn[1],    // 1-2: JCN 4, 0x06 (not taken, A != 0)
            encode_ldm(1),     // 3: A = 1 (executed)
            encode_hlt(),      // 4: halt
            0x00,              // 5: padding
            encode_ldm(9),     // 6: not reached
            encode_hlt(),      // 7: not reached
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 1);
    }

    #[test]
    fn jcn_test_carry() {
        // JCN 2 (test carry==1)
        let jcn = encode_jcn(0x2, 0x05);
        let program = vec![
            encode_stc(),      // 0: set carry
            jcn[0], jcn[1],    // 1-2: JCN 2, 0x05 (taken, carry is set)
            encode_ldm(1),     // 3: skipped
            encode_hlt(),      // 4: skipped
            encode_ldm(7),     // 5: A = 7
            encode_hlt(),      // 6: halt
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 7);
    }

    #[test]
    fn jcn_invert() {
        // JCN 0xC (invert | test_zero): jump if A != 0
        let jcn = encode_jcn(0xC, 0x05);
        let program = vec![
            encode_ldm(3),     // 0: A = 3 (not zero)
            jcn[0], jcn[1],    // 1-2: JCN 0xC, 0x05 (invert test_zero: jump if A!=0)
            encode_ldm(1),     // 3: skipped
            encode_hlt(),      // 4: skipped
            encode_ldm(8),     // 5: A = 8
            encode_hlt(),      // 6: halt
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 8);
    }

    // ===================================================================
    // FIM — Fetch Immediate to register pair
    // ===================================================================

    #[test]
    fn fim_loads_pair() {
        let fim = encode_fim(0, 0xAB);
        let program = vec![
            fim[0], fim[1],  // FIM P0, 0xAB → R0=0xA, R1=0xB
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.registers[0], 0xA);
        assert_eq!(sim.registers[1], 0xB);
    }

    // ===================================================================
    // SRC — Send Register Control
    // ===================================================================

    #[test]
    fn src_sets_ram_address() {
        let fim = encode_fim(1, 0x25); // P1 = 0x25 → R2=2, R3=5
        let program = vec![
            fim[0], fim[1],
            encode_src(1),  // SRC P1 → ram_register=2, ram_character=5
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.ram_register, 2);
        assert_eq!(sim.ram_character, 5);
    }

    // ===================================================================
    // FIN — Fetch Indirect from ROM
    // ===================================================================

    #[test]
    fn fin_fetches_from_rom() {
        // Put data at ROM address 0x0A (within page 0)
        // Set P0 (R0:R1) = 0x0A, then FIN P1 reads ROM[0x0A] into P1
        let fim = encode_fim(0, 0x0A); // P0 = 0x0A
        let mut program = vec![
            fim[0], fim[1],   // 0-1: FIM P0, 0x0A
            encode_fin(1),    // 2: FIN P1 → read ROM[page|0x0A]
            encode_hlt(),     // 3: halt
        ];
        // Pad to address 0x0A and put data there
        while program.len() < 0x0A {
            program.push(0x00);
        }
        program.push(0x37); // ROM[0x0A] = 0x37

        let sim = run_program(&program);
        assert_eq!(sim.registers[2], 0x3); // P1 high = R2
        assert_eq!(sim.registers[3], 0x7); // P1 low = R3
    }

    // ===================================================================
    // JIN — Jump Indirect
    // ===================================================================

    #[test]
    fn jin_jumps_indirect() {
        let fim = encode_fim(1, 0x08); // P1 = 0x08
        let program = vec![
            fim[0], fim[1],   // 0-1: FIM P1, 0x08
            encode_jin(1),    // 2: JIN P1 → jump to page|0x08
            encode_ldm(1),    // 3: skipped
            encode_hlt(),     // 4: skipped
            0x00, 0x00, 0x00, // 5-7: padding
            encode_ldm(6),    // 8: A = 6
            encode_hlt(),     // 9: halt
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 6);
    }

    // ===================================================================
    // ISZ — Increment and Skip if Zero
    // ===================================================================

    #[test]
    fn isz_loops_until_zero() {
        // Loop: use ISZ to count R0 from 14 to 0 (2 iterations)
        let isz = encode_isz(0, 0x03); // ISZ R0, jump to addr 0x03
        let program = vec![
            encode_ldm(14),    // 0: A = 14
            encode_xch(0),     // 1: R0 = 14
            encode_ldm(0),     // 2: A = 0
            isz[0], isz[1],   // 3-4: ISZ R0, 0x03 (loop back to addr 3)
            encode_hlt(),      // 5: halt (when R0 wraps to 0)
        ];
        let sim = run_program(&program);
        // R0 went 14→15→0, ISZ looped once (15!=0), then second time 0==0, fell through
        assert_eq!(sim.registers[0], 0);
    }

    #[test]
    fn isz_falls_through_on_zero() {
        // R0 = 15, ISZ increments to 0, falls through
        let isz = encode_isz(0, 0x00);
        let program = vec![
            encode_ldm(15),
            encode_xch(0),     // R0 = 15
            isz[0], isz[1],   // ISZ R0, 0x00
            encode_ldm(7),     // Falls through, A = 7
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.registers[0], 0);
        assert_eq!(sim.accumulator, 7);
    }

    // ===================================================================
    // CLB — Clear Both
    // ===================================================================

    #[test]
    fn clb_clears_both() {
        let sim = run_program(&[
            encode_ldm(15),
            encode_stc(),
            encode_clb(),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }

    // ===================================================================
    // CLC — Clear Carry
    // ===================================================================

    #[test]
    fn clc_clears_carry() {
        let sim = run_program(&[encode_stc(), encode_clc(), encode_hlt()]);
        assert!(!sim.carry);
    }

    // ===================================================================
    // IAC — Increment Accumulator
    // ===================================================================

    #[test]
    fn iac_increments() {
        let sim = run_program(&[encode_ldm(5), encode_iac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 6);
        assert!(!sim.carry);
    }

    #[test]
    fn iac_wraps_and_sets_carry() {
        let sim = run_program(&[encode_ldm(15), encode_iac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(sim.carry);
    }

    // ===================================================================
    // CMC — Complement Carry
    // ===================================================================

    #[test]
    fn cmc_toggles_carry() {
        let sim = run_program(&[encode_cmc(), encode_hlt()]); // false → true
        assert!(sim.carry);

        let sim = run_program(&[encode_stc(), encode_cmc(), encode_hlt()]); // true → false
        assert!(!sim.carry);
    }

    // ===================================================================
    // CMA — Complement Accumulator
    // ===================================================================

    #[test]
    fn cma_complements() {
        let sim = run_program(&[encode_ldm(0b0101), encode_cma(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b1010);

        let sim = run_program(&[encode_ldm(0), encode_cma(), encode_hlt()]);
        assert_eq!(sim.accumulator, 15);

        let sim = run_program(&[encode_ldm(15), encode_cma(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
    }

    // ===================================================================
    // RAL — Rotate Left through carry
    // ===================================================================

    #[test]
    fn ral_rotates_left() {
        // A=0b0101=5, carry=0
        // After: carry=0 (bit3 was 0), A=0b1010=10
        let sim = run_program(&[encode_ldm(0b0101), encode_clc(), encode_ral(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b1010);
        assert!(!sim.carry);
    }

    #[test]
    fn ral_with_carry() {
        // A=0b1001=9, carry=1
        // After: carry=1 (bit3 was 1), A=0b0011=3
        let sim = run_program(&[encode_ldm(0b1001), encode_stc(), encode_ral(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b0011);
        assert!(sim.carry);
    }

    // ===================================================================
    // RAR — Rotate Right through carry
    // ===================================================================

    #[test]
    fn rar_rotates_right() {
        // A=0b1010=10, carry=0
        // After: carry=0 (bit0 was 0), A=0b0101=5
        let sim = run_program(&[encode_ldm(0b1010), encode_clc(), encode_rar(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b0101);
        assert!(!sim.carry);
    }

    #[test]
    fn rar_with_carry() {
        // A=0b0110=6, carry=1
        // After: carry=0 (bit0 was 0), A=0b1011=11
        let sim = run_program(&[encode_ldm(0b0110), encode_stc(), encode_rar(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b1011);
        assert!(!sim.carry);
    }

    #[test]
    fn rar_bit0_to_carry() {
        // A=0b0011=3, carry=0
        // After: carry=1 (bit0 was 1), A=0b0001=1
        let sim = run_program(&[encode_ldm(0b0011), encode_clc(), encode_rar(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b0001);
        assert!(sim.carry);
    }

    // ===================================================================
    // TCC — Transfer Carry to Accumulator
    // ===================================================================

    #[test]
    fn tcc_with_carry_set() {
        let sim = run_program(&[encode_stc(), encode_tcc(), encode_hlt()]);
        assert_eq!(sim.accumulator, 1);
        assert!(!sim.carry); // carry cleared after transfer
    }

    #[test]
    fn tcc_with_carry_clear() {
        let sim = run_program(&[encode_clc(), encode_tcc(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }

    // ===================================================================
    // DAC — Decrement Accumulator
    // ===================================================================

    #[test]
    fn dac_decrements() {
        let sim = run_program(&[encode_ldm(5), encode_dac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 4);
        assert!(sim.carry, "carry=true when no borrow");
    }

    #[test]
    fn dac_borrow_at_zero() {
        let sim = run_program(&[encode_ldm(0), encode_dac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 15);
        assert!(!sim.carry, "carry=false when borrow");
    }

    // ===================================================================
    // TCS — Transfer Carry Subtract
    // ===================================================================

    #[test]
    fn tcs_with_carry() {
        let sim = run_program(&[encode_stc(), encode_tcs(), encode_hlt()]);
        assert_eq!(sim.accumulator, 10);
        assert!(!sim.carry);
    }

    #[test]
    fn tcs_without_carry() {
        let sim = run_program(&[encode_clc(), encode_tcs(), encode_hlt()]);
        assert_eq!(sim.accumulator, 9);
        assert!(!sim.carry);
    }

    // ===================================================================
    // STC — Set Carry
    // ===================================================================

    #[test]
    fn stc_sets_carry() {
        let sim = run_program(&[encode_stc(), encode_hlt()]);
        assert!(sim.carry);
    }

    // ===================================================================
    // DAA — Decimal Adjust Accumulator
    // ===================================================================

    #[test]
    fn daa_adjusts_when_above_9() {
        // A=12, DAA adds 6: 12+6=18, A=2, carry=true
        let sim = run_program(&[encode_ldm(12), encode_clc(), encode_daa(), encode_hlt()]);
        assert_eq!(sim.accumulator, 2);
        assert!(sim.carry);
    }

    #[test]
    fn daa_no_adjust_when_9_or_below() {
        let sim = run_program(&[encode_ldm(9), encode_clc(), encode_daa(), encode_hlt()]);
        assert_eq!(sim.accumulator, 9);
        assert!(!sim.carry);
    }

    #[test]
    fn daa_adjusts_when_carry_set() {
        // Even if A <= 9, if carry is set, add 6
        let sim = run_program(&[encode_ldm(3), encode_stc(), encode_daa(), encode_hlt()]);
        assert_eq!(sim.accumulator, 9);
        // 3+6=9, no overflow, but carry was already set and DAA only sets carry on overflow
        // The Python reference doesn't clear carry if no overflow — carry stays as-is
        // unless the addition overflows.
        assert!(sim.carry); // carry was set, and 3+6=9 doesn't overflow, carry remains
    }

    // ===================================================================
    // KBP — Keyboard Process
    // ===================================================================

    #[test]
    fn kbp_conversions() {
        let cases = [(0, 0), (1, 1), (2, 2), (4, 3), (8, 4), (3, 15), (5, 15), (15, 15)];
        for (input, expected) in cases {
            let sim = run_program(&[encode_ldm(input), encode_kbp(), encode_hlt()]);
            assert_eq!(sim.accumulator, expected, "KBP({input}) should be {expected}");
        }
    }

    // ===================================================================
    // DCL — Designate Command Line
    // ===================================================================

    #[test]
    fn dcl_selects_bank() {
        let sim = run_program(&[encode_ldm(2), encode_dcl(), encode_hlt()]);
        assert_eq!(sim.ram_bank, 2);
    }

    #[test]
    fn dcl_clamps_to_valid_range() {
        // Bank 5 (0b101) → clamped to 1 (5 & 3 = 1)
        let sim = run_program(&[encode_ldm(5), encode_dcl(), encode_hlt()]);
        assert_eq!(sim.ram_bank, 1);
    }

    // ===================================================================
    // WRM / RDM — Write/Read RAM main character
    // ===================================================================

    #[test]
    fn wrm_rdm_roundtrip() {
        let fim = encode_fim(0, 0x00); // P0 → ram_register=0, ram_character=0
        let program = vec![
            fim[0], fim[1],
            encode_src(0),     // SRC P0
            encode_ldm(11),    // A = 11
            encode_wrm(),      // Write A to RAM[0][0][0]
            encode_ldm(0),     // A = 0
            encode_rdm(),      // Read RAM[0][0][0] into A
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 11);
    }

    // ===================================================================
    // WMP — Write RAM output port
    // ===================================================================

    #[test]
    fn wmp_writes_output() {
        let sim = run_program(&[encode_ldm(7), encode_wmp(), encode_hlt()]);
        assert_eq!(sim.ram_output[0], 7); // bank 0 by default
    }

    // ===================================================================
    // WRR / RDR — Write/Read ROM I/O port
    // ===================================================================

    #[test]
    fn wrr_rdr_roundtrip() {
        let sim = run_program(&[
            encode_ldm(13),
            encode_wrr(),      // ROM port = 13
            encode_ldm(0),     // A = 0
            encode_rdr(),      // A = ROM port = 13
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 13);
    }

    // ===================================================================
    // WPM — Write Program Memory (NOP)
    // ===================================================================

    #[test]
    fn wpm_is_nop() {
        let sim = run_program(&[encode_ldm(5), encode_wpm(), encode_hlt()]);
        assert_eq!(sim.accumulator, 5); // A unchanged
    }

    // ===================================================================
    // WR0-WR3 / RD0-RD3 — Write/Read RAM status characters
    // ===================================================================

    #[test]
    fn wr0_rd0_roundtrip() {
        let fim = encode_fim(0, 0x00);
        let program = vec![
            fim[0], fim[1],
            encode_src(0),
            encode_ldm(4),
            encode_wr0(),
            encode_ldm(0),
            encode_rd0(),
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 4);
    }

    #[test]
    fn wr1_rd1_roundtrip() {
        let fim = encode_fim(0, 0x00);
        let program = vec![
            fim[0], fim[1], encode_src(0),
            encode_ldm(8), encode_wr1(), encode_ldm(0), encode_rd1(), encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 8);
    }

    #[test]
    fn wr2_rd2_roundtrip() {
        let fim = encode_fim(0, 0x00);
        let program = vec![
            fim[0], fim[1], encode_src(0),
            encode_ldm(3), encode_wr2(), encode_ldm(0), encode_rd2(), encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 3);
    }

    #[test]
    fn wr3_rd3_roundtrip() {
        let fim = encode_fim(0, 0x00);
        let program = vec![
            fim[0], fim[1], encode_src(0),
            encode_ldm(15), encode_wr3(), encode_ldm(0), encode_rd3(), encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 15);
    }

    // ===================================================================
    // SBM — Subtract RAM from accumulator
    // ===================================================================

    #[test]
    fn sbm_subtracts_ram() {
        let fim = encode_fim(0, 0x00);
        let program = vec![
            fim[0], fim[1],
            encode_src(0),
            encode_ldm(3),
            encode_wrm(),      // RAM[0][0][0] = 3
            encode_ldm(7),     // A = 7
            encode_clc(),      // carry = false (borrow_in = 1)
            encode_sbm(),      // A = 7 + ~3 + 1 = 7 + 12 + 1 = 20 → A=4, carry=true
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 4);
        assert!(sim.carry);
    }

    // ===================================================================
    // ADM — Add RAM to accumulator
    // ===================================================================

    #[test]
    fn adm_adds_ram() {
        let fim = encode_fim(0, 0x00);
        let program = vec![
            fim[0], fim[1],
            encode_src(0),
            encode_ldm(5),
            encode_wrm(),      // RAM[0][0][0] = 5
            encode_ldm(3),     // A = 3
            encode_clc(),      // carry = false
            encode_adm(),      // A = 3 + 5 + 0 = 8
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 8);
        assert!(!sim.carry);
    }

    // ===================================================================
    // Stack wraps at 3 levels
    // ===================================================================

    #[test]
    fn stack_wraps_mod_3() {
        // Push 3 addresses, then pop — should get LIFO order with mod-3 wrap
        let mut sim = Intel4004Simulator::new(4096);
        sim.stack_push(0x100);
        sim.stack_push(0x200);
        sim.stack_push(0x300);
        // Stack: [0x100, 0x200, 0x300], sp=0
        // Pop should give 0x300, 0x200, 0x100
        assert_eq!(sim.stack_pop(), 0x300);
        assert_eq!(sim.stack_pop(), 0x200);
        assert_eq!(sim.stack_pop(), 0x100);
    }

    #[test]
    fn stack_overflow_wraps() {
        // Push 4 addresses — the 4th overwrites the 1st
        let mut sim = Intel4004Simulator::new(4096);
        sim.stack_push(0x100);
        sim.stack_push(0x200);
        sim.stack_push(0x300);
        sim.stack_push(0x400); // Overwrites 0x100
        assert_eq!(sim.stack_pop(), 0x400);
        assert_eq!(sim.stack_pop(), 0x300);
        assert_eq!(sim.stack_pop(), 0x200);
    }

    // ===================================================================
    // Encoding helpers — verify they produce correct bytes
    // ===================================================================

    #[test]
    fn encoding_sanity() {
        assert_eq!(encode_nop(), 0x00);
        assert_eq!(encode_hlt(), 0x01);
        assert_eq!(encode_ldm(5), 0xD5);
        assert_eq!(encode_ld(3), 0xA3);
        assert_eq!(encode_xch(7), 0xB7);
        assert_eq!(encode_add(0), 0x80);
        assert_eq!(encode_sub(1), 0x91);
        assert_eq!(encode_inc(4), 0x64);
        assert_eq!(encode_bbl(0), 0xC0);
        assert_eq!(encode_jun(0x123), [0x41, 0x23]);
        assert_eq!(encode_jms(0x456), [0x54, 0x56]);
        assert_eq!(encode_jcn(0x4, 0xAA), [0x14, 0xAA]);
        assert_eq!(encode_fim(0, 0xFF), [0x20, 0xFF]);
        assert_eq!(encode_src(1), 0x23);
        assert_eq!(encode_fin(2), 0x34);
        assert_eq!(encode_jin(3), 0x37);
        assert_eq!(encode_isz(5, 0x10), [0x75, 0x10]);
    }

    // ===================================================================
    // Two-byte detection
    // ===================================================================

    #[test]
    fn two_byte_detection() {
        // 2-byte instructions
        assert!(is_two_byte(0x10)); // JCN
        assert!(is_two_byte(0x1F)); // JCN
        assert!(is_two_byte(0x20)); // FIM (even)
        assert!(is_two_byte(0x40)); // JUN
        assert!(is_two_byte(0x50)); // JMS
        assert!(is_two_byte(0x70)); // ISZ

        // 1-byte instructions
        assert!(!is_two_byte(0x00)); // NOP
        assert!(!is_two_byte(0x01)); // HLT
        assert!(!is_two_byte(0x21)); // SRC (odd)
        assert!(!is_two_byte(0x30)); // FIN
        assert!(!is_two_byte(0x31)); // JIN
        assert!(!is_two_byte(0x60)); // INC
        assert!(!is_two_byte(0x80)); // ADD
        assert!(!is_two_byte(0xD0)); // LDM
        assert!(!is_two_byte(0xE0)); // WRM
        assert!(!is_two_byte(0xF0)); // CLB
    }

    // ===================================================================
    // Trace recording
    // ===================================================================

    #[test]
    fn trace_records_before_after() {
        let mut sim = Intel4004Simulator::new(4096);
        let traces = sim.run(
            &[encode_ldm(0), encode_ldm(5), encode_hlt()],
            10,
        );
        assert_eq!(traces[1].accumulator_before, 0);
        assert_eq!(traces[1].accumulator_after, 5);
        assert_eq!(traces[1].mnemonic, "LDM 5");
    }

    #[test]
    fn trace_two_byte_instruction() {
        let jun = encode_jun(0x004);
        let mut sim = Intel4004Simulator::new(4096);
        let program = vec![jun[0], jun[1], 0x00, 0x00, encode_hlt()];
        let traces = sim.run(&program, 10);
        assert_eq!(traces[0].raw, jun[0]);
        assert_eq!(traces[0].raw2, Some(jun[1]));
        assert_eq!(traces[0].mnemonic, "JUN 0x004");
    }

    // ===================================================================
    // Integration: compute 3 + 4 = 7 via subroutine
    // ===================================================================

    #[test]
    fn integration_add_via_subroutine() {
        // Main: load 3 and 4 into R0 and R1, call add_subroutine, check R2
        // add_subroutine at 0x10: LD R0, ADD R1, XCH R2, BBL 0
        let jms = encode_jms(0x010);
        let mut program = vec![
            encode_ldm(3),     // 0: A = 3
            encode_xch(0),     // 1: R0 = 3
            encode_ldm(4),     // 2: A = 4
            encode_xch(1),     // 3: R1 = 4
            encode_clc(),      // 4: clear carry before add
            jms[0], jms[1],    // 5-6: JMS 0x010
            encode_hlt(),      // 7: halt
        ];
        // Pad to 0x10
        while program.len() < 0x10 {
            program.push(0x00);
        }
        // Subroutine: add R0 + R1, store in R2
        program.push(encode_ld(0));     // 0x10: A = R0
        program.push(encode_add(1));    // 0x11: A = A + R1
        program.push(encode_xch(2));    // 0x12: R2 = A
        program.push(encode_bbl(0));    // 0x13: return

        let sim = run_program(&program);
        assert_eq!(sim.registers[2], 7);
    }

    // ===================================================================
    // Integration: count down using ISZ loop
    // ===================================================================

    #[test]
    fn integration_isz_countdown() {
        // Use ISZ to loop 4 times (R0 starts at 12 = -4 in 4-bit)
        // Each iteration increments R1
        let isz = encode_isz(0, 0x04);
        let program = vec![
            encode_ldm(12),    // 0: R0 = 12 (-4 in 4-bit)
            encode_xch(0),     // 1: R0 = 12
            encode_ldm(0),     // 2: A = 0 (clear for loop body)
            encode_xch(1),     // 3: R1 = 0 (loop counter)
            encode_inc(1),     // 4: R1++ (loop body)
            isz[0], isz[1],   // 5-6: ISZ R0, 0x04 (jump to 4 if R0 != 0)
            encode_hlt(),      // 7: halt
        ];
        let sim = run_program(&program);
        assert_eq!(sim.registers[0], 0);  // R0 wrapped to 0
        assert_eq!(sim.registers[1], 4);  // R1 counted 4 iterations
    }

    // ===================================================================
    // Integration: RAM bank switching
    // ===================================================================

    #[test]
    fn integration_ram_banks() {
        let fim = encode_fim(0, 0x00); // P0 = 0x00 → reg=0, char=0
        let program = vec![
            fim[0], fim[1],
            encode_src(0),
            // Write 5 to bank 0
            encode_ldm(5),
            encode_wrm(),
            // Switch to bank 1
            encode_ldm(1),
            encode_dcl(),
            // Write 9 to bank 1
            encode_ldm(9),
            encode_wrm(),
            // Read from bank 1
            encode_ldm(0),
            encode_rdm(),
            encode_xch(0),     // R0 = value from bank 1
            // Switch back to bank 0
            encode_ldm(0),
            encode_dcl(),
            // Read from bank 0
            encode_rdm(),
            encode_xch(1),     // R1 = value from bank 0
            encode_hlt(),
        ];
        let sim = run_program(&program);
        assert_eq!(sim.registers[0], 9);  // from bank 1
        assert_eq!(sim.registers[1], 5);  // from bank 0
    }

    // ===================================================================
    // Unknown instruction
    // ===================================================================

    #[test]
    fn unknown_instruction_produces_mnemonic() {
        let mut sim = Intel4004Simulator::new(4096);
        let traces = sim.run(&[0xFE, encode_hlt()], 10);
        assert!(traces[0].mnemonic.contains("UNKNOWN"));
    }

    // ===================================================================
    // BCD addition using DAA
    // ===================================================================

    #[test]
    fn integration_bcd_addition() {
        // BCD: 7 + 8 = 15 decimal
        // Binary: 7 + 8 = 15. DAA: 15 > 9, so 15 + 6 = 21, A = 5, carry = 1
        // Result: carry=1, A=5 → BCD "15"
        let sim = run_program(&[
            encode_ldm(8),
            encode_xch(0),     // R0 = 8
            encode_ldm(7),     // A = 7
            encode_clc(),
            encode_add(0),     // A = 7 + 8 = 15, carry=false (15 <= 15)
            encode_daa(),      // A > 9, so A = 15+6=21, A=5, carry=true
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 5);
        assert!(sim.carry); // represents the "1" in "15"
    }

    // ===================================================================
    // Register pair read/write
    // ===================================================================

    #[test]
    fn pair_read_write() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.write_pair(3, 0xAB);
        assert_eq!(sim.registers[6], 0xA);
        assert_eq!(sim.registers[7], 0xB);
        assert_eq!(sim.read_pair(3), 0xAB);
    }

    // ===================================================================
    // Reset clears everything
    // ===================================================================

    #[test]
    fn reset_clears_state() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.accumulator = 5;
        sim.carry = true;
        sim.registers[0] = 7;
        sim.ram_bank = 2;
        sim.halted = true;
        sim.stack_push(0x100);

        sim.reset();

        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
        assert_eq!(sim.registers[0], 0);
        assert_eq!(sim.ram_bank, 0);
        assert!(!sim.halted);
        assert_eq!(sim.hw_stack, [0, 0, 0]);
    }
}
