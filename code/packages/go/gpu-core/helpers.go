package gpucore

// Helper constructors -- make programs readable.
//
// Writing programs as raw Instruction{...} literals is verbose. These helper
// functions make programs read like assembly language:
//
//	// Without helpers (verbose):
//	program := []Instruction{
//	    {Op: OpLIMM, Rd: 0, Immediate: 2.0},
//	    {Op: OpLIMM, Rd: 1, Immediate: 3.0},
//	    {Op: OpFMUL, Rd: 2, Rs1: 0, Rs2: 1},
//	    {Op: OpHALT},
//	}
//
//	// With helpers (clean):
//	program := []Instruction{
//	    Limm(0, 2.0),
//	    Limm(1, 3.0),
//	    Fmul(2, 0, 1),
//	    Halt(),
//	}

// =========================================================================
// Arithmetic helpers
// =========================================================================

// Fadd creates an FADD instruction: Rd = Rs1 + Rs2.
//
// Floating-point addition. The fp-arithmetic package's gate-level FPAdd
// function does the actual computation.
func Fadd(rd, rs1, rs2 int) Instruction {
	return Instruction{Op: OpFADD, Rd: rd, Rs1: rs1, Rs2: rs2}
}

// Fsub creates an FSUB instruction: Rd = Rs1 - Rs2.
//
// Floating-point subtraction. Implemented as addition with the second
// operand negated: a - b = a + (-b).
func Fsub(rd, rs1, rs2 int) Instruction {
	return Instruction{Op: OpFSUB, Rd: rd, Rs1: rs1, Rs2: rs2}
}

// Fmul creates an FMUL instruction: Rd = Rs1 * Rs2.
//
// Floating-point multiplication. Sign is XOR of operand signs, exponents
// add, mantissas multiply.
func Fmul(rd, rs1, rs2 int) Instruction {
	return Instruction{Op: OpFMUL, Rd: rd, Rs1: rs1, Rs2: rs2}
}

// Ffma creates an FFMA instruction: Rd = Rs1 * Rs2 + Rs3.
//
// Fused multiply-add. Computes the product and sum with only ONE rounding
// step at the end, giving better precision than separate multiply + add.
// This is the fundamental operation in matrix multiplication and ML training.
func Ffma(rd, rs1, rs2, rs3 int) Instruction {
	return Instruction{Op: OpFFMA, Rd: rd, Rs1: rs1, Rs2: rs2, Rs3: rs3}
}

// Fneg creates an FNEG instruction: Rd = -Rs1.
//
// Negate a floating-point value. In hardware, this is a single XOR gate
// on the sign bit.
func Fneg(rd, rs1 int) Instruction {
	return Instruction{Op: OpFNEG, Rd: rd, Rs1: rs1}
}

// Fabs creates an FABS instruction: Rd = |Rs1|.
//
// Absolute value. In hardware, this just forces the sign bit to 0.
func Fabs(rd, rs1 int) Instruction {
	return Instruction{Op: OpFABS, Rd: rd, Rs1: rs1}
}

// =========================================================================
// Memory helpers
// =========================================================================

// Load creates a LOAD instruction: Rd = Mem[Rs1 + offset].
//
// Load a floating-point value from local memory into a register. The address
// is computed as the float value in Rs1 plus the immediate offset.
func Load(rd, rs1 int, offset float64) Instruction {
	return Instruction{Op: OpLOAD, Rd: rd, Rs1: rs1, Immediate: offset}
}

// Store creates a STORE instruction: Mem[Rs1 + offset] = Rs2.
//
// Store a register's floating-point value to local memory. The address
// is computed as the float value in Rs1 plus the immediate offset.
func Store(rs1, rs2 int, offset float64) Instruction {
	return Instruction{Op: OpSTORE, Rs1: rs1, Rs2: rs2, Immediate: offset}
}

// =========================================================================
// Data movement helpers
// =========================================================================

// Mov creates a MOV instruction: Rd = Rs1.
//
// Copy one register to another. The source register is unchanged.
func Mov(rd, rs1 int) Instruction {
	return Instruction{Op: OpMOV, Rd: rd, Rs1: rs1}
}

// Limm creates a LIMM instruction: Rd = value.
//
// Load an immediate (literal) float value directly into a register.
// "LIMM" stands for "Load Immediate." This is how constants enter
// the register file.
func Limm(rd int, value float64) Instruction {
	return Instruction{Op: OpLIMM, Rd: rd, Immediate: value}
}

// =========================================================================
// Control flow helpers
// =========================================================================

// Beq creates a BEQ instruction: if Rs1 == Rs2, PC += offset.
//
// Branch if equal. If the two source registers hold equal values,
// the program counter jumps forward (positive offset) or backward
// (negative offset) by the given number of instructions.
func Beq(rs1, rs2, offset int) Instruction {
	return Instruction{Op: OpBEQ, Rs1: rs1, Rs2: rs2, Immediate: float64(offset)}
}

// Blt creates a BLT instruction: if Rs1 < Rs2, PC += offset.
//
// Branch if less than. Uses the fp-arithmetic FPCompare function to
// determine ordering.
func Blt(rs1, rs2, offset int) Instruction {
	return Instruction{Op: OpBLT, Rs1: rs1, Rs2: rs2, Immediate: float64(offset)}
}

// Bne creates a BNE instruction: if Rs1 != Rs2, PC += offset.
//
// Branch if not equal. The complement of BEQ.
func Bne(rs1, rs2, offset int) Instruction {
	return Instruction{Op: OpBNE, Rs1: rs1, Rs2: rs2, Immediate: float64(offset)}
}

// Jmp creates a JMP instruction: PC = target.
//
// Unconditional jump to an absolute program counter address.
func Jmp(target int) Instruction {
	return Instruction{Op: OpJMP, Immediate: float64(target)}
}

// Nop creates a NOP instruction.
//
// No operation -- the program counter advances but nothing else happens.
// Useful as a placeholder or for timing purposes.
func Nop() Instruction {
	return Instruction{Op: OpNOP}
}

// Halt creates a HALT instruction.
//
// Stop execution. The core enters the halted state and will not execute
// any more instructions until reset.
func Halt() Instruction {
	return Instruction{Op: OpHALT}
}
