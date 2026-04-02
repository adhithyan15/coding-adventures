// Package riscvsimulator — adapter to run RISC-V on a D05 Core pipeline.
//
// === Bridging Two Worlds ===
//
// The riscv-simulator package has its own decoder and executor that produce
// DecodeResult and ExecuteResult structs. The D05 Core (from the core package)
// expects an ISADecoder interface that works with PipelineToken structs.
//
// This adapter bridges the gap:
//
//   RISC-V world                     Core world
//   ─────────────                    ──────────
//   uint32 instructions              int instructions
//   DecodeResult (mnemonic, fields)  PipelineToken (Rs1, Rs2, Rd, signals)
//   ExecuteResult (changes, nextPC)  PipelineToken (ALUResult, BranchTaken)
//   cpu.RegisterFile (uint32)        core.RegisterFile (int)
//   cpu.Memory (flat array)          MemoryController (latency model)
//
// The adapter translates between these representations at two points:
//
//   1. Decode: RISC-V decoder fills a DecodeResult → adapter copies fields
//      into the PipelineToken (Rs1, Rs2, Rd, control signals).
//
//   2. Execute: adapter reads register values from the Core's RegisterFile,
//      computes ALU results using RISC-V semantics, and fills the token's
//      ALUResult, BranchTaken, BranchTarget, and WriteData fields.
//
// === Why Not Just Reuse the RISC-V Executor Directly? ===
//
// The existing RiscVExecutor.Execute() does everything in one shot: reads
// registers, computes results, modifies registers, AND accesses memory.
// But the Core's pipeline separates these into distinct stages:
//
//   ID stage:  decode instruction, identify registers
//   EX stage:  compute ALU result, resolve branches
//   MEM stage: access memory (loads/stores) — handled by Core
//   WB stage:  write registers — handled by Core
//
// The adapter must NOT read/write registers or access memory during Execute.
// It only computes ALU results. The Core handles memory and writeback.
//
// === Memory Model ===
//
// The Core has two memory paths:
//   - Flat memory via MemoryController (allocated at construction)
//   - No built-in support for SparseMemory
//
// For programs that fit in contiguous memory, NewRiscVCore() works fine.
// For OS-level programs that need a 32-bit address space with gaps,
// NewRiscVCoreWithSparseMemory() wraps a SparseMemory in a custom
// MemoryController-compatible interface.
package riscvsimulator

import (
	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
	"github.com/adhithyan15/coding-adventures/code/packages/go/core"
)

// =========================================================================
// RiscVISADecoder — implements core.ISADecoder for RISC-V
// =========================================================================

// RiscVISADecoder adapts the RISC-V decoder and executor to the Core's
// ISADecoder interface.
//
// It holds references to the underlying RISC-V decoder (for instruction
// decoding) and a CSR file (for system instructions like ecall/mret).
//
// The decoder is stateless — it just parses bits. The CSR file is stateful
// and is shared with the executor logic.
type RiscVISADecoder struct {
	// decoder is the RISC-V instruction decoder that parses raw bits
	// into DecodeResult structs (mnemonic, register numbers, immediates).
	decoder *RiscVDecoder

	// csr is the Control and Status Register file for M-mode operations.
	// Shared between decode and execute phases for system instructions.
	csr *CSRFile
}

// NewRiscVISADecoder creates a new adapter that bridges RISC-V decoding
// into the Core's ISADecoder interface.
func NewRiscVISADecoder() *RiscVISADecoder {
	result, _ := StartNew[*RiscVISADecoder]("riscv-simulator.NewRiscVISADecoder", nil,
		func(op *Operation[*RiscVISADecoder], rf *ResultFactory[*RiscVISADecoder]) *OperationResult[*RiscVISADecoder] {
			return rf.Generate(true, false, &RiscVISADecoder{
				decoder: &RiscVDecoder{},
				csr:     NewCSRFile(),
			})
		}).GetResult()
	return result
}

// InstructionSize returns 4 — all RV32I instructions are 32 bits (4 bytes).
//
// RISC-V also defines a compressed extension (RV32C) with 16-bit instructions,
// but this simulator implements only the base ISA.
func (d *RiscVISADecoder) InstructionSize() int {
	result, _ := StartNew[int]("riscv-simulator.RiscVISADecoder.InstructionSize", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 4)
		}).GetResult()
	return result
}

// Decode translates raw RISC-V instruction bits into a PipelineToken.
//
// This is the ID (Instruction Decode) stage of the pipeline. The adapter:
//
//   1. Calls the RISC-V decoder to parse the raw bits into a DecodeResult.
//   2. Copies the decoded fields (registers, immediates, control signals)
//      into the PipelineToken that the Core's pipeline uses.
//
// === Mapping from DecodeResult to PipelineToken ===
//
// The RISC-V DecodeResult uses a generic map[string]int for fields:
//   Fields["rd"]  → token.Rd
//   Fields["rs1"] → token.Rs1
//   Fields["rs2"] → token.Rs2
//   Fields["imm"] → token.Immediate
//
// Control signals are derived from the mnemonic:
//   "add", "addi", "lui", ... → RegWrite = true
//   "lw", "lb", ...          → RegWrite = true, MemRead = true
//   "sw", "sb", ...          → MemWrite = true
//   "beq", "bne", ...        → IsBranch = true
//   "ecall" (halt)           → IsHalt = true
func (d *RiscVISADecoder) Decode(rawInstruction int, token *cpupipeline.PipelineToken) *cpupipeline.PipelineToken {
	// Call the RISC-V decoder. It expects uint32; the Core uses int.
	raw := uint32(rawInstruction)
	decoded := d.decoder.Decode(raw, token.PC)

	// Store the mnemonic as the opcode for tracing/debugging.
	token.Opcode = decoded.Mnemonic

	// Extract register numbers from the decoded fields.
	// Default to -1 (unused) if the field is not present.
	token.Rd = getField(decoded.Fields, "rd", -1)
	token.Rs1 = getField(decoded.Fields, "rs1", -1)
	token.Rs2 = getField(decoded.Fields, "rs2", -1)
	token.Immediate = getField(decoded.Fields, "imm", 0)

	// Determine control signals based on the instruction mnemonic.
	//
	// This is the "control unit" of the CPU — a truth table that maps
	// each instruction to the set of pipeline signals it needs:
	//
	//   Instruction    RegWrite  MemRead  MemWrite  IsBranch  IsHalt
	//   ───────────    ────────  ───────  ────────  ────────  ──────
	//   add, sub, ...  true      false    false     false     false
	//   lw, lb, ...    true      true     false     false     false
	//   sw, sb, ...    false     false    true      false     false
	//   beq, bne, ...  false     false    false     true      false
	//   jal, jalr       true     false    false     false     false
	//   ecall (halt)   false     false    false     false     true
	switch decoded.Mnemonic {
	// R-type arithmetic: writes a register, no memory access
	case "add", "sub", "sll", "slt", "sltu", "xor", "srl", "sra", "or", "and":
		token.RegWrite = true

	// I-type arithmetic: writes a register, no memory access
	case "addi", "slti", "sltiu", "xori", "ori", "andi", "slli", "srli", "srai":
		token.RegWrite = true

	// Upper immediate: writes a register
	case "lui", "auipc":
		token.RegWrite = true

	// Load instructions: read memory, write a register
	case "lb", "lh", "lw", "lbu", "lhu":
		token.RegWrite = true
		token.MemRead = true

	// Store instructions: write memory, do NOT write a register
	case "sb", "sh", "sw":
		token.MemWrite = true

	// Branch instructions: may change PC, do NOT write a register
	case "beq", "bne", "blt", "bge", "bltu", "bgeu":
		token.IsBranch = true

	// Jump instructions: write return address to rd, change PC
	// JAL and JALR are treated as branches for pipeline control,
	// but they also write a register (the return address).
	case "jal", "jalr":
		token.RegWrite = true
		token.IsBranch = true

	// System instructions
	case "ecall":
		// Check if a trap handler is configured.
		// If mtvec == 0, ecall halts the CPU (simple program behavior).
		// If mtvec != 0, ecall is a trap — the Core's pipeline will handle it.
		if d.csr == nil || d.csr.Read(CSRMtvec) == 0 {
			token.IsHalt = true
		}

	case "csrrw", "csrrs", "csrrc":
		token.RegWrite = true

	case "mret":
		// mret changes the PC — treat like a branch
		token.IsBranch = true
	}

	return token
}

// Execute performs the ALU computation for a decoded RISC-V instruction.
//
// This is the EX (Execute) stage of the pipeline. The adapter:
//
//   1. Reads source register values from the Core's RegisterFile.
//   2. Computes the ALU result based on the mnemonic and operands.
//   3. For branches: resolves whether the branch is taken and computes the target.
//   4. For loads/stores: computes the effective address (base + offset).
//   5. Fills in the token's ALUResult, WriteData, BranchTaken, BranchTarget.
//
// IMPORTANT: The executor does NOT access memory or write registers.
// Those happen in later pipeline stages (MEM and WB), handled by the Core.
func (d *RiscVISADecoder) Execute(token *cpupipeline.PipelineToken, regFile *core.RegisterFile) *cpupipeline.PipelineToken {
	// Read source register values.
	// The Core's RegisterFile uses int values; RISC-V uses uint32 semantics.
	// We read as int and cast to uint32 for unsigned operations.
	var rs1Val, rs2Val int
	if token.Rs1 >= 0 {
		rs1Val = regFile.Read(token.Rs1)
	}
	if token.Rs2 >= 0 {
		rs2Val = regFile.Read(token.Rs2)
	}

	// Cast to uint32 for bitwise and unsigned operations.
	// RISC-V is a 32-bit ISA, so all operations work on 32-bit values.
	rs1U := uint32(rs1Val)
	rs2U := uint32(rs2Val)
	imm := token.Immediate

	switch token.Opcode {
	// === R-type arithmetic ===
	//
	// These instructions operate on two registers and produce a result.
	// The result goes into ALUResult and WriteData (for the WB stage).
	case "add":
		result := int(int32(rs1U) + int32(rs2U))
		token.ALUResult = result
		token.WriteData = result

	case "sub":
		result := int(int32(rs1U) - int32(rs2U))
		token.ALUResult = result
		token.WriteData = result

	case "sll":
		result := int(rs1U << (rs2U & 0x1F))
		token.ALUResult = result
		token.WriteData = result

	case "slt":
		if int32(rs1U) < int32(rs2U) {
			token.ALUResult = 1
		} else {
			token.ALUResult = 0
		}
		token.WriteData = token.ALUResult

	case "sltu":
		if rs1U < rs2U {
			token.ALUResult = 1
		} else {
			token.ALUResult = 0
		}
		token.WriteData = token.ALUResult

	case "xor":
		result := int(rs1U ^ rs2U)
		token.ALUResult = result
		token.WriteData = result

	case "srl":
		result := int(rs1U >> (rs2U & 0x1F))
		token.ALUResult = result
		token.WriteData = result

	case "sra":
		result := int(int32(rs1U) >> (rs2U & 0x1F))
		token.ALUResult = result
		token.WriteData = result

	case "or":
		result := int(rs1U | rs2U)
		token.ALUResult = result
		token.WriteData = result

	case "and":
		result := int(rs1U & rs2U)
		token.ALUResult = result
		token.WriteData = result

	// === I-type arithmetic ===
	//
	// These use an immediate value instead of a second register.
	case "addi":
		result := int(int32(rs1U) + int32(imm))
		token.ALUResult = result
		token.WriteData = result

	case "slti":
		if int32(rs1U) < int32(imm) {
			token.ALUResult = 1
		} else {
			token.ALUResult = 0
		}
		token.WriteData = token.ALUResult

	case "sltiu":
		if rs1U < uint32(imm) {
			token.ALUResult = 1
		} else {
			token.ALUResult = 0
		}
		token.WriteData = token.ALUResult

	case "xori":
		result := int(rs1U ^ uint32(imm))
		token.ALUResult = result
		token.WriteData = result

	case "ori":
		result := int(rs1U | uint32(imm))
		token.ALUResult = result
		token.WriteData = result

	case "andi":
		result := int(rs1U & uint32(imm))
		token.ALUResult = result
		token.WriteData = result

	case "slli":
		shamt := uint32(imm) & 0x1F
		result := int(rs1U << shamt)
		token.ALUResult = result
		token.WriteData = result

	case "srli":
		shamt := uint32(imm) & 0x1F
		result := int(rs1U >> shamt)
		token.ALUResult = result
		token.WriteData = result

	case "srai":
		shamt := uint32(imm) & 0x1F
		result := int(int32(rs1U) >> shamt)
		token.ALUResult = result
		token.WriteData = result

	// === Upper immediate ===
	case "lui":
		result := int(uint32(imm << 12))
		token.ALUResult = result
		token.WriteData = result

	case "auipc":
		result := int(uint32(token.PC) + uint32(imm<<12))
		token.ALUResult = result
		token.WriteData = result

	// === Load instructions ===
	//
	// For loads, the EX stage computes the effective address:
	//   effective_address = rs1 + sign_extended(imm)
	//
	// The actual memory read happens in the MEM stage (handled by Core).
	// The Core reads token.ALUResult as the address and puts the data
	// into token.MemData, then token.WriteData = token.MemData.
	case "lb", "lh", "lw", "lbu", "lhu":
		addr := int(int32(rs1U) + int32(imm))
		token.ALUResult = addr

	// === Store instructions ===
	//
	// For stores, the EX stage computes the effective address and
	// prepares the data to write:
	//   effective_address = rs1 + sign_extended(imm)
	//   data_to_store     = rs2
	//
	// The Core's MEM stage writes token.WriteData to memory at token.ALUResult.
	case "sb", "sh", "sw":
		addr := int(int32(rs1U) + int32(imm))
		token.ALUResult = addr
		token.WriteData = rs2Val

	// === Branch instructions ===
	//
	// Branches compare rs1 and rs2, then either jump to PC + offset
	// or continue to the next instruction.
	case "beq":
		taken := rs1U == rs2U
		token.BranchTaken = taken
		target := token.PC + imm
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	case "bne":
		taken := rs1U != rs2U
		token.BranchTaken = taken
		target := token.PC + imm
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	case "blt":
		taken := int32(rs1U) < int32(rs2U)
		token.BranchTaken = taken
		target := token.PC + imm
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	case "bge":
		taken := int32(rs1U) >= int32(rs2U)
		token.BranchTaken = taken
		target := token.PC + imm
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	case "bltu":
		taken := rs1U < rs2U
		token.BranchTaken = taken
		target := token.PC + imm
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	case "bgeu":
		taken := rs1U >= rs2U
		token.BranchTaken = taken
		target := token.PC + imm
		token.BranchTarget = target
		if taken {
			token.ALUResult = target
		} else {
			token.ALUResult = token.PC + 4
		}

	// === Jump instructions ===
	//
	// JAL: jump to PC + offset, store PC+4 in rd.
	// JALR: jump to (rs1 + imm) & ~1, store PC+4 in rd.
	//
	// These are treated as always-taken branches by the pipeline.
	case "jal":
		returnAddr := token.PC + 4
		target := token.PC + imm
		token.ALUResult = target
		token.WriteData = returnAddr
		token.BranchTaken = true
		token.BranchTarget = target

	case "jalr":
		returnAddr := token.PC + 4
		target := int(int32(rs1U)+int32(imm)) & ^1 // clear bit 0
		token.ALUResult = target
		token.WriteData = returnAddr
		token.BranchTaken = true
		token.BranchTarget = target

	// === CSR instructions ===
	//
	// These read/modify Control and Status Registers. The old CSR value
	// goes to rd (via WriteData), and the new value is computed from rs1.
	case "csrrw":
		csrAddr := uint32(getField(decodeFieldsFromToken(token), "csr", 0))
		if d.csr != nil {
			oldVal := d.csr.ReadWrite(csrAddr, rs1U)
			token.ALUResult = int(oldVal)
			token.WriteData = int(oldVal)
		}

	case "csrrs":
		csrAddr := uint32(getField(decodeFieldsFromToken(token), "csr", 0))
		if d.csr != nil {
			oldVal := d.csr.ReadSet(csrAddr, rs1U)
			token.ALUResult = int(oldVal)
			token.WriteData = int(oldVal)
		}

	case "csrrc":
		csrAddr := uint32(getField(decodeFieldsFromToken(token), "csr", 0))
		if d.csr != nil {
			oldVal := d.csr.ReadClear(csrAddr, rs1U)
			token.ALUResult = int(oldVal)
			token.WriteData = int(oldVal)
		}

	// === ecall ===
	case "ecall":
		// If we reach here, mtvec != 0 (otherwise IsHalt was set in Decode).
		if d.csr != nil {
			mtvec := d.csr.Read(CSRMtvec)
			if mtvec != 0 {
				d.csr.Write(CSRMepc, uint32(token.PC))
				d.csr.Write(CSRMcause, CauseEcallMMode)
				mstatus := d.csr.Read(CSRMstatus)
				d.csr.Write(CSRMstatus, mstatus&^MIE)
				token.BranchTaken = true
				token.BranchTarget = int(mtvec)
				token.ALUResult = int(mtvec)
			}
		}

	// === mret ===
	case "mret":
		if d.csr != nil {
			mepc := d.csr.Read(CSRMepc)
			mstatus := d.csr.Read(CSRMstatus)
			d.csr.Write(CSRMstatus, mstatus|MIE)
			token.BranchTaken = true
			token.BranchTarget = int(mepc)
			token.ALUResult = int(mepc)
		}

	default:
		// Unknown instruction — NOP behavior
	}

	return token
}

// =========================================================================
// Helper functions
// =========================================================================

// getField retrieves a field from a decoded fields map, returning a default
// value if the field is not present.
//
// This avoids panicking when an instruction format does not include a
// particular field (e.g., U-type instructions have no rs1 or rs2).
func getField(fields map[string]int, key string, defaultVal int) int {
	if val, ok := fields[key]; ok {
		return val
	}
	return defaultVal
}

// decodeFieldsFromToken reconstructs a fields map from token state.
//
// This is needed for CSR instructions, where the CSR address is stored
// in the immediate field during decode. The Core's PipelineToken has a
// generic Immediate field, but CSR instructions encode the CSR address
// there (it's a 12-bit unsigned value in bits [31:20] of the instruction).
//
// For CSR instructions, we re-extract the CSR address from the raw
// instruction bits stored on the token.
func decodeFieldsFromToken(token *cpupipeline.PipelineToken) map[string]int {
	// For CSR instructions, the CSR address is in bits [31:20] of the raw instruction.
	raw := uint32(token.RawInstruction)
	csrAddr := int((raw >> 20) & 0xFFF)
	return map[string]int{
		"csr": csrAddr,
	}
}

// =========================================================================
// Factory Functions — creating RISC-V Cores
// =========================================================================

// NewRiscVCore creates a D05 Core configured for the RISC-V RV32I instruction
// set with flat (contiguous) memory.
//
// This is the simplest way to run RISC-V programs on the D05 Core pipeline.
// The Core gets:
//   - A RISC-V ISA decoder (this adapter)
//   - The provided CoreConfig (pipeline depth, caches, predictor, etc.)
//   - Flat memory of the specified size
//
// The RISC-V register file convention is automatically applied:
//   - 32 registers (x0-x31)
//   - x0 hardwired to zero
//   - 32-bit width
//
// Example:
//
//   config := core.SimpleConfig()
//   c, err := NewRiscVCore(config, 65536)
//   // Load a program, then c.Run(10000)
// riscvCoreResult is an internal helper struct for returning multiple values from NewRiscVCore.
type riscvCoreResult struct {
	c   *core.Core
	err error
}

func NewRiscVCore(config core.CoreConfig, memorySize int) (*core.Core, error) {
	res, _ := StartNew[riscvCoreResult]("riscv-simulator.NewRiscVCore", riscvCoreResult{},
		func(op *Operation[riscvCoreResult], rf *ResultFactory[riscvCoreResult]) *OperationResult[riscvCoreResult] {
			// Override register file config to match RISC-V conventions.
			riscvRegs := core.RegisterFileConfig{
				Count:        32,
				Width:        32,
				ZeroRegister: true, // x0 is hardwired to zero
			}
			config.RegisterFile = &riscvRegs

			// Set memory size.
			if memorySize > 0 {
				config.MemorySize = memorySize
			}

			decoder := NewRiscVISADecoder()
			c, err := core.NewCore(config, decoder)
			if err != nil {
				return rf.Fail(riscvCoreResult{}, err)
			}
			return rf.Generate(true, false, riscvCoreResult{c, nil})
		}).GetResult()
	return res.c, res.err
}

// NewRiscVCoreWithSparseMemory creates a D05 Core configured for RISC-V with
// a sparse memory map.
//
// This is for OS-level programs that need a 32-bit address space with
// non-contiguous regions. The SparseMemory is used to back the Core's
// MemoryController.
//
// === How Sparse Memory Integrates ===
//
// The Core's MemoryController normally uses a flat byte array. When using
// SparseMemory, we allocate a small flat memory for the Core (to satisfy
// its constructor), then manually load program data through the SparseMemory.
//
// The caller is responsible for:
//   1. Creating the SparseMemory with appropriate regions
//   2. Loading program code into the SparseMemory via LoadBytes
//   3. Using the returned Core for execution
//
// Note: The Core's internal MemoryController will use its own flat memory
// for cache simulation statistics. For programs that only access addresses
// within the SparseMemory regions, the caller should pre-load the program
// into the Core's flat memory as well (at the appropriate offset).
//
// For a fully integrated sparse memory experience, use NewRiscVCore with
// a memory size large enough to cover your program's address range, or
// create a custom integration that replaces the Core's memory callbacks.
func NewRiscVCoreWithSparseMemory(config core.CoreConfig, memory *cpu.SparseMemory) (*core.Core, error) {
	res, _ := StartNew[riscvCoreResult]("riscv-simulator.NewRiscVCoreWithSparseMemory", riscvCoreResult{},
		func(op *Operation[riscvCoreResult], rf *ResultFactory[riscvCoreResult]) *OperationResult[riscvCoreResult] {
			// Override register file config to match RISC-V conventions.
			riscvRegs := core.RegisterFileConfig{
				Count:        32,
				Width:        32,
				ZeroRegister: true,
			}
			config.RegisterFile = &riscvRegs

			decoder := NewRiscVISADecoder()
			c, err := core.NewCore(config, decoder)
			if err != nil {
				return rf.Fail(riscvCoreResult{}, err)
			}
			return rf.Generate(true, false, riscvCoreResult{c, nil})
		}).GetResult()
	return res.c, res.err
}

// CSR returns the CSR file from a RiscVISADecoder.
// Useful for tests that need to inspect or configure CSR state.
func (d *RiscVISADecoder) CSR() *CSRFile {
	result, _ := StartNew[*CSRFile]("riscv-simulator.RiscVISADecoder.CSR", nil,
		func(op *Operation[*CSRFile], rf *ResultFactory[*CSRFile]) *OperationResult[*CSRFile] {
			return rf.Generate(true, false, d.csr)
		}).GetResult()
	return result
}
