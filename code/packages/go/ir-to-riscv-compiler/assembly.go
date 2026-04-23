package irtoriscvcompiler

import (
	"fmt"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

func (c *IrToRiscVCompiler) emitAssembly(program *ir.IrProgram, plan *compilePlan) (string, error) {
	var builder strings.Builder
	builder.WriteString(".text\n")
	if plan.usesCallFrames() {
		for _, line := range emitStackSetupAssembly(plan.stackTop) {
			builder.WriteString(line)
			builder.WriteByte('\n')
		}
	}
	if plan.entryTrampoline {
		builder.WriteString(fmt.Sprintf("  j %s\n", program.EntryLabel))
	}

	for _, instruction := range program.Instructions {
		lines, err := c.emitAssemblyInstruction(instruction, plan)
		if err != nil {
			return "", err
		}
		for _, line := range lines {
			builder.WriteString(line)
			builder.WriteByte('\n')
		}
	}

	if len(program.Data) > 0 || plan.usesCallFrames() {
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
		if plan.usesCallFrames() {
			builder.WriteString(fmt.Sprintf("%s:\n", callFrameStackLabel))
			builder.WriteString(fmt.Sprintf("  .zero %d\n", callFrameStackSize))
		}
	}

	return builder.String(), nil
}

func (c *IrToRiscVCompiler) emitAssemblyInstruction(instruction ir.IrInstruction, plan *compilePlan) ([]string, error) {
	switch instruction.Opcode {
	case ir.OpLabel:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		lines := []string{fmt.Sprintf("%s:", label.Name)}
		if plan.callTargets[label.Name] {
			lines = append(lines, emitCallTargetPrologueAssembly()...)
		}
		return lines, nil
	case ir.OpComment:
		text := ""
		if len(instruction.Operands) > 0 {
			text = instruction.Operands[0].String()
		}
		return []string{fmt.Sprintf("  # %s", text)}, nil
	case ir.OpLoadImm:
		dst, err := physicalRegOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		imm, err := immOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		return []string{fmt.Sprintf("  li %s, %d", regName(dst), imm.Value)}, nil
	case ir.OpLoadAddr:
		dst, err := physicalRegOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		label, err := labelOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		return []string{fmt.Sprintf("  la %s, %s", regName(dst), label.Name)}, nil
	case ir.OpLoadByte:
		return c.emitLoadAssembly(instruction, "lbu")
	case ir.OpLoadWord:
		return c.emitLoadAssembly(instruction, "lw")
	case ir.OpStoreByte:
		return c.emitStoreAssembly(instruction, "sb")
	case ir.OpStoreWord:
		return c.emitStoreAssembly(instruction, "sw")
	case ir.OpAdd:
		return emitThreeRegAssembly(instruction, "add")
	case ir.OpSub:
		return emitThreeRegAssembly(instruction, "sub")
	case ir.OpAnd:
		return emitThreeRegAssembly(instruction, "and")
	case ir.OpAddImm:
		return emitRegImmAssembly(instruction, "addi", "add")
	case ir.OpAndImm:
		return emitRegImmAssembly(instruction, "andi", "and")
	case ir.OpCmpEq:
		return emitCmpEqNeAssembly(instruction, true)
	case ir.OpCmpNe:
		return emitCmpEqNeAssembly(instruction, false)
	case ir.OpCmpLt:
		return emitThreeRegAssembly(instruction, "slt")
	case ir.OpCmpGt:
		dst, left, right, err := threePhysicalRegs(instruction)
		if err != nil {
			return nil, err
		}
		return []string{fmt.Sprintf("  slt %s, %s, %s", regName(dst), regName(right), regName(left))}, nil
	case ir.OpJump:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		return []string{fmt.Sprintf("  j %s", label.Name)}, nil
	case ir.OpBranchZ, ir.OpBranchNz:
		register, err := physicalRegOperand(instruction, 0)
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
		return []string{fmt.Sprintf("  %s %s, zero, %s", opcode, regName(register), label.Name)}, nil
	case ir.OpCall:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		return emitCallWithCallerSavesAssembly(label.Name, plan.callerSavedRegs), nil
	case ir.OpRet:
		if !plan.usesCallFrames() {
			return []string{"  ret"}, nil
		}
		return emitRetEpilogueAssembly(), nil
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

func (c *IrToRiscVCompiler) emitLoadAssembly(instruction ir.IrInstruction, opcode string) ([]string, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	base, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		return []string{fmt.Sprintf("  %s %s, %d(%s)", opcode, regName(dst), imm.Value, regName(base))}, nil
	}
	offset, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	return []string{
		fmt.Sprintf("  add %s, %s, %s", regName(xBackendScratch), regName(base), regName(offset)),
		fmt.Sprintf("  %s %s, 0(%s)", opcode, regName(dst), regName(xBackendScratch)),
	}, nil
}

func (c *IrToRiscVCompiler) emitStoreAssembly(instruction ir.IrInstruction, opcode string) ([]string, error) {
	src, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return nil, err
	}
	base, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return nil, err
	}
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		return []string{fmt.Sprintf("  %s %s, %d(%s)", opcode, regName(src), imm.Value, regName(base))}, nil
	}
	offset, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	return []string{
		fmt.Sprintf("  add %s, %s, %s", regName(xBackendScratch), regName(base), regName(offset)),
		fmt.Sprintf("  %s %s, 0(%s)", opcode, regName(src), regName(xBackendScratch)),
	}, nil
}

func emitThreeRegAssembly(instruction ir.IrInstruction, opcode string) ([]string, error) {
	dst, left, right, err := threePhysicalRegs(instruction)
	if err != nil {
		return nil, err
	}
	return []string{fmt.Sprintf("  %s %s, %s, %s", opcode, regName(dst), regName(left), regName(right))}, nil
}

func emitRegImmAssembly(instruction ir.IrInstruction, immOpcode string, regOpcode string) ([]string, error) {
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
		return []string{fmt.Sprintf("  %s %s, %s, %d", immOpcode, regName(dst), regName(src), imm.Value)}, nil
	}
	return []string{
		fmt.Sprintf("  li %s, %d", regName(xBackendScratch), imm.Value),
		fmt.Sprintf("  %s %s, %s, %s", regOpcode, regName(dst), regName(src), regName(xBackendScratch)),
	}, nil
}

func emitCmpEqNeAssembly(instruction ir.IrInstruction, equal bool) ([]string, error) {
	dst, left, right, err := threePhysicalRegs(instruction)
	if err != nil {
		return nil, err
	}
	if equal {
		return []string{
			fmt.Sprintf("  xor %s, %s, %s", regName(dst), regName(left), regName(right)),
			fmt.Sprintf("  sltiu %s, %s, 1", regName(dst), regName(dst)),
		}, nil
	}
	return []string{
		fmt.Sprintf("  xor %s, %s, %s", regName(dst), regName(left), regName(right)),
		fmt.Sprintf("  sltu %s, zero, %s", regName(dst), regName(dst)),
	}, nil
}

func emitCallTargetPrologueAssembly() []string {
	return []string{
		"  addi sp, sp, -4",
		"  sw ra, 0(sp)",
	}
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
	lines := []string{fmt.Sprintf("  addi sp, sp, -%d", callerSaveFrameBytes(regs))}
	for index, register := range regs {
		lines = append(lines, fmt.Sprintf("  sw %s, %d(sp)", regName(register.physical), index*4))
	}
	return lines
}

func emitCallerSaveEpilogueAssembly(regs []callerSavedRegister) []string {
	if len(regs) == 0 {
		return nil
	}
	lines := make([]string, 0, len(regs)+1)
	for index, register := range regs {
		lines = append(lines, fmt.Sprintf("  lw %s, %d(sp)", regName(register.physical), index*4))
	}
	lines = append(lines, fmt.Sprintf("  addi sp, sp, %d", callerSaveFrameBytes(regs)))
	return lines
}

func emitStackSetupAssembly(stackTop int) []string {
	upper, lower := splitUpperLower(stackTop)
	return []string{
		fmt.Sprintf("  lui sp, %d", upper),
		fmt.Sprintf("  addi sp, sp, %d", lower),
	}
}

func emitRetEpilogueAssembly() []string {
	return []string{
		"  lw ra, 0(sp)",
		"  addi sp, sp, 4",
		"  ret",
	}
}

func threePhysicalRegs(instruction ir.IrInstruction) (int, int, int, error) {
	dst, err := physicalRegOperand(instruction, 0)
	if err != nil {
		return 0, 0, 0, err
	}
	left, err := physicalRegOperand(instruction, 1)
	if err != nil {
		return 0, 0, 0, err
	}
	right, err := physicalRegOperand(instruction, 2)
	if err != nil {
		return 0, 0, 0, err
	}
	return dst, left, right, nil
}

func regName(index int) string {
	return fmt.Sprintf("x%d", index)
}
