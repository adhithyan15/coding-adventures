package compilerir

import (
	"fmt"
	"strconv"
	"strings"
)

// ──────────────────────────────────────────────────────────────────────────────
// IR Parser — text → IrProgram
//
// The parser reads the canonical IR text format (produced by Print) and
// reconstructs an IrProgram. This enables:
//
//   1. Golden-file testing — load an expected .ir file, parse it, compare
//   2. Roundtrip verification — parse(print(program)) == program
//   3. Manual IR authoring — write IR by hand for testing backends
//
// ──────────────────────────────────────────────────────────────────────────────
// Parsing strategy
// ──────────────────────────────────────────────────────────────────────────────
//
// The parser processes the text line by line:
//
//   1. Lines starting with ".version" set the program version
//   2. Lines starting with ".data" add a data declaration
//   3. Lines starting with ".entry" set the entry label
//   4. Lines ending with ":" define a label
//   5. Lines starting with whitespace are instructions
//   6. Lines starting with ";" are standalone comments
//   7. Blank lines are skipped
//
// Each instruction line is split into: opcode, operands, and optional
// "; #N" ID comment. Operands are parsed as registers (v0, v1, ...),
// immediates (42, -1), or labels (any other identifier).
//
// ──────────────────────────────────────────────────────────────────────────────

// Maximum limits for IR text parsing. These prevent denial-of-service
// from adversarial input by capping memory allocation.
const (
	maxLines               = 1_000_000 // max lines in an IR text file
	maxOperandsPerInstr    = 16        // max operands per instruction
	maxRegisterIndex       = 65535     // max virtual register index (v0..v65535)
)

// Parse converts IR text into an IrProgram.
// Returns an error if the text is malformed.
func Parse(text string) (*IrProgram, error) {
	program := &IrProgram{Version: 1}
	lines := strings.Split(text, "\n")

	if len(lines) > maxLines {
		return nil, fmt.Errorf("input too large: %d lines (max %d)", len(lines), maxLines)
	}

	for lineNum, line := range lines {
		trimmed := strings.TrimSpace(line)

		// Skip blank lines
		if trimmed == "" {
			continue
		}

		// Version directive
		if strings.HasPrefix(trimmed, ".version") {
			parts := strings.Fields(trimmed)
			if len(parts) != 2 {
				return nil, fmt.Errorf("line %d: invalid .version directive: %q", lineNum+1, line)
			}
			v, err := strconv.Atoi(parts[1])
			if err != nil {
				return nil, fmt.Errorf("line %d: invalid version number: %q", lineNum+1, parts[1])
			}
			program.Version = v
			continue
		}

		// Data declaration
		if strings.HasPrefix(trimmed, ".data") {
			parts := strings.Fields(trimmed)
			if len(parts) != 4 {
				return nil, fmt.Errorf("line %d: invalid .data directive: %q", lineNum+1, line)
			}
			size, err := strconv.Atoi(parts[2])
			if err != nil {
				return nil, fmt.Errorf("line %d: invalid data size: %q", lineNum+1, parts[2])
			}
			init, err := strconv.Atoi(parts[3])
			if err != nil {
				return nil, fmt.Errorf("line %d: invalid data init: %q", lineNum+1, parts[3])
			}
			program.AddData(IrDataDecl{Label: parts[1], Size: size, Init: init})
			continue
		}

		// Entry point
		if strings.HasPrefix(trimmed, ".entry") {
			parts := strings.Fields(trimmed)
			if len(parts) != 2 {
				return nil, fmt.Errorf("line %d: invalid .entry directive: %q", lineNum+1, line)
			}
			program.EntryLabel = parts[1]
			continue
		}

		// Label definition (line ends with ":")
		if strings.HasSuffix(trimmed, ":") && !strings.HasPrefix(trimmed, ";") {
			labelName := strings.TrimSuffix(trimmed, ":")
			// Extract ID from comment if present (shouldn't be for labels, but be safe)
			program.AddInstruction(IrInstruction{
				Opcode:   OpLabel,
				Operands: []IrOperand{IrLabel{Name: labelName}},
				ID:       -1, // labels don't have meaningful IDs
			})
			continue
		}

		// Standalone comment line ("; text")
		if strings.HasPrefix(trimmed, ";") {
			commentText := strings.TrimSpace(strings.TrimPrefix(trimmed, ";"))
			// Check if it's just a comment (not an ID comment)
			if !strings.HasPrefix(commentText, "#") {
				program.AddInstruction(IrInstruction{
					Opcode:   OpComment,
					Operands: []IrOperand{IrLabel{Name: commentText}},
					ID:       -1,
				})
			}
			continue
		}

		// Instruction line
		instr, err := parseInstructionLine(trimmed, lineNum+1)
		if err != nil {
			return nil, err
		}
		program.AddInstruction(instr)
	}

	return program, nil
}

// parseInstructionLine parses a single instruction line like:
//   "LOAD_IMM   v0, 42  ; #3"
func parseInstructionLine(line string, lineNum int) (IrInstruction, error) {
	// Split off the "; #N" ID comment if present
	id := -1
	instructionPart := line
	if idx := strings.LastIndex(line, "; #"); idx >= 0 {
		idStr := strings.TrimSpace(line[idx+3:])
		parsed, err := strconv.Atoi(idStr)
		if err == nil {
			id = parsed
		}
		instructionPart = strings.TrimSpace(line[:idx])
	}

	// Split into opcode and operands
	fields := strings.Fields(instructionPart)
	if len(fields) == 0 {
		return IrInstruction{}, fmt.Errorf("line %d: empty instruction", lineNum)
	}

	opcodeName := fields[0]
	opcode, ok := ParseOp(opcodeName)
	if !ok {
		return IrInstruction{}, fmt.Errorf("line %d: unknown opcode %q", lineNum, opcodeName)
	}

	// Parse operands (everything after the opcode, comma-separated)
	var operands []IrOperand
	if len(fields) > 1 {
		// Rejoin and split by comma to handle "v0, v1, 42" format
		operandStr := strings.Join(fields[1:], " ")
		parts := strings.Split(operandStr, ",")
		if len(parts) > maxOperandsPerInstr {
			return IrInstruction{}, fmt.Errorf("line %d: too many operands (%d, max %d)", lineNum, len(parts), maxOperandsPerInstr)
		}
		for _, part := range parts {
			part = strings.TrimSpace(part)
			if part == "" {
				continue
			}
			operand, err := parseOperand(part)
			if err != nil {
				return IrInstruction{}, fmt.Errorf("line %d: %w", lineNum, err)
			}
			operands = append(operands, operand)
		}
	}

	return IrInstruction{
		Opcode:   opcode,
		Operands: operands,
		ID:       id,
	}, nil
}

// parseOperand parses a single operand string into an IrOperand.
//
// Parsing rules:
//   - Starts with "v" followed by digits → IrRegister{Index: N}
//   - Parseable as integer → IrImmediate{Value: N}
//   - Anything else → IrLabel{Name: str}
func parseOperand(s string) (IrOperand, error) {
	// Register: v0, v1, v2, ...
	if len(s) > 1 && s[0] == 'v' {
		idx, err := strconv.Atoi(s[1:])
		if err == nil {
			if idx < 0 || idx > maxRegisterIndex {
				return nil, fmt.Errorf("register index %d out of range (max %d)", idx, maxRegisterIndex)
			}
			return IrRegister{Index: idx}, nil
		}
		// Not a valid register number — fall through to label
	}

	// Immediate: 42, -1, 255, ...
	val, err := strconv.Atoi(s)
	if err == nil {
		return IrImmediate{Value: val}, nil
	}

	// Label: _start, loop_0_end, tape, ...
	return IrLabel{Name: s}, nil
}
