package riscvassembler

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"

	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

type AssembleResult struct {
	Bytes        []byte
	Instructions []uint32
	Data         []byte
	LabelOffsets map[string]int
	DataOffsets  map[string]int
	TextSize     int
}

type AssemblyError struct {
	Line    int
	Message string
}

func (e *AssemblyError) Error() string {
	if e.Line > 0 {
		return fmt.Sprintf("line %d: %s", e.Line, e.Message)
	}
	return e.Message
}

type Assembler struct{}

func NewAssembler() *Assembler {
	return &Assembler{}
}

func Assemble(source string) (*AssembleResult, error) {
	return NewAssembler().Assemble(source)
}

type segment int

const (
	segmentText segment = iota
	segmentData
)

type sourceItem struct {
	lineNo     int
	segment    segment
	opcode     string
	operands   []string
	textOffset int
	dataOffset int
}

type labelDef struct {
	segment segment
	offset  int
}

type assemblyPlan struct {
	items    []sourceItem
	labels   map[string]labelDef
	textSize int
	dataSize int
}

func (a *Assembler) Assemble(source string) (*AssembleResult, error) {
	plan, err := a.plan(source)
	if err != nil {
		return nil, err
	}

	textLabels := map[string]int{}
	dataLabels := map[string]int{}
	allLabels := map[string]int{}
	for name, def := range plan.labels {
		offset := def.offset
		if def.segment == segmentData {
			offset = plan.textSize + def.offset
			dataLabels[name] = offset
		} else {
			textLabels[name] = offset
		}
		allLabels[name] = offset
	}

	words := []uint32{}
	data := []byte{}
	for _, item := range plan.items {
		if item.segment == segmentData {
			emitted, err := emitData(item, allLabels)
			if err != nil {
				return nil, err
			}
			data = append(data, emitted...)
			continue
		}
		emitted, err := encodeInstruction(item, allLabels)
		if err != nil {
			return nil, err
		}
		words = append(words, emitted...)
	}

	textBytes := riscv.Assemble(words)
	image := append([]byte{}, textBytes...)
	image = append(image, data...)

	return &AssembleResult{
		Bytes:        image,
		Instructions: words,
		Data:         data,
		LabelOffsets: textLabels,
		DataOffsets:  dataLabels,
		TextSize:     plan.textSize,
	}, nil
}

func (a *Assembler) plan(source string) (*assemblyPlan, error) {
	current := segmentText
	textOffset := 0
	dataOffset := 0
	labels := map[string]labelDef{}
	items := []sourceItem{}

	for index, raw := range strings.Split(source, "\n") {
		lineNo := index + 1
		line := stripComment(raw)
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		for {
			label, rest, ok := splitLeadingLabel(line)
			if !ok {
				break
			}
			if _, exists := labels[label]; exists {
				return nil, lineError(lineNo, "duplicate label %q", label)
			}
			offset := textOffset
			if current == segmentData {
				offset = dataOffset
			}
			labels[label] = labelDef{segment: current, offset: offset}
			line = strings.TrimSpace(rest)
			if line == "" {
				break
			}
		}
		if line == "" {
			continue
		}

		opcode, operands := splitOpcodeAndOperands(line)
		opcode = strings.ToLower(opcode)
		item := sourceItem{
			lineNo:     lineNo,
			segment:    current,
			opcode:     opcode,
			operands:   operands,
			textOffset: textOffset,
			dataOffset: dataOffset,
		}

		if strings.HasPrefix(opcode, ".") {
			switch opcode {
			case ".text":
				current = segmentText
				continue
			case ".data":
				current = segmentData
				continue
			case ".globl", ".global":
				continue
			}
			if current != segmentData {
				return nil, lineError(lineNo, "directive %s is only supported in .data", opcode)
			}
			size, err := dataDirectiveSize(item)
			if err != nil {
				return nil, err
			}
			item.segment = segmentData
			item.dataOffset = dataOffset
			items = append(items, item)
			dataOffset += size
			continue
		}

		if current != segmentText {
			return nil, lineError(lineNo, "instruction %s is only supported in .text", opcode)
		}
		size, err := instructionSize(item)
		if err != nil {
			return nil, err
		}
		item.textOffset = textOffset
		items = append(items, item)
		textOffset += size * 4
	}

	return &assemblyPlan{
		items:    items,
		labels:   labels,
		textSize: textOffset,
		dataSize: dataOffset,
	}, nil
}

func stripComment(line string) string {
	cut := len(line)
	for _, marker := range []string{";", "#", "//"} {
		if index := strings.Index(line, marker); index >= 0 && index < cut {
			cut = index
		}
	}
	return line[:cut]
}

func splitLeadingLabel(line string) (string, string, bool) {
	colon := strings.Index(line, ":")
	if colon < 0 {
		return "", line, false
	}
	candidate := strings.TrimSpace(line[:colon])
	if !isIdentifier(candidate) {
		return "", line, false
	}
	return candidate, line[colon+1:], true
}

func splitOpcodeAndOperands(line string) (string, []string) {
	line = strings.TrimSpace(line)
	for index, r := range line {
		if unicode.IsSpace(r) {
			opcode := line[:index]
			rest := strings.TrimSpace(line[index:])
			return opcode, splitOperands(rest)
		}
	}
	return line, nil
}

func splitOperands(rest string) []string {
	if rest == "" {
		return nil
	}
	parts := strings.Split(rest, ",")
	operands := make([]string, 0, len(parts))
	for _, part := range parts {
		trimmed := strings.TrimSpace(part)
		if trimmed != "" {
			operands = append(operands, trimmed)
		}
	}
	return operands
}

func isIdentifier(value string) bool {
	if value == "" {
		return false
	}
	for index, r := range value {
		if index == 0 {
			if !(unicode.IsLetter(r) || r == '_' || r == '.') {
				return false
			}
			continue
		}
		if !(unicode.IsLetter(r) || unicode.IsDigit(r) || r == '_' || r == '.' || r == '$') {
			return false
		}
	}
	return true
}

func instructionSize(item sourceItem) (int, error) {
	switch item.opcode {
	case "li":
		if err := requireOperands(item, 2); err != nil {
			return 0, err
		}
		imm, err := parseNumberOperand(item, item.operands[1])
		if err != nil {
			return 0, err
		}
		return loadConstSize(imm), nil
	case "la":
		if err := requireOperands(item, 2); err != nil {
			return 0, err
		}
		return 2, nil
	case "halt":
		if err := requireOperands(item, 0); err != nil {
			return 0, err
		}
		return 2, nil
	case "mv", "j", "call", "ret", "nop",
		"add", "sub", "sll", "slt", "sltu", "xor", "srl", "sra", "or", "and",
		"addi", "slti", "sltiu", "xori", "ori", "andi", "slli", "srli", "srai",
		"lb", "lh", "lw", "lbu", "lhu",
		"sb", "sh", "sw",
		"beq", "bne", "blt", "bge", "bltu", "bgeu",
		"jal", "jalr", "lui", "auipc", "ecall", "mret", "csrrw", "csrrs", "csrrc":
		return 1, nil
	default:
		return 0, lineError(item.lineNo, "unsupported instruction %q", item.opcode)
	}
}

func dataDirectiveSize(item sourceItem) (int, error) {
	switch item.opcode {
	case ".byte":
		if len(item.operands) == 0 {
			return 0, lineError(item.lineNo, ".byte requires at least one value")
		}
		return len(item.operands), nil
	case ".word":
		if len(item.operands) == 0 {
			return 0, lineError(item.lineNo, ".word requires at least one value")
		}
		return len(item.operands) * 4, nil
	case ".zero", ".space":
		if err := requireOperands(item, 1); err != nil {
			return 0, err
		}
		size, err := parseNumberOperand(item, item.operands[0])
		if err != nil {
			return 0, err
		}
		if size < 0 {
			return 0, lineError(item.lineNo, "%s size must be non-negative", item.opcode)
		}
		return size, nil
	default:
		return 0, lineError(item.lineNo, "unsupported directive %q", item.opcode)
	}
}

func encodeInstruction(item sourceItem, labels map[string]int) ([]uint32, error) {
	switch item.opcode {
	case "li":
		if err := requireOperands(item, 2); err != nil {
			return nil, err
		}
		rd, err := parseRegisterOperand(item, item.operands[0])
		if err != nil {
			return nil, err
		}
		imm, err := parseNumberOperand(item, item.operands[1])
		if err != nil {
			return nil, err
		}
		return emitLoadConst(rd, imm), nil
	case "la":
		if err := requireOperands(item, 2); err != nil {
			return nil, err
		}
		rd, err := parseRegisterOperand(item, item.operands[0])
		if err != nil {
			return nil, err
		}
		address, err := parseLabelOperand(item, item.operands[1], labels)
		if err != nil {
			return nil, err
		}
		return emitLoadAddress(rd, address), nil
	case "mv":
		if err := requireOperands(item, 2); err != nil {
			return nil, err
		}
		rd, rs, err := parseTwoRegs(item)
		if err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeAddi(rd, rs, 0)}, nil
	case "j":
		if err := requireOperands(item, 1); err != nil {
			return nil, err
		}
		offset, err := branchTargetOffset(item, item.operands[0], labels, 21)
		if err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeJal(0, offset)}, nil
	case "call":
		if err := requireOperands(item, 1); err != nil {
			return nil, err
		}
		offset, err := branchTargetOffset(item, item.operands[0], labels, 21)
		if err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeJal(1, offset)}, nil
	case "ret":
		if err := requireOperands(item, 0); err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeJalr(0, 1, 0)}, nil
	case "nop":
		if err := requireOperands(item, 0); err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeAddi(0, 0, 0)}, nil
	case "halt":
		if err := requireOperands(item, 0); err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeAddi(17, 0, 10), riscv.EncodeEcall()}, nil
	case "add":
		return encodeThreeReg(item, riscv.EncodeAdd)
	case "sub":
		return encodeThreeReg(item, riscv.EncodeSub)
	case "sll":
		return encodeThreeReg(item, riscv.EncodeSll)
	case "slt":
		return encodeThreeReg(item, riscv.EncodeSlt)
	case "sltu":
		return encodeThreeReg(item, riscv.EncodeSltu)
	case "xor":
		return encodeThreeReg(item, riscv.EncodeXor)
	case "srl":
		return encodeThreeReg(item, riscv.EncodeSrl)
	case "sra":
		return encodeThreeReg(item, riscv.EncodeSra)
	case "or":
		return encodeThreeReg(item, riscv.EncodeOr)
	case "and":
		return encodeThreeReg(item, riscv.EncodeAnd)
	case "addi":
		return encodeRegImm(item, riscv.EncodeAddi)
	case "slti":
		return encodeRegImm(item, riscv.EncodeSlti)
	case "sltiu":
		return encodeRegImm(item, riscv.EncodeSltiu)
	case "xori":
		return encodeRegImm(item, riscv.EncodeXori)
	case "ori":
		return encodeRegImm(item, riscv.EncodeOri)
	case "andi":
		return encodeRegImm(item, riscv.EncodeAndi)
	case "slli":
		return encodeShiftImm(item, riscv.EncodeSlli)
	case "srli":
		return encodeShiftImm(item, riscv.EncodeSrli)
	case "srai":
		return encodeShiftImm(item, riscv.EncodeSrai)
	case "lb":
		return encodeLoad(item, riscv.EncodeLb)
	case "lh":
		return encodeLoad(item, riscv.EncodeLh)
	case "lw":
		return encodeLoad(item, riscv.EncodeLw)
	case "lbu":
		return encodeLoad(item, riscv.EncodeLbu)
	case "lhu":
		return encodeLoad(item, riscv.EncodeLhu)
	case "sb":
		return encodeStore(item, riscv.EncodeSb)
	case "sh":
		return encodeStore(item, riscv.EncodeSh)
	case "sw":
		return encodeStore(item, riscv.EncodeSw)
	case "beq":
		return encodeBranch(item, labels, riscv.EncodeBeq)
	case "bne":
		return encodeBranch(item, labels, riscv.EncodeBne)
	case "blt":
		return encodeBranch(item, labels, riscv.EncodeBlt)
	case "bge":
		return encodeBranch(item, labels, riscv.EncodeBge)
	case "bltu":
		return encodeBranch(item, labels, riscv.EncodeBltu)
	case "bgeu":
		return encodeBranch(item, labels, riscv.EncodeBgeu)
	case "jal":
		return encodeJal(item, labels)
	case "jalr":
		return encodeJalr(item)
	case "lui":
		return encodeUpper(item, riscv.EncodeLui)
	case "auipc":
		return encodeUpper(item, riscv.EncodeAuipc)
	case "ecall":
		if err := requireOperands(item, 0); err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeEcall()}, nil
	case "mret":
		if err := requireOperands(item, 0); err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeMret()}, nil
	case "csrrw":
		return encodeCSR(item, riscv.EncodeCsrrw)
	case "csrrs":
		return encodeCSR(item, riscv.EncodeCsrrs)
	case "csrrc":
		return encodeCSR(item, riscv.EncodeCsrrc)
	default:
		return nil, lineError(item.lineNo, "unsupported instruction %q", item.opcode)
	}
}

func encodeThreeReg(item sourceItem, encode func(rd, rs1, rs2 int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 3); err != nil {
		return nil, err
	}
	rd, rs1, rs2, err := parseThreeRegs(item)
	if err != nil {
		return nil, err
	}
	return []uint32{encode(rd, rs1, rs2)}, nil
}

func encodeRegImm(item sourceItem, encode func(rd, rs1, imm int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 3); err != nil {
		return nil, err
	}
	rd, rs1, err := parseTwoRegs(item)
	if err != nil {
		return nil, err
	}
	imm, err := parseNumberOperand(item, item.operands[2])
	if err != nil {
		return nil, err
	}
	if !fitsSigned(imm, 12) {
		return nil, lineError(item.lineNo, "immediate %d is outside signed 12-bit range", imm)
	}
	return []uint32{encode(rd, rs1, imm)}, nil
}

func encodeShiftImm(item sourceItem, encode func(rd, rs1, imm int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 3); err != nil {
		return nil, err
	}
	rd, rs1, err := parseTwoRegs(item)
	if err != nil {
		return nil, err
	}
	shamt, err := parseNumberOperand(item, item.operands[2])
	if err != nil {
		return nil, err
	}
	if shamt < 0 || shamt > 31 {
		return nil, lineError(item.lineNo, "shift amount %d is outside 0..31", shamt)
	}
	return []uint32{encode(rd, rs1, shamt)}, nil
}

func encodeLoad(item sourceItem, encode func(rd, rs1, imm int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 2); err != nil {
		return nil, err
	}
	rd, err := parseRegisterOperand(item, item.operands[0])
	if err != nil {
		return nil, err
	}
	imm, rs1, err := parseMemoryOperand(item, item.operands[1])
	if err != nil {
		return nil, err
	}
	return []uint32{encode(rd, rs1, imm)}, nil
}

func encodeStore(item sourceItem, encode func(rs2, rs1, imm int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 2); err != nil {
		return nil, err
	}
	rs2, err := parseRegisterOperand(item, item.operands[0])
	if err != nil {
		return nil, err
	}
	imm, rs1, err := parseMemoryOperand(item, item.operands[1])
	if err != nil {
		return nil, err
	}
	return []uint32{encode(rs2, rs1, imm)}, nil
}

func encodeBranch(item sourceItem, labels map[string]int, encode func(rs1, rs2, offset int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 3); err != nil {
		return nil, err
	}
	rs1, rs2, err := parseTwoRegs(item)
	if err != nil {
		return nil, err
	}
	offset, err := branchTargetOffset(item, item.operands[2], labels, 13)
	if err != nil {
		return nil, err
	}
	return []uint32{encode(rs1, rs2, offset)}, nil
}

func encodeJal(item sourceItem, labels map[string]int) ([]uint32, error) {
	if len(item.operands) != 1 && len(item.operands) != 2 {
		return nil, lineError(item.lineNo, "jal expects 1 or 2 operands")
	}
	rd := 1
	target := item.operands[0]
	if len(item.operands) == 2 {
		parsed, err := parseRegisterOperand(item, item.operands[0])
		if err != nil {
			return nil, err
		}
		rd = parsed
		target = item.operands[1]
	}
	offset, err := branchTargetOffset(item, target, labels, 21)
	if err != nil {
		return nil, err
	}
	return []uint32{riscv.EncodeJal(rd, offset)}, nil
}

func encodeJalr(item sourceItem) ([]uint32, error) {
	if len(item.operands) == 2 {
		rd, err := parseRegisterOperand(item, item.operands[0])
		if err != nil {
			return nil, err
		}
		imm, rs1, err := parseMemoryOperand(item, item.operands[1])
		if err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeJalr(rd, rs1, imm)}, nil
	}
	if err := requireOperands(item, 3); err != nil {
		return nil, err
	}
	rd, rs1, err := parseTwoRegs(item)
	if err != nil {
		return nil, err
	}
	imm, err := parseNumberOperand(item, item.operands[2])
	if err != nil {
		return nil, err
	}
	if !fitsSigned(imm, 12) {
		return nil, lineError(item.lineNo, "jalr immediate %d is outside signed 12-bit range", imm)
	}
	return []uint32{riscv.EncodeJalr(rd, rs1, imm)}, nil
}

func encodeUpper(item sourceItem, encode func(rd, imm int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 2); err != nil {
		return nil, err
	}
	rd, err := parseRegisterOperand(item, item.operands[0])
	if err != nil {
		return nil, err
	}
	imm, err := parseNumberOperand(item, item.operands[1])
	if err != nil {
		return nil, err
	}
	if !fitsSigned(imm, 32) {
		return nil, lineError(item.lineNo, "upper immediate %d is outside 32-bit range", imm)
	}
	return []uint32{encode(rd, imm)}, nil
}

func encodeCSR(item sourceItem, encode func(rd, csr, rs1 int) uint32) ([]uint32, error) {
	if err := requireOperands(item, 3); err != nil {
		return nil, err
	}
	rd, err := parseRegisterOperand(item, item.operands[0])
	if err != nil {
		return nil, err
	}
	csr, err := parseCSR(item, item.operands[1])
	if err != nil {
		return nil, err
	}
	rs1, err := parseRegisterOperand(item, item.operands[2])
	if err != nil {
		return nil, err
	}
	return []uint32{encode(rd, csr, rs1)}, nil
}

func emitData(item sourceItem, labels map[string]int) ([]byte, error) {
	out := []byte{}
	switch item.opcode {
	case ".byte":
		for _, operand := range item.operands {
			value, err := parseImmediateOrLabel(item, operand, labels)
			if err != nil {
				return nil, err
			}
			out = append(out, byte(value&0xFF))
		}
	case ".word":
		for _, operand := range item.operands {
			value, err := parseImmediateOrLabel(item, operand, labels)
			if err != nil {
				return nil, err
			}
			out = append(out, byte(value&0xFF), byte((value>>8)&0xFF), byte((value>>16)&0xFF), byte((value>>24)&0xFF))
		}
	case ".zero", ".space":
		size, err := parseNumberOperand(item, item.operands[0])
		if err != nil {
			return nil, err
		}
		out = make([]byte, size)
	default:
		return nil, lineError(item.lineNo, "unsupported directive %q", item.opcode)
	}
	return out, nil
}

func parseThreeRegs(item sourceItem) (int, int, int, error) {
	rd, err := parseRegisterOperand(item, item.operands[0])
	if err != nil {
		return 0, 0, 0, err
	}
	rs1, err := parseRegisterOperand(item, item.operands[1])
	if err != nil {
		return 0, 0, 0, err
	}
	rs2, err := parseRegisterOperand(item, item.operands[2])
	if err != nil {
		return 0, 0, 0, err
	}
	return rd, rs1, rs2, nil
}

func parseTwoRegs(item sourceItem) (int, int, error) {
	first, err := parseRegisterOperand(item, item.operands[0])
	if err != nil {
		return 0, 0, err
	}
	second, err := parseRegisterOperand(item, item.operands[1])
	if err != nil {
		return 0, 0, err
	}
	return first, second, nil
}

func parseRegisterOperand(item sourceItem, operand string) (int, error) {
	reg, ok := registerNames[strings.ToLower(strings.TrimSpace(operand))]
	if !ok {
		return 0, lineError(item.lineNo, "unknown register %q", operand)
	}
	return reg, nil
}

func parseMemoryOperand(item sourceItem, operand string) (int, int, error) {
	open := strings.Index(operand, "(")
	close := strings.LastIndex(operand, ")")
	if open < 0 || close != len(operand)-1 || close < open {
		return 0, 0, lineError(item.lineNo, "memory operand %q must look like imm(reg)", operand)
	}
	immText := strings.TrimSpace(operand[:open])
	baseText := strings.TrimSpace(operand[open+1 : close])
	imm := 0
	if immText != "" {
		parsed, err := parseNumberOperand(item, immText)
		if err != nil {
			return 0, 0, err
		}
		imm = parsed
	}
	if !fitsSigned(imm, 12) {
		return 0, 0, lineError(item.lineNo, "memory immediate %d is outside signed 12-bit range", imm)
	}
	base, err := parseRegisterOperand(item, baseText)
	if err != nil {
		return 0, 0, err
	}
	return imm, base, nil
}

func branchTargetOffset(item sourceItem, operand string, labels map[string]int, bits int) (int, error) {
	if value, ok, err := parseNumber(operand); err != nil {
		return 0, lineError(item.lineNo, "invalid immediate %q", operand)
	} else if ok {
		if err := validateRelativeOffset(item, value, bits); err != nil {
			return 0, err
		}
		return value, nil
	}

	target, err := parseLabelOperand(item, operand, labels)
	if err != nil {
		return 0, err
	}
	offset := target - item.textOffset
	if err := validateRelativeOffset(item, offset, bits); err != nil {
		return 0, err
	}
	return offset, nil
}

func validateRelativeOffset(item sourceItem, offset int, bits int) error {
	if offset%2 != 0 {
		return lineError(item.lineNo, "relative offset %d is not 2-byte aligned", offset)
	}
	if !fitsSigned(offset, bits) {
		return lineError(item.lineNo, "relative offset %d is outside signed %d-bit range", offset, bits)
	}
	return nil
}

func parseLabelOperand(item sourceItem, operand string, labels map[string]int) (int, error) {
	label := strings.TrimSpace(operand)
	value, ok := labels[label]
	if !ok {
		return 0, lineError(item.lineNo, "unknown label %q", operand)
	}
	return value, nil
}

func parseImmediateOrLabel(item sourceItem, operand string, labels map[string]int) (int, error) {
	if value, ok, err := parseNumber(operand); err != nil {
		return 0, lineError(item.lineNo, "invalid immediate %q", operand)
	} else if ok {
		return value, nil
	}
	return parseLabelOperand(item, operand, labels)
}

func parseNumberOperand(item sourceItem, operand string) (int, error) {
	value, ok, err := parseNumber(operand)
	if err != nil {
		return 0, lineError(item.lineNo, "invalid immediate %q", operand)
	}
	if !ok {
		return 0, lineError(item.lineNo, "expected numeric immediate, got %q", operand)
	}
	return value, nil
}

func parseNumber(operand string) (int, bool, error) {
	text := strings.TrimSpace(operand)
	if text == "" {
		return 0, false, nil
	}
	if strings.HasPrefix(text, "'") && strings.HasSuffix(text, "'") && len(text) >= 3 {
		unquoted, err := strconv.Unquote(text)
		if err != nil {
			return 0, true, err
		}
		runes := []rune(unquoted)
		if len(runes) != 1 {
			return 0, true, fmt.Errorf("character literal must contain one rune")
		}
		return int(runes[0]), true, nil
	}

	start := 0
	if text[0] == '-' || text[0] == '+' {
		start = 1
	}
	if start == len(text) || !(unicode.IsDigit(rune(text[start]))) {
		return 0, false, nil
	}
	value, err := strconv.ParseInt(text, 0, strconv.IntSize)
	if err != nil {
		return 0, true, err
	}
	return int(value), true, nil
}

func parseCSR(item sourceItem, operand string) (int, error) {
	switch strings.ToLower(strings.TrimSpace(operand)) {
	case "mstatus":
		return riscv.CSRMstatus, nil
	case "mtvec":
		return riscv.CSRMtvec, nil
	case "mscratch":
		return riscv.CSRMscratch, nil
	case "mepc":
		return riscv.CSRMepc, nil
	case "mcause":
		return riscv.CSRMcause, nil
	default:
		return parseNumberOperand(item, operand)
	}
}

func emitLoadConst(rd int, value int) []uint32 {
	if fitsSigned(value, 12) {
		return []uint32{riscv.EncodeAddi(rd, 0, value)}
	}
	upper, lower := splitUpperLower(value)
	words := []uint32{riscv.EncodeLui(rd, upper)}
	if lower != 0 {
		words = append(words, riscv.EncodeAddi(rd, rd, lower))
	}
	return words
}

func emitLoadAddress(rd int, address int) []uint32 {
	upper, lower := splitUpperLower(address)
	return []uint32{
		riscv.EncodeLui(rd, upper),
		riscv.EncodeAddi(rd, rd, lower),
	}
}

func loadConstSize(value int) int {
	if fitsSigned(value, 12) {
		return 1
	}
	_, lower := splitUpperLower(value)
	if lower == 0 {
		return 1
	}
	return 2
}

func splitUpperLower(value int) (upper, lower int) {
	upper = (value + 0x800) >> 12
	lower = value - (upper << 12)
	return upper, lower
}

func fitsSigned(value, bits int) bool {
	min := -(1 << (bits - 1))
	max := (1 << (bits - 1)) - 1
	return value >= min && value <= max
}

func requireOperands(item sourceItem, count int) error {
	if len(item.operands) != count {
		return lineError(item.lineNo, "%s expects %d operands, got %d", item.opcode, count, len(item.operands))
	}
	return nil
}

func lineError(line int, format string, args ...any) error {
	return &AssemblyError{
		Line:    line,
		Message: fmt.Sprintf(format, args...),
	}
}

var registerNames = map[string]int{
	"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4, "x5": 5, "x6": 6, "x7": 7,
	"x8": 8, "x9": 9, "x10": 10, "x11": 11, "x12": 12, "x13": 13, "x14": 14, "x15": 15,
	"x16": 16, "x17": 17, "x18": 18, "x19": 19, "x20": 20, "x21": 21, "x22": 22, "x23": 23,
	"x24": 24, "x25": 25, "x26": 26, "x27": 27, "x28": 28, "x29": 29, "x30": 30, "x31": 31,

	"zero": 0, "ra": 1, "sp": 2, "gp": 3, "tp": 4,
	"t0": 5, "t1": 6, "t2": 7, "s0": 8, "fp": 8, "s1": 9,
	"a0": 10, "a1": 11, "a2": 12, "a3": 13, "a4": 14, "a5": 15, "a6": 16, "a7": 17,
	"s2": 18, "s3": 19, "s4": 20, "s5": 21, "s6": 22, "s7": 23, "s8": 24, "s9": 25, "s10": 26, "s11": 27,
	"t3": 28, "t4": 29, "t5": 30, "t6": 31,
}
