// Package compilerir provides the intermediate representation (IR) for the
// AOT native compiler pipeline.
//
// ──────────────────────────────────────────────────────────────────────────────
// Design Philosophy
// ──────────────────────────────────────────────────────────────────────────────
//
// This IR is **general-purpose** — designed to serve as the compilation
// target for any compiled language, not just Brainfuck. The current v1
// instruction set is sufficient for Brainfuck; BASIC (the next planned
// frontend) will add opcodes for multiplication, division, floating-point
// arithmetic, and string operations.
//
// Key rules:
//   1. Existing opcodes never change semantics — only new ones are appended.
//   2. A new opcode is added only when a frontend needs it AND it cannot
//      be efficiently expressed as a sequence of existing opcodes.
//   3. All frontends and backends remain forward-compatible.
//
// ──────────────────────────────────────────────────────────────────────────────
// IR Characteristics
// ──────────────────────────────────────────────────────────────────────────────
//
// The IR is:
//   - Linear: no basic blocks, no SSA, no phi nodes
//   - Register-based: infinite virtual registers (v0, v1, ...)
//   - Target-independent: backends map IR to physical ISA
//   - Versioned: .version directive in text format (v1 = Brainfuck subset)
//
package compilerir

// ──────────────────────────────────────────────────────────────────────────────
// IrOp — the opcode enumeration
//
// Each opcode represents a single operation. Opcodes are grouped by
// category:
//
//   Constants:    LOAD_IMM, LOAD_ADDR
//   Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
//   Arithmetic:   ADD, ADD_IMM, SUB, AND, AND_IMM
//   Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
//   Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
//   System:       SYSCALL, HALT
//   Meta:         NOP, COMMENT
//
// ──────────────────────────────────────────────────────────────────────────────

type IrOp int

const (
	// ── Constants ──────────────────────────────────────────────────────────
	// Load an immediate integer value into a register.
	//   LOAD_IMM  v0, 42    →  v0 = 42
	OpLoadImm IrOp = iota

	// Load the address of a data label into a register.
	//   LOAD_ADDR v0, tape  →  v0 = &tape
	OpLoadAddr

	// ── Memory ────────────────────────────────────────────────────────────
	// Load a byte from memory: dst = mem[base + offset] (zero-extended).
	//   LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF
	OpLoadByte

	// Store a byte to memory: mem[base + offset] = src & 0xFF.
	//   STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF
	OpStoreByte

	// Load a machine word from memory: dst = *(word*)(base + offset).
	//   LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)
	OpLoadWord

	// Store a machine word to memory: *(word*)(base + offset) = src.
	//   STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2
	OpStoreWord

	// ── Arithmetic ────────────────────────────────────────────────────────
	// Register-register addition: dst = lhs + rhs.
	//   ADD v3, v1, v2  →  v3 = v1 + v2
	OpAdd

	// Register-immediate addition: dst = src + immediate.
	//   ADD_IMM v1, v1, 1  →  v1 = v1 + 1
	OpAddImm

	// Register-register subtraction: dst = lhs - rhs.
	//   SUB v3, v1, v2  →  v3 = v1 - v2
	OpSub

	// Register-register bitwise AND: dst = lhs & rhs.
	//   AND v3, v1, v2  →  v3 = v1 & v2
	OpAnd

	// Register-immediate bitwise AND: dst = src & immediate.
	//   AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF
	OpAndImm

	// ── Comparison ────────────────────────────────────────────────────────
	// Set dst = 1 if lhs == rhs, else 0.
	//   CMP_EQ v4, v1, v2  →  v4 = (v1 == v2) ? 1 : 0
	OpCmpEq

	// Set dst = 1 if lhs != rhs, else 0.
	//   CMP_NE v4, v1, v2  →  v4 = (v1 != v2) ? 1 : 0
	OpCmpNe

	// Set dst = 1 if lhs < rhs (signed), else 0.
	//   CMP_LT v4, v1, v2  →  v4 = (v1 < v2) ? 1 : 0
	OpCmpLt

	// Set dst = 1 if lhs > rhs (signed), else 0.
	//   CMP_GT v4, v1, v2  →  v4 = (v1 > v2) ? 1 : 0
	OpCmpGt

	// ── Control Flow ──────────────────────────────────────────────────────
	// Define a label at this point in the instruction stream.
	// Labels produce no machine code — they just record an address.
	//   LABEL loop_start
	OpLabel

	// Unconditional jump to a label.
	//   JUMP loop_start  →  PC = &loop_start
	OpJump

	// Conditional branch: jump to label if register == 0.
	//   BRANCH_Z v2, loop_end  →  if v2 == 0 then PC = &loop_end
	OpBranchZ

	// Conditional branch: jump to label if register != 0.
	//   BRANCH_NZ v2, loop_end  →  if v2 != 0 then PC = &loop_end
	OpBranchNz

	// Call a subroutine at the given label. Pushes return address.
	//   CALL my_func
	OpCall

	// Return from a subroutine. Pops return address.
	//   RET
	OpRet

	// ── System ────────────────────────────────────────────────────────────
	// Invoke a system call. The syscall number is an immediate operand.
	// Arguments and return values follow the platform's syscall ABI.
	//   SYSCALL 1  →  ecall with a7=1 (write)
	OpSyscall

	// Halt execution. The program terminates.
	//   HALT  →  ecall with a7=10 (exit)
	OpHalt

	// ── Meta ──────────────────────────────────────────────────────────────
	// No operation. Produces a single NOP instruction in the backend.
	//   NOP
	OpNop

	// A human-readable comment. Produces no machine code.
	// Useful for debugging IR output.
	//   COMMENT "load tape base address"
	OpComment
)

// ──────────────────────────────────────────────────────────────────────────────
// String representation
//
// Maps each opcode to its canonical text name. These names are used by
// the IR printer and parser for roundtrip fidelity.
// ──────────────────────────────────────────────────────────────────────────────

var opNames = map[IrOp]string{
	OpLoadImm:  "LOAD_IMM",
	OpLoadAddr: "LOAD_ADDR",
	OpLoadByte: "LOAD_BYTE",
	OpStoreByte: "STORE_BYTE",
	OpLoadWord: "LOAD_WORD",
	OpStoreWord: "STORE_WORD",
	OpAdd:      "ADD",
	OpAddImm:   "ADD_IMM",
	OpSub:      "SUB",
	OpAnd:      "AND",
	OpAndImm:   "AND_IMM",
	OpCmpEq:    "CMP_EQ",
	OpCmpNe:    "CMP_NE",
	OpCmpLt:    "CMP_LT",
	OpCmpGt:    "CMP_GT",
	OpLabel:    "LABEL",
	OpJump:     "JUMP",
	OpBranchZ:  "BRANCH_Z",
	OpBranchNz: "BRANCH_NZ",
	OpCall:     "CALL",
	OpRet:      "RET",
	OpSyscall:  "SYSCALL",
	OpHalt:     "HALT",
	OpNop:      "NOP",
	OpComment:  "COMMENT",
}

// String returns the canonical text name for an IR opcode.
func (op IrOp) String() string {
	if name, ok := opNames[op]; ok {
		return name
	}
	return "UNKNOWN"
}

// ──────────────────────────────────────────────────────────────────────────────
// ParseOp converts a text opcode name to its IrOp value.
//
// Returns the opcode and true if found, or (0, false) if the name is
// not recognised. This is the inverse of IrOp.String().
// ──────────────────────────────────────────────────────────────────────────────

var nameToOp map[string]IrOp

func init() {
	nameToOp = make(map[string]IrOp, len(opNames))
	for op, name := range opNames {
		nameToOp[name] = op
	}
}

// ParseOp converts a text opcode name (e.g., "ADD_IMM") to its IrOp value.
func ParseOp(name string) (IrOp, bool) {
	op, ok := nameToOp[name]
	return op, ok
}
