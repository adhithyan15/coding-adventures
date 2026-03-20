package gpucore

// Opcodes and Instructions -- the vocabulary of GPU core programs.
//
// # What is an Opcode?
//
// An opcode (operation code) is a number or name that tells the processor what
// to do. It's like a verb in a sentence:
//
//	English:  "Add the first two numbers and store in the third"
//	Assembly: FADD R2, R0, R1
//
// The opcode is FADD. The registers R0, R1, R2 are the operands.
//
// # Instruction Representation
//
// Real GPU hardware represents instructions as binary words (32 or 64 bits of
// 1s and 0s packed together). But at this layer -- the processing element
// simulator -- we use a structured Go struct instead:
//
//	Binary (real hardware): 01001000_00000010_00000000_00000001
//	Our representation:     Instruction{Op: OpFADD, Rd: 2, Rs1: 0, Rs2: 1}
//
// Why? Because binary encoding is the job of the *assembler* layer above us.
// The processing element receives already-decoded instructions from the
// instruction cache. We're simulating what happens *after* decode.
//
// # The Instruction Set
//
// Our GenericISA has 16 opcodes organized into four categories:
//
//	Arithmetic:  FADD, FSUB, FMUL, FFMA, FNEG, FABS  (6 opcodes)
//	Memory:      LOAD, STORE                           (2 opcodes)
//	Data move:   MOV, LIMM                             (2 opcodes)
//	Control:     BEQ, BLT, BNE, JMP, NOP, HALT         (6 opcodes)
//
// This is deliberately minimal. Real ISAs have hundreds of opcodes, but these
// 16 are enough to write any floating-point program (they're Turing-complete
// when combined with branches and memory).

import "fmt"

// =========================================================================
// Opcode -- the set of operations a GPU core can perform
// =========================================================================

// Opcode represents a GPU core operation. We use iota for a compact enum.
//
// The opcodes are organized by category:
//
//	Floating-point arithmetic (uses fp-arithmetic package):
//	    OpFADD  -- add two registers
//	    OpFSUB  -- subtract two registers
//	    OpFMUL  -- multiply two registers
//	    OpFFMA  -- fused multiply-add (three source registers)
//	    OpFNEG  -- negate a register
//	    OpFABS  -- absolute value of a register
//
//	Memory operations:
//	    OpLOAD  -- load float from memory into register
//	    OpSTORE -- store register value to memory
//
//	Data movement:
//	    OpMOV   -- copy one register to another
//	    OpLIMM  -- load an immediate (literal) float value
//
//	Control flow:
//	    OpBEQ   -- branch if equal
//	    OpBLT   -- branch if less than
//	    OpBNE   -- branch if not equal
//	    OpJMP   -- unconditional jump
//	    OpNOP   -- no operation
//	    OpHALT  -- stop execution
type Opcode int

const (
	// Arithmetic opcodes. These perform floating-point math using the
	// fp-arithmetic package's gate-level implementations.
	OpFADD Opcode = iota // Rd = Rs1 + Rs2
	OpFSUB               // Rd = Rs1 - Rs2
	OpFMUL               // Rd = Rs1 * Rs2
	OpFFMA               // Rd = Rs1 * Rs2 + Rs3
	OpFNEG               // Rd = -Rs1
	OpFABS               // Rd = |Rs1|

	// Memory opcodes. These move data between registers and local memory.
	OpLOAD  // Rd = Mem[Rs1 + immediate]
	OpSTORE // Mem[Rs1 + immediate] = Rs2

	// Data movement opcodes. These copy values between registers or load
	// literal constants.
	OpMOV  // Rd = Rs1
	OpLIMM // Rd = immediate (literal float)

	// Control flow opcodes. These change the program counter based on
	// conditions or unconditionally.
	OpBEQ  // if Rs1 == Rs2: PC += immediate
	OpBLT  // if Rs1 < Rs2: PC += immediate
	OpBNE  // if Rs1 != Rs2: PC += immediate
	OpJMP  // PC = immediate (absolute jump)
	OpNOP  // no operation, advance PC
	OpHALT // stop execution
)

// opcodeNames maps each Opcode to its assembly mnemonic.
var opcodeNames = map[Opcode]string{
	OpFADD: "FADD", OpFSUB: "FSUB", OpFMUL: "FMUL", OpFFMA: "FFMA",
	OpFNEG: "FNEG", OpFABS: "FABS",
	OpLOAD: "LOAD", OpSTORE: "STORE",
	OpMOV: "MOV", OpLIMM: "LIMM",
	OpBEQ: "BEQ", OpBLT: "BLT", OpBNE: "BNE", OpJMP: "JMP",
	OpNOP: "NOP", OpHALT: "HALT",
}

// String returns the assembly mnemonic for an opcode.
func (op Opcode) String() string {
	if name, ok := opcodeNames[op]; ok {
		return name
	}
	return fmt.Sprintf("UNKNOWN(%d)", int(op))
}

// =========================================================================
// Instruction -- a single GPU core instruction
// =========================================================================

// Instruction is a structured representation of a GPU core instruction.
//
// This is NOT a binary encoding -- it contains all the information needed to
// execute the instruction: the opcode and up to four operands.
//
// Fields:
//   - Op: What operation to perform (see Opcode constants).
//   - Rd: Destination register index (0-255).
//   - Rs1: First source register index (0-255).
//   - Rs2: Second source register index (0-255).
//   - Rs3: Third source register (used only by FFMA).
//   - Immediate: A literal float value (used by LIMM, branch offsets,
//     memory offsets). For branches, this is the number of instructions
//     to skip (positive = forward, negative = back).
type Instruction struct {
	Op        Opcode
	Rd        int
	Rs1       int
	Rs2       int
	Rs3       int
	Immediate float64
}

// String pretty-prints the instruction in assembly-like syntax.
//
// This makes programs readable when printed. Each opcode has its own
// formatting convention:
//
//	FADD R2, R0, R1          (three-register arithmetic)
//	FFMA R3, R0, R1, R2      (four-register FMA)
//	LOAD R0, [R1+0]          (memory load with offset)
//	LIMM R0, 3.14            (immediate load)
//	BEQ R0, R1, +3           (conditional branch with offset)
//	HALT                     (no operands)
func (inst Instruction) String() string {
	switch inst.Op {
	case OpFADD, OpFSUB, OpFMUL:
		return fmt.Sprintf("%s R%d, R%d, R%d", inst.Op, inst.Rd, inst.Rs1, inst.Rs2)
	case OpFFMA:
		return fmt.Sprintf("%s R%d, R%d, R%d, R%d", inst.Op, inst.Rd, inst.Rs1, inst.Rs2, inst.Rs3)
	case OpFNEG, OpFABS:
		return fmt.Sprintf("%s R%d, R%d", inst.Op, inst.Rd, inst.Rs1)
	case OpLOAD:
		return fmt.Sprintf("%s R%d, [R%d+%g]", inst.Op, inst.Rd, inst.Rs1, inst.Immediate)
	case OpSTORE:
		return fmt.Sprintf("%s [R%d+%g], R%d", inst.Op, inst.Rs1, inst.Immediate, inst.Rs2)
	case OpMOV:
		return fmt.Sprintf("%s R%d, R%d", inst.Op, inst.Rd, inst.Rs1)
	case OpLIMM:
		return fmt.Sprintf("%s R%d, %g", inst.Op, inst.Rd, inst.Immediate)
	case OpBEQ, OpBLT, OpBNE:
		sign := "+"
		if inst.Immediate < 0 {
			sign = ""
		}
		return fmt.Sprintf("%s R%d, R%d, %s%d", inst.Op, inst.Rs1, inst.Rs2, sign, int(inst.Immediate))
	case OpJMP:
		return fmt.Sprintf("%s %d", inst.Op, int(inst.Immediate))
	case OpNOP:
		return "NOP"
	case OpHALT:
		return "HALT"
	default:
		return fmt.Sprintf("%s rd=%d rs1=%d rs2=%d", inst.Op, inst.Rd, inst.Rs1, inst.Rs2)
	}
}
