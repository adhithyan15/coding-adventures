//! # intel-8008-assembler — Two-pass assembler for Intel 8008 assembly text.
//!
//! This crate sits in the Oct → Intel 8008 compiler pipeline, after
//! `ir-to-intel-8008-compiler` (which produces assembly text from an `IrProgram`)
//! and produces raw binary bytes suitable for packaging into an Intel HEX file
//! by the `intel-8008-packager` crate.
//!
//! ## Pipeline position
//!
//! ```text
//! Oct source (.oct)
//!   → oct-lexer, oct-parser, oct-type-checker
//! AST / Typed AST
//!   → oct-ir-compiler
//! IrProgram
//!   → intel-8008-ir-validator
//! Validated IrProgram
//!   → ir-to-intel-8008-compiler
//! 8008 Assembly text (.asm)      ← intel-8008-assembler reads THIS
//!   → (this crate)
//! Binary bytes                   → fed to intel-8008-packager
//!   → intel-8008-packager
//! Intel HEX file (.hex)          → fed to intel8008-simulator
//! ```
//!
//! ## Two-pass algorithm
//!
//! Assembling machine code from symbolic text requires two passes because of
//! **forward references**: a `JMP loop_end` appears *before* `loop_end:` is
//! defined.  We cannot know `loop_end`'s address on the first encounter.
//!
//! **Pass 1 — Symbol collection:**
//! Walk every line.  Keep a program counter (PC).  When a label definition
//! `my_label:` is seen, record `symbols["my_label"] = pc`.  When an instruction
//! is seen, advance PC by the instruction's encoded byte size (1, 2, or 3).
//!
//! **Pass 2 — Code emission:**
//! Walk every line again.  Encode each instruction using the completed symbol
//! table.  Any forward reference can now be resolved.
//!
//! ## Quick start
//!
//! ```
//! use intel_8008_assembler::assemble;
//!
//! let binary = assemble("
//!     ORG 0x0000
//! _start:
//!     MVI  B, 0
//!     HLT
//! ").unwrap();
//! assert_eq!(binary, vec![0x06, 0x00, 0xFF]);
//! ```

use std::collections::HashMap;

// ===========================================================================
// AssemblerError — the single error type for all assembly failures
// ===========================================================================

/// An unrecoverable error during assembly.
///
/// Examples:
/// - Unknown mnemonic
/// - Undefined label reference
/// - Immediate value out of 8-bit range
/// - Address too large for the 14-bit address space
/// - Port number out of range
/// - Wrong operand count
///
/// # Display
///
/// ```
/// use intel_8008_assembler::AssemblerError;
/// let e = AssemblerError("Unknown mnemonic: 'FOO'".to_string());
/// assert_eq!(e.to_string(), "Unknown mnemonic: 'FOO'");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerError(pub String);

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AssemblerError {}

// ===========================================================================
// Hardware constants
// ===========================================================================

/// Maximum 14-bit address on the Intel 8008 (16 KB address space).
const MAX_ADDRESS: usize = 0x3FFF;

/// Register encoding: B=0, C=1, D=2, E=3, H=4, L=5, M=6 (mem at H:L), A=7
const REG_B: u8 = 0;
const REG_C: u8 = 1;
const REG_D: u8 = 2;
const REG_E: u8 = 3;
const REG_H: u8 = 4;
const REG_L: u8 = 5;
const REG_M: u8 = 6;
const REG_A: u8 = 7;

// ===========================================================================
// ParsedLine — structured representation of one source line
// ===========================================================================

/// A single parsed line of Intel 8008 assembly source.
///
/// The lexer extracts four fields:
///
/// | Field      | Meaning                                              |
/// |------------|------------------------------------------------------|
/// | `label`    | Label declared on this line, or `None`               |
/// | `mnemonic` | Uppercased opcode / directive, or `None` if blank    |
/// | `operands` | Operand strings in source order (stripped of spaces) |
/// | `source`   | Original raw line (for diagnostics)                  |
#[derive(Debug, Clone)]
struct ParsedLine {
    label: Option<String>,
    mnemonic: Option<String>,
    operands: Vec<String>,
    /// Original source line, preserved for diagnostic messages.
    #[allow(dead_code)]
    source: String,
}

// ===========================================================================
// Lexer — turn each line into a ParsedLine
// ===========================================================================

/// Tokenise a single line of Intel 8008 assembly.
///
/// Steps:
/// 1. Strip comments (everything from the first `;` onwards).
/// 2. Strip trailing whitespace.
/// 3. Check for a leading label (`ident:`).  If found, consume it.
/// 4. Extract the mnemonic (first whitespace-delimited token).
/// 5. Split the remainder on `,` to get operands.
///
/// The `hi(sym)` and `lo(sym)` expressions are kept verbatim; the encoder
/// resolves them during Pass 2.
fn lex_line(source: &str) -> ParsedLine {
    // Step 1: strip comment
    let text = source.split(';').next().unwrap_or("").trim_end();

    // Step 2: strip leading whitespace for label detection
    let stripped = text.trim_start();

    // Step 3: check for label (identifier followed immediately by ':')
    let (label, after_label) = parse_label_prefix(stripped);

    // Step 4: if nothing remains after the label, return a label-only line
    let rest = after_label.trim_start();
    if rest.is_empty() {
        return ParsedLine {
            label,
            mnemonic: None,
            operands: vec![],
            source: source.to_string(),
        };
    }

    // Step 5: extract mnemonic (first whitespace-delimited token)
    let (mnemonic_raw, operand_text) = match rest.find(|c: char| c.is_whitespace()) {
        Some(idx) => (&rest[..idx], rest[idx..].trim_start()),
        None => (rest, ""),
    };
    let mnemonic = mnemonic_raw.to_uppercase();

    // Step 6: split operands on comma
    let operands: Vec<String> = if operand_text.is_empty() {
        vec![]
    } else {
        operand_text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    ParsedLine {
        label,
        mnemonic: Some(mnemonic),
        operands,
        source: source.to_string(),
    }
}

/// Tokenise every line in a multi-line assembly program.
///
/// Blank lines and comment-only lines are included as `ParsedLine` objects
/// with `mnemonic = None`.  This preserves line numbering for error messages.
fn lex_program(text: &str) -> Vec<ParsedLine> {
    text.lines().map(lex_line).collect()
}

/// Try to parse a label prefix `ident:` from the start of `s`.
///
/// Returns `(Some(label_name), rest_after_colon)` if found,
/// or `(None, s)` if not.
fn parse_label_prefix(s: &str) -> (Option<String>, &str) {
    if s.is_empty() {
        return (None, s);
    }
    // A label starts with a letter or underscore
    let first = s.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return (None, s);
    }
    // Consume identifier characters
    let end = s
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(s.len());
    // Check that the next char is ':'
    if s[end..].starts_with(':') {
        let name = s[..end].to_string();
        let rest = &s[end + 1..];
        (Some(name), rest)
    } else {
        (None, s)
    }
}

// ===========================================================================
// Instruction size — needed by Pass 1 before symbols are known
// ===========================================================================

/// Return the encoded byte size of a mnemonic.
///
/// This is called during **Pass 1** when labels are being collected.
/// We must know sizes *before* we know addresses.
///
/// Sizes:
/// - **1 byte**: Fixed opcodes (HLT, RFC/RET, rotations, conditional returns),
///   ALU-register (ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP), MOV, INR, DCR, IN, OUT,
///   RST
/// - **2 bytes**: MVI and ALU-immediate (ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI)
/// - **3 bytes**: JMP/CAL and all conditional jumps/calls
/// - **0 bytes**: ORG directive
fn instruction_size(mnemonic: &str) -> Result<usize, AssemblerError> {
    match mnemonic {
        // Fixed 1-byte: returns, rotations, halt
        "RFC" | "RET" | "RTC" | "RFZ" | "RTZ" | "RFS" | "RTS" | "RFP" | "RTP"
        | "RLC" | "RRC" | "RAL" | "RAR" | "HLT" => Ok(1),
        // ALU register (Group 10): 1 byte
        "ADD" | "ADC" | "SUB" | "SBB" | "ANA" | "XRA" | "ORA" | "CMP" => Ok(1),
        // MOV, INR, DCR, IN, OUT, RST: 1 byte
        "MOV" | "INR" | "DCR" | "IN" | "OUT" | "RST" => Ok(1),
        // MVI + ALU immediate: 2 bytes
        "MVI" | "ADI" | "ACI" | "SUI" | "SBI" | "ANI" | "XRI" | "ORI" | "CPI" => Ok(2),
        // Jump/call (unconditional + conditional): 3 bytes
        "JMP" | "CAL"
        | "JFC" | "JTC" | "JFZ" | "JTZ" | "JFS" | "JTS" | "JFP" | "JTP"
        | "CFC" | "CTC" | "CFZ" | "CTZ" | "CFS" | "CTS" | "CFP" | "CTP" => Ok(3),
        // ORG directive: no bytes emitted
        "ORG" => Ok(0),
        _ => Err(AssemblerError(format!("Unknown mnemonic: {mnemonic:?}"))),
    }
}

// ===========================================================================
// Encoder helpers
// ===========================================================================

/// Parse an 8008 register name to its 3-bit code (0–7).
///
/// Valid names (case-insensitive): A, B, C, D, E, H, L, M.
///
/// ```text
/// B=0, C=1, D=2, E=3, H=4, L=5, M=6 (memory at H:L), A=7
/// ```
fn parse_register(name: &str) -> Result<u8, AssemblerError> {
    match name.trim().to_uppercase().as_str() {
        "B" => Ok(REG_B),
        "C" => Ok(REG_C),
        "D" => Ok(REG_D),
        "E" => Ok(REG_E),
        "H" => Ok(REG_H),
        "L" => Ok(REG_L),
        "M" => Ok(REG_M),
        "A" => Ok(REG_A),
        other => Err(AssemblerError(format!(
            "Invalid 8008 register: {other:?}. Valid registers: A, B, C, D, E, H, L, M"
        ))),
    }
}

/// Parse a numeric literal: decimal or `0x`-prefixed hex.
fn parse_number(s: &str) -> Result<usize, AssemblerError> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16)
            .map_err(|_| AssemblerError(format!("Invalid hex literal: {t:?}")))
    } else {
        t.parse::<usize>()
            .map_err(|_| AssemblerError(format!("Invalid numeric literal: {t:?}")))
    }
}

/// Resolve an operand to an integer value.
///
/// Handles four operand kinds:
/// - `$`         — the current program counter
/// - `0x...`     — hexadecimal literal
/// - decimal     — decimal literal
/// - `hi(sym)`   — high 6 bits of sym's 14-bit address: `(addr >> 8) & 0x3F`
/// - `lo(sym)`   — low 8 bits: `addr & 0xFF`
/// - identifier  — a label reference in the symbol table
fn resolve_operand(
    operand: &str,
    symbols: &HashMap<String, usize>,
    pc: usize,
) -> Result<usize, AssemblerError> {
    let s = operand.trim();

    // `$` = current program counter
    if s == "$" {
        return Ok(pc);
    }

    // hi(sym) / lo(sym) — high/low byte of a 14-bit symbol address
    if let Some(value) = resolve_hi_lo(s, symbols)? {
        return Ok(value);
    }

    // Numeric literals (decimal or hex)
    if s.starts_with("0x") || s.starts_with("0X") || s.chars().next().map_or(false, |c| c.is_ascii_digit() || c == '-') {
        return parse_number(s);
    }

    // Label reference
    symbols
        .get(s)
        .copied()
        .ok_or_else(|| AssemblerError(format!("Undefined label: {s:?}")))
}

/// Try to resolve `hi(sym)` or `lo(sym)` expressions.
///
/// `hi(addr) = (addr >> 8) & 0x3F` — high 6 bits of the 14-bit address.
/// `lo(addr) = addr & 0xFF`         — low 8 bits.
///
/// Returns `Ok(Some(value))` if matched, `Ok(None)` if not a hi/lo expression,
/// `Err(...)` if the symbol is undefined.
fn resolve_hi_lo(
    s: &str,
    symbols: &HashMap<String, usize>,
) -> Result<Option<usize>, AssemblerError> {
    let lower = s.to_lowercase();
    let (kind, _inner) = if let Some(rest) = lower.strip_prefix("hi(") {
        if let Some(sym_part) = rest.strip_suffix(')') {
            ("hi", sym_part)
        } else {
            return Ok(None);
        }
    } else if let Some(rest) = lower.strip_prefix("lo(") {
        if let Some(sym_part) = rest.strip_suffix(')') {
            ("lo", sym_part)
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // The symbol name in the original (case-preserved) string
    let sym = &s[3..s.len() - 1]; // works because "hi(" and "lo(" are both 3 chars
    let addr = symbols
        .get(sym)
        .copied()
        .ok_or_else(|| AssemblerError(format!("Undefined label in {s:?}: {sym:?}")))?;

    let value = if kind == "hi" {
        // High 6 bits of 14-bit address: bits 13:8
        (addr >> 8) & 0x3F
    } else {
        // Low 8 bits
        addr & 0xFF
    };
    Ok(Some(value))
}

/// Check that `value` is within `[lo, hi]` (inclusive).
fn check_range(name: &str, value: usize, lo: usize, hi: usize) -> Result<(), AssemblerError> {
    if value < lo || value > hi {
        return Err(AssemblerError(format!(
            "{name} value {value} (0x{value:X}) is out of range [{lo}, {hi}] (0x{lo:X}..0x{hi:X})"
        )));
    }
    Ok(())
}

/// Assert that `operands.len() == count`, else error.
fn expect_operands(mnemonic: &str, operands: &[String], count: usize) -> Result<(), AssemblerError> {
    if operands.len() != count {
        return Err(AssemblerError(format!(
            "{mnemonic} expects {count} operand(s), got {}: {:?}",
            operands.len(),
            operands,
        )));
    }
    Ok(())
}

// ===========================================================================
// Fixed opcode tables
// ===========================================================================

/// Map mnemonic → 1-byte opcode for fixed (no-operand) instructions.
///
/// Return instruction encoding: `00 CCC T11`
/// - CCC = condition code: 0=CY, 1=Z, 2=S, 3=P
/// - T   = sense bit: 0=if-false (RF*), 1=if-true (RT*)
/// - bits[1:0] = `11`
///
/// ```text
/// RFC = 00_000_0_11 = 0x03   (carry false — unconditional return in practice)
/// RTC = 00_000_1_11 = 0x07
/// RFZ = 00_001_0_11 = 0x0B
/// RTZ = 00_001_1_11 = 0x0F
/// RFS = 00_010_0_11 = 0x13
/// RTS = 00_010_1_11 = 0x17
/// RFP = 00_011_0_11 = 0x1B
/// RTP = 00_011_1_11 = 0x1F
/// ```
fn fixed_opcode(mnemonic: &str) -> Option<u8> {
    match mnemonic {
        // Rotations (Group 00)
        "RLC" => Some(0x02),
        "RRC" => Some(0x0A),
        "RAL" => Some(0x12),
        "RAR" => Some(0x1A),
        // Conditional returns
        "RFC" | "RET" => Some(0x03), // RFC = unconditional return (carry always 0 after ALU)
        "RFZ" => Some(0x0B),
        "RFS" => Some(0x13),
        "RFP" => Some(0x1B),
        "RTC" => Some(0x07),
        "RTZ" => Some(0x0F),
        "RTS" => Some(0x17),
        "RTP" => Some(0x1F),
        // Halt
        "HLT" => Some(0xFF),
        _ => None,
    }
}

/// Map mnemonic → base opcode for ALU-register instructions (Group 10).
///
/// Full opcode = `base | reg_code` (1 byte).
/// Example: `ADD C = 0x80 | 1 = 0x81`
fn alu_reg_base(mnemonic: &str) -> Option<u8> {
    match mnemonic {
        "ADD" => Some(0x80),
        "ADC" => Some(0x88),
        "SUB" => Some(0x90),
        "SBB" => Some(0x98),
        "ANA" => Some(0xA0),
        "XRA" => Some(0xA8),
        "ORA" => Some(0xB0),
        "CMP" => Some(0xB8),
        _ => None,
    }
}

/// Map mnemonic → opcode byte for ALU-immediate instructions (Group 11).
///
/// Encoding: `11 OOO 100` where OOO = operation index.
/// 2-byte instruction: `[opcode, d8]`.
///
/// ```text
/// ADI = 0xC4  (11_000_100)
/// ACI = 0xCC  (11_001_100)
/// SUI = 0xD4  (11_010_100)
/// SBI = 0xDC  (11_011_100)
/// ANI = 0xE4  (11_100_100)
/// XRI = 0xEC  (11_101_100)
/// ORI = 0xF4  (11_110_100)
/// CPI = 0xFC  (11_111_100)
/// ```
fn alu_imm_opcode(mnemonic: &str) -> Option<u8> {
    match mnemonic {
        "ADI" => Some(0xC4),
        "ACI" => Some(0xCC),
        "SUI" => Some(0xD4),
        "SBI" => Some(0xDC),
        "ANI" => Some(0xE4),
        "XRI" => Some(0xEC),
        "ORI" => Some(0xF4),
        "CPI" => Some(0xFC),
        _ => None,
    }
}

/// Map mnemonic → first opcode byte for 3-byte jump/call instructions.
///
/// Unconditional: `JMP = 0x7C`, `CAL = 0x7E`.
///
/// Conditional encoding: `01 CCC T00` (jump) / `01 CCC T10` (call)
/// - CCC = condition code: 0=CY, 1=Z, 2=S, 3=P
/// - T   = sense bit: 0=if-false (JF*/CF*), 1=if-true (JT*/CT*)
///
/// ```text
/// JFC=0x40 JTC=0x44 JFZ=0x48 JTZ=0x4C JFS=0x50 JTS=0x54 JFP=0x58 JTP=0x5C
/// CFC=0x42 CTC=0x46 CFZ=0x4A CTZ=0x4E CFS=0x52 CTS=0x56 CFP=0x5A CTP=0x5E
/// ```
fn jump_call_opcode(mnemonic: &str) -> Option<u8> {
    match mnemonic {
        "JMP" => Some(0x7C),
        "CAL" => Some(0x7E),
        "JFC" => Some(0x40),
        "JTC" => Some(0x44),
        "JFZ" => Some(0x48),
        "JTZ" => Some(0x4C),
        "JFS" => Some(0x50),
        "JTS" => Some(0x54),
        "JFP" => Some(0x58),
        "JTP" => Some(0x5C),
        "CFC" => Some(0x42),
        "CTC" => Some(0x46),
        "CFZ" => Some(0x4A),
        "CTZ" => Some(0x4E),
        "CFS" => Some(0x52),
        "CTS" => Some(0x56),
        "CFP" => Some(0x5A),
        "CTP" => Some(0x5E),
        _ => None,
    }
}

// ===========================================================================
// Main instruction encoder (Pass 2)
// ===========================================================================

/// Encode one Intel 8008 instruction into its binary representation.
///
/// This is the heart of **Pass 2**.  For each instruction we:
/// 1. Validate operand count.
/// 2. Resolve any label / `$` / `hi()` / `lo()` references.
/// 3. Range-check immediates.
/// 4. Build and return the byte sequence.
///
/// # Address encoding (3-byte instructions)
///
/// Format: `[opcode, lo8(addr), hi6(addr)]`
/// - `lo8 = addr & 0xFF`
/// - `hi6 = (addr >> 8) & 0x3F`
///
/// The CPU reconstructs the 14-bit address as `(hi6 << 8) | lo8`.
fn encode_instruction(
    mnemonic: &str,
    operands: &[String],
    symbols: &HashMap<String, usize>,
    pc: usize,
) -> Result<Vec<u8>, AssemblerError> {
    // ORG directive — emits nothing
    if mnemonic == "ORG" {
        return Ok(vec![]);
    }

    // Fixed 1-byte instructions (no operands)
    if let Some(opcode) = fixed_opcode(mnemonic) {
        expect_operands(mnemonic, operands, 0)?;
        return Ok(vec![opcode]);
    }

    // MOV dst, src  (Group 01: 0x40 | dst<<3 | src)
    //
    // Copies one register to another.  Both dst and src are 3-bit register codes.
    // Opcode: 0x40 | (dst_code << 3) | src_code
    //
    // Note: MOV M, M (0x76) is an alternate HLT encoding on real hardware.
    // Example: MOV A, B = 0x40 | (7<<3) | 0 = 0x78
    if mnemonic == "MOV" {
        expect_operands(mnemonic, operands, 2)?;
        let dst = parse_register(&operands[0])?;
        let src = parse_register(&operands[1])?;
        return Ok(vec![0x40 | (dst << 3) | src]);
    }

    // MVI r, d8  (Group 00: (r<<3) | 0x06, d8)
    //
    // Move Immediate: load 8-bit constant into register r.
    // First opcode byte: (r_code << 3) | 0x06
    // Example: MVI B, 42 → [0x06, 0x2A]   (0<<3)|0x06=0x06; 42=0x2A
    // Example: MVI H, hi(counter) → [0x26, hi_value]
    if mnemonic == "MVI" {
        expect_operands(mnemonic, operands, 2)?;
        let r = parse_register(&operands[0])?;
        let d8 = resolve_operand(&operands[1], symbols, pc)?;
        check_range(&format!("{mnemonic} immediate"), d8, 0, 255)?;
        let opcode = (r << 3) | 0x06;
        return Ok(vec![opcode, d8 as u8]);
    }

    // INR r  (Group 00: r<<3)
    //
    // Increment register r.  Does NOT affect the carry flag.
    // Example: INR B → 0x00,  INR D → 0x10
    if mnemonic == "INR" {
        expect_operands(mnemonic, operands, 1)?;
        let r = parse_register(&operands[0])?;
        return Ok(vec![r << 3]);
    }

    // DCR r  (Group 00: (r<<3) | 0x01)
    //
    // Decrement register r.  Does NOT affect the carry flag.
    // Example: DCR B → 0x01,  DCR C → 0x09
    if mnemonic == "DCR" {
        expect_operands(mnemonic, operands, 1)?;
        let r = parse_register(&operands[0])?;
        return Ok(vec![(r << 3) | 0x01]);
    }

    // RST n  (Group 00: (n<<3) | 0x05)
    //
    // Restart: push PC onto stack and jump to address n×8 (page 0).
    // n must be 0–7.
    if mnemonic == "RST" {
        expect_operands(mnemonic, operands, 1)?;
        let n = resolve_operand(&operands[0], symbols, pc)?;
        check_range("RST n", n, 0, 7)?;
        return Ok(vec![((n as u8) << 3) | 0x05]);
    }

    // ALU register operations (Group 10)
    //
    // ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP — all 1 byte.
    // Opcode: base_opcode | reg_code
    // Example: ADD B → 0x80 | 0 = 0x80,  CMP C → 0xB8 | 1 = 0xB9
    if let Some(base) = alu_reg_base(mnemonic) {
        expect_operands(mnemonic, operands, 1)?;
        let r = parse_register(&operands[0])?;
        return Ok(vec![base | r]);
    }

    // ALU immediate operations (Group 11)
    //
    // ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI — all 2 bytes: [opcode, d8]
    if let Some(opcode) = alu_imm_opcode(mnemonic) {
        expect_operands(mnemonic, operands, 1)?;
        let d8 = resolve_operand(&operands[0], symbols, pc)?;
        check_range(&format!("{mnemonic} immediate"), d8, 0, 255)?;
        return Ok(vec![opcode, d8 as u8]);
    }

    // IN p  (Group 01: 0x41 | p<<3)
    //
    // Read 8-bit input from port p (p = 0–7) into the accumulator.
    // Encoding: 0x41 | (p << 3)
    // p=0 → 0x41, p=1 → 0x49, ..., p=7 → 0x79
    if mnemonic == "IN" {
        expect_operands(mnemonic, operands, 1)?;
        let p = resolve_operand(&operands[0], symbols, pc)?;
        check_range("IN port", p, 0, 7)?;
        return Ok(vec![0x41 | ((p as u8) << 3)]);
    }

    // OUT p  (opcode = p << 1)
    //
    // Write accumulator to output port p (p = 0–23).
    // Only ports 17 and 21 are reliably simulator-compatible (produce opcodes
    // that decode unambiguously as OUT).  Other ports conflict with other
    // instructions but are still assembled as specified.
    if mnemonic == "OUT" {
        expect_operands(mnemonic, operands, 1)?;
        let p = resolve_operand(&operands[0], symbols, pc)?;
        check_range("OUT port", p, 0, 23)?;
        return Ok(vec![(p as u8) << 1]);
    }

    // 3-byte jump and call instructions (JMP, CAL, conditional variants)
    //
    // Format: [opcode, lo8(addr), hi6(addr)]
    // lo8(addr) = addr & 0xFF
    // hi6(addr) = (addr >> 8) & 0x3F
    if let Some(opcode) = jump_call_opcode(mnemonic) {
        expect_operands(mnemonic, operands, 1)?;
        let addr = resolve_operand(&operands[0], symbols, pc)?;
        check_range(&format!("{mnemonic} address"), addr, 0, MAX_ADDRESS)?;
        let lo8 = (addr & 0xFF) as u8;
        let hi6 = ((addr >> 8) & 0x3F) as u8;
        return Ok(vec![opcode, lo8, hi6]);
    }

    Err(AssemblerError(format!("Unknown mnemonic: {mnemonic:?}")))
}

// ===========================================================================
// Pass 1 — build the symbol table
// ===========================================================================

/// Collect all label addresses from the parsed lines.
///
/// Rules:
/// - Start with `pc = 0`.
/// - `ORG addr` sets `pc = addr`.
/// - A label on a line records `{label: pc}` *before* advancing PC for any
///   instruction on the same line (labels point at the instruction that follows).
/// - Instructions advance `pc` by `instruction_size(mnemonic)`.
/// - Blank / comment lines leave PC unchanged.
fn pass1(lines: &[ParsedLine]) -> Result<HashMap<String, usize>, AssemblerError> {
    let mut symbols: HashMap<String, usize> = HashMap::new();
    let mut pc: usize = 0;

    for line in lines {
        // Record label at current PC (before the instruction advances it)
        if let Some(ref label) = line.label {
            symbols.insert(label.clone(), pc);
        }

        let mnemonic = match &line.mnemonic {
            Some(m) => m.as_str(),
            None => continue, // blank or comment-only
        };

        if mnemonic == "ORG" {
            let addr_str = line.operands.first().ok_or_else(|| {
                AssemblerError("ORG requires an address operand".to_string())
            })?;
            let addr = parse_number(addr_str)?;
            if addr > MAX_ADDRESS {
                return Err(AssemblerError(format!(
                    "ORG address 0x{addr:X} exceeds Intel 8008 address space (max 0x{MAX_ADDRESS:X})"
                )));
            }
            pc = addr;
            continue;
        }

        pc += instruction_size(mnemonic)?;
    }

    Ok(symbols)
}

// ===========================================================================
// Pass 2 — emit bytes
// ===========================================================================

/// Emit encoded bytes for each instruction using the completed symbol table.
///
/// ORG pads to the target address with `0xFF` (erased flash / ROM state)
/// when advancing forward.  Backward ORG is not supported — the existing
/// output is truncated to the new address.
fn pass2(
    lines: &[ParsedLine],
    symbols: &HashMap<String, usize>,
) -> Result<Vec<u8>, AssemblerError> {
    let mut output: Vec<u8> = Vec::new();
    let mut pc: usize = 0;

    for line in lines {
        let mnemonic = match &line.mnemonic {
            Some(m) => m.as_str(),
            None => continue,
        };

        if mnemonic == "ORG" {
            let addr_str = line.operands.first().ok_or_else(|| {
                AssemblerError("ORG requires an address operand".to_string())
            })?;
            let org_addr = parse_number(addr_str)?;
            if org_addr > MAX_ADDRESS {
                return Err(AssemblerError(format!(
                    "ORG address 0x{org_addr:X} exceeds Intel 8008 address space (max 0x{MAX_ADDRESS:X})"
                )));
            }
            // Pad forward with 0xFF (erased flash state)
            if org_addr > pc {
                output.extend(std::iter::repeat(0xFF).take(org_addr - pc));
            }
            pc = org_addr;
            continue;
        }

        let encoded = encode_instruction(mnemonic, &line.operands, symbols, pc)?;
        pc += encoded.len();
        output.extend(encoded);
    }

    Ok(output)
}

// ===========================================================================
// Public API
// ===========================================================================

/// Two-pass Intel 8008 assembler.
///
/// The assembler is stateless between calls — each call to `assemble()` gets
/// a fresh symbol table and program counter.  It is safe to reuse the same
/// instance for multiple programs.
///
/// # Example
///
/// ```
/// use intel_8008_assembler::Intel8008Assembler;
///
/// let binary = Intel8008Assembler.assemble("
///     ORG 0x0000
/// _start:
///     MVI  B, 0
///     HLT
/// ").unwrap();
/// assert_eq!(binary, vec![0x06, 0x00, 0xFF]);
/// ```
pub struct Intel8008Assembler;

impl Intel8008Assembler {
    /// Assemble Intel 8008 assembly source text into binary bytes.
    ///
    /// Runs two passes:
    /// - **Pass 1**: builds the symbol table.
    /// - **Pass 2**: encodes instructions using the completed symbol table.
    ///
    /// # Errors
    ///
    /// Returns `Err(AssemblerError)` on unknown mnemonics, undefined labels,
    /// out-of-range values, or wrong operand counts.
    pub fn assemble(&self, text: &str) -> Result<Vec<u8>, AssemblerError> {
        let lines = lex_program(text);
        let symbols = pass1(&lines)?;
        pass2(&lines, &symbols)
    }
}

/// Assemble Intel 8008 assembly source text into binary bytes.
///
/// Convenience wrapper around `Intel8008Assembler::assemble()`.
///
/// # Example
///
/// ```
/// use intel_8008_assembler::assemble;
///
/// let binary = assemble("
///     ORG 0x0000
/// _start:
///     HLT
/// ").unwrap();
/// assert_eq!(binary, vec![0xFF]);
/// ```
pub fn assemble(text: &str) -> Result<Vec<u8>, AssemblerError> {
    Intel8008Assembler.assemble(text)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Lexer tests
    // ------------------------------------------------------------------

    #[test]
    fn lex_blank_line() {
        let l = lex_line("");
        assert!(l.label.is_none());
        assert!(l.mnemonic.is_none());
        assert!(l.operands.is_empty());
    }

    #[test]
    fn lex_comment_only_line() {
        let l = lex_line("; this is a comment");
        assert!(l.label.is_none());
        assert!(l.mnemonic.is_none());
    }

    #[test]
    fn lex_label_only() {
        let l = lex_line("loop_0_start:");
        assert_eq!(l.label.as_deref(), Some("loop_0_start"));
        assert!(l.mnemonic.is_none());
    }

    #[test]
    fn lex_label_with_instruction() {
        let l = lex_line("_start:  MVI  B, 42");
        assert_eq!(l.label.as_deref(), Some("_start"));
        assert_eq!(l.mnemonic.as_deref(), Some("MVI"));
        assert_eq!(l.operands, vec!["B", "42"]);
    }

    #[test]
    fn lex_instruction_no_label() {
        let l = lex_line("    MOV  A, B");
        assert!(l.label.is_none());
        assert_eq!(l.mnemonic.as_deref(), Some("MOV"));
        assert_eq!(l.operands, vec!["A", "B"]);
    }

    #[test]
    fn lex_hi_lo_operand_preserved() {
        let l = lex_line("    MVI  H, hi(counter)");
        assert_eq!(l.operands, vec!["H", "hi(counter)"]);
    }

    #[test]
    fn lex_comment_stripped() {
        let l = lex_line("    HLT  ; halt the processor");
        assert_eq!(l.mnemonic.as_deref(), Some("HLT"));
        assert!(l.operands.is_empty());
    }

    // ------------------------------------------------------------------
    // Instruction size tests
    // ------------------------------------------------------------------

    #[test]
    fn size_of_fixed_opcodes() {
        for m in &["HLT", "RFC", "RET", "RLC", "RRC", "RAL", "RAR", "RFZ", "RTC"] {
            assert_eq!(instruction_size(m).unwrap(), 1, "{m}");
        }
    }

    #[test]
    fn size_of_alu_reg() {
        for m in &["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"] {
            assert_eq!(instruction_size(m).unwrap(), 1, "{m}");
        }
    }

    #[test]
    fn size_of_two_byte_instructions() {
        for m in &["MVI", "ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"] {
            assert_eq!(instruction_size(m).unwrap(), 2, "{m}");
        }
    }

    #[test]
    fn size_of_three_byte_instructions() {
        for m in &["JMP", "CAL", "JFC", "JTC", "JFZ", "JTZ", "JFP", "JTP",
                    "CFC", "CTC", "CFZ", "CTZ"] {
            assert_eq!(instruction_size(m).unwrap(), 3, "{m}");
        }
    }

    #[test]
    fn size_of_org_is_zero() {
        assert_eq!(instruction_size("ORG").unwrap(), 0);
    }

    #[test]
    fn size_of_unknown_errors() {
        assert!(instruction_size("BOGUS").is_err());
    }

    // ------------------------------------------------------------------
    // Encoder tests
    // ------------------------------------------------------------------

    #[test]
    fn encode_hlt() {
        let b = encode_instruction("HLT", &[], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0xFF]);
    }

    #[test]
    fn encode_rfc_and_ret_same_opcode() {
        let syms = HashMap::new();
        assert_eq!(
            encode_instruction("RFC", &[], &syms, 0).unwrap(),
            encode_instruction("RET", &[], &syms, 0).unwrap()
        );
        assert_eq!(encode_instruction("RFC", &[], &syms, 0).unwrap(), vec![0x03]);
    }

    #[test]
    fn encode_mvi_b_42() {
        // MVI B, 42 → [(0<<3)|0x06, 42] = [0x06, 0x2A]
        let b = encode_instruction("MVI", &["B".to_string(), "42".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x06, 0x2A]);
    }

    #[test]
    fn encode_mvi_h_32() {
        // MVI H, 0x20 → [(4<<3)|0x06, 0x20] = [0x26, 0x20]
        let b = encode_instruction("MVI", &["H".to_string(), "0x20".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x26, 0x20]);
    }

    #[test]
    fn encode_mov_a_b() {
        // MOV A, B → 0x40 | (7<<3) | 0 = 0x78
        let b = encode_instruction("MOV", &["A".to_string(), "B".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x78]);
    }

    #[test]
    fn encode_add_c() {
        // ADD C → 0x80 | 1 = 0x81
        let b = encode_instruction("ADD", &["C".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x81]);
    }

    #[test]
    fn encode_jmp_label() {
        // JMP 0x000A → [0x7C, 0x0A, 0x00]
        let b = encode_instruction("JMP", &["0x000A".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x7C, 0x0A, 0x00]);
    }

    #[test]
    fn encode_cal_label() {
        // CAL 0x0100 → [0x7E, 0x00, 0x01]
        let b = encode_instruction("CAL", &["0x0100".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x7E, 0x00, 0x01]);
    }

    #[test]
    fn encode_jtz_label_resolved() {
        // JTZ loop_end where loop_end = 0x0010 → [0x4C, 0x10, 0x00]
        let mut syms = HashMap::new();
        syms.insert("loop_end".to_string(), 0x0010);
        let b = encode_instruction("JTZ", &["loop_end".to_string()], &syms, 0).unwrap();
        assert_eq!(b, vec![0x4C, 0x10, 0x00]);
    }

    #[test]
    fn encode_adi_immediate() {
        // ADI 5 → [0xC4, 0x05]
        let b = encode_instruction("ADI", &["5".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0xC4, 0x05]);
    }

    #[test]
    fn encode_in_port_2() {
        // IN 2 → 0x41 | (2<<3) = 0x51
        let b = encode_instruction("IN", &["2".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x51]);
    }

    #[test]
    fn encode_out_port_17() {
        // OUT 17 → 17<<1 = 0x22
        let b = encode_instruction("OUT", &["17".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x22]);
    }

    #[test]
    fn encode_inr_d() {
        // INR D → 2<<3 = 0x10
        let b = encode_instruction("INR", &["D".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x10]);
    }

    #[test]
    fn encode_dcr_c() {
        // DCR C → (1<<3) | 0x01 = 0x09
        let b = encode_instruction("DCR", &["C".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x09]);
    }

    #[test]
    fn encode_rst_3() {
        // RST 3 → (3<<3) | 0x05 = 0x1D
        let b = encode_instruction("RST", &["3".to_string()], &HashMap::new(), 0).unwrap();
        assert_eq!(b, vec![0x1D]);
    }

    #[test]
    fn encode_hi_lo_operands() {
        // MVI H, hi(counter) where counter = 0x2000
        // hi(0x2000) = (0x2000 >> 8) & 0x3F = 0x20
        // MVI H → (4<<3) | 0x06 = 0x26
        let mut syms = HashMap::new();
        syms.insert("counter".to_string(), 0x2000);
        let b = encode_instruction("MVI", &["H".to_string(), "hi(counter)".to_string()], &syms, 0).unwrap();
        assert_eq!(b, vec![0x26, 0x20]);

        // MVI L, lo(counter)  → lo(0x2000) = 0x00
        // MVI L → (5<<3) | 0x06 = 0x2E
        let b = encode_instruction("MVI", &["L".to_string(), "lo(counter)".to_string()], &syms, 0).unwrap();
        assert_eq!(b, vec![0x2E, 0x00]);
    }

    // ------------------------------------------------------------------
    // Full assembler tests (two-pass)
    // ------------------------------------------------------------------

    #[test]
    fn assemble_minimal_halt() {
        let b = assemble("    ORG 0x0000\n_start:\n    HLT\n").unwrap();
        assert_eq!(b, vec![0xFF]);
    }

    #[test]
    fn assemble_mvi_b_then_halt() {
        let b = assemble("    ORG 0x0000\n_start:\n    MVI  B, 0\n    HLT\n").unwrap();
        assert_eq!(b, vec![0x06, 0x00, 0xFF]);
    }

    #[test]
    fn assemble_forward_label_reference() {
        // JMP loop_end should resolve even though loop_end is declared after JMP
        let src = "
            ORG 0x0000
        _start:
            JMP loop_end
        loop_end:
            HLT
        ";
        let b = assemble(src).unwrap();
        // JMP loop_end — loop_end is at byte offset 3 (after the 3-byte JMP)
        // JMP 0x0003 → [0x7C, 0x03, 0x00]
        // HLT → [0xFF]
        assert_eq!(b, vec![0x7C, 0x03, 0x00, 0xFF]);
    }

    #[test]
    fn assemble_cal_and_ret() {
        let src = "
            ORG 0x0000
        _start:
            CAL _fn_main
            HLT
        _fn_main:
            MVI  D, 42
            MOV  A, D
            RFC
        ";
        let b = assemble(src).unwrap();
        // CAL _fn_main — _fn_main is at offset 4 (3+1)
        // CAL 0x0004 → [0x7E, 0x04, 0x00]
        // HLT        → [0xFF]
        // MVI D, 42  → [0x16, 0x2A]   (D=(2), (2<<3)|0x06=0x16)
        // MOV A, D   → 0x40 | (7<<3) | 2 = 0x7A
        // RFC        → 0x03
        assert_eq!(b[0..3], [0x7E, 0x04, 0x00]); // CAL _fn_main
        assert_eq!(b[3], 0xFF);                    // HLT
        assert_eq!(b[4..6], [0x16, 0x2A]);         // MVI D, 42
        assert_eq!(b[6], 0x7A);                    // MOV A, D
        assert_eq!(b[7], 0x03);                    // RFC
    }

    #[test]
    fn assemble_loop_with_jtz() {
        // Minimal loop: load 5, decrement until zero
        let src = "
            ORG 0x0000
        _start:
            MVI  B, 5
        loop:
            DCR  B
            JTZ  done
            JMP  loop
        done:
            HLT
        ";
        let b = assemble(src).unwrap();
        assert!(!b.is_empty());
        assert_eq!(*b.last().unwrap(), 0xFF); // last instruction is HLT
    }

    #[test]
    fn assemble_org_padding() {
        // ORG 0x0003 should pad with 0xFF
        let src = "
            ORG 0x0000
            MVI  B, 1
            ORG 0x0005
            HLT
        ";
        let b = assemble(src).unwrap();
        // MVI B, 1 → [0x06, 0x01] at 0x0000
        // padding 0xFF at 0x0002, 0x0003, 0x0004
        // HLT → 0xFF at 0x0005
        assert_eq!(b[0..2], [0x06, 0x01]);
        assert_eq!(&b[2..5], &[0xFF, 0xFF, 0xFF]); // padding
        assert_eq!(b[5], 0xFF); // HLT
    }

    #[test]
    fn assemble_error_unknown_mnemonic() {
        let result = assemble("    BOGUS\n");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("BOGUS"));
    }

    #[test]
    fn assemble_error_undefined_label() {
        let result = assemble("    JMP undefined_label\n");
        assert!(result.is_err());
    }

    #[test]
    fn assemble_error_imm_out_of_range() {
        let result = assemble("    MVI B, 256\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("256") || msg.contains("range"));
    }

    #[test]
    fn assembler_error_display() {
        let e = AssemblerError("test error".to_string());
        assert_eq!(e.to_string(), "test error");
    }

    #[test]
    fn dollar_sign_resolves_to_pc() {
        // JMP $ at PC=0 → JMP 0x0000 → [0x7C, 0x00, 0x00]
        let b = assemble("    ORG 0x0000\n    JMP $\n").unwrap();
        assert_eq!(b, vec![0x7C, 0x00, 0x00]);
    }
}
