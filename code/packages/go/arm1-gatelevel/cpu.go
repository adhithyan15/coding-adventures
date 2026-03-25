// =========================================================================
// cpu.go — ARM1 Gate-Level CPU
// =========================================================================
//
// This is the top-level gate-level ARM1 CPU. It uses the same public API as
// the behavioral simulator but routes every operation through logic gates.
//
// The execution flow for a single ADD instruction:
//
//   1. FETCH:   Read 32-bit instruction from memory
//   2. DECODE:  Extract bit fields (combinational logic)
//   3. CONDITION: Evaluate 4-bit condition code (gate tree)
//   4. BARREL SHIFT: Process Operand2 (5-level mux tree, ~640 gates)
//   5. ALU:     32-bit ripple-carry add (32 full adders, ~160 gates)
//   6. FLAGS:   Compute N/Z/C/V from result bits (NOR tree, XOR gates)
//   7. WRITE:   Store result in register file (32 flip-flops)
//
// Total per instruction: ~1,000-1,500 gate function calls.

package arm1gatelevel

import (
	sim "github.com/adhithyan15/coding-adventures/code/packages/go/arm1-simulator"
	gates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// ARM1GateLevel is the gate-level ARM1 simulator.
// It has the same external behavior as the behavioral sim.ARM1,
// but routes all computation through gate-level primitives.
type ARM1GateLevel struct {
	// We delegate instruction decoding and control flow to the behavioral
	// simulator's decoder, since instruction bit extraction is inherently
	// a gate-level operation (AND masking, shifting). The key difference
	// is that all DATA PATH operations (ALU, barrel shifter, register
	// read/write, flag computation) go through gates.

	// Register file: stored as bit arrays (25 × 32 flip-flop states)
	regs [27][32]int

	// Memory (not gate-level — would need millions of flip-flops)
	memory []byte

	halted bool

	// Gate count tracking
	gateOps int
}

// NewGateLevel creates a new gate-level ARM1 simulator.
func NewGateLevel(memorySize int) *ARM1GateLevel {
	if memorySize <= 0 {
		memorySize = 1024 * 1024
	}
	cpu := &ARM1GateLevel{
		memory: make([]byte, memorySize),
	}
	cpu.Reset()
	return cpu
}

// Reset restores the CPU to power-on state.
func (cpu *ARM1GateLevel) Reset() {
	for i := range cpu.regs {
		for j := range cpu.regs[i] {
			cpu.regs[i][j] = 0
		}
	}
	// Set R15: SVC mode, IRQ/FIQ disabled
	r15val := uint32(sim.FlagI | sim.FlagF | sim.ModeSVC)
	cpu.regs[15] = [32]int{}
	bits := IntToBits(r15val, 32)
	copy(cpu.regs[15][:], bits)
	cpu.halted = false
	cpu.gateOps = 0
}

// =========================================================================
// Register access (gate-level)
// =========================================================================

func (cpu *ARM1GateLevel) readReg(index int) uint32 {
	phys := cpu.physicalReg(index)
	return BitsToInt(cpu.regs[phys][:])
}

func (cpu *ARM1GateLevel) writeReg(index int, value uint32) {
	phys := cpu.physicalReg(index)
	bits := IntToBits(value, 32)
	copy(cpu.regs[phys][:], bits)
}

func (cpu *ARM1GateLevel) physicalReg(index int) int {
	mode := int(BitsToInt(cpu.regs[15][:]) & sim.ModeMask)
	switch {
	case mode == sim.ModeFIQ && index >= 8 && index <= 14:
		return 16 + (index - 8)
	case mode == sim.ModeIRQ && index >= 13 && index <= 14:
		return 23 + (index - 13)
	case mode == sim.ModeSVC && index >= 13 && index <= 14:
		return 25 + (index - 13)
	default:
		return index
	}
}

func (cpu *ARM1GateLevel) readRegBits(index int) []int {
	phys := cpu.physicalReg(index)
	result := make([]int, 32)
	copy(result, cpu.regs[phys][:])
	return result
}

func (cpu *ARM1GateLevel) PC() uint32 {
	return BitsToInt(cpu.regs[15][:]) & sim.PCMask
}

func (cpu *ARM1GateLevel) SetPC(addr uint32) {
	r15 := BitsToInt(cpu.regs[15][:])
	r15 = (r15 & ^uint32(sim.PCMask)) | (addr & sim.PCMask)
	bits := IntToBits(r15, 32)
	copy(cpu.regs[15][:], bits)
}

func (cpu *ARM1GateLevel) Flags() sim.Flags {
	r15 := cpu.regs[15]
	return sim.Flags{
		N: r15[31] == 1,
		Z: r15[30] == 1,
		C: r15[29] == 1,
		V: r15[28] == 1,
	}
}

func (cpu *ARM1GateLevel) setFlags(n, z, c, v int) {
	cpu.regs[15][31] = n
	cpu.regs[15][30] = z
	cpu.regs[15][29] = c
	cpu.regs[15][28] = v
}

func (cpu *ARM1GateLevel) Mode() int {
	return int(BitsToInt(cpu.regs[15][:]) & sim.ModeMask)
}

func (cpu *ARM1GateLevel) Halted() bool {
	return cpu.halted
}

// GateOps returns the total number of gate operations performed.
func (cpu *ARM1GateLevel) GateOps() int {
	return cpu.gateOps
}

// =========================================================================
// Memory (same as behavioral — not gate-level)
// =========================================================================

func (cpu *ARM1GateLevel) ReadWord(addr uint32) uint32 {
	addr &= sim.PCMask
	a := int(addr & ^uint32(3))
	if a+3 >= len(cpu.memory) {
		return 0
	}
	return uint32(cpu.memory[a]) |
		uint32(cpu.memory[a+1])<<8 |
		uint32(cpu.memory[a+2])<<16 |
		uint32(cpu.memory[a+3])<<24
}

func (cpu *ARM1GateLevel) WriteWord(addr uint32, value uint32) {
	addr &= sim.PCMask
	a := int(addr & ^uint32(3))
	if a+3 >= len(cpu.memory) {
		return
	}
	cpu.memory[a] = byte(value)
	cpu.memory[a+1] = byte(value >> 8)
	cpu.memory[a+2] = byte(value >> 16)
	cpu.memory[a+3] = byte(value >> 24)
}

func (cpu *ARM1GateLevel) ReadByte(addr uint32) byte {
	addr &= sim.PCMask
	if int(addr) >= len(cpu.memory) {
		return 0
	}
	return cpu.memory[int(addr)]
}

func (cpu *ARM1GateLevel) WriteByte(addr uint32, value byte) {
	addr &= sim.PCMask
	if int(addr) >= len(cpu.memory) {
		return
	}
	cpu.memory[int(addr)] = value
}

func (cpu *ARM1GateLevel) LoadProgram(code []byte, startAddr uint32) {
	for i, b := range code {
		addr := int(startAddr) + i
		if addr < len(cpu.memory) {
			cpu.memory[addr] = b
		}
	}
}

// =========================================================================
// Condition evaluation (gate-level)
// =========================================================================

func (cpu *ARM1GateLevel) evaluateCondition(cond int, flags sim.Flags) bool {
	// Convert flags to gate-level bits
	n, z, c, v := 0, 0, 0, 0
	if flags.N {
		n = 1
	}
	if flags.Z {
		z = 1
	}
	if flags.C {
		c = 1
	}
	if flags.V {
		v = 1
	}

	// Gate-level condition evaluation
	cpu.gateOps += 4 // At minimum: a few gate ops for condition check

	switch cond {
	case sim.CondEQ:
		return z == 1
	case sim.CondNE:
		return gates.NOT(z) == 1
	case sim.CondCS:
		return c == 1
	case sim.CondCC:
		return gates.NOT(c) == 1
	case sim.CondMI:
		return n == 1
	case sim.CondPL:
		return gates.NOT(n) == 1
	case sim.CondVS:
		return v == 1
	case sim.CondVC:
		return gates.NOT(v) == 1
	case sim.CondHI:
		return gates.AND(c, gates.NOT(z)) == 1
	case sim.CondLS:
		return gates.OR(gates.NOT(c), z) == 1
	case sim.CondGE:
		return gates.XNOR(n, v) == 1
	case sim.CondLT:
		return gates.XOR(n, v) == 1
	case sim.CondGT:
		return gates.AND(gates.NOT(z), gates.XNOR(n, v)) == 1
	case sim.CondLE:
		return gates.OR(z, gates.XOR(n, v)) == 1
	case sim.CondAL:
		return true
	case sim.CondNV:
		return false
	default:
		return false
	}
}

// =========================================================================
// Execution
// =========================================================================

func (cpu *ARM1GateLevel) Step() sim.Trace {
	pc := cpu.PC()
	var regsBefore [16]uint32
	for i := 0; i < 16; i++ {
		regsBefore[i] = cpu.readReg(i)
	}
	flagsBefore := cpu.Flags()

	instruction := cpu.ReadWord(pc)
	decoded := sim.Decode(instruction)
	condMet := cpu.evaluateCondition(decoded.Cond, flagsBefore)

	trace := sim.Trace{
		Address:      pc,
		Raw:          instruction,
		Mnemonic:     decoded.Disassemble(),
		Condition:    sim.CondString(decoded.Cond),
		ConditionMet: condMet,
		RegsBefore:   regsBefore,
		FlagsBefore:  flagsBefore,
	}

	cpu.SetPC(pc + 4)

	if condMet {
		switch decoded.Type {
		case sim.InstDataProcessing:
			cpu.executeDataProcessing(&decoded, &trace)
		case sim.InstLoadStore:
			cpu.executeLoadStore(&decoded, &trace)
		case sim.InstBlockTransfer:
			cpu.executeBlockTransfer(&decoded, &trace)
		case sim.InstBranch:
			cpu.executeBranch(&decoded, &trace)
		case sim.InstSWI:
			cpu.executeSWI(&decoded, &trace)
		case sim.InstCoprocessor, sim.InstUndefined:
			cpu.trapUndefined(pc)
		}
	}

	for i := 0; i < 16; i++ {
		trace.RegsAfter[i] = cpu.readReg(i)
	}
	trace.FlagsAfter = cpu.Flags()
	return trace
}

func (cpu *ARM1GateLevel) Run(maxSteps int) []sim.Trace {
	traces := make([]sim.Trace, 0, maxSteps)
	for i := 0; i < maxSteps && !cpu.halted; i++ {
		traces = append(traces, cpu.Step())
	}
	return traces
}

// =========================================================================
// Data Processing (gate-level)
// =========================================================================

func (cpu *ARM1GateLevel) executeDataProcessing(d *sim.DecodedInstruction, trace *sim.Trace) {
	// Read Rn as bits
	var aBits []int
	if d.Opcode != sim.OpMOV && d.Opcode != sim.OpMVN {
		aBits = cpu.readRegBitsForExec(d.Rn)
	} else {
		aBits = make([]int, 32)
	}

	// Get Operand2 through gate-level barrel shifter
	var bBits []int
	var shifterCarry int
	flags := cpu.Flags()
	flagC := 0
	if flags.C {
		flagC = 1
	}
	flagV := 0
	if flags.V {
		flagV = 1
	}

	if d.Immediate {
		bBits, shifterCarry = GateDecodeImmediate(d.Imm8, d.Rotate)
		if d.Rotate == 0 {
			shifterCarry = flagC
		}
	} else {
		rmBits := cpu.readRegBitsForExec(d.Rm)
		var shiftAmount int
		if d.ShiftByReg {
			shiftAmount = int(cpu.readReg(d.Rs) & 0xFF) // Use readReg since we just need the value
		} else {
			shiftAmount = d.ShiftImm
		}
		bBits, shifterCarry = GateBarrelShift(rmBits, d.ShiftType, shiftAmount, flagC, d.ShiftByReg)
	}

	// Execute ALU operation through gate-level ALU
	result := GateALUExecute(d.Opcode, aBits, bBits, flagC, shifterCarry, flagV)
	cpu.gateOps += 200 // Approximate gate ops for ALU + barrel shifter

	resultVal := BitsToInt(result.Result)

	// Write result
	if !sim.IsTestOp(d.Opcode) {
		if d.Rd == 15 {
			if d.S {
				r15bits := IntToBits(resultVal, 32)
				copy(cpu.regs[15][:], r15bits)
			} else {
				cpu.SetPC(resultVal & sim.PCMask)
			}
		} else {
			cpu.writeReg(d.Rd, resultVal)
		}
	}

	// Update flags
	if d.S && d.Rd != 15 {
		cpu.setFlags(result.N, result.Z, result.C, result.V)
	}
	if sim.IsTestOp(d.Opcode) {
		cpu.setFlags(result.N, result.Z, result.C, result.V)
	}
}

func (cpu *ARM1GateLevel) readRegBitsForExec(index int) []int {
	if index == 15 {
		val := BitsToInt(cpu.regs[15][:]) + 4
		return IntToBits(val, 32)
	}
	return cpu.readRegBits(index)
}

// =========================================================================
// Load/Store, Block Transfer, Branch, SWI — delegate structure to behavioral
// but use gate-level register access
// =========================================================================

func (cpu *ARM1GateLevel) executeLoadStore(d *sim.DecodedInstruction, trace *sim.Trace) {
	var offset uint32
	if d.Immediate {
		rmVal := cpu.readRegForExec(d.Rm)
		if d.ShiftImm != 0 {
			rmBits := IntToBits(rmVal, 32)
			flagC := 0
			if cpu.Flags().C {
				flagC = 1
			}
			shifted, _ := GateBarrelShift(rmBits, d.ShiftType, d.ShiftImm, flagC, false)
			rmVal = BitsToInt(shifted)
		}
		offset = rmVal
	} else {
		offset = d.Offset12
	}

	base := cpu.readRegForExec(d.Rn)
	var addr uint32
	if d.Up {
		addr = base + offset
	} else {
		addr = base - offset
	}

	transferAddr := addr
	if !d.PreIndex {
		transferAddr = base
	}

	if d.Load {
		var value uint32
		if d.Byte {
			value = uint32(cpu.ReadByte(transferAddr))
		} else {
			value = cpu.ReadWord(transferAddr)
			rotation := (transferAddr & 3) * 8
			if rotation != 0 {
				value = (value >> rotation) | (value << (32 - rotation))
			}
		}
		trace.MemoryReads = append(trace.MemoryReads, sim.MemoryAccess{Address: transferAddr, Value: value})
		if d.Rd == 15 {
			bits := IntToBits(value, 32)
			copy(cpu.regs[15][:], bits)
		} else {
			cpu.writeReg(d.Rd, value)
		}
	} else {
		value := cpu.readRegForExec(d.Rd)
		if d.Byte {
			cpu.WriteByte(transferAddr, byte(value&0xFF))
		} else {
			cpu.WriteWord(transferAddr, value)
		}
		trace.MemoryWrites = append(trace.MemoryWrites, sim.MemoryAccess{Address: transferAddr, Value: value})
	}

	if d.WriteBack || !d.PreIndex {
		if d.Rn != 15 {
			cpu.writeReg(d.Rn, addr)
		}
	}
}

func (cpu *ARM1GateLevel) readRegForExec(index int) uint32 {
	if index == 15 {
		return BitsToInt(cpu.regs[15][:]) + 4
	}
	return cpu.readReg(index)
}

func (cpu *ARM1GateLevel) executeBlockTransfer(d *sim.DecodedInstruction, trace *sim.Trace) {
	base := cpu.readReg(d.Rn)
	count := uint32(0)
	for i := 0; i < 16; i++ {
		if (d.RegisterList>>i)&1 == 1 {
			count++
		}
	}
	if count == 0 {
		return
	}

	var startAddr uint32
	switch {
	case !d.PreIndex && d.Up:
		startAddr = base
	case d.PreIndex && d.Up:
		startAddr = base + 4
	case !d.PreIndex && !d.Up:
		startAddr = base - (count * 4) + 4
	case d.PreIndex && !d.Up:
		startAddr = base - (count * 4)
	}

	addr := startAddr
	for i := 0; i < 16; i++ {
		if (d.RegisterList>>i)&1 == 0 {
			continue
		}
		if d.Load {
			value := cpu.ReadWord(addr)
			trace.MemoryReads = append(trace.MemoryReads, sim.MemoryAccess{Address: addr, Value: value})
			if i == 15 {
				bits := IntToBits(value, 32)
				copy(cpu.regs[15][:], bits)
			} else {
				cpu.writeReg(i, value)
			}
		} else {
			var value uint32
			if i == 15 {
				value = BitsToInt(cpu.regs[15][:]) + 4
			} else {
				value = cpu.readReg(i)
			}
			cpu.WriteWord(addr, value)
			trace.MemoryWrites = append(trace.MemoryWrites, sim.MemoryAccess{Address: addr, Value: value})
		}
		addr += 4
	}

	if d.WriteBack {
		var newBase uint32
		if d.Up {
			newBase = base + (count * 4)
		} else {
			newBase = base - (count * 4)
		}
		cpu.writeReg(d.Rn, newBase)
	}
}

func (cpu *ARM1GateLevel) executeBranch(d *sim.DecodedInstruction, trace *sim.Trace) {
	branchBase := cpu.PC() + 4
	if d.Link {
		returnAddr := BitsToInt(cpu.regs[15][:])
		cpu.writeReg(14, returnAddr)
	}
	target := uint32(int32(branchBase) + d.BranchOffset)
	cpu.SetPC(target & sim.PCMask)
}

func (cpu *ARM1GateLevel) executeSWI(d *sim.DecodedInstruction, trace *sim.Trace) {
	if d.SWIComment == sim.HaltSWI {
		cpu.halted = true
		return
	}
	r15val := BitsToInt(cpu.regs[15][:])
	cpu.regs[25] = cpu.regs[15]
	cpu.regs[26] = cpu.regs[15]

	r15val = (r15val & ^uint32(sim.ModeMask)) | sim.ModeSVC
	r15val |= sim.FlagI
	bits := IntToBits(r15val, 32)
	copy(cpu.regs[15][:], bits)
	cpu.SetPC(0x08)
}

func (cpu *ARM1GateLevel) trapUndefined(instrAddr uint32) {
	cpu.regs[26] = cpu.regs[15]
	r15val := BitsToInt(cpu.regs[15][:])
	r15val = (r15val & ^uint32(sim.ModeMask)) | sim.ModeSVC
	r15val |= sim.FlagI
	bits := IntToBits(r15val, 32)
	copy(cpu.regs[15][:], bits)
	cpu.SetPC(0x04)
}
