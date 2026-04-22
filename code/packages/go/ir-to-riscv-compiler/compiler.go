package irtoriscvcompiler

import (
	"fmt"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	sm "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const (
	x0              = 0
	xRa             = 1
	xSyscallNumber  = 17
	xBackendScratch = 31
)

// MachineCodeResult is the output of lowering IR to a flat RISC-V image.
type MachineCodeResult struct {
	Bytes           []byte
	Instructions    []uint32
	LabelOffsets    map[string]int
	DataOffsets     map[string]int
	IrToMachineCode *sm.IrToMachineCode
}

// LoweringError reports an IR instruction that could not be lowered.
type LoweringError struct {
	IrID    int
	Opcode  ir.IrOp
	Message string
}

func (e LoweringError) Error() string {
	if e.IrID >= 0 {
		return fmt.Sprintf("lower %s #%d: %s", e.Opcode, e.IrID, e.Message)
	}
	return fmt.Sprintf("lower %s: %s", e.Opcode, e.Message)
}

type compilePlan struct {
	labelOffsets    map[string]int
	dataOffsets     map[string]int
	textSize        int
	entryTrampoline bool
}

// IrToRiscVCompiler lowers compiler IR to RV32I machine code.
type IrToRiscVCompiler struct{}

func NewIrToRiscVCompiler() *IrToRiscVCompiler {
	return &IrToRiscVCompiler{}
}

func (c *IrToRiscVCompiler) Compile(program *ir.IrProgram) (*MachineCodeResult, error) {
	if program == nil {
		return nil, fmt.Errorf("program is nil")
	}

	plan, err := c.plan(program)
	if err != nil {
		return nil, err
	}

	words := make([]uint32, 0, plan.textSize/4)
	if plan.entryTrampoline {
		entryOffset := plan.labelOffsets[program.EntryLabel]
		if err := validateJalOffset(entryOffset); err != nil {
			return nil, err
		}
		words = append(words, riscv.EncodeJal(x0, entryOffset))
	}

	irToMC := &sm.IrToMachineCode{}
	for _, instruction := range program.Instructions {
		pc := len(words) * 4
		emitted, err := c.emitInstruction(instruction, pc, plan)
		if err != nil {
			return nil, err
		}
		if instruction.ID >= 0 && len(emitted) > 0 {
			irToMC.Add(instruction.ID, pc, len(emitted)*4)
		}
		words = append(words, emitted...)
	}

	image := riscv.Assemble(words)
	image = append(image, dataBytes(program.Data)...)

	return &MachineCodeResult{
		Bytes:           image,
		Instructions:    words,
		LabelOffsets:    plan.labelOffsets,
		DataOffsets:     plan.dataOffsets,
		IrToMachineCode: irToMC,
	}, nil
}

func (c *IrToRiscVCompiler) plan(program *ir.IrProgram) (*compilePlan, error) {
	labelOffsets := map[string]int{}
	pc := 0
	for _, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpLabel {
			label, err := labelOperand(instruction, 0)
			if err != nil {
				return nil, err
			}
			if _, exists := labelOffsets[label.Name]; exists {
				return nil, loweringError(instruction, "duplicate label %q", label.Name)
			}
			labelOffsets[label.Name] = pc
		}

		size, err := c.instructionSize(instruction)
		if err != nil {
			return nil, err
		}
		pc += size * 4
	}

	entryTrampoline := false
	if program.EntryLabel != "" {
		if entryOffset, ok := labelOffsets[program.EntryLabel]; ok && entryOffset != 0 {
			entryTrampoline = true
			for name, offset := range labelOffsets {
				labelOffsets[name] = offset + 4
			}
			pc += 4
		}
	}

	dataOffsets := map[string]int{}
	dataPC := pc
	for _, decl := range program.Data {
		if decl.Size < 0 {
			return nil, fmt.Errorf("data %q has negative size %d", decl.Label, decl.Size)
		}
		if _, exists := dataOffsets[decl.Label]; exists {
			return nil, fmt.Errorf("duplicate data label %q", decl.Label)
		}
		dataOffsets[decl.Label] = dataPC
		dataPC += decl.Size
	}

	return &compilePlan{
		labelOffsets:    labelOffsets,
		dataOffsets:     dataOffsets,
		textSize:        pc,
		entryTrampoline: entryTrampoline,
	}, nil
}

func (c *IrToRiscVCompiler) instructionSize(instruction ir.IrInstruction) (int, error) {
	switch instruction.Opcode {
	case ir.OpLabel, ir.OpComment:
		return 0, nil
	case ir.OpLoadImm:
		imm, err := immOperand(instruction, 1)
		if err != nil {
			return 0, err
		}
		return loadConstSize(imm.Value), nil
	case ir.OpLoadAddr:
		return 2, nil
	case ir.OpLoadByte, ir.OpLoadWord, ir.OpStoreByte, ir.OpStoreWord:
		if _, ok := thirdOperandAsImmediate(instruction); ok {
			return 1, nil
		}
		return 2, nil
	case ir.OpAdd, ir.OpSub, ir.OpAnd, ir.OpCmpLt, ir.OpCmpGt:
		return 1, nil
	case ir.OpCmpEq, ir.OpCmpNe:
		return 2, nil
	case ir.OpAddImm, ir.OpAndImm:
		imm, err := immOperand(instruction, 2)
		if err != nil {
			return 0, err
		}
		if fitsSigned(imm.Value, 12) {
			return 1, nil
		}
		return loadConstSize(imm.Value) + 1, nil
	case ir.OpJump, ir.OpBranchZ, ir.OpBranchNz, ir.OpCall, ir.OpRet, ir.OpNop:
		return 1, nil
	case ir.OpSyscall:
		imm, err := immOperand(instruction, 0)
		if err != nil {
			return 0, err
		}
		return loadConstSize(imm.Value) + 1, nil
	case ir.OpHalt:
		return 2, nil
	default:
		return 0, loweringError(instruction, "unsupported opcode")
	}
}

func (c *IrToRiscVCompiler) emitInstruction(instruction ir.IrInstruction, pc int, plan *compilePlan) ([]uint32, error) {
	switch instruction.Opcode {
	case ir.OpLabel, ir.OpComment:
		return nil, nil
	case ir.OpLoadImm:
		dst, err := physicalRegOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		imm, err := immOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		return emitLoadConst(dst, imm.Value), nil
	case ir.OpLoadAddr:
		dst, err := physicalRegOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		label, err := labelOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		offset, ok := plan.dataOffsets[label.Name]
		if !ok {
			offset, ok = plan.labelOffsets[label.Name]
		}
		if !ok {
			return nil, loweringError(instruction, "unknown label %q", label.Name)
		}
		return emitLoadAddress(dst, offset), nil
	case ir.OpLoadByte:
		return c.emitLoad(instruction, riscv.EncodeLbu)
	case ir.OpLoadWord:
		return c.emitLoad(instruction, riscv.EncodeLw)
	case ir.OpStoreByte:
		return c.emitStore(instruction, riscv.EncodeSb)
	case ir.OpStoreWord:
		return c.emitStore(instruction, riscv.EncodeSw)
	case ir.OpAdd:
		return emitThreeReg(instruction, riscv.EncodeAdd)
	case ir.OpSub:
		return emitThreeReg(instruction, riscv.EncodeSub)
	case ir.OpAnd:
		return emitThreeReg(instruction, riscv.EncodeAnd)
	case ir.OpAddImm:
		return emitRegImm(instruction, riscv.EncodeAddi, riscv.EncodeAdd)
	case ir.OpAndImm:
		return emitRegImm(instruction, riscv.EncodeAndi, riscv.EncodeAnd)
	case ir.OpCmpEq:
		return emitCmpEqNe(instruction, true)
	case ir.OpCmpNe:
		return emitCmpEqNe(instruction, false)
	case ir.OpCmpLt:
		return emitThreeReg(instruction, riscv.EncodeSlt)
	case ir.OpCmpGt:
		return emitCmpGt(instruction)
	case ir.OpJump:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		offset, err := jalOffset(instruction, label, pc, plan)
		if err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeJal(x0, offset)}, nil
	case ir.OpBranchZ, ir.OpBranchNz:
		return emitBranch(instruction, pc, plan)
	case ir.OpCall:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		offset, err := jalOffset(instruction, label, pc, plan)
		if err != nil {
			return nil, err
		}
		return []uint32{riscv.EncodeJal(xRa, offset)}, nil
	case ir.OpRet:
		return []uint32{riscv.EncodeJalr(x0, xRa, 0)}, nil
	case ir.OpSyscall:
		imm, err := immOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		words := emitLoadConst(xSyscallNumber, imm.Value)
		words = append(words, riscv.EncodeEcall())
		return words, nil
	case ir.OpHalt:
		return []uint32{riscv.EncodeAddi(xSyscallNumber, x0, 10), riscv.EncodeEcall()}, nil
	case ir.OpNop:
		return []uint32{riscv.EncodeAddi(x0, x0, 0)}, nil
	default:
		return nil, loweringError(instruction, "unsupported opcode")
	}
}

func (c *IrToRiscVCompiler) emitLoad(instruction ir.IrInstruction, encode func(rd, rs1, imm int) uint32) ([]uint32, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	base, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		if !fitsSigned(imm.Value, 12) {
			return nil, loweringError(instruction, "memory immediate %d is outside signed 12-bit range", imm.Value)
		}
		return []uint32{encode(dst, base, imm.Value)}, nil
	}
	offset, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	return []uint32{
		riscv.EncodeAdd(xBackendScratch, base, offset),
		encode(dst, xBackendScratch, 0),
	}, nil
}

func (c *IrToRiscVCompiler) emitStore(instruction ir.IrInstruction, encode func(rs2, rs1, imm int) uint32) ([]uint32, error) {
	src, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	base, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		if !fitsSigned(imm.Value, 12) {
			return nil, loweringError(instruction, "memory immediate %d is outside signed 12-bit range", imm.Value)
		}
		return []uint32{encode(src, base, imm.Value)}, nil
	}
	offset, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	return []uint32{
		riscv.EncodeAdd(xBackendScratch, base, offset),
		encode(src, xBackendScratch, 0),
	}, nil
}

func emitThreeReg(instruction ir.IrInstruction, encode func(rd, rs1, rs2 int) uint32) ([]uint32, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	left, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	right, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	return []uint32{encode(dst, left, right)}, nil
}

func emitRegImm(instruction ir.IrInstruction, encodeImm func(rd, rs1, imm int) uint32, encodeReg func(rd, rs1, rs2 int) uint32) ([]uint32, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	src, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	imm, err := immOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	if fitsSigned(imm.Value, 12) {
		return []uint32{encodeImm(dst, src, imm.Value)}, nil
	}
	words := emitLoadConst(xBackendScratch, imm.Value)
	words = append(words, encodeReg(dst, src, xBackendScratch))
	return words, nil
}

func emitCmpEqNe(instruction ir.IrInstruction, equal bool) ([]uint32, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	left, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	right, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	if equal {
		return []uint32{
			riscv.EncodeXor(dst, left, right),
			riscv.EncodeSltiu(dst, dst, 1),
		}, nil
	}
	return []uint32{
		riscv.EncodeXor(dst, left, right),
		riscv.EncodeSltu(dst, x0, dst),
	}, nil
}

func emitCmpGt(instruction ir.IrInstruction) ([]uint32, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	left, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	right, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	return []uint32{riscv.EncodeSlt(dst, right, left)}, nil
}

func emitBranch(instruction ir.IrInstruction, pc int, plan *compilePlan) ([]uint32, error) {
	register, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	label, err := labelOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	target, ok := plan.labelOffsets[label.Name]
	if !ok {
		return nil, loweringError(instruction, "unknown label %q", label.Name)
	}
	offset := target - pc
	if err := validateBranchOffset(offset); err != nil {
		return nil, loweringError(instruction, "%v", err)
	}
	if instruction.Opcode == ir.OpBranchZ {
		return []uint32{riscv.EncodeBeq(register, x0, offset)}, nil
	}
	return []uint32{riscv.EncodeBne(register, x0, offset)}, nil
}

func jalOffset(instruction ir.IrInstruction, label ir.IrLabel, pc int, plan *compilePlan) (int, error) {
	target, ok := plan.labelOffsets[label.Name]
	if !ok {
		return 0, loweringError(instruction, "unknown label %q", label.Name)
	}
	offset := target - pc
	if err := validateJalOffset(offset); err != nil {
		return 0, loweringError(instruction, "%v", err)
	}
	return offset, nil
}

func emitLoadConst(rd, value int) []uint32 {
	if fitsSigned(value, 12) {
		return []uint32{riscv.EncodeAddi(rd, x0, value)}
	}
	upper, lower := splitUpperLower(value)
	words := []uint32{riscv.EncodeLui(rd, upper)}
	if lower != 0 {
		words = append(words, riscv.EncodeAddi(rd, rd, lower))
	}
	return words
}

func emitLoadAddress(rd, address int) []uint32 {
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

func physicalRegOperand(instruction ir.IrInstruction, index int) (int, error) {
	reg, err := regOperand(instruction, index)
	if err != nil {
		return 0, err
	}
	physical, ok := physicalRegister(reg.Index)
	if !ok {
		return 0, loweringError(instruction, "virtual register v%d has no physical mapping in the starter allocator", reg.Index)
	}
	return physical, nil
}

func physicalRegister(index int) (int, bool) {
	fixed := map[int]int{
		0: 5,  // tape/data base
		1: 6,  // tape/data offset
		2: 7,  // temp
		3: 28, // temp2
		4: 10, // a0, syscall arg/result
		5: 29, // temp3
		6: 30, // zero/bounds-check temp
	}
	if physical, ok := fixed[index]; ok {
		return physical, true
	}
	extra := []int{11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27}
	extraIndex := index - 7
	if extraIndex >= 0 && extraIndex < len(extra) {
		return extra[extraIndex], true
	}
	return 0, false
}

func regOperand(instruction ir.IrInstruction, index int) (ir.IrRegister, error) {
	if index >= len(instruction.Operands) {
		return ir.IrRegister{}, loweringError(instruction, "missing register operand %d", index)
	}
	reg, ok := instruction.Operands[index].(ir.IrRegister)
	if !ok {
		return ir.IrRegister{}, loweringError(instruction, "operand %d is %T, expected IrRegister", index, instruction.Operands[index])
	}
	return reg, nil
}

func immOperand(instruction ir.IrInstruction, index int) (ir.IrImmediate, error) {
	if index >= len(instruction.Operands) {
		return ir.IrImmediate{}, loweringError(instruction, "missing immediate operand %d", index)
	}
	imm, ok := instruction.Operands[index].(ir.IrImmediate)
	if !ok {
		return ir.IrImmediate{}, loweringError(instruction, "operand %d is %T, expected IrImmediate", index, instruction.Operands[index])
	}
	return imm, nil
}

func labelOperand(instruction ir.IrInstruction, index int) (ir.IrLabel, error) {
	if index >= len(instruction.Operands) {
		return ir.IrLabel{}, loweringError(instruction, "missing label operand %d", index)
	}
	label, ok := instruction.Operands[index].(ir.IrLabel)
	if !ok {
		return ir.IrLabel{}, loweringError(instruction, "operand %d is %T, expected IrLabel", index, instruction.Operands[index])
	}
	return label, nil
}

func thirdOperandAsImmediate(instruction ir.IrInstruction) (ir.IrImmediate, bool) {
	if len(instruction.Operands) < 3 {
		return ir.IrImmediate{}, false
	}
	imm, ok := instruction.Operands[2].(ir.IrImmediate)
	return imm, ok
}

func dataBytes(data []ir.IrDataDecl) []byte {
	var out []byte
	for _, decl := range data {
		init := byte(decl.Init & 0xFF)
		for i := 0; i < decl.Size; i++ {
			out = append(out, init)
		}
	}
	return out
}

func fitsSigned(value, bits int) bool {
	min := -(1 << (bits - 1))
	max := (1 << (bits - 1)) - 1
	return value >= min && value <= max
}

func validateBranchOffset(offset int) error {
	if offset%2 != 0 {
		return fmt.Errorf("branch offset %d is not 2-byte aligned", offset)
	}
	if !fitsSigned(offset, 13) {
		return fmt.Errorf("branch offset %d is outside signed 13-bit range", offset)
	}
	return nil
}

func validateJalOffset(offset int) error {
	if offset%2 != 0 {
		return fmt.Errorf("jal offset %d is not 2-byte aligned", offset)
	}
	if !fitsSigned(offset, 21) {
		return fmt.Errorf("jal offset %d is outside signed 21-bit range", offset)
	}
	return nil
}

func loweringError(instruction ir.IrInstruction, format string, args ...any) error {
	return LoweringError{
		IrID:    instruction.ID,
		Opcode:  instruction.Opcode,
		Message: fmt.Sprintf(format, args...),
	}
}
