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
//! The 4004 uses both 1-byte and 2-byte instructions:
//!
//! ```text
//!     1-byte:          2-byte:
//!     7    4  3    0   7    4  3    0   7          0
//!     +------+------+  +------+------+  +----------+
//!     |opcode|operand|  |opcode|operand|  | data/addr|
//!     |4 bits|4 bits |  |4 bits|4 bits |  |  8 bits  |
//!     +------+------+  +------+------+  +----------+
//! ```
//!
//! 2-byte instructions: JCN, FIM, JUN, JMS, ISZ, and even-addressed SRC variants.
//!
//! ## Memory architecture
//!
//! The 4004 has a rich memory hierarchy:
//!
//! - **ROM**: Up to 4096 bytes of program memory (12-bit address space)
//! - **RAM**: 4 banks x 4 registers x 16 characters (nibbles) of data memory
//! - **RAM status**: 4 banks x 4 registers x 4 status nibbles
//! - **Hardware stack**: 3-level deep for subroutine return addresses
//!
//! ## All 46 instructions
//!
//! | Byte     | Mnemonic | Description                                   |
//! |----------|----------|-----------------------------------------------|
//! | 0x00     | NOP      | No operation                                  |
//! | 0x01     | HLT      | Halt (custom for testing, not original 4004)  |
//! | 0x1C_    | JCN      | Jump conditional (2-byte)                     |
//! | 0x2E_    | FIM      | Fetch immediate to register pair (2-byte)     |
//! | 0x2O_    | SRC      | Send register control (set RAM address)       |
//! | 0x3E_    | FIN      | Fetch indirect from ROM to register pair      |
//! | 0x3O_    | JIN      | Jump indirect through register pair           |
//! | 0x4__    | JUN      | Jump unconditional (2-byte, 12-bit address)   |
//! | 0x5__    | JMS      | Jump to subroutine (2-byte, push return addr) |
//! | 0x6_     | INC      | Increment register                            |
//! | 0x7__    | ISZ      | Increment and skip if zero (2-byte)           |
//! | 0x8_     | ADD      | Add register to accumulator with carry         |
//! | 0x9_     | SUB      | Subtract register from accumulator (compl.)   |
//! | 0xA_     | LD       | Load register into accumulator                |
//! | 0xB_     | XCH      | Exchange accumulator with register             |
//! | 0xC_     | BBL      | Branch back and load (return from subroutine) |
//! | 0xD_     | LDM      | Load immediate into accumulator               |
//! | 0xE0     | WRM      | Write accumulator to RAM main memory          |
//! | 0xE1     | WMP      | Write accumulator to RAM output port          |
//! | 0xE2     | WRR      | Write accumulator to ROM port                 |
//! | 0xE3     | WPM      | Write program memory (not impl in sim)        |
//! | 0xE4     | WR0      | Write accumulator to RAM status char 0        |
//! | 0xE5     | WR1      | Write accumulator to RAM status char 1        |
//! | 0xE6     | WR2      | Write accumulator to RAM status char 2        |
//! | 0xE7     | WR3      | Write accumulator to RAM status char 3        |
//! | 0xE8     | SBM      | Subtract RAM main memory from accumulator     |
//! | 0xE9     | RDM      | Read RAM main memory into accumulator         |
//! | 0xEA     | RDR      | Read ROM port into accumulator                |
//! | 0xEB     | ADM      | Add RAM main memory to accumulator            |
//! | 0xEC     | RD0      | Read RAM status char 0 into accumulator       |
//! | 0xED     | RD1      | Read RAM status char 1 into accumulator       |
//! | 0xEE     | RD2      | Read RAM status char 2 into accumulator       |
//! | 0xEF     | RD3      | Read RAM status char 3 into accumulator       |
//! | 0xF0     | CLB      | Clear both accumulator and carry              |
//! | 0xF1     | CLC      | Clear carry                                   |
//! | 0xF2     | IAC      | Increment accumulator                         |
//! | 0xF3     | CMC      | Complement carry                              |
//! | 0xF4     | CMA      | Complement accumulator                        |
//! | 0xF5     | RAL      | Rotate accumulator left through carry         |
//! | 0xF6     | RAR      | Rotate accumulator right through carry        |
//! | 0xF7     | TCC      | Transfer carry to accumulator, clear carry    |
//! | 0xF8     | DAC      | Decrement accumulator                         |
//! | 0xF9     | TCS      | Transfer carry subtract, clear carry          |
//! | 0xFA     | STC      | Set carry                                     |
//! | 0xFB     | DAA      | Decimal adjust accumulator                    |
//! | 0xFC     | KBP      | Keyboard process                              |
//! | 0xFD     | DCL      | Designate command line (select RAM bank)      |

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
    pub address: usize,
    pub raw: u8,
    pub raw2: Option<u8>,
    pub mnemonic: String,
    pub accumulator_before: u8,
    pub accumulator_after: u8,
    pub carry_before: bool,
    pub carry_after: bool,
}

// ===========================================================================
// Two-byte instruction detection
// ===========================================================================

/// Determine whether a given first byte indicates a 2-byte instruction.
///
/// The 4004 has six 2-byte instruction types:
///
/// ```text
///   Upper nibble 0x1 → JCN (jump conditional)
///   Upper nibble 0x2, bit0=0 → FIM (fetch immediate to register pair)
///   Upper nibble 0x4 → JUN (jump unconditional)
///   Upper nibble 0x5 → JMS (jump to subroutine)
///   Upper nibble 0x7 → ISZ (increment and skip if zero)
/// ```
///
/// All other instructions are single-byte.
fn is_two_byte(raw: u8) -> bool {
    let upper = (raw >> 4) & 0xF;
    matches!(upper, 0x1 | 0x4 | 0x5 | 0x7) || (upper == 0x2 && (raw & 0x1) == 0)
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
///     +-------------------------------------------+
///     |           Intel 4004                       |
///     |  Accumulator:     4 bits                   |
///     |  Registers:       16 x 4 bits              |
///     |  Carry:           1 bit                    |
///     |  Program memory:  byte-addressable ROM     |
///     |  Data RAM:        4x4x16 nibbles           |
///     |  RAM status:      4x4x4 nibbles            |
///     |  Hardware stack:  3-level x 12-bit addrs   |
///     |  ROM port:        4 bits                   |
///     |  RAM output port: 4 bits per bank          |
///     +-------------------------------------------+
/// ```
pub struct Intel4004Simulator {
    /// The 4-bit accumulator (0-15).
    pub accumulator: u8,
    /// 16 general-purpose 4-bit registers (R0-R15).
    /// Arranged as 8 pairs: (R0,R1), (R2,R3), ..., (R14,R15).
    /// Each register holds a 4-bit value (0-15).
    pub registers: Vec<u8>,
    /// The carry flag -- set when arithmetic overflows or underflows.
    pub carry: bool,
    /// Byte-addressable program memory (ROM).
    /// The real 4004 supports up to 4096 bytes.
    pub memory: Vec<u8>,
    /// Program counter (12-bit address space, 0-4095).
    pub pc: usize,
    /// Whether the CPU has halted (via the custom HLT instruction).
    pub halted: bool,

    // ----- New fields for full 4004 support -----

    /// Hardware subroutine stack.
    ///
    /// The real 4004 has a 3-level deep hardware stack (no RAM-based stack).
    /// Each entry holds a 12-bit return address. When you call JMS, the
    /// return address is pushed. When you execute BBL, it is popped.
    /// If you nest more than 3 levels, the oldest address is lost (wraps).
    pub hw_stack: [u16; 3],

    /// Stack pointer (0-2). Points to the next free slot.
    /// On the real 4004, the stack wraps silently -- there is no overflow trap.
    pub stack_pointer: usize,

    /// Data RAM: 4 banks x 4 registers x 16 characters (nibbles).
    ///
    /// ```text
    ///   ram[bank][register][character]
    ///        0-3     0-3       0-15
    /// ```
    ///
    /// Each element stores a 4-bit value (0-15). Selected by DCL (bank)
    /// and SRC (register + character). Read with RDM, written with WRM.
    pub ram: [[[u8; 16]; 4]; 4],

    /// RAM status characters: 4 banks x 4 registers x 4 status nibbles.
    ///
    /// Each register has 4 extra status nibbles (separate from the 16 main
    /// characters). Read with RD0-RD3, written with WR0-WR3.
    pub ram_status: [[[u8; 4]; 4]; 4],

    /// RAM output port: one 4-bit latch per bank.
    /// Written by WMP, which writes the accumulator to the currently
    /// selected bank's output port.
    pub ram_output: [u8; 4],

    /// Currently selected RAM bank (set by DCL). Range: 0-3.
    pub ram_bank: usize,

    /// Currently selected RAM register within the bank (set by SRC). Range: 0-3.
    pub ram_register: usize,

    /// Currently selected RAM character within the register (set by SRC). Range: 0-15.
    pub ram_character: usize,

    /// ROM I/O port: 4-bit value. Written by WRR, read by RDR.
    pub rom_port: u8,
}

impl Intel4004Simulator {
    /// Create a new Intel 4004 simulator with the given program memory size.
    ///
    /// All state is initialized to zero: accumulator, registers, carry, stack,
    /// RAM, ports, and program counter.
    pub fn new(memory_size: usize) -> Self {
        Intel4004Simulator {
            accumulator: 0,
            registers: vec![0; 16],
            carry: false,
            memory: vec![0; memory_size],
            pc: 0,
            halted: false,
            hw_stack: [0; 3],
            stack_pointer: 0,
            ram: [[[0u8; 16]; 4]; 4],
            ram_status: [[[0u8; 4]; 4]; 4],
            ram_output: [0u8; 4],
            ram_bank: 0,
            ram_register: 0,
            ram_character: 0,
            rom_port: 0,
        }
    }

    /// Reset all CPU state to initial values (zeros).
    ///
    /// Called automatically by `run()` before loading a new program.
    /// Useful for running multiple programs sequentially on the same
    /// simulator instance without constructing a new one.
    pub fn reset(&mut self) {
        self.accumulator = 0;
        for r in self.registers.iter_mut() {
            *r = 0;
        }
        self.carry = false;
        for b in self.memory.iter_mut() {
            *b = 0;
        }
        self.pc = 0;
        self.halted = false;
        self.hw_stack = [0; 3];
        self.stack_pointer = 0;
        self.ram = [[[0u8; 16]; 4]; 4];
        self.ram_status = [[[0u8; 4]; 4]; 4];
        self.ram_output = [0u8; 4];
        self.ram_bank = 0;
        self.ram_register = 0;
        self.ram_character = 0;
        self.rom_port = 0;
    }

    /// Load a program into memory starting at address 0.
    pub fn load_program(&mut self, program: &[u8]) {
        for (i, &b) in program.iter().enumerate() {
            self.memory[i] = b;
        }
        self.pc = 0;
        self.halted = false;
    }

    // -----------------------------------------------------------------------
    // Register pair helpers
    // -----------------------------------------------------------------------

    /// Read an 8-bit value from register pair `p`.
    ///
    /// The 4004 groups its 16 registers into 8 pairs:
    ///   Pair 0 = (R0, R1), Pair 1 = (R2, R3), ..., Pair 7 = (R14, R15)
    ///
    /// The high nibble comes from the even register, the low nibble from
    /// the odd register. This gives an 8-bit value (0-255).
    ///
    /// ```text
    ///   Pair p -> R[2p] (high nibble) : R[2p+1] (low nibble)
    ///   Example: Pair 0, R0=0xA, R1=0x3 -> 0xA3
    /// ```
    fn read_pair(&self, p: usize) -> u8 {
        let hi = self.registers[p * 2] & 0xF;
        let lo = self.registers[p * 2 + 1] & 0xF;
        (hi << 4) | lo
    }

    /// Write an 8-bit value to register pair `p`.
    ///
    /// The high nibble goes into the even register, the low nibble into
    /// the odd register.
    fn write_pair(&mut self, p: usize, val: u8) {
        self.registers[p * 2] = (val >> 4) & 0xF;
        self.registers[p * 2 + 1] = val & 0xF;
    }

    // -----------------------------------------------------------------------
    // Hardware stack helpers
    // -----------------------------------------------------------------------

    /// Push a 12-bit address onto the hardware stack.
    ///
    /// The 4004's stack is only 3 levels deep. If you push a 4th value,
    /// it wraps around and overwrites the oldest entry. The real hardware
    /// does this silently -- no exception, no warning.
    fn stack_push(&mut self, addr: u16) {
        self.hw_stack[self.stack_pointer % 3] = addr & 0xFFF;
        self.stack_pointer = (self.stack_pointer + 1) % 3;
    }

    /// Pop a 12-bit address from the hardware stack.
    ///
    /// Returns the most recently pushed address. If the stack is empty
    /// (underflow), it wraps and returns whatever value happens to be
    /// in that slot -- just like the real hardware.
    fn stack_pop(&mut self) -> u16 {
        self.stack_pointer = if self.stack_pointer == 0 { 2 } else { self.stack_pointer - 1 };
        self.hw_stack[self.stack_pointer % 3]
    }

    // -----------------------------------------------------------------------
    // RAM access helpers
    // -----------------------------------------------------------------------

    /// Read the currently addressed RAM main memory nibble.
    ///
    /// The address is set by DCL (bank) and SRC (register + character).
    /// Returns a 4-bit value (0-15).
    fn ram_read_main(&self) -> u8 {
        self.ram[self.ram_bank][self.ram_register][self.ram_character] & 0xF
    }

    /// Write a 4-bit value to the currently addressed RAM main memory nibble.
    fn ram_write_main(&mut self, val: u8) {
        self.ram[self.ram_bank][self.ram_register][self.ram_character] = val & 0xF;
    }

    /// Read a RAM status nibble at the given index (0-3).
    fn ram_read_status(&self, idx: usize) -> u8 {
        self.ram_status[self.ram_bank][self.ram_register][idx] & 0xF
    }

    /// Write a RAM status nibble at the given index (0-3).
    fn ram_write_status(&mut self, idx: usize, val: u8) {
        self.ram_status[self.ram_bank][self.ram_register][idx] = val & 0xF;
    }

    // -----------------------------------------------------------------------
    // Step (fetch-decode-execute cycle)
    // -----------------------------------------------------------------------

    /// Execute one instruction and return a trace.
    ///
    /// The fetch-decode-execute cycle:
    /// 1. **Fetch**: Read the byte at the program counter.
    /// 2. **Detect 2-byte**: If this opcode needs a second byte, fetch it too.
    /// 3. **Decode**: Split the first byte into opcode (upper nibble) and
    ///    operand (lower nibble).
    /// 4. **Execute**: Dispatch to the appropriate instruction handler.
    /// 5. **Trace**: Record before/after state for debugging.
    ///
    /// # Panics
    ///
    /// Panics if the CPU has already halted.
    pub fn step(&mut self) -> Intel4004Trace {
        assert!(!self.halted, "CPU is halted");

        let address = self.pc;
        let raw = self.memory[self.pc];
        self.pc += 1;

        // Fetch second byte for 2-byte instructions.
        let raw2 = if is_two_byte(raw) {
            let b = self.memory[self.pc];
            self.pc += 1;
            Some(b)
        } else {
            None
        };

        let acc_before = self.accumulator;
        let carry_before = self.carry;

        // Decode: split 8-bit instruction into 4-bit opcode and operand.
        let opcode = (raw >> 4) & 0xF;
        let operand = raw & 0xF;

        let mnemonic = self.execute(opcode, operand, raw, raw2, address);

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

    // -----------------------------------------------------------------------
    // Execute (instruction dispatch)
    // -----------------------------------------------------------------------

    /// Execute the decoded instruction, updating all CPU state.
    ///
    /// The instruction set is organized by upper nibble:
    ///
    /// ```text
    ///   0x0: NOP (0x00), HLT (0x01)
    ///   0x1: JCN -- conditional jump
    ///   0x2: FIM (even) / SRC (odd)
    ///   0x3: FIN (even) / JIN (odd)
    ///   0x4: JUN -- unconditional jump
    ///   0x5: JMS -- jump to subroutine
    ///   0x6: INC -- increment register
    ///   0x7: ISZ -- increment and skip if zero
    ///   0x8: ADD -- add register to accumulator
    ///   0x9: SUB -- subtract register from accumulator
    ///   0xA: LD  -- load register to accumulator
    ///   0xB: XCH -- exchange accumulator and register
    ///   0xC: BBL -- branch back and load
    ///   0xD: LDM -- load immediate to accumulator
    ///   0xE: I/O and RAM instructions
    ///   0xF: Accumulator group instructions
    /// ```
    fn execute(
        &mut self,
        opcode: u8,
        operand: u8,
        raw: u8,
        raw2: Option<u8>,
        address: usize,
    ) -> String {
        // ----- Special single-byte instructions at 0x00 and 0x01 -----
        if raw == 0x00 {
            // NOP -- No Operation.
            // The CPU does nothing for one instruction cycle.
            // Used for timing delays or as placeholder padding.
            return "NOP".to_string();
        }
        if raw == 0x01 {
            // HLT -- Halt (custom instruction for testing).
            // Not part of the original 4004 instruction set, but invaluable
            // for testing: it stops execution cleanly.
            self.halted = true;
            return "HLT".to_string();
        }

        match opcode {
            // =================================================================
            // 0x1: JCN -- Jump Conditional
            // =================================================================
            //
            // Format: 2 bytes -- [0x1C] [address_low]
            //   C = condition nibble (4 bits):
            //     bit 3 (8): invert -- if set, jump when condition is FALSE
            //     bit 2 (4): test zero -- true if accumulator == 0
            //     bit 1 (2): test carry -- true if carry flag is set
            //     bit 0 (1): test pin -- true if test pin is active (always 0 in sim)
            //
            // The condition bits are OR'd together. If any enabled test is true
            // (before inversion), the condition is met.
            //
            // Target address: same ROM page as the JCN instruction, low byte
            // replaced by the second byte. Formally:
            //   target = (address_of_JCN + 2) & 0xF00 | raw2
            //
            // This limits JCN to jumping within the current 256-byte page.
            0x1 => {
                let cond = operand;
                let addr_low = raw2.unwrap_or(0);

                let invert = cond & 0x8 != 0;
                let test_zero = cond & 0x4 != 0;
                let test_carry = cond & 0x2 != 0;
                let test_pin = cond & 0x1 != 0;

                // Evaluate: OR together enabled tests.
                let mut result = false;
                if test_zero {
                    result = result || (self.accumulator == 0);
                }
                if test_carry {
                    result = result || self.carry;
                }
                if test_pin {
                    // Test pin is always 0 in this simulator (no external hardware).
                    result = result || false;
                }

                // Apply inversion.
                if invert {
                    result = !result;
                }

                if result {
                    // Jump target: keep the page (high nibble of 12-bit addr),
                    // replace the low 8 bits.
                    let page = (address + 2) & 0xF00;
                    self.pc = page | (addr_low as usize);
                }

                format!("JCN 0x{:X},0x{:02X}", cond, addr_low)
            }

            // =================================================================
            // 0x2 (even): FIM -- Fetch Immediate to Register Pair
            // =================================================================
            //
            // Format: 2 bytes -- [0x2P0] [data8]
            //   P = register pair number (0-7)
            //   data8 = 8-bit immediate value
            //
            // Loads an 8-bit value into a register pair. The high nibble goes
            // to the even register, the low nibble to the odd register.
            //
            // Example: FIM P0, 0xA3 -> R0=0xA, R1=0x3
            0x2 if raw & 1 == 0 => {
                let pair = (operand >> 1) as usize;
                let data = raw2.unwrap_or(0);
                self.write_pair(pair, data);
                format!("FIM P{},0x{:02X}", pair, data)
            }

            // =================================================================
            // 0x2 (odd): SRC -- Send Register Control
            // =================================================================
            //
            // Format: 1 byte -- [0x2P1]
            //   P = register pair number (0-7)
            //
            // Sets the RAM address for subsequent I/O instructions (WRM, RDM,
            // WR0-WR3, RD0-RD3, WMP). The register pair provides:
            //   - Bits 5-4 of pair value: selects RAM register (0-3)
            //   - Bits 3-0 of pair value: selects character (0-15)
            0x2 => {
                let pair = (operand >> 1) as usize;
                let pair_val = self.read_pair(pair);
                // Bits 5-4 select the RAM register (0-3).
                // Bits 3-0 select the character within that register (0-15).
                self.ram_register = ((pair_val >> 4) & 0x3) as usize;
                self.ram_character = (pair_val & 0xF) as usize;
                format!("SRC P{}", pair)
            }

            // =================================================================
            // 0x3 (even): FIN -- Fetch Indirect from ROM
            // =================================================================
            //
            // Format: 1 byte -- [0x3P0]
            //   P = register pair number (0-7)
            //
            // Reads 1 byte from ROM at the address formed by register pair 0
            // (R0:R1), and stores it in register pair P. This is an indirect
            // load from ROM -- useful for table lookups.
            //
            // The ROM address is: (current_page & 0xF00) | R0:R1
            0x3 if raw & 1 == 0 => {
                let pair = (operand >> 1) as usize;
                let rom_addr = self.read_pair(0) as usize;
                let page = self.pc & 0xF00;
                let full_addr = page | rom_addr;
                let data = self.memory[full_addr];
                self.write_pair(pair, data);
                format!("FIN P{}", pair)
            }

            // =================================================================
            // 0x3 (odd): JIN -- Jump Indirect
            // =================================================================
            //
            // Format: 1 byte -- [0x3P1]
            //   P = register pair number (0-7)
            //
            // Jumps to the address formed by: current page | register pair value.
            // Like JCN, limited to the current 256-byte page.
            0x3 => {
                let pair = (operand >> 1) as usize;
                let pair_val = self.read_pair(pair) as usize;
                let page = self.pc & 0xF00;
                self.pc = page | pair_val;
                format!("JIN P{}", pair)
            }

            // =================================================================
            // 0x4: JUN -- Jump Unconditional
            // =================================================================
            //
            // Format: 2 bytes -- [0x4H] [LL]
            //   Full 12-bit address = (H << 8) | LL
            //   H = operand (lower nibble of first byte), provides bits 11-8
            //   LL = second byte, provides bits 7-0
            //
            // Jumps anywhere in the 4096-byte address space.
            0x4 => {
                let addr_hi = (operand as u16) << 8;
                let addr_lo = raw2.unwrap_or(0) as u16;
                let target = (addr_hi | addr_lo) as usize;
                self.pc = target;
                format!("JUN 0x{:03X}", target)
            }

            // =================================================================
            // 0x5: JMS -- Jump to Subroutine
            // =================================================================
            //
            // Format: 2 bytes -- [0x5H] [LL]
            //   Same address format as JUN.
            //
            // Pushes the return address (PC after this instruction) onto the
            // hardware stack, then jumps to the target address. Use BBL to return.
            //
            // The stack is only 3 deep -- nesting more than 3 subroutine calls
            // silently corrupts the stack (oldest return address lost).
            0x5 => {
                let addr_hi = (operand as u16) << 8;
                let addr_lo = raw2.unwrap_or(0) as u16;
                let target = (addr_hi | addr_lo) as usize;
                self.stack_push(self.pc as u16);
                self.pc = target;
                format!("JMS 0x{:03X}", target)
            }

            // =================================================================
            // 0x6: INC -- Increment Register
            // =================================================================
            //
            // Format: 1 byte -- [0x6R]
            //   R = register number (0-15)
            //
            // Adds 1 to the register, wrapping from 15 to 0.
            // Does NOT affect the carry flag (unlike ADD).
            0x6 => {
                let reg = operand as usize;
                self.registers[reg] = (self.registers[reg].wrapping_add(1)) & 0xF;
                format!("INC R{}", reg)
            }

            // =================================================================
            // 0x7: ISZ -- Increment and Skip if Zero
            // =================================================================
            //
            // Format: 2 bytes -- [0x7R] [address_low]
            //   R = register number (0-15)
            //   address_low = jump target (low 8 bits, same page)
            //
            // Increments register R (wrapping at 15->0). If the result is NOT
            // zero, jumps to the target address (same page). If zero, falls
            // through to the next instruction.
            //
            // This is the 4004's loop instruction: set a register to a count,
            // then ISZ decrements (via wrapping increment from complement)
            // until it hits zero.
            0x7 => {
                let reg = operand as usize;
                self.registers[reg] = (self.registers[reg].wrapping_add(1)) & 0xF;
                let addr_low = raw2.unwrap_or(0);
                if self.registers[reg] != 0 {
                    let page = (address + 2) & 0xF00;
                    self.pc = page | (addr_low as usize);
                }
                format!("ISZ R{},0x{:02X}", reg, addr_low)
            }

            // =================================================================
            // 0x8: ADD -- Add Register to Accumulator (with carry)
            // =================================================================
            //
            // The 4004's ADD includes the carry flag in the addition:
            //   result = Accumulator + Register[R] + Carry
            //
            // This enables multi-digit BCD addition: the carry from adding
            // one digit propagates to the next digit.
            //
            // Carry is set if result > 15 (overflow).
            // Accumulator gets result & 0xF.
            //
            // Example: A=9, R0=8, carry=1 -> result=18, carry=true, A=2
            0x8 => {
                let reg = (operand & 0xF) as usize;
                let carry_in = if self.carry { 1u16 } else { 0 };
                let result = self.accumulator as u16 + self.registers[reg] as u16 + carry_in;
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                format!("ADD R{}", reg)
            }

            // =================================================================
            // 0x9: SUB -- Subtract Register from Accumulator
            // =================================================================
            //
            // The 4004 implements subtraction using complement-and-add:
            //   complement = NOT(Register[R]) & 0xF   (ones' complement)
            //   borrow_in = if carry then 0 else 1
            //   result = Accumulator + complement + borrow_in
            //
            // This is mathematically equivalent to: A - R - borrow
            //
            // The carry flag has inverted meaning for subtraction:
            //   carry=true  means NO borrow occurred (result >= 0)
            //   carry=false means borrow occurred (result < 0)
            //
            // After: carry = (result > 15), Accumulator = result & 0xF
            //
            // The carry flag starts false, so borrow_in=1 for standalone subtracts,
            // giving the correct result. For chained subtracts, the carry from the
            // previous operation propagates correctly.
            //
            // Example: A=5, R0=3, carry=false (fresh)
            //   complement(3)=12, borrow_in=1, 5+12+1=18, carry=true, A=2
            //   Result: 5-3=2, no borrow. Correct!
            0x9 => {
                let reg = (operand & 0xF) as usize;
                let complement = (!self.registers[reg]) & 0xF;
                let borrow_in: u16 = if self.carry { 0 } else { 1 };
                let result = self.accumulator as u16 + complement as u16 + borrow_in;
                self.carry = result > 0xF;
                self.accumulator = (result & 0xF) as u8;
                format!("SUB R{}", reg)
            }

            // =================================================================
            // 0xA: LD -- Load Register to Accumulator
            // =================================================================
            //
            // Copies the value of register R into the accumulator.
            // The register is unchanged. Both values are 4-bit.
            0xA => {
                let reg = (operand & 0xF) as usize;
                self.accumulator = self.registers[reg] & 0xF;
                format!("LD R{}", reg)
            }

            // =================================================================
            // 0xB: XCH -- Exchange Accumulator with Register
            // =================================================================
            //
            // Swaps the accumulator and register R.
            // Both values are masked to 4 bits.
            0xB => {
                let reg = (operand & 0xF) as usize;
                let old_a = self.accumulator;
                self.accumulator = self.registers[reg] & 0xF;
                self.registers[reg] = old_a & 0xF;
                format!("XCH R{}", reg)
            }

            // =================================================================
            // 0xC: BBL -- Branch Back and Load
            // =================================================================
            //
            // Returns from a subroutine: pops the return address from the
            // hardware stack and loads an immediate 4-bit value into the
            // accumulator. The immediate value is in the operand nibble.
            //
            // This is the ONLY way to return from JMS. The "load" part is
            // a bonus -- it lets subroutines return a small result without
            // needing extra instructions.
            0xC => {
                let ret_addr = self.stack_pop();
                self.pc = ret_addr as usize;
                self.accumulator = operand & 0xF;
                format!("BBL {}", operand)
            }

            // =================================================================
            // 0xD: LDM -- Load Immediate to Accumulator
            // =================================================================
            //
            // Loads a 4-bit immediate value into the accumulator.
            // The value is in the operand nibble (lower 4 bits of the instruction).
            0xD => {
                self.accumulator = operand & 0xF;
                format!("LDM {}", operand)
            }

            // =================================================================
            // 0xE: I/O and RAM Instructions
            // =================================================================
            //
            // These instructions interact with the RAM data memory, RAM status
            // characters, RAM output port, and ROM I/O port. The full byte
            // determines the specific operation.
            0xE => {
                match raw {
                    // WRM -- Write accumulator to RAM main memory.
                    0xE0 => {
                        self.ram_write_main(self.accumulator);
                        "WRM".to_string()
                    }
                    // WMP -- Write accumulator to RAM output port.
                    // The port is specific to the currently selected RAM bank.
                    0xE1 => {
                        self.ram_output[self.ram_bank] = self.accumulator & 0xF;
                        "WMP".to_string()
                    }
                    // WRR -- Write accumulator to ROM port.
                    0xE2 => {
                        self.rom_port = self.accumulator & 0xF;
                        "WRR".to_string()
                    }
                    // WPM -- Write program memory (not implemented in simulator).
                    // On the real 4004, this writes to an external program
                    // memory device. We treat it as a NOP.
                    0xE3 => {
                        "WPM".to_string()
                    }
                    // WR0-WR3 -- Write accumulator to RAM status character 0-3.
                    0xE4 => {
                        self.ram_write_status(0, self.accumulator);
                        "WR0".to_string()
                    }
                    0xE5 => {
                        self.ram_write_status(1, self.accumulator);
                        "WR1".to_string()
                    }
                    0xE6 => {
                        self.ram_write_status(2, self.accumulator);
                        "WR2".to_string()
                    }
                    0xE7 => {
                        self.ram_write_status(3, self.accumulator);
                        "WR3".to_string()
                    }
                    // SBM -- Subtract RAM main memory from accumulator.
                    // Uses the same complement-and-add method as SUB.
                    0xE8 => {
                        let mem_val = self.ram_read_main();
                        let complement = (!mem_val) & 0xF;
                        let borrow_in: u16 = if self.carry { 0 } else { 1 };
                        let result = self.accumulator as u16 + complement as u16 + borrow_in;
                        self.carry = result > 0xF;
                        self.accumulator = (result & 0xF) as u8;
                        "SBM".to_string()
                    }
                    // RDM -- Read RAM main memory into accumulator.
                    0xE9 => {
                        self.accumulator = self.ram_read_main();
                        "RDM".to_string()
                    }
                    // RDR -- Read ROM port into accumulator.
                    0xEA => {
                        self.accumulator = self.rom_port & 0xF;
                        "RDR".to_string()
                    }
                    // ADM -- Add RAM main memory to accumulator (with carry).
                    0xEB => {
                        let mem_val = self.ram_read_main();
                        let carry_in = if self.carry { 1u16 } else { 0 };
                        let result = self.accumulator as u16 + mem_val as u16 + carry_in;
                        self.carry = result > 0xF;
                        self.accumulator = (result & 0xF) as u8;
                        "ADM".to_string()
                    }
                    // RD0-RD3 -- Read RAM status character 0-3 into accumulator.
                    0xEC => {
                        self.accumulator = self.ram_read_status(0);
                        "RD0".to_string()
                    }
                    0xED => {
                        self.accumulator = self.ram_read_status(1);
                        "RD1".to_string()
                    }
                    0xEE => {
                        self.accumulator = self.ram_read_status(2);
                        "RD2".to_string()
                    }
                    0xEF => {
                        self.accumulator = self.ram_read_status(3);
                        "RD3".to_string()
                    }
                    _ => format!("UNKNOWN(0x{:02X})", raw),
                }
            }

            // =================================================================
            // 0xF: Accumulator Group Instructions
            // =================================================================
            //
            // These instructions operate on the accumulator and/or carry flag.
            // Each is identified by the full byte (0xF0-0xFD).
            0xF => {
                match raw {
                    // CLB -- Clear both accumulator and carry.
                    // The simplest reset: zero out everything in one instruction.
                    0xF0 => {
                        self.accumulator = 0;
                        self.carry = false;
                        "CLB".to_string()
                    }
                    // CLC -- Clear carry flag only.
                    // Useful before starting a fresh addition chain.
                    0xF1 => {
                        self.carry = false;
                        "CLC".to_string()
                    }
                    // IAC -- Increment Accumulator.
                    // Adds 1 to the accumulator, wrapping 15->0.
                    // Sets carry if overflow (was 15, now 0).
                    0xF2 => {
                        let result = self.accumulator as u16 + 1;
                        self.carry = result > 0xF;
                        self.accumulator = (result & 0xF) as u8;
                        "IAC".to_string()
                    }
                    // CMC -- Complement Carry.
                    // Flips the carry flag: true->false, false->true.
                    0xF3 => {
                        self.carry = !self.carry;
                        "CMC".to_string()
                    }
                    // CMA -- Complement Accumulator.
                    // Inverts all 4 bits: A = NOT(A) & 0xF.
                    // Example: A=0b0101 (5) -> A=0b1010 (10)
                    0xF4 => {
                        self.accumulator = (!self.accumulator) & 0xF;
                        "CMA".to_string()
                    }
                    // RAL -- Rotate Accumulator Left through carry.
                    //
                    // Before:  [carry] [b3 b2 b1 b0]
                    // After:   [b3]    [b2 b1 b0 carry_old]
                    //
                    // The old carry becomes bit 0 of the accumulator.
                    // Bit 3 of the accumulator becomes the new carry.
                    // This is a 5-bit rotate (4 data bits + carry).
                    0xF5 => {
                        let old_carry = if self.carry { 1u8 } else { 0 };
                        self.carry = self.accumulator & 0x8 != 0;
                        self.accumulator = ((self.accumulator << 1) | old_carry) & 0xF;
                        "RAL".to_string()
                    }
                    // RAR -- Rotate Accumulator Right through carry.
                    //
                    // Before:  [carry] [b3 b2 b1 b0]
                    // After:   [b0]    [carry_old b3 b2 b1]
                    //
                    // The old carry becomes bit 3 of the accumulator.
                    // Bit 0 of the accumulator becomes the new carry.
                    0xF6 => {
                        let old_carry = if self.carry { 0x8u8 } else { 0 };
                        self.carry = self.accumulator & 0x1 != 0;
                        self.accumulator = ((self.accumulator >> 1) | old_carry) & 0xF;
                        "RAR".to_string()
                    }
                    // TCC -- Transfer Carry to accumulator and Clear Carry.
                    // A = 1 if carry was set, 0 otherwise. Carry is cleared.
                    0xF7 => {
                        self.accumulator = if self.carry { 1 } else { 0 };
                        self.carry = false;
                        "TCC".to_string()
                    }
                    // DAC -- Decrement Accumulator.
                    // Subtracts 1 from the accumulator, wrapping 0->15.
                    // Carry is set if NO borrow (i.e., A was > 0).
                    // Carry is cleared if borrow occurred (A was 0).
                    0xF8 => {
                        self.carry = self.accumulator > 0;
                        self.accumulator = (self.accumulator.wrapping_sub(1)) & 0xF;
                        "DAC".to_string()
                    }
                    // TCS -- Transfer Carry Subtract.
                    // If carry was set: A = 10. If carry was clear: A = 9.
                    // Carry is cleared.
                    //
                    // This is designed for BCD (Binary Coded Decimal) subtraction.
                    // After subtracting two BCD digits, TCS provides the correction
                    // factor based on whether there was a borrow.
                    0xF9 => {
                        self.accumulator = if self.carry { 10 } else { 9 };
                        self.carry = false;
                        "TCS".to_string()
                    }
                    // STC -- Set Carry.
                    0xFA => {
                        self.carry = true;
                        "STC".to_string()
                    }
                    // DAA -- Decimal Adjust Accumulator.
                    //
                    // After a binary ADD, if the result is > 9 or carry is set,
                    // add 6 to correct back to BCD. If the correction itself
                    // overflows, set carry.
                    //
                    // Example: 8 + 5 = 13 (binary). DAA adds 6 -> 19, carry=true,
                    // A = 19 & 0xF = 3. So BCD result is 13: carry=1, digit=3.
                    0xFB => {
                        if self.accumulator > 9 || self.carry {
                            let result = self.accumulator as u16 + 6;
                            if result > 0xF {
                                self.carry = true;
                            }
                            self.accumulator = (result & 0xF) as u8;
                        }
                        "DAA".to_string()
                    }
                    // KBP -- Keyboard Process.
                    //
                    // Converts a one-hot encoded keyboard input to a binary position:
                    //   0b0000 (0)  -> 0 (no key pressed)
                    //   0b0001 (1)  -> 1 (key 1)
                    //   0b0010 (2)  -> 2 (key 2)
                    //   0b0100 (4)  -> 3 (key 3)
                    //   0b1000 (8)  -> 4 (key 4)
                    //   anything else -> 15 (error: multiple keys pressed)
                    //
                    // This was designed for the Busicom calculator keyboard.
                    0xFC => {
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
                    // DCL -- Designate Command Line (select RAM bank).
                    //
                    // Sets the RAM bank based on the lower 3 bits of the accumulator.
                    // Banks 0-3 are valid; bits beyond 2 are masked off.
                    //
                    // On the real 4004, the 3 accumulator bits drive 3 "command lines"
                    // (CM0, CM1, CM2) that select which RAM bank responds to subsequent
                    // I/O instructions.
                    0xFD => {
                        self.ram_bank = (self.accumulator & 0x7) as usize;
                        if self.ram_bank > 3 {
                            self.ram_bank &= 3;
                        }
                        "DCL".to_string()
                    }
                    _ => format!("UNKNOWN(0x{:02X})", raw),
                }
            }

            _ => format!("UNKNOWN(0x{:02X})", raw),
        }
    }

    /// Run a program to completion or until max_steps is reached.
    ///
    /// Resets all CPU state before loading the program, ensuring a clean
    /// execution environment. Returns the trace of every executed instruction.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Vec<Intel4004Trace> {
        self.reset();
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

/// Encode `NOP` (no operation).
pub fn encode_nop() -> u8 {
    0x00
}

/// Encode `HLT` (halt).
pub fn encode_hlt() -> u8 {
    0x01
}

/// Encode `LDM N` (load immediate N into accumulator).
pub fn encode_ldm(n: u8) -> u8 {
    (0xD << 4) | (n & 0xF)
}

/// Encode `LD Rn` (load register n into accumulator).
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

/// Encode `SUB Rn` (subtract register n from accumulator).
pub fn encode_sub(r: u8) -> u8 {
    (0x9 << 4) | (r & 0xF)
}

/// Encode `INC Rn` (increment register n).
pub fn encode_inc(r: u8) -> u8 {
    (0x6 << 4) | (r & 0xF)
}

/// Encode `BBL N` (branch back and load N into accumulator).
pub fn encode_bbl(n: u8) -> u8 {
    (0xC << 4) | (n & 0xF)
}

/// Encode `JCN cond, addr` (conditional jump -- 2 bytes).
/// Returns (byte1, byte2).
pub fn encode_jcn(cond: u8, addr: u8) -> (u8, u8) {
    ((0x1 << 4) | (cond & 0xF), addr)
}

/// Encode `FIM Pp, data` (fetch immediate to register pair -- 2 bytes).
/// Returns (byte1, byte2).
pub fn encode_fim(pair: u8, data: u8) -> (u8, u8) {
    ((0x2 << 4) | ((pair & 0x7) << 1), data)
}

/// Encode `SRC Pp` (send register control).
pub fn encode_src(pair: u8) -> u8 {
    (0x2 << 4) | ((pair & 0x7) << 1) | 1
}

/// Encode `FIN Pp` (fetch indirect from ROM to register pair).
pub fn encode_fin(pair: u8) -> u8 {
    (0x3 << 4) | ((pair & 0x7) << 1)
}

/// Encode `JIN Pp` (jump indirect through register pair).
pub fn encode_jin(pair: u8) -> u8 {
    (0x3 << 4) | ((pair & 0x7) << 1) | 1
}

/// Encode `JUN addr` (unconditional jump -- 2 bytes).
/// Returns (byte1, byte2). addr is a 12-bit address.
pub fn encode_jun(addr: u16) -> (u8, u8) {
    let hi = ((addr >> 8) & 0xF) as u8;
    let lo = (addr & 0xFF) as u8;
    ((0x4 << 4) | hi, lo)
}

/// Encode `JMS addr` (jump to subroutine -- 2 bytes).
/// Returns (byte1, byte2). addr is a 12-bit address.
pub fn encode_jms(addr: u16) -> (u8, u8) {
    let hi = ((addr >> 8) & 0xF) as u8;
    let lo = (addr & 0xFF) as u8;
    ((0x5 << 4) | hi, lo)
}

/// Encode `ISZ Rn, addr` (increment and skip if zero -- 2 bytes).
/// Returns (byte1, byte2).
pub fn encode_isz(r: u8, addr: u8) -> (u8, u8) {
    ((0x7 << 4) | (r & 0xF), addr)
}

// I/O instruction encoders (all single-byte, fixed encoding).

/// Encode `WRM` (write accumulator to RAM main memory).
pub fn encode_wrm() -> u8 { 0xE0 }
/// Encode `WMP` (write accumulator to RAM output port).
pub fn encode_wmp() -> u8 { 0xE1 }
/// Encode `WRR` (write accumulator to ROM port).
pub fn encode_wrr() -> u8 { 0xE2 }
/// Encode `WPM` (write program memory -- no-op in simulator).
pub fn encode_wpm() -> u8 { 0xE3 }
/// Encode `WR0` (write accumulator to RAM status char 0).
pub fn encode_wr0() -> u8 { 0xE4 }
/// Encode `WR1` (write accumulator to RAM status char 1).
pub fn encode_wr1() -> u8 { 0xE5 }
/// Encode `WR2` (write accumulator to RAM status char 2).
pub fn encode_wr2() -> u8 { 0xE6 }
/// Encode `WR3` (write accumulator to RAM status char 3).
pub fn encode_wr3() -> u8 { 0xE7 }
/// Encode `SBM` (subtract RAM from accumulator).
pub fn encode_sbm() -> u8 { 0xE8 }
/// Encode `RDM` (read RAM main memory into accumulator).
pub fn encode_rdm() -> u8 { 0xE9 }
/// Encode `RDR` (read ROM port into accumulator).
pub fn encode_rdr() -> u8 { 0xEA }
/// Encode `ADM` (add RAM main memory to accumulator).
pub fn encode_adm() -> u8 { 0xEB }
/// Encode `RD0` (read RAM status char 0).
pub fn encode_rd0() -> u8 { 0xEC }
/// Encode `RD1` (read RAM status char 1).
pub fn encode_rd1() -> u8 { 0xED }
/// Encode `RD2` (read RAM status char 2).
pub fn encode_rd2() -> u8 { 0xEE }
/// Encode `RD3` (read RAM status char 3).
pub fn encode_rd3() -> u8 { 0xEF }

// Accumulator group encoders (all single-byte, fixed encoding).

/// Encode `CLB` (clear both accumulator and carry).
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
/// Encode `TCC` (transfer carry to accumulator, clear carry).
pub fn encode_tcc() -> u8 { 0xF7 }
/// Encode `DAC` (decrement accumulator).
pub fn encode_dac() -> u8 { 0xF8 }
/// Encode `TCS` (transfer carry subtract, clear carry).
pub fn encode_tcs() -> u8 { 0xF9 }
/// Encode `STC` (set carry).
pub fn encode_stc() -> u8 { 0xFA }
/// Encode `DAA` (decimal adjust accumulator).
pub fn encode_daa() -> u8 { 0xFB }
/// Encode `KBP` (keyboard process).
pub fn encode_kbp() -> u8 { 0xFC }
/// Encode `DCL` (designate command line -- select RAM bank).
pub fn encode_dcl() -> u8 { 0xFD }

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper to build and run a program conveniently
    // -----------------------------------------------------------------------

    fn run_program(program: &[u8]) -> Intel4004Simulator {
        let mut sim = Intel4004Simulator::new(4096);
        sim.run(program, 1000);
        sim
    }

    // =======================================================================
    // NOP and HLT
    // =======================================================================

    #[test]
    fn nop_does_nothing() {
        let mut sim = Intel4004Simulator::new(4096);
        let program = vec![encode_nop(), encode_nop(), encode_hlt()];
        let traces = sim.run(&program, 10);
        assert_eq!(traces.len(), 3);
        assert_eq!(traces[0].mnemonic, "NOP");
        assert_eq!(traces[1].mnemonic, "NOP");
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }

    #[test]
    fn hlt_stops_execution() {
        let sim = run_program(&[encode_hlt()]);
        assert!(sim.halted);
    }

    #[test]
    #[should_panic(expected = "CPU is halted")]
    fn halted_step_panics() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.halted = true;
        sim.step();
    }

    // =======================================================================
    // LDM -- Load Immediate
    // =======================================================================

    #[test]
    fn ldm_loads_immediate() {
        let sim = run_program(&[encode_ldm(7), encode_hlt()]);
        assert_eq!(sim.accumulator, 7);
    }

    #[test]
    fn ldm_all_values() {
        for n in 0..=15u8 {
            let sim = run_program(&[encode_ldm(n), encode_hlt()]);
            assert_eq!(sim.accumulator, n, "LDM {} failed", n);
        }
    }

    // =======================================================================
    // LD -- Load Register
    // =======================================================================

    #[test]
    fn ld_loads_register() {
        let sim = run_program(&[
            encode_ldm(9),
            encode_xch(3),  // R3 = 9
            encode_ldm(0),  // A = 0
            encode_ld(3),   // A = R3 = 9
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 9);
    }

    // =======================================================================
    // XCH -- Exchange Accumulator and Register
    // =======================================================================

    #[test]
    fn xch_exchanges_values() {
        let sim = run_program(&[
            encode_ldm(5),
            encode_xch(0),  // R0=5, A=0
            encode_ldm(3),  // A=3
            encode_xch(0),  // R0=3, A=5
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 5);
        assert_eq!(sim.registers[0], 3);
    }

    // =======================================================================
    // ADD -- Add with Carry
    // =======================================================================

    #[test]
    fn add_basic() {
        // 1 + 2 = 3, no carry
        let sim = run_program(&[
            encode_ldm(2), encode_xch(0),  // R0=2
            encode_ldm(1),                  // A=1
            encode_add(0),                  // A=1+2+0=3
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 3);
        assert!(!sim.carry);
    }

    #[test]
    fn add_overflow_sets_carry() {
        // 15 + 15 = 30 -> carry=true, A=14
        let sim = run_program(&[
            encode_ldm(15), encode_xch(0),
            encode_ldm(15),
            encode_add(0),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 14);
        assert!(sim.carry);
    }

    #[test]
    fn add_includes_carry() {
        // A=5, R0=3, set carry first -> 5+3+1=9
        let mut sim = Intel4004Simulator::new(4096);
        sim.load_program(&[
            encode_ldm(3), encode_xch(0),  // R0=3
            encode_ldm(5),                  // A=5
            encode_stc(),                   // carry=true
            encode_add(0),                  // A=5+3+1=9
            encode_hlt(),
        ]);
        // Run manually since run() resets
        while !sim.halted { sim.step(); }
        assert_eq!(sim.accumulator, 9);
        assert!(!sim.carry);
    }

    // =======================================================================
    // SUB -- Subtract (complement-add)
    // =======================================================================

    #[test]
    fn sub_basic_no_borrow() {
        // A=5, R0=3, carry starts false -> borrow_in=1
        // complement(3) = 12, 5+12+1 = 18, carry=true, A=2
        let sim = run_program(&[
            encode_ldm(3), encode_xch(0),
            encode_ldm(5),
            encode_sub(0),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 2);
        assert!(sim.carry, "No borrow: carry should be true");
    }

    #[test]
    fn sub_with_borrow() {
        // A=3, R0=5, carry=false -> borrow_in=1
        // complement(5) = 10, 3+10+1 = 14, carry=false, A=14
        let sim = run_program(&[
            encode_ldm(5), encode_xch(0),
            encode_ldm(3),
            encode_sub(0),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 14);
        assert!(!sim.carry, "Borrow occurred: carry should be false");
    }

    #[test]
    fn sub_zero_minus_one() {
        // A=0, R0=1, carry=false -> borrow_in=1
        // complement(1)=14, 0+14+1=15, carry=false, A=15
        let sim = run_program(&[
            encode_ldm(1), encode_xch(0),
            encode_ldm(0),
            encode_sub(0),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 15);
        assert!(!sim.carry);
    }

    // =======================================================================
    // INC -- Increment Register
    // =======================================================================

    #[test]
    fn inc_basic() {
        let sim = run_program(&[
            encode_ldm(7), encode_xch(2),  // R2=7
            encode_inc(2),                  // R2=8
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[2], 8);
    }

    #[test]
    fn inc_wraps_at_15() {
        let sim = run_program(&[
            encode_ldm(15), encode_xch(0),
            encode_inc(0),
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[0], 0);
    }

    #[test]
    fn inc_does_not_affect_carry() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.load_program(&[
            encode_ldm(15), encode_xch(0),
            encode_stc(),    // set carry
            encode_inc(0),   // R0: 15->0, carry should remain true
            encode_hlt(),
        ]);
        while !sim.halted { sim.step(); }
        assert_eq!(sim.registers[0], 0);
        assert!(sim.carry, "INC should not affect carry");
    }

    // =======================================================================
    // JUN -- Jump Unconditional
    // =======================================================================

    #[test]
    fn jun_jumps() {
        let (b1, b2) = encode_jun(0x004); // Jump to address 4
        let sim = run_program(&[
            b1, b2,             // JUN 0x004: skip next 2 bytes
            encode_ldm(15),     // skipped
            encode_hlt(),       // skipped
            encode_ldm(7),      // landed here
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 7);
    }

    // =======================================================================
    // JMS / BBL -- Subroutine call and return
    // =======================================================================

    #[test]
    fn jms_and_bbl() {
        // Main: JMS to subroutine, which returns with BBL 3.
        // After return, A should be 3 (BBL loads its operand into A).
        let (b1, b2) = encode_jms(0x004); // subroutine at addr 4
        let sim = run_program(&[
            b1, b2,             // 0x000: JMS 0x004
            encode_hlt(),       // 0x002: HLT (return here)
            0x00,               // 0x003: padding
            encode_ldm(5),      // 0x004: subroutine body: A=5
            encode_bbl(3),      // 0x005: return, A=3
        ]);
        assert_eq!(sim.accumulator, 3);
    }

    #[test]
    fn nested_subroutine_calls() {
        // Test 2 levels of nesting:
        // Main calls sub1 at 0x010, sub1 calls sub2 at 0x020.
        // sub2 returns BBL 7, sub1 returns BBL 9.
        let mut program = vec![0u8; 256];

        // Main at 0x000
        let (b1, b2) = encode_jms(0x010);
        program[0] = b1;
        program[1] = b2;
        program[2] = encode_hlt();

        // Sub1 at 0x010
        let (b1, b2) = encode_jms(0x020);
        program[0x010] = b1;
        program[0x011] = b2;
        // After sub2 returns, A=7. Now return with BBL 9.
        program[0x012] = encode_bbl(9);

        // Sub2 at 0x020
        program[0x020] = encode_bbl(7);

        let sim = run_program(&program);
        // Final A should be 9 (from sub1's BBL)
        assert_eq!(sim.accumulator, 9);
    }

    // =======================================================================
    // JCN -- Conditional Jump
    // =======================================================================

    #[test]
    fn jcn_jump_if_accumulator_zero() {
        // Condition 0x4: test_zero. A=0, so should jump.
        // Address layout:
        //   0: b1  1: b2  2: ldm15  3: hlt  4: nop  5: ldm1  6: hlt
        // JCN target = (0+2) & 0xF00 | 0x05 = 0x05 (addr 5)
        let (b1, b2) = encode_jcn(0x4, 0x05);
        let sim = run_program(&[
            b1, b2,             // addr 0-1: JCN test_zero, 0x05
            encode_ldm(15),     // addr 2: skipped
            encode_hlt(),       // addr 3: skipped
            0x00,               // addr 4: padding
            encode_ldm(1),      // addr 5: landed here
            encode_hlt(),       // addr 6
        ]);
        assert_eq!(sim.accumulator, 1);
    }

    #[test]
    fn jcn_no_jump_if_accumulator_nonzero() {
        // Condition 0x4: test_zero. A=5 (not zero), so should NOT jump.
        let (b1, b2) = encode_jcn(0x4, 0x06);
        let sim = run_program(&[
            encode_ldm(5),      // A=5
            b1, b2,             // JCN test_zero, 0x06 -- no jump
            encode_ldm(2),      // executed (A=2)
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 2);
    }

    #[test]
    fn jcn_invert_test() {
        // Condition 0xC: invert + test_zero. Jump if A is NOT zero.
        // Address layout:
        //   0: ldm5  1: b1  2: b2  3: ldm15  4: hlt  5: ldm1  6: hlt
        // JCN at addr 1, target = (1+2) & 0xF00 | 0x05 = 0x05
        let (b1, b2) = encode_jcn(0xC, 0x05);
        let sim = run_program(&[
            encode_ldm(5),      // addr 0: A=5 (not zero)
            b1, b2,             // addr 1-2: JCN invert+zero, 0x05
            encode_ldm(15),     // addr 3: skipped
            encode_hlt(),       // addr 4: skipped
            encode_ldm(1),      // addr 5: landed here
            encode_hlt(),       // addr 6
        ]);
        assert_eq!(sim.accumulator, 1);
    }

    #[test]
    fn jcn_test_carry() {
        // Condition 0x2: test_carry. Carry is set -> should jump.
        // Address layout:
        //   0: stc  1: b1  2: b2  3: ldm15  4: hlt  5: ldm1  6: hlt
        // JCN at addr 1, target = (1+2) & 0xF00 | 0x05 = 0x05
        let (b1, b2) = encode_jcn(0x2, 0x05);
        let sim = run_program(&[
            encode_stc(),       // addr 0: set carry
            b1, b2,             // addr 1-2: JCN test_carry, 0x05
            encode_ldm(15),     // addr 3: skipped
            encode_hlt(),       // addr 4: skipped
            encode_ldm(1),      // addr 5: landed here
            encode_hlt(),       // addr 6
        ]);
        assert_eq!(sim.accumulator, 1);
    }

    // =======================================================================
    // ISZ -- Increment and Skip if Zero
    // =======================================================================

    #[test]
    fn isz_loops_until_zero() {
        // Count from 14 to 0 (incrementing wraps: 14->15->0).
        // ISZ increments R0, jumps back if not zero.
        // After 2 iterations: R0=14->15->0, loop exits.
        let (isz_b1, isz_b2) = encode_isz(0, 0x02);
        let sim = run_program(&[
            encode_ldm(14), encode_xch(0),  // R0=14
            isz_b1, isz_b2,                 // addr 2: ISZ R0, 0x02
            encode_hlt(),                   // addr 4: exits when R0=0
        ]);
        assert_eq!(sim.registers[0], 0);
    }

    #[test]
    fn isz_falls_through_when_zero() {
        // R0=15, ISZ increments to 0 -> falls through (no jump).
        let (isz_b1, isz_b2) = encode_isz(0, 0x10);
        let sim = run_program(&[
            encode_ldm(15), encode_xch(0),
            isz_b1, isz_b2,    // ISZ R0, 0x10 -- R0=0, no jump
            encode_ldm(7),      // falls through here
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[0], 0);
        assert_eq!(sim.accumulator, 7);
    }

    // =======================================================================
    // FIM -- Fetch Immediate to Register Pair
    // =======================================================================

    #[test]
    fn fim_loads_pair() {
        let (b1, b2) = encode_fim(0, 0xA3);
        let sim = run_program(&[b1, b2, encode_hlt()]);
        assert_eq!(sim.registers[0], 0xA); // R0 = high nibble
        assert_eq!(sim.registers[1], 0x3); // R1 = low nibble
    }

    #[test]
    fn fim_all_pairs() {
        // Load different values into all 8 pairs.
        let mut program = Vec::new();
        for p in 0..8u8 {
            let val = (p << 4) | (15 - p);
            let (b1, b2) = encode_fim(p, val);
            program.push(b1);
            program.push(b2);
        }
        program.push(encode_hlt());
        let sim = run_program(&program);
        for p in 0..8usize {
            let val = ((p as u8) << 4) | (15 - p as u8);
            assert_eq!(sim.registers[p * 2], (val >> 4) & 0xF);
            assert_eq!(sim.registers[p * 2 + 1], val & 0xF);
        }
    }

    // =======================================================================
    // SRC -- Send Register Control
    // =======================================================================

    #[test]
    fn src_sets_ram_address() {
        // FIM P0, 0x25 -> R0=2, R1=5
        // SRC P0 -> ram_register = (0x25 >> 4) & 3 = 2, ram_character = 5
        let (b1, b2) = encode_fim(0, 0x25);
        let sim = run_program(&[b1, b2, encode_src(0), encode_hlt()]);
        assert_eq!(sim.ram_register, 2);
        assert_eq!(sim.ram_character, 5);
    }

    // =======================================================================
    // FIN -- Fetch Indirect from ROM
    // =======================================================================

    #[test]
    fn fin_reads_from_rom() {
        // Set R0:R1 = 0x08 (address 8 in ROM).
        // At ROM address 8, store 0xBC.
        // FIN P1 should read 0xBC -> R2=0xB, R3=0xC.
        let (fim_b1, fim_b2) = encode_fim(0, 0x08);
        let mut program = vec![
            fim_b1, fim_b2,     // FIM P0, 0x08
            encode_fin(1),      // FIN P1 (read ROM[0x08])
            encode_hlt(),
            0, 0, 0, 0,        // padding to addr 8
            0xBC,               // addr 8: data to read
        ];
        // Ensure program is long enough.
        while program.len() < 9 {
            program.push(0);
        }
        let sim = run_program(&program);
        assert_eq!(sim.registers[2], 0xB);
        assert_eq!(sim.registers[3], 0xC);
    }

    // =======================================================================
    // JIN -- Jump Indirect
    // =======================================================================

    #[test]
    fn jin_jumps_to_pair_address() {
        // FIM P0, 0x05 -> pair 0 = 0x05
        // JIN P0 -> jump to addr 0x05 (on current page)
        // Address layout:
        //   0: fim_b1  1: fim_b2  2: jin  3: ldm15  4: hlt  5: ldm3  6: hlt
        let (fim_b1, fim_b2) = encode_fim(0, 0x05);
        let sim = run_program(&[
            fim_b1, fim_b2,     // addr 0-1: FIM P0, 0x05
            encode_jin(0),      // addr 2: JIN P0 -> jump to 0x05
            encode_ldm(15),     // addr 3: skipped
            encode_hlt(),       // addr 4: skipped
            encode_ldm(3),      // addr 5: landed here
            encode_hlt(),       // addr 6
        ]);
        assert_eq!(sim.accumulator, 3);
    }

    // =======================================================================
    // RAM operations: WRM, RDM, SRC, DCL
    // =======================================================================

    #[test]
    fn wrm_and_rdm_roundtrip() {
        // Write 7 to RAM, read it back.
        let (fim_b1, fim_b2) = encode_fim(0, 0x00); // register 0, character 0
        let sim = run_program(&[
            fim_b1, fim_b2,
            encode_src(0),      // SRC P0 -> address RAM[0][0][0]
            encode_ldm(7),      // A=7
            encode_wrm(),       // RAM[0][0][0] = 7
            encode_ldm(0),      // A=0 (clear accumulator)
            encode_rdm(),       // A = RAM[0][0][0] = 7
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 7);
    }

    #[test]
    fn dcl_selects_ram_bank() {
        // Write to bank 0, then bank 2, then read both back.
        let (fim_b1, fim_b2) = encode_fim(0, 0x00);
        let sim = run_program(&[
            // Set up SRC to register 0, character 0.
            fim_b1, fim_b2,
            encode_src(0),
            // Write 5 to bank 0.
            encode_ldm(0),
            encode_dcl(),       // bank = 0
            encode_ldm(5),
            encode_wrm(),
            // Write 9 to bank 2.
            encode_ldm(2),
            encode_dcl(),       // bank = 2
            encode_ldm(9),
            encode_wrm(),
            // Read back bank 0.
            encode_ldm(0),
            encode_dcl(),       // bank = 0
            encode_rdm(),       // A = 5
            encode_xch(2),      // R2 = 5
            // Read back bank 2.
            encode_ldm(2),
            encode_dcl(),       // bank = 2
            encode_rdm(),       // A = 9
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 9);
        assert_eq!(sim.registers[2], 5);
    }

    // =======================================================================
    // RAM status: WR0-WR3, RD0-RD3
    // =======================================================================

    #[test]
    fn wr_rd_status_roundtrip() {
        let (fim_b1, fim_b2) = encode_fim(0, 0x00);
        let sim = run_program(&[
            fim_b1, fim_b2,
            encode_src(0),
            // Write status chars 0-3.
            encode_ldm(1),  encode_wr0(),
            encode_ldm(2),  encode_wr1(),
            encode_ldm(3),  encode_wr2(),
            encode_ldm(4),  encode_wr3(),
            // Read them back.
            encode_rd0(),   encode_xch(4),  // R4 = 1
            encode_rd1(),   encode_xch(5),  // R5 = 2
            encode_rd2(),   encode_xch(6),  // R6 = 3
            encode_rd3(),                   // A = 4
            encode_hlt(),
        ]);
        assert_eq!(sim.registers[4], 1);
        assert_eq!(sim.registers[5], 2);
        assert_eq!(sim.registers[6], 3);
        assert_eq!(sim.accumulator, 4);
    }

    // =======================================================================
    // ROM port: WRR, RDR
    // =======================================================================

    #[test]
    fn wrr_and_rdr_roundtrip() {
        let sim = run_program(&[
            encode_ldm(11),
            encode_wrr(),       // rom_port = 11
            encode_ldm(0),      // A = 0
            encode_rdr(),       // A = rom_port = 11
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 11);
    }

    // =======================================================================
    // WMP -- RAM output port
    // =======================================================================

    #[test]
    fn wmp_writes_output_port() {
        let sim = run_program(&[
            encode_ldm(0),
            encode_dcl(),       // bank 0
            encode_ldm(13),
            encode_wmp(),       // ram_output[0] = 13
            encode_hlt(),
        ]);
        assert_eq!(sim.ram_output[0], 13);
    }

    // =======================================================================
    // ADM -- Add RAM to Accumulator
    // =======================================================================

    #[test]
    fn adm_adds_ram_to_accumulator() {
        let (fim_b1, fim_b2) = encode_fim(0, 0x00);
        let sim = run_program(&[
            fim_b1, fim_b2,
            encode_src(0),
            encode_ldm(6),
            encode_wrm(),       // RAM[0][0][0] = 6
            encode_ldm(3),      // A = 3
            encode_adm(),       // A = 3 + 6 + 0 = 9
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 9);
        assert!(!sim.carry);
    }

    // =======================================================================
    // SBM -- Subtract RAM from Accumulator
    // =======================================================================

    #[test]
    fn sbm_subtracts_ram_from_accumulator() {
        let (fim_b1, fim_b2) = encode_fim(0, 0x00);
        let sim = run_program(&[
            fim_b1, fim_b2,
            encode_src(0),
            encode_ldm(3),
            encode_wrm(),       // RAM[0][0][0] = 3
            encode_ldm(7),      // A = 7
            encode_sbm(),       // A = 7 - 3 = 4, carry=true (no borrow)
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 4);
        assert!(sim.carry);
    }

    // =======================================================================
    // CLB -- Clear Both
    // =======================================================================

    #[test]
    fn clb_clears_accumulator_and_carry() {
        let sim = run_program(&[
            encode_ldm(15),
            encode_stc(),
            encode_clb(),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }

    // =======================================================================
    // CLC -- Clear Carry
    // =======================================================================

    #[test]
    fn clc_clears_carry_only() {
        let sim = run_program(&[
            encode_ldm(7),
            encode_stc(),
            encode_clc(),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 7);
        assert!(!sim.carry);
    }

    // =======================================================================
    // IAC -- Increment Accumulator
    // =======================================================================

    #[test]
    fn iac_increments() {
        let sim = run_program(&[encode_ldm(4), encode_iac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 5);
        assert!(!sim.carry);
    }

    #[test]
    fn iac_wraps_and_sets_carry() {
        let sim = run_program(&[encode_ldm(15), encode_iac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(sim.carry);
    }

    // =======================================================================
    // CMC -- Complement Carry
    // =======================================================================

    #[test]
    fn cmc_toggles_carry() {
        let sim = run_program(&[encode_cmc(), encode_hlt()]);
        assert!(sim.carry); // was false, now true

        let sim = run_program(&[encode_stc(), encode_cmc(), encode_hlt()]);
        assert!(!sim.carry); // was true, now false
    }

    // =======================================================================
    // CMA -- Complement Accumulator
    // =======================================================================

    #[test]
    fn cma_complements() {
        // A=5 (0101) -> complement = 10 (1010)
        let sim = run_program(&[encode_ldm(5), encode_cma(), encode_hlt()]);
        assert_eq!(sim.accumulator, 10);
    }

    #[test]
    fn cma_zero() {
        let sim = run_program(&[encode_ldm(0), encode_cma(), encode_hlt()]);
        assert_eq!(sim.accumulator, 15);
    }

    // =======================================================================
    // RAL -- Rotate Accumulator Left through Carry
    // =======================================================================

    #[test]
    fn ral_basic() {
        // A=0b0101 (5), carry=false
        // After RAL: carry=0 (bit3 was 0), A=0b1010 (10)
        let sim = run_program(&[encode_ldm(5), encode_ral(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b1010); // 10
        assert!(!sim.carry);
    }

    #[test]
    fn ral_carry_in() {
        // A=0b0101 (5), carry=true
        // After RAL: carry=0 (bit3 was 0), A=0b1011 (11)
        let sim = run_program(&[encode_ldm(5), encode_stc(), encode_ral(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b1011); // 11
        assert!(!sim.carry);
    }

    #[test]
    fn ral_carry_out() {
        // A=0b1000 (8), carry=false
        // After RAL: carry=1 (bit3 was 1), A=0b0000 (0)
        let sim = run_program(&[encode_ldm(8), encode_ral(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(sim.carry);
    }

    // =======================================================================
    // RAR -- Rotate Accumulator Right through Carry
    // =======================================================================

    #[test]
    fn rar_basic() {
        // A=0b0110 (6), carry=false
        // After RAR: carry=0 (bit0 was 0), A=0b0011 (3)
        let sim = run_program(&[encode_ldm(6), encode_rar(), encode_hlt()]);
        assert_eq!(sim.accumulator, 3);
        assert!(!sim.carry);
    }

    #[test]
    fn rar_carry_in() {
        // A=0b0110 (6), carry=true
        // After RAR: carry=0, A=0b1011 (11) -- old carry goes to bit3
        let sim = run_program(&[encode_ldm(6), encode_stc(), encode_rar(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0b1011); // 11
        assert!(!sim.carry);
    }

    #[test]
    fn rar_carry_out() {
        // A=0b0001 (1), carry=false
        // After RAR: carry=1 (bit0 was 1), A=0b0000 (0)
        let sim = run_program(&[encode_ldm(1), encode_rar(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(sim.carry);
    }

    // =======================================================================
    // TCC -- Transfer Carry to Accumulator, Clear Carry
    // =======================================================================

    #[test]
    fn tcc_carry_set() {
        let sim = run_program(&[encode_stc(), encode_tcc(), encode_hlt()]);
        assert_eq!(sim.accumulator, 1);
        assert!(!sim.carry);
    }

    #[test]
    fn tcc_carry_clear() {
        let sim = run_program(&[encode_tcc(), encode_hlt()]);
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }

    // =======================================================================
    // DAC -- Decrement Accumulator
    // =======================================================================

    #[test]
    fn dac_basic() {
        let sim = run_program(&[encode_ldm(5), encode_dac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 4);
        assert!(sim.carry, "No borrow: carry should be true");
    }

    #[test]
    fn dac_wraps_and_clears_carry() {
        let sim = run_program(&[encode_ldm(0), encode_dac(), encode_hlt()]);
        assert_eq!(sim.accumulator, 15);
        assert!(!sim.carry, "Borrow: carry should be false");
    }

    // =======================================================================
    // TCS -- Transfer Carry Subtract
    // =======================================================================

    #[test]
    fn tcs_carry_set() {
        let sim = run_program(&[encode_stc(), encode_tcs(), encode_hlt()]);
        assert_eq!(sim.accumulator, 10);
        assert!(!sim.carry);
    }

    #[test]
    fn tcs_carry_clear() {
        let sim = run_program(&[encode_tcs(), encode_hlt()]);
        assert_eq!(sim.accumulator, 9);
        assert!(!sim.carry);
    }

    // =======================================================================
    // STC -- Set Carry
    // =======================================================================

    #[test]
    fn stc_sets_carry() {
        let sim = run_program(&[encode_stc(), encode_hlt()]);
        assert!(sim.carry);
    }

    // =======================================================================
    // DAA -- Decimal Adjust Accumulator
    // =======================================================================

    #[test]
    fn daa_no_adjustment_needed() {
        // A=5, carry=false: no adjustment (5 <= 9 and no carry)
        let sim = run_program(&[encode_ldm(5), encode_daa(), encode_hlt()]);
        assert_eq!(sim.accumulator, 5);
        assert!(!sim.carry);
    }

    #[test]
    fn daa_adjustment_on_overflow() {
        // Simulate BCD: 8 + 5 = 13 (binary), DAA should add 6 -> 19, A=3, carry=1
        let sim = run_program(&[
            encode_ldm(5), encode_xch(0),
            encode_ldm(8),
            encode_add(0),      // A = 8+5+0 = 13, carry=false
            encode_daa(),       // A > 9, add 6: 13+6=19, A=3, carry=true
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 3);
        assert!(sim.carry);
    }

    #[test]
    fn daa_with_carry_already_set() {
        // If carry is set (from previous BCD add), DAA adjusts even if A <= 9.
        // A=2, carry=true -> add 6: 2+6=8, no overflow.
        // Carry remains true because DAA only sets carry on overflow,
        // it doesn't clear existing carry.
        let sim = run_program(&[
            encode_ldm(2),
            encode_stc(),
            encode_daa(),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 8);
        assert!(sim.carry);
    }

    // =======================================================================
    // KBP -- Keyboard Process
    // =======================================================================

    #[test]
    fn kbp_valid_inputs() {
        // One-hot to binary position.
        let cases = [(0, 0), (1, 1), (2, 2), (4, 3), (8, 4)];
        for (input, expected) in cases {
            let sim = run_program(&[encode_ldm(input), encode_kbp(), encode_hlt()]);
            assert_eq!(sim.accumulator, expected, "KBP({}) should be {}", input, expected);
        }
    }

    #[test]
    fn kbp_invalid_inputs() {
        // Multiple bits set -> error (15).
        let invalids = [3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15];
        for input in invalids {
            let sim = run_program(&[encode_ldm(input), encode_kbp(), encode_hlt()]);
            assert_eq!(sim.accumulator, 15, "KBP({}) should be 15 (error)", input);
        }
    }

    // =======================================================================
    // DCL -- Designate Command Line
    // =======================================================================

    #[test]
    fn dcl_bank_selection() {
        for bank in 0..4u8 {
            let sim = run_program(&[encode_ldm(bank), encode_dcl(), encode_hlt()]);
            assert_eq!(sim.ram_bank, bank as usize);
        }
    }

    #[test]
    fn dcl_masks_to_valid_bank() {
        // A=7 (0b111) -> bank = 7 & 3 = 3 (since > 3, masked)
        let sim = run_program(&[encode_ldm(7), encode_dcl(), encode_hlt()]);
        assert_eq!(sim.ram_bank, 3);
    }

    // =======================================================================
    // WPM -- Write Program Memory (no-op in simulator)
    // =======================================================================

    #[test]
    fn wpm_is_noop() {
        let sim = run_program(&[encode_ldm(5), encode_wpm(), encode_hlt()]);
        assert_eq!(sim.accumulator, 5); // unchanged
    }

    // =======================================================================
    // Reset
    // =======================================================================

    #[test]
    fn reset_clears_all_state() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.accumulator = 15;
        sim.carry = true;
        sim.registers[0] = 7;
        sim.pc = 100;
        sim.halted = true;
        sim.hw_stack[0] = 0xABC;
        sim.stack_pointer = 2;
        sim.ram[1][2][3] = 5;
        sim.ram_status[1][2][1] = 8;
        sim.ram_output[1] = 3;
        sim.ram_bank = 2;
        sim.ram_register = 3;
        sim.ram_character = 7;
        sim.rom_port = 12;

        sim.reset();

        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
        assert_eq!(sim.registers[0], 0);
        assert_eq!(sim.pc, 0);
        assert!(!sim.halted);
        assert_eq!(sim.hw_stack, [0; 3]);
        assert_eq!(sim.stack_pointer, 0);
        assert_eq!(sim.ram[1][2][3], 0);
        assert_eq!(sim.ram_status[1][2][1], 0);
        assert_eq!(sim.ram_output[1], 0);
        assert_eq!(sim.ram_bank, 0);
        assert_eq!(sim.ram_register, 0);
        assert_eq!(sim.ram_character, 0);
        assert_eq!(sim.rom_port, 0);
    }

    // =======================================================================
    // Trace verification
    // =======================================================================

    #[test]
    fn trace_records_raw2_for_two_byte_instructions() {
        let (b1, b2) = encode_jun(0x004);
        let mut sim = Intel4004Simulator::new(4096);
        sim.load_program(&[b1, b2, 0, 0, encode_hlt()]);
        let trace = sim.step();
        assert_eq!(trace.raw, b1);
        assert_eq!(trace.raw2, Some(b2));
    }

    #[test]
    fn trace_records_none_for_single_byte() {
        let mut sim = Intel4004Simulator::new(4096);
        sim.load_program(&[encode_ldm(5), encode_hlt()]);
        let trace = sim.step();
        assert_eq!(trace.raw2, None);
    }

    // =======================================================================
    // Two-byte detection
    // =======================================================================

    #[test]
    fn is_two_byte_detection() {
        // JCN (0x1_)
        assert!(is_two_byte(0x10));
        assert!(is_two_byte(0x1F));
        // FIM (0x2_, even)
        assert!(is_two_byte(0x20));
        assert!(is_two_byte(0x2E));
        // SRC (0x2_, odd) -- NOT two-byte
        assert!(!is_two_byte(0x21));
        assert!(!is_two_byte(0x2F));
        // JUN (0x4_)
        assert!(is_two_byte(0x40));
        // JMS (0x5_)
        assert!(is_two_byte(0x50));
        // ISZ (0x7_)
        assert!(is_two_byte(0x70));
        // Single-byte instructions
        assert!(!is_two_byte(0x00)); // NOP
        assert!(!is_two_byte(0x60)); // INC
        assert!(!is_two_byte(0xD0)); // LDM
        assert!(!is_two_byte(0xF0)); // CLB
    }

    // =======================================================================
    // Unknown instructions
    // =======================================================================

    #[test]
    fn unknown_instruction_produces_mnemonic() {
        // 0xFE and 0xFF are not defined instructions
        let mut sim = Intel4004Simulator::new(4096);
        sim.load_program(&[0xFE, encode_hlt()]);
        let trace = sim.step();
        assert!(trace.mnemonic.contains("UNKNOWN"));
    }

    // =======================================================================
    // End-to-end programs
    // =======================================================================

    /// Compute x = 1 + 2 using accumulator architecture.
    ///
    /// Steps: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
    /// This takes 6 instructions vs 4 on a register machine (RISC-V),
    /// illustrating the accumulator architecture's verbosity.
    #[test]
    fn program_add_1_plus_2() {
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

    /// Count down from 5 to 0 using DAC in a loop.
    ///
    /// This demonstrates:
    /// - DAC (decrement accumulator)
    /// - JCN with invert+zero test (jump while A != 0)
    /// - A simple loop pattern
    #[test]
    fn program_countdown() {
        // Start A=5, decrement in a loop until A=0.
        //
        // addr 0: LDM 5       -- A=5
        // addr 1: DAC          -- A = A-1
        // addr 2-3: JCN 0xC, 0x01  -- if A != 0, jump to addr 1
        // addr 4: HLT
        let (jcn_b1, jcn_b2) = encode_jcn(0xC, 0x01); // invert+test_zero: jump if NOT zero
        let sim = run_program(&[
            encode_ldm(5),
            encode_dac(),
            jcn_b1, jcn_b2,
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 0);
    }

    /// BCD addition of 8 + 5 = 13 (BCD: carry=1, digit=3).
    ///
    /// Demonstrates the DAA instruction for decimal arithmetic.
    #[test]
    fn program_bcd_addition() {
        let sim = run_program(&[
            encode_ldm(5), encode_xch(0),   // R0 = 5
            encode_ldm(8),                   // A = 8
            encode_clc(),                    // clear carry for clean add
            encode_add(0),                   // A = 8+5 = 13 (binary)
            encode_daa(),                    // DAA: 13+6 = 19, A=3, carry=1
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 3);
        assert!(sim.carry);
    }

    /// Subroutine that doubles a value.
    ///
    /// Main: load 6 into R0, call double subroutine, check result in R1.
    #[test]
    fn program_double_subroutine() {
        let mut program = vec![0u8; 256];

        // Main: load 6, call double at 0x010, halt.
        program[0] = encode_ldm(6);      // A = 6
        program[1] = encode_xch(0);      // R0 = 6
        let (b1, b2) = encode_jms(0x010);
        program[2] = b1;
        program[3] = b2;                  // JMS 0x010
        // After return, load R1 to get the result.
        program[4] = encode_ld(1);        // A = R1 = 12
        program[5] = encode_hlt();

        // Double subroutine at 0x010:
        // Expects input in R0. Stores R0 * 2 in R1.
        program[0x010] = encode_ld(0);    // A = R0 = 6
        program[0x011] = encode_add(0);   // A = 6+6 = 12
        program[0x012] = encode_xch(1);   // R1 = 12
        program[0x013] = encode_bbl(0);   // return, A=0

        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 12);
    }

    /// Store and retrieve multiple values from RAM.
    #[test]
    fn program_ram_array() {
        // Store values 1, 3, 5 in RAM characters 0, 1, 2, then read them back.
        let mut program = Vec::new();

        // Use FIM to set up different SRC addresses.
        let values = [1u8, 3, 5];
        for (i, &v) in values.iter().enumerate() {
            let (b1, b2) = encode_fim(0, i as u8); // register 0, character i
            program.push(b1);
            program.push(b2);
            program.push(encode_src(0));
            program.push(encode_ldm(v));
            program.push(encode_wrm());
        }

        // Read back character 1 (should be 3).
        let (b1, b2) = encode_fim(0, 1);
        program.push(b1);
        program.push(b2);
        program.push(encode_src(0));
        program.push(encode_rdm());
        program.push(encode_hlt());

        let sim = run_program(&program);
        assert_eq!(sim.accumulator, 3);
    }

    /// ISZ-based loop that sums 1+2+3.
    #[test]
    fn program_isz_loop_sum() {
        // Use R0 as loop counter (start at 13, wraps to 0 after 3 increments).
        // Use R1 to track which iteration we're on (1, 2, 3).
        // Accumulate sum in R2.
        let (fim_b1, fim_b2) = encode_fim(0, (13 << 4) | 0);
        let (isz_b1, isz_b2) = encode_isz(0, 0x04);
        let sim = run_program(&[
            fim_b1, fim_b2,     // R0=13, R1=0
            encode_ldm(0),      // A=0
            encode_xch(2),      // R2=0 (sum)
            encode_inc(1),      // R1++ (iteration)
            encode_ld(2),       // A = sum
            encode_add(1),      // A = sum + iteration
            encode_xch(2),      // R2 = new sum
            isz_b1, isz_b2,    // R0++, jump to 0x04 if R0 != 0
            encode_ld(2),       // A = final sum
            encode_hlt(),
        ]);
        // R0 goes 13->14->15->0: three iterations.
        // R1 goes 0->1->2->3.
        // Sum = 1 + 2 + 3 = 6.
        assert_eq!(sim.accumulator, 6);
    }

    /// Test the register pair read/write helpers.
    #[test]
    fn register_pair_operations() {
        let (b1, b2) = encode_fim(3, 0xDE);
        let sim = run_program(&[b1, b2, encode_hlt()]);
        // Pair 3 = (R6, R7)
        assert_eq!(sim.registers[6], 0xD);
        assert_eq!(sim.registers[7], 0xE);
    }

    /// Rotate left twice: equivalent to shift left 2 (with carry chain).
    #[test]
    fn double_rotate_left() {
        // A=0b0001 (1), carry=false
        // RAL 1: carry=0, A=0b0010 (2)
        // RAL 2: carry=0, A=0b0100 (4)
        let sim = run_program(&[
            encode_ldm(1),
            encode_ral(),
            encode_ral(),
            encode_hlt(),
        ]);
        assert_eq!(sim.accumulator, 4);
        assert!(!sim.carry);
    }

    /// Test all KBP conversions exhaustively.
    #[test]
    fn kbp_exhaustive() {
        let expected = [0, 1, 2, 15, 3, 15, 15, 15, 4, 15, 15, 15, 15, 15, 15, 15];
        for input in 0..16u8 {
            let sim = run_program(&[encode_ldm(input), encode_kbp(), encode_hlt()]);
            assert_eq!(
                sim.accumulator, expected[input as usize],
                "KBP({}) = {}, expected {}", input, sim.accumulator, expected[input as usize]
            );
        }
    }

    /// Test stack wrapping: 4 pushes into a 3-level stack.
    #[test]
    fn stack_wraps_on_overflow() {
        // Call 4 subroutines without returning -- the first return address is lost.
        let mut program = vec![0u8; 256];

        // Sub at 0x10: calls 0x20
        let (b1, b2) = encode_jms(0x20);
        program[0x10] = b1;
        program[0x11] = b2;
        program[0x12] = encode_bbl(0);

        // Sub at 0x20: calls 0x30
        let (b1, b2) = encode_jms(0x30);
        program[0x20] = b1;
        program[0x21] = b2;
        program[0x22] = encode_bbl(0);

        // Sub at 0x30: calls 0x40
        let (b1, b2) = encode_jms(0x40);
        program[0x30] = b1;
        program[0x31] = b2;
        program[0x32] = encode_bbl(0);

        // Sub at 0x40: just returns
        program[0x40] = encode_bbl(0);

        // Main at 0x00: calls 0x10
        let (b1, b2) = encode_jms(0x10);
        program[0x00] = b1;
        program[0x01] = b2;
        program[0x02] = encode_hlt();

        // This nests 4 deep: main->0x10->0x20->0x30->0x40
        // Stack has 3 slots, so the main return address (0x02) may be lost.
        // We just verify it doesn't crash.
        let mut sim = Intel4004Simulator::new(4096);
        sim.run(&program, 100);
    }

    /// Encoding roundtrip: all encoder functions produce expected byte patterns.
    #[test]
    fn encoding_roundtrip() {
        assert_eq!(encode_nop(), 0x00);
        assert_eq!(encode_hlt(), 0x01);
        assert_eq!(encode_ldm(5), 0xD5);
        assert_eq!(encode_ld(3), 0xA3);
        assert_eq!(encode_xch(7), 0xB7);
        assert_eq!(encode_add(2), 0x82);
        assert_eq!(encode_sub(4), 0x94);
        assert_eq!(encode_inc(6), 0x66);
        assert_eq!(encode_bbl(1), 0xC1);

        let (b1, b2) = encode_jcn(0x4, 0x10);
        assert_eq!(b1, 0x14);
        assert_eq!(b2, 0x10);

        let (b1, b2) = encode_fim(2, 0xAB);
        assert_eq!(b1, 0x24);
        assert_eq!(b2, 0xAB);

        assert_eq!(encode_src(1), 0x23);

        assert_eq!(encode_fin(3), 0x36);
        assert_eq!(encode_jin(3), 0x37);

        let (b1, b2) = encode_jun(0x123);
        assert_eq!(b1, 0x41);
        assert_eq!(b2, 0x23);

        let (b1, b2) = encode_jms(0x456);
        assert_eq!(b1, 0x54);
        assert_eq!(b2, 0x56);

        let (b1, b2) = encode_isz(5, 0x20);
        assert_eq!(b1, 0x75);
        assert_eq!(b2, 0x20);

        // I/O encoders
        assert_eq!(encode_wrm(), 0xE0);
        assert_eq!(encode_wmp(), 0xE1);
        assert_eq!(encode_wrr(), 0xE2);
        assert_eq!(encode_wpm(), 0xE3);
        assert_eq!(encode_wr0(), 0xE4);
        assert_eq!(encode_wr1(), 0xE5);
        assert_eq!(encode_wr2(), 0xE6);
        assert_eq!(encode_wr3(), 0xE7);
        assert_eq!(encode_sbm(), 0xE8);
        assert_eq!(encode_rdm(), 0xE9);
        assert_eq!(encode_rdr(), 0xEA);
        assert_eq!(encode_adm(), 0xEB);
        assert_eq!(encode_rd0(), 0xEC);
        assert_eq!(encode_rd1(), 0xED);
        assert_eq!(encode_rd2(), 0xEE);
        assert_eq!(encode_rd3(), 0xEF);

        // Accumulator group encoders
        assert_eq!(encode_clb(), 0xF0);
        assert_eq!(encode_clc(), 0xF1);
        assert_eq!(encode_iac(), 0xF2);
        assert_eq!(encode_cmc(), 0xF3);
        assert_eq!(encode_cma(), 0xF4);
        assert_eq!(encode_ral(), 0xF5);
        assert_eq!(encode_rar(), 0xF6);
        assert_eq!(encode_tcc(), 0xF7);
        assert_eq!(encode_dac(), 0xF8);
        assert_eq!(encode_tcs(), 0xF9);
        assert_eq!(encode_stc(), 0xFA);
        assert_eq!(encode_daa(), 0xFB);
        assert_eq!(encode_kbp(), 0xFC);
        assert_eq!(encode_dcl(), 0xFD);
    }

    /// Verify run() calls reset() so sequential programs start clean.
    #[test]
    fn run_resets_between_programs() {
        let mut sim = Intel4004Simulator::new(4096);

        // First program: set A=15, carry=true
        sim.run(&[encode_ldm(15), encode_stc(), encode_hlt()], 10);
        assert_eq!(sim.accumulator, 15);
        assert!(sim.carry);

        // Second program: should start with A=0, carry=false
        sim.run(&[encode_hlt()], 10);
        assert_eq!(sim.accumulator, 0);
        assert!(!sim.carry);
    }
}
