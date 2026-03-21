package riscvsimulator

import (
	"testing"

	cpu "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
	"github.com/adhithyan15/coding-adventures/code/packages/go/core"
)

// =========================================================================
// Helpers
// =========================================================================

// nop encodes a RISC-V NOP instruction (addi x0, x0, 0).
//
// In a pipelined CPU, dependent instructions need padding between them
// to allow earlier results to reach the writeback stage before the next
// instruction reads them. This is the standard pattern used by the Core's
// own tests with the MockDecoder.
//
// === Why NOP padding? ===
//
// In a 5-stage pipeline (IF-ID-EX-MEM-WB), an instruction's result is
// written in WB (cycle N+4). The next instruction reads registers in ID
// (cycle N+2). Without forwarding/stalling, this creates a RAW hazard.
// The Core's hazard detection handles some forwarding, but back-to-back
// dependent instructions still need 2+ NOPs to ensure correct results.
var nop = EncodeAddi(0, 0, 0)

// assembleWithHalt converts RISC-V instructions into bytes, adding
// an ecall (halt) at the end so the core stops after the program.
func assembleWithHalt(instructions ...uint32) []byte {
	all := make([]uint32, len(instructions))
	copy(all, instructions)
	all = append(all, EncodeEcall())
	return Assemble(all)
}

// =========================================================================
// Test: Create a RISC-V core with simple config
// =========================================================================

func TestNewRiscVCore_CreatesSuccessfully(t *testing.T) {
	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}
	if c == nil {
		t.Fatal("NewRiscVCore returned nil core")
	}
}

func TestNewRiscVCore_RegisterFileConfig(t *testing.T) {
	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	// RISC-V should have 32 registers
	regFile := c.RegisterFile()
	if regFile.Count() != 32 {
		t.Errorf("expected 32 registers, got %d", regFile.Count())
	}

	// x0 should be hardwired to zero
	regFile.Write(0, 999)
	if got := regFile.Read(0); got != 0 {
		t.Errorf("x0 should always be 0, got %d", got)
	}
}

// =========================================================================
// Test: Load and run a small program — add two numbers
// =========================================================================

func TestRiscVCore_AddTwoNumbers(t *testing.T) {
	// Program: compute 10 + 32 = 42
	// NOP padding between dependent instructions allows pipeline writeback.
	program := assembleWithHalt(
		EncodeAddi(1, 0, 10),    // x1 = 10
		EncodeAddi(2, 0, 32),    // x2 = 32
		nop, nop,                // let x1, x2 reach WB
		EncodeAdd(3, 1, 2),      // x3 = x1 + x2 = 42
		nop, nop, nop, nop,      // let x3 reach WB
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	if !c.IsHalted() {
		t.Fatal("core should have halted after ecall")
	}

	if got := c.ReadRegister(1); got != 10 {
		t.Errorf("x1: expected 10, got %d", got)
	}
	if got := c.ReadRegister(2); got != 32 {
		t.Errorf("x2: expected 32, got %d", got)
	}
	if got := c.ReadRegister(3); got != 42 {
		t.Errorf("x3: expected 42 (10+32), got %d", got)
	}
}

// =========================================================================
// Test: Subtraction
// =========================================================================

func TestRiscVCore_Subtraction(t *testing.T) {
	// Program: compute 100 - 58 = 42
	program := assembleWithHalt(
		EncodeAddi(1, 0, 100),
		EncodeAddi(2, 0, 58),
		nop, nop,
		EncodeSub(3, 1, 2),
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	if got := c.ReadRegister(3); got != 42 {
		t.Errorf("x3: expected 42, got %d", got)
	}
}

// =========================================================================
// Test: Bitwise operations
// =========================================================================

func TestRiscVCore_BitwiseOperations(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 0xFF),  // x1 = 255
		EncodeAddi(2, 0, 0x0F),  // x2 = 15
		nop, nop,
		EncodeAnd(3, 1, 2),       // x3 = 0x0F
		EncodeOr(4, 1, 2),        // x4 = 0xFF
		EncodeXor(5, 1, 2),       // x5 = 0xF0
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	if got := c.ReadRegister(3); got != 0x0F {
		t.Errorf("x3 (AND): expected 0x0F, got 0x%X", got)
	}
	if got := c.ReadRegister(4); got != 0xFF {
		t.Errorf("x4 (OR): expected 0xFF, got 0x%X", got)
	}
	if got := c.ReadRegister(5); got != 0xF0 {
		t.Errorf("x5 (XOR): expected 0xF0, got 0x%X", got)
	}
}

// =========================================================================
// Test: LUI (Load Upper Immediate)
// =========================================================================

func TestRiscVCore_LUI(t *testing.T) {
	program := assembleWithHalt(
		EncodeLui(1, 0x12345),
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	expected := int(uint32(0x12345000))
	if got := c.ReadRegister(1); got != expected {
		t.Errorf("x1: expected 0x%X, got 0x%X", expected, got)
	}
}

// =========================================================================
// Test: Shift operations
// =========================================================================

func TestRiscVCore_Shifts(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 1),    // x1 = 1
		nop, nop, nop, nop,
		EncodeSlli(2, 1, 4),    // x2 = 1 << 4 = 16
		nop, nop, nop, nop,
		EncodeSrli(3, 2, 2),    // x3 = 16 >> 2 = 4
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(2000)

	if got := c.ReadRegister(2); got != 16 {
		t.Errorf("x2 (slli): expected 16, got %d", got)
	}
	if got := c.ReadRegister(3); got != 4 {
		t.Errorf("x3 (srli): expected 4, got %d", got)
	}
}

// =========================================================================
// Test: x0 always reads as zero
// =========================================================================

func TestRiscVCore_X0AlwaysZero(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(0, 0, 999),  // write to x0 — should be discarded
		nop, nop, nop, nop,
		EncodeAddi(1, 0, 7),    // x1 = 0 + 7 = 7
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	if got := c.ReadRegister(0); got != 0 {
		t.Errorf("x0: expected 0, got %d", got)
	}
	if got := c.ReadRegister(1); got != 7 {
		t.Errorf("x1: expected 7, got %d", got)
	}
}

// =========================================================================
// Test: ISA decoder interface compliance
// =========================================================================

func TestRiscVISADecoder_InstructionSize(t *testing.T) {
	decoder := NewRiscVISADecoder()
	if got := decoder.InstructionSize(); got != 4 {
		t.Errorf("InstructionSize: expected 4, got %d", got)
	}
}

func TestRiscVISADecoder_CSRAccessor(t *testing.T) {
	decoder := NewRiscVISADecoder()
	csr := decoder.CSR()
	if csr == nil {
		t.Fatal("CSR() returned nil")
	}
}

// =========================================================================
// Test: Sparse memory — map low RAM + high ROM
// =========================================================================

func TestNewRiscVCoreWithSparseMemory_CreatesSuccessfully(t *testing.T) {
	mem := cpu.NewSparseMemory([]cpu.MemoryRegion{
		{Base: 0x00000000, Size: 0x10000, Name: "RAM"},
		{Base: 0xFFFF0000, Size: 0x100, Name: "ROM", ReadOnly: true},
	})

	config := core.SimpleConfig()
	c, err := NewRiscVCoreWithSparseMemory(config, mem)
	if err != nil {
		t.Fatalf("NewRiscVCoreWithSparseMemory failed: %v", err)
	}
	if c == nil {
		t.Fatal("returned nil core")
	}

	// Verify RISC-V register config
	if c.RegisterFile().Count() != 32 {
		t.Errorf("expected 32 registers, got %d", c.RegisterFile().Count())
	}
}

func TestSparseMemory_UsedAsProgramStorage(t *testing.T) {
	// Create sparse memory with RAM at base 0
	mem := cpu.NewSparseMemory([]cpu.MemoryRegion{
		{Base: 0x00000000, Size: 0x10000, Name: "RAM"},
		{Base: 0xFFFF0000, Size: 0x100, Name: "ROM", ReadOnly: true},
	})

	// Load a program into sparse memory
	program := assembleWithHalt(EncodeAddi(1, 0, 42))
	mem.LoadBytes(0, program)

	// Verify the program was loaded
	word := mem.ReadWord(0)
	if word == 0 {
		t.Error("program was not loaded into sparse memory")
	}

	// High ROM should still be zero
	highByte := mem.ReadByte(0xFFFF0000)
	if highByte != 0 {
		t.Errorf("ROM should be zero, got 0x%02X", highByte)
	}
}

// =========================================================================
// Test: Core runs with default config
// =========================================================================

func TestRiscVCore_DefaultConfig(t *testing.T) {
	config := core.DefaultCoreConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore with DefaultCoreConfig failed: %v", err)
	}

	program := assembleWithHalt(
		EncodeAddi(1, 0, 5),
		EncodeAddi(2, 0, 3),
		nop, nop,
		EncodeAdd(3, 1, 2),
		nop, nop, nop, nop,
	)

	c.LoadProgram(program, 0)
	stats := c.Run(1000)

	if !c.IsHalted() {
		t.Fatal("core did not halt")
	}
	if got := c.ReadRegister(3); got != 8 {
		t.Errorf("x3: expected 8, got %d", got)
	}
	if stats.TotalCycles == 0 {
		t.Error("expected non-zero cycle count")
	}
}

// =========================================================================
// Test: Accumulator pattern (chained addi on same register)
// =========================================================================

func TestRiscVCore_AccumulatorPattern(t *testing.T) {
	// x1 = 10, then x1 = x1 + 20 = 30, then x1 = x1 + 12 = 42
	program := assembleWithHalt(
		EncodeAddi(1, 0, 10),
		nop, nop, nop, nop,
		EncodeAddi(1, 1, 20),
		nop, nop, nop, nop,
		EncodeAddi(1, 1, 12),
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(2000)

	if got := c.ReadRegister(1); got != 42 {
		t.Errorf("x1: expected 42, got %d", got)
	}
}

// =========================================================================
// Test: Set-less-than
// =========================================================================

func TestRiscVCore_SLT(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 5),
		EncodeAddi(2, 0, 10),
		nop, nop,
		EncodeSlt(3, 1, 2),   // 5 < 10 → 1
		EncodeSlt(4, 2, 1),   // 10 < 5 → 0
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("NewRiscVCore failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	if got := c.ReadRegister(3); got != 1 {
		t.Errorf("x3 (5<10): expected 1, got %d", got)
	}
	if got := c.ReadRegister(4); got != 0 {
		t.Errorf("x4 (10<5): expected 0, got %d", got)
	}
}

// =========================================================================
// Test: getField helper
// =========================================================================

func TestGetField_ExistingKey(t *testing.T) {
	fields := map[string]int{"rd": 5, "rs1": 3}
	if got := getField(fields, "rd", -1); got != 5 {
		t.Errorf("expected 5, got %d", got)
	}
}

func TestGetField_MissingKey(t *testing.T) {
	fields := map[string]int{"rd": 5}
	if got := getField(fields, "rs2", -1); got != -1 {
		t.Errorf("expected default -1, got %d", got)
	}
}

func TestGetField_EmptyMap(t *testing.T) {
	fields := map[string]int{}
	if got := getField(fields, "rd", 42); got != 42 {
		t.Errorf("expected default 42, got %d", got)
	}
}

// =========================================================================
// Test: Immediate arithmetic variants (ori, andi, xori)
// =========================================================================

func TestRiscVCore_ImmediateArithmetic(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 0xFF),   // x1 = 0xFF
		nop, nop, nop, nop,
		EncodeOri(2, 1, 0x100),   // x2 = 0xFF | 0x100 = 0x1FF
		nop, nop, nop, nop,
		EncodeAndi(3, 2, 0x0F0),  // x3 = 0x1FF & 0x0F0 = 0xF0
		EncodeXori(4, 1, 0x0F),   // x4 = 0xFF ^ 0x0F = 0xF0
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(2000)

	if got := c.ReadRegister(2); got != 0x1FF {
		t.Errorf("x2 (ori): expected 0x1FF, got 0x%X", got)
	}
	if got := c.ReadRegister(3); got != 0xF0 {
		t.Errorf("x3 (andi): expected 0xF0, got 0x%X", got)
	}
	if got := c.ReadRegister(4); got != 0xF0 {
		t.Errorf("x4 (xori): expected 0xF0, got 0x%X", got)
	}
}

// =========================================================================
// Test: decodeFieldsFromToken for CSR extraction
// =========================================================================

func TestDecodeFieldsFromToken_CSR(t *testing.T) {
	raw := EncodeCsrrw(1, 0x300, 2) // csrrw x1, 0x300, x2
	token := cpupipeline.NewToken()
	token.RawInstruction = int(raw)

	fields := decodeFieldsFromToken(token)
	if got := fields["csr"]; got != 0x300 {
		t.Errorf("expected CSR addr 0x300, got 0x%X", got)
	}
}

// =========================================================================
// Test: Core stats are populated
// =========================================================================

func TestRiscVCore_StatsPopulated(t *testing.T) {
	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	program := assembleWithHalt(
		EncodeAddi(1, 0, 42),
		nop, nop, nop, nop,
	)

	c.LoadProgram(program, 0)
	stats := c.Run(1000)

	if stats.TotalCycles == 0 {
		t.Error("expected non-zero total cycles")
	}
	if stats.InstructionsCompleted == 0 {
		t.Error("expected non-zero completed instructions")
	}
}

// =========================================================================
// Test: SLTI (set less than immediate)
// =========================================================================

func TestRiscVCore_SLTI(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 5),
		nop, nop, nop, nop,
		EncodeSlti(2, 1, 10),   // 5 < 10 → 1
		EncodeSlti(3, 1, 3),    // 5 < 3 → 0
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(2000)

	if got := c.ReadRegister(2); got != 1 {
		t.Errorf("x2 (5<10): expected 1, got %d", got)
	}
	if got := c.ReadRegister(3); got != 0 {
		t.Errorf("x3 (5<3): expected 0, got %d", got)
	}
}

// =========================================================================
// Test: SLTIU (set less than immediate, unsigned)
// =========================================================================

func TestRiscVCore_SLTIU(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 5),
		nop, nop, nop, nop,
		EncodeSltiu(2, 1, 10),  // 5 <u 10 → 1
		EncodeSltiu(3, 1, 3),   // 5 <u 3 → 0
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(2000)

	if got := c.ReadRegister(2); got != 1 {
		t.Errorf("x2: expected 1, got %d", got)
	}
	if got := c.ReadRegister(3); got != 0 {
		t.Errorf("x3: expected 0, got %d", got)
	}
}

// =========================================================================
// Test: AUIPC
// =========================================================================

func TestRiscVCore_AUIPC(t *testing.T) {
	program := assembleWithHalt(
		EncodeAuipc(1, 1),  // x1 = PC + (1 << 12) = 0 + 4096 = 4096
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	// AUIPC at PC=0: x1 = 0 + (1 << 12) = 4096
	if got := c.ReadRegister(1); got != 4096 {
		t.Errorf("x1: expected 4096, got %d", got)
	}
}

// =========================================================================
// Test: SRAI (arithmetic right shift immediate)
// =========================================================================

func TestRiscVCore_SRAI(t *testing.T) {
	// Load a negative number using LUI + ADDI, then arithmetic shift right.
	// -16 in 32-bit = 0xFFFFFFF0, arithmetic shift right by 2 = 0xFFFFFFFC = -4
	program := assembleWithHalt(
		EncodeAddi(1, 0, -16),  // x1 = -16 (0xFFFFFFF0)
		nop, nop, nop, nop,
		EncodeSrai(2, 1, 2),    // x2 = -16 >> 2 = -4 (arithmetic)
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(2000)

	// The Core's RegisterFile uses a 32-bit mask, so -4 is stored as
	// 0xFFFFFFFC. When read back as int, it's 4294967292 (unsigned).
	// We compare by masking to 32 bits.
	got := c.ReadRegister(2)
	expected := int(uint32(0xFFFFFFFC)) // -4 as unsigned 32-bit
	if got != expected {
		t.Errorf("x2 (srai): expected %d (0x%X), got %d (0x%X)", expected, uint32(expected), got, uint32(got))
	}
}

// =========================================================================
// Test: R-type register operations (SLL, SLTU, SRL, SRA)
// =========================================================================

func TestRiscVCore_RTypeShiftsAndCompares(t *testing.T) {
	program := assembleWithHalt(
		EncodeAddi(1, 0, 8),    // x1 = 8
		EncodeAddi(2, 0, 2),    // x2 = 2
		nop, nop,
		EncodeSll(3, 1, 2),     // x3 = 8 << 2 = 32
		EncodeSrl(4, 1, 2),     // x4 = 8 >> 2 = 2
		EncodeSltu(5, 2, 1),    // x5 = (2 <u 8) = 1
		nop, nop, nop, nop,
	)

	config := core.SimpleConfig()
	c, err := NewRiscVCore(config, 65536)
	if err != nil {
		t.Fatalf("failed: %v", err)
	}

	c.LoadProgram(program, 0)
	c.Run(1000)

	if got := c.ReadRegister(3); got != 32 {
		t.Errorf("x3 (sll): expected 32, got %d", got)
	}
	if got := c.ReadRegister(4); got != 2 {
		t.Errorf("x4 (srl): expected 2, got %d", got)
	}
	if got := c.ReadRegister(5); got != 1 {
		t.Errorf("x5 (sltu): expected 1, got %d", got)
	}
}

// =========================================================================
// Test: Decode control signals for various instruction types
// =========================================================================

func TestRiscVISADecoder_DecodeControlSignals(t *testing.T) {
	decoder := NewRiscVISADecoder()

	tests := []struct {
		name      string
		raw       uint32
		regWrite  bool
		memRead   bool
		memWrite  bool
		isBranch  bool
		isHalt    bool
	}{
		{"add", EncodeAdd(3, 1, 2), true, false, false, false, false},
		{"addi", EncodeAddi(1, 0, 42), true, false, false, false, false},
		{"lw", EncodeLw(1, 2, 0), true, true, false, false, false},
		{"sw", EncodeSw(1, 2, 0), false, false, true, false, false},
		{"beq", EncodeBeq(1, 2, 8), false, false, false, true, false},
		{"bne", EncodeBne(1, 2, 8), false, false, false, true, false},
		{"blt", EncodeBlt(1, 2, 8), false, false, false, true, false},
		{"bge", EncodeBge(1, 2, 8), false, false, false, true, false},
		{"bltu", EncodeBltu(1, 2, 8), false, false, false, true, false},
		{"bgeu", EncodeBgeu(1, 2, 8), false, false, false, true, false},
		{"jal", EncodeJal(1, 8), true, false, false, true, false},
		{"jalr", EncodeJalr(1, 2, 0), true, false, false, true, false},
		{"lui", EncodeLui(1, 0x12345), true, false, false, false, false},
		{"auipc", EncodeAuipc(1, 0x12345), true, false, false, false, false},
		{"ecall", EncodeEcall(), false, false, false, false, true},
		{"slli", EncodeSlli(1, 2, 3), true, false, false, false, false},
		{"srli", EncodeSrli(1, 2, 3), true, false, false, false, false},
		{"srai", EncodeSrai(1, 2, 3), true, false, false, false, false},
		{"sb", EncodeSb(1, 2, 0), false, false, true, false, false},
		{"sh", EncodeSh(1, 2, 0), false, false, true, false, false},
		{"lb", EncodeLb(1, 2, 0), true, true, false, false, false},
		{"lh", EncodeLh(1, 2, 0), true, true, false, false, false},
		{"lbu", EncodeLbu(1, 2, 0), true, true, false, false, false},
		{"lhu", EncodeLhu(1, 2, 0), true, true, false, false, false},
		{"slt", EncodeSlt(3, 1, 2), true, false, false, false, false},
		{"sltu", EncodeSltu(3, 1, 2), true, false, false, false, false},
		{"xor", EncodeXor(3, 1, 2), true, false, false, false, false},
		{"srl", EncodeSrl(3, 1, 2), true, false, false, false, false},
		{"sra", EncodeSra(3, 1, 2), true, false, false, false, false},
		{"or", EncodeOr(3, 1, 2), true, false, false, false, false},
		{"and", EncodeAnd(3, 1, 2), true, false, false, false, false},
		{"sub", EncodeSub(3, 1, 2), true, false, false, false, false},
		{"sll", EncodeSll(3, 1, 2), true, false, false, false, false},
		{"slti", EncodeSlti(1, 2, 5), true, false, false, false, false},
		{"sltiu", EncodeSltiu(1, 2, 5), true, false, false, false, false},
		{"xori", EncodeXori(1, 2, 5), true, false, false, false, false},
		{"ori", EncodeOri(1, 2, 5), true, false, false, false, false},
		{"andi", EncodeAndi(1, 2, 5), true, false, false, false, false},
		{"csrrw", EncodeCsrrw(1, 0x300, 2), true, false, false, false, false},
		{"csrrs", EncodeCsrrs(1, 0x300, 2), true, false, false, false, false},
		{"csrrc", EncodeCsrrc(1, 0x300, 2), true, false, false, false, false},
		{"mret", EncodeMret(), false, false, false, true, false},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			token := cpupipeline.NewToken()
			token.PC = 0
			decoder.Decode(int(tc.raw), token)

			if token.RegWrite != tc.regWrite {
				t.Errorf("%s: RegWrite expected %v, got %v", tc.name, tc.regWrite, token.RegWrite)
			}
			if token.MemRead != tc.memRead {
				t.Errorf("%s: MemRead expected %v, got %v", tc.name, tc.memRead, token.MemRead)
			}
			if token.MemWrite != tc.memWrite {
				t.Errorf("%s: MemWrite expected %v, got %v", tc.name, tc.memWrite, token.MemWrite)
			}
			if token.IsBranch != tc.isBranch {
				t.Errorf("%s: IsBranch expected %v, got %v", tc.name, tc.isBranch, token.IsBranch)
			}
			if token.IsHalt != tc.isHalt {
				t.Errorf("%s: IsHalt expected %v, got %v", tc.name, tc.isHalt, token.IsHalt)
			}
		})
	}
}

// =========================================================================
// Test: Execute various instruction types via direct decoder call
// =========================================================================

func TestRiscVISADecoder_ExecuteDirectly(t *testing.T) {
	decoder := NewRiscVISADecoder()
	regCfg := core.RegisterFileConfig{Count: 32, Width: 32, ZeroRegister: true}
	regFile := core.NewRegisterFile(&regCfg)

	// Set up register values for testing
	regFile.Write(1, 10)  // x1 = 10
	regFile.Write(2, 3)   // x2 = 3

	tests := []struct {
		name        string
		raw         uint32
		expectedALU int
	}{
		// R-type
		{"add", EncodeAdd(3, 1, 2), 13},
		{"sub", EncodeSub(3, 1, 2), 7},
		{"sll", EncodeSll(3, 1, 2), 80},     // 10 << 3 = 80
		{"srl", EncodeSrl(3, 1, 2), 1},      // 10 >> 3 = 1
		{"xor", EncodeXor(3, 1, 2), 10 ^ 3},
		{"or", EncodeOr(3, 1, 2), 10 | 3},
		{"and", EncodeAnd(3, 1, 2), 10 & 3},

		// I-type
		{"addi", EncodeAddi(3, 1, 5), 15},
		{"slli", EncodeSlli(3, 1, 2), 40},   // 10 << 2 = 40
		{"srli", EncodeSrli(3, 1, 1), 5},    // 10 >> 1 = 5

		// Upper immediate
		{"lui", EncodeLui(3, 1), 4096},       // 1 << 12 = 4096

		// Loads compute effective address
		{"lw", EncodeLw(3, 1, 4), 14},        // addr = 10 + 4 = 14

		// Stores compute effective address
		{"sw", EncodeSw(2, 1, 8), 18},        // addr = 10 + 8 = 18

		// Branches: beq(10, 3) is not taken
		{"beq_not_taken", EncodeBeq(1, 2, 100), 4}, // PC+4 since not taken
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			token := cpupipeline.NewToken()
			token.PC = 0
			decoder.Decode(int(tc.raw), token)
			decoder.Execute(token, regFile)

			if token.ALUResult != tc.expectedALU {
				t.Errorf("%s: ALUResult expected %d, got %d", tc.name, tc.expectedALU, token.ALUResult)
			}
		})
	}
}

// =========================================================================
// Test: Branch resolution through Execute
// =========================================================================

func TestRiscVISADecoder_BranchExecution(t *testing.T) {
	decoder := NewRiscVISADecoder()
	regCfg := core.RegisterFileConfig{Count: 32, Width: 32, ZeroRegister: true}
	regFile := core.NewRegisterFile(&regCfg)

	regFile.Write(1, 5)
	regFile.Write(2, 5)  // equal to x1
	regFile.Write(3, 10) // greater than x1

	tests := []struct {
		name   string
		raw    uint32
		taken  bool
		target int
	}{
		{"beq_taken", EncodeBeq(1, 2, 20), true, 20},
		{"beq_not_taken", EncodeBeq(1, 3, 20), false, 0},
		{"bne_taken", EncodeBne(1, 3, 20), true, 20},
		{"bne_not_taken", EncodeBne(1, 2, 20), false, 0},
		{"blt_taken", EncodeBlt(1, 3, 20), true, 20},
		{"blt_not_taken", EncodeBlt(3, 1, 20), false, 0},
		{"bge_taken", EncodeBge(3, 1, 20), true, 20},
		{"bge_not_taken", EncodeBge(1, 3, 20), false, 0},
		{"bltu_taken", EncodeBltu(1, 3, 20), true, 20},
		{"bgeu_taken", EncodeBgeu(3, 1, 20), true, 20},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			token := cpupipeline.NewToken()
			token.PC = 0
			decoder.Decode(int(tc.raw), token)
			decoder.Execute(token, regFile)

			if token.BranchTaken != tc.taken {
				t.Errorf("%s: BranchTaken expected %v, got %v", tc.name, tc.taken, token.BranchTaken)
			}
			if tc.taken && token.BranchTarget != tc.target {
				t.Errorf("%s: BranchTarget expected %d, got %d", tc.name, tc.target, token.BranchTarget)
			}
		})
	}
}

// =========================================================================
// Test: JAL/JALR execution
// =========================================================================

func TestRiscVISADecoder_JumpExecution(t *testing.T) {
	decoder := NewRiscVISADecoder()
	regCfg := core.RegisterFileConfig{Count: 32, Width: 32, ZeroRegister: true}
	regFile := core.NewRegisterFile(&regCfg)

	regFile.Write(5, 100)  // x5 = 100 (base for jalr)

	// JAL: jump to PC + 20, store PC+4 in rd
	t.Run("jal", func(t *testing.T) {
		token := cpupipeline.NewToken()
		token.PC = 8
		decoder.Decode(int(EncodeJal(1, 20)), token)
		decoder.Execute(token, regFile)

		if !token.BranchTaken {
			t.Error("JAL should always be taken")
		}
		if token.BranchTarget != 28 { // PC(8) + 20
			t.Errorf("target: expected 28, got %d", token.BranchTarget)
		}
		if token.WriteData != 12 { // PC(8) + 4
			t.Errorf("return addr: expected 12, got %d", token.WriteData)
		}
	})

	// JALR: jump to (x5 + 8) & ~1, store PC+4 in rd
	t.Run("jalr", func(t *testing.T) {
		token := cpupipeline.NewToken()
		token.PC = 16
		decoder.Decode(int(EncodeJalr(1, 5, 8)), token)
		decoder.Execute(token, regFile)

		if !token.BranchTaken {
			t.Error("JALR should always be taken")
		}
		if token.BranchTarget != 108 { // (100 + 8) & ~1 = 108
			t.Errorf("target: expected 108, got %d", token.BranchTarget)
		}
		if token.WriteData != 20 { // PC(16) + 4
			t.Errorf("return addr: expected 20, got %d", token.WriteData)
		}
	})
}

// =========================================================================
// Test: SRA (arithmetic right shift via register)
// =========================================================================

func TestRiscVISADecoder_SRA(t *testing.T) {
	decoder := NewRiscVISADecoder()
	regCfg := core.RegisterFileConfig{Count: 32, Width: 32, ZeroRegister: true}
	regFile := core.NewRegisterFile(&regCfg)

	regFile.Write(1, int(int32(-16))) // x1 = -16
	regFile.Write(2, 2)               // x2 = 2

	token := cpupipeline.NewToken()
	token.PC = 0
	decoder.Decode(int(EncodeSra(3, 1, 2)), token)
	decoder.Execute(token, regFile)

	expected := int(int32(-4)) // -16 >> 2 = -4 (arithmetic)
	if token.ALUResult != expected {
		t.Errorf("SRA: expected %d, got %d", expected, token.ALUResult)
	}
}

// =========================================================================
// Test: Unknown instruction defaults to NOP
// =========================================================================

func TestRiscVISADecoder_UnknownInstruction(t *testing.T) {
	decoder := NewRiscVISADecoder()
	regCfg := core.RegisterFileConfig{Count: 32, Width: 32, ZeroRegister: true}
	regFile := core.NewRegisterFile(&regCfg)

	// Raw instruction with unknown opcode bits
	token := cpupipeline.NewToken()
	token.PC = 0
	token.Opcode = "SOMETHING_UNKNOWN"
	// Execute with unknown opcode should not panic
	decoder.Execute(token, regFile)
	// Just verifying it doesn't crash
}

// =========================================================================
// Test: SLT / SLTU through Execute
// =========================================================================

func TestRiscVISADecoder_SLT_Execute(t *testing.T) {
	decoder := NewRiscVISADecoder()
	regCfg := core.RegisterFileConfig{Count: 32, Width: 32, ZeroRegister: true}
	regFile := core.NewRegisterFile(&regCfg)

	regFile.Write(1, 5)
	regFile.Write(2, 10)

	// slt: 5 < 10 → 1
	token := cpupipeline.NewToken()
	token.PC = 0
	decoder.Decode(int(EncodeSlt(3, 1, 2)), token)
	decoder.Execute(token, regFile)
	if token.ALUResult != 1 {
		t.Errorf("slt(5,10): expected 1, got %d", token.ALUResult)
	}

	// sltu: 5 <u 10 → 1
	token2 := cpupipeline.NewToken()
	token2.PC = 0
	decoder.Decode(int(EncodeSltu(3, 1, 2)), token2)
	decoder.Execute(token2, regFile)
	if token2.ALUResult != 1 {
		t.Errorf("sltu(5,10): expected 1, got %d", token2.ALUResult)
	}
}
