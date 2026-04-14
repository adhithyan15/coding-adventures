package intel4004assembler

import (
	"fmt"
	"strconv"
	"strings"
)

type AssemblerError struct {
	Message string
}

func (e AssemblerError) Error() string { return e.Message }

type parsedLine struct {
	label    string
	mnemonic string
	operands []string
}

type Intel4004Assembler struct{}

func NewIntel4004Assembler() *Intel4004Assembler {
	return &Intel4004Assembler{}
}

func Assemble(text string) ([]byte, error) {
	return NewIntel4004Assembler().Assemble(text)
}

func (a *Intel4004Assembler) Assemble(text string) ([]byte, error) {
	lines := lexProgram(text)
	symbols, err := pass1(lines)
	if err != nil {
		return nil, err
	}
	return pass2(lines, symbols)
}

func lexProgram(text string) []parsedLine {
	rawLines := strings.Split(strings.ReplaceAll(text, "\r\n", "\n"), "\n")
	lines := make([]parsedLine, 0, len(rawLines))
	for _, raw := range rawLines {
		line := strings.SplitN(raw, ";", 2)[0]
		line = strings.TrimSpace(line)
		if line == "" {
			lines = append(lines, parsedLine{})
			continue
		}
		label := ""
		if colon := strings.Index(line, ":"); colon >= 0 {
			prefix := strings.TrimSpace(line[:colon])
			if prefix != "" && !strings.Contains(prefix, " ") && !strings.Contains(prefix, "\t") {
				label = prefix
				line = strings.TrimSpace(line[colon+1:])
			}
		}
		if line == "" {
			lines = append(lines, parsedLine{label: label})
			continue
		}
		fields := strings.Fields(line)
		mnemonic := strings.ToUpper(fields[0])
		operandText := strings.TrimSpace(line[len(fields[0]):])
		operands := []string{}
		if operandText != "" {
			for _, operand := range strings.Split(operandText, ",") {
				trimmed := strings.TrimSpace(operand)
				if trimmed != "" {
					operands = append(operands, trimmed)
				}
			}
		}
		lines = append(lines, parsedLine{label: label, mnemonic: mnemonic, operands: operands})
	}
	return lines
}

func pass1(lines []parsedLine) (map[string]int, error) {
	symbols := map[string]int{}
	pc := 0
	for _, line := range lines {
		if line.label != "" {
			symbols[line.label] = pc
		}
		if line.mnemonic == "" {
			continue
		}
		if line.mnemonic == "ORG" {
			if len(line.operands) == 0 {
				return nil, AssemblerError{Message: "ORG requires an address operand"}
			}
			address, err := parseNumber(line.operands[0])
			if err != nil {
				return nil, err
			}
			pc = address
			continue
		}
		size, err := instructionSize(line.mnemonic)
		if err != nil {
			return nil, err
		}
		pc += size
	}
	return symbols, nil
}

func pass2(lines []parsedLine, symbols map[string]int) ([]byte, error) {
	output := []byte{}
	pc := 0
	for _, line := range lines {
		if line.mnemonic == "" {
			continue
		}
		if line.mnemonic == "ORG" {
			address, err := parseNumber(line.operands[0])
			if err != nil {
				return nil, err
			}
			for pc < address {
				output = append(output, 0x00)
				pc++
			}
			continue
		}
		encoded, err := encodeInstruction(line.mnemonic, line.operands, symbols, pc)
		if err != nil {
			return nil, err
		}
		output = append(output, encoded...)
		pc += len(encoded)
	}
	return output, nil
}

var fixedOpcodes = map[string]byte{
	"NOP": 0x00, "HLT": 0x01, "WRM": 0xE0, "WMP": 0xE1, "WRR": 0xE2,
	"WR0": 0xE4, "WR1": 0xE5, "WR2": 0xE6, "WR3": 0xE7,
	"SBM": 0xE8, "RDM": 0xE9, "RDR": 0xEA, "ADM": 0xEB,
	"RD0": 0xEC, "RD1": 0xED, "RD2": 0xEE, "RD3": 0xEF,
	"CLB": 0xF0, "CLC": 0xF1, "IAC": 0xF2, "CMC": 0xF3,
	"CMA": 0xF4, "RAL": 0xF5, "RAR": 0xF6, "TCC": 0xF7,
	"DAC": 0xF8, "TCS": 0xF9, "STC": 0xFA, "DAA": 0xFB,
	"KBP": 0xFC, "DCL": 0xFD,
}

func instructionSize(mnemonic string) (int, error) {
	if _, ok := fixedOpcodes[mnemonic]; ok {
		return 1, nil
	}
	switch mnemonic {
	case "INC", "ADD", "SUB", "LD", "XCH", "BBL", "LDM", "SRC", "FIN", "JIN":
		return 1, nil
	case "JCN", "FIM", "JUN", "JMS", "ISZ", "ADD_IMM":
		return 2, nil
	case "ORG":
		return 0, nil
	default:
		return 0, AssemblerError{Message: fmt.Sprintf("Unknown mnemonic: '%s'", mnemonic)}
	}
}

func encodeInstruction(mnemonic string, operands []string, symbols map[string]int, pc int) ([]byte, error) {
	if opcode, ok := fixedOpcodes[mnemonic]; ok {
		if len(operands) != 0 {
			return nil, AssemblerError{Message: fmt.Sprintf("%s expects 0 operand(s), got %d", mnemonic, len(operands))}
		}
		return []byte{opcode}, nil
	}
	switch mnemonic {
	case "ORG":
		return nil, nil
	case "LDM":
		value, err := resolveOperand(oneOperand(mnemonic, operands), symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0xD0 | (value & 0xF))}, nil
	case "BBL":
		value, err := resolveOperand(oneOperand(mnemonic, operands), symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0xC0 | (value & 0xF))}, nil
	case "INC":
		return []byte{byte(0x60 | parseRegister(oneOperand(mnemonic, operands)))}, nil
	case "ADD":
		return []byte{byte(0x80 | parseRegister(oneOperand(mnemonic, operands)))}, nil
	case "SUB":
		return []byte{byte(0x90 | parseRegister(oneOperand(mnemonic, operands)))}, nil
	case "LD":
		return []byte{byte(0xA0 | parseRegister(oneOperand(mnemonic, operands)))}, nil
	case "XCH":
		return []byte{byte(0xB0 | parseRegister(oneOperand(mnemonic, operands)))}, nil
	case "SRC":
		return []byte{byte(0x20 | (2*parsePair(oneOperand(mnemonic, operands)) + 1))}, nil
	case "FIN":
		return []byte{byte(0x30 | (2 * parsePair(oneOperand(mnemonic, operands))))}, nil
	case "JIN":
		return []byte{byte(0x30 | (2*parsePair(oneOperand(mnemonic, operands)) + 1))}, nil
	case "FIM":
		if len(operands) != 2 {
			return nil, AssemblerError{Message: fmt.Sprintf("FIM expects 2 operand(s), got %d", len(operands))}
		}
		immediate, err := resolveOperand(operands[1], symbols, pc)
		if err != nil {
			return nil, err
		}
		if immediate < 0 || immediate > 0xFF {
			return nil, AssemblerError{Message: fmt.Sprintf("FIM immediate out of range: '%d'", immediate)}
		}
		return []byte{byte(0x20 | (2 * parsePair(operands[0]))), byte(immediate)}, nil
	case "JCN":
		if len(operands) != 2 {
			return nil, AssemblerError{Message: fmt.Sprintf("JCN expects 2 operand(s), got %d", len(operands))}
		}
		cond, err := resolveOperand(operands[0], symbols, pc)
		if err != nil {
			return nil, err
		}
		address, err := resolveOperand(operands[1], symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0x10 | (cond & 0xF)), byte(address & 0xFF)}, nil
	case "JUN":
		address, err := resolveOperand(oneOperand(mnemonic, operands), symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0x40 | ((address >> 8) & 0xF)), byte(address & 0xFF)}, nil
	case "JMS":
		address, err := resolveOperand(oneOperand(mnemonic, operands), symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0x50 | ((address >> 8) & 0xF)), byte(address & 0xFF)}, nil
	case "ISZ":
		if len(operands) != 2 {
			return nil, AssemblerError{Message: fmt.Sprintf("ISZ expects 2 operand(s), got %d", len(operands))}
		}
		address, err := resolveOperand(operands[1], symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0x70 | parseRegister(operands[0])), byte(address & 0xFF)}, nil
	case "ADD_IMM":
		if len(operands) != 3 {
			return nil, AssemblerError{Message: fmt.Sprintf("ADD_IMM expects 3 operand(s), got %d", len(operands))}
		}
		register := parseRegister(operands[1])
		immediate, err := resolveOperand(operands[2], symbols, pc)
		if err != nil {
			return nil, err
		}
		return []byte{byte(0xD0 | (immediate & 0xF)), byte(0x80 | register)}, nil
	default:
		return nil, AssemblerError{Message: fmt.Sprintf("Unknown mnemonic: '%s'", mnemonic)}
	}
}

func oneOperand(mnemonic string, operands []string) string {
	if len(operands) != 1 {
		panic(AssemblerError{Message: fmt.Sprintf("%s expects 1 operand(s), got %d", mnemonic, len(operands))})
	}
	return operands[0]
}

func parseRegister(name string) uint8 {
	if !strings.HasPrefix(strings.ToUpper(name), "R") {
		panic(AssemblerError{Message: fmt.Sprintf("Invalid register name: '%s'", name)})
	}
	value, err := strconv.ParseUint(name[1:], 10, 4)
	if err != nil || value > 15 {
		panic(AssemblerError{Message: fmt.Sprintf("Invalid register name: '%s'", name)})
	}
	return uint8(value)
}

func parsePair(name string) uint8 {
	if !strings.HasPrefix(strings.ToUpper(name), "P") {
		panic(AssemblerError{Message: fmt.Sprintf("Invalid register pair name: '%s'", name)})
	}
	value, err := strconv.ParseUint(name[1:], 10, 3)
	if err != nil || value > 7 {
		panic(AssemblerError{Message: fmt.Sprintf("Invalid register pair name: '%s'", name)})
	}
	return uint8(value)
}

func parseNumber(value string) (int, error) {
	if strings.HasPrefix(strings.ToLower(value), "0x") {
		parsed, err := strconv.ParseUint(value[2:], 16, 16)
		if err != nil {
			return 0, AssemblerError{Message: fmt.Sprintf("Invalid numeric literal: '%s'", value)}
		}
		if converted, ok := checkedIntFromUint(parsed); ok {
			return converted, nil
		}
		return 0, AssemblerError{Message: fmt.Sprintf("Numeric literal out of range: '%s'", value)}
	}
	parsed, err := strconv.ParseUint(value, 10, 16)
	if err != nil {
		return 0, AssemblerError{Message: fmt.Sprintf("Invalid numeric literal: '%s'", value)}
	}
	if converted, ok := checkedIntFromUint(parsed); ok {
		return converted, nil
	}
	return 0, AssemblerError{Message: fmt.Sprintf("Numeric literal out of range: '%s'", value)}
}

func checkedIntFromUint(value uint64) (int, bool) {
	maxInt := uint64(^uint(0) >> 1)
	if value > maxInt {
		return 0, false
	}
	return int(value), true
}

func resolveOperand(operand string, symbols map[string]int, pc int) (int, error) {
	if operand == "$" {
		return pc, nil
	}
	if strings.HasPrefix(strings.ToLower(operand), "0x") || (operand != "" && operand[0] >= '0' && operand[0] <= '9') {
		return parseNumber(operand)
	}
	value, ok := symbols[operand]
	if !ok {
		return 0, AssemblerError{Message: fmt.Sprintf("Undefined label: '%s'", operand)}
	}
	return value, nil
}
