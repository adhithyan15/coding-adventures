// Package intel8008simulator implements the Intel 8008, the world's first 8-bit
// microprocessor (April 1972).
//
// # Historical Context
//
// The Intel 8008 was designed at the request of Computer Terminal Corporation (CTC),
// who wanted a CPU for their Datapoint 2200 terminal. Ted Hoff, Stanley Mazor, and
// Hal Feeney built it at Intel. CTC ultimately rejected it as too slow — so Intel
// sold it commercially instead. That decision launched 8-bit computing.
//
// The 8008 directly inspired the 8080 (1974), which inspired the Z80 and the 8086.
// The 8086 became the x86 architecture, still running most of the world's computers
// today. Every time you type on a modern PC, you are using the descendant of this
// 3,500-transistor chip.
//
// # Architecture Summary
//
// The 8008 is a significant evolution beyond the Intel 4004 (1971):
//
//   - 8-bit data path (vs 4-bit on 4004)
//   - 7 general-purpose registers: A (accumulator), B, C, D, E, H, L
//   - 14-bit program counter (16 KiB address space, vs 12-bit/4 KiB on 4004)
//   - 8-level internal push-down stack (vs 3-level on 4004)
//   - 4 condition flags: Carry, Zero, Sign, Parity
//   - M pseudo-register: indirect memory access via [H:L] address pair
//   - 8 input ports (IN 0-7) and 24 output ports (OUT 0-23)
//
// # Instruction Set
//
// The 8008 has 48 distinct operations with 1-, 2-, or 3-byte encodings:
//
//   Group 1: MOV D,S       01 DDD SSS  (register-to-register copy)
//            MVI D,d       00 DDD 110  (load immediate, 2-byte)
//            INR D         00 DDD 000  (increment register)
//            DCR D         00 DDD 001  (decrement register)
//   Group 2: ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP S  10 OOO SSS
//            ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI d  11 OOO 100 (2-byte)
//   Group 3: RLC/RRC/RAL/RAR         rotate accumulator
//   Group 4: JMP/JFC/JFZ/...         3-byte jumps
//   Group 5: CAL/CFC/CFZ/...         3-byte calls
//   Group 6: RET/RFC/RFZ/...         1-byte returns
//   Group 7: RST n                   1-byte call to n*8
//   Group 8: IN P / OUT P            port I/O
//   Group 9: HLT                     halt (0x76 or 0xFF)
//
// # Register Encoding
//
// All instructions use a 3-bit register field:
//
//	000 = B   001 = C   010 = D   011 = E
//	100 = H   101 = L   110 = M (memory [H:L])   111 = A
//
// # The M Pseudo-Register
//
// Register code 110 is not a physical register — it is a pointer to memory.
// The effective address is formed from H and L:
//
//	addr = ((H & 0x3F) << 8) | L      // 14-bit address, H contributes top 6 bits
//
// Instructions that read or write M actually access main memory at that address.
// This allows the 8008 to address any byte in its 16 KiB memory space with
// just two registers — a technique that persisted through the 8080 (BC, DE, HL
// pairs) and into the x86 (DS:BX, etc.).
//
// # Push-Down Stack
//
// The 8008's stack is unlike any modern CPU's stack. There is no stack pointer
// register. Instead, the chip contains 8 x 14-bit registers arranged as a
// circular buffer, where entry[0] is ALWAYS the current program counter:
//
//	CALL (pushAndJump):
//	  1. Shift all entries down: entry[7]←entry[6], ..., entry[1]←entry[0]
//	  2. Load target address into entry[0] (new PC)
//	  The old PC is now in entry[1] — the return address.
//
//	RETURN (popReturn):
//	  1. Shift all entries up: entry[0]←entry[1], ..., entry[6]←entry[7]
//	  2. entry[0] now holds the saved return address — this IS the new PC.
//
// Since entry[0] is consumed as the current PC, programs can nest at most
// 7 subroutine calls before the stack wraps silently (8th call overwrites
// the oldest return address with no error).
//
// # Parity Flag Convention
//
// P=1 means EVEN parity (even number of 1-bits in the result).
// P=0 means ODD parity.
// This is the opposite of what you might expect from "P=1 means parity triggered."
// The hardware implements: P = NOT(XOR of all result bits).
package intel8008simulator

import (
	"fmt"
)

// ─────────────────────────────────────────────────────────────────────────────
// Register constants
//
// The 8008 uses a 3-bit field in each instruction to identify registers.
// We define symbolic names to avoid magic numbers throughout the code.
// ─────────────────────────────────────────────────────────────────────────────

const (
	RegB = 0 // General purpose
	RegC = 1 // General purpose
	RegD = 2 // General purpose
	RegE = 3 // General purpose
	RegH = 4 // High byte of memory address pair [H:L]
	RegL = 5 // Low byte of memory address pair [H:L]
	RegM = 6 // Pseudo-register: memory at address [H:L]
	RegA = 7 // Accumulator — target of all ALU operations
)

// ─────────────────────────────────────────────────────────────────────────────
// Flags
//
// The 8008 maintains 4 condition flags, set by most ALU operations.
// ─────────────────────────────────────────────────────────────────────────────

// Flags holds the 4 condition flags of the Intel 8008.
//
// These are set (or cleared) after arithmetic and logical operations,
// and tested by conditional jump/call/return instructions.
type Flags struct {
	// Carry: set when an 8-bit addition overflows (result > 255),
	// or when a subtraction requires a borrow (result < 0).
	// Also set/cleared by rotate instructions.
	// Note: SUB CY=1 means borrow occurred (unlike some architectures where CY=0 after borrow).
	Carry bool

	// Zero: set when the result is exactly 0x00.
	Zero bool

	// Sign: set when bit 7 of the result is 1 (the result is negative
	// if treated as a signed two's-complement byte).
	Sign bool

	// Parity: set (true) when the result has an EVEN number of 1-bits.
	// P=1 → even parity. P=0 → odd parity.
	// Implemented in hardware as NOT(XOR of all 8 result bits).
	Parity bool
}

// ─────────────────────────────────────────────────────────────────────────────
// Trace
//
// Each Step() returns a Trace capturing complete before/after state.
// ─────────────────────────────────────────────────────────────────────────────

// Trace records one fetch-decode-execute cycle for debugging and analysis.
//
// Every instruction execution produces a Trace, capturing the PC where the
// instruction was fetched, the raw bytes, decoded mnemonic, and the complete
// before/after state of the accumulator and flags.
type Trace struct {
	// PC of the first byte of this instruction
	Address int

	// Raw instruction bytes (1, 2, or 3 bytes depending on instruction type)
	Raw []byte

	// Human-readable assembly mnemonic, e.g. "MOV A, B", "ADI 0x05", "JMP 0x0100"
	Mnemonic string

	// Accumulator value before and after execution
	ABefore int
	AAfter  int

	// Flags before and after execution
	FlagsBefore Flags
	FlagsAfter  Flags

	// MemAddress is the 14-bit memory address accessed by M-register instructions.
	// Nil if this instruction did not access memory through [H:L].
	MemAddress *int

	// MemValue is the byte read or written at MemAddress.
	// Nil if this instruction did not access memory through [H:L].
	MemValue *int
}

// ─────────────────────────────────────────────────────────────────────────────
// Simulator
// ─────────────────────────────────────────────────────────────────────────────

// Simulator models the complete Intel 8008 microprocessor.
//
// This is a standalone behavioral simulator — it does not depend on a generic
// VM framework. The 8008's variable-length instructions, 14-bit address space,
// push-down stack, and I/O model are sufficiently unique that a custom
// fetch-decode-execute loop is cleaner than adapting a generic VM.
//
// All state is held in exported-friendly fields, allowing direct inspection
// in tests. Execution is via Step() (single instruction) or Run() (full program).
type Simulator struct {
	// regs holds the 8 registers indexed by the 3-bit register field:
	// [B=0, C=1, D=2, E=3, H=4, L=5, _=6 (unused), A=7]
	// Index 6 is never directly written — it maps to the M pseudo-register.
	regs [8]int

	// memory is the 16 KiB unified address space.
	// The 8008 has no separate code/data spaces — program and data share 0x0000-0x3FFF.
	memory [16384]byte

	// stack is the 8-level push-down stack where entry[0] is always the PC.
	//
	// When a CALL executes, entries shift down (entry[N] → entry[N+1]) and
	// the target loads into entry[0]. When a RETURN executes, entries shift
	// up (entry[N+1] → entry[N]) and the old PC is restored to entry[0].
	//
	// Since entry[0] IS the PC, the maximum nesting depth is 7 (entries 1-7
	// hold return addresses). An 8th call silently overwrites the oldest entry.
	stack [8]int

	// stackDepth tracks how many return addresses are currently saved (0-7).
	// This is for diagnostic purposes — the real hardware has no depth counter.
	stackDepth int

	// flags holds the 4 condition flags.
	flags Flags

	// halted is true after executing HLT (0x76 or 0xFF).
	// A halted CPU cannot execute further instructions.
	halted bool

	// inputPorts holds values for the 8 input ports (IN 0-7).
	// Programs read these via IN instructions. Set externally before execution.
	inputPorts [8]int

	// outputPorts holds values written by OUT instructions.
	// The 8008 has 24 output ports (0-23). Read externally after execution.
	outputPorts [24]int
}

// New creates a new, reset Intel 8008 simulator.
//
// All registers are 0, PC is 0, memory is zeroed, flags are clear, halted=false.
func New() *Simulator {
	return &Simulator{}
}

// ─────────────────────────────────────────────────────────────────────────────
// Accessor methods — expose internal state for tests and external code
// ─────────────────────────────────────────────────────────────────────────────

// A returns the accumulator value (register A = index 7).
func (s *Simulator) A() int { return s.regs[RegA] }

// B returns register B.
func (s *Simulator) B() int { return s.regs[RegB] }

// C returns register C.
func (s *Simulator) C() int { return s.regs[RegC] }

// D returns register D.
func (s *Simulator) D() int { return s.regs[RegD] }

// E returns register E.
func (s *Simulator) E() int { return s.regs[RegE] }

// H returns register H (high byte of address pair).
func (s *Simulator) H() int { return s.regs[RegH] }

// L returns register L (low byte of address pair).
func (s *Simulator) L() int { return s.regs[RegL] }

// PC returns the current program counter (14-bit, 0-16383).
func (s *Simulator) PC() int { return s.stack[0] }

// Flags returns a copy of the current condition flags.
func (s *Simulator) GetFlags() Flags { return s.flags }

// Halted returns true if the CPU has executed a HLT instruction.
func (s *Simulator) Halted() bool { return s.halted }

// HLAddress returns the 14-bit memory address formed by H and L.
//
// The formula: addr = ((H & 0x3F) << 8) | L
// Only the lower 6 bits of H are used for addressing (H bits 7-6 are "don't care").
func (s *Simulator) HLAddress() int {
	return ((s.regs[RegH] & 0x3F) << 8) | s.regs[RegL]
}

// Memory returns a slice of the full 16 KiB memory.
func (s *Simulator) Memory() []byte { return s.memory[:] }

// Stack returns the current stack contents (up to 7 saved return addresses).
func (s *Simulator) Stack() []int {
	depth := s.stackDepth
	if depth > 7 {
		depth = 7
	}
	result := make([]int, depth)
	for i := 0; i < depth; i++ {
		result[i] = s.stack[i+1]
	}
	return result
}

// StackDepth returns the number of saved return addresses (0-7).
func (s *Simulator) StackDepth() int { return s.stackDepth }

// SetInputPort sets the value of an input port (port 0-7).
// Programs read this value via the IN instruction.
func (s *Simulator) SetInputPort(port, value int) {
	if port < 0 || port > 7 {
		panic(fmt.Sprintf("intel8008: input port must be 0-7, got %d", port))
	}
	s.inputPorts[port] = value & 0xFF
}

// GetOutputPort returns the value written to an output port (port 0-23).
func (s *Simulator) GetOutputPort(port int) int {
	if port < 0 || port > 23 {
		panic(fmt.Sprintf("intel8008: output port must be 0-23, got %d", port))
	}
	return s.outputPorts[port]
}

// ─────────────────────────────────────────────────────────────────────────────
// Program loading
// ─────────────────────────────────────────────────────────────────────────────

// LoadProgram copies program bytes into memory starting at startAddress.
//
// Resets the PC to startAddress and clears the halted flag. Does NOT clear
// memory outside the program area — useful for pre-loading data.
func (s *Simulator) LoadProgram(program []byte, startAddress int) {
	for i, b := range program {
		addr := startAddress + i
		if addr < len(s.memory) {
			s.memory[addr] = b
		}
	}
	s.stack[0] = startAddress & 0x3FFF
	s.halted = false
}

// Reset clears all CPU state to power-on defaults.
//
// Zeroes all registers, memory, stack, flags, I/O ports.
func (s *Simulator) Reset() {
	*s = Simulator{}
}

// ─────────────────────────────────────────────────────────────────────────────
// Stack mechanics
//
// The 8008's push-down stack is unusual. Entry[0] is always the PC.
// CALL shifts everything down; RETURN shifts everything up.
// ─────────────────────────────────────────────────────────────────────────────

// pushAndJump saves the current PC (next instruction after the call) and
// jumps to target.
//
// The "push" is: rotate stack entries down (each entry moves to the slot below),
// then load target into entry[0] (the new PC).
//
// Before: [PC, ret1, ret2, ret3, ret4, ret5, ret6, ret7]
// After:  [target, PC, ret1, ret2, ret3, ret4, ret5, ret6]
//         ← entry[7] (ret7) is overwritten if the stack was full.
func (s *Simulator) pushAndJump(target int) {
	// Rotate entries down: entry[N] ← entry[N-1], starting from the bottom
	// This makes room at entry[0] for the new PC.
	for i := 7; i > 0; i-- {
		s.stack[i] = s.stack[i-1]
	}
	// Load target into entry[0] — this IS the new program counter.
	s.stack[0] = target & 0x3FFF
	// Track depth (max 7 return addresses can be saved before wrapping)
	if s.stackDepth < 7 {
		s.stackDepth++
	}
}

// popReturn restores the most recently saved return address.
//
// The "pop" is: rotate stack entries up (each entry moves to the slot above),
// which places the saved return address into entry[0] (the PC).
//
// Before: [curPC, ret1, ret2, ret3, ...]
// After:  [ret1, ret2, ret3, ...]
//         ← curPC is discarded.
func (s *Simulator) popReturn() {
	// Rotate entries up: entry[N-1] ← entry[N]
	for i := 0; i < 7; i++ {
		s.stack[i] = s.stack[i+1]
	}
	if s.stackDepth > 0 {
		s.stackDepth--
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Register read/write (handles M pseudo-register transparently)
// ─────────────────────────────────────────────────────────────────────────────

// readReg reads a register by its 3-bit index.
//
// For index 6 (M), reads memory at [(H & 0x3F) << 8 | L].
// For all other indices, reads the register directly.
func (s *Simulator) readReg(reg int) (int, *int) {
	if reg == RegM {
		addr := s.HLAddress()
		val := int(s.memory[addr])
		return val, &addr
	}
	return s.regs[reg], nil
}

// writeReg writes a value to a register by its 3-bit index.
//
// For index 6 (M), writes memory at [(H & 0x3F) << 8 | L].
// For all other indices, writes the register directly.
// All values are masked to 8 bits.
func (s *Simulator) writeReg(reg, value int) *int {
	value = value & 0xFF
	if reg == RegM {
		addr := s.HLAddress()
		s.memory[addr] = byte(value)
		return &addr
	}
	s.regs[reg] = value
	return nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Flag computation
// ─────────────────────────────────────────────────────────────────────────────

// computeZSP computes Zero, Sign, and Parity flags from an 8-bit result.
//
// Zero:   result == 0
// Sign:   bit 7 of result is 1 (MSB set → negative in two's complement)
// Parity: NOT(XOR of all 8 bits) — P=1 means even number of 1-bits
//
// These three flags are always computed together from the ALU output.
// Carry is computed separately because it depends on the operation type.
func computeZSP(result int) (zero, sign, parity bool) {
	result = result & 0xFF
	zero = result == 0
	sign = (result >> 7) & 1 == 1

	// Parity: XOR all 8 bits, then NOT.
	// XOR chain = 1 when odd number of 1-bits.
	// NOT(XOR chain) = 1 when even number of 1-bits.
	// This is exactly the Intel 8008 Parity flag: P=1 means even parity.
	xorChain := 0
	for i := 0; i < 8; i++ {
		xorChain ^= (result >> i) & 1
	}
	parity = xorChain == 0 // NOT(xorChain): 0 → even parity → P=true

	return
}

// setFlagsFromResult updates all four flags given an ALU result and carry.
func (s *Simulator) setFlagsFromResult(result int, carry bool) {
	z, si, p := computeZSP(result)
	s.flags.Zero = z
	s.flags.Sign = si
	s.flags.Parity = p
	s.flags.Carry = carry
}

// setFlagsNoCarry updates Z, S, P flags but leaves Carry unchanged.
// Used by INR and DCR which do not affect the Carry flag.
func (s *Simulator) setFlagsNoCarry(result int) {
	z, si, p := computeZSP(result)
	s.flags.Zero = z
	s.flags.Sign = si
	s.flags.Parity = p
	// s.flags.Carry is intentionally unchanged
}

// ─────────────────────────────────────────────────────────────────────────────
// ALU operations
//
// All arithmetic routes through these functions, which compute the 8-bit
// result and all relevant flags. The real 8008 used a ripple-carry adder
// built from full adders (gate-level detail is in the companion package
// intel8008-gatelevel).
// ─────────────────────────────────────────────────────────────────────────────

// add8 computes a + b + carryIn (8-bit), returns (result, carryOut).
func add8(a, b, carryIn int) (int, bool) {
	sum := a + b + carryIn
	return sum & 0xFF, sum > 255
}

// sub8 computes a - b - borrowIn (8-bit), returns (result, borrowOut).
//
// Subtraction on the 8008 uses two's complement: A - B = A + (~B) + 1.
// CY=1 after SUB means a borrow occurred (result was negative).
// CY=0 means no borrow (unsigned A >= B).
//
// Note: this is the OPPOSITE of how some architectures define carry after
// subtraction. On the 8008, borrow is signaled by CY=1 (not CY=0).
func sub8(a, b, borrowIn int) (int, bool) {
	// Two's complement subtraction: a - b - borrow = a + (~b) + (1 - borrow)
	// Using unsigned 9-bit arithmetic to detect underflow:
	diff := a - b - borrowIn
	// CY=1 means borrow (result < 0 before masking)
	carry := diff < 0
	return diff & 0xFF, carry
}

// ─────────────────────────────────────────────────────────────────────────────
// ALU operation dispatch (for register and immediate ALU instructions)
// ─────────────────────────────────────────────────────────────────────────────

// aluOp performs one of the 8 ALU operations on A and operand, updates flags.
//
// Operations (indexed by 3-bit OOO field from the opcode):
//
//	0=ADD, 1=ADC, 2=SUB, 3=SBB, 4=ANA, 5=XRA, 6=ORA, 7=CMP
//
// For CMP, A is unchanged; only flags are updated.
// For ANA/XRA/ORA, Carry is always cleared.
//
// Returns the A value after the operation (unchanged for CMP).
func (s *Simulator) aluOp(op int, operand int) int {
	a := s.regs[RegA]
	var result int
	var carry bool

	switch op {
	case 0: // ADD: A ← A + operand
		result, carry = add8(a, operand, 0)
	case 1: // ADC: A ← A + operand + CY
		cin := 0
		if s.flags.Carry {
			cin = 1
		}
		result, carry = add8(a, operand, cin)
	case 2: // SUB: A ← A - operand
		result, carry = sub8(a, operand, 0)
	case 3: // SBB: A ← A - operand - CY
		bin := 0
		if s.flags.Carry {
			bin = 1
		}
		result, carry = sub8(a, operand, bin)
	case 4: // ANA: A ← A & operand, CY=0
		result = a & operand
		carry = false
	case 5: // XRA: A ← A ^ operand, CY=0
		result = a ^ operand
		carry = false
	case 6: // ORA: A ← A | operand, CY=0
		result = a | operand
		carry = false
	case 7: // CMP: set flags for A - operand, A unchanged
		result, carry = sub8(a, operand, 0)
		s.setFlagsFromResult(result, carry)
		return a // A is NOT written for CMP
	default:
		panic(fmt.Sprintf("intel8008: unknown ALU op %d", op))
	}

	s.setFlagsFromResult(result, carry)
	s.regs[RegA] = result
	return result
}

// aluOpName returns a human-readable name for an ALU operation code.
func aluOpName(op int) string {
	names := []string{"ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"}
	if op < len(names) {
		return names[op]
	}
	return "???"
}

// aluImmOpName returns the immediate form name for an ALU operation code.
func aluImmOpName(op int) string {
	names := []string{"ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"}
	if op < len(names) {
		return names[op]
	}
	return "???"
}

// regName returns the assembly name for a 3-bit register code.
func regName(reg int) string {
	names := []string{"B", "C", "D", "E", "H", "L", "M", "A"}
	if reg < len(names) {
		return names[reg]
	}
	return "?"
}

// ─────────────────────────────────────────────────────────────────────────────
// Instruction length detection
// ─────────────────────────────────────────────────────────────────────────────

// instrLen returns the number of bytes in an instruction given its opcode.
//
// The 8008 has three instruction lengths:
//
//	1 byte: MOV, ALU register, rotates, INR, DCR, RET, RST, IN, OUT, HLT
//	2 bytes: MVI (00 DDD 110) and ALU-immediate (11 OOO 100)
//	3 bytes: JMP/conditional jumps and CALL/conditional calls
//
// The group 01 space is the trickiest because MOV, IN, JMP, CAL, and HLT
// all share the same top-2-bit pattern. The discrimination uses specific
// opcode ranges:
//
//	JMP opcodes: 0x40, 0x44, 0x48, 0x4C, 0x50, 0x54, 0x58, 0x5C, 0x7C
//	CAL opcodes: 0x42, 0x46, 0x4A, 0x4E, 0x52, 0x56, 0x5A, 0x5E, 0x7E
//	IN opcodes:  0x41, 0x49, 0x51, 0x59, 0x61, 0x69, 0x71, 0x79
//	Everything else in group 01 is 1-byte (MOV or HLT)
func instrLen(opcode byte) int {
	// Check specific 3-byte opcodes first (jump and call families)
	switch opcode {
	// Unconditional
	case 0x7C, // JMP
		0x7E: // CAL
		return 3
	// Conditional jumps: 01 CCC T00 where CCC ∈ {0,1,2,3} and T ∈ {0,1}
	// These are in range 0x40-0x5F, specifically when SSS = T00 (0 or 4)
	// and DDD=CCC < 4.
	// JFC=0x40, JTC=0x44, JFZ=0x48, JTZ=0x4C, JFS=0x50, JTS=0x54, JFP=0x58, JTP=0x5C
	case 0x40, 0x44, 0x48, 0x4C, 0x50, 0x54, 0x58, 0x5C:
		return 3
	// Conditional calls: 01 CCC T10 where CCC ∈ {0,1,2,3}
	// CFC=0x42, CTC=0x46, CFZ=0x4A, CTZ=0x4E, CFS=0x52, CTS=0x56, CFP=0x5A, CTP=0x5E
	case 0x42, 0x46, 0x4A, 0x4E, 0x52, 0x56, 0x5A, 0x5E:
		return 3
	}

	group := (opcode >> 6) & 0x03
	sss := opcode & 0x07

	switch group {
	case 0x00:
		// 00 DDD 110 → MVI (2-byte)
		if sss == 0x06 {
			return 2
		}
		return 1
	case 0x01:
		// All remaining group-01 opcodes are 1-byte (MOV, IN, HLT)
		return 1
	case 0x02:
		// 10 OOO SSS → ALU register (1-byte)
		return 1
	case 0x03:
		// 11 OOO 100 → ALU immediate (2-byte)
		if sss == 0x04 {
			return 2
		}
		return 1
	}
	return 1
}

// ─────────────────────────────────────────────────────────────────────────────
// Conditional evaluation
//
// Conditional instructions (JFC, JTZ, RFC, etc.) test one of 4 flags.
// CCC=condition code, T=sense (0=if-false, 1=if-true).
// ─────────────────────────────────────────────────────────────────────────────

// evalCondition tests the condition code CCC with sense T.
//
//	CCC: 0=CY, 1=Z, 2=S, 3=P
//	T:   0=if-false (flag=0), 1=if-true (flag=1)
//
// Returns true if the condition is satisfied (branch should be taken).
func (s *Simulator) evalCondition(ccc, t int) bool {
	var flagValue bool
	switch ccc {
	case 0:
		flagValue = s.flags.Carry
	case 1:
		flagValue = s.flags.Zero
	case 2:
		flagValue = s.flags.Sign
	case 3:
		flagValue = s.flags.Parity
	default:
		// CCC >= 4 is reserved; treat as always-false
		return false
	}

	if t == 1 {
		return flagValue // JTC: take if carry IS set
	}
	return !flagValue // JFC: take if carry is NOT set
}

// ─────────────────────────────────────────────────────────────────────────────
// Fetch, Decode, Execute
// ─────────────────────────────────────────────────────────────────────────────

// Step performs one fetch-decode-execute cycle, returning a Trace.
//
// The execution model:
//
//  1. FETCH:  Read opcode byte at memory[PC]; advance PC by 1.
//  2. FETCH2: For 2-byte instructions, read the data byte; PC += 1.
//  3. FETCH3: For 3-byte instructions, read addr_lo and addr_hi; PC += 2.
//  4. DECODE: Extract group (bits 7-6), DDD (bits 5-3), SSS (bits 2-0).
//  5. EXECUTE: Dispatch by (group, DDD, SSS) to the appropriate handler.
//
// Returns a Trace capturing the complete before/after state.
// Panics if the CPU is already halted.
func (s *Simulator) Step() Trace {
	if s.halted {
		panic("intel8008: CPU is halted — cannot Step()")
	}

	addr := s.stack[0]
	opcode := s.memory[addr]

	// --- Fetch instruction bytes ---
	length := instrLen(opcode)
	raw := make([]byte, length)
	raw[0] = opcode
	s.stack[0] = (addr + 1) & 0x3FFF

	var imm8 int   // immediate data byte (for 2-byte instructions)
	var addrLo int // address low byte (for 3-byte instructions)
	var addrHi int // address high byte (for 3-byte instructions)

	if length >= 2 {
		imm8 = int(s.memory[s.stack[0]])
		raw[1] = byte(imm8)
		s.stack[0] = (s.stack[0] + 1) & 0x3FFF
	}
	if length == 3 {
		addrLo = imm8
		addrHi = int(s.memory[s.stack[0]])
		raw[1] = byte(addrLo)
		raw[2] = byte(addrHi)
		s.stack[0] = (s.stack[0] + 1) & 0x3FFF
	}

	// Snapshot state before execution
	aBefore := s.regs[RegA]
	flagsBefore := s.flags

	// Decode opcode fields
	group := (int(opcode) >> 6) & 0x03 // bits 7-6: major instruction group
	ddd := (int(opcode) >> 3) & 0x07   // bits 5-3: destination reg or ALU op
	sss := int(opcode) & 0x07          // bits 2-0: source reg or sub-op

	var mnemonic string
	var memAddr *int
	var memVal *int

	// --- Execute ---
	switch group {
	case 0x00:
		// ─── Group 00 ──────────────────────────────────────────────────────────
		// Opcodes: 00 DDD SSS
		// SSS distinguishes: 000=INR, 001=DCR, 010=rotate, 110=MVI, 101=RST, 011/111=RET
		switch sss {
		case 0x00: // INR D: 00 DDD 000
			// Increment register DDD. Does NOT affect Carry.
			if ddd == RegM {
				a := s.HLAddress()
				val := (int(s.memory[a]) + 1) & 0xFF
				s.memory[a] = byte(val)
				s.setFlagsNoCarry(val)
				mnemonic = "INR M"
				memAddr = &a
				v := val
				memVal = &v
			} else {
				s.regs[ddd] = (s.regs[ddd] + 1) & 0xFF
				s.setFlagsNoCarry(s.regs[ddd])
				mnemonic = fmt.Sprintf("INR %s", regName(ddd))
			}

		case 0x01: // DCR D: 00 DDD 001
			// Decrement register DDD. Does NOT affect Carry.
			if ddd == RegM {
				a := s.HLAddress()
				val := (int(s.memory[a]) - 1 + 256) & 0xFF
				s.memory[a] = byte(val)
				s.setFlagsNoCarry(val)
				mnemonic = "DCR M"
				memAddr = &a
				v := val
				memVal = &v
			} else {
				s.regs[ddd] = (s.regs[ddd] - 1 + 256) & 0xFF
				s.setFlagsNoCarry(s.regs[ddd])
				mnemonic = fmt.Sprintf("DCR %s", regName(ddd))
			}

		case 0x02: // Group 00, sss=010: Rotates OR OUT instruction
			// Rotates are exactly 4 fixed opcodes: 0x02, 0x0A, 0x12, 0x1A.
			// These have DDD = 0, 1, 2, 3 (DDD[2]=0, i.e., DDD < 4).
			// All other group-00/sss-010 opcodes are OUT instructions.
			//
			// OUT P: 00 PPP P10 — port number in bits [4:1] of opcode byte.
			//   port = (opcode >> 1) & 0x0F   (4-bit field, ports 0-15)
			//   Extended ports 0-23 use the full 5-bit field bits [5:1]:
			//   port = (opcode >> 1) & 0x1F   (but only 0-23 are valid)
			//
			// Rotates use DDD < 4 (bits [4:3] < 4, meaning bit 5 = 0 AND DDD < 4).
			if ddd < 4 {
				// Rotate accumulator: 00 0RR 010
				// The rotate type is the ddd field (0-3 = RLC/RRC/RAL/RAR).
				a := s.regs[RegA]
				switch ddd {
				case 0: // RLC: 0x02 — CY←A[7]; A[0]←A[7]
					bit7 := (a >> 7) & 1
					s.regs[RegA] = ((a << 1) | bit7) & 0xFF
					s.flags.Carry = bit7 == 1
					mnemonic = "RLC"
				case 1: // RRC: 0x0A — CY←A[0]; A[7]←A[0]
					bit0 := a & 1
					s.regs[RegA] = ((a >> 1) | (bit0 << 7)) & 0xFF
					s.flags.Carry = bit0 == 1
					mnemonic = "RRC"
				case 2: // RAL: 0x12 — new_CY←A[7]; A[0]←old_CY
					bit7 := (a >> 7) & 1
					oldCY := 0
					if s.flags.Carry {
						oldCY = 1
					}
					s.regs[RegA] = ((a << 1) | oldCY) & 0xFF
					s.flags.Carry = bit7 == 1
					mnemonic = "RAL"
				case 3: // RAR: 0x1A — new_CY←A[0]; A[7]←old_CY
					bit0 := a & 1
					oldCY := 0
					if s.flags.Carry {
						oldCY = 1
					}
					s.regs[RegA] = ((oldCY << 7) | (a >> 1)) & 0xFF
					s.flags.Carry = bit0 == 1
					mnemonic = "RAR"
				}
			} else {
				// OUT P: write A to output port.
				// Port number is in bits [5:1] (5-bit field for 24 ports).
				port := (int(opcode) >> 1) & 0x1F
				if port < 24 {
					s.outputPorts[port] = s.regs[RegA]
					mnemonic = fmt.Sprintf("OUT %d", port)
				} else {
					mnemonic = fmt.Sprintf("UNKNOWN(0x%02X)", opcode)
				}
			}

		case 0x05: // RST n: 00 AAA 101 — 1-byte CALL to address n*8
			// The 3-bit AAA field gives n. Target = n * 8 = n << 3.
			// This is identical to CAL target but encoded in a single byte,
			// saving 2 bytes. Used for fast interrupt handlers at fixed addresses.
			n := ddd
			target := n * 8
			// pushAndJump saves current PC (already advanced past opcode)
			// and jumps to target.
			s.pushAndJump(target)
			mnemonic = fmt.Sprintf("RST %d", n)

		case 0x03, 0x07: // Return instructions: 00 CCC T11
			// Bits [2:0] of opcode are 011 or 111 (sss = 3 or 7).
			// But wait: sss encodes bits [2:0], so:
			//   sss=3 → bits 2-0 = 011 → T=0 (return if false)
			//   sss=7 → bits 2-0 = 111 → T=1 (return if true)
			// CCC is encoded in bits [5:3] = ddd field.
			// Unconditional RET: 00 111 111 = 0x3F (ddd=7, sss=7)
			ccc := ddd
			t := (sss >> 2) & 1 // bit 2 of sss

			// Unconditional return: opcode 0x3F (ddd=111, sss=111)
			if ddd == 7 && sss == 7 {
				s.popReturn()
				mnemonic = "RET"
			} else {
				// Conditional return
				if s.evalCondition(ccc, t) {
					s.popReturn()
				}
				condNames := [][]string{
					{"RFC", "RTC"}, {"RFZ", "RTZ"}, {"RFS", "RTS"}, {"RFP", "RTP"},
				}
				if ccc < 4 {
					mnemonic = condNames[ccc][t]
				} else {
					mnemonic = fmt.Sprintf("RET cond%d%d", ccc, t)
				}
			}

		case 0x06: // MVI D, d: 00 DDD 110, data8 — load immediate into register
			// The immediate byte was fetched as imm8.
			if ddd == RegM {
				a := s.HLAddress()
				s.memory[a] = byte(imm8)
				mnemonic = fmt.Sprintf("MVI M, 0x%02X", imm8)
				memAddr = &a
				v := imm8
				memVal = &v
			} else {
				s.regs[ddd] = imm8
				mnemonic = fmt.Sprintf("MVI %s, 0x%02X", regName(ddd), imm8)
			}

		default:
			mnemonic = fmt.Sprintf("UNKNOWN(0x%02X)", opcode)
		}

	case 0x01:
		// ─── Group 01 ──────────────────────────────────────────────────────────
		// This group contains a mix of instruction types. They are identified by
		// specific opcode values, not by a simple bit-field rule.
		//
		// Instruction identification priority:
		//  1. Specific opcode 0x76 → HLT
		//  2. Specific opcode 0x7C → JMP (unconditional, 3-byte already fetched)
		//  3. Specific opcode 0x7E → CAL (unconditional, 3-byte already fetched)
		//  4. Opcodes 0x40,0x44,0x48,0x4C,0x50,0x54,0x58,0x5C → conditional JMP (3-byte)
		//  5. Opcodes 0x42,0x46,0x4A,0x4E,0x52,0x56,0x5A,0x5E → conditional CAL (3-byte)
		//  6. SSS = 001 → IN instruction
		//  7. Everything else → MOV D, S
		//
		// Note: the 3-byte jump/call instructions have already had their address
		// bytes fetched (addrLo and addrHi) because instrLen returned 3 for them.

		switch int(opcode) {
		case 0x76: // HLT — MOV M, M is the designated HALT opcode
			// The Intel 8008 designers chose to use the otherwise-meaningless
			// "copy M to M" as the halt encoding. It's elegant: no separate
			// HALT circuitry needed — the decoder just recognizes this pattern.
			s.halted = true
			mnemonic = "HLT"

		case 0x7C: // JMP unconditional — 3-byte, address already fetched
			target := ((addrHi & 0x3F) << 8) | addrLo
			s.stack[0] = target
			mnemonic = fmt.Sprintf("JMP 0x%04X", target)

		case 0x7E: // CAL unconditional — 3-byte, address already fetched
			target := ((addrHi & 0x3F) << 8) | addrLo
			s.pushAndJump(target)
			mnemonic = fmt.Sprintf("CAL 0x%04X", target)

		case 0x40, 0x44, 0x48, 0x4C, 0x50, 0x54, 0x58, 0x5C: // Conditional JMP — 3-byte
			// Encoding: 01 CCC T00 where CCC ∈ {0,1,2,3} and T ∈ {0,1}
			// CCC = bits [5:3] = ddd field; T = bit 2 of opcode
			target := ((addrHi & 0x3F) << 8) | addrLo
			ccc := ddd
			t := (int(opcode) >> 2) & 1
			if s.evalCondition(ccc, t) {
				s.stack[0] = target
			}
			condJumpNames := [][]string{
				{"JFC", "JTC"}, {"JFZ", "JTZ"}, {"JFS", "JTS"}, {"JFP", "JTP"},
			}
			mnemonic = fmt.Sprintf("%s 0x%04X", condJumpNames[ccc][t], target)

		case 0x42, 0x46, 0x4A, 0x4E, 0x52, 0x56, 0x5A, 0x5E: // Conditional CAL — 3-byte
			// Encoding: 01 CCC T10 where CCC ∈ {0,1,2,3} and T ∈ {0,1}
			target := ((addrHi & 0x3F) << 8) | addrLo
			ccc := ddd
			t := (int(opcode) >> 2) & 1
			if s.evalCondition(ccc, t) {
				s.pushAndJump(target)
			}
			condCallNames := [][]string{
				{"CFC", "CTC"}, {"CFZ", "CTZ"}, {"CFS", "CTS"}, {"CFP", "CTP"},
			}
			mnemonic = fmt.Sprintf("%s 0x%04X", condCallNames[ccc][t], target)

		default: // 1-byte: IN, or MOV, or conditional jump/call
			// The remaining disambiguation for 1-byte vs 3-byte is done by instrLen;
			// by the time we get here, the opcode was classified as 1-byte.
			// However, we also need to handle the overlap: conditional jumps/calls
			// where ddd < 4 AND sss & 3 == 0 or 2. instrLen handles these as 3-byte.
			// This default branch only sees 1-byte opcodes that instrLen classified as 1.
			//
			// The remaining 1-byte cases are:
			//   - sss == 1: IN instruction (takes over SSS=C/E/L/A slots in group 01)
			//   - ddd >= 4 or sss & 3 == 3: MOV
			//   - ddd < 4 and sss == 5 or sss == 7: MOV (SSS=L or SSS=A with DDD a cond code)
			//
			// Note: The 8008 encoding reuses some MOV opcode slots for IN, JMP, CAL.
			// So MOV with source=C (sss=1) is not available — those opcodes are IN.
			// Similarly MOV with source=M (sss=6) when ddd≤3 is not available — CAL.

			if sss == 1 {
				// IN P: 01 PPP 001 — port number in DDD field (bits [5:3])
				// The port field (DDD) can be 0-7.
				// Note: opcode 0x79 (IN 7) conflicts with what would be MOV A, C.
				// The real 8008 resolves this by making sss=001 always mean IN.
				port := ddd
				s.regs[RegA] = s.inputPorts[port] & 0xFF
				mnemonic = fmt.Sprintf("IN %d", port)
			} else if ddd <= 3 && (sss&3) == 0 {
				// Would be conditional JMP but instrLen should have returned 3.
				// If we're here, something went wrong. Treat as MOV for safety.
				val, ma := s.readReg(sss)
				wa := s.writeReg(ddd, val)
				if ma != nil {
					memAddr = ma; v := val; memVal = &v
				} else if wa != nil {
					memAddr = wa; v := val; memVal = &v
				}
				mnemonic = fmt.Sprintf("MOV %s, %s", regName(ddd), regName(sss))
			} else if ddd <= 3 && (sss&3) == 2 {
				// Would be conditional CAL but instrLen should have returned 3.
				// If we're here, treat as MOV for safety.
				val, ma := s.readReg(sss)
				wa := s.writeReg(ddd, val)
				if ma != nil {
					memAddr = ma; v := val; memVal = &v
				} else if wa != nil {
					memAddr = wa; v := val; memVal = &v
				}
				mnemonic = fmt.Sprintf("MOV %s, %s", regName(ddd), regName(sss))
			} else {
				// MOV D, S: register-to-register copy
				// Valid when: ddd >= 4, OR sss & 3 == 3 (SSS=E/A), OR other combos
				// that don't conflict with IN/JMP/CAL.
				val, ma := s.readReg(sss)
				wa := s.writeReg(ddd, val)
				if ma != nil {
					memAddr = ma
					v := val
					memVal = &v
				} else if wa != nil {
					memAddr = wa
					v := val
					memVal = &v
				}
				mnemonic = fmt.Sprintf("MOV %s, %s", regName(ddd), regName(sss))
			}
		}

	case 0x02:
		// ─── Group 10 ──────────────────────────────────────────────────────────
		// ALU register instructions: 10 OOO SSS
		// DDD field = ALU operation (OOO); SSS = source register.
		//
		// All 8 operations: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP.
		aluOp := ddd // bits [5:3] = ALU operation selector
		src := sss   // bits [2:0] = source register
		operand, ma := s.readReg(src)
		if ma != nil {
			memAddr = ma
			v := operand
			memVal = &v
		}
		s.aluOp(aluOp, operand)
		mnemonic = fmt.Sprintf("%s %s", aluOpName(aluOp), regName(src))

	case 0x03:
		// ─── Group 11 ──────────────────────────────────────────────────────────
		// Group 11 (bits [7:6] = 11) covers two instruction families:
		//   - ALU immediate: 11 OOO 100, data8  (sss=100=4, 2-byte)
		//   - HLT: 0xFF (11 111 111)
		//
		// Note: return instructions are group 00 (00 CCC T11), not group 11.
		if opcode == 0xFF {
			// 0xFF = 11 111 111 — the second HALT encoding.
			// The first is 0x76 (MOV M,M in group 01).
			s.halted = true
			mnemonic = "HLT"
		} else if sss == 0x04 {
			// ALU immediate: 11 OOO 100, data8
			// OOO = ddd field: 0=ADI, 1=ACI, 2=SUI, 3=SBI, 4=ANI, 5=XRI, 6=ORI, 7=CPI
			aluOp := ddd
			s.aluOp(aluOp, imm8)
			mnemonic = fmt.Sprintf("%s 0x%02X", aluImmOpName(aluOp), imm8)
		} else {
			mnemonic = fmt.Sprintf("UNKNOWN(0x%02X)", opcode)
		}
	}

	if mnemonic == "" {
		mnemonic = fmt.Sprintf("UNKNOWN(0x%02X)", opcode)
	}

	return Trace{
		Address:     addr,
		Raw:         raw,
		Mnemonic:    mnemonic,
		ABefore:     aBefore,
		AAfter:      s.regs[RegA],
		FlagsBefore: flagsBefore,
		FlagsAfter:  s.flags,
		MemAddress:  memAddr,
		MemValue:    memVal,
	}
}

// Run loads a program and executes it until HLT or maxSteps instructions.
//
// The program is loaded at address 0. maxSteps prevents infinite loops during
// testing. Returns the list of Trace records produced by each Step().
func (s *Simulator) Run(program []byte, maxSteps int) []Trace {
	s.Reset()
	s.LoadProgram(program, 0)

	var traces []Trace
	for !s.halted && len(traces) < maxSteps {
		t := s.Step()
		traces = append(traces, t)
	}
	return traces
}
