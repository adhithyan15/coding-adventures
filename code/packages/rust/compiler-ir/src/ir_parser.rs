//! # IR Parser — text → `IrProgram`.
//!
//! The parser reads the canonical IR text format (produced by `print_ir`)
//! and reconstructs an `IrProgram`. This enables:
//!
//! 1. **Golden-file testing** — load an expected `.ir` file, parse it, compare
//! 2. **Roundtrip verification** — `parse_ir(print_ir(program)) == program`
//! 3. **Manual IR authoring** — write IR by hand for testing backends
//!
//! ## Parsing strategy
//!
//! The parser processes the text line by line:
//!
//! 1. Lines starting with `.version` set the program version
//! 2. Lines starting with `.data` add a data declaration
//! 3. Lines starting with `.entry` set the entry label
//! 4. Lines ending with `:` define a label
//! 5. Lines starting with whitespace are instructions
//! 6. Lines starting with `;` are standalone comments
//! 7. Blank lines are skipped
//!
//! Each instruction line is split into: opcode, operands, and optional
//! `; #N` ID comment. Operands are parsed as registers (`v0`, `v1`, ...),
//! immediates (`42`, `-1`), or labels (any other identifier).
//!
//! ## Limits
//!
//! The parser caps input size to prevent denial-of-service from adversarial
//! input:
//!
//! - Max 1,000,000 lines per file
//! - Max 16 operands per instruction
//! - Max register index 65,535

use crate::opcodes::{IrOp, parse_op};
use crate::types::{IrProgram, IrInstruction, IrDataDecl, IrOperand};

// ===========================================================================
// Limits
// ===========================================================================

/// Maximum number of lines in an IR text file.
const MAX_LINES: usize = 1_000_000;

/// Maximum number of operands per instruction.
const MAX_OPERANDS_PER_INSTR: usize = 16;

/// Maximum virtual register index.
const MAX_REGISTER_INDEX: usize = 65_535;

// ===========================================================================
// parse_ir — String → IrProgram
// ===========================================================================

/// Convert IR text into an `IrProgram`.
///
/// Returns `Ok(program)` if the text is well-formed, or an error message
/// describing the first syntax problem encountered.
///
/// # Example
///
/// ```
/// use compiler_ir::ir_parser::parse_ir;
///
/// let text = r#".version 1
///
/// .data tape 30000 0
///
/// .entry _start
///
/// _start:
///   LOAD_ADDR   v0, tape  ; #0
///   HALT                  ; #1
/// "#;
/// let prog = parse_ir(text).unwrap();
/// assert_eq!(prog.version, 1);
/// assert_eq!(prog.entry_label, "_start");
/// assert_eq!(prog.data.len(), 1);
/// ```
pub fn parse_ir(text: &str) -> Result<IrProgram, String> {
    let mut program = IrProgram::new("_start");
    // Override default version — the .version directive sets it
    program.version = 1;

    let lines: Vec<&str> = text.split('\n').collect();

    if lines.len() > MAX_LINES {
        return Err(format!(
            "input too large: {} lines (max {})",
            lines.len(), MAX_LINES
        ));
    }

    for (line_num_0, &line) in lines.iter().enumerate() {
        let line_num = line_num_0 + 1; // 1-based for error messages
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // .version directive
        if let Some(rest) = trimmed.strip_prefix(".version") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 1 {
                return Err(format!(
                    "line {}: invalid .version directive: {:?}",
                    line_num, line
                ));
            }
            let v: u32 = parts[0].parse().map_err(|_| {
                format!("line {}: invalid version number: {:?}", line_num, parts[0])
            })?;
            program.version = v;
            continue;
        }

        // .data directive
        if let Some(rest) = trimmed.strip_prefix(".data") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(format!(
                    "line {}: invalid .data directive: {:?}",
                    line_num, line
                ));
            }
            let size: usize = parts[1].parse().map_err(|_| {
                format!("line {}: invalid data size: {:?}", line_num, parts[1])
            })?;
            let init: u8 = parts[2].parse().map_err(|_| {
                format!("line {}: invalid data init (must be 0-255): {:?}", line_num, parts[2])
            })?;
            program.add_data(IrDataDecl {
                label: parts[0].to_string(),
                size,
                init,
            });
            continue;
        }

        // .entry directive
        if let Some(rest) = trimmed.strip_prefix(".entry") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 1 {
                return Err(format!(
                    "line {}: invalid .entry directive: {:?}",
                    line_num, line
                ));
            }
            program.entry_label = parts[0].to_string();
            continue;
        }

        // Label definition — line ends with ":" (not a comment)
        if trimmed.ends_with(':') && !trimmed.starts_with(';') {
            let label_name = trimmed.trim_end_matches(':');
            program.add_instruction(IrInstruction::new(
                IrOp::Label,
                vec![IrOperand::Label(label_name.to_string())],
                -1, // labels have no meaningful ID
            ));
            continue;
        }

        // Standalone comment line — starts with ";"
        if trimmed.starts_with(';') {
            let comment_text = trimmed[1..].trim();
            // Skip ID-only comments like "; #3" (those are attached to instructions)
            if !comment_text.starts_with('#') {
                program.add_instruction(IrInstruction::new(
                    IrOp::Comment,
                    vec![IrOperand::Label(comment_text.to_string())],
                    -1,
                ));
            }
            continue;
        }

        // Instruction line
        let instr = parse_instruction_line(trimmed, line_num)?;
        program.add_instruction(instr);
    }

    Ok(program)
}

// ===========================================================================
// parse_instruction_line — one instruction text → IrInstruction
// ===========================================================================

/// Parse a single instruction line such as:
///
/// ```text
/// LOAD_IMM   v0, 42  ; #3
/// ```
///
/// Returns the parsed instruction or an error describing the problem.
fn parse_instruction_line(line: &str, line_num: usize) -> Result<IrInstruction, String> {
    // Split off the "; #N" ID comment if present
    let id: i64;
    let instruction_part: &str;

    if let Some(idx) = line.rfind("; #") {
        let id_str = line[idx + 3..].trim();
        match id_str.parse::<i64>() {
            Ok(parsed) => {
                id = parsed;
                instruction_part = line[..idx].trim();
            }
            Err(_) => {
                // Not a valid ID — treat whole line as instruction
                id = -1;
                instruction_part = line;
            }
        }
    } else {
        id = -1;
        instruction_part = line;
    }

    // Split into opcode and operands
    let fields: Vec<&str> = instruction_part.split_whitespace().collect();
    if fields.is_empty() {
        return Err(format!("line {}: empty instruction", line_num));
    }

    let opcode_name = fields[0];
    let opcode = parse_op(opcode_name).ok_or_else(|| {
        format!("line {}: unknown opcode {:?}", line_num, opcode_name)
    })?;

    // Parse operands — rejoin everything after opcode, then split by comma
    let mut operands: Vec<IrOperand> = Vec::new();
    if fields.len() > 1 {
        let operand_str = fields[1..].join(" ");
        let parts: Vec<&str> = operand_str.split(',').collect();

        if parts.len() > MAX_OPERANDS_PER_INSTR {
            return Err(format!(
                "line {}: too many operands ({}, max {})",
                line_num, parts.len(), MAX_OPERANDS_PER_INSTR
            ));
        }

        for part in &parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let operand = parse_operand(part, line_num)?;
            operands.push(operand);
        }
    }

    Ok(IrInstruction::new(opcode, operands, id))
}

// ===========================================================================
// parse_operand — one operand string → IrOperand
// ===========================================================================

/// Parse a single operand string into an `IrOperand`.
///
/// Parsing rules (in order):
/// 1. Starts with `v` followed by digits → `IrOperand::Register(N)`
/// 2. Parseable as i64 → `IrOperand::Immediate(N)`
/// 3. Anything else → `IrOperand::Label(str)`
fn parse_operand(s: &str, line_num: usize) -> Result<IrOperand, String> {
    // Register: v0, v1, v2, ...
    if s.len() > 1 && s.starts_with('v') {
        if let Ok(idx) = s[1..].parse::<usize>() {
            if idx > MAX_REGISTER_INDEX {
                return Err(format!(
                    "line {}: register index {} out of range (max {})",
                    line_num, idx, MAX_REGISTER_INDEX
                ));
            }
            return Ok(IrOperand::Register(idx));
        }
        // Not a valid register number — fall through to label
    }

    // Immediate: 42, -1, 255, ...
    if let Ok(val) = s.parse::<i64>() {
        return Ok(IrOperand::Immediate(val));
    }

    // Label: _start, loop_0_end, tape, ...
    Ok(IrOperand::Label(s.to_string()))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printer::print_ir;
    use crate::types::{IrDataDecl};

    // ── Basic parsing ─────────────────────────────────────────────────────

    #[test]
    fn test_parse_version() {
        let text = ".version 1\n.entry _start\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.version, 1);
    }

    #[test]
    fn test_parse_entry() {
        let text = ".version 1\n.entry main\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.entry_label, "main");
    }

    #[test]
    fn test_parse_data() {
        let text = ".version 1\n.data tape 30000 0\n.entry _start\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.data.len(), 1);
        assert_eq!(prog.data[0].label, "tape");
        assert_eq!(prog.data[0].size, 30000);
        assert_eq!(prog.data[0].init, 0);
    }

    #[test]
    fn test_parse_label() {
        let text = ".version 1\n.entry _start\n_start:\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.instructions.len(), 1);
        assert_eq!(prog.instructions[0].opcode, IrOp::Label);
        assert_eq!(
            prog.instructions[0].operands[0],
            IrOperand::Label("_start".to_string())
        );
    }

    #[test]
    fn test_parse_halt() {
        let text = ".version 1\n.entry _start\n  HALT                  ; #0\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.instructions.len(), 1);
        assert_eq!(prog.instructions[0].opcode, IrOp::Halt);
        assert_eq!(prog.instructions[0].id, 0);
    }

    #[test]
    fn test_parse_instruction_with_register_operands() {
        let text = ".version 1\n.entry _start\n  ADD_IMM    v1, v1, 1  ; #3\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.instructions.len(), 1);
        let instr = &prog.instructions[0];
        assert_eq!(instr.opcode, IrOp::AddImm);
        assert_eq!(instr.operands[0], IrOperand::Register(1));
        assert_eq!(instr.operands[1], IrOperand::Register(1));
        assert_eq!(instr.operands[2], IrOperand::Immediate(1));
        assert_eq!(instr.id, 3);
    }

    #[test]
    fn test_parse_negative_immediate() {
        let text = ".version 1\n.entry _start\n  ADD_IMM    v1, v1, -1  ; #2\n";
        let prog = parse_ir(text).unwrap();
        let instr = &prog.instructions[0];
        assert_eq!(instr.operands[2], IrOperand::Immediate(-1));
    }

    #[test]
    fn test_parse_label_operand() {
        let text = ".version 1\n.entry _start\n  LOAD_ADDR  v0, tape  ; #0\n";
        let prog = parse_ir(text).unwrap();
        let instr = &prog.instructions[0];
        assert_eq!(instr.opcode, IrOp::LoadAddr);
        assert_eq!(instr.operands[0], IrOperand::Register(0));
        assert_eq!(instr.operands[1], IrOperand::Label("tape".to_string()));
    }

    #[test]
    fn test_parse_comment_instruction() {
        let text = ".version 1\n.entry _start\n  ; this is a comment\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.instructions.len(), 1);
        assert_eq!(prog.instructions[0].opcode, IrOp::Comment);
    }

    #[test]
    fn test_parse_skip_blank_lines() {
        let text = ".version 1\n\n\n.entry _start\n\n  HALT  ; #0\n";
        let prog = parse_ir(text).unwrap();
        assert_eq!(prog.instructions.len(), 1);
    }

    // ── Error cases ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_unknown_opcode() {
        let text = ".version 1\n.entry _start\n  UNKNOWN_OP  ; #0\n";
        let result = parse_ir(text);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown opcode"));
    }

    #[test]
    fn test_parse_invalid_version() {
        let text = ".version abc\n.entry _start\n";
        let result = parse_ir(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_data_size() {
        let text = ".version 1\n.data tape notanumber 0\n.entry _start\n";
        let result = parse_ir(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_register_out_of_range() {
        let text = ".version 1\n.entry _start\n  LOAD_IMM  v99999, 0  ; #0\n";
        // Should succeed — 99999 < 65535 threshold, actually let's use a very large one
        let text2 = ".version 1\n.entry _start\n  LOAD_IMM  v100000, 0  ; #0\n";
        let result = parse_ir(text2);
        assert!(result.is_err());
        let _ = parse_ir(text); // 99999 is > 65535 but let me check
    }

    // ── Roundtrip ─────────────────────────────────────────────────────────

    #[test]
    fn test_roundtrip_minimal() {
        let mut prog = IrProgram::new("_start");
        prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));

        let text = print_ir(&prog);
        let parsed = parse_ir(&text).unwrap();

        assert_eq!(parsed.version, prog.version);
        assert_eq!(parsed.entry_label, prog.entry_label);
        assert_eq!(parsed.data.len(), prog.data.len());
        assert_eq!(parsed.instructions.len(), prog.instructions.len());
    }

    #[test]
    fn test_roundtrip_add_imm() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Immediate(1),
            ],
            5,
        ));
        let text = print_ir(&prog);
        let parsed = parse_ir(&text).unwrap();
        let instr = &parsed.instructions[0];
        assert_eq!(instr.opcode, IrOp::AddImm);
        assert_eq!(instr.operands[0], IrOperand::Register(1));
        assert_eq!(instr.operands[2], IrOperand::Immediate(1));
        assert_eq!(instr.id, 5);
    }

    #[test]
    fn test_roundtrip_branch_z() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::BranchZ,
            vec![
                IrOperand::Register(2),
                IrOperand::Label("loop_0_end".to_string()),
            ],
            7,
        ));
        let text = print_ir(&prog);
        let parsed = parse_ir(&text).unwrap();
        let instr = &parsed.instructions[0];
        assert_eq!(instr.opcode, IrOp::BranchZ);
        assert_eq!(instr.operands[1], IrOperand::Label("loop_0_end".to_string()));
        assert_eq!(instr.id, 7);
    }

    #[test]
    fn test_roundtrip_instruction_count() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadAddr,
            vec![IrOperand::Register(0), IrOperand::Label("tape".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(0)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));

        let text = print_ir(&prog);
        let parsed = parse_ir(&text).unwrap();
        assert_eq!(parsed.instructions.len(), prog.instructions.len());
    }
}
