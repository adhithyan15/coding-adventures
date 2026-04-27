package irtoriscvcompiler

import (
	"fmt"
	"sort"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	sm "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const (
	x0              = 0
	xRa             = 1
	xSp             = 2
	xSyscallNumber  = 17
	xSpillTemp0     = 28
	xSpillTemp1     = 29
	xSpillTemp2     = 30
	xBackendScratch = 31

	callFrameStackLabel = "__riscv_call_stack"
	callFrameStackSize  = 1024
)

var starterPhysicalRegisters = []int{
	5, 6, 7,
	11, 10, 12, 13, 14, 15, 16,
	18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
}

// MachineCodeResult is the output of lowering IR to a flat RISC-V image.
type MachineCodeResult struct {
	Bytes           []byte
	Assembly        string
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
	callTargets     map[string]bool
	functionFrames  map[string]*functionFrame
	functionRoots   map[string]bool
	functionByIndex []string
	callerSavedRegs []callerSavedRegister
	stackTop        int
	textSize        int
	entryTrampoline bool
}

type functionFrame struct {
	label              string
	savesReturnAddress bool
	frameSize          int
	spillSlots         map[int]int
}

func (p *compilePlan) usesStack() bool {
	return p.stackTop != 0
}

func (p *compilePlan) frameForInstruction(index int) *functionFrame {
	if index < 0 || index >= len(p.functionByIndex) {
		return nil
	}
	root := p.functionByIndex[index]
	if root == "" {
		return nil
	}
	return p.functionFrames[root]
}

func (f *functionFrame) isEntryLabel(name string) bool {
	return f != nil && f.label == name
}

func (f *functionFrame) prologueSize() int {
	if f == nil || f.frameSize == 0 {
		return 0
	}
	size := stackAdjustInstructionSize(-f.frameSize)
	if f.savesReturnAddress {
		size += stackSlotAccessSize(0)
	}
	return size
}

func (f *functionFrame) retEpilogueSize() int {
	if f == nil || f.frameSize == 0 {
		return 1
	}
	size := stackAdjustInstructionSize(f.frameSize) + 1
	if f.savesReturnAddress {
		size += stackSlotAccessSize(0)
	}
	return size
}

type callerSavedRegister struct {
	virtual  int
	physical int
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
	if plan.usesStack() {
		words = append(words, emitLoadAddress(xSp, plan.stackTop)...)
	}
	if plan.entryTrampoline {
		entryOffset := plan.labelOffsets[program.EntryLabel]
		if err := validateJalOffset(entryOffset); err != nil {
			return nil, err
		}
		words = append(words, riscv.EncodeJal(x0, entryOffset))
	}

	irToMC := &sm.IrToMachineCode{}
	for index, instruction := range program.Instructions {
		pc := len(words) * 4
		emitted, err := c.emitInstruction(instruction, index, pc, plan)
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
	if plan.usesStack() {
		image = append(image, make([]byte, callFrameStackSize)...)
	}
	assembly, err := c.emitAssembly(program, plan)
	if err != nil {
		return nil, err
	}

	return &MachineCodeResult{
		Bytes:           image,
		Assembly:        assembly,
		Instructions:    words,
		LabelOffsets:    plan.labelOffsets,
		DataOffsets:     plan.dataOffsets,
		IrToMachineCode: irToMC,
	}, nil
}

func (c *IrToRiscVCompiler) plan(program *ir.IrProgram) (*compilePlan, error) {
	callTargets, err := collectCallTargets(program)
	if err != nil {
		return nil, err
	}
	functionRoots, err := collectFunctionRoots(program, callTargets)
	if err != nil {
		return nil, err
	}
	functionFrames, functionByIndex, err := collectFunctionFrames(program, functionRoots, callTargets)
	if err != nil {
		return nil, err
	}
	callerSavedRegs := []callerSavedRegister{}
	if len(callTargets) > 0 {
		callerSavedRegs = collectCallerSavedRegisters(program)
	}

	labelOffsets := map[string]int{}
	pc := stackSetupSize(functionFrames) * 4
	fallthroughOffset := pc
	for index, instruction := range program.Instructions {
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

		size, err := c.instructionSize(instruction, index, functionFrames[functionByIndex[index]], callerSavedRegs)
		if err != nil {
			return nil, err
		}
		pc += size * 4
	}

	entryTrampoline := false
	if program.EntryLabel != "" {
		if entryOffset, ok := labelOffsets[program.EntryLabel]; ok && entryOffset != fallthroughOffset {
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
	stackTop := 0
	if usesStack(functionFrames) {
		if _, exists := dataOffsets[callFrameStackLabel]; exists {
			return nil, fmt.Errorf("duplicate data label %q", callFrameStackLabel)
		}
		dataOffsets[callFrameStackLabel] = dataPC
		stackTop = dataPC + callFrameStackSize
		dataPC += callFrameStackSize
	}

	return &compilePlan{
		labelOffsets:    labelOffsets,
		dataOffsets:     dataOffsets,
		callTargets:     callTargets,
		functionFrames:  functionFrames,
		functionRoots:   functionRoots,
		functionByIndex: functionByIndex,
		callerSavedRegs: callerSavedRegs,
		stackTop:        stackTop,
		textSize:        pc,
		entryTrampoline: entryTrampoline,
	}, nil
}

func (c *IrToRiscVCompiler) instructionSize(instruction ir.IrInstruction, instructionIndex int, frame *functionFrame, callerSavedRegs []callerSavedRegister) (int, error) {
	switch instruction.Opcode {
	case ir.OpLabel:
		if frame == nil {
			return 0, nil
		}
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return 0, err
		}
		if frame.isEntryLabel(label.Name) {
			return frame.prologueSize(), nil
		}
		return 0, nil
	case ir.OpComment:
		return 0, nil
	case ir.OpLoadImm:
		imm, err := immOperand(instruction, 1)
		if err != nil {
			return 0, err
		}
		size, err := destinationRegisterSize(instruction, frame, 0)
		if err != nil {
			return 0, err
		}
		return loadConstSize(imm.Value) + size, nil
	case ir.OpLoadAddr:
		size, err := destinationRegisterSize(instruction, frame, 0)
		if err != nil {
			return 0, err
		}
		return 2 + size, nil
	case ir.OpLoadByte, ir.OpLoadWord, ir.OpStoreByte, ir.OpStoreWord:
		size, err := sourceRegisterSize(instruction, frame, 1)
		if err != nil {
			return 0, err
		}
		if instruction.Opcode == ir.OpStoreByte || instruction.Opcode == ir.OpStoreWord {
			sourceSize, err := sourceRegisterSize(instruction, frame, 0)
			if err != nil {
				return 0, err
			}
			size += sourceSize
		} else {
			destSize, err := destinationRegisterSize(instruction, frame, 0)
			if err != nil {
				return 0, err
			}
			size += destSize
		}
		if _, ok := thirdOperandAsImmediate(instruction); ok {
			return size + 1, nil
		}
		offsetSize, err := sourceRegisterSize(instruction, frame, 2)
		if err != nil {
			return 0, err
		}
		return size + offsetSize + 2, nil
	case ir.OpAdd, ir.OpSub, ir.OpAnd, ir.OpCmpLt, ir.OpCmpGt:
		return c.threeRegInstructionSize(instruction, frame, 1)
	case ir.OpCmpEq, ir.OpCmpNe:
		return c.threeRegInstructionSize(instruction, frame, 2)
	case ir.OpAddImm, ir.OpAndImm:
		size, err := sourceRegisterSize(instruction, frame, 1)
		if err != nil {
			return 0, err
		}
		destSize, err := destinationRegisterSize(instruction, frame, 0)
		if err != nil {
			return 0, err
		}
		imm, err := immOperand(instruction, 2)
		if err != nil {
			return 0, err
		}
		if fitsSigned(imm.Value, 12) {
			return size + 1 + destSize, nil
		}
		return size + loadConstSize(imm.Value) + 1 + destSize, nil
	case ir.OpJump, ir.OpBranchZ, ir.OpBranchNz, ir.OpNop:
		if instruction.Opcode == ir.OpJump || instruction.Opcode == ir.OpNop {
			return 1, nil
		}
		size, err := sourceRegisterSize(instruction, frame, 0)
		if err != nil {
			return 0, err
		}
		return size + 1, nil
	case ir.OpCall:
		return callInstructionSize(callerSavedRegs), nil
	case ir.OpRet:
		if frame == nil {
			return 1, nil
		}
		return frame.retEpilogueSize(), nil
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

func (c *IrToRiscVCompiler) threeRegInstructionSize(instruction ir.IrInstruction, frame *functionFrame, opWords int) (int, error) {
	leftSize, err := sourceRegisterSize(instruction, frame, 1)
	if err != nil {
		return 0, err
	}
	rightSize, err := sourceRegisterSize(instruction, frame, 2)
	if err != nil {
		return 0, err
	}
	destSize, err := destinationRegisterSize(instruction, frame, 0)
	if err != nil {
		return 0, err
	}
	return leftSize + rightSize + opWords + destSize, nil
}

func (c *IrToRiscVCompiler) emitInstruction(instruction ir.IrInstruction, instructionIndex int, pc int, plan *compilePlan) ([]uint32, error) {
	frame := plan.frameForInstruction(instructionIndex)
	switch instruction.Opcode {
	case ir.OpLabel:
		if frame == nil {
			return nil, nil
		}
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		if frame.isEntryLabel(label.Name) {
			return emitFunctionPrologue(frame), nil
		}
		return nil, nil
	case ir.OpComment:
		return nil, nil
	case ir.OpLoadImm:
		dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
		if err != nil {
			return nil, err
		}
		imm, err := immOperand(instruction, 1)
		if err != nil {
			return nil, err
		}
		words := emitLoadConst(dst, imm.Value)
		return appendSpillStore(words, dst, spillOffset), nil
	case ir.OpLoadAddr:
		dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
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
		words := emitLoadAddress(dst, offset)
		return appendSpillStore(words, dst, spillOffset), nil
	case ir.OpLoadByte:
		return c.emitLoad(instruction, frame, riscv.EncodeLbu)
	case ir.OpLoadWord:
		return c.emitLoad(instruction, frame, riscv.EncodeLw)
	case ir.OpStoreByte:
		return c.emitStore(instruction, frame, riscv.EncodeSb)
	case ir.OpStoreWord:
		return c.emitStore(instruction, frame, riscv.EncodeSw)
	case ir.OpAdd:
		return emitThreeReg(instruction, frame, riscv.EncodeAdd)
	case ir.OpSub:
		return emitThreeReg(instruction, frame, riscv.EncodeSub)
	case ir.OpAnd:
		return emitThreeReg(instruction, frame, riscv.EncodeAnd)
	case ir.OpAddImm:
		return emitRegImm(instruction, frame, riscv.EncodeAddi, riscv.EncodeAdd)
	case ir.OpAndImm:
		return emitRegImm(instruction, frame, riscv.EncodeAndi, riscv.EncodeAnd)
	case ir.OpCmpEq:
		return emitCmpEqNe(instruction, frame, true)
	case ir.OpCmpNe:
		return emitCmpEqNe(instruction, frame, false)
	case ir.OpCmpLt:
		return emitThreeReg(instruction, frame, riscv.EncodeSlt)
	case ir.OpCmpGt:
		return emitCmpGt(instruction, frame)
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
		return emitBranch(instruction, frame, pc, plan)
	case ir.OpCall:
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		jalPC := pc + callerSavePrologueSize(plan.callerSavedRegs)*4
		offset, err := jalOffset(instruction, label, jalPC, plan)
		if err != nil {
			return nil, err
		}
		return emitCallWithCallerSaves(offset, plan.callerSavedRegs), nil
	case ir.OpRet:
		return emitRetEpilogue(frame), nil
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

func (c *IrToRiscVCompiler) emitLoad(instruction ir.IrInstruction, frame *functionFrame, encode func(rd, rs1, imm int) uint32) ([]uint32, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	base, baseSetup, err := sourceRegister(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	words := append([]uint32{}, baseSetup...)
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		if !fitsSigned(imm.Value, 12) {
			return nil, loweringError(instruction, "memory immediate %d is outside signed 12-bit range", imm.Value)
		}
		words = append(words, encode(dst, base, imm.Value))
		return appendSpillStore(words, dst, spillOffset), nil
	}
	offset, offsetSetup, err := sourceRegister(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	words = append(words, offsetSetup...)
	words = append(words,
		riscv.EncodeAdd(xBackendScratch, base, offset),
		encode(dst, xBackendScratch, 0),
	)
	return appendSpillStore(words, dst, spillOffset), nil
}

func (c *IrToRiscVCompiler) emitStore(instruction ir.IrInstruction, frame *functionFrame, encode func(rs2, rs1, imm int) uint32) ([]uint32, error) {
	src, srcSetup, err := sourceRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	base, baseSetup, err := sourceRegister(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	words := append([]uint32{}, srcSetup...)
	words = append(words, baseSetup...)
	if imm, ok := thirdOperandAsImmediate(instruction); ok {
		if !fitsSigned(imm.Value, 12) {
			return nil, loweringError(instruction, "memory immediate %d is outside signed 12-bit range", imm.Value)
		}
		return append(words, encode(src, base, imm.Value)), nil
	}
	offset, offsetSetup, err := sourceRegister(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	words = append(words, offsetSetup...)
	return append(words,
		riscv.EncodeAdd(xBackendScratch, base, offset),
		encode(src, xBackendScratch, 0),
	), nil
}

func collectCallTargets(program *ir.IrProgram) (map[string]bool, error) {
	targets := map[string]bool{}
	for _, instruction := range program.Instructions {
		if instruction.Opcode != ir.OpCall {
			continue
		}
		label, err := labelOperand(instruction, 0)
		if err != nil {
			return nil, err
		}
		targets[label.Name] = true
	}
	return targets, nil
}

func collectFunctionRoots(program *ir.IrProgram, callTargets map[string]bool) (map[string]bool, error) {
	roots := map[string]bool{}
	startNewRoot := true
	for _, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpLabel {
			label, err := labelOperand(instruction, 0)
			if err != nil {
				return nil, err
			}
			if startNewRoot || label.Name == program.EntryLabel || callTargets[label.Name] {
				roots[label.Name] = true
				startNewRoot = false
			}
		}
		if instruction.Opcode == ir.OpRet || instruction.Opcode == ir.OpHalt {
			startNewRoot = true
		}
	}
	return roots, nil
}

func collectFunctionFrames(program *ir.IrProgram, roots map[string]bool, callTargets map[string]bool) (map[string]*functionFrame, []string, error) {
	frames := map[string]*functionFrame{}
	functionByIndex := make([]string, len(program.Instructions))
	currentRoot := ""

	for index, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpLabel {
			label, err := labelOperand(instruction, 0)
			if err != nil {
				return nil, nil, err
			}
			if roots[label.Name] {
				currentRoot = label.Name
				if _, exists := frames[currentRoot]; !exists {
					frames[currentRoot] = &functionFrame{
						label:              currentRoot,
						savesReturnAddress: callTargets[currentRoot],
						spillSlots:         map[int]int{},
					}
				}
			}
		}
		functionByIndex[index] = currentRoot
		if currentRoot == "" {
			continue
		}
		frame := frames[currentRoot]
		for _, operand := range instruction.Operands {
			register, ok := operand.(ir.IrRegister)
			if !ok {
				continue
			}
			if _, ok := physicalRegister(register.Index); ok {
				continue
			}
			frame.spillSlots[register.Index] = 0
		}
	}

	for _, frame := range frames {
		offset := 0
		if frame.savesReturnAddress {
			offset = 4
			frame.frameSize = 4
		}
		virtuals := make([]int, 0, len(frame.spillSlots))
		for virtual := range frame.spillSlots {
			virtuals = append(virtuals, virtual)
		}
		sort.Ints(virtuals)
		for _, virtual := range virtuals {
			frame.spillSlots[virtual] = offset
			offset += 4
		}
		if offset > frame.frameSize {
			frame.frameSize = offset
		}
	}

	return frames, functionByIndex, nil
}

func collectCallerSavedRegisters(program *ir.IrProgram) []callerSavedRegister {
	seen := map[int]bool{}
	for _, instruction := range program.Instructions {
		for _, operand := range instruction.Operands {
			register, ok := operand.(ir.IrRegister)
			if !ok || isVolatileVirtualRegister(register.Index) {
				continue
			}
			physical, ok := physicalRegister(register.Index)
			if !ok || physical == xRa || physical == xSp || physical == xBackendScratch {
				continue
			}
			seen[register.Index] = true
		}
	}
	virtuals := make([]int, 0, len(seen))
	for virtual := range seen {
		virtuals = append(virtuals, virtual)
	}
	sort.Ints(virtuals)

	registers := make([]callerSavedRegister, 0, len(virtuals))
	for _, virtual := range virtuals {
		physical, _ := physicalRegister(virtual)
		registers = append(registers, callerSavedRegister{virtual: virtual, physical: physical})
	}
	return registers
}

func isVolatileVirtualRegister(index int) bool {
	return index == 0 || index == 1
}

func usesStack(frames map[string]*functionFrame) bool {
	for _, frame := range frames {
		if frame.frameSize > 0 {
			return true
		}
	}
	return false
}

func stackSetupSize(frames map[string]*functionFrame) int {
	if !usesStack(frames) {
		return 0
	}
	return 2
}

func callInstructionSize(regs []callerSavedRegister) int {
	return callerSavePrologueSize(regs) + 1 + callerSaveEpilogueSize(regs)
}

func callerSavePrologueSize(regs []callerSavedRegister) int {
	if len(regs) == 0 {
		return 0
	}
	return stackAdjustInstructionSize(-callerSaveFrameBytes(regs)) + len(regs)
}

func callerSaveEpilogueSize(regs []callerSavedRegister) int {
	if len(regs) == 0 {
		return 0
	}
	return len(regs) + stackAdjustInstructionSize(callerSaveFrameBytes(regs))
}

func callerSaveFrameBytes(regs []callerSavedRegister) int {
	return len(regs) * 4
}

func emitCallWithCallerSaves(offset int, regs []callerSavedRegister) []uint32 {
	words := emitCallerSavePrologue(regs)
	words = append(words, riscv.EncodeJal(xRa, offset))
	words = append(words, emitCallerSaveEpilogue(regs)...)
	return words
}

func emitCallerSavePrologue(regs []callerSavedRegister) []uint32 {
	if len(regs) == 0 {
		return nil
	}
	words := emitStackAdjust(-callerSaveFrameBytes(regs))
	for index, register := range regs {
		words = append(words, riscv.EncodeSw(register.physical, xSp, index*4))
	}
	return words
}

func emitCallerSaveEpilogue(regs []callerSavedRegister) []uint32 {
	if len(regs) == 0 {
		return nil
	}
	words := make([]uint32, 0, len(regs)+stackAdjustInstructionSize(callerSaveFrameBytes(regs)))
	for index, register := range regs {
		words = append(words, riscv.EncodeLw(register.physical, xSp, index*4))
	}
	words = append(words, emitStackAdjust(callerSaveFrameBytes(regs))...)
	return words
}

func emitFunctionPrologue(frame *functionFrame) []uint32 {
	if frame == nil || frame.frameSize == 0 {
		return nil
	}
	words := emitStackAdjust(-frame.frameSize)
	if frame.savesReturnAddress {
		words = append(words, emitStackSlotStore(xRa, 0)...)
	}
	return words
}

func emitRetEpilogue(frame *functionFrame) []uint32 {
	if frame == nil || frame.frameSize == 0 {
		return []uint32{riscv.EncodeJalr(x0, xRa, 0)}
	}
	words := []uint32{}
	if frame.savesReturnAddress {
		words = append(words, emitStackSlotLoad(xRa, 0)...)
	}
	words = append(words, emitStackAdjust(frame.frameSize)...)
	words = append(words, riscv.EncodeJalr(x0, xRa, 0))
	return words
}

func emitThreeReg(instruction ir.IrInstruction, frame *functionFrame, encode func(rd, rs1, rs2 int) uint32) ([]uint32, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	left, leftSetup, err := sourceRegister(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	right, rightSetup, err := sourceRegister(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	words := append([]uint32{}, leftSetup...)
	words = append(words, rightSetup...)
	words = append(words, encode(dst, left, right))
	return appendSpillStore(words, dst, spillOffset), nil
}

func emitRegImm(instruction ir.IrInstruction, frame *functionFrame, encodeImm func(rd, rs1, imm int) uint32, encodeReg func(rd, rs1, rs2 int) uint32) ([]uint32, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	src, srcSetup, err := sourceRegister(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	imm, err := immOperand(instruction, 2)
	if err != nil {
		return nil, err
	}
	words := append([]uint32{}, srcSetup...)
	if fitsSigned(imm.Value, 12) {
		words = append(words, encodeImm(dst, src, imm.Value))
		return appendSpillStore(words, dst, spillOffset), nil
	}
	words = append(words, emitLoadConst(xBackendScratch, imm.Value)...)
	words = append(words, encodeReg(dst, src, xBackendScratch))
	return appendSpillStore(words, dst, spillOffset), nil
}

func emitCmpEqNe(instruction ir.IrInstruction, frame *functionFrame, equal bool) ([]uint32, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	left, leftSetup, err := sourceRegister(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	right, rightSetup, err := sourceRegister(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	words := append([]uint32{}, leftSetup...)
	words = append(words, rightSetup...)
	if equal {
		words = append(words,
			riscv.EncodeXor(dst, left, right),
			riscv.EncodeSltiu(dst, dst, 1),
		)
		return appendSpillStore(words, dst, spillOffset), nil
	}
	words = append(words,
		riscv.EncodeXor(dst, left, right),
		riscv.EncodeSltu(dst, x0, dst),
	)
	return appendSpillStore(words, dst, spillOffset), nil
}

func emitCmpGt(instruction ir.IrInstruction, frame *functionFrame) ([]uint32, error) {
	dst, spillOffset, err := destinationRegister(instruction, frame, 0, xSpillTemp0)
	if err != nil {
		return nil, err
	}
	left, leftSetup, err := sourceRegister(instruction, frame, 1, xSpillTemp1)
	if err != nil {
		return nil, err
	}
	right, rightSetup, err := sourceRegister(instruction, frame, 2, xSpillTemp2)
	if err != nil {
		return nil, err
	}
	words := append([]uint32{}, leftSetup...)
	words = append(words, rightSetup...)
	words = append(words, riscv.EncodeSlt(dst, right, left))
	return appendSpillStore(words, dst, spillOffset), nil
}

func emitBranch(instruction ir.IrInstruction, frame *functionFrame, pc int, plan *compilePlan) ([]uint32, error) {
	register, setup, err := sourceRegister(instruction, frame, 0, xSpillTemp0)
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
	branchPC := pc + len(setup)*4
	offset := target - branchPC
	if err := validateBranchOffset(offset); err != nil {
		return nil, loweringError(instruction, "%v", err)
	}
	if instruction.Opcode == ir.OpBranchZ {
		return append(setup, riscv.EncodeBeq(register, x0, offset)), nil
	}
	return append(setup, riscv.EncodeBne(register, x0, offset)), nil
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

func sourceRegisterSize(instruction ir.IrInstruction, frame *functionFrame, operandIndex int) (int, error) {
	reg, err := regOperand(instruction, operandIndex)
	if err != nil {
		return 0, err
	}
	if _, ok := physicalRegister(reg.Index); ok {
		return 0, nil
	}
	offset, ok := spillSlotOffset(frame, reg.Index)
	if !ok {
		return 0, loweringError(instruction, "virtual register v%d has no physical mapping or spill slot", reg.Index)
	}
	return stackSlotAccessSize(offset), nil
}

func destinationRegisterSize(instruction ir.IrInstruction, frame *functionFrame, operandIndex int) (int, error) {
	reg, err := regOperand(instruction, operandIndex)
	if err != nil {
		return 0, err
	}
	if _, ok := physicalRegister(reg.Index); ok {
		return 0, nil
	}
	offset, ok := spillSlotOffset(frame, reg.Index)
	if !ok {
		return 0, loweringError(instruction, "virtual register v%d has no physical mapping or spill slot", reg.Index)
	}
	return stackSlotAccessSize(offset), nil
}

func sourceRegister(instruction ir.IrInstruction, frame *functionFrame, operandIndex int, spillTemp int) (int, []uint32, error) {
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
	return spillTemp, emitStackSlotLoad(spillTemp, offset), nil
}

func destinationRegister(instruction ir.IrInstruction, frame *functionFrame, operandIndex int, spillTemp int) (int, int, error) {
	reg, err := regOperand(instruction, operandIndex)
	if err != nil {
		return 0, -1, err
	}
	if physical, ok := physicalRegister(reg.Index); ok {
		return physical, -1, nil
	}
	offset, ok := spillSlotOffset(frame, reg.Index)
	if !ok {
		return 0, -1, loweringError(instruction, "virtual register v%d has no physical mapping or spill slot", reg.Index)
	}
	return spillTemp, offset, nil
}

func spillSlotOffset(frame *functionFrame, virtual int) (int, bool) {
	if frame == nil {
		return 0, false
	}
	offset, ok := frame.spillSlots[virtual]
	return offset, ok
}

func appendSpillStore(words []uint32, physical int, spillOffset int) []uint32 {
	if spillOffset < 0 {
		return words
	}
	return append(words, emitStackSlotStore(physical, spillOffset)...)
}

func stackAdjustInstructionSize(amount int) int {
	if amount == 0 {
		return 0
	}
	if fitsSigned(amount, 12) {
		return 1
	}
	return loadConstSize(amount) + 1
}

func stackSlotAccessSize(offset int) int {
	if fitsSigned(offset, 12) {
		return 1
	}
	return loadConstSize(offset) + 2
}

func emitStackAdjust(amount int) []uint32 {
	if amount == 0 {
		return nil
	}
	if fitsSigned(amount, 12) {
		return []uint32{riscv.EncodeAddi(xSp, xSp, amount)}
	}
	words := emitLoadConst(xBackendScratch, amount)
	words = append(words, riscv.EncodeAdd(xSp, xSp, xBackendScratch))
	return words
}

func emitStackSlotLoad(rd, offset int) []uint32 {
	if fitsSigned(offset, 12) {
		return []uint32{riscv.EncodeLw(rd, xSp, offset)}
	}
	words := emitLoadConst(xBackendScratch, offset)
	words = append(words,
		riscv.EncodeAdd(xBackendScratch, xSp, xBackendScratch),
		riscv.EncodeLw(rd, xBackendScratch, 0),
	)
	return words
}

func emitStackSlotStore(rs, offset int) []uint32 {
	if fitsSigned(offset, 12) {
		return []uint32{riscv.EncodeSw(rs, xSp, offset)}
	}
	words := emitLoadConst(xBackendScratch, offset)
	words = append(words,
		riscv.EncodeAdd(xBackendScratch, xSp, xBackendScratch),
		riscv.EncodeSw(rs, xBackendScratch, 0),
	)
	return words
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
	if index >= 0 && index < len(starterPhysicalRegisters) {
		return starterPhysicalRegisters[index], true
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
