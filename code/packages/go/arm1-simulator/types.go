// =========================================================================
// types.go — ARM1 Data Types and Constants
// =========================================================================
//
// The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
// in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
// correctly on the very first attempt. This file defines the data types
// that model the ARM1's architectural state.
//
// # R15: The Combined PC + Status Register
//
// ARMv1's most distinctive architectural feature is that the program counter
// and processor status flags share a single 32-bit register (R15):
//
//   Bit 31: N (Negative)     Bit 27: I (IRQ disable)
//   Bit 30: Z (Zero)         Bit 26: F (FIQ disable)
//   Bit 29: C (Carry)        Bits 25:2: Program Counter (24 bits)
//   Bit 28: V (Overflow)     Bits 1:0: Processor Mode
//
// Because instructions are 32-bit and word-aligned, the bottom 2 bits of
// the address are always 0 — so they're repurposed for the mode bits.
// Later ARM architectures (ARMv3+) separated PC and status into distinct
// registers (PC and CPSR). But on the ARM1, they're one and the same.

package arm1simulator

// =========================================================================
// Processor Mode
// =========================================================================
//
// The ARM1 supports 4 processor modes. Each mode has its own banked copies
// of certain registers, allowing fast context switching (especially for FIQ,
// which banks 7 registers to avoid saving/restoring them in the handler).
//
//   Mode  M1:M0  Banked Registers
//   ────  ─────  ────────────────
//   USR   0b00   (none — base set)
//   FIQ   0b01   R8_fiq..R12_fiq, R13_fiq, R14_fiq
//   IRQ   0b10   R13_irq, R14_irq
//   SVC   0b11   R13_svc, R14_svc

const (
	ModeUSR = 0 // User mode — normal program execution
	ModeFIQ = 1 // Fast Interrupt — banks R8-R14 for zero-overhead handlers
	ModeIRQ = 2 // Normal Interrupt — banks R13-R14
	ModeSVC = 3 // Supervisor — entered via SWI or Reset
)

// ModeString returns a human-readable name for a processor mode.
func ModeString(mode int) string {
	switch mode {
	case ModeUSR:
		return "USR"
	case ModeFIQ:
		return "FIQ"
	case ModeIRQ:
		return "IRQ"
	case ModeSVC:
		return "SVC"
	default:
		return "???"
	}
}

// =========================================================================
// Condition Codes
// =========================================================================
//
// Every ARM instruction has a 4-bit condition code in bits 31:28.
// The instruction only executes if the condition is met. This is ARM's
// signature feature — even data processing and load/store instructions
// can be conditional, eliminating many branches.
//
// Example: ADDNE R0, R1, R2  — only executes if Z flag is clear (not equal)

const (
	CondEQ = 0x0 // Equal — Z set
	CondNE = 0x1 // Not equal — Z clear
	CondCS = 0x2 // Carry set / unsigned higher or same
	CondCC = 0x3 // Carry clear / unsigned lower
	CondMI = 0x4 // Minus / negative — N set
	CondPL = 0x5 // Plus / positive or zero — N clear
	CondVS = 0x6 // Overflow set
	CondVC = 0x7 // Overflow clear
	CondHI = 0x8 // Unsigned higher — C set AND Z clear
	CondLS = 0x9 // Unsigned lower or same — C clear OR Z set
	CondGE = 0xA // Signed greater or equal — N == V
	CondLT = 0xB // Signed less than — N != V
	CondGT = 0xC // Signed greater than — Z clear AND N == V
	CondLE = 0xD // Signed less or equal — Z set OR N != V
	CondAL = 0xE // Always (unconditional)
	CondNV = 0xF // Never (reserved — do not use)
)

// CondString returns the assembly-language suffix for a condition code.
func CondString(cond int) string {
	switch cond {
	case CondEQ:
		return "EQ"
	case CondNE:
		return "NE"
	case CondCS:
		return "CS"
	case CondCC:
		return "CC"
	case CondMI:
		return "MI"
	case CondPL:
		return "PL"
	case CondVS:
		return "VS"
	case CondVC:
		return "VC"
	case CondHI:
		return "HI"
	case CondLS:
		return "LS"
	case CondGE:
		return "GE"
	case CondLT:
		return "LT"
	case CondGT:
		return "GT"
	case CondLE:
		return "LE"
	case CondAL:
		return ""
	case CondNV:
		return "NV"
	default:
		return "??"
	}
}

// =========================================================================
// ALU Opcodes
// =========================================================================
//
// The ARM1's ALU supports 16 operations, selected by bits 24:21 of a data
// processing instruction. Four of these (TST, TEQ, CMP, CMN) only set flags
// and do not write a result to the destination register.

const (
	OpAND = 0x0 // Rd = Rn AND Op2
	OpEOR = 0x1 // Rd = Rn XOR Op2
	OpSUB = 0x2 // Rd = Rn - Op2
	OpRSB = 0x3 // Rd = Op2 - Rn
	OpADD = 0x4 // Rd = Rn + Op2
	OpADC = 0x5 // Rd = Rn + Op2 + Carry
	OpSBC = 0x6 // Rd = Rn - Op2 - NOT(Carry)
	OpRSC = 0x7 // Rd = Op2 - Rn - NOT(Carry)
	OpTST = 0x8 // Rn AND Op2, flags only
	OpTEQ = 0x9 // Rn XOR Op2, flags only
	OpCMP = 0xA // Rn - Op2, flags only
	OpCMN = 0xB // Rn + Op2, flags only
	OpORR = 0xC // Rd = Rn OR Op2
	OpMOV = 0xD // Rd = Op2
	OpBIC = 0xE // Rd = Rn AND NOT(Op2)
	OpMVN = 0xF // Rd = NOT(Op2)
)

// OpString returns the mnemonic for an ALU opcode.
func OpString(opcode int) string {
	names := [16]string{
		"AND", "EOR", "SUB", "RSB", "ADD", "ADC", "SBC", "RSC",
		"TST", "TEQ", "CMP", "CMN", "ORR", "MOV", "BIC", "MVN",
	}
	if opcode >= 0 && opcode < 16 {
		return names[opcode]
	}
	return "???"
}

// IsTestOp returns true if the ALU opcode is a test-only operation
// (TST, TEQ, CMP, CMN) that does not write to the destination register.
func IsTestOp(opcode int) bool {
	return opcode >= OpTST && opcode <= OpCMN
}

// IsLogicalOp returns true if the ALU opcode is a logical operation.
// For logical ops, the C flag comes from the barrel shifter carry-out
// rather than the ALU's adder carry.
func IsLogicalOp(opcode int) bool {
	switch opcode {
	case OpAND, OpEOR, OpTST, OpTEQ, OpORR, OpMOV, OpBIC, OpMVN:
		return true
	default:
		return false
	}
}

// =========================================================================
// Shift Types
// =========================================================================
//
// The barrel shifter supports 4 shift types, encoded in bits 6:5 of the
// operand2 field. The barrel shifter is the ARM1's most distinctive hardware
// feature — it allows one operand to be shifted or rotated FOR FREE as part
// of any data processing instruction.
//
// Example: ADD R0, R1, R2, LSL #3  means  R0 = R1 + (R2 << 3)
// The shift costs zero extra cycles.

const (
	ShiftLSL = 0 // Logical Shift Left
	ShiftLSR = 1 // Logical Shift Right
	ShiftASR = 2 // Arithmetic Shift Right (sign-extending)
	ShiftROR = 3 // Rotate Right (ROR #0 encodes RRX)
)

// ShiftString returns the mnemonic for a shift type.
func ShiftString(shiftType int) string {
	switch shiftType {
	case ShiftLSL:
		return "LSL"
	case ShiftLSR:
		return "LSR"
	case ShiftASR:
		return "ASR"
	case ShiftROR:
		return "ROR"
	default:
		return "???"
	}
}

// =========================================================================
// R15 bit positions
// =========================================================================

const (
	FlagN   = 1 << 31 // Negative flag
	FlagZ   = 1 << 30 // Zero flag
	FlagC   = 1 << 29 // Carry flag
	FlagV   = 1 << 28 // Overflow flag
	FlagI   = 1 << 27 // IRQ disable
	FlagF   = 1 << 26 // FIQ disable
	PCMask  = 0x03FFFFFC // Bits 25:2 — the 24-bit PC field, shifted to form 26-bit address
	ModeMask = 0x3 // Bits 1:0 — processor mode
)

// HaltSWI is the SWI comment field we use as a halt instruction.
// The simulator intercepts SWI with this value to stop execution.
const HaltSWI = 0x123456

// =========================================================================
// Flags
// =========================================================================

// Flags represents the ARM1's four condition flags.
type Flags struct {
	N bool // Negative — set when result's bit 31 is 1
	Z bool // Zero — set when result is 0
	C bool // Carry — set on unsigned overflow or shifter carry-out
	V bool // Overflow — set on signed overflow
}

// =========================================================================
// Trace
// =========================================================================

// Trace records the state change caused by executing one instruction.
// It captures the complete before/after snapshot for debugging and
// cross-language validation.
type Trace struct {
	Address        uint32   // PC where this instruction was fetched
	Raw            uint32   // The 32-bit instruction word
	Mnemonic       string   // Disassembled form ("ADDS R0, R1, R2, LSL #3")
	Condition      string   // Condition code suffix ("EQ", "NE", "", etc.)
	ConditionMet   bool     // Did the condition check pass?
	RegsBefore     [16]uint32
	RegsAfter      [16]uint32
	FlagsBefore    Flags
	FlagsAfter     Flags
	MemoryReads    []MemoryAccess
	MemoryWrites   []MemoryAccess
}

// MemoryAccess records a single memory read or write.
type MemoryAccess struct {
	Address uint32
	Value   uint32
}
