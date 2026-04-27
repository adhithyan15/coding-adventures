//! # IR Printer — `IrProgram` → human-readable text.
//!
//! The printer converts an `IrProgram` into its canonical text format.
//! This format serves three purposes:
//!
//! 1. **Debugging** — humans can read the IR to understand what the compiler did
//! 2. **Golden-file tests** — expected IR output is committed as `.ir` text files
//! 3. **Roundtrip** — `parse(print(program)) == program` is a testable invariant
//!
//! ## Text Format
//!
//! ```text
//! .version 1
//!
//! .data tape 30000 0
//!
//! .entry _start
//!
//! _start:
//!   LOAD_ADDR   v0, tape          ; #0
//!   LOAD_IMM    v1, 0             ; #1
//!   HALT                          ; #2
//! ```
//!
//! Key rules:
//! - `.version N` is always the first non-comment line
//! - `.data` declarations come before `.entry`
//! - Labels are on their own unindented line with a trailing colon
//! - Instructions are indented with two spaces
//! - `; #N` comments show instruction IDs (informational only)
//! - `COMMENT` instructions emit as `; <text>` on their own indented line

use crate::opcodes::IrOp;
use crate::types::{IrProgram, IrOperand};

// ===========================================================================
// print_ir — IrProgram → String
// ===========================================================================

/// Convert an `IrProgram` to its canonical text representation.
///
/// The output is deterministic and suitable for golden-file testing.
/// Parsing the output with `parse_ir` should yield a structurally
/// equivalent program (roundtrip).
///
/// # Example
///
/// ```
/// use compiler_ir::types::{IrProgram, IrInstruction, IrDataDecl, IrOperand};
/// use compiler_ir::opcodes::IrOp;
/// use compiler_ir::printer::print_ir;
///
/// let mut prog = IrProgram::new("_start");
/// prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
/// prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
///
/// let text = print_ir(&prog);
/// assert!(text.contains(".version 1"));
/// assert!(text.contains(".data tape 30000 0"));
/// assert!(text.contains(".entry _start"));
/// assert!(text.contains("HALT"));
/// ```
pub fn print_ir(program: &IrProgram) -> String {
    let mut out = String::new();

    // Version directive — always first
    out.push_str(&format!(".version {}\n", program.version));

    // Data declarations
    for d in &program.data {
        out.push('\n');
        out.push_str(&format!(".data {} {} {}\n", d.label, d.size, d.init));
    }

    // Entry point
    out.push('\n');
    out.push_str(&format!(".entry {}\n", program.entry_label));

    // Instructions
    for instr in &program.instructions {
        if instr.opcode == IrOp::Label {
            // Labels get their own unindented line with a trailing colon
            let name = match instr.operands.first() {
                Some(IrOperand::Label(n)) => n.as_str(),
                Some(IrOperand::Register(i)) => {
                    // Unlikely but handle gracefully
                    out.push('\n');
                    out.push_str(&format!("v{}:\n", i));
                    continue;
                }
                _ => "unknown",
            };
            out.push('\n');
            out.push_str(&format!("{}:\n", name));
            continue;
        }

        if instr.opcode == IrOp::Comment {
            // Comments emit as "  ; <text>"
            let text = match instr.operands.first() {
                Some(IrOperand::Label(t)) => t.as_str(),
                Some(IrOperand::Immediate(v)) => {
                    out.push_str(&format!("  ; {}\n", v));
                    continue;
                }
                _ => "",
            };
            out.push_str(&format!("  ; {}\n", text));
            continue;
        }

        // Regular instruction: "  OPCODE      operands  ; #ID"
        out.push_str("  ");

        // Left-pad the opcode to 11 characters for alignment
        let opcode_str = instr.opcode.to_string();
        out.push_str(&format!("{:<11}", opcode_str));

        // Operands, comma-separated
        let operand_strs: Vec<String> = instr.operands.iter()
            .map(|op| op.to_string())
            .collect();
        if !operand_strs.is_empty() {
            out.push_str(&operand_strs.join(", "));
        }

        // Instruction ID comment
        out.push_str(&format!("  ; #{}\n", instr.id));
    }

    out
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IrProgram, IrInstruction, IrDataDecl, IrOperand};
    use crate::opcodes::IrOp;

    fn make_minimal_program() -> IrProgram {
        let mut prog = IrProgram::new("_start");
        prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
        // Label instruction
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        // LOAD_ADDR v0, tape
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadAddr,
            vec![IrOperand::Register(0), IrOperand::Label("tape".to_string())],
            0,
        ));
        // HALT
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        prog
    }

    #[test]
    fn test_print_has_version() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        assert!(text.contains(".version 1"), "missing .version: {}", text);
    }

    #[test]
    fn test_print_has_data() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        assert!(text.contains(".data tape 30000 0"), "missing .data: {}", text);
    }

    #[test]
    fn test_print_has_entry() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        assert!(text.contains(".entry _start"), "missing .entry: {}", text);
    }

    #[test]
    fn test_print_has_label() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        assert!(text.contains("_start:"), "missing _start label: {}", text);
    }

    #[test]
    fn test_print_has_halt() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        assert!(text.contains("HALT"), "missing HALT: {}", text);
    }

    #[test]
    fn test_print_instruction_id_comment() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        assert!(text.contains("; #0"), "missing id comment: {}", text);
        assert!(text.contains("; #1"), "missing id comment: {}", text);
    }

    #[test]
    fn test_print_comment_instruction() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Comment,
            vec![IrOperand::Label("this is a comment".to_string())],
            -1,
        ));
        let text = print_ir(&prog);
        assert!(text.contains("; this is a comment"), "missing comment: {}", text);
        // Comment instruction should NOT emit instruction ID
        assert!(!text.contains("; #-1"), "comment should not have ID: {}", text);
    }

    #[test]
    fn test_print_multiple_operands() {
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
        assert!(text.contains("v1, v1, 1"), "missing operands: {}", text);
        assert!(text.contains("; #5"), "missing id: {}", text);
    }

    #[test]
    fn test_print_negative_immediate() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Immediate(-1),
            ],
            3,
        ));
        let text = print_ir(&prog);
        assert!(text.contains("-1"), "missing negative immediate: {}", text);
    }

    #[test]
    fn test_print_empty_program() {
        let prog = IrProgram::new("main");
        let text = print_ir(&prog);
        assert!(text.contains(".version 1"));
        assert!(text.contains(".entry main"));
        // Should not have data section without data
        assert!(!text.contains(".data"));
    }

    #[test]
    fn test_print_version_first() {
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        let version_pos = text.find(".version").unwrap();
        assert_eq!(version_pos, 0, ".version should be first");
    }

    #[test]
    fn test_print_opcode_alignment() {
        // LOAD_ADDR should be left-padded to 11 chars
        let prog = make_minimal_program();
        let text = print_ir(&prog);
        // Find the LOAD_ADDR line
        for line in text.lines() {
            if line.contains("LOAD_ADDR") {
                // Should start with 2 spaces + "LOAD_ADDR  " (padded to 11)
                assert!(line.starts_with("  LOAD_ADDR"), "alignment off: {:?}", line);
                break;
            }
        }
    }
}
