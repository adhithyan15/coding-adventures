// Package riscvsimulator — instruction executor for all RV32I + M-mode instructions.
//
// === How execution works ===
//
// After the decoder extracts instruction fields (registers, immediates, etc.),
// the executor performs the actual computation: arithmetic, memory access,
// branching, or privilege operations. Each instruction produces an
// ExecuteResult describing what changed (registers, memory, next PC).
//
// === The x0 invariant ===
//
// Every instruction that writes to a register must check: is the destination
// register x0? If so, the write is silently discarded. This is enforced
// throughout the executor by the writeRd helper function.
//
// === Signed vs unsigned arithmetic ===
//
// RISC-V registers hold 32-bit values (uint32 in Go). Some instructions
// interpret these as signed (int32) and others as unsigned (uint32).
// For example:
//   - slt  compares as signed:   -1 < 1  → true
//   - sltu compares as unsigned: 0xFFFFFFFF > 1 → true (because -1 as unsigned is huge)
//
// We use Go's type conversions (int32↔uint32) to handle this correctly.
package riscvsimulator

import (
	"fmt"

	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
)

// Execute dispatches to the appropriate instruction handler based on the
// decoded mnemonic. This is the main entry point called by the CPU pipeline.
func (e *RiscVExecutor) Execute(decoded cpu.DecodeResult, registers *cpu.RegisterFile, memory *cpu.Memory, pc int) cpu.ExecuteResult {
	switch decoded.Mnemonic {
	// === I-type arithmetic ===
	case "addi":
		return e.execImmArith(decoded, registers, pc, "addi", func(a int32, b int32) uint32 { return uint32(a + b) })
	case "slti":
		return e.execImmArith(decoded, registers, pc, "slti", func(a int32, b int32) uint32 {
			if a < b {
				return 1
			}
			return 0
		})
	case "sltiu":
		return e.execImmArith(decoded, registers, pc, "sltiu", func(a int32, b int32) uint32 {
			// Compare as unsigned: cast both to uint32 first
			if uint32(a) < uint32(b) {
				return 1
			}
			return 0
		})
	case "xori":
		return e.execImmArith(decoded, registers, pc, "xori", func(a int32, b int32) uint32 { return uint32(a) ^ uint32(b) })
	case "ori":
		return e.execImmArith(decoded, registers, pc, "ori", func(a int32, b int32) uint32 { return uint32(a) | uint32(b) })
	case "andi":
		return e.execImmArith(decoded, registers, pc, "andi", func(a int32, b int32) uint32 { return uint32(a) & uint32(b) })
	case "slli":
		return e.execShiftImm(decoded, registers, pc, "slli", func(val uint32, shamt uint32) uint32 { return val << shamt })
	case "srli":
		return e.execShiftImm(decoded, registers, pc, "srli", func(val uint32, shamt uint32) uint32 { return val >> shamt })
	case "srai":
		return e.execShiftImm(decoded, registers, pc, "srai", func(val uint32, shamt uint32) uint32 {
			// Arithmetic right shift preserves the sign bit.
			// Go's >> on signed integers does arithmetic shift.
			return uint32(int32(val) >> shamt)
		})

	// === R-type arithmetic ===
	case "add":
		return e.execRegArith(decoded, registers, pc, "add", func(a, b uint32) uint32 { return uint32(int32(a) + int32(b)) })
	case "sub":
		return e.execRegArith(decoded, registers, pc, "sub", func(a, b uint32) uint32 { return uint32(int32(a) - int32(b)) })
	case "sll":
		return e.execRegArith(decoded, registers, pc, "sll", func(a, b uint32) uint32 { return a << (b & 0x1F) })
	case "slt":
		return e.execRegArith(decoded, registers, pc, "slt", func(a, b uint32) uint32 {
			if int32(a) < int32(b) {
				return 1
			}
			return 0
		})
	case "sltu":
		return e.execRegArith(decoded, registers, pc, "sltu", func(a, b uint32) uint32 {
			if a < b {
				return 1
			}
			return 0
		})
	case "xor":
		return e.execRegArith(decoded, registers, pc, "xor", func(a, b uint32) uint32 { return a ^ b })
	case "srl":
		return e.execRegArith(decoded, registers, pc, "srl", func(a, b uint32) uint32 { return a >> (b & 0x1F) })
	case "sra":
		return e.execRegArith(decoded, registers, pc, "sra", func(a, b uint32) uint32 { return uint32(int32(a) >> (b & 0x1F)) })
	case "or":
		return e.execRegArith(decoded, registers, pc, "or", func(a, b uint32) uint32 { return a | b })
	case "and":
		return e.execRegArith(decoded, registers, pc, "and", func(a, b uint32) uint32 { return a & b })

	// === Load instructions ===
	case "lb", "lh", "lw", "lbu", "lhu":
		return e.execLoad(decoded, registers, memory, pc)

	// === Store instructions ===
	case "sb", "sh", "sw":
		return e.execStore(decoded, registers, memory, pc)

	// === Branch instructions ===
	case "beq":
		return e.execBranch(decoded, registers, pc, "beq", func(a, b uint32) bool { return a == b })
	case "bne":
		return e.execBranch(decoded, registers, pc, "bne", func(a, b uint32) bool { return a != b })
	case "blt":
		return e.execBranch(decoded, registers, pc, "blt", func(a, b uint32) bool { return int32(a) < int32(b) })
	case "bge":
		return e.execBranch(decoded, registers, pc, "bge", func(a, b uint32) bool { return int32(a) >= int32(b) })
	case "bltu":
		return e.execBranch(decoded, registers, pc, "bltu", func(a, b uint32) bool { return a < b })
	case "bgeu":
		return e.execBranch(decoded, registers, pc, "bgeu", func(a, b uint32) bool { return a >= b })

	// === Jump instructions ===
	case "jal":
		return e.execJAL(decoded, registers, pc)
	case "jalr":
		return e.execJALR(decoded, registers, pc)

	// === Upper immediate instructions ===
	case "lui":
		return e.execLUI(decoded, registers, pc)
	case "auipc":
		return e.execAUIPC(decoded, registers, pc)

	// === System / privileged instructions ===
	case "ecall":
		return e.execEcall(decoded, registers, pc)
	case "mret":
		return e.execMret(decoded, registers, pc)
	case "csrrw":
		return e.execCSRRW(decoded, registers, pc)
	case "csrrs":
		return e.execCSRRS(decoded, registers, pc)
	case "csrrc":
		return e.execCSRRC(decoded, registers, pc)

	default:
		return cpu.ExecuteResult{
			Description:      fmt.Sprintf("Unknown instruction: %s", decoded.Mnemonic),
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc + 4,
		}
	}
}

// === Helper: write to a destination register, respecting the x0 invariant ===
//
// Returns the map of changed registers for the ExecuteResult.
func writeRd(registers *cpu.RegisterFile, rd int, value uint32) map[string]uint32 {
	changes := map[string]uint32{}
	if rd != 0 {
		registers.Write(rd, value)
		changes[fmt.Sprintf("x%d", rd)] = value
	}
	return changes
}

// === I-type arithmetic executor ===
//
// Most I-type arithmetic instructions follow the same pattern:
//
//	result = op(rs1_value, sign_extended_immediate)
//
// This generic helper avoids duplicating the register read/write boilerplate.
func (e *RiscVExecutor) execImmArith(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int, name string, op func(int32, int32) uint32) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	imm := decoded.Fields["imm"]

	rs1Val := int32(registers.Read(rs1))
	result := op(rs1Val, int32(imm))
	changes := writeRd(registers, rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("%s: x%d = x%d(%d), imm=%d -> %d", name, rd, rs1, rs1Val, imm, int32(result)),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === Shift-immediate executor ===
//
// Shift instructions use only the lower 5 bits of the immediate as the
// shift amount (0-31), since shifting a 32-bit value by more than 31
// would be meaningless.
func (e *RiscVExecutor) execShiftImm(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int, name string, op func(uint32, uint32) uint32) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	shamt := uint32(decoded.Fields["imm"] & 0x1F)

	rs1Val := registers.Read(rs1)
	result := op(rs1Val, shamt)
	changes := writeRd(registers, rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("%s: x%d = x%d(0x%x) %s %d -> 0x%x", name, rd, rs1, rs1Val, name, shamt, result),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === R-type arithmetic executor ===
//
// R-type instructions operate on two registers:
//
//	result = op(rs1_value, rs2_value)
func (e *RiscVExecutor) execRegArith(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int, name string, op func(uint32, uint32) uint32) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	rs2 := decoded.Fields["rs2"]

	rs1Val := registers.Read(rs1)
	rs2Val := registers.Read(rs2)
	result := op(rs1Val, rs2Val)
	changes := writeRd(registers, rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("%s: x%d = x%d(%d) op x%d(%d) -> %d", name, rd, rs1, int32(rs1Val), rs2, int32(rs2Val), int32(result)),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === Load instruction executor ===
//
// All load instructions compute an address as: addr = rs1 + sign_extend(imm)
// Then they read 1, 2, or 4 bytes from memory and either sign-extend or
// zero-extend the result to 32 bits before writing it to rd.
//
// === Sign extension vs zero extension ===
//
// When loading a byte (8 bits) into a 32-bit register, we need to decide
// what goes in the upper 24 bits:
//   - Sign extension (lb):  if the byte's MSB is 1, fill upper bits with 1s
//     0xFF -> 0xFFFFFFFF (-1 as int32)
//   - Zero extension (lbu): always fill upper bits with 0s
//     0xFF -> 0x000000FF (255 as uint32)
//
// This distinction matters for signed arithmetic. If you load a -1 stored
// as 0xFF and want to add it to something, you need sign extension.
func (e *RiscVExecutor) execLoad(decoded cpu.DecodeResult, registers *cpu.RegisterFile, memory *cpu.Memory, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	imm := decoded.Fields["imm"]
	mnemonic := decoded.Mnemonic

	addr := int(int32(registers.Read(rs1)) + int32(imm))

	var result uint32
	switch mnemonic {
	case "lb":
		// Load byte, sign-extend: read 1 byte, treat as signed
		b := memory.ReadByte(addr)
		result = uint32(int32(int8(b))) // int8 sign-extends, then widen to 32
	case "lh":
		// Load halfword, sign-extend: read 2 bytes (little-endian), treat as signed
		lo := uint32(memory.ReadByte(addr))
		hi := uint32(memory.ReadByte(addr + 1))
		half := uint16(lo | (hi << 8))
		result = uint32(int32(int16(half))) // int16 sign-extends, then widen
	case "lw":
		// Load word: read 4 bytes (little-endian)
		result = memory.ReadWord(addr)
	case "lbu":
		// Load byte, zero-extend: read 1 byte, upper bits are 0
		result = uint32(memory.ReadByte(addr))
	case "lhu":
		// Load halfword, zero-extend: read 2 bytes, upper bits are 0
		lo := uint32(memory.ReadByte(addr))
		hi := uint32(memory.ReadByte(addr + 1))
		result = lo | (hi << 8)
	}

	changes := writeRd(registers, rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("%s: x%d = mem[x%d(%d) + %d] = mem[%d] -> 0x%x", mnemonic, rd, rs1, registers.Read(rs1), imm, addr, result),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === Store instruction executor ===
//
// Store instructions compute an address as: addr = rs1 + sign_extend(imm)
// Then they write 1, 2, or 4 bytes from rs2 to memory.
//
// Note: stores do NOT write to any register — they only change memory.
// The MemoryChanged map in the result tracks which bytes were modified.
func (e *RiscVExecutor) execStore(decoded cpu.DecodeResult, registers *cpu.RegisterFile, memory *cpu.Memory, pc int) cpu.ExecuteResult {
	rs1 := decoded.Fields["rs1"]
	rs2 := decoded.Fields["rs2"]
	imm := decoded.Fields["imm"]
	mnemonic := decoded.Mnemonic

	addr := int(int32(registers.Read(rs1)) + int32(imm))
	val := registers.Read(rs2)
	memChanges := map[int]byte{}

	switch mnemonic {
	case "sb":
		// Store byte: write lowest 8 bits of rs2
		b := byte(val & 0xFF)
		memory.WriteByte(addr, b)
		memChanges[addr] = b
	case "sh":
		// Store halfword: write lowest 16 bits of rs2 (little-endian)
		lo := byte(val & 0xFF)
		hi := byte((val >> 8) & 0xFF)
		memory.WriteByte(addr, lo)
		memory.WriteByte(addr+1, hi)
		memChanges[addr] = lo
		memChanges[addr+1] = hi
	case "sw":
		// Store word: write all 32 bits (little-endian)
		memory.WriteWord(addr, val)
		memChanges[addr] = byte(val & 0xFF)
		memChanges[addr+1] = byte((val >> 8) & 0xFF)
		memChanges[addr+2] = byte((val >> 16) & 0xFF)
		memChanges[addr+3] = byte((val >> 24) & 0xFF)
	}

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("%s: mem[x%d + %d] = mem[%d] <- x%d(0x%x)", mnemonic, rs1, imm, addr, rs2, val),
		RegistersChanged: map[string]uint32{},
		MemoryChanged:    memChanges,
		NextPC:           pc + 4,
	}
}

// === Branch instruction executor ===
//
// Branches compare rs1 and rs2 using a condition function. If the condition
// is true, the PC jumps to PC + offset. If false, execution continues to
// the next instruction (PC + 4).
//
// === Why PC-relative? ===
//
// Branch offsets are relative to the current PC, not absolute addresses.
// This makes code "position-independent" — the same binary works regardless
// of where it's loaded in memory. The offset is a signed value, allowing
// both forward jumps (positive offset) and backward jumps (loops).
func (e *RiscVExecutor) execBranch(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int, name string, cond func(uint32, uint32) bool) cpu.ExecuteResult {
	rs1 := decoded.Fields["rs1"]
	rs2 := decoded.Fields["rs2"]
	imm := decoded.Fields["imm"]

	rs1Val := registers.Read(rs1)
	rs2Val := registers.Read(rs2)

	taken := cond(rs1Val, rs2Val)
	nextPC := pc + 4
	if taken {
		nextPC = pc + imm
	}

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("%s: x%d(%d) vs x%d(%d) -> %v, next PC=%d", name, rs1, int32(rs1Val), rs2, int32(rs2Val), taken, nextPC),
		RegistersChanged: map[string]uint32{},
		MemoryChanged:    map[int]byte{},
		NextPC:           nextPC,
	}
}

// === JAL (Jump And Link) executor ===
//
// JAL stores the return address (PC + 4) in rd, then jumps to PC + offset.
// This is the primary mechanism for calling functions:
//
//	jal ra, function_label
//
// Here "ra" is the return address register (x1 by convention). The function
// can later return with: jalr x0, ra, 0 (jump to the saved return address).
func (e *RiscVExecutor) execJAL(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	imm := decoded.Fields["imm"]

	returnAddr := uint32(pc + 4)
	changes := writeRd(registers, rd, returnAddr)
	targetPC := pc + imm

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("jal: x%d = PC+4 = %d, jump to PC+%d = %d", rd, returnAddr, imm, targetPC),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           targetPC,
	}
}

// === JALR (Jump And Link Register) executor ===
//
// JALR stores PC+4 in rd, then jumps to (rs1 + imm) with the lowest bit
// cleared. The bit-clearing ensures the target is always 2-byte aligned.
//
// Common uses:
//
//	jalr x0, x1, 0   — return from function (jump to ra, discard link)
//	jalr x1, x5, 0   — indirect call through function pointer in x5
func (e *RiscVExecutor) execJALR(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	imm := decoded.Fields["imm"]

	returnAddr := uint32(pc + 4)
	// Target = (rs1 + imm) with bit 0 cleared
	target := int(int32(registers.Read(rs1))+int32(imm)) & ^1
	changes := writeRd(registers, rd, returnAddr)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("jalr: x%d = %d, jump to (x%d + %d) & ~1 = %d", rd, returnAddr, rs1, imm, target),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           target,
	}
}

// === LUI (Load Upper Immediate) executor ===
//
// LUI places a 20-bit immediate into the upper 20 bits of rd, with the
// lower 12 bits set to zero:
//
//	rd = imm << 12
//
// This is used as the first half of a two-instruction sequence to load
// a full 32-bit constant:
//
//	lui  x1, 0x12345     // x1 = 0x12345000
//	addi x1, x1, 0x678   // x1 = 0x12345678
func (e *RiscVExecutor) execLUI(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	imm := decoded.Fields["imm"]

	result := uint32(imm << 12)
	changes := writeRd(registers, rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("lui: x%d = 0x%x << 12 = 0x%08x", rd, imm&0xFFFFF, result),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === AUIPC (Add Upper Immediate to PC) executor ===
//
// AUIPC adds a 20-bit immediate (shifted left 12) to the current PC:
//
//	rd = PC + (imm << 12)
//
// This enables PC-relative addressing for data that is far away. Combined
// with addi or load instructions, it can reach any address in the 32-bit
// address space relative to the current instruction.
func (e *RiscVExecutor) execAUIPC(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	imm := decoded.Fields["imm"]

	result := uint32(pc) + uint32(imm<<12)
	changes := writeRd(registers, rd, result)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("auipc: x%d = PC(%d) + 0x%x << 12 = 0x%08x", rd, pc, imm&0xFFFFF, result),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === ecall executor ===
//
// ecall (environment call) triggers a software trap. In a full OS setup,
// this is how user programs request services from the kernel.
//
// Behavior depends on whether a trap handler is configured:
//
//	If mtvec != 0:  Raise a proper trap — save PC to mepc, set mcause,
//	                disable interrupts, jump to mtvec.
//
//	If mtvec == 0 and HostIO is configured: Handle the small host syscall ABI
//	                used by compiler end-to-end tests (write byte, read byte,
//	                exit).
//
//	If mtvec == 0:  Halt the CPU (legacy behavior for simple programs
//	                that don't set up a trap handler).
func (e *RiscVExecutor) execEcall(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	if e.CSR == nil {
		// No CSR file configured — simple halt behavior
		return cpu.ExecuteResult{
			Description:      "ecall: halt (no CSR file)",
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc,
			Halted:           true,
		}
	}

	mtvec := e.CSR.Read(CSRMtvec)
	if mtvec == 0 {
		if e.Host != nil {
			return e.execHostSyscall(registers, pc)
		}
		// No trap handler configured — halt as fallback
		return cpu.ExecuteResult{
			Description:      "ecall: halt (mtvec=0, no trap handler)",
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc,
			Halted:           true,
		}
	}

	// Raise a proper trap:
	// 1. Save current PC to mepc
	e.CSR.Write(CSRMepc, uint32(pc))
	// 2. Set trap cause (11 = ecall from M-mode)
	e.CSR.Write(CSRMcause, CauseEcallMMode)
	// 3. Disable interrupts (clear MIE bit in mstatus)
	mstatus := e.CSR.Read(CSRMstatus)
	e.CSR.Write(CSRMstatus, mstatus&^MIE)
	// 4. Jump to trap handler
	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("ecall: trap to mtvec=0x%x, mepc=%d, mcause=%d", mtvec, pc, CauseEcallMMode),
		RegistersChanged: map[string]uint32{},
		MemoryChanged:    map[int]byte{},
		NextPC:           int(mtvec),
	}
}

func (e *RiscVExecutor) execHostSyscall(registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	syscallNumber := registers.Read(17)
	switch syscallNumber {
	case SyscallWriteByte:
		value := byte(registers.Read(10) & 0xFF)
		e.Host.WriteByte(value)
		return cpu.ExecuteResult{
			Description:      fmt.Sprintf("ecall: host write byte 0x%02x", value),
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc + 4,
		}
	case SyscallReadByte:
		value := uint32(e.Host.ReadByte())
		registers.Write(10, value)
		return cpu.ExecuteResult{
			Description:      fmt.Sprintf("ecall: host read byte 0x%02x", value),
			RegistersChanged: map[string]uint32{"x10": value},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc + 4,
		}
	case SyscallExit:
		exitCode := registers.Read(10)
		e.Host.Exited = true
		e.Host.ExitCode = exitCode
		return cpu.ExecuteResult{
			Description:      fmt.Sprintf("ecall: host exit %d", exitCode),
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc,
			Halted:           true,
		}
	default:
		return cpu.ExecuteResult{
			Description:      fmt.Sprintf("ecall: unknown host syscall %d", syscallNumber),
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc,
			Halted:           true,
		}
	}
}

// === mret executor ===
//
// mret returns from a machine-mode trap handler by:
//  1. Restoring PC from mepc CSR
//  2. Re-enabling interrupts (set MIE bit in mstatus)
//
// After mret, execution resumes at the instruction that caused the trap
// (or the next one, depending on the trap type). For ecall, the trap
// handler typically increments mepc by 4 before executing mret so that
// execution resumes at the instruction after the ecall.
func (e *RiscVExecutor) execMret(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	if e.CSR == nil {
		return cpu.ExecuteResult{
			Description:      "mret: no CSR file configured",
			RegistersChanged: map[string]uint32{},
			MemoryChanged:    map[int]byte{},
			NextPC:           pc + 4,
		}
	}

	mepc := e.CSR.Read(CSRMepc)
	// Re-enable interrupts
	mstatus := e.CSR.Read(CSRMstatus)
	e.CSR.Write(CSRMstatus, mstatus|MIE)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("mret: return to mepc=0x%x, re-enable interrupts", mepc),
		RegistersChanged: map[string]uint32{},
		MemoryChanged:    map[int]byte{},
		NextPC:           int(mepc),
	}
}

// === CSRRW (CSR Read-Write) executor ===
//
// Atomically swaps the value in rs1 with the CSR:
//
//	old_csr = CSR[csr_addr]
//	CSR[csr_addr] = rs1
//	rd = old_csr
//
// If rd=x0, the read is still performed but the result is discarded.
func (e *RiscVExecutor) execCSRRW(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	csr := uint32(decoded.Fields["csr"])

	rs1Val := registers.Read(rs1)
	oldCSR := e.CSR.ReadWrite(csr, rs1Val)
	changes := writeRd(registers, rd, oldCSR)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("csrrw: x%d = CSR[0x%x](%d), CSR[0x%x] <- x%d(%d)", rd, csr, oldCSR, csr, rs1, rs1Val),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === CSRRS (CSR Read-Set) executor ===
//
// Reads the CSR, then sets bits specified by rs1:
//
//	old_csr = CSR[csr_addr]
//	CSR[csr_addr] = old_csr | rs1
//	rd = old_csr
//
// If rs1=x0 (value 0), no bits are set — this is a pure read (csrr pseudo-instruction).
func (e *RiscVExecutor) execCSRRS(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	csr := uint32(decoded.Fields["csr"])

	rs1Val := registers.Read(rs1)
	oldCSR := e.CSR.ReadSet(csr, rs1Val)
	changes := writeRd(registers, rd, oldCSR)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("csrrs: x%d = CSR[0x%x](%d), CSR[0x%x] |= x%d(%d)", rd, csr, oldCSR, csr, rs1, rs1Val),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}

// === CSRRC (CSR Read-Clear) executor ===
//
// Reads the CSR, then clears bits specified by rs1:
//
//	old_csr = CSR[csr_addr]
//	CSR[csr_addr] = old_csr & ~rs1
//	rd = old_csr
func (e *RiscVExecutor) execCSRRC(decoded cpu.DecodeResult, registers *cpu.RegisterFile, pc int) cpu.ExecuteResult {
	rd := decoded.Fields["rd"]
	rs1 := decoded.Fields["rs1"]
	csr := uint32(decoded.Fields["csr"])

	rs1Val := registers.Read(rs1)
	oldCSR := e.CSR.ReadClear(csr, rs1Val)
	changes := writeRd(registers, rd, oldCSR)

	return cpu.ExecuteResult{
		Description:      fmt.Sprintf("csrrc: x%d = CSR[0x%x](%d), CSR[0x%x] &^= x%d(%d)", rd, csr, oldCSR, csr, rs1, rs1Val),
		RegistersChanged: changes,
		MemoryChanged:    map[int]byte{},
		NextPC:           pc + 4,
	}
}
