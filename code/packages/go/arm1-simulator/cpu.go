// =========================================================================
// cpu.go — ARM1 Simulator (Behavioral)
// =========================================================================
//
// This is the top-level ARM1 CPU simulator. It implements the complete
// ARMv1 instruction set as designed by Sophie Wilson and Steve Furber
// at Acorn Computers in 1984-1985.
//
// # Architecture Summary
//
//   - 32-bit RISC processor, 25,000 transistors
//   - 16 visible registers (R0-R15), 25 physical (banked for FIQ/IRQ/SVC)
//   - R15 = combined Program Counter + Status Register
//   - 3-stage pipeline: Fetch → Decode → Execute
//   - Every instruction is conditional (4-bit condition code)
//   - Inline barrel shifter on Operand2 (shift for free)
//   - No multiply instruction (added in ARM2)
//   - No cache, no MMU
//   - 26-bit address space (64 MiB)
//
// # The Fetch-Decode-Execute Cycle
//
//   1. FETCH:  Read 32-bit instruction from memory at PC
//   2. DECODE: Extract fields (condition, opcode, registers, shift, etc.)
//   3. CHECK:  Evaluate condition code against current flags
//   4. EXECUTE: If condition met, perform the operation
//   5. ADVANCE: PC += 4 (unless a branch or PC write occurred)
//
// The 3-stage pipeline means the PC is always 8 bytes ahead of the
// currently executing instruction. When you read R15 during execution
// of an instruction at address A, you get A + 8. We model this by
// advancing the PC by 8 before execution and adjusting as needed.

package arm1simulator

import (
	"fmt"
)

// =========================================================================
// ARM1 Simulator
// =========================================================================

// ARM1 is the top-level simulator for the first ARM processor.
type ARM1 struct {
	// Register file: 25 physical 32-bit registers
	//
	// Layout:
	//   [0..15]  = R0-R15 (User/System mode base registers)
	//   [16..22] = R8_fiq, R9_fiq, R10_fiq, R11_fiq, R12_fiq, R13_fiq, R14_fiq
	//   [23..24] = R13_irq, R14_irq
	//   [25..26] = R13_svc, R14_svc
	regs [27]uint32

	// Memory — byte-addressable, little-endian
	memory []byte

	// Has the CPU been halted? (by our pseudo-HLT = SWI 0x123456)
	halted bool
}

// New creates a new ARM1 simulator with the given memory size.
//
// The ARM1 has a 26-bit address space (64 MiB). The default memory size
// is 1 MiB, which is more than enough for most test programs.
//
// On power-on, the ARM1 enters Supervisor mode with IRQs and FIQs disabled,
// and begins executing from address 0x00000000 (the Reset vector).
func New(memorySize int) *ARM1 {
	result, _ := StartNew[*ARM1]("arm1-simulator.New", nil,
		func(op *Operation[*ARM1], rf *ResultFactory[*ARM1]) *OperationResult[*ARM1] {
			op.AddProperty("memorySize", memorySize)
			if memorySize <= 0 {
				memorySize = 1024 * 1024 // 1 MiB default
			}
			cpu := &ARM1{
				memory: make([]byte, memorySize),
			}
			cpu.Reset()
			return rf.Generate(true, false, cpu)
		}).GetResult()
	return result
}

// Reset restores the CPU to its power-on state:
//   - Supervisor mode (SVC)
//   - IRQs and FIQs disabled
//   - PC = 0
//   - All flags cleared
func (cpu *ARM1) Reset() {
	_, _ = StartNew[struct{}]("arm1-simulator.Reset", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i := range cpu.regs {
				cpu.regs[i] = 0
			}
			// Set R15: SVC mode (bits 1:0 = 11), IRQ/FIQ disabled (bits 27,26 = 11)
			cpu.regs[15] = FlagI | FlagF | ModeSVC
			cpu.halted = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Register access
// =========================================================================

// ReadRegister reads a register (R0-R15), respecting mode banking.
func (cpu *ARM1) ReadRegister(index int) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.ReadRegister", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("index", index)
			return rf.Generate(true, false, cpu.regs[cpu.physicalReg(index)])
		}).GetResult()
	return result
}

// WriteRegister writes a register (R0-R15), respecting mode banking.
func (cpu *ARM1) WriteRegister(index int, value uint32) {
	_, _ = StartNew[struct{}]("arm1-simulator.WriteRegister", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			op.AddProperty("value", value)
			cpu.regs[cpu.physicalReg(index)] = value
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// physicalReg maps a logical register index (0-15) to a physical register
// index (0-26) based on the current processor mode.
func (cpu *ARM1) physicalReg(index int) int {
	mode := cpu.Mode()

	switch {
	case mode == ModeFIQ && index >= 8 && index <= 14:
		// FIQ banks R8-R14 (7 registers)
		return 16 + (index - 8)
	case mode == ModeIRQ && index >= 13 && index <= 14:
		// IRQ banks R13-R14 (2 registers)
		return 23 + (index - 13)
	case mode == ModeSVC && index >= 13 && index <= 14:
		// SVC banks R13-R14 (2 registers)
		return 25 + (index - 13)
	default:
		return index
	}
}

// PC returns the current program counter (26-bit address).
func (cpu *ARM1) PC() uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.PC", 0,
		func(_ *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, cpu.regs[15]&PCMask)
		}).GetResult()
	return result
}

// SetPC sets the program counter portion of R15 without changing flags/mode.
func (cpu *ARM1) SetPC(addr uint32) {
	_, _ = StartNew[struct{}]("arm1-simulator.SetPC", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("addr", addr)
			cpu.regs[15] = (cpu.regs[15] & ^uint32(PCMask)) | (addr & PCMask)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Flags returns the current condition flags.
func (cpu *ARM1) Flags() Flags {
	result, _ := StartNew[Flags]("arm1-simulator.Flags", Flags{},
		func(_ *Operation[Flags], rf *ResultFactory[Flags]) *OperationResult[Flags] {
			r15 := cpu.regs[15]
			return rf.Generate(true, false, Flags{
				N: (r15 & FlagN) != 0,
				Z: (r15 & FlagZ) != 0,
				C: (r15 & FlagC) != 0,
				V: (r15 & FlagV) != 0,
			})
		}).GetResult()
	return result
}

// SetFlags updates the condition flags in R15.
func (cpu *ARM1) SetFlags(f Flags) {
	_, _ = StartNew[struct{}]("arm1-simulator.SetFlags", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			r15 := cpu.regs[15] & ^uint32(FlagN|FlagZ|FlagC|FlagV)
			if f.N {
				r15 |= FlagN
			}
			if f.Z {
				r15 |= FlagZ
			}
			if f.C {
				r15 |= FlagC
			}
			if f.V {
				r15 |= FlagV
			}
			cpu.regs[15] = r15
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Mode returns the current processor mode (0=USR, 1=FIQ, 2=IRQ, 3=SVC).
func (cpu *ARM1) Mode() int {
	result, _ := StartNew[int]("arm1-simulator.Mode", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, int(cpu.regs[15]&ModeMask))
		}).GetResult()
	return result
}

// Halted returns true if the CPU has been halted.
func (cpu *ARM1) Halted() bool {
	result, _ := StartNew[bool]("arm1-simulator.Halted", false,
		func(_ *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, cpu.halted)
		}).GetResult()
	return result
}

// =========================================================================
// Memory access
// =========================================================================

// ReadWord reads a 32-bit word from memory (little-endian).
// The address must be within the memory bounds.
func (cpu *ARM1) ReadWord(addr uint32) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.ReadWord", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("addr", addr)
			addr &= PCMask // Mask to 26-bit address space
			a := int(addr & ^uint32(3)) // Word-align
			if a+3 >= len(cpu.memory) {
				return rf.Generate(true, false, uint32(0))
			}
			return rf.Generate(true, false, uint32(cpu.memory[a])|
				uint32(cpu.memory[a+1])<<8|
				uint32(cpu.memory[a+2])<<16|
				uint32(cpu.memory[a+3])<<24)
		}).GetResult()
	return result
}

// WriteWord writes a 32-bit word to memory (little-endian).
func (cpu *ARM1) WriteWord(addr uint32, value uint32) {
	_, _ = StartNew[struct{}]("arm1-simulator.WriteWord", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("addr", addr)
			op.AddProperty("value", value)
			addr &= PCMask
			a := int(addr & ^uint32(3))
			if a+3 >= len(cpu.memory) {
				return rf.Generate(true, false, struct{}{})
			}
			cpu.memory[a] = byte(value)
			cpu.memory[a+1] = byte(value >> 8)
			cpu.memory[a+2] = byte(value >> 16)
			cpu.memory[a+3] = byte(value >> 24)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadByte reads a single byte from memory.
func (cpu *ARM1) ReadByte(addr uint32) byte {
	result, _ := StartNew[byte]("arm1-simulator.ReadByte", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("addr", addr)
			addr &= PCMask
			a := int(addr)
			if a >= len(cpu.memory) {
				return rf.Generate(true, false, byte(0))
			}
			return rf.Generate(true, false, cpu.memory[a])
		}).GetResult()
	return result
}

// WriteByte writes a single byte to memory.
func (cpu *ARM1) WriteByte(addr uint32, value byte) {
	_, _ = StartNew[struct{}]("arm1-simulator.WriteByte", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("addr", addr)
			op.AddProperty("value", value)
			addr &= PCMask
			a := int(addr)
			if a >= len(cpu.memory) {
				return rf.Generate(true, false, struct{}{})
			}
			cpu.memory[a] = value
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Memory returns a reference to the raw memory array.
func (cpu *ARM1) Memory() []byte {
	result, _ := StartNew[[]byte]("arm1-simulator.Memory", nil,
		func(_ *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, cpu.memory)
		}).GetResult()
	return result
}

// LoadProgram loads machine code into memory at the given start address.
func (cpu *ARM1) LoadProgram(code []byte, startAddr uint32) {
	_, _ = StartNew[struct{}]("arm1-simulator.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("startAddr", startAddr)
			for i, b := range code {
				addr := int(startAddr) + i
				if addr < len(cpu.memory) {
					cpu.memory[addr] = b
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Execution
// =========================================================================

// Step executes one instruction and returns a trace of what happened.
func (cpu *ARM1) Step() Trace {
	result, _ := StartNew[Trace]("arm1-simulator.Step", Trace{},
		func(_ *Operation[Trace], rf *ResultFactory[Trace]) *OperationResult[Trace] {
			// Capture state before execution (direct field access to avoid nested ops)
			pc := cpu.regs[15] & PCMask
			var regsBefore [16]uint32
			for i := 0; i < 16; i++ {
				regsBefore[i] = cpu.regs[cpu.physicalReg(i)]
			}
			r15 := cpu.regs[15]
			flagsBefore := Flags{
				N: (r15 & FlagN) != 0,
				Z: (r15 & FlagZ) != 0,
				C: (r15 & FlagC) != 0,
				V: (r15 & FlagV) != 0,
			}

			// Fetch: read 32-bit word at PC (direct memory access)
			instruction := cpu.readWordRaw(pc)

			// Decode
			decoded := Decode(instruction)

			// Evaluate condition
			condMet := EvaluateCondition(decoded.Cond, flagsBefore)

			trace := Trace{
				Address:      pc,
				Raw:          instruction,
				Mnemonic:     decoded.Disassemble(),
				Condition:    CondString(decoded.Cond),
				ConditionMet: condMet,
				RegsBefore:   regsBefore,
				FlagsBefore:  flagsBefore,
			}

			// Advance PC (default: next instruction)
			cpu.regs[15] = (cpu.regs[15] & ^uint32(PCMask)) | ((pc + 4) & PCMask)

			if condMet {
				// Execute
				switch decoded.Type {
				case InstDataProcessing:
					cpu.executeDataProcessing(&decoded, &trace)
				case InstLoadStore:
					cpu.executeLoadStore(&decoded, &trace)
				case InstBlockTransfer:
					cpu.executeBlockTransfer(&decoded, &trace)
				case InstBranch:
					cpu.executeBranch(&decoded, &trace)
				case InstSWI:
					cpu.executeSWI(&decoded, &trace)
				case InstCoprocessor:
					// ARM1 has no coprocessor — trigger undefined instruction trap
					cpu.trapUndefined(pc)
				case InstUndefined:
					cpu.trapUndefined(pc)
				}
			}

			// Capture state after execution
			for i := 0; i < 16; i++ {
				trace.RegsAfter[i] = cpu.regs[cpu.physicalReg(i)]
			}
			r15After := cpu.regs[15]
			trace.FlagsAfter = Flags{
				N: (r15After & FlagN) != 0,
				Z: (r15After & FlagZ) != 0,
				C: (r15After & FlagC) != 0,
				V: (r15After & FlagV) != 0,
			}

			return rf.Generate(true, false, trace)
		}).GetResult()
	return result
}

// readWordRaw reads a 32-bit word from memory without the Operations wrapper.
// Used internally by Step to avoid nested Operation instrumentation.
func (cpu *ARM1) readWordRaw(addr uint32) uint32 {
	addr &= PCMask
	a := int(addr & ^uint32(3))
	if a+3 >= len(cpu.memory) {
		return 0
	}
	return uint32(cpu.memory[a]) |
		uint32(cpu.memory[a+1])<<8 |
		uint32(cpu.memory[a+2])<<16 |
		uint32(cpu.memory[a+3])<<24
}

// Run executes instructions until halted or max_steps reached.
func (cpu *ARM1) Run(maxSteps int) []Trace {
	result, _ := StartNew[[]Trace]("arm1-simulator.Run", nil,
		func(op *Operation[[]Trace], rf *ResultFactory[[]Trace]) *OperationResult[[]Trace] {
			op.AddProperty("maxSteps", maxSteps)
			traces := make([]Trace, 0, maxSteps)
			for i := 0; i < maxSteps && !cpu.halted; i++ {
				trace := cpu.Step()
				traces = append(traces, trace)
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// =========================================================================
// Data Processing execution
// =========================================================================

func (cpu *ARM1) executeDataProcessing(d *DecodedInstruction, trace *Trace) {
	// Get first operand (Rn)
	var a uint32
	if d.Opcode != OpMOV && d.Opcode != OpMVN {
		a = cpu.readRegForExec(d.Rn)
	}

	// Get second operand (Operand2) through barrel shifter
	var b uint32
	var shifterCarry bool
	flags := cpu.Flags()

	if d.Immediate {
		b, shifterCarry = DecodeImmediate(d.Imm8, d.Rotate)
		if d.Rotate == 0 {
			shifterCarry = flags.C // Carry unchanged when no rotation
		}
	} else {
		rmVal := cpu.readRegForExec(d.Rm)
		var shiftAmount int
		if d.ShiftByReg {
			shiftAmount = int(cpu.readRegForExec(d.Rs) & 0xFF)
		} else {
			shiftAmount = d.ShiftImm
		}
		b, shifterCarry = BarrelShift(rmVal, d.ShiftType, shiftAmount, flags.C, d.ShiftByReg)
	}

	// Execute ALU operation
	result := ALUExecute(d.Opcode, a, b, flags.C, shifterCarry, flags.V)

	// Write result to Rd (unless test-only operation)
	if result.WriteResult {
		if d.Rd == 15 {
			// Writing to R15 — update the entire register (PC + flags if S set)
			if d.S {
				// MOVS PC, LR — restore PC and flags (used for exception return)
				cpu.regs[15] = result.Result
			} else {
				// MOV PC, Rn — only update PC portion
				cpu.SetPC(result.Result & PCMask)
			}
		} else {
			cpu.WriteRegister(d.Rd, result.Result)
		}
	}

	// Update flags if S bit set (and Rd is not R15, which is handled above)
	if d.S && d.Rd != 15 {
		cpu.SetFlags(Flags{
			N: result.N,
			Z: result.Z,
			C: result.C,
			V: result.V,
		})
	}
	// For test-only ops (TST/TEQ/CMP/CMN), always update flags
	if IsTestOp(d.Opcode) {
		cpu.SetFlags(Flags{
			N: result.N,
			Z: result.Z,
			C: result.C,
			V: result.V,
		})
	}
}

// readRegForExec reads a register value as it would appear during instruction
// execution. For R15, this returns PC + 8 (accounting for the 3-stage pipeline).
func (cpu *ARM1) readRegForExec(index int) uint32 {
	if index == 15 {
		// R15 reads as PC + 8 during execution (pipeline effect)
		// But we've already advanced PC by 4 in Step(), so we add 4 more
		return cpu.regs[15] + 4
	}
	return cpu.ReadRegister(index)
}

// =========================================================================
// Load/Store execution
// =========================================================================

func (cpu *ARM1) executeLoadStore(d *DecodedInstruction, trace *Trace) {
	// Compute offset
	var offset uint32
	if d.Immediate {
		// Register offset (with optional shift)
		rmVal := cpu.readRegForExec(d.Rm)
		if d.ShiftImm != 0 {
			rmVal, _ = BarrelShift(rmVal, d.ShiftType, d.ShiftImm, cpu.Flags().C, false)
		}
		offset = rmVal
	} else {
		offset = d.Offset12
	}

	// Base address
	base := cpu.readRegForExec(d.Rn)

	// Compute effective address
	var addr uint32
	if d.Up {
		addr = base + offset
	} else {
		addr = base - offset
	}

	// Pre-indexed: use computed address
	// Post-indexed: use base address for the transfer, then update base
	transferAddr := addr
	if !d.PreIndex {
		transferAddr = base
	}

	if d.Load {
		// LDR / LDRB
		var value uint32
		if d.Byte {
			value = uint32(cpu.ReadByte(transferAddr))
		} else {
			value = cpu.ReadWord(transferAddr)
			// ARM1 quirk: unaligned word loads rotate the data
			rotation := (transferAddr & 3) * 8
			if rotation != 0 {
				value = (value >> rotation) | (value << (32 - rotation))
			}
		}
		trace.MemoryReads = append(trace.MemoryReads, MemoryAccess{Address: transferAddr, Value: value})

		if d.Rd == 15 {
			cpu.regs[15] = value
		} else {
			cpu.WriteRegister(d.Rd, value)
		}
	} else {
		// STR / STRB
		value := cpu.readRegForExec(d.Rd)
		if d.Byte {
			cpu.WriteByte(transferAddr, byte(value&0xFF))
		} else {
			cpu.WriteWord(transferAddr, value)
		}
		trace.MemoryWrites = append(trace.MemoryWrites, MemoryAccess{Address: transferAddr, Value: value})
	}

	// Write-back (update base register)
	if d.WriteBack || !d.PreIndex {
		if d.Rn != 15 {
			cpu.WriteRegister(d.Rn, addr)
		}
	}
}

// =========================================================================
// Block Transfer execution (LDM/STM)
// =========================================================================

func (cpu *ARM1) executeBlockTransfer(d *DecodedInstruction, trace *Trace) {
	base := cpu.ReadRegister(d.Rn)
	regList := d.RegisterList

	// Count registers in the list
	count := uint32(0)
	for i := 0; i < 16; i++ {
		if (regList>>i)&1 == 1 {
			count++
		}
	}

	if count == 0 {
		// Empty register list — architecturally unpredictable, we do nothing
		return
	}

	// Calculate the lowest and highest addresses
	// Registers are always stored lowest-numbered at lowest address
	var startAddr uint32
	switch {
	case !d.PreIndex && d.Up: // IA (Increment After)
		startAddr = base
	case d.PreIndex && d.Up: // IB (Increment Before)
		startAddr = base + 4
	case !d.PreIndex && !d.Up: // DA (Decrement After)
		startAddr = base - (count * 4) + 4
	case d.PreIndex && !d.Up: // DB (Decrement Before)
		startAddr = base - (count * 4)
	}

	addr := startAddr
	for i := 0; i < 16; i++ {
		if (regList>>i)&1 == 0 {
			continue
		}

		if d.Load {
			value := cpu.ReadWord(addr)
			trace.MemoryReads = append(trace.MemoryReads, MemoryAccess{Address: addr, Value: value})
			if i == 15 {
				cpu.regs[15] = value
			} else {
				cpu.WriteRegister(i, value)
			}
		} else {
			var value uint32
			if i == 15 {
				value = cpu.regs[15] + 4 // PC + 8 but we already added 4
			} else {
				value = cpu.ReadRegister(i)
			}
			cpu.WriteWord(addr, value)
			trace.MemoryWrites = append(trace.MemoryWrites, MemoryAccess{Address: addr, Value: value})
		}
		addr += 4
	}

	// Write-back
	if d.WriteBack {
		var newBase uint32
		if d.Up {
			newBase = base + (count * 4)
		} else {
			newBase = base - (count * 4)
		}
		cpu.WriteRegister(d.Rn, newBase)
	}
}

// =========================================================================
// Branch execution
// =========================================================================

func (cpu *ARM1) executeBranch(d *DecodedInstruction, trace *Trace) {
	// Current PC (already advanced by 4 in Step)
	// The branch offset is relative to PC + 8 from the original instruction.
	// Since we already did PC += 4, we need PC + 4 more = current PC + 4.
	branchBase := cpu.PC() + 4

	if d.Link {
		// BL: save return address in R14 (LR)
		// In ARMv1, R14 gets the full R15 value (PC + flags) of the next instruction
		// The next instruction after the BL is at (original PC + 4), which is
		// the current PC value (since we already incremented by 4).
		returnAddr := cpu.regs[15] // Full R15 with current PC (already advanced)
		// Adjust: R14 should contain the address of the instruction after BL
		// with current flags/mode
		cpu.WriteRegister(14, returnAddr)
	}

	// Compute target address
	target := uint32(int32(branchBase) + d.BranchOffset)
	cpu.SetPC(target & PCMask)
}

// =========================================================================
// SWI execution
// =========================================================================

func (cpu *ARM1) executeSWI(d *DecodedInstruction, trace *Trace) {
	if d.SWIComment == HaltSWI {
		// Our pseudo-halt instruction
		cpu.halted = true
		return
	}

	// Real SWI: enter Supervisor mode
	// 1. Save R15 (PC + flags) to R14_svc
	cpu.regs[25] = cpu.regs[15] // R13_svc index, R14_svc = index 26
	cpu.regs[26] = cpu.regs[15]

	// 2. Set mode to SVC, disable IRQs
	r15 := cpu.regs[15]
	r15 = (r15 & ^uint32(ModeMask)) | ModeSVC // Set SVC mode
	r15 |= FlagI                               // Disable IRQs
	cpu.regs[15] = r15

	// 3. Jump to SWI vector (0x08)
	cpu.SetPC(0x08)
}

// =========================================================================
// Exception handling
// =========================================================================

func (cpu *ARM1) trapUndefined(instrAddr uint32) {
	// Save R15 to R14_svc
	cpu.regs[26] = cpu.regs[15]

	// Enter SVC mode, disable IRQs
	r15 := cpu.regs[15]
	r15 = (r15 & ^uint32(ModeMask)) | ModeSVC
	r15 |= FlagI
	cpu.regs[15] = r15

	// Jump to Undefined Instruction vector (0x04)
	cpu.SetPC(0x04)
}

// =========================================================================
// Convenience: encode instructions
// =========================================================================

// EncodeDataProcessing creates a data processing instruction word.
// This is useful for writing test programs without an assembler.
func EncodeDataProcessing(cond, opcode, s, rn, rd int, operand2 uint32) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeDataProcessing", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("opcode", opcode)
			op.AddProperty("s", s)
			op.AddProperty("rn", rn)
			op.AddProperty("rd", rd)
			return rf.Generate(true, false, uint32(cond)<<28|operand2|
				uint32(opcode)<<21|uint32(s)<<20|
				uint32(rn)<<16|uint32(rd)<<12)
		}).GetResult()
	return result
}

// EncodeMovImm creates a MOV immediate instruction.
// Example: EncodeMovImm(CondAL, 0, 42) → MOV R0, #42
func EncodeMovImm(cond, rd int, imm8 uint32) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeMovImm", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("rd", rd)
			op.AddProperty("imm8", imm8)
			return rf.Generate(true, false, EncodeDataProcessing(cond, OpMOV, 0, 0, rd, (1<<25)|imm8))
		}).GetResult()
	return result
}

// EncodeALUReg creates a data processing instruction with a register operand.
// Example: EncodeALUReg(CondAL, OpADD, 1, 0, 1, 2) → ADDS R0, R1, R2
func EncodeALUReg(cond, opcode, s, rd, rn, rm int) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeALUReg", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("opcode", opcode)
			op.AddProperty("s", s)
			op.AddProperty("rd", rd)
			op.AddProperty("rn", rn)
			op.AddProperty("rm", rm)
			return rf.Generate(true, false, EncodeDataProcessing(cond, opcode, s, rn, rd, uint32(rm)))
		}).GetResult()
	return result
}

// EncodeBranch creates a Branch or Branch-with-Link instruction.
func EncodeBranch(cond int, link bool, offset int32) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeBranch", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("link", link)
			op.AddProperty("offset", offset)
			inst := uint32(cond)<<28 | 0x0A000000
			if link {
				inst |= 0x01000000
			}
			encoded := uint32((offset >> 2) & 0x00FFFFFF)
			inst |= encoded
			return rf.Generate(true, false, inst)
		}).GetResult()
	return result
}

// EncodeHalt creates our pseudo-halt instruction (SWI 0x123456).
func EncodeHalt() uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeHalt", 0,
		func(_ *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32(CondAL)<<28|0x0F000000|HaltSWI)
		}).GetResult()
	return result
}

// EncodeLDR creates a Load Register instruction with immediate offset.
func EncodeLDR(cond, rd, rn int, offset int, preIndex bool) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeLDR", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("rd", rd)
			op.AddProperty("rn", rn)
			op.AddProperty("offset", offset)
			op.AddProperty("preIndex", preIndex)
			inst := uint32(cond)<<28 | 0x04100000
			inst |= uint32(rd) << 12
			inst |= uint32(rn) << 16
			if preIndex {
				inst |= 1 << 24
			}
			if offset >= 0 {
				inst |= 1 << 23
				inst |= uint32(offset) & 0xFFF
			} else {
				inst |= uint32(-offset) & 0xFFF
			}
			return rf.Generate(true, false, inst)
		}).GetResult()
	return result
}

// EncodeSTR creates a Store Register instruction with immediate offset.
func EncodeSTR(cond, rd, rn int, offset int, preIndex bool) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeSTR", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("rd", rd)
			op.AddProperty("rn", rn)
			op.AddProperty("offset", offset)
			op.AddProperty("preIndex", preIndex)
			inst := uint32(cond)<<28 | 0x04000000
			inst |= uint32(rd) << 12
			inst |= uint32(rn) << 16
			if preIndex {
				inst |= 1 << 24
			}
			if offset >= 0 {
				inst |= 1 << 23
				inst |= uint32(offset) & 0xFFF
			} else {
				inst |= uint32(-offset) & 0xFFF
			}
			return rf.Generate(true, false, inst)
		}).GetResult()
	return result
}

// EncodeLDM creates a Load Multiple instruction.
func EncodeLDM(cond, rn int, regList uint16, writeBack bool, mode string) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeLDM", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("rn", rn)
			op.AddProperty("writeBack", writeBack)
			op.AddProperty("mode", mode)
			inst := uint32(cond)<<28 | 0x08100000
			inst |= uint32(rn) << 16
			inst |= uint32(regList)
			if writeBack {
				inst |= 1 << 21
			}
			switch mode {
			case "IA":
				inst |= 1 << 23
			case "IB":
				inst |= 1 << 24
				inst |= 1 << 23
			case "DA":
				// P=0, U=0 (both already 0)
			case "DB":
				inst |= 1 << 24
			}
			return rf.Generate(true, false, inst)
		}).GetResult()
	return result
}

// EncodeSTM creates a Store Multiple instruction.
func EncodeSTM(cond, rn int, regList uint16, writeBack bool, mode string) uint32 {
	result, _ := StartNew[uint32]("arm1-simulator.EncodeSTM", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("cond", cond)
			op.AddProperty("rn", rn)
			op.AddProperty("writeBack", writeBack)
			op.AddProperty("mode", mode)
			inst := EncodeLDM(cond, rn, regList, writeBack, mode)
			inst &= ^uint32(1 << 20)
			return rf.Generate(true, false, inst)
		}).GetResult()
	return result
}

// String returns a formatted representation of the CPU state.
func (cpu *ARM1) String() string {
	result, _ := StartNew[string]("arm1-simulator.String", "",
		func(_ *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			mode := ModeString(cpu.Mode())
			flags := cpu.Flags()
			flagStr := ""
			if flags.N {
				flagStr += "N"
			} else {
				flagStr += "n"
			}
			if flags.Z {
				flagStr += "Z"
			} else {
				flagStr += "z"
			}
			if flags.C {
				flagStr += "C"
			} else {
				flagStr += "c"
			}
			if flags.V {
				flagStr += "V"
			} else {
				flagStr += "v"
			}

			s := fmt.Sprintf("ARM1 [%s] %s PC=%08X\n", mode, flagStr, cpu.PC())
			for i := 0; i < 16; i += 4 {
				s += fmt.Sprintf("  R%-2d=%08X  R%-2d=%08X  R%-2d=%08X  R%-2d=%08X\n",
					i, cpu.ReadRegister(i),
					i+1, cpu.ReadRegister(i+1),
					i+2, cpu.ReadRegister(i+2),
					i+3, cpu.ReadRegister(i+3))
			}
			return rf.Generate(true, false, s)
		}).GetResult()
	return result
}
