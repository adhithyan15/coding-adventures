package irtoriscvcompiler

import (
	"fmt"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

func (c *IrToRiscVCompiler) emitAssembly(program *ir.IrProgram, plan *compilePlan) (string, error) {
	var builder strings.Builder
	builder.WriteString(".text\n")
	if plan.usesStack() {
		for _, line := range emitStackSetupAssembly(plan.stackTop) {
			builder.WriteString(line)
			builder.WriteByte('\n')
		}
	}
	if plan.entryTrampoline {
		builder.WriteString(fmt.Sprintf("  j %s\n", program.EntryLabel))
	}

	for index, instruction := range program.Instructions {
		lines, err := c.emitAssemblyInstruction(instruction, index, plan)
		if err != nil {
			return "", err
		}
		for _, line := range lines {
			builder.WriteString(line)
			builder.WriteByte('\n')
		}
	}

	if len(program.Data) > 0 || plan.usesStack() {
		builder.WriteString("\n.data\n")
		for _, decl := range program.Data {
			builder.WriteString(fmt.Sprintf("%s:\n", decl.Label))
			if decl.Size == 0 {
				continue
			}
			if decl.Init == 0 {
				builder.WriteString(fmt.Sprintf("  .zero %d\n", decl.Size))
				continue
			}
			values := make([]string, decl.Size)
			for index := range values {
				values[index] = fmt.Sprintf("%d", decl.Init&0xFF)
			}
			builder.WriteString(fmt.Sprintf("  .byte %s\n", strings.Join(values, ", ")))
		}
		if plan.usesStack() {
			builder.WriteString(fmt.Sprintf("%s:\n", callFrameStackLabel))
			builder.WriteString(fmt.Sprintf("  .zero %d\n", callFrameStackSize))
		}
	}

	return builder.String(), nil
}

func (c *IrToRiscVCompiler) emitAssemblyInstruction(instruction ir.IrInstruction, instructionIndex int, plan *compilePlan) ([]string, error) {
	frame := plan.frameForInstruction(instructionIndex)
	switch instruction.Opcode {
	case ir.OpLabel:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		lines := []string{fmt.Sprintf("%s:", label.Name)}
		if frame != nil && frame.isEntryLabel(label.Name) {
			lines = append(lines, emitFunctionPrologueAssembly(frame)...)
		}
		return lines, nil
	case ir.OpComment:
		text := ""
		if len(instruction.Operands) > 0 {
			text = instruction.Operands[0].String()
		}
		return []string{fmt.Sprintf("  # %s", text)}, nil
	case ir.OpLoadImm:
		dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
		if err != nil {
			return nil, err
		}
		imm, err := immOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		lines := []string{fmt.Sprintf("  li %s, %d", regName(dst), imm.Value)}
		return appendSpillStoreAssembly(lines, dst, spillOffset), nil
	case ir.OpLoadAddr:
		dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
		if err != nil {
			return nil, err
		}
		label, err := labelOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		lines := []string{fmt.Sprintf("  la %s, %s", regName(dst), label.Name)}
		return appendSpillStoreAssembly(lines, dst, spillOffset), nil
	case ir.OpLoadByte:
		return c.emitLoadAssembly(instruction, frame, "lbu")
	case ir.OpLoadWord:
		return c.emitLoadAssembly(instruction, frame, "lw")
	case ir.OpStoreByte:
		return c.emitStoreAssembly(instruction, frame, "sb")
	case ir.OpStoreWord:
		return c.emitStoreAssembly(instruction, frame, "sw")
	case ir.OpAdd:
		return emitThreeRegAssembly(instruction, frame, "add")
	case ir.OpSub:
		return emitThreeRegAssembly(instruction, frame, "sub")
	case ir.OpAnd:
		return emitThreeRegAssembly(instruction, frame, "and")
	case ir.OpAddImm:
		return emitRegImmAssembly(instruction, frame, "addi", "add")
	case ir.OpAndImm:
		return emitRegImmAssembly(instruction, frame, "andi", "and")
	case ir.OpCmpEq:
		return emitCmpEqNeAssembly(instruction, frame, true)
	case ir.OpCmpNe:
		return emitCmpEqNeAssembly(instruction, frame, false)
	case ir.OpCmpLt:
		return emitThreeRegAssembly(instruction, frame, "slt")
	case ir.OpCmpGt:
		dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
		if err != nil {
			return nil, err
		}
		left, leftSetup, err := sourceRegisterAssembly(instruction, frame, 1, xSpillTemp1)
		if err != nil {
			return nil, err
		}
		right, rightSetup, err := sourceRegisterAssembly(instruction, frame, 2, xSpillTemp2)
		if err != nil {
			return nil, err
		}
		lines := append([]string{}, leftSetup...)
		lines = append(lines, rightSetup...)
		lines = append(lines, fmt.Sprintf("  slt %s, %s, %s", regName(dst), regName(right), regName(left)))
		return appendSpillStoreAssembly(lines, dst, spillOffset), nil
	case ir.OpJump:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		return []string{fmt.Sprintf("  j %s", label.Name)}, nil
	case ir.OpBranchZ, ir.OpBranchNz:
		register, setup, err := sourceRegisterAssembly(instruction, frame, 0, xSpillTemp0)
		if err != nil {
			return nil, err
		}
		label, err := labelOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		opcode := "beq"
		if instruction.Opcode == ir.OpBranchNz {
			opcode = "bne"
		}
		lines := append([]string{}, setup...)
		lines = append(lines, fmt.Sprintf("  %s %s, zero, %s", opcode, regName(register), label.Name))
		return lines, nil
	case ir.OpCall:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		return emitCallWithCallerSavesAssembly(label.Name, plan.callerSavedRegs), nil
	case ir.OpRet:
		return emitRetEpilogueAssembly(frame), nil
	case ir.OpSyscall:
		imm, err := immOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		return []string{
			fmt.Sprintf("  li %s, %d", regName(xSyscallNumber), imm.Value),
			"  ecall",
		}, nil
	case ir.OpHalt:
		return []string{"  halt"}, nil
	case ir.OpNop:
		return []string{"  nop"}, nil
	default:
		return nil, loweringError(instruction, "unsupported opcode")
	}
}

func (c *IrToRiscVCompiler) emitLoadAssembly(instruction ir.IrInstruction, frame *functionFrame, opcode string) ([]string, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	base, baseSetup, err := sourceRegisterAssembly(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	lines := append([]string{}, baseSetup...)
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		lines = append(lines, fmt.Sprintf("  %s %s, %d(%s)", opcode, regName(dst), imm.Value, regName(base)))
		return appendSpillStoreAssembly(lines, dst, spillOffset), nil
	}
	offset, offsetSetup, err := sourceRegisterAssembly(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	lines = append(lines, offsetSetup...)
	lines = append(lines,
		fmt.Sprintf("  add %s, %s, %s", regName(xBackendScratch), regName(base), regName(offset)),
		fmt.Sprintf("  %s %s, 0(%s)", opcode, regName(dst), regName(xBackendScratch)),
	)
	return appendSpillStoreAssembly(lines, dst, spillOffset), nil
}

func (c *IrToRiscVCompiler) emitStoreAssembly(instruction ir.IrInstruction, frame *functionFrame, opcode string) ([]string, error) {
	src, srcSetup, err := sourceRegisterAssembly(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	base, baseSetup, err := sourceRegisterAssembly(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	lines := append([]string{}, srcSetup...)
	lines = append(lines, baseSetup...)
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		lines = append(lines, fmt.Sprintf("  %s %s, %d(%s)", opcode, regName(src), imm.Value, regName(base)))
		return lines, nil
	}
	offset, offsetSetup, err := sourceRegisterAssembly(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	lines = append(lines, offsetSetup...)
	lines = append(lines,
		fmt.Sprintf("  add %s, %s, %s", regName(xBackendScratch), regName(base), regName(offset)),
		fmt.Sprintf("  %s %s, 0(%s)", opcode, regName(src), regName(xBackendScratch)),
	)
	return lines, nil
}

func emitThreeRegAssembly(instruction ir.IrInstruction, frame *functionFrame, opcode string) ([]string, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	left, leftSetup, err := sourceRegisterAssembly(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	right, rightSetup, err := sourceRegisterAssembly(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	lines := append([]string{}, leftSetup...)
	lines = append(lines, rightSetup...)
	lines = append(lines, fmt.Sprintf("  %s %s, %s, %s", opcode, regName(dst), regName(left), regName(right)))
	return appendSpillStoreAssembly(lines, dst, spillOffset), nil
}

func emitRegImmAssembly(instruction ir.IrInstruction, frame *functionFrame, immOpcode string, regOpcode string) ([]string, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	src, srcSetup, err := sourceRegisterAssembly(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	imm, err := immOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	lines := append([]string{}, srcSetup...)
	if fitsSigned(imm.Value, 12) {
		lines = append(lines, fmt.Sprintf("  %s %s, %s, %d", immOpcode, regName(dst), regName(src), imm.Value))
		return appendSpillStoreAssembly(lines, dst, spillOffset), nil
	}
	lines = append(lines,
		fmt.Sprintf("  li %s, %d", regName(xBackendScratch), imm.Value),
		fmt.Sprintf("  %s %s, %s, %s", regOpcode, regName(dst), regName(src), regName(xBackendScratch)),
	)
	return appendSpillStoreAssembly(lines, dst, spillOffset), nil
}

func emitCmpEqNeAssembly(instruction ir.IrInstruction, frame *functionFrame, equal bool) ([]string, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	left, leftSetup, err := sourceRegisterAssembly(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	right, rightSetup, err := sourceRegisterAssembly(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	lines := append([]string{}, leftSetup...)
	lines = append(lines, rightSetup...)
	if equal {
		lines = append(lines,
			fmt.Sprintf("  xor %s, %s, %s", regName(dst), regName(left), regName(right)),
			fmt.Sprintf("  sltiu %s, %s, 1", regName(dst), regName(dst)),
		)
		return appendSpillStoreAssembly(lines, dst, spillOffset), nil
	}
	lines = append(lines,
		fmt.Sprintf("  xor %s, %s, %s", regName(dst), regName(left), regName(right)),
		fmt.Sprintf("  sltu %s, zero, %s", regName(dst), regName(dst)),
	)
	return appendSpillStoreAssembly(lines, dst, spillOffset), nil
}

func emitFunctionPrologueAssembly(frame *functionFrame) []string {
	if frame == nil || frame.frameSize == 0 {
		return nil
	}
	lines := emitStackAdjustAssembly(-frame.frameSize)
	if frame.savesReturnAddress {
		lines = append(lines, emitStackSlotStoreAssembly(xRa, 0)...)
	}
	return lines
}

func emitCallWithCallerSavesAssembly(label string, regs []callerSavedRegister) []string {
	lines := emitCallerSavePrologueAssembly(regs)
	lines = append(lines, fmt.Sprintf("  call %s", label))
	lines = append(lines, emitCallerSaveEpilogueAssembly(regs)...)
	return lines
}

func emitCallerSavePrologueAssembly(regs []callerSavedRegister) []string {
	if len(regs) == 0 {
		return nil
	}
	lines := emitStackAdjustAssembly(-callerSaveFrameBytes(regs))
	for index, register := range regs {
		lines = append(lines, fmt.Sprintf("  sw %s, %d(sp)", regName(register.physical), index*4))
	}
	return lines
}

func emitCallerSaveEpilogueAssembly(regs []callerSavedRegister) []string {
	if len(regs) == 0 {
		return nil
	}
	lines := make([]string, 0, len(regs)+len(emitStackAdjustAssembly(callerSaveFrameBytes(regs))))
	for index, register := range regs {
		lines = append(lines, fmt.Sprintf("  lw %s, %d(sp)", regName(register.physical), index*4))
	}
	lines = append(lines, emitStackAdjustAssembly(callerSaveFrameBytes(regs))...)
	return lines
}

func emitStackSetupAssembly(stackTop int) []string {
	upper, lower := splitUpperLower(stackTop)
	return []string{
		fmt.Sprintf("  lui sp, %d", upper),
		fmt.Sprintf("  addi sp, sp, %d", lower),
	}
}

func emitRetEpilogueAssembly(frame *functionFrame) []string {
	if frame == nil || frame.frameSize == 0 {
		return []string{"  ret"}
	}
	lines := []string{}
	if frame.savesReturnAddress {
		lines = append(lines, emitStackSlotLoadAssembly(xRa, 0)...)
	}
	lines = append(lines, emitStackAdjustAssembly(frame.frameSize)...)
	lines = append(lines, "  ret")
	return lines
}

func sourceRegisterAssembly(instruction ir.IrInstruction, frame *functionFrame, operandIndex int, spillTemp int) (int, []string, error) {
	reg, err := regOperand(instruction, operandIndex)
	if err != nil {
		return 0, nil, err
	}
	if physical, ok := physicalRegister(reg.Index); ok {
		return physical, nil, nil
	}
	offset, ok := spillSlotOffset(frame, reg.Index)
	if !ok {
		return 0, nil, loweringError(instruction, "virtual register v%d has no physical mapping or spill slot", reg.Index)
	}
	return spillTemp, emitStackSlotLoadAssembly(spillTemp, offset), nil
}

func appendSpillStoreAssembly(lines []string, physical int, spillOffset int) []string {
	if spillOffset < 0 {
		return lines
	}
	return append(lines, emitStackSlotStoreAssembly(physical, spillOffset)...)
}

func emitStackAdjustAssembly(amount int) []string {
	if amount == 0 {
		return nil
	}
	if fitsSigned(amount, 12) {
		return []string{fmt.Sprintf("  addi sp, sp, %d", amount)}
	}
	return []string{
		fmt.Sprintf("  li %s, %d", regName(xBackendScratch), amount),
		fmt.Sprintf("  add sp, sp, %s", regName(xBackendScratch)),
	}
}

func emitStackSlotLoadAssembly(rd, offset int) []string {
	if fitsSigned(offset, 12) {
		return []string{fmt.Sprintf("  lw %s, %d(sp)", regName(rd), offset)}
	}
	return []string{
		fmt.Sprintf("  li %s, %d", regName(xBackendScratch), offset),
		fmt.Sprintf("  add %s, sp, %s", regName(xBackendScratch), regName(xBackendScratch)),
		fmt.Sprintf("  lw %s, 0(%s)", regName(rd), regName(xBackendScratch)),
	}
}

func emitStackSlotStoreAssembly(rs, offset int) []string {
	if fitsSigned(offset, 12) {
		return []string{fmt.Sprintf("  sw %s, %d(sp)", regName(rs), offset)}
	}
	return []string{
		fmt.Sprintf("  li %s, %d", regName(xBackendScratch), offset),
		fmt.Sprintf("  add %s, sp, %s", regName(xBackendScratch), regName(xBackendScratch)),
		fmt.Sprintf("  sw %s, 0(%s)", regName(rs), regName(xBackendScratch)),
	}
}

func regName(index int) string {
	switch index {
	case 0:
		return "zero"
	case 1:
		return "ra"
	case 2:
		return "sp"
	}
	return fmt.Sprintf("x%d", index)
}
