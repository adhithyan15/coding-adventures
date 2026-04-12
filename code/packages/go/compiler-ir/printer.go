package compilerir

import (
	"fmt"
	"strings"
)

// ──────────────────────────────────────────────────────────────────────────────
// IR Printer — IrProgram → human-readable text
//
// The printer converts an IrProgram into its canonical text format.
// This format serves three purposes:
//
//   1. Debugging — humans can read the IR to understand what the compiler did
//   2. Golden-file tests — expected IR output is committed as .ir text files
//   3. Roundtrip — parse(print(program)) == program is a testable invariant
//
// ──────────────────────────────────────────────────────────────────────────────
// Text Format
// ──────────────────────────────────────────────────────────────────────────────
//
//   .version 1
//
//   .data tape 30000 0
//
//   .entry _start
//
//   _start:
//     LOAD_ADDR  v0, tape          ; #0
//     LOAD_IMM   v1, 0             ; #1
//     HALT                          ; #2
//
// Key rules:
//   - .version N is always the first non-comment line
//   - .data declarations come before .entry
//   - Labels are on their own line with a trailing colon
//   - Instructions are indented with two spaces
//   - ; #N comments show instruction IDs (informational)
//   - COMMENT instructions emit as "; <text>" on their own line
//
// ──────────────────────────────────────────────────────────────────────────────

// Print converts an IrProgram to its canonical text representation.
func Print(program *IrProgram) string {
	var sb strings.Builder

	// Version directive
	sb.WriteString(fmt.Sprintf(".version %d\n", program.Version))

	// Data declarations
	for _, d := range program.Data {
		sb.WriteString(fmt.Sprintf("\n.data %s %d %d\n", d.Label, d.Size, d.Init))
	}

	// Entry point
	sb.WriteString(fmt.Sprintf("\n.entry %s\n", program.EntryLabel))

	// Instructions
	for _, instr := range program.Instructions {
		if instr.Opcode == OpLabel {
			// Labels get their own unindented line with a colon
			sb.WriteString(fmt.Sprintf("\n%s:\n", instr.Operands[0].String()))
			continue
		}

		if instr.Opcode == OpComment {
			// Comments emit as "; <text>"
			text := ""
			if len(instr.Operands) > 0 {
				text = instr.Operands[0].String()
			}
			sb.WriteString(fmt.Sprintf("  ; %s\n", text))
			continue
		}

		// Regular instruction: "  OPCODE  operands  ; #ID"
		sb.WriteString("  ")
		sb.WriteString(fmt.Sprintf("%-11s", instr.Opcode.String()))

		// Operands, comma-separated
		operandStrs := make([]string, len(instr.Operands))
		for i, op := range instr.Operands {
			operandStrs[i] = op.String()
		}
		if len(operandStrs) > 0 {
			sb.WriteString(strings.Join(operandStrs, ", "))
		}

		// Instruction ID comment
		sb.WriteString(fmt.Sprintf("  ; #%d\n", instr.ID))
	}

	return sb.String()
}
