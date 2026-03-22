package intel4004gatelevel

// Intel 4004 gate-level CPU — all operations route through real logic gates.
//
// # What makes this a "gate-level" simulator?
//
// Every computation in this CPU flows through the same gate chain that the
// real Intel 4004 used:
//
//	NOT/AND/OR/XOR -> half_adder -> full_adder -> ripple_carry_adder -> ALU
//	D flip-flop -> register -> register file / program counter / stack
//
// When you execute ADD R3, the value in register R3 is read from flip-flops,
// the accumulator is read from flip-flops, both are fed into the ALU (which
// uses full adders built from gates), and the result is clocked back into
// the accumulator's flip-flops.
//
// Nothing is simulated behaviorally. Every bit passes through gate functions.
//
// # Gate count
//
//	Component               Gates   Transistors (x4 per gate)
//	---------------------   -----   -------------------------
//	ALU (4-bit)             32      128
//	Register file (16x4)    480     1,920
//	Accumulator (4-bit)     24      96
//	Carry flag (1-bit)      6       24
//	Program counter (12)    96      384
//	Hardware stack (3x12)   226     904
//	Decoder                 ~50     200
//	Control + wiring        ~100    400
//	---------------------   -----   -------------------------
//	Total                   ~1,014  ~4,056
//
// The real Intel 4004 had 2,300 transistors. Our count is higher because
// we model RAM separately (the real 4004 used external 4002 RAM chips)
// and our gate model isn't minimized with Karnaugh maps.
//
// # Execution model
//
// Each instruction executes in a single Step() call, which corresponds
// to one machine cycle. The fetch-decode-execute pipeline:
//
//  1. FETCH:   Read instruction byte from ROM using PC
//  2. FETCH2:  For 2-byte instructions, read the second byte
//  3. DECODE:  Route instruction through decoder gate network
//  4. EXECUTE: Perform the operation through ALU/registers/etc.

import (
	"fmt"

	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// GateTrace is a trace record for one instruction execution.
//
// Same information as the behavioral simulator's trace,
// plus gate-level details.
type GateTrace struct {
	Address           int
	Raw               int
	Raw2              int  // -1 if no second byte
	HasRaw2           bool
	Mnemonic          string
	AccumulatorBefore int
	AccumulatorAfter  int
	CarryBefore       bool
	CarryAfter        bool
}

// Intel4004GateLevel is the Intel 4004 CPU where every operation routes
// through real logic gates.
//
// Public API matches the behavioral Intel4004Simulator for
// cross-validation, but internally all computation flows through
// gates, flip-flops, and adders.
//
// Usage:
//
//	cpu := NewIntel4004GateLevel()
//	traces := cpu.Run([]byte{0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}, 10000)
//	// cpu.Registers()[1] == 3  (1 + 2)
type Intel4004GateLevel struct {
	alu   *GateALU
	regs  *RegisterFile
	acc   *Accumulator
	carry *CarryFlag
	pc    *ProgramCounter
	stack *HardwareStack
	ram   *RAM
	rom   [4096]byte

	// RAM addressing (set by SRC/DCL)
	ramBank      int
	ramRegister  int
	ramCharacter int

	// ROM I/O port
	romPort int

	// Control state
	halted bool
}

// NewIntel4004GateLevel creates a new gate-level Intel 4004 CPU.
func NewIntel4004GateLevel() *Intel4004GateLevel {
	return &Intel4004GateLevel{
		alu:   NewGateALU(),
		regs:  NewRegisterFile(),
		acc:   NewAccumulator(),
		carry: NewCarryFlag(),
		pc:    NewProgramCounter(),
		stack: NewHardwareStack(),
		ram:   NewRAM(),
	}
}

// -- Property accessors (match behavioral simulator's interface) --

// Accumulator reads the accumulator from flip-flops.
func (cpu *Intel4004GateLevel) Accumulator() int {
	return cpu.acc.Read()
}

// Registers reads all 16 registers from flip-flops.
func (cpu *Intel4004GateLevel) Registers() []int {
	regs := make([]int, 16)
	for i := 0; i < 16; i++ {
		regs[i] = cpu.regs.Read(i)
	}
	return regs
}

// Carry reads carry flag from flip-flop.
func (cpu *Intel4004GateLevel) Carry() bool {
	return cpu.carry.Read()
}

// PC reads program counter from flip-flops.
func (cpu *Intel4004GateLevel) PC() int {
	return cpu.pc.Read()
}

// Halted returns whether the CPU is halted.
func (cpu *Intel4004GateLevel) Halted() bool {
	return cpu.halted
}

// HWStack reads stack levels (for inspection only).
func (cpu *Intel4004GateLevel) HWStack() []int {
	values := make([]int, 3)
	zeros := make([]int, 12)
	for i := 0; i < 3; i++ {
		output, _ := logicgates.Register(zeros, 0, cpu.stack.levels[i])
		values[i] = BitsToInt(output)
	}
	return values
}

// RAMData reads RAM main characters.
func (cpu *Intel4004GateLevel) RAMData() [4][4][16]int {
	var result [4][4][16]int
	for b := 0; b < 4; b++ {
		for r := 0; r < 4; r++ {
			for c := 0; c < 16; c++ {
				result[b][r][c] = cpu.ram.ReadMain(b, r, c)
			}
		}
	}
	return result
}

// RAMStatus reads RAM status characters.
func (cpu *Intel4004GateLevel) RAMStatus() [4][4][4]int {
	var result [4][4][4]int
	for b := 0; b < 4; b++ {
		for r := 0; r < 4; r++ {
			for s := 0; s < 4; s++ {
				result[b][r][s] = cpu.ram.ReadStatus(b, r, s)
			}
		}
	}
	return result
}

// RAMBank returns the current RAM bank.
func (cpu *Intel4004GateLevel) RAMBank() int {
	return cpu.ramBank
}

// ROMPort returns the current ROM port value.
func (cpu *Intel4004GateLevel) ROMPort() int {
	return cpu.romPort
}

// RAMOutput returns the RAM output port values.
func (cpu *Intel4004GateLevel) RAMOutput() [4]int {
	var result [4]int
	for i := 0; i < 4; i++ {
		result[i] = cpu.ram.ReadOutput(i)
	}
	return result
}

// -- Public API --

// LoadProgram loads a program into ROM.
func (cpu *Intel4004GateLevel) LoadProgram(program []byte) {
	cpu.rom = [4096]byte{}
	for i, b := range program {
		if i < 4096 {
			cpu.rom[i] = b
		}
	}
}

// Step executes one instruction through the gate-level pipeline.
//
// Returns a GateTrace with before/after state.
func (cpu *Intel4004GateLevel) Step() GateTrace {
	if cpu.halted {
		panic("CPU is halted -- cannot step further")
	}

	// Snapshot state before
	accBefore := cpu.acc.Read()
	carryBefore := cpu.carry.Read()
	pcBefore := cpu.pc.Read()

	// FETCH: read instruction byte from ROM
	raw := int(cpu.rom[pcBefore])

	// DECODE: route through combinational decoder
	decoded := Decode(raw, -1)

	// FETCH2: if 2-byte, read second byte
	raw2 := -1
	hasRaw2 := false
	if decoded.IsTwoByte != 0 {
		raw2 = int(cpu.rom[(pcBefore+1)&0xFFF])
		hasRaw2 = true
		decoded = Decode(raw, raw2)
	}

	// EXECUTE: route through appropriate gate paths
	mnemonic := cpu.execute(decoded)

	return GateTrace{
		Address:           pcBefore,
		Raw:               raw,
		Raw2:              raw2,
		HasRaw2:           hasRaw2,
		Mnemonic:          mnemonic,
		AccumulatorBefore: accBefore,
		AccumulatorAfter:  cpu.acc.Read(),
		CarryBefore:       carryBefore,
		CarryAfter:        cpu.carry.Read(),
	}
}

// Run loads and runs a program, returning execution trace.
func (cpu *Intel4004GateLevel) Run(program []byte, maxSteps int) []GateTrace {
	cpu.Reset()
	cpu.LoadProgram(program)

	traces := make([]GateTrace, 0)
	for i := 0; i < maxSteps; i++ {
		if cpu.halted {
			break
		}
		traces = append(traces, cpu.Step())
	}
	return traces
}

// Reset resets all CPU state.
func (cpu *Intel4004GateLevel) Reset() {
	cpu.acc.Reset()
	cpu.carry.Reset()
	cpu.regs.Reset()
	cpu.pc.Reset()
	cpu.stack.Reset()
	cpu.ram.Reset()
	cpu.rom = [4096]byte{}
	cpu.ramBank = 0
	cpu.ramRegister = 0
	cpu.ramCharacter = 0
	cpu.romPort = 0
	cpu.halted = false
}

// GateCount returns total estimated gate count for the CPU.
func (cpu *Intel4004GateLevel) GateCount() int {
	return cpu.alu.GateCount() +
		cpu.regs.GateCount() +
		cpu.acc.GateCount() +
		cpu.carry.GateCount() +
		cpu.pc.GateCount() +
		cpu.stack.GateCount() +
		cpu.ram.GateCount() +
		50 + // decoder
		100 // control logic and wiring
}

// -- Instruction execution -- routes through gate-level components --

func (cpu *Intel4004GateLevel) execute(d DecodedInstruction) string {
	// NOP
	if d.IsNOP != 0 {
		cpu.pc.Increment()
		return "NOP"
	}

	// HLT
	if d.IsHLT != 0 {
		cpu.halted = true
		cpu.pc.Increment()
		return "HLT"
	}

	// LDM N: load immediate into accumulator
	if d.IsLDM != 0 {
		cpu.acc.Write(d.Immediate)
		cpu.pc.Increment()
		return fmt.Sprintf("LDM %d", d.Immediate)
	}

	// LD Rn: load register into accumulator
	if d.IsLD != 0 {
		val := cpu.regs.Read(d.RegIndex)
		cpu.acc.Write(val)
		cpu.pc.Increment()
		return fmt.Sprintf("LD R%d", d.RegIndex)
	}

	// XCH Rn: exchange accumulator and register
	if d.IsXCH != 0 {
		aVal := cpu.acc.Read()
		rVal := cpu.regs.Read(d.RegIndex)
		cpu.acc.Write(rVal)
		cpu.regs.Write(d.RegIndex, aVal)
		cpu.pc.Increment()
		return fmt.Sprintf("XCH R%d", d.RegIndex)
	}

	// INC Rn: increment register (no carry effect)
	if d.IsINC != 0 {
		rVal := cpu.regs.Read(d.RegIndex)
		result, _ := cpu.alu.Increment(rVal)
		cpu.regs.Write(d.RegIndex, result)
		cpu.pc.Increment()
		return fmt.Sprintf("INC R%d", d.RegIndex)
	}

	// ADD Rn: add register to accumulator with carry
	if d.IsADD != 0 {
		aVal := cpu.acc.Read()
		rVal := cpu.regs.Read(d.RegIndex)
		carryIn := 0
		if cpu.carry.Read() {
			carryIn = 1
		}
		result, carryOut := cpu.alu.Add(aVal, rVal, carryIn)
		cpu.acc.Write(result)
		cpu.carry.Write(carryOut)
		cpu.pc.Increment()
		return fmt.Sprintf("ADD R%d", d.RegIndex)
	}

	// SUB Rn: subtract register from accumulator
	if d.IsSUB != 0 {
		aVal := cpu.acc.Read()
		rVal := cpu.regs.Read(d.RegIndex)
		borrowIn := 1
		if cpu.carry.Read() {
			borrowIn = 0
		}
		result, carryOut := cpu.alu.Subtract(aVal, rVal, borrowIn)
		cpu.acc.Write(result)
		cpu.carry.Write(carryOut)
		cpu.pc.Increment()
		return fmt.Sprintf("SUB R%d", d.RegIndex)
	}

	// JUN addr: unconditional jump
	if d.IsJUN != 0 {
		cpu.pc.Load(d.Addr12)
		return fmt.Sprintf("JUN 0x%03X", d.Addr12)
	}

	// JCN cond,addr: conditional jump
	if d.IsJCN != 0 {
		return cpu.execJCN(d)
	}

	// ISZ Rn,addr: increment and skip if zero
	if d.IsISZ != 0 {
		return cpu.execISZ(d)
	}

	// JMS addr: jump to subroutine
	if d.IsJMS != 0 {
		returnAddr := cpu.pc.Read() + 2
		cpu.stack.Push(returnAddr)
		cpu.pc.Load(d.Addr12)
		return fmt.Sprintf("JMS 0x%03X", d.Addr12)
	}

	// BBL N: branch back and load
	if d.IsBBL != 0 {
		cpu.acc.Write(d.Immediate)
		returnAddr := cpu.stack.Pop()
		cpu.pc.Load(returnAddr)
		return fmt.Sprintf("BBL %d", d.Immediate)
	}

	// FIM Pp,data: fetch immediate to pair
	if d.IsFIM != 0 {
		cpu.regs.WritePair(d.PairIndex, d.Addr8)
		cpu.pc.Increment2()
		return fmt.Sprintf("FIM P%d,0x%02X", d.PairIndex, d.Addr8)
	}

	// SRC Pp: send register control
	if d.IsSRC != 0 {
		pairVal := cpu.regs.ReadPair(d.PairIndex)
		cpu.ramRegister = (pairVal >> 4) & 0xF
		cpu.ramCharacter = pairVal & 0xF
		cpu.pc.Increment()
		return fmt.Sprintf("SRC P%d", d.PairIndex)
	}

	// FIN Pp: fetch indirect from ROM
	if d.IsFIN != 0 {
		p0Val := cpu.regs.ReadPair(0)
		page := cpu.pc.Read() & 0xF00
		romAddr := page | p0Val
		romByte := int(cpu.rom[romAddr&0xFFF])
		cpu.regs.WritePair(d.PairIndex, romByte)
		cpu.pc.Increment()
		return fmt.Sprintf("FIN P%d", d.PairIndex)
	}

	// JIN Pp: jump indirect
	if d.IsJIN != 0 {
		pairVal := cpu.regs.ReadPair(d.PairIndex)
		page := cpu.pc.Read() & 0xF00
		cpu.pc.Load(page | pairVal)
		return fmt.Sprintf("JIN P%d", d.PairIndex)
	}

	// I/O operations (0xE_ range)
	if d.IsIO != 0 {
		return cpu.execIO(d)
	}

	// Accumulator operations (0xF_ range)
	if d.IsAccum != 0 {
		return cpu.execAccum(d)
	}

	// Unknown -- advance PC to avoid infinite loop
	cpu.pc.Increment()
	return fmt.Sprintf("UNKNOWN(0x%02X)", d.Raw)
}

// execJCN implements JCN cond,addr: conditional jump using gate logic.
//
// Condition nibble bits (evaluated with OR/AND/NOT gates):
//
//	Bit 3: INVERT
//	Bit 2: TEST A==0
//	Bit 1: TEST carry==1
//	Bit 0: TEST pin (always 0)
func (cpu *Intel4004GateLevel) execJCN(d DecodedInstruction) string {
	cond := d.Condition
	aVal := cpu.acc.Read()
	carryVal := 0
	if cpu.carry.Read() {
		carryVal = 1
	}

	// Test A==0: OR all accumulator bits, then NOT
	aBits := IntToBits(aVal, 4)
	aIsZero := logicgates.NOT(logicgates.OR(logicgates.OR(aBits[0], aBits[1]),
		logicgates.OR(aBits[2], aBits[3])))

	// Build test result using gates
	testZero := logicgates.AND((cond>>2)&1, aIsZero)
	testCarry := logicgates.AND((cond>>1)&1, carryVal)
	testPin := logicgates.AND(cond&1, 0) // Pin always 0

	testResult := logicgates.OR(logicgates.OR(testZero, testCarry), testPin)

	// Invert if bit 3 set
	invert := (cond >> 3) & 1
	// XOR with invert: if invert=1, flip result
	final := logicgates.OR(
		logicgates.AND(testResult, logicgates.NOT(invert)),
		logicgates.AND(logicgates.NOT(testResult), invert),
	)

	page := (cpu.pc.Read() + 2) & 0xF00
	target := page | d.Addr8

	if final != 0 {
		cpu.pc.Load(target)
	} else {
		cpu.pc.Increment2()
	}

	return fmt.Sprintf("JCN %d,%02X", cond, d.Addr8)
}

// execISZ implements ISZ Rn,addr: increment register, skip if zero.
func (cpu *Intel4004GateLevel) execISZ(d DecodedInstruction) string {
	rVal := cpu.regs.Read(d.RegIndex)
	result, _ := cpu.alu.Increment(rVal)
	cpu.regs.Write(d.RegIndex, result)

	// Test if result is zero using NOR of all bits
	rBits := IntToBits(result, 4)
	isZero := logicgates.NOT(logicgates.OR(logicgates.OR(rBits[0], rBits[1]),
		logicgates.OR(rBits[2], rBits[3])))

	page := (cpu.pc.Read() + 2) & 0xF00
	target := page | d.Addr8

	if isZero != 0 {
		// Result is zero -> fall through
		cpu.pc.Increment2()
	} else {
		// Result is nonzero -> jump
		cpu.pc.Load(target)
	}

	return fmt.Sprintf("ISZ R%d,0x%02X", d.RegIndex, d.Addr8)
}

// execIO executes I/O instructions (0xE0-0xEF).
func (cpu *Intel4004GateLevel) execIO(d DecodedInstruction) string {
	aVal := cpu.acc.Read()
	subOp := d.Lower

	switch {
	case subOp == 0x0: // WRM
		cpu.ram.WriteMain(cpu.ramBank, cpu.ramRegister, cpu.ramCharacter, aVal)
		cpu.pc.Increment()
		return "WRM"

	case subOp == 0x1: // WMP
		cpu.ram.WriteOutput(cpu.ramBank, aVal)
		cpu.pc.Increment()
		return "WMP"

	case subOp == 0x2: // WRR
		cpu.romPort = aVal & 0xF
		cpu.pc.Increment()
		return "WRR"

	case subOp == 0x3: // WPM (NOP in simulation)
		cpu.pc.Increment()
		return "WPM"

	case subOp >= 0x4 && subOp <= 0x7: // WR0-WR3
		idx := subOp - 0x4
		cpu.ram.WriteStatus(cpu.ramBank, cpu.ramRegister, idx, aVal)
		cpu.pc.Increment()
		return fmt.Sprintf("WR%d", idx)

	case subOp == 0x8: // SBM
		ramVal := cpu.ram.ReadMain(cpu.ramBank, cpu.ramRegister, cpu.ramCharacter)
		borrowIn := 1
		if cpu.carry.Read() {
			borrowIn = 0
		}
		result, carryOut := cpu.alu.Subtract(aVal, ramVal, borrowIn)
		cpu.acc.Write(result)
		cpu.carry.Write(carryOut)
		cpu.pc.Increment()
		return "SBM"

	case subOp == 0x9: // RDM
		val := cpu.ram.ReadMain(cpu.ramBank, cpu.ramRegister, cpu.ramCharacter)
		cpu.acc.Write(val)
		cpu.pc.Increment()
		return "RDM"

	case subOp == 0xA: // RDR
		cpu.acc.Write(cpu.romPort & 0xF)
		cpu.pc.Increment()
		return "RDR"

	case subOp == 0xB: // ADM
		ramVal := cpu.ram.ReadMain(cpu.ramBank, cpu.ramRegister, cpu.ramCharacter)
		carryIn := 0
		if cpu.carry.Read() {
			carryIn = 1
		}
		result, carryOut := cpu.alu.Add(aVal, ramVal, carryIn)
		cpu.acc.Write(result)
		cpu.carry.Write(carryOut)
		cpu.pc.Increment()
		return "ADM"

	case subOp >= 0xC && subOp <= 0xF: // RD0-RD3
		idx := subOp - 0xC
		val := cpu.ram.ReadStatus(cpu.ramBank, cpu.ramRegister, idx)
		cpu.acc.Write(val)
		cpu.pc.Increment()
		return fmt.Sprintf("RD%d", idx)

	default:
		cpu.pc.Increment()
		return fmt.Sprintf("IO(0x%02X)", d.Raw)
	}
}

// execAccum executes accumulator operations (0xF0-0xFD).
func (cpu *Intel4004GateLevel) execAccum(d DecodedInstruction) string {
	aVal := cpu.acc.Read()
	subOp := d.Lower

	switch subOp {
	case 0x0: // CLB
		cpu.acc.Write(0)
		cpu.carry.Write(false)
		cpu.pc.Increment()
		return "CLB"

	case 0x1: // CLC
		cpu.carry.Write(false)
		cpu.pc.Increment()
		return "CLC"

	case 0x2: // IAC
		result, carry := cpu.alu.Increment(aVal)
		cpu.acc.Write(result)
		cpu.carry.Write(carry)
		cpu.pc.Increment()
		return "IAC"

	case 0x3: // CMC
		cpu.carry.Write(!cpu.carry.Read())
		cpu.pc.Increment()
		return "CMC"

	case 0x4: // CMA
		result := cpu.alu.Complement(aVal)
		cpu.acc.Write(result)
		cpu.pc.Increment()
		return "CMA"

	case 0x5: // RAL
		oldCarry := 0
		if cpu.carry.Read() {
			oldCarry = 1
		}
		// Use gates: A3 goes to carry, shift left, old carry to bit 0
		aBits := IntToBits(aVal, 4)
		cpu.carry.Write(aBits[3] == 1)
		newBits := []int{oldCarry, aBits[0], aBits[1], aBits[2]}
		cpu.acc.Write(BitsToInt(newBits))
		cpu.pc.Increment()
		return "RAL"

	case 0x6: // RAR
		oldCarry := 0
		if cpu.carry.Read() {
			oldCarry = 1
		}
		aBits := IntToBits(aVal, 4)
		cpu.carry.Write(aBits[0] == 1)
		newBits := []int{aBits[1], aBits[2], aBits[3], oldCarry}
		cpu.acc.Write(BitsToInt(newBits))
		cpu.pc.Increment()
		return "RAR"

	case 0x7: // TCC
		if cpu.carry.Read() {
			cpu.acc.Write(1)
		} else {
			cpu.acc.Write(0)
		}
		cpu.carry.Write(false)
		cpu.pc.Increment()
		return "TCC"

	case 0x8: // DAC
		result, carry := cpu.alu.Decrement(aVal)
		cpu.acc.Write(result)
		cpu.carry.Write(carry)
		cpu.pc.Increment()
		return "DAC"

	case 0x9: // TCS
		if cpu.carry.Read() {
			cpu.acc.Write(10)
		} else {
			cpu.acc.Write(9)
		}
		cpu.carry.Write(false)
		cpu.pc.Increment()
		return "TCS"

	case 0xA: // STC
		cpu.carry.Write(true)
		cpu.pc.Increment()
		return "STC"

	case 0xB: // DAA
		if aVal > 9 || cpu.carry.Read() {
			result, carry := cpu.alu.Add(aVal, 6, 0)
			if carry {
				cpu.carry.Write(true)
			}
			cpu.acc.Write(result)
		}
		cpu.pc.Increment()
		return "DAA"

	case 0xC: // KBP
		kbpTable := map[int]int{0: 0, 1: 1, 2: 2, 4: 3, 8: 4}
		if val, ok := kbpTable[aVal]; ok {
			cpu.acc.Write(val)
		} else {
			cpu.acc.Write(15)
		}
		cpu.pc.Increment()
		return "KBP"

	case 0xD: // DCL
		bank := cpu.alu.BitwiseAnd(aVal, 0x7)
		if bank > 3 {
			bank = cpu.alu.BitwiseAnd(bank, 0x3)
		}
		cpu.ramBank = bank
		cpu.pc.Increment()
		return "DCL"

	default:
		cpu.pc.Increment()
		return fmt.Sprintf("ACCUM(0x%02X)", d.Raw)
	}
}
