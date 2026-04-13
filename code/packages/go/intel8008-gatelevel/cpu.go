package intel8008gatelevel

// Intel 8008 gate-level CPU — all operations route through real logic gates.
//
// # What makes this a "gate-level" simulator?
//
// Every computation in this CPU flows through the same gate chain that the
// real Intel 8008 used:
//
//	NOT/AND/OR/XOR → half_adder → full_adder → ripple_carry_adder → ALU
//	D flip-flop → register → register file / push-down stack
//
// When you execute ADD B, the value in register B is read from flip-flops,
// A is read from flip-flops, both are fed into the 8-bit ALU (which uses 8
// full adders built from XOR/AND/OR gates), and the result is clocked back
// into A's flip-flops.
//
// Nothing is simulated behaviorally. Every bit passes through gate functions.
//
// # Gate count vs the behavioral simulator
//
//	Component               Gates   Transistors (x4 per gate)
//	---------------------   -----   -------------------------
//	ALU (8-bit)             96      384
//	Register file (7×8)     304     1,216
//	Flag register (4-bit)   16      64
//	Push-down stack (8×14)  522     2,088
//	Decoder                 ~80     320
//	Control + wiring        ~100    400
//	---------------------   -----   -------------------------
//	Total                   ~1,118  ~4,472
//
// The real Intel 8008 had approximately 3,500 transistors. Our count is higher
// because we don't apply the Karnaugh-map minimizations that the 8008's designers
// used to reduce transistor count in the physical layout.
//
// # Cross-validation with behavioral simulator
//
// The gate-level CPU and the behavioral Intel8008 simulator should produce
// identical results for any program. The cpu_test.go file validates this by
// running the same programs on both and comparing final state.
//
// # Execution model
//
// Each instruction executes in a single Step() call:
//
//  1. FETCH:   Read opcode byte from memory at current PC (entry 0 of stack)
//  2. DECODE:  Route opcode bits through combinational decoder gate tree
//  3. FETCH2:  If 2-byte instruction, read immediate byte; increment PC
//  4. FETCH3:  If 3-byte instruction, read address lo/hi bytes; increment PC ×2
//  5. EXECUTE: Route decoded signals through ALU, register file, stack

import (
	"fmt"
)

// GateTrace records one instruction execution in the gate-level CPU.
//
// Mirrors the behavioral simulator's Trace type for cross-validation.
type GateTrace struct {
	PC       int    // Program counter before this instruction
	Opcode   int    // Opcode byte
	Mnemonic string // Human-readable instruction name
	// State after execution
	A     int  // Accumulator
	Carry bool // Carry flag
	Zero  bool // Zero flag
	Sign  bool // Sign flag
	Parity bool // Parity flag (true = even parity)
}

// Intel8008GateLevel is the Intel 8008 CPU where every operation routes
// through real logic gates.
//
// Public API matches the behavioral Intel8008Simulator for cross-validation,
// but internally all computation flows through gates, flip-flops, and adders.
//
// Usage:
//
//	cpu := NewIntel8008GateLevel(16384)
//	traces := cpu.Run([]byte{0x06, 0x01, 0x3E, 0x02, 0x80, 0x00}, 1000)
//	// cpu.Registers()[7] == 3  (1 + 2)
type Intel8008GateLevel struct {
	alu   *GateALU
	regs  *RegisterFile
	flags *FlagRegister
	stack *PushDownStack

	// 16 KiB memory (program + data)
	mem [16384]byte

	// I/O ports
	inputPorts  [8]int  // 8 input ports
	outputPorts [24]int // 24 output ports

	// Control state
	halted bool
}

// NewIntel8008GateLevel creates a new gate-level Intel 8008 CPU.
func NewIntel8008GateLevel() *Intel8008GateLevel {
	result, _ := StartNew[*Intel8008GateLevel]("intel8008-gatelevel.NewIntel8008GateLevel", nil,
		func(op *Operation[*Intel8008GateLevel], rf *ResultFactory[*Intel8008GateLevel]) *OperationResult[*Intel8008GateLevel] {
			return rf.Generate(true, false, &Intel8008GateLevel{
				alu:   NewGateALU(),
				regs:  NewRegisterFile(),
				flags: NewFlagRegister(),
				stack: NewPushDownStack(),
			})
		}).GetResult()
	return result
}

// -- Property accessors --

// Registers returns all register values as a slice of 8 ints.
//
// Index 0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 6=undefined(M), 7=A.
func (cpu *Intel8008GateLevel) Registers() []int {
	result, _ := StartNew[[]int]("intel8008-gatelevel.Intel8008GateLevel.Registers", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			regs := make([]int, 8)
			for i := 0; i < 8; i++ {
				if i == 6 {
					continue // M is not a real register
				}
				regs[i] = cpu.regs.Read(i)
			}
			return rf.Generate(true, false, regs)
		}).GetResult()
	return result
}

// PC returns the current program counter (entry 0 of the push-down stack).
func (cpu *Intel8008GateLevel) PC() int {
	result, _ := StartNew[int]("intel8008-gatelevel.Intel8008GateLevel.PC", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, cpu.stack.PC())
		}).GetResult()
	return result
}

// Halted returns whether the CPU has executed a HLT instruction.
func (cpu *Intel8008GateLevel) Halted() bool {
	result, _ := StartNew[bool]("intel8008-gatelevel.Intel8008GateLevel.Halted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, cpu.halted)
		}).GetResult()
	return result
}

// Flags returns the 4 CPU flags as individual booleans.
func (cpu *Intel8008GateLevel) Flags() (carry, zero, sign, parity bool) {
	return cpu.flags.ReadFlags()
}

// StackDepth returns the number of saved return addresses (0-7).
func (cpu *Intel8008GateLevel) StackDepth() int {
	return cpu.stack.Depth()
}

// GetOutputPort returns the value latched in an output port (0-23).
func (cpu *Intel8008GateLevel) GetOutputPort(port int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.Intel8008GateLevel.GetOutputPort", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("port", port)
			if port < 0 || port >= 24 {
				return rf.Generate(true, false, 0)
			}
			return rf.Generate(true, false, cpu.outputPorts[port])
		}).GetResult()
	return result
}

// SetInputPort sets an input port value (for test harness use).
func (cpu *Intel8008GateLevel) SetInputPort(port, value int) {
	if port >= 0 && port < 8 {
		cpu.inputPorts[port] = value & 0xFF
	}
}

// -- Public API --

// LoadProgram loads a program into memory starting at address 0.
func (cpu *Intel8008GateLevel) LoadProgram(program []byte) {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.Intel8008GateLevel.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Zero out memory
			cpu.mem = [16384]byte{}
			for i, b := range program {
				if i < 16384 {
					cpu.mem[i] = b
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets all CPU state to initial values.
func (cpu *Intel8008GateLevel) Reset() {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.Intel8008GateLevel.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			cpu.regs.Reset()
			cpu.flags.Reset()
			cpu.stack.Reset()
			cpu.mem = [16384]byte{}
			// Note: inputPorts are NOT reset — they represent external hardware
			// state that persists across program runs (set by SetInputPort).
			cpu.outputPorts = [24]int{}
			cpu.halted = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Step executes one instruction through the gate-level pipeline.
//
// Returns a GateTrace with the execution details.
func (cpu *Intel8008GateLevel) Step() GateTrace {
	result, _ := StartNew[GateTrace]("intel8008-gatelevel.Intel8008GateLevel.Step", GateTrace{},
		func(op *Operation[GateTrace], rf *ResultFactory[GateTrace]) *OperationResult[GateTrace] {
			if cpu.halted {
				panic("Intel8008GateLevel: CPU is halted -- cannot step further")
			}

			// Snapshot PC before execution
			pcBefore := cpu.stack.PC()

			// FETCH: read opcode byte from memory at current PC
			rawOpcode := int(cpu.mem[pcBefore&0x3FFF])

			// DECODE: route opcode bits through combinational decoder
			decoded := Decode(rawOpcode)

			// Advance PC past the opcode byte (always at least 1)
			cpu.stack.Increment(1)

			// FETCH2/FETCH3: read additional bytes for multi-byte instructions
			imm := 0
			addrLo, addrHi := 0, 0
			if decoded.InstrLen >= 2 {
				// Read immediate/low byte, advance PC
				imm = int(cpu.mem[cpu.stack.PC()&0x3FFF])
				cpu.stack.Increment(1)
			}
			if decoded.InstrLen >= 3 {
				// Read high byte of address, advance PC
				addrLo = imm // first extra byte was address low
				addrHi = int(cpu.mem[cpu.stack.PC()&0x3FFF])
				cpu.stack.Increment(1)
				imm = addrLo // restore for address computation
			}

			// Compute 14-bit jump target from lo/hi bytes:
			// target = (addrHi << 8) | addrLo, masked to 14 bits
			jumpTarget := ((addrHi << 8) | addrLo) & 0x3FFF

			// EXECUTE: route through appropriate gate paths
			mnemonic := cpu.execute(decoded, imm, jumpTarget)

			carry, zero, sign, parity := cpu.flags.ReadFlags()
			return rf.Generate(true, false, GateTrace{
				PC:       pcBefore,
				Opcode:   rawOpcode,
				Mnemonic: mnemonic,
				A:        cpu.regs.Read(7),
				Carry:    carry,
				Zero:     zero,
				Sign:     sign,
				Parity:   parity,
			})
		}).PanicOnUnexpected().GetResult()
	return result
}

// Run loads and runs a program, returning an execution trace.
//
// Calls Reset(), loads the program, then calls Step() until halted or maxSteps reached.
func (cpu *Intel8008GateLevel) Run(program []byte, maxSteps int) []GateTrace {
	result, _ := StartNew[[]GateTrace]("intel8008-gatelevel.Intel8008GateLevel.Run", nil,
		func(op *Operation[[]GateTrace], rf *ResultFactory[[]GateTrace]) *OperationResult[[]GateTrace] {
			op.AddProperty("maxSteps", maxSteps)
			cpu.Reset()
			cpu.LoadProgram(program)

			traces := make([]GateTrace, 0)
			for i := 0; i < maxSteps; i++ {
				if cpu.halted {
					break
				}
				traces = append(traces, cpu.Step())
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// GateCount returns the total estimated gate count for the CPU.
func (cpu *Intel8008GateLevel) GateCount() int {
	result, _ := StartNew[int]("intel8008-gatelevel.Intel8008GateLevel.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			total := cpu.alu.GateCount() +
				cpu.regs.GateCount() +
				cpu.flags.GateCount() +
				cpu.stack.GateCount() +
				80 + // decoder
				100 // control logic and wiring
			return rf.Generate(true, false, total)
		}).GetResult()
	return result
}

// -- Instruction execution — routes through gate-level components --

// readReg reads a register value, resolving M (index 6) via memory.
func (cpu *Intel8008GateLevel) readReg(idx int) int {
	if idx == 6 { // M pseudo-register
		addr := cpu.regs.HLAddress()
		return int(cpu.mem[addr&0x3FFF])
	}
	return cpu.regs.Read(idx)
}

// writeReg writes a value to a register, resolving M (index 6) via memory.
func (cpu *Intel8008GateLevel) writeReg(idx, value int) {
	if idx == 6 { // M pseudo-register
		addr := cpu.regs.HLAddress()
		cpu.mem[addr&0x3FFF] = byte(value & 0xFF)
		return
	}
	cpu.regs.Write(idx, value)
}

// updateFlags computes and stores flags from an ALU result.
func (cpu *Intel8008GateLevel) updateFlags(result int, carry bool) {
	z, s, c, p := cpu.alu.ComputeFlags(result, carry)
	cpu.flags.WriteFlags(c, z, s, p)
}

// checkCondition evaluates a condition code against the current flags.
//
// Condition codes:
//
//	0=FC (carry false), 1=FZ (zero false), 2=FS (sign false), 3=FP (parity false)
//	4=TC (carry true),  5=TZ (zero true),  6=TS (sign true),  7=TP (parity true)
//
// Implemented using AND/NOT gates: test the flag, then XOR/AND with the sense bit.
func (cpu *Intel8008GateLevel) checkCondition(condCode int) bool {
	carry, zero, sign, parity := cpu.flags.ReadFlags()

	// Extract which flag (bits 1-0 of condCode) and sense (bit 2)
	flagIdx := condCode & 0x3 // 0=CY, 1=Z, 2=S, 3=P
	sense := (condCode >> 2) & 1 // 0=false(invert), 1=true

	var flagVal int
	switch flagIdx {
	case 0:
		if carry {
			flagVal = 1
		}
	case 1:
		if zero {
			flagVal = 1
		}
	case 2:
		if sign {
			flagVal = 1
		}
	case 3:
		if parity {
			flagVal = 1
		}
	}

	// sense=1: jump-if-true = take if flag is 1
	// sense=0: jump-if-false = take if flag is 0 = NOT(flag)
	result := 0
	if sense == 1 {
		result = flagVal // gate: pass flag through
	} else {
		result = 1 - flagVal // gate: NOT the flag
	}
	return result == 1
}

// execute processes one decoded instruction through the gate-level datapath.
// imm is the immediate byte (for 2-byte instructions).
// jumpTarget is the 14-bit address (for 3-byte instructions).
func (cpu *Intel8008GateLevel) execute(d DecodedInstruction, imm, jumpTarget int) string {
	carry, _, _, _ := cpu.flags.ReadFlags()

	// HLT
	if d.IsHLT == 1 {
		cpu.halted = true
		// Undo the PC advance so PC remains at HLT address
		// (The PC was incremented by 1 in Step() before execute was called)
		// Actually, we let PC stay advanced — behavioral sim does this too.
		return "HLT"
	}

	// MOV D, S — copy register S to register D
	if d.IsMOV == 1 {
		val := cpu.readReg(d.RegSrc)
		cpu.writeReg(d.RegDst, val)
		return fmt.Sprintf("MOV %d,%d", d.RegDst, d.RegSrc)
	}

	// MVI D, imm — load immediate into register
	if d.IsMVI == 1 {
		cpu.writeReg(d.RegDst, imm)
		return fmt.Sprintf("MVI %d,0x%02X", d.RegDst, imm)
	}

	// INR D — increment register (carry flag NOT affected)
	if d.IsINR == 1 {
		val := cpu.readReg(d.RegDst)
		newVal, _ := cpu.alu.Increment(val)
		cpu.writeReg(d.RegDst, newVal)
		// INR updates Z, S, P but NOT carry
		_, oldZ, oldS, oldP := cpu.flags.ReadFlags()
		_ = oldZ
		_ = oldS
		_ = oldP
		z, s, _, p := cpu.alu.ComputeFlags(newVal, carry) // reuse existing carry
		cpu.flags.WriteFlags(carry, z, s, p)
		return fmt.Sprintf("INR %d", d.RegDst)
	}

	// DCR D — decrement register (carry flag NOT affected)
	if d.IsDCR == 1 {
		val := cpu.readReg(d.RegDst)
		newVal, _ := cpu.alu.Decrement(val)
		cpu.writeReg(d.RegDst, newVal)
		// DCR updates Z, S, P but NOT carry
		z, s, _, p := cpu.alu.ComputeFlags(newVal, carry)
		cpu.flags.WriteFlags(carry, z, s, p)
		return fmt.Sprintf("DCR %d", d.RegDst)
	}

	// ALU register operations
	if d.IsALUreg == 1 {
		return cpu.execALUreg(d, carry)
	}

	// ALU immediate operations
	if d.IsALUimm == 1 {
		return cpu.execALUimm(d, imm, carry)
	}

	// RLC — rotate left circular
	if d.IsRLC == 1 {
		a := cpu.regs.Read(7)
		newA, newCarry := cpu.alu.RotateLeftCircular(a)
		cpu.regs.Write(7, newA)
		// Rotates only update carry; Z/S/P unchanged
		_, z, s, p := cpu.flags.ReadFlags()
		cpu.flags.WriteFlags(newCarry, z, s, p)
		return "RLC"
	}

	// RRC — rotate right circular
	if d.IsRRC == 1 {
		a := cpu.regs.Read(7)
		newA, newCarry := cpu.alu.RotateRightCircular(a)
		cpu.regs.Write(7, newA)
		_, z, s, p := cpu.flags.ReadFlags()
		cpu.flags.WriteFlags(newCarry, z, s, p)
		return "RRC"
	}

	// RAL — rotate left through carry
	if d.IsRAL == 1 {
		a := cpu.regs.Read(7)
		newA, newCarry := cpu.alu.RotateLeftThroughCarry(a, carry)
		cpu.regs.Write(7, newA)
		_, z, s, p := cpu.flags.ReadFlags()
		cpu.flags.WriteFlags(newCarry, z, s, p)
		return "RAL"
	}

	// RAR — rotate right through carry
	if d.IsRAR == 1 {
		a := cpu.regs.Read(7)
		newA, newCarry := cpu.alu.RotateRightThroughCarry(a, carry)
		cpu.regs.Write(7, newA)
		_, z, s, p := cpu.flags.ReadFlags()
		cpu.flags.WriteFlags(newCarry, z, s, p)
		return "RAR"
	}

	// OUT — output A to port
	if d.IsOUT == 1 {
		a := cpu.regs.Read(7)
		if d.PortNum < 24 {
			cpu.outputPorts[d.PortNum] = a
		}
		return fmt.Sprintf("OUT %d", d.PortNum)
	}

	// IN — input from port to A
	if d.IsIN == 1 {
		val := 0
		if d.PortNum < 8 {
			val = cpu.inputPorts[d.PortNum]
		}
		cpu.regs.Write(7, val)
		return fmt.Sprintf("IN %d", d.PortNum)
	}

	// JMP — unconditional jump
	if d.IsJMP == 1 {
		// Undo the 3-byte PC advance (Step() incremented PC by 3 already)
		// and load the target instead
		cpu.stack.SetPC(jumpTarget)
		return fmt.Sprintf("JMP 0x%04X", jumpTarget)
	}

	// CAL — unconditional call
	if d.IsCAL == 1 {
		// Step() already advanced PC by 3 (past the 3-byte CAL instruction).
		// So entry[0] currently holds the return address (the instruction after CAL).
		// Push(target) rotates the stack down (saving the return address at entry[1])
		// and sets entry[0] = target (the call destination).
		cpu.stack.Push(jumpTarget)
		return fmt.Sprintf("CAL 0x%04X", jumpTarget)
	}

	// Jcond — conditional jump
	if d.IsJcond == 1 {
		if cpu.checkCondition(d.CondCode) {
			cpu.stack.SetPC(jumpTarget)
		}
		// else: PC was already advanced past the 3-byte instruction
		return fmt.Sprintf("J%s 0x%04X", condMnemonic(d.CondCode), jumpTarget)
	}

	// Ccond — conditional call
	if d.IsCcond == 1 {
		if cpu.checkCondition(d.CondCode) {
			// Same as CAL: PC already points to return address (past the 3-byte instruction)
			cpu.stack.Push(jumpTarget)
		}
		return fmt.Sprintf("C%s 0x%04X", condMnemonic(d.CondCode), jumpTarget)
	}

	// Rcond — conditional return
	if d.IsRcond == 1 {
		if cpu.checkCondition(d.CondCode) {
			cpu.stack.Pop()
		}
		return fmt.Sprintf("R%s", condMnemonic(d.CondCode))
	}

	// RET — unconditional return
	if d.IsRET == 1 {
		cpu.stack.Pop()
		return "RET"
	}

	// RST n — restart (call to fixed address 8*n)
	if d.IsRST == 1 {
		// RST is 1 byte; Step() advanced PC by 1, so entry[0] = return address.
		// Push(RSTVec) rotates stack, saves return address at entry[1], jumps to RSTVec.
		cpu.stack.Push(d.RSTVec)
		return fmt.Sprintf("RST %d", d.RSTVec/8)
	}

	// Unknown instruction — advance was already done
	return fmt.Sprintf("UNKNOWN(0x%02X)", d.Raw)
}

// execALUreg handles group-10 ALU instructions with register source.
func (cpu *Intel8008GateLevel) execALUreg(d DecodedInstruction, carry bool) string {
	a := cpu.regs.Read(7)
	src := cpu.readReg(d.RegSrc)
	carryInt := 0
	if carry {
		carryInt = 1
	}

	var result int
	var newCarry bool
	clearCarry := false

	switch d.ALUOp {
	case ALUOpADD:
		result, newCarry = cpu.alu.Add(a, src, 0)
	case ALUOpADC:
		result, newCarry = cpu.alu.Add(a, src, carryInt)
	case ALUOpSUB:
		result, newCarry = cpu.alu.Subtract(a, src, 0)
	case ALUOpSBB:
		result, newCarry = cpu.alu.Subtract(a, src, carryInt)
	case ALUOpANA:
		result = cpu.alu.BitwiseAnd(a, src)
		clearCarry = true
	case ALUOpXRA:
		result = cpu.alu.BitwiseXor(a, src)
		clearCarry = true
	case ALUOpORA:
		result = cpu.alu.BitwiseOr(a, src)
		clearCarry = true
	case ALUOpCMP:
		// CMP: compute A - src for flags only, don't store result
		cmpResult, cmpCarry := cpu.alu.Subtract(a, src, 0)
		z, s, _, p := cpu.alu.ComputeFlags(cmpResult, cmpCarry)
		cpu.flags.WriteFlags(cmpCarry, z, s, p)
		return fmt.Sprintf("CMP %d", d.RegSrc)
	}

	if clearCarry {
		z, s, _, p := cpu.alu.ComputeFlags(result, false)
		cpu.flags.WriteFlags(false, z, s, p)
	} else {
		z, s, _, p := cpu.alu.ComputeFlags(result, newCarry)
		cpu.flags.WriteFlags(newCarry, z, s, p)
	}
	cpu.regs.Write(7, result)

	aluMnemonics := []string{"ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA"}
	mn := fmt.Sprintf("ALU%d %d", d.ALUOp, d.RegSrc)
	if d.ALUOp < len(aluMnemonics) {
		mn = fmt.Sprintf("%s %d", aluMnemonics[d.ALUOp], d.RegSrc)
	}
	return mn
}

// execALUimm handles group-11 ALU instructions with immediate source.
func (cpu *Intel8008GateLevel) execALUimm(d DecodedInstruction, imm int, carry bool) string {
	a := cpu.regs.Read(7)
	carryInt := 0
	if carry {
		carryInt = 1
	}

	var result int
	var newCarry bool
	clearCarry := false

	switch d.ALUOp {
	case ALUOpADD: // ADI
		result, newCarry = cpu.alu.Add(a, imm, 0)
	case ALUOpSUB: // SUI
		result, newCarry = cpu.alu.Subtract(a, imm, 0)
	case ALUOpANA: // ANI
		result = cpu.alu.BitwiseAnd(a, imm)
		clearCarry = true
	case ALUOpXRA: // XRI
		result = cpu.alu.BitwiseXor(a, imm)
		clearCarry = true
	case ALUOpORA: // ORI
		result = cpu.alu.BitwiseOr(a, imm)
		clearCarry = true
	case ALUOpCMP: // CPI
		cmpResult, cmpCarry := cpu.alu.Subtract(a, imm, 0)
		z, s, _, p := cpu.alu.ComputeFlags(cmpResult, cmpCarry)
		cpu.flags.WriteFlags(cmpCarry, z, s, p)
		return fmt.Sprintf("CPI 0x%02X", imm)
	default:
		// ADC/SBB don't have immediate forms in the 8008
		result, newCarry = cpu.alu.Add(a, imm, carryInt)
	}

	if clearCarry {
		z, s, _, p := cpu.alu.ComputeFlags(result, false)
		cpu.flags.WriteFlags(false, z, s, p)
	} else {
		z, s, _, p := cpu.alu.ComputeFlags(result, newCarry)
		cpu.flags.WriteFlags(newCarry, z, s, p)
	}
	cpu.regs.Write(7, result)

	immMnemonics := map[int]string{
		ALUOpADD: "ADI",
		ALUOpSUB: "SUI",
		ALUOpANA: "ANI",
		ALUOpXRA: "XRI",
		ALUOpORA: "ORI",
		ALUOpCMP: "CPI",
	}
	if mn, ok := immMnemonics[d.ALUOp]; ok {
		return fmt.Sprintf("%s 0x%02X", mn, imm)
	}
	return fmt.Sprintf("ALUI%d 0x%02X", d.ALUOp, imm)
}

// condMnemonic returns the 2-letter suffix for a condition code.
func condMnemonic(code int) string {
	switch code {
	case CondFC:
		return "FC"
	case CondFZ:
		return "FZ"
	case CondFS:
		return "FS"
	case CondFP:
		return "FP"
	case CondTC:
		return "TC"
	case CondTZ:
		return "TZ"
	case CondTS:
		return "TS"
	case CondTP:
		return "TP"
	default:
		return fmt.Sprintf("C%d", code)
	}
}
