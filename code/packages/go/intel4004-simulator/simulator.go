// Package intel4004simulator implements the first commercial processor — the Intel 4004.
//
// === What is the Intel 4004? ===
//
// The Intel 4004 was the world's first commercial single-chip microprocessor,
// released by Intel in 1971. Designed by Federico Faggin, Ted Hoff, and Stanley
// Mazor for the Busicom 141-PF calculator, it contained just 2,300 transistors.
// For perspective, a modern CPU has billions. The 4004 ran at 740 kHz — about a
// million times slower than today's processors. Yet it proved that a general-purpose
// processor could be built on a single chip, launching the microprocessor revolution.
//
// === Why 4-bit? ===
//
// The 4004 is natively 4-bit. Every data value is 4 bits wide (0-15).
// All computations are forcefully masked to 4 bits (& 0xF). There is no native
// support for 8-bit, 16-bit, or 32-bit math anywhere in the data path. This was
// perfect for BCD calculator arithmetic, where each digit (0-9) fits in 4 bits.
//
// === Accumulator Architecture ===
//
// Unlike modern register-to-register architectures (RISC-V, ARM), the 4004 funnels
// all computation through a single Accumulator register (A). To add two values,
// you must: load A with value1, swap it out to a register, load A with value2,
// then add the register back into A. This requires more instructions but simplifies
// the hardware enormously — critical when you only have 2,300 transistors.
//
// === Complete Instruction Set (46 instructions) ===
//
//	0x00       NOP          No operation
//	0x01       HLT          Halt (simulator-only, not on real hardware)
//	0x1_       JCN c,a  *   Conditional jump (c=condition nibble)
//	0x2_ even  FIM Pp,d *   Fetch immediate to register pair
//	0x2_ odd   SRC Pp       Send register control (set RAM/ROM address)
//	0x3_ even  FIN Pp       Fetch indirect from ROM via pair P0
//	0x3_ odd   JIN Pp       Jump indirect via register pair
//	0x4_       JUN a    *   Unconditional jump (12-bit address)
//	0x5_       JMS a    *   Jump to subroutine (push return addr)
//	0x6_       INC Rn       Increment register
//	0x7_       ISZ Rn,a *   Increment register, jump if not zero
//	0x8_       ADD Rn       Add register to accumulator with carry
//	0x9_       SUB Rn       Subtract register from accumulator with borrow
//	0xA_       LD Rn        Load register into accumulator
//	0xB_       XCH Rn       Exchange accumulator and register
//	0xC_       BBL n        Branch back (return) and load immediate
//	0xD_       LDM n        Load immediate into accumulator
//	0xE0-0xEF  I/O ops      RAM/ROM read/write operations
//	0xF0-0xFD  Accum ops    Accumulator manipulation
//
//	* = 2-byte instruction (second byte is data or address)
//
// === Memory Layout ===
//
// The 4004 has a unique memory architecture reflecting its calculator origins:
//
//   - ROM: 4096 bytes of program storage (read-only in real hardware)
//   - RAM: organized as 4 banks x 4 registers x (16 main + 4 status) nibbles
//     - Main characters (16 per register): general-purpose data storage
//     - Status characters (4 per register): flag/state storage
//     - Output ports (1 per bank): written by WMP for external I/O
//   - The SRC instruction sets which RAM register/character to access
//   - The DCL instruction selects which RAM bank is active
//
// === Hardware Call Stack ===
//
// The 4004 has a 3-level hardware call stack for subroutine calls. Unlike modern
// processors that use RAM for the stack, the 4004's stack is built from dedicated
// registers inside the chip. This means:
//   - Maximum call depth is 3 (no recursion beyond 3 levels)
//   - Stack overflow wraps silently (the 4th push overwrites the oldest entry)
//   - No stack pointer register is visible to programs
//   - JMS pushes, BBL pops — that's the only stack access
package intel4004simulator

import (
	"fmt"
)

// Intel4004Trace logs step execution details for debugging and analysis.
// Each trace captures the complete before/after state of the accumulator
// and carry flag, plus the raw instruction bytes and decoded mnemonic.
type Intel4004Trace struct {
	Address           int    // ROM address where this instruction was fetched
	Raw               int    // First byte of the instruction (0x00-0xFF)
	Raw2              int    // Second byte for 2-byte instructions (-1 if single-byte)
	Mnemonic          string // Human-readable form, e.g., "ADD R3", "JUN 0x100"
	AccumulatorBefore int    // Value of A before execution
	AccumulatorAfter  int    // Value of A after execution
	CarryBefore       bool   // Carry flag before execution
	CarryAfter        bool   // Carry flag after execution
}

// Intel4004Simulator models the complete Intel 4004 microprocessor.
//
// This simulator is standalone — it does not depend on a generic VM framework.
// The 4004's 4-bit data path, 3-level hardware stack, and BCD-oriented
// instructions are sufficiently unique that a generic VM would add complexity
// without benefit.
type Intel4004Simulator struct {
	// --- CPU Registers ---
	// The Accumulator is the heart of all computation. Every arithmetic and
	// logic operation reads from and writes to A. Values are always 0-15.
	Accumulator int // 4-bit accumulator (0-15)

	// 16 general-purpose 4-bit registers (R0-R15), organized as 8 pairs.
	// Pair 0 = R0:R1, Pair 1 = R2:R3, ..., Pair 7 = R14:R15.
	// Some instructions (FIM, SRC, FIN, JIN) operate on pairs, treating the
	// even register as the high nibble and the odd register as the low nibble.
	Registers [16]int // R0-R15, each 0-15

	// The Carry flag serves double duty:
	//   - After ADD: set if result > 15 (overflow)
	//   - After SUB: set if NO borrow (result >= 0), cleared if borrow
	// This inverted-borrow convention matches the MCS-4 hardware manual.
	Carry bool

	// --- Program Memory (ROM) ---
	// The 4004 addresses up to 4096 bytes of ROM. In real hardware this is
	// read-only, but our simulator allows writing for program loading.
	Memory []byte // ROM — program storage

	// --- Program Counter ---
	// 12 bits wide, addressing 0-4095 bytes of ROM.
	PC int

	// --- Hardware Call Stack ---
	// The 4004 has exactly 3 stack slots built from on-chip registers.
	// JMS pushes the return address, BBL pops it. On overflow, the stack
	// wraps silently (modulo 3) — there is no stack overflow exception.
	// This limits subroutine nesting to 3 levels on real hardware.
	HwStack      [3]uint16 // 3-level hardware call stack (12-bit addresses)
	StackPointer int       // 0-2, wraps mod 3

	// --- RAM ---
	// The 4004's RAM is organized hierarchically for calculator use:
	//   4 banks (selected by DCL instruction)
	//     4 registers per bank (selected by SRC high nibble)
	//       16 main characters per register (selected by SRC low nibble)
	//       4 status characters per register (accessed by WR0-WR3/RD0-RD3)
	//       1 output port per bank (written by WMP)
	//
	// "Characters" is Intel's term for 4-bit nibbles in this context.
	// Main characters store data (e.g., calculator digits).
	// Status characters store flags/state (e.g., sign, decimal point position).
	RAM       [4][4][16]uint8 // [bank][register][character] — main storage
	RAMStatus [4][4][4]uint8  // [bank][register][index] — status nibbles
	RAMOutput [4]uint8        // one output port per bank

	// --- RAM Addressing (set by SRC and DCL) ---
	// The SRC instruction loads the RAM register and character from a register
	// pair. The DCL instruction selects the bank. Subsequent I/O instructions
	// (WRM, RDM, etc.) use these stored values.
	RAMBank      int // selected by DCL (0-3)
	RAMRegister  int // high nibble from SRC pair value
	RAMCharacter int // low nibble from SRC pair value

	// --- ROM I/O Port ---
	// A single 4-bit I/O port accessible via WRR (write) and RDR (read).
	// On real hardware, each ROM chip had its own I/O port; we simulate one.
	ROMPort uint8

	// --- Control ---
	Halted bool
}

// NewIntel4004Simulator creates a simulator with the given ROM size.
// The standard 4004 addresses 4096 bytes, but smaller sizes work for testing.
func NewIntel4004Simulator(memorySize int) *Intel4004Simulator {
	return &Intel4004Simulator{
		Memory: make([]byte, memorySize),
	}
}

// LoadProgram copies a program into ROM starting at address 0.
// The PC is reset to 0 and the Halted flag is cleared.
func (s *Intel4004Simulator) LoadProgram(program []byte) {
	// Clear ROM before loading
	for i := range s.Memory {
		s.Memory[i] = 0
	}
	for i, b := range program {
		if i < len(s.Memory) {
			s.Memory[i] = b
		}
	}
	s.PC = 0
	s.Halted = false
}

// Reset clears all CPU state to power-on defaults.
func (s *Intel4004Simulator) Reset() {
	s.Accumulator = 0
	s.Registers = [16]int{}
	s.Carry = false
	for i := range s.Memory {
		s.Memory[i] = 0
	}
	s.PC = 0
	s.Halted = false
	s.HwStack = [3]uint16{}
	s.StackPointer = 0
	s.RAM = [4][4][16]uint8{}
	s.RAMStatus = [4][4][4]uint8{}
	s.RAMOutput = [4]uint8{}
	s.RAMBank = 0
	s.RAMRegister = 0
	s.RAMCharacter = 0
	s.ROMPort = 0
}

// ---------------------------------------------------------------------------
// 2-byte instruction detection
// ---------------------------------------------------------------------------

// isTwoByte returns true if the given raw byte starts a 2-byte instruction.
//
// The 4004 has five 2-byte instruction families:
//
//	0x1_ JCN  — conditional jump
//	0x2_ FIM  — fetch immediate (even lower nibble only)
//	0x4_ JUN  — unconditional jump
//	0x5_ JMS  — jump to subroutine
//	0x7_ ISZ  — increment and skip if zero
//
// All other instructions are single-byte. FIM vs SRC is distinguished by
// the lowest bit: even = FIM (2-byte), odd = SRC (1-byte).
func isTwoByte(raw byte) bool {
	upper := (raw >> 4) & 0xF
	switch upper {
	case 0x1, 0x4, 0x5, 0x7:
		return true
	case 0x2:
		// FIM is 2-byte (even lower nibble), SRC is 1-byte (odd)
		return (raw & 0x1) == 0
	}
	return false
}

// ---------------------------------------------------------------------------
// Register pair helpers
// ---------------------------------------------------------------------------

// readPair reads an 8-bit value from a register pair.
// Pair 0 = R0:R1, Pair 1 = R2:R3, ..., Pair 7 = R14:R15.
// The even register provides the high nibble, the odd register the low nibble.
func (s *Intel4004Simulator) readPair(pairIdx int) int {
	highReg := pairIdx * 2
	lowReg := highReg + 1
	return (s.Registers[highReg] << 4) | s.Registers[lowReg]
}

// writePair writes an 8-bit value to a register pair.
func (s *Intel4004Simulator) writePair(pairIdx int, value int) {
	highReg := pairIdx * 2
	lowReg := highReg + 1
	s.Registers[highReg] = (value >> 4) & 0xF
	s.Registers[lowReg] = value & 0xF
}

// ---------------------------------------------------------------------------
// Stack helpers
// ---------------------------------------------------------------------------

// stackPush pushes a 12-bit return address onto the 3-level hardware stack.
// The real 4004 wraps silently on overflow — the 4th push overwrites the
// oldest entry. There is no stack overflow exception.
func (s *Intel4004Simulator) stackPush(address int) {
	s.HwStack[s.StackPointer] = uint16(address & 0xFFF)
	s.StackPointer = (s.StackPointer + 1) % 3
}

// stackPop pops a return address from the hardware stack.
func (s *Intel4004Simulator) stackPop() int {
	s.StackPointer = (s.StackPointer - 1 + 3) % 3
	return int(s.HwStack[s.StackPointer])
}

// ---------------------------------------------------------------------------
// RAM helpers
// ---------------------------------------------------------------------------

// ramReadMain reads the current RAM main character (selected by SRC + DCL).
func (s *Intel4004Simulator) ramReadMain() int {
	return int(s.RAM[s.RAMBank][s.RAMRegister][s.RAMCharacter])
}

// ramWriteMain writes to the current RAM main character.
func (s *Intel4004Simulator) ramWriteMain(value int) {
	s.RAM[s.RAMBank][s.RAMRegister][s.RAMCharacter] = uint8(value & 0xF)
}

// ramReadStatus reads a RAM status character (0-3) for the current register.
func (s *Intel4004Simulator) ramReadStatus(index int) int {
	return int(s.RAMStatus[s.RAMBank][s.RAMRegister][index])
}

// ramWriteStatus writes a RAM status character (0-3).
func (s *Intel4004Simulator) ramWriteStatus(index int, value int) {
	s.RAMStatus[s.RAMBank][s.RAMRegister][index] = uint8(value & 0xF)
}

// ---------------------------------------------------------------------------
// Fetch-Decode-Execute
// ---------------------------------------------------------------------------

// Step performs one fetch-decode-execute cycle.
//
// The 4004's instruction cycle is simple compared to modern pipelined CPUs:
//  1. Fetch: read the byte at Memory[PC]
//  2. Decode: check if it's a 2-byte instruction; if so, fetch the second byte
//  3. Execute: dispatch based on the upper nibble (or full byte for 0xE_/0xF_)
//
// Returns a trace record capturing the complete before/after state.
func (s *Intel4004Simulator) Step() Intel4004Trace {
	if s.Halted {
		panic("CPU is halted")
	}

	address := s.PC
	raw := int(s.Memory[s.PC])
	s.PC++

	// Check for 2-byte instruction and fetch second byte
	raw2 := -1
	if isTwoByte(byte(raw)) {
		raw2 = int(s.Memory[s.PC])
		s.PC++
	}

	accBefore := s.Accumulator
	carryBefore := s.Carry

	mnemonic := s.execute(raw, raw2)

	return Intel4004Trace{
		Address:           address,
		Raw:               raw,
		Raw2:              raw2,
		Mnemonic:          mnemonic,
		AccumulatorBefore: accBefore,
		AccumulatorAfter:  s.Accumulator,
		CarryBefore:       carryBefore,
		CarryAfter:        s.Carry,
	}
}

// execute dispatches the instruction based on its encoding.
//
// The Intel 4004 instruction set uses the upper nibble of the first byte
// to determine the instruction family. For 0xE_ and 0xF_ ranges, the full
// byte determines the specific instruction (16 I/O ops + 14 accumulator ops).
func (s *Intel4004Simulator) execute(raw, raw2 int) string {
	upper := (raw >> 4) & 0xF
	lower := raw & 0xF

	switch upper {

	// --- 0x0_: NOP and HLT ---
	case 0x0:
		if raw == 0x00 {
			return s.execNOP()
		}
		if raw == 0x01 {
			return s.execHLT()
		}
		return fmt.Sprintf("UNKNOWN(0x%02X)", raw)

	// --- 0x1_: JCN (conditional jump) ---
	// 2-byte instruction: first byte encodes condition, second byte is target
	case 0x1:
		return s.execJCN(lower, raw2)

	// --- 0x2_: FIM (even) or SRC (odd) ---
	// FIM loads an 8-bit immediate into a register pair (2-byte).
	// SRC sends a register pair as a RAM/ROM address (1-byte).
	case 0x2:
		pair := lower >> 1
		if lower&1 == 0 {
			return s.execFIM(pair, raw2)
		}
		return s.execSRC(pair)

	// --- 0x3_: FIN (even) or JIN (odd) ---
	// FIN fetches from ROM indirectly via P0 into pair Pp (1-byte).
	// JIN jumps indirectly via register pair Pp (1-byte).
	case 0x3:
		pair := lower >> 1
		if lower&1 == 0 {
			return s.execFIN(pair)
		}
		return s.execJIN(pair)

	// --- 0x4_: JUN (unconditional jump) ---
	// 2-byte: 12-bit address formed from lower nibble (high 4 bits) + second byte
	case 0x4:
		addr12 := (lower << 8) | raw2
		return s.execJUN(addr12)

	// --- 0x5_: JMS (jump to subroutine) ---
	// 2-byte: same encoding as JUN, but pushes return address first
	case 0x5:
		addr12 := (lower << 8) | raw2
		return s.execJMS(addr12)

	// --- 0x6_: INC (increment register) ---
	case 0x6:
		return s.execINC(lower)

	// --- 0x7_: ISZ (increment and skip if zero) ---
	// 2-byte: increment register, jump to target if NOT zero
	case 0x7:
		return s.execISZ(lower, raw2)

	// --- 0x8_: ADD (add register to accumulator with carry) ---
	case 0x8:
		return s.execADD(lower)

	// --- 0x9_: SUB (subtract register using complement-add) ---
	case 0x9:
		return s.execSUB(lower)

	// --- 0xA_: LD (load register into accumulator) ---
	case 0xA:
		return s.execLD(lower)

	// --- 0xB_: XCH (exchange accumulator and register) ---
	case 0xB:
		return s.execXCH(lower)

	// --- 0xC_: BBL (branch back and load) ---
	case 0xC:
		return s.execBBL(lower)

	// --- 0xD_: LDM (load immediate) ---
	case 0xD:
		return s.execLDM(lower)

	// --- 0xE_: I/O operations ---
	// Each instruction in this range is identified by the full byte.
	// These instructions interact with the 4004's RAM/ROM I/O subsystem.
	case 0xE:
		return s.executeIO(raw)

	// --- 0xF_: Accumulator operations ---
	// Each instruction in this range is identified by the full byte.
	// These perform various manipulations on A and the carry flag.
	case 0xF:
		return s.executeAccum(raw)
	}

	return fmt.Sprintf("UNKNOWN(0x%02X)", raw)
}

// ---------------------------------------------------------------------------
// Individual instruction implementations
// ---------------------------------------------------------------------------
// Each method below implements one (or a small family) of 4004 instructions.
// The naming convention is execXXX where XXX is the mnemonic.

// === NOP (0x00) ===
// No operation. The simplest possible instruction — the CPU does nothing
// except advance the program counter. Useful for timing delays or as a
// placeholder during program development.
func (s *Intel4004Simulator) execNOP() string {
	return "NOP"
}

// === HLT (0x01) ===
// Halt execution. This is a simulator-only instruction — the real 4004 had
// no halt instruction. Programs ran until power was removed. We add HLT to
// provide a clean way to stop program execution in testing.
func (s *Intel4004Simulator) execHLT() string {
	s.Halted = true
	return "HLT"
}

// === LDM N (0xDN) ===
// Load immediate: A = N (4-bit value from the instruction's lower nibble).
// This is how constants enter the CPU. Since N is only 4 bits, values are
// limited to 0-15. For larger constants, use FIM with a register pair.
func (s *Intel4004Simulator) execLDM(n int) string {
	s.Accumulator = n & 0xF
	return fmt.Sprintf("LDM %d", n)
}

// === LD Rn (0xAR) ===
// Load register into accumulator: A = Rn.
// This copies the register value into A without modifying the register.
// It's the 4004's "read" instruction for registers.
func (s *Intel4004Simulator) execLD(reg int) string {
	s.Accumulator = s.Registers[reg] & 0xF
	return fmt.Sprintf("LD R%d", reg)
}

// === XCH Rn (0xBR) ===
// Exchange accumulator and register: swap A and Rn.
// Since there is no "store" instruction, XCH is how you move values FROM
// the accumulator TO a register. The old register value moves into A.
func (s *Intel4004Simulator) execXCH(reg int) string {
	oldA := s.Accumulator
	s.Accumulator = s.Registers[reg] & 0xF
	s.Registers[reg] = oldA & 0xF
	return fmt.Sprintf("XCH R%d", reg)
}

// === INC Rn (0x6R) ===
// Increment register: Rn = (Rn + 1) & 0xF.
// Note: INC does NOT affect the carry flag. It wraps from 15 to 0 silently.
// This is purely a register operation — the accumulator is not involved.
func (s *Intel4004Simulator) execINC(reg int) string {
	s.Registers[reg] = (s.Registers[reg] + 1) & 0xF
	return fmt.Sprintf("INC R%d", reg)
}

// === ADD Rn (0x8R) ===
// Add register to accumulator with carry: A = A + Rn + carry_in.
// The carry flag participates in the addition — this is how multi-digit
// BCD arithmetic chains across digits. After adding two BCD digits, the
// carry propagates to the next digit pair.
//
// Carry is set if the result exceeds 15 (4-bit overflow).
func (s *Intel4004Simulator) execADD(reg int) string {
	carryIn := 0
	if s.Carry {
		carryIn = 1
	}
	result := s.Accumulator + s.Registers[reg] + carryIn
	s.Carry = result > 0xF
	s.Accumulator = result & 0xF
	return fmt.Sprintf("ADD R%d", reg)
}

// === SUB Rn (0x9R) ===
// Subtract register from accumulator using complement-add with borrow.
//
// The 4004 computes subtraction as: A = A + (~Rn & 0xF) + borrow_in
// where borrow_in = 1 if carry is CLEAR (no previous borrow), 0 if SET.
//
// The carry flag is INVERTED from what you might expect:
//   - carry=1 after SUB means NO borrow occurred (result >= 0)
//   - carry=0 after SUB means borrow occurred (result was negative)
//
// This complement-add approach eliminates the need for a subtraction circuit
// in hardware — the same adder used for ADD works for SUB by complementing
// one input. This saved transistors on the 2,300-transistor 4004.
//
// Example: A=3, R0=1, carry=false (no prior borrow)
//
//	complement = ~1 & 0xF = 14
//	borrow_in = 1 (carry is clear, so no incoming borrow)
//	result = 3 + 14 + 1 = 18
//	carry = true (18 > 15, meaning no borrow)
//	A = 18 & 0xF = 2 (correct: 3 - 1 = 2)
func (s *Intel4004Simulator) execSUB(reg int) string {
	complement := (^s.Registers[reg]) & 0xF
	borrowIn := 0
	if !s.Carry {
		borrowIn = 1
	}
	result := s.Accumulator + complement + borrowIn
	s.Carry = result > 0xF
	s.Accumulator = result & 0xF
	return fmt.Sprintf("SUB R%d", reg)
}

// === JCN cond,addr (0x1C 0xAA) ===
// Conditional jump. The condition nibble C has 4 bits that control the test:
//
//	Bit 3 (0x8): INVERT — if set, invert the final test result
//	Bit 2 (0x4): TEST_ZERO — test if accumulator == 0
//	Bit 1 (0x2): TEST_CARRY — test if carry flag is set
//	Bit 0 (0x1): TEST_PIN — test input pin (always 0 in our simulator)
//
// Multiple test bits can be set simultaneously — they are OR'd together.
// If the (possibly inverted) result is true, the jump is taken to the
// address within the same 256-byte page.
//
// Common condition codes:
//
//	0x4 = jump if A == 0
//	0xC = jump if A != 0 (0x4 | 0x8, inverted zero test)
//	0x2 = jump if carry set
//	0xA = jump if carry clear (0x2 | 0x8, inverted carry test)
func (s *Intel4004Simulator) execJCN(cond, raw2 int) string {
	// Evaluate condition tests (OR'd together)
	testResult := false
	if cond&0x4 != 0 { // Test accumulator == 0
		testResult = testResult || (s.Accumulator == 0)
	}
	if cond&0x2 != 0 { // Test carry flag
		testResult = testResult || s.Carry
	}
	if cond&0x1 != 0 { // Test input pin (always false in simulator)
		testResult = testResult || false
	}

	// Invert if bit 3 is set
	if cond&0x8 != 0 {
		testResult = !testResult
	}

	if testResult {
		// Jump target is within the same 256-byte page as the instruction
		// AFTER this 2-byte JCN. PC already advanced past both bytes.
		page := s.PC & 0xF00
		s.PC = page | (raw2 & 0xFF)
	}

	return fmt.Sprintf("JCN %d,0x%02X", cond, raw2)
}

// === FIM Pp,data (0x2P 0xDD) ===
// Fetch immediate to register pair. The 8-bit data byte is split into two
// 4-bit halves: the high nibble goes to R(2*p), the low nibble to R(2*p+1).
// This is the only way to load an 8-bit value in one instruction.
func (s *Intel4004Simulator) execFIM(pair, data int) string {
	s.writePair(pair, data)
	return fmt.Sprintf("FIM P%d,0x%02X", pair, data)
}

// === SRC Pp (0x2P+1) ===
// Send register control. The 8-bit value in register pair Pp becomes the
// address for subsequent RAM/ROM I/O operations:
//   - High nibble = RAM register index (0-3, though the full nibble is stored)
//   - Low nibble = RAM character index (0-15)
//
// SRC must be executed before any I/O instruction (WRM, RDM, etc.) to set
// the target address. The bank is selected separately by DCL.
func (s *Intel4004Simulator) execSRC(pair int) string {
	pairVal := s.readPair(pair)
	s.RAMRegister = (pairVal >> 4) & 0xF
	s.RAMCharacter = pairVal & 0xF
	return fmt.Sprintf("SRC P%d", pair)
}

// === FIN Pp (0x3P, even) ===
// Fetch indirect from ROM. Reads the ROM byte at the address given by
// register pair P0 (R0:R1), using the same page as the current PC.
// The result is stored into register pair Pp.
//
// This is an indirect load: P0 provides an offset within the current page,
// and the fetched byte is split into the target pair's two registers.
func (s *Intel4004Simulator) execFIN(pair int) string {
	// Address comes from P0 (R0:R1)
	p0Val := s.readPair(0)
	// Same page as the current PC (which has already advanced past this instruction)
	currentPage := s.PC & 0xF00
	romAddr := currentPage | p0Val
	romByte := 0
	if romAddr < len(s.Memory) {
		romByte = int(s.Memory[romAddr])
	}
	s.writePair(pair, romByte)
	return fmt.Sprintf("FIN P%d", pair)
}

// === JIN Pp (0x3P+1, odd) ===
// Jump indirect via register pair. The PC is set to the current page
// combined with the 8-bit value in register pair Pp.
// PC[11:8] stays the same, PC[7:0] = pair value.
func (s *Intel4004Simulator) execJIN(pair int) string {
	pairVal := s.readPair(pair)
	currentPage := s.PC & 0xF00
	s.PC = currentPage | pairVal
	return fmt.Sprintf("JIN P%d", pair)
}

// === JUN addr (0x4H 0xLL) ===
// Unconditional jump to a 12-bit address. The address is formed by
// combining the lower nibble of the first byte (high 4 bits of address)
// with the full second byte (low 8 bits of address).
// This can jump anywhere in the 4096-byte ROM address space.
func (s *Intel4004Simulator) execJUN(addr12 int) string {
	s.PC = addr12 & 0xFFF
	return fmt.Sprintf("JUN 0x%03X", addr12)
}

// === JMS addr (0x5H 0xLL) ===
// Jump to subroutine. Push the return address (PC after this 2-byte
// instruction) onto the 3-level hardware stack, then jump to the target.
// The return address is the address immediately after this JMS instruction.
func (s *Intel4004Simulator) execJMS(addr12 int) string {
	// PC already points past this 2-byte instruction — that's our return address
	s.stackPush(s.PC)
	s.PC = addr12 & 0xFFF
	return fmt.Sprintf("JMS 0x%03X", addr12)
}

// === ISZ Rn,addr (0x7R 0xAA) ===
// Increment register and skip if zero. Increment Rn, then:
//   - If Rn != 0 after increment: jump to addr (within same page)
//   - If Rn == 0 (wrapped from 15): fall through to the next instruction
//
// This is the 4004's primary loop construct. To loop N times, load a register
// with (16 - N) and ISZ will count up to 0. For example, to loop 4 times:
// load R0 with 12 (16 - 4), then ISZ R0 loops until R0 wraps to 0.
func (s *Intel4004Simulator) execISZ(reg, raw2 int) string {
	s.Registers[reg] = (s.Registers[reg] + 1) & 0xF

	if s.Registers[reg] != 0 {
		// Jump to target within same page
		page := s.PC & 0xF00
		s.PC = page | (raw2 & 0xFF)
	}
	// If register == 0, fall through (PC already advanced past both bytes)

	return fmt.Sprintf("ISZ R%d,0x%02X", reg, raw2)
}

// === BBL N (0xCN) ===
// Branch back and load. Pop the hardware stack to get a return address,
// load the immediate value N into the accumulator, then jump to the
// return address.
//
// This is the 4004's "return from subroutine" instruction with a bonus:
// it also loads A with an immediate value. This lets subroutines return
// a simple status code (0-15) without needing a separate LDM instruction.
func (s *Intel4004Simulator) execBBL(n int) string {
	s.Accumulator = n & 0xF
	returnAddr := s.stackPop()
	s.PC = returnAddr
	return fmt.Sprintf("BBL %d", n)
}

// ---------------------------------------------------------------------------
// I/O instruction dispatch (0xE0-0xEF)
// ---------------------------------------------------------------------------

// executeIO dispatches I/O instructions. The 4004 has 16 I/O instructions
// in the 0xE0-0xEF range, each identified by the full byte.
func (s *Intel4004Simulator) executeIO(raw int) string {
	switch raw {

	// === WRM (0xE0) ===
	// Write accumulator to RAM main character at the address set by SRC/DCL.
	case 0xE0:
		s.ramWriteMain(s.Accumulator)
		return "WRM"

	// === WMP (0xE1) ===
	// Write accumulator to RAM output port. Each bank has one output port.
	// On real hardware, this drove external display or control lines.
	case 0xE1:
		s.RAMOutput[s.RAMBank] = uint8(s.Accumulator & 0xF)
		return "WMP"

	// === WRR (0xE2) ===
	// Write accumulator to ROM I/O port. Each ROM chip had a 4-bit I/O port.
	case 0xE2:
		s.ROMPort = uint8(s.Accumulator & 0xF)
		return "WRR"

	// === WPM (0xE3) ===
	// Write program RAM. On real hardware, this was used for EPROM programming.
	// Not meaningful in simulation — treated as NOP.
	case 0xE3:
		return "WPM"

	// === WR0-WR3 (0xE4-0xE7) ===
	// Write accumulator to RAM status character 0-3.
	// Status characters are a secondary storage area (4 nibbles per register)
	// separate from the 16 main characters. They were used for flags and
	// metadata in calculator applications.
	case 0xE4:
		s.ramWriteStatus(0, s.Accumulator)
		return "WR0"
	case 0xE5:
		s.ramWriteStatus(1, s.Accumulator)
		return "WR1"
	case 0xE6:
		s.ramWriteStatus(2, s.Accumulator)
		return "WR2"
	case 0xE7:
		s.ramWriteStatus(3, s.Accumulator)
		return "WR3"

	// === SBM (0xE8) ===
	// Subtract RAM main character from accumulator (complement-add, like SUB).
	// A = A + ~RAM_char + borrow_in. Same inverted-carry convention as SUB.
	case 0xE8:
		ramVal := s.ramReadMain()
		complement := (^ramVal) & 0xF
		borrowIn := 0
		if !s.Carry {
			borrowIn = 1
		}
		result := s.Accumulator + complement + borrowIn
		s.Carry = result > 0xF
		s.Accumulator = result & 0xF
		return "SBM"

	// === RDM (0xE9) ===
	// Read RAM main character into accumulator. A = RAM[bank][register][character].
	case 0xE9:
		s.Accumulator = s.ramReadMain()
		return "RDM"

	// === RDR (0xEA) ===
	// Read ROM I/O port into accumulator.
	case 0xEA:
		s.Accumulator = int(s.ROMPort) & 0xF
		return "RDR"

	// === ADM (0xEB) ===
	// Add RAM main character to accumulator with carry.
	// A = A + RAM_char + carry_in. Same as ADD but with RAM instead of register.
	case 0xEB:
		ramVal := s.ramReadMain()
		carryIn := 0
		if s.Carry {
			carryIn = 1
		}
		result := s.Accumulator + ramVal + carryIn
		s.Carry = result > 0xF
		s.Accumulator = result & 0xF
		return "ADM"

	// === RD0-RD3 (0xEC-0xEF) ===
	// Read RAM status character 0-3 into accumulator.
	case 0xEC:
		s.Accumulator = s.ramReadStatus(0)
		return "RD0"
	case 0xED:
		s.Accumulator = s.ramReadStatus(1)
		return "RD1"
	case 0xEE:
		s.Accumulator = s.ramReadStatus(2)
		return "RD2"
	case 0xEF:
		s.Accumulator = s.ramReadStatus(3)
		return "RD3"
	}

	return fmt.Sprintf("UNKNOWN(0x%02X)", raw)
}

// ---------------------------------------------------------------------------
// Accumulator instruction dispatch (0xF0-0xFD)
// ---------------------------------------------------------------------------

// executeAccum dispatches accumulator manipulation instructions.
// These instructions operate on the accumulator and/or carry flag without
// involving registers or RAM. They provide bit manipulation, BCD support,
// and flag control.
func (s *Intel4004Simulator) executeAccum(raw int) string {
	switch raw {

	// === CLB (0xF0) ===
	// Clear both: A = 0, carry = 0. A convenient reset for starting fresh
	// calculations.
	case 0xF0:
		s.Accumulator = 0
		s.Carry = false
		return "CLB"

	// === CLC (0xF1) ===
	// Clear carry: carry = 0. Useful before a chain of ADDs to ensure
	// no stale carry propagates.
	case 0xF1:
		s.Carry = false
		return "CLC"

	// === IAC (0xF2) ===
	// Increment accumulator: A = (A + 1) & 0xF.
	// Carry is set if A wraps from 15 to 0.
	case 0xF2:
		result := s.Accumulator + 1
		s.Carry = result > 0xF
		s.Accumulator = result & 0xF
		return "IAC"

	// === CMC (0xF3) ===
	// Complement carry: carry = !carry. Flips the carry flag.
	case 0xF3:
		s.Carry = !s.Carry
		return "CMC"

	// === CMA (0xF4) ===
	// Complement accumulator: A = ~A & 0xF (4-bit bitwise NOT).
	// For example: A=5 (0101) becomes A=10 (1010).
	case 0xF4:
		s.Accumulator = (^s.Accumulator) & 0xF
		return "CMA"

	// === RAL (0xF5) ===
	// Rotate accumulator left through carry. This is a 5-bit rotation
	// where the carry acts as the 5th bit:
	//
	//   Before: [carry | A3 A2 A1 A0]
	//   After:  [A3   | A2 A1 A0 carry_old]
	//
	// The highest bit of A shifts into carry, and the old carry shifts
	// into the lowest bit of A. This is essential for multi-nibble
	// shift operations in calculator arithmetic.
	case 0xF5:
		oldCarry := 0
		if s.Carry {
			oldCarry = 1
		}
		s.Carry = (s.Accumulator & 0x8) != 0
		s.Accumulator = ((s.Accumulator << 1) | oldCarry) & 0xF
		return "RAL"

	// === RAR (0xF6) ===
	// Rotate accumulator right through carry. Mirror of RAL:
	//
	//   Before: [carry | A3 A2 A1 A0]
	//   After:  [A0   | carry_old A3 A2 A1]
	//
	// The lowest bit of A shifts into carry, and the old carry shifts
	// into the highest bit of A.
	case 0xF6:
		oldCarry := 0
		if s.Carry {
			oldCarry = 1
		}
		s.Carry = (s.Accumulator & 0x1) != 0
		s.Accumulator = ((s.Accumulator >> 1) | (oldCarry << 3)) & 0xF
		return "RAR"

	// === TCC (0xF7) ===
	// Transfer carry to accumulator and clear carry.
	// A = 1 if carry was set, else A = 0. Carry is always cleared.
	// Useful for converting the carry flag into a numerical value.
	case 0xF7:
		if s.Carry {
			s.Accumulator = 1
		} else {
			s.Accumulator = 0
		}
		s.Carry = false
		return "TCC"

	// === DAC (0xF8) ===
	// Decrement accumulator: A = (A - 1) & 0xF.
	// Carry is SET if no borrow (A > 0), CLEARED if borrow (A was 0).
	// When A=0, it wraps to 15 and carry is cleared (borrow occurred).
	case 0xF8:
		result := s.Accumulator - 1
		s.Carry = result >= 0 // No borrow if result >= 0
		s.Accumulator = result & 0xF
		return "DAC"

	// === TCS (0xF9) ===
	// Transfer carry subtract. A = 10 if carry was set, else A = 9.
	// Carry is always cleared.
	//
	// This instruction exists for BCD subtraction. When subtracting BCD
	// digits, you need to add a correction factor. TCS provides either
	// 9 (no carry from previous digit) or 10 (carry from previous digit)
	// as the tens-complement correction.
	case 0xF9:
		if s.Carry {
			s.Accumulator = 10
		} else {
			s.Accumulator = 9
		}
		s.Carry = false
		return "TCS"

	// === STC (0xFA) ===
	// Set carry: carry = 1. The complement of CLC.
	case 0xFA:
		s.Carry = true
		return "STC"

	// === DAA (0xFB) ===
	// Decimal adjust accumulator (BCD correction).
	//
	// If A > 9 or carry is set, add 6 to A. If the addition causes
	// overflow past 15, set carry.
	//
	// This instruction exists because the 4004 was built for BCD calculators.
	// When adding two BCD digits (0-9 each), the binary result might exceed 9
	// (e.g., 7 + 8 = 15 in binary). DAA corrects this by adding 6:
	//   15 + 6 = 21 → keep lower nibble (5), set carry for tens digit
	// Result: 15 becomes "1 carry, 5" = BCD 15. Exactly right!
	case 0xFB:
		if s.Accumulator > 9 || s.Carry {
			result := s.Accumulator + 6
			if result > 0xF {
				s.Carry = true
			}
			s.Accumulator = result & 0xF
		}
		return "DAA"

	// === KBP (0xFC) ===
	// Keyboard process. Converts a one-hot encoded keyboard input to a
	// binary position number:
	//
	//   Input (one-hot)  →  Output (position)
	//   0b0000 (0)       →  0  (no key pressed)
	//   0b0001 (1)       →  1  (key 1)
	//   0b0010 (2)       →  2  (key 2)
	//   0b0100 (4)       →  3  (key 3)
	//   0b1000 (8)       →  4  (key 4)
	//   anything else    →  15 (error: multiple keys pressed)
	//
	// This was designed for the Busicom calculator's keyboard scanning
	// circuit. Each key closes one switch, producing a one-hot pattern.
	// KBP converts it to a key number for software processing.
	case 0xFC:
		switch s.Accumulator {
		case 0:
			s.Accumulator = 0
		case 1:
			s.Accumulator = 1
		case 2:
			s.Accumulator = 2
		case 4:
			s.Accumulator = 3
		case 8:
			s.Accumulator = 4
		default:
			s.Accumulator = 15
		}
		return "KBP"

	// === DCL (0xFD) ===
	// Designate command line (select RAM bank). The lower 3 bits of A
	// select which RAM bank (0-7) is active for subsequent I/O operations.
	// Since the 4004 only has 4 banks, we clamp to 0-3.
	case 0xFD:
		s.RAMBank = s.Accumulator & 0x7
		if s.RAMBank > 3 {
			s.RAMBank = s.RAMBank & 0x3
		}
		return "DCL"
	}

	return fmt.Sprintf("UNKNOWN(0x%02X)", raw)
}

// ---------------------------------------------------------------------------
// Run — execute a complete program
// ---------------------------------------------------------------------------

// Run loads and executes a program, returning a trace of every instruction.
// Execution continues until HLT is encountered or maxSteps is reached.
func (s *Intel4004Simulator) Run(program []byte, maxSteps int) []Intel4004Trace {
	s.LoadProgram(program)
	var traces []Intel4004Trace
	for i := 0; i < maxSteps; i++ {
		if s.Halted || s.PC >= len(s.Memory) {
			break
		}
		trace := s.Step()
		traces = append(traces, trace)
	}
	return traces
}

// ---------------------------------------------------------------------------
// Encoder helpers — build instruction bytes for testing
// ---------------------------------------------------------------------------
// These functions encode 4004 instructions into their binary format.
// They're primarily used in tests to build programs without hand-coding hex.

// EncodeNOP returns the NOP instruction byte.
func EncodeNOP() byte { return 0x00 }

// EncodeHlt returns the HLT instruction byte (simulator-only).
func EncodeHlt() byte { return 0x01 }

// EncodeLdm returns a LDM N instruction byte.
func EncodeLdm(n int) byte { return byte(0xD0 | (n & 0xF)) }

// EncodeLd returns a LD Rn instruction byte.
func EncodeLd(r int) byte { return byte(0xA0 | (r & 0xF)) }

// EncodeXch returns a XCH Rn instruction byte.
func EncodeXch(r int) byte { return byte(0xB0 | (r & 0xF)) }

// EncodeAdd returns an ADD Rn instruction byte.
func EncodeAdd(r int) byte { return byte(0x80 | (r & 0xF)) }

// EncodeSub returns a SUB Rn instruction byte.
func EncodeSub(r int) byte { return byte(0x90 | (r & 0xF)) }

// EncodeInc returns an INC Rn instruction byte.
func EncodeInc(r int) byte { return byte(0x60 | (r & 0xF)) }

// EncodeBbl returns a BBL N instruction byte.
func EncodeBbl(n int) byte { return byte(0xC0 | (n & 0xF)) }

// EncodeJun returns a 2-byte JUN addr instruction.
func EncodeJun(addr int) (byte, byte) {
	return byte(0x40 | ((addr >> 8) & 0xF)), byte(addr & 0xFF)
}

// EncodeJms returns a 2-byte JMS addr instruction.
func EncodeJms(addr int) (byte, byte) {
	return byte(0x50 | ((addr >> 8) & 0xF)), byte(addr & 0xFF)
}

// EncodeJcn returns a 2-byte JCN cond,addr instruction.
func EncodeJcn(cond, addr int) (byte, byte) {
	return byte(0x10 | (cond & 0xF)), byte(addr & 0xFF)
}

// EncodeFim returns a 2-byte FIM Pp,data instruction.
func EncodeFim(pair, data int) (byte, byte) {
	return byte(0x20 | ((pair & 0x7) << 1)), byte(data & 0xFF)
}

// EncodeSrc returns a SRC Pp instruction byte.
func EncodeSrc(pair int) byte { return byte(0x21 | ((pair & 0x7) << 1)) }

// EncodeFin returns a FIN Pp instruction byte.
func EncodeFin(pair int) byte { return byte(0x30 | ((pair & 0x7) << 1)) }

// EncodeJin returns a JIN Pp instruction byte.
func EncodeJin(pair int) byte { return byte(0x31 | ((pair & 0x7) << 1)) }

// EncodeIsz returns a 2-byte ISZ Rn,addr instruction.
func EncodeIsz(reg, addr int) (byte, byte) {
	return byte(0x70 | (reg & 0xF)), byte(addr & 0xFF)
}
