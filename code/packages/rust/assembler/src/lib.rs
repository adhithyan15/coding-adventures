//! # Assembler -- ARM assembly parser and binary encoder.
//!
//! An **assembler** is the bridge between human-readable assembly language and
//! machine-executable binary code. It performs two key tasks:
//!
//! 1. **Parsing**: Read assembly source text and break it into structured
//!    instructions (mnemonic, operands, labels, directives).
//!
//! 2. **Encoding**: Convert each structured instruction into the binary format
//!    that the target CPU expects.
//!
//! ## Why ARM?
//!
//! ARM is one of the most widely deployed instruction set architectures (ISAs)
//! in the world. Every smartphone, most tablets, the Apple M-series chips, and
//! billions of embedded devices run ARM code. Understanding ARM assembly gives
//! insight into how real processors work.
//!
//! ## ARM instruction encoding
//!
//! ARM instructions are 32 bits (4 bytes) wide, with a very regular structure.
//! The top 4 bits encode the **condition code** (execute always, execute if zero,
//! etc.), and the remaining 28 bits encode the operation and operands.
//!
//! ```text
//! 31-28  27-26  25    24-21   20    19-16  15-12  11-0
//! [cond] [00]   [I]   [opcode][S]   [Rn]   [Rd]   [operand2]
//! ```
//!
//! - **cond**: Condition code (0xE = always execute).
//! - **I**: Immediate flag (1 = operand2 is an immediate value).
//! - **opcode**: The actual operation (ADD=0100, SUB=0010, MOV=1101, etc.).
//! - **S**: Set condition flags (1 = update CPSR).
//! - **Rn**: First source register.
//! - **Rd**: Destination register.
//! - **operand2**: Second operand (register or immediate).
//!
//! ## Supported instructions
//!
//! | Mnemonic | Example            | Description                     |
//! |----------|--------------------|---------------------------------|
//! | MOV      | `MOV R0, #42`      | Move immediate to register      |
//! | ADD      | `ADD R2, R0, R1`   | Add two registers               |
//! | SUB      | `SUB R2, R0, R1`   | Subtract two registers          |
//! | AND      | `AND R2, R0, R1`   | Bitwise AND                     |
//! | ORR      | `ORR R2, R0, R1`   | Bitwise OR                      |
//! | CMP      | `CMP R0, R1`       | Compare (sets flags, no result) |
//! | LDR      | `LDR R0, [R1]`     | Load from memory                |
//! | STR      | `STR R0, [R1]`     | Store to memory                 |
//! | NOP      | `NOP`              | No operation                    |

use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// Register representation
// ===========================================================================

/// Parse a register name like "R0", "R1", ..., "R15" into its numeric index.
///
/// ARM has 16 general-purpose registers (R0-R15). Some have special roles:
/// - R13 = SP (Stack Pointer)
/// - R14 = LR (Link Register -- return address for function calls)
/// - R15 = PC (Program Counter)
///
/// We accept both forms: "R0" and "SP" (etc.) are valid.
fn parse_register(s: &str) -> Option<u32> {
    let s = s.trim().to_uppercase();
    match s.as_str() {
        "SP" => Some(13),
        "LR" => Some(14),
        "PC" => Some(15),
        _ => {
            if s.starts_with('R') {
                s[1..].parse::<u32>().ok().filter(|&n| n <= 15)
            } else {
                None
            }
        }
    }
}

/// Parse an immediate value like "#42" or "#0xFF" into a u32.
///
/// Immediate values in ARM assembly are prefixed with `#`. They can be
/// decimal (`#42`) or hexadecimal (`#0xFF`).
fn parse_immediate(s: &str) -> Option<u32> {
    let s = s.trim();
    let s = if let Some(stripped) = s.strip_prefix('#') {
        stripped.trim()
    } else {
        s
    };
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

// ===========================================================================
// ARM opcodes
// ===========================================================================

/// ARM data processing opcodes (bits 24-21 of the instruction word).
///
/// These are the 4-bit values that the ARM processor uses to identify
/// which operation to perform. The values here match the ARM Architecture
/// Reference Manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmOpcode {
    /// Bitwise AND: Rd = Rn AND operand2.
    And = 0x0,
    /// Bitwise exclusive OR: Rd = Rn XOR operand2.
    Eor = 0x1,
    /// Subtract: Rd = Rn - operand2.
    Sub = 0x2,
    /// Reverse subtract: Rd = operand2 - Rn.
    Rsb = 0x3,
    /// Add: Rd = Rn + operand2.
    Add = 0x4,
    /// Compare: sets flags for Rn - operand2, discards result.
    Cmp = 0xA,
    /// Bitwise OR: Rd = Rn OR operand2.
    Orr = 0xC,
    /// Move: Rd = operand2 (Rn is ignored).
    Mov = 0xD,
}

/// Look up an ARM opcode from its mnemonic string.
fn mnemonic_to_opcode(mnemonic: &str) -> Option<ArmOpcode> {
    match mnemonic.to_uppercase().as_str() {
        "AND" => Some(ArmOpcode::And),
        "EOR" => Some(ArmOpcode::Eor),
        "SUB" => Some(ArmOpcode::Sub),
        "RSB" => Some(ArmOpcode::Rsb),
        "ADD" => Some(ArmOpcode::Add),
        "CMP" => Some(ArmOpcode::Cmp),
        "ORR" => Some(ArmOpcode::Orr),
        "MOV" => Some(ArmOpcode::Mov),
        _ => None,
    }
}

// ===========================================================================
// Parsed instruction
// ===========================================================================

/// A parsed but not yet encoded ARM instruction.
///
/// This is the intermediate representation between source text and binary.
/// The assembler first parses each line into an `ArmInstruction`, then
/// encodes it into a 32-bit binary word.
#[derive(Debug, Clone, PartialEq)]
pub enum ArmInstruction {
    /// A data processing instruction (ADD, SUB, MOV, CMP, AND, ORR, etc.).
    DataProcessing {
        /// The operation to perform.
        opcode: ArmOpcode,
        /// Destination register (0-15). None for CMP (no destination).
        rd: Option<u32>,
        /// First source register (0-15). None for MOV.
        rn: Option<u32>,
        /// Second operand: either a register index or an immediate value.
        operand2: Operand2,
        /// Whether to update condition flags (the 'S' suffix).
        set_flags: bool,
    },
    /// Load from memory: LDR Rd, [Rn].
    Load {
        /// Destination register.
        rd: u32,
        /// Base address register.
        rn: u32,
    },
    /// Store to memory: STR Rd, [Rn].
    Store {
        /// Source register.
        rd: u32,
        /// Base address register.
        rn: u32,
    },
    /// No operation -- does nothing. Encoded as `MOV R0, R0`.
    Nop,
    /// A label (e.g., `loop:`). Not an instruction, but a named address.
    Label(String),
}

/// The second operand of a data processing instruction.
///
/// ARM instructions can take either a register or an immediate value
/// as their second operand. The `I` bit (bit 25) in the instruction
/// encoding distinguishes between the two.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand2 {
    /// A register operand (e.g., R1).
    Register(u32),
    /// An immediate value (e.g., #42).
    Immediate(u32),
}

// ===========================================================================
// Assembler errors
// ===========================================================================

/// An error that occurs during assembly.
#[derive(Debug, Clone, PartialEq)]
pub enum AssemblerError {
    /// Unrecognized mnemonic.
    UnknownMnemonic(String),
    /// Invalid register name.
    InvalidRegister(String),
    /// Invalid immediate value.
    InvalidImmediate(String),
    /// Wrong number of operands for the instruction.
    InvalidOperandCount { mnemonic: String, expected: usize, got: usize },
    /// General parse error.
    ParseError(String),
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssemblerError::UnknownMnemonic(m) => write!(f, "Unknown mnemonic: {}", m),
            AssemblerError::InvalidRegister(r) => write!(f, "Invalid register: {}", r),
            AssemblerError::InvalidImmediate(v) => write!(f, "Invalid immediate: {}", v),
            AssemblerError::InvalidOperandCount { mnemonic, expected, got } => {
                write!(
                    f,
                    "{}: expected {} operands, got {}",
                    mnemonic, expected, got
                )
            }
            AssemblerError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for AssemblerError {}

// ===========================================================================
// Assembler
// ===========================================================================

/// An ARM assembler that parses assembly source and encodes it to binary.
///
/// ## Usage
///
/// ```
/// use assembler::Assembler;
///
/// let mut asm = Assembler::new();
/// let instructions = asm.parse("MOV R0, #42\nADD R2, R0, R1").unwrap();
/// let binary = asm.encode(&instructions).unwrap();
/// assert_eq!(binary.len(), 2); // Two 32-bit words
/// ```
pub struct Assembler {
    /// Label-to-address mapping for branch resolution.
    pub labels: HashMap<String, usize>,
}

impl Assembler {
    /// Create a new assembler.
    pub fn new() -> Self {
        Assembler {
            labels: HashMap::new(),
        }
    }

    /// Parse assembly source into a list of instructions.
    ///
    /// This is the first pass. It reads each line, strips comments and
    /// whitespace, and converts the text into structured [`ArmInstruction`]s.
    /// Labels are recorded in the label table for later branch resolution.
    pub fn parse(&mut self, source: &str) -> Result<Vec<ArmInstruction>, AssemblerError> {
        let mut instructions = Vec::new();
        let mut address = 0;

        for line in source.lines() {
            // Strip comments (everything after `;` or `//`).
            let line = line.split(';').next().unwrap_or("");
            let line = line.split("//").next().unwrap_or("");
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            // Check for labels (lines ending with `:` or just `label:`).
            if let Some(label) = line.strip_suffix(':') {
                let label = label.trim().to_string();
                self.labels.insert(label.clone(), address);
                instructions.push(ArmInstruction::Label(label));
                continue;
            }

            let instr = self.parse_instruction(line)?;
            // Labels don't count as instructions for address calculation.
            if !matches!(instr, ArmInstruction::Label(_)) {
                address += 1;
            }
            instructions.push(instr);
        }

        Ok(instructions)
    }

    /// Parse a single instruction line.
    fn parse_instruction(&self, line: &str) -> Result<ArmInstruction, AssemblerError> {
        // Split into mnemonic and operands.
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let mnemonic = parts[0].trim().to_uppercase();
        let operands_str = if parts.len() > 1 { parts[1].trim() } else { "" };

        match mnemonic.as_str() {
            "NOP" => Ok(ArmInstruction::Nop),

            "MOV" | "MOVS" => {
                let set_flags = mnemonic.ends_with('S') && mnemonic != "MOVS" || mnemonic == "MOVS";
                let operands = self.split_operands(operands_str);
                if operands.len() != 2 {
                    return Err(AssemblerError::InvalidOperandCount {
                        mnemonic,
                        expected: 2,
                        got: operands.len(),
                    });
                }
                let rd = parse_register(operands[0])
                    .ok_or_else(|| AssemblerError::InvalidRegister(operands[0].to_string()))?;
                let operand2 = self.parse_operand2(operands[1])?;
                Ok(ArmInstruction::DataProcessing {
                    opcode: ArmOpcode::Mov,
                    rd: Some(rd),
                    rn: None,
                    operand2,
                    set_flags: mnemonic == "MOVS",
                })
            }

            "ADD" | "ADDS" | "SUB" | "SUBS" | "AND" | "ANDS" | "ORR" | "ORRS" | "EOR"
            | "EORS" | "RSB" | "RSBS" => {
                let base = mnemonic.trim_end_matches('S');
                let set_flags = mnemonic.len() > base.len();
                let opcode = mnemonic_to_opcode(base)
                    .ok_or_else(|| AssemblerError::UnknownMnemonic(mnemonic.clone()))?;
                let operands = self.split_operands(operands_str);
                if operands.len() != 3 {
                    return Err(AssemblerError::InvalidOperandCount {
                        mnemonic,
                        expected: 3,
                        got: operands.len(),
                    });
                }
                let rd = parse_register(operands[0])
                    .ok_or_else(|| AssemblerError::InvalidRegister(operands[0].to_string()))?;
                let rn = parse_register(operands[1])
                    .ok_or_else(|| AssemblerError::InvalidRegister(operands[1].to_string()))?;
                let operand2 = self.parse_operand2(operands[2])?;
                Ok(ArmInstruction::DataProcessing {
                    opcode,
                    rd: Some(rd),
                    rn: Some(rn),
                    operand2,
                    set_flags,
                })
            }

            "CMP" => {
                let operands = self.split_operands(operands_str);
                if operands.len() != 2 {
                    return Err(AssemblerError::InvalidOperandCount {
                        mnemonic,
                        expected: 2,
                        got: operands.len(),
                    });
                }
                let rn = parse_register(operands[0])
                    .ok_or_else(|| AssemblerError::InvalidRegister(operands[0].to_string()))?;
                let operand2 = self.parse_operand2(operands[1])?;
                Ok(ArmInstruction::DataProcessing {
                    opcode: ArmOpcode::Cmp,
                    rd: None,
                    rn: Some(rn),
                    operand2,
                    set_flags: true, // CMP always sets flags.
                })
            }

            "LDR" => {
                let operands = self.split_operands(operands_str);
                if operands.len() != 2 {
                    return Err(AssemblerError::InvalidOperandCount {
                        mnemonic,
                        expected: 2,
                        got: operands.len(),
                    });
                }
                let rd = parse_register(operands[0])
                    .ok_or_else(|| AssemblerError::InvalidRegister(operands[0].to_string()))?;
                // Parse [Rn] form.
                let base = operands[1]
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim();
                let rn = parse_register(base)
                    .ok_or_else(|| AssemblerError::InvalidRegister(base.to_string()))?;
                Ok(ArmInstruction::Load { rd, rn })
            }

            "STR" => {
                let operands = self.split_operands(operands_str);
                if operands.len() != 2 {
                    return Err(AssemblerError::InvalidOperandCount {
                        mnemonic,
                        expected: 2,
                        got: operands.len(),
                    });
                }
                let rd = parse_register(operands[0])
                    .ok_or_else(|| AssemblerError::InvalidRegister(operands[0].to_string()))?;
                let base = operands[1]
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim();
                let rn = parse_register(base)
                    .ok_or_else(|| AssemblerError::InvalidRegister(base.to_string()))?;
                Ok(ArmInstruction::Store { rd, rn })
            }

            _ => Err(AssemblerError::UnknownMnemonic(mnemonic)),
        }
    }

    /// Split an operand string by commas, respecting brackets.
    fn split_operands<'a>(&self, s: &'a str) -> Vec<&'a str> {
        if s.is_empty() {
            return vec![];
        }
        s.split(',').map(|p| p.trim()).collect()
    }

    /// Parse an operand2 value (register or immediate).
    fn parse_operand2(&self, s: &str) -> Result<Operand2, AssemblerError> {
        let s = s.trim();
        if s.starts_with('#') {
            let val = parse_immediate(s)
                .ok_or_else(|| AssemblerError::InvalidImmediate(s.to_string()))?;
            Ok(Operand2::Immediate(val))
        } else if let Some(reg) = parse_register(s) {
            Ok(Operand2::Register(reg))
        } else {
            Err(AssemblerError::ParseError(format!(
                "Cannot parse operand: {}",
                s
            )))
        }
    }

    /// Encode a list of parsed instructions into 32-bit binary words.
    ///
    /// This is the second pass. Each [`ArmInstruction`] is converted into
    /// its 32-bit binary encoding according to the ARM instruction format.
    pub fn encode(&self, instructions: &[ArmInstruction]) -> Result<Vec<u32>, AssemblerError> {
        let mut binary = Vec::new();

        for instr in instructions {
            match instr {
                ArmInstruction::Label(_) => {
                    // Labels don't produce binary output.
                }

                ArmInstruction::Nop => {
                    // NOP is encoded as MOV R0, R0 (which does nothing).
                    // Condition=AL(0xE), I=0, opcode=MOV(0xD), S=0, Rn=0, Rd=0, operand2=R0
                    let word = 0xE1A00000u32; // Standard ARM NOP encoding.
                    binary.push(word);
                }

                ArmInstruction::DataProcessing {
                    opcode,
                    rd,
                    rn,
                    operand2,
                    set_flags,
                } => {
                    let cond: u32 = 0xE; // Always execute.
                    let rd_val = rd.unwrap_or(0);
                    let rn_val = rn.unwrap_or(0);
                    let s_bit: u32 = if *set_flags { 1 } else { 0 };
                    let opcode_val = *opcode as u32;

                    let (i_bit, op2_val) = match operand2 {
                        Operand2::Immediate(imm) => (1u32, *imm & 0xFFF),
                        Operand2::Register(reg) => (0u32, *reg & 0xF),
                    };

                    // Assemble the 32-bit instruction word:
                    // [31-28: cond] [27-26: 00] [25: I] [24-21: opcode]
                    // [20: S] [19-16: Rn] [15-12: Rd] [11-0: operand2]
                    let word = (cond << 28)
                        | (i_bit << 25)
                        | (opcode_val << 21)
                        | (s_bit << 20)
                        | (rn_val << 16)
                        | (rd_val << 12)
                        | op2_val;

                    binary.push(word);
                }

                ArmInstruction::Load { rd, rn } => {
                    // LDR encoding: condition=AL, [27-26]=01, [24]=1 (pre-indexed),
                    // [23]=1 (add offset), [22]=0 (word), [21]=0 (no writeback),
                    // [20]=1 (load), Rn, Rd, offset=0.
                    let word = 0xE5900000u32 | ((*rn) << 16) | ((*rd) << 12);
                    binary.push(word);
                }

                ArmInstruction::Store { rd, rn } => {
                    // STR encoding: same as LDR but [20]=0 (store).
                    let word = 0xE5800000u32 | ((*rn) << 16) | ((*rd) << 12);
                    binary.push(word);
                }
            }
        }

        Ok(binary)
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Register parsing
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_register() {
        assert_eq!(parse_register("R0"), Some(0));
        assert_eq!(parse_register("R15"), Some(15));
        assert_eq!(parse_register("SP"), Some(13));
        assert_eq!(parse_register("LR"), Some(14));
        assert_eq!(parse_register("PC"), Some(15));
        assert_eq!(parse_register("R16"), None); // Out of range.
        assert_eq!(parse_register("X0"), None);  // Not a register.
    }

    #[test]
    fn test_parse_register_case_insensitive() {
        assert_eq!(parse_register("r0"), Some(0));
        assert_eq!(parse_register("sp"), Some(13));
    }

    // -----------------------------------------------------------------------
    // Immediate parsing
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_immediate_decimal() {
        assert_eq!(parse_immediate("#42"), Some(42));
        assert_eq!(parse_immediate("#0"), Some(0));
        assert_eq!(parse_immediate("#255"), Some(255));
    }

    #[test]
    fn test_parse_immediate_hex() {
        assert_eq!(parse_immediate("#0xFF"), Some(255));
        assert_eq!(parse_immediate("#0x10"), Some(16));
    }

    #[test]
    fn test_parse_immediate_no_hash() {
        // Allow bare numbers (without #).
        assert_eq!(parse_immediate("42"), Some(42));
    }

    // -----------------------------------------------------------------------
    // Instruction parsing
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_mov_immediate() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("MOV R0, #42").unwrap();
        assert_eq!(instructions.len(), 1);
        match &instructions[0] {
            ArmInstruction::DataProcessing {
                opcode,
                rd,
                operand2,
                ..
            } => {
                assert_eq!(*opcode, ArmOpcode::Mov);
                assert_eq!(*rd, Some(0));
                assert_eq!(*operand2, Operand2::Immediate(42));
            }
            _ => panic!("Expected DataProcessing"),
        }
    }

    #[test]
    fn test_parse_add_registers() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("ADD R2, R0, R1").unwrap();
        assert_eq!(instructions.len(), 1);
        match &instructions[0] {
            ArmInstruction::DataProcessing {
                opcode,
                rd,
                rn,
                operand2,
                ..
            } => {
                assert_eq!(*opcode, ArmOpcode::Add);
                assert_eq!(*rd, Some(2));
                assert_eq!(*rn, Some(0));
                assert_eq!(*operand2, Operand2::Register(1));
            }
            _ => panic!("Expected DataProcessing"),
        }
    }

    #[test]
    fn test_parse_sub() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("SUB R3, R1, R2").unwrap();
        match &instructions[0] {
            ArmInstruction::DataProcessing { opcode, .. } => {
                assert_eq!(*opcode, ArmOpcode::Sub);
            }
            _ => panic!("Expected DataProcessing"),
        }
    }

    #[test]
    fn test_parse_cmp() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("CMP R0, R1").unwrap();
        match &instructions[0] {
            ArmInstruction::DataProcessing {
                opcode,
                rd,
                set_flags,
                ..
            } => {
                assert_eq!(*opcode, ArmOpcode::Cmp);
                assert_eq!(*rd, None); // CMP has no destination.
                assert!(*set_flags);   // CMP always sets flags.
            }
            _ => panic!("Expected DataProcessing"),
        }
    }

    #[test]
    fn test_parse_ldr() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("LDR R0, [R1]").unwrap();
        match &instructions[0] {
            ArmInstruction::Load { rd, rn } => {
                assert_eq!(*rd, 0);
                assert_eq!(*rn, 1);
            }
            _ => panic!("Expected Load"),
        }
    }

    #[test]
    fn test_parse_str() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("STR R0, [R1]").unwrap();
        match &instructions[0] {
            ArmInstruction::Store { rd, rn } => {
                assert_eq!(*rd, 0);
                assert_eq!(*rn, 1);
            }
            _ => panic!("Expected Store"),
        }
    }

    #[test]
    fn test_parse_nop() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("NOP").unwrap();
        assert_eq!(instructions[0], ArmInstruction::Nop);
    }

    #[test]
    fn test_parse_label() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("loop:").unwrap();
        assert_eq!(
            instructions[0],
            ArmInstruction::Label("loop".to_string())
        );
        assert_eq!(asm.labels.get("loop"), Some(&0));
    }

    #[test]
    fn test_parse_comments_stripped() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("MOV R0, #1 ; load one").unwrap();
        assert_eq!(instructions.len(), 1);
    }

    #[test]
    fn test_parse_empty_lines_skipped() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("\n\nMOV R0, #1\n\n").unwrap();
        assert_eq!(instructions.len(), 1);
    }

    #[test]
    fn test_parse_multiple_instructions() {
        let mut asm = Assembler::new();
        let instructions = asm
            .parse("MOV R0, #10\nMOV R1, #20\nADD R2, R0, R1")
            .unwrap();
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_unknown_mnemonic() {
        let mut asm = Assembler::new();
        let result = asm.parse("BLAH R0, R1");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Binary encoding
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_mov_immediate() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("MOV R0, #42").unwrap();
        let binary = asm.encode(&instructions).unwrap();
        assert_eq!(binary.len(), 1);

        // Verify the encoding manually:
        // cond=0xE, I=1, opcode=MOV(0xD), S=0, Rn=0, Rd=0, imm=42
        let word = binary[0];
        let cond = (word >> 28) & 0xF;
        let i_bit = (word >> 25) & 0x1;
        let opcode = (word >> 21) & 0xF;
        let rd = (word >> 12) & 0xF;
        let imm = word & 0xFFF;

        assert_eq!(cond, 0xE);    // Always execute.
        assert_eq!(i_bit, 1);      // Immediate operand.
        assert_eq!(opcode, 0xD);   // MOV.
        assert_eq!(rd, 0);         // R0.
        assert_eq!(imm, 42);       // #42.
    }

    #[test]
    fn test_encode_add_registers() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("ADD R2, R0, R1").unwrap();
        let binary = asm.encode(&instructions).unwrap();
        assert_eq!(binary.len(), 1);

        let word = binary[0];
        let cond = (word >> 28) & 0xF;
        let i_bit = (word >> 25) & 0x1;
        let opcode = (word >> 21) & 0xF;
        let rn = (word >> 16) & 0xF;
        let rd = (word >> 12) & 0xF;
        let rm = word & 0xF;

        assert_eq!(cond, 0xE);    // Always.
        assert_eq!(i_bit, 0);      // Register operand.
        assert_eq!(opcode, 0x4);   // ADD.
        assert_eq!(rn, 0);         // R0.
        assert_eq!(rd, 2);         // R2.
        assert_eq!(rm, 1);         // R1.
    }

    #[test]
    fn test_encode_nop() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("NOP").unwrap();
        let binary = asm.encode(&instructions).unwrap();
        assert_eq!(binary.len(), 1);
        assert_eq!(binary[0], 0xE1A00000); // Standard ARM NOP.
    }

    #[test]
    fn test_encode_ldr() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("LDR R0, [R1]").unwrap();
        let binary = asm.encode(&instructions).unwrap();
        assert_eq!(binary.len(), 1);

        let word = binary[0];
        // LDR has bit 20 set (load), bits 27-26 = 01.
        let load_bit = (word >> 20) & 0x1;
        assert_eq!(load_bit, 1);
    }

    #[test]
    fn test_encode_str() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("STR R0, [R1]").unwrap();
        let binary = asm.encode(&instructions).unwrap();
        assert_eq!(binary.len(), 1);

        let word = binary[0];
        // STR has bit 20 clear (store).
        let load_bit = (word >> 20) & 0x1;
        assert_eq!(load_bit, 0);
    }

    #[test]
    fn test_encode_labels_produce_no_binary() {
        let mut asm = Assembler::new();
        let instructions = asm.parse("start:\nMOV R0, #1").unwrap();
        let binary = asm.encode(&instructions).unwrap();
        // Label produces no binary, only MOV does.
        assert_eq!(binary.len(), 1);
    }

    #[test]
    fn test_full_program() {
        let source = "\
            MOV R0, #10
            MOV R1, #20
            ADD R2, R0, R1
            STR R2, [R3]
        ";
        let mut asm = Assembler::new();
        let instructions = asm.parse(source).unwrap();
        let binary = asm.encode(&instructions).unwrap();
        assert_eq!(binary.len(), 4);
    }

    // -----------------------------------------------------------------------
    // Error tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_assembler_error_display() {
        let err = AssemblerError::UnknownMnemonic("BLAH".to_string());
        assert_eq!(format!("{}", err), "Unknown mnemonic: BLAH");
    }

    #[test]
    fn test_invalid_register_error() {
        let mut asm = Assembler::new();
        let result = asm.parse("MOV X0, #1");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_operand_count() {
        let mut asm = Assembler::new();
        let result = asm.parse("ADD R0, R1");
        assert!(result.is_err());
    }
}
