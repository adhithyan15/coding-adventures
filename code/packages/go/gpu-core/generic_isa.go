package gpucore

// GenericISA -- a simplified, educational instruction set.
//
// # What is this?
//
// This is the default InstructionSet implementation -- a vendor-neutral ISA
// designed for teaching, not for matching any real hardware. It proves that
// the pluggable ISA design works: if you can implement GenericISA, you can
// implement NVIDIA PTX, AMD GCN, Intel Xe, or ARM Mali the same way.
//
// # How it works
//
// The GenericISA.Execute() method is a big switch statement. For each opcode,
// it:
//  1. Reads source registers
//  2. Calls the appropriate fp-arithmetic function
//  3. Writes the result to the destination register
//  4. Returns an ExecuteResult describing what happened
//
//	FADD R2, R0, R1:
//	    a = regs.Read(R0)          // read 3.14
//	    b = regs.Read(R1)          // read 2.71
//	    result = FPAdd(a, b)       // 3.14 + 2.71 = 5.85
//	    regs.Write(R2, result)     // store in R2
//	    return ExecuteResult{Description: "R2 = R0 + R1 = 3.14 + 2.71 = 5.85", ...}
//
// # Future ISAs follow the same pattern
//
//	type PTXISA struct{}
//	func (p PTXISA) Execute(inst, regs, mem) ExecuteResult {
//	    switch inst.Op {
//	    case PTXOpADDF32:   // same as FADD but with PTX naming
//	    case PTXOpFMARNF32: // same as FFMA but with PTX naming
//	    }
//	}
//
// The GPUCore doesn't care which ISA is plugged in -- it just calls
// isa.Execute() and processes the ExecuteResult.

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// GenericISA is a simplified, educational instruction set for GPU cores.
//
// This ISA is not tied to any vendor -- it's a teaching tool. It has
// 16 opcodes covering arithmetic, memory, data movement, and control
// flow. Any floating-point program can be expressed with these.
//
// To use a different ISA, create a type with the same Name() and Execute()
// methods and pass it to NewGPUCore(WithISA(yourISA)).
type GenericISA struct{}

// Name returns the ISA identifier.
func (g GenericISA) Name() string {
	return "Generic"
}

// Execute executes a single instruction.
//
// This is the heart of the ISA -- a dispatch table that maps opcodes to
// their implementations. Each case reads operands, performs the operation,
// writes results, and returns a trace description.
func (g GenericISA) Execute(inst Instruction, regs *FPRegisterFile, mem *LocalMemory) ExecuteResult {
	switch inst.Op {
	// --- Floating-point arithmetic ---
	case OpFADD:
		return g.execFadd(inst, regs)
	case OpFSUB:
		return g.execFsub(inst, regs)
	case OpFMUL:
		return g.execFmul(inst, regs)
	case OpFFMA:
		return g.execFfma(inst, regs)
	case OpFNEG:
		return g.execFneg(inst, regs)
	case OpFABS:
		return g.execFabs(inst, regs)

	// --- Memory ---
	case OpLOAD:
		return g.execLoad(inst, regs, mem)
	case OpSTORE:
		return g.execStore(inst, regs, mem)

	// --- Data movement ---
	case OpMOV:
		return g.execMov(inst, regs)
	case OpLIMM:
		return g.execLimm(inst, regs)

	// --- Control flow ---
	case OpBEQ:
		return g.execBeq(inst, regs)
	case OpBLT:
		return g.execBlt(inst, regs)
	case OpBNE:
		return g.execBne(inst, regs)
	case OpJMP:
		return g.execJmp(inst)
	case OpNOP:
		return NewExecuteResult("No operation")
	case OpHALT:
		return ExecuteResult{Description: "Halted", NextPCOffset: 1, Halted: true}

	default:
		// This should never happen if all opcodes are covered.
		return ExecuteResult{
			Description:  fmt.Sprintf("Unknown opcode: %s", inst.Op),
			NextPCOffset: 1,
		}
	}
}

// =========================================================================
// Arithmetic implementations
// =========================================================================

// execFadd implements FADD Rd, Rs1, Rs2 -> Rd = Rs1 + Rs2.
func (g GenericISA) execFadd(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	result := fp.FPAdd(a, b)
	_ = regs.Write(inst.Rd, result)

	aF, bF, rF := fp.BitsToFloat(a), fp.BitsToFloat(b), fp.BitsToFloat(result)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = R%d + R%d = %g + %g = %g",
			inst.Rd, inst.Rs1, inst.Rs2, aF, bF, rF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): rF},
	}
}

// execFsub implements FSUB Rd, Rs1, Rs2 -> Rd = Rs1 - Rs2.
func (g GenericISA) execFsub(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	result := fp.FPSub(a, b)
	_ = regs.Write(inst.Rd, result)

	aF, bF, rF := fp.BitsToFloat(a), fp.BitsToFloat(b), fp.BitsToFloat(result)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = R%d - R%d = %g - %g = %g",
			inst.Rd, inst.Rs1, inst.Rs2, aF, bF, rF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): rF},
	}
}

// execFmul implements FMUL Rd, Rs1, Rs2 -> Rd = Rs1 * Rs2.
func (g GenericISA) execFmul(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	result := fp.FPMul(a, b)
	_ = regs.Write(inst.Rd, result)

	aF, bF, rF := fp.BitsToFloat(a), fp.BitsToFloat(b), fp.BitsToFloat(result)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = R%d * R%d = %g * %g = %g",
			inst.Rd, inst.Rs1, inst.Rs2, aF, bF, rF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): rF},
	}
}

// execFfma implements FFMA Rd, Rs1, Rs2, Rs3 -> Rd = Rs1 * Rs2 + Rs3.
//
// Fused multiply-add is the fundamental ML operation. It computes the
// product and sum with only ONE rounding step, giving better precision
// than separate multiply + add.
func (g GenericISA) execFfma(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	c, _ := regs.Read(inst.Rs3)
	result := fp.FMA(a, b, c)
	_ = regs.Write(inst.Rd, result)

	aF := fp.BitsToFloat(a)
	bF := fp.BitsToFloat(b)
	cF := fp.BitsToFloat(c)
	rF := fp.BitsToFloat(result)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = R%d * R%d + R%d = %g * %g + %g = %g",
			inst.Rd, inst.Rs1, inst.Rs2, inst.Rs3, aF, bF, cF, rF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): rF},
	}
}

// execFneg implements FNEG Rd, Rs1 -> Rd = -Rs1.
func (g GenericISA) execFneg(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	result := fp.FPNeg(a)
	_ = regs.Write(inst.Rd, result)

	aF, rF := fp.BitsToFloat(a), fp.BitsToFloat(result)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = -R%d = -%g = %g",
			inst.Rd, inst.Rs1, aF, rF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): rF},
	}
}

// execFabs implements FABS Rd, Rs1 -> Rd = |Rs1|.
func (g GenericISA) execFabs(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	result := fp.FPAbs(a)
	_ = regs.Write(inst.Rd, result)

	aF, rF := fp.BitsToFloat(a), fp.BitsToFloat(result)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = |R%d| = |%g| = %g",
			inst.Rd, inst.Rs1, aF, rF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): rF},
	}
}

// =========================================================================
// Memory implementations
// =========================================================================

// execLoad implements LOAD Rd, [Rs1+imm] -> Rd = Mem[Rs1 + immediate].
func (g GenericISA) execLoad(inst Instruction, regs *FPRegisterFile, mem *LocalMemory) ExecuteResult {
	base, _ := regs.ReadFloat(inst.Rs1)
	address := int(base + inst.Immediate)
	value, _ := mem.LoadFloat(address, regs.Fmt)
	_ = regs.Write(inst.Rd, value)

	valF := fp.BitsToFloat(value)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = Mem[R%d+%g] = Mem[%d] = %g",
			inst.Rd, inst.Rs1, inst.Immediate, address, valF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): valF},
	}
}

// execStore implements STORE [Rs1+imm], Rs2 -> Mem[Rs1 + immediate] = Rs2.
func (g GenericISA) execStore(inst Instruction, regs *FPRegisterFile, mem *LocalMemory) ExecuteResult {
	base, _ := regs.ReadFloat(inst.Rs1)
	address := int(base + inst.Immediate)
	value, _ := regs.Read(inst.Rs2)
	_ = mem.StoreFloat(address, value)

	valF := fp.BitsToFloat(value)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"Mem[R%d+%g] = R%d -> Mem[%d] = %g",
			inst.Rs1, inst.Immediate, inst.Rs2, address, valF,
		),
		NextPCOffset:    1,
		MemoryChanged:   map[int]float64{address: valF},
	}
}

// =========================================================================
// Data movement implementations
// =========================================================================

// execMov implements MOV Rd, Rs1 -> Rd = Rs1.
func (g GenericISA) execMov(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	value, _ := regs.Read(inst.Rs1)
	_ = regs.Write(inst.Rd, value)

	valF := fp.BitsToFloat(value)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = R%d = %g",
			inst.Rd, inst.Rs1, valF,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): valF},
	}
}

// execLimm implements LIMM Rd, immediate -> Rd = float literal.
func (g GenericISA) execLimm(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	_ = regs.WriteFloat(inst.Rd, inst.Immediate)
	return ExecuteResult{
		Description: fmt.Sprintf(
			"R%d = %g",
			inst.Rd, inst.Immediate,
		),
		NextPCOffset:     1,
		RegistersChanged: map[string]float64{fmt.Sprintf("R%d", inst.Rd): inst.Immediate},
	}
}

// =========================================================================
// Control flow implementations
// =========================================================================

// execBeq implements BEQ Rs1, Rs2, offset -> if Rs1 == Rs2: PC += offset.
func (g GenericISA) execBeq(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	cmp := fp.FPCompare(a, b)
	taken := cmp == 0

	offset := 1
	if taken {
		offset = int(inst.Immediate)
	}

	aF := fp.BitsToFloat(a)
	bF := fp.BitsToFloat(b)
	takenStr := "No -> fall through"
	if taken {
		takenStr = "Yes -> branch"
	}
	return ExecuteResult{
		Description: fmt.Sprintf(
			"BEQ R%d(%g) == R%d(%g)? %s",
			inst.Rs1, aF, inst.Rs2, bF, takenStr,
		),
		NextPCOffset: offset,
	}
}

// execBlt implements BLT Rs1, Rs2, offset -> if Rs1 < Rs2: PC += offset.
func (g GenericISA) execBlt(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	cmp := fp.FPCompare(a, b)
	taken := cmp < 0

	offset := 1
	if taken {
		offset = int(inst.Immediate)
	}

	aF := fp.BitsToFloat(a)
	bF := fp.BitsToFloat(b)
	takenStr := "No -> fall through"
	if taken {
		takenStr = "Yes -> branch"
	}
	return ExecuteResult{
		Description: fmt.Sprintf(
			"BLT R%d(%g) < R%d(%g)? %s",
			inst.Rs1, aF, inst.Rs2, bF, takenStr,
		),
		NextPCOffset: offset,
	}
}

// execBne implements BNE Rs1, Rs2, offset -> if Rs1 != Rs2: PC += offset.
func (g GenericISA) execBne(inst Instruction, regs *FPRegisterFile) ExecuteResult {
	a, _ := regs.Read(inst.Rs1)
	b, _ := regs.Read(inst.Rs2)
	cmp := fp.FPCompare(a, b)
	taken := cmp != 0

	offset := 1
	if taken {
		offset = int(inst.Immediate)
	}

	aF := fp.BitsToFloat(a)
	bF := fp.BitsToFloat(b)
	takenStr := "No -> fall through"
	if taken {
		takenStr = "Yes -> branch"
	}
	return ExecuteResult{
		Description: fmt.Sprintf(
			"BNE R%d(%g) != R%d(%g)? %s",
			inst.Rs1, aF, inst.Rs2, bF, takenStr,
		),
		NextPCOffset: offset,
	}
}

// execJmp implements JMP target -> PC = target (absolute jump).
func (g GenericISA) execJmp(inst Instruction) ExecuteResult {
	target := int(inst.Immediate)
	return ExecuteResult{
		Description:  fmt.Sprintf("Jump to PC=%d", target),
		NextPCOffset: target,
		AbsoluteJump: true,
	}
}
