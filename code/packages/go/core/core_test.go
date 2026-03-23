package core

import (
	"testing"

	branchpredictor "github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor"
	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

// =========================================================================
// Test Helpers
// =========================================================================

// makeSimpleCore creates a Core with the SimpleConfig and MockDecoder.
// This is the default for most tests.
func makeSimpleCore(t *testing.T) *Core {
	t.Helper()
	c, err := NewCore(SimpleConfig(), NewMockDecoder())
	if err != nil {
		t.Fatalf("failed to create simple core: %v", err)
	}
	return c
}

// makeDefaultCore creates a Core with DefaultCoreConfig and MockDecoder.
func makeDefaultCore(t *testing.T) *Core {
	t.Helper()
	c, err := NewCore(DefaultCoreConfig(), NewMockDecoder())
	if err != nil {
		t.Fatalf("failed to create default core: %v", err)
	}
	return c
}

// =========================================================================
// Core Assembly Tests
// =========================================================================

// TestCoreConstruction verifies that a Core initializes all sub-components
// from a config without error.
func TestCoreConstruction(t *testing.T) {
	c := makeSimpleCore(t)

	if c.pipeline == nil {
		t.Error("pipeline should not be nil")
	}
	if c.predictor == nil {
		t.Error("predictor should not be nil")
	}
	if c.btb == nil {
		t.Error("BTB should not be nil")
	}
	if c.hazardUnit == nil {
		t.Error("hazard unit should not be nil")
	}
	if c.cacheHierarchy == nil {
		t.Error("cache hierarchy should not be nil")
	}
	if c.regFile == nil {
		t.Error("register file should not be nil")
	}
	if c.memCtrl == nil {
		t.Error("memory controller should not be nil")
	}
	if c.clk == nil {
		t.Error("clock should not be nil")
	}
}

// TestSimpleConfigRuns verifies that a core with SimpleConfig runs without error.
func TestSimpleConfigRuns(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(EncodeHALT())
	c.LoadProgram(program, 0)
	stats := c.Run(100)

	if stats.TotalCycles == 0 {
		t.Error("expected at least one cycle")
	}
}

// TestComplexConfigRuns verifies that a CortexA78-like config runs without error.
func TestComplexConfigRuns(t *testing.T) {
	config := CortexA78LikeConfig()
	c, err := NewCore(config, NewMockDecoder())
	if err != nil {
		t.Fatalf("failed to create Cortex-A78-like core: %v", err)
	}

	program := EncodeProgram(EncodeHALT())
	c.LoadProgram(program, 0)
	stats := c.Run(200)

	if stats.TotalCycles == 0 {
		t.Error("expected at least one cycle")
	}
}

// TestMissingOptional verifies that a core works without L2 cache and FP unit.
func TestMissingOptional(t *testing.T) {
	config := SimpleConfig()
	config.L2Cache = nil
	config.FPUnit = nil

	c, err := NewCore(config, NewMockDecoder())
	if err != nil {
		t.Fatalf("failed to create core without optionals: %v", err)
	}

	program := EncodeProgram(EncodeADDI(1, 0, 10), EncodeHALT())
	c.LoadProgram(program, 0)
	c.Run(100)

	if !c.IsHalted() {
		t.Error("core should have halted")
	}
}

// TestDefaultCoreConfig verifies the default config produces a working core.
func TestDefaultCoreConfig(t *testing.T) {
	c := makeDefaultCore(t)

	program := EncodeProgram(EncodeNOP(), EncodeHALT())
	c.LoadProgram(program, 0)
	c.Run(100)

	if !c.IsHalted() {
		t.Error("core should have halted")
	}
}

// =========================================================================
// Single-Instruction Tests
// =========================================================================

// TestNOP verifies that a NOP runs through the pipeline without side effects.
func TestNOP(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(EncodeNOP(), EncodeHALT())
	c.LoadProgram(program, 0)
	c.Run(100)

	if !c.IsHalted() {
		t.Error("core should have halted")
	}
	// NOP should not modify any register.
	for i := 0; i < c.regFile.Count(); i++ {
		if c.regFile.Read(i) != 0 {
			t.Errorf("register R%d should be 0 after NOP, got %d", i, c.regFile.Read(i))
		}
	}
}

// TestADD verifies that ADD produces the correct result in the destination register.
func TestADD(t *testing.T) {
	c := makeSimpleCore(t)

	// R1 = 10, R2 = 20, R3 = R1 + R2 = 30
	program := EncodeProgram(
		EncodeADDI(1, 0, 10),  // R1 = 0 + 10
		EncodeADDI(2, 0, 20),  // R2 = 0 + 20
		EncodeNOP(),           // avoid data hazard
		EncodeNOP(),           // avoid data hazard
		EncodeADD(3, 1, 2),    // R3 = R1 + R2
		EncodeNOP(),           // pipeline drain
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(200)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}

	r1 := c.ReadRegister(1)
	r2 := c.ReadRegister(2)
	r3 := c.ReadRegister(3)

	if r1 != 10 {
		t.Errorf("R1 expected 10, got %d", r1)
	}
	if r2 != 20 {
		t.Errorf("R2 expected 20, got %d", r2)
	}
	if r3 != 30 {
		t.Errorf("R3 expected 30 (10+20), got %d", r3)
	}
}

// TestADDI verifies immediate addition.
func TestADDI(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(
		EncodeADDI(1, 0, 42), // R1 = 0 + 42
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(100)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}
	if c.ReadRegister(1) != 42 {
		t.Errorf("R1 expected 42, got %d", c.ReadRegister(1))
	}
}

// TestSUB verifies subtraction.
func TestSUB(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(
		EncodeADDI(1, 0, 50),  // R1 = 50
		EncodeADDI(2, 0, 20),  // R2 = 20
		EncodeNOP(),
		EncodeNOP(),
		EncodeSUB(3, 1, 2),    // R3 = R1 - R2 = 30
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(200)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}
	if c.ReadRegister(3) != 30 {
		t.Errorf("R3 expected 30 (50-20), got %d", c.ReadRegister(3))
	}
}

// TestLOAD verifies that LOAD reads data from memory into a register.
func TestLOAD(t *testing.T) {
	c := makeSimpleCore(t)

	// Store 0xDEAD at address 512 in memory, then load it.
	// Note: we use address 512 because our 12-bit immediate is sign-extended,
	// so values >= 0x800 (2048) become negative. 512 = 0x200 is safe.
	c.memCtrl.WriteWord(512, 0xDEAD)

	program := EncodeProgram(
		EncodeADDI(1, 0, 0),   // R1 = 0 (base address)
		EncodeNOP(),
		EncodeNOP(),
		EncodeLOAD(2, 1, 512), // R2 = Memory[R1 + 512] = Memory[512]
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(200)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}
	if c.ReadRegister(2) != 0xDEAD {
		t.Errorf("R2 expected 0xDEAD, got 0x%X", c.ReadRegister(2))
	}
}

// TestSTORE verifies that STORE writes data from a register to memory.
func TestSTORE(t *testing.T) {
	c := makeSimpleCore(t)

	// Use address 512 (0x200) to stay within the positive 12-bit immediate range.
	// Values >= 0x800 (2048) are sign-extended to negative numbers.
	program := EncodeProgram(
		EncodeADDI(1, 0, 0),     // R1 = 0 (base address)
		EncodeADDI(2, 0, 0x42),  // R2 = 0x42 (value to store)
		EncodeNOP(),
		EncodeNOP(),
		EncodeSTORE(1, 2, 512),  // Memory[R1 + 512] = R2
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(200)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}
	val := c.memCtrl.ReadWord(512)
	if val != 0x42 {
		t.Errorf("Memory[512] expected 0x42, got 0x%X", val)
	}
}

// TestHALT verifies that HALT stops the pipeline.
func TestHALT(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(EncodeHALT())
	c.LoadProgram(program, 0)
	c.Run(100)

	if !c.IsHalted() {
		t.Error("core should have halted")
	}
}

// =========================================================================
// Program Execution Tests
// =========================================================================

// TestSimpleSequence verifies a LOAD, ADD, STORE sequence.
func TestSimpleSequence(t *testing.T) {
	c := makeSimpleCore(t)

	// Store initial value at address 512.
	c.memCtrl.WriteWord(512, 100)

	program := EncodeProgram(
		EncodeADDI(1, 0, 0),     // R1 = 0 (base)
		EncodeNOP(),
		EncodeNOP(),
		EncodeLOAD(2, 1, 512),   // R2 = Memory[512] = 100
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeADDI(3, 2, 50),    // R3 = R2 + 50 = 150
		EncodeNOP(),
		EncodeNOP(),
		EncodeSTORE(1, 3, 516),  // Memory[516] = R3
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(500)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}
	if c.ReadRegister(2) != 100 {
		t.Errorf("R2 expected 100, got %d", c.ReadRegister(2))
	}
	if c.ReadRegister(3) != 150 {
		t.Errorf("R3 expected 150, got %d", c.ReadRegister(3))
	}
	memVal := c.memCtrl.ReadWord(516)
	if memVal != 150 {
		t.Errorf("Memory[516] expected 150, got %d", memVal)
	}
}

// TestCountingProgram verifies a program that counts to a value.
func TestCountingProgram(t *testing.T) {
	c := makeSimpleCore(t)

	// R1 = counter (starts at 0)
	// R2 = step (1)
	// Loop body: R1 = R1 + R2
	// After 5 iterations via unrolling: R1 = 5
	program := EncodeProgram(
		EncodeADDI(1, 0, 0),   // R1 = 0
		EncodeADDI(2, 0, 1),   // R2 = 1
		EncodeNOP(),
		EncodeNOP(),
		EncodeADD(1, 1, 2),    // R1 = R1 + 1 = 1
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeADD(1, 1, 2),    // R1 = R1 + 1 = 2
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeADD(1, 1, 2),    // R1 = R1 + 1 = 3
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeADD(1, 1, 2),    // R1 = R1 + 1 = 4
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeADD(1, 1, 2),    // R1 = R1 + 1 = 5
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(500)

	if !c.IsHalted() {
		t.Fatal("core should have halted")
	}
	if c.ReadRegister(1) != 5 {
		t.Errorf("R1 expected 5, got %d", c.ReadRegister(1))
	}
}

// =========================================================================
// Statistics Tests
// =========================================================================

// TestIPCCalculation verifies IPC = instructions / cycles.
func TestIPCCalculation(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(
		EncodeADDI(1, 0, 1),
		EncodeADDI(2, 0, 2),
		EncodeADDI(3, 0, 3),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	stats := c.Run(200)

	if stats.InstructionsCompleted == 0 {
		t.Error("expected at least one completed instruction")
	}
	if stats.TotalCycles == 0 {
		t.Error("expected at least one cycle")
	}

	expectedIPC := float64(stats.InstructionsCompleted) / float64(stats.TotalCycles)
	actualIPC := stats.IPC()
	if actualIPC != expectedIPC {
		t.Errorf("IPC expected %.4f, got %.4f", expectedIPC, actualIPC)
	}

	// CPI should be the inverse.
	if stats.InstructionsCompleted > 0 {
		expectedCPI := float64(stats.TotalCycles) / float64(stats.InstructionsCompleted)
		if stats.CPI() != expectedCPI {
			t.Errorf("CPI expected %.4f, got %.4f", expectedCPI, stats.CPI())
		}
	}
}

// TestAggregateStats verifies that CoreStats aggregates sub-component stats.
func TestAggregateStats(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(
		EncodeADDI(1, 0, 10),
		EncodeADDI(2, 0, 20),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	stats := c.Run(200)

	// Pipeline stats should be populated.
	if stats.PipelineStats.TotalCycles == 0 {
		t.Error("pipeline stats should have cycles")
	}

	// Predictor stats should exist.
	if stats.PredictorStats == nil {
		t.Error("predictor stats should not be nil")
	}

	// Cache stats should have L1I and L1D.
	if _, ok := stats.CacheStats["L1I"]; !ok {
		t.Error("cache stats should contain L1I")
	}
	if _, ok := stats.CacheStats["L1D"]; !ok {
		t.Error("cache stats should contain L1D")
	}

	// L1I should have been accessed (instruction fetches).
	l1iStats := stats.CacheStats["L1I"]
	if l1iStats.TotalAccesses() == 0 {
		t.Error("L1I should have been accessed during instruction fetch")
	}
}

// TestStatsString verifies that stats can be formatted as a string.
func TestStatsString(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(EncodeADDI(1, 0, 1), EncodeHALT())
	c.LoadProgram(program, 0)
	stats := c.Run(100)

	s := stats.String()
	if s == "" {
		t.Error("stats string should not be empty")
	}
}

// =========================================================================
// ISA Decoder Injection Tests
// =========================================================================

// TestMockDecoderProtocol verifies the MockDecoder implements ISADecoder.
func TestMockDecoderProtocol(t *testing.T) {
	var _ ISADecoder = NewMockDecoder() // compile-time check

	d := NewMockDecoder()
	if d.InstructionSize() != 4 {
		t.Errorf("instruction size expected 4, got %d", d.InstructionSize())
	}
}

// TestMockDecoderDecode verifies decoding of each instruction type.
func TestMockDecoderDecode(t *testing.T) {
	d := NewMockDecoder()

	tests := []struct {
		name   string
		raw    int
		opcode string
		rd     int
		rs1    int
		rs2    int
	}{
		{"NOP", EncodeNOP(), "NOP", -1, -1, -1},
		{"ADD", EncodeADD(3, 1, 2), "ADD", 3, 1, 2},
		{"SUB", EncodeSUB(3, 1, 2), "SUB", 3, 1, 2},
		{"ADDI", EncodeADDI(1, 0, 42), "ADDI", 1, 0, -1},
		{"LOAD", EncodeLOAD(1, 2, 100), "LOAD", 1, 2, -1},
		{"STORE", EncodeSTORE(2, 3, 100), "STORE", -1, 2, 3},
		{"BRANCH", EncodeBRANCH(1, 2, 4), "BRANCH", -1, 1, 2},
		{"HALT", EncodeHALT(), "HALT", -1, -1, -1},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			token := cpupipeline.NewToken()
			result := d.Decode(tt.raw, token)
			if result.Opcode != tt.opcode {
				t.Errorf("opcode: expected %s, got %s", tt.opcode, result.Opcode)
			}
			if result.Rd != tt.rd {
				t.Errorf("Rd: expected %d, got %d", tt.rd, result.Rd)
			}
			if result.Rs1 != tt.rs1 {
				t.Errorf("Rs1: expected %d, got %d", tt.rs1, result.Rs1)
			}
			if result.Rs2 != tt.rs2 {
				t.Errorf("Rs2: expected %d, got %d", tt.rs2, result.Rs2)
			}
		})
	}
}

// TestMockDecoderExecute verifies execution of each instruction type.
func TestMockDecoderExecute(t *testing.T) {
	d := NewMockDecoder()
	regFile := NewRegisterFile(nil)

	// Set up register values.
	regFile.Write(1, 10)
	regFile.Write(2, 20)

	t.Run("ADD", func(t *testing.T) {
		token := cpupipeline.NewToken()
		d.Decode(EncodeADD(3, 1, 2), token)
		d.Execute(token, regFile)
		if token.ALUResult != 30 {
			t.Errorf("ALUResult expected 30, got %d", token.ALUResult)
		}
	})

	t.Run("SUB", func(t *testing.T) {
		token := cpupipeline.NewToken()
		d.Decode(EncodeSUB(3, 2, 1), token)
		d.Execute(token, regFile)
		if token.ALUResult != 10 {
			t.Errorf("ALUResult expected 10 (20-10), got %d", token.ALUResult)
		}
	})

	t.Run("ADDI", func(t *testing.T) {
		token := cpupipeline.NewToken()
		d.Decode(EncodeADDI(3, 1, 5), token)
		d.Execute(token, regFile)
		if token.ALUResult != 15 {
			t.Errorf("ALUResult expected 15 (10+5), got %d", token.ALUResult)
		}
	})

	t.Run("LOAD effective address", func(t *testing.T) {
		token := cpupipeline.NewToken()
		d.Decode(EncodeLOAD(3, 1, 100), token)
		d.Execute(token, regFile)
		if token.ALUResult != 110 {
			t.Errorf("ALUResult expected 110 (10+100), got %d", token.ALUResult)
		}
	})

	t.Run("BRANCH taken", func(t *testing.T) {
		token := cpupipeline.NewToken()
		token.PC = 100
		d.Decode(EncodeBRANCH(1, 1, 3), token) // Rs1==Rs1 always true
		d.Execute(token, regFile)
		if !token.BranchTaken {
			t.Error("branch should be taken (Rs1 == Rs1)")
		}
		if token.BranchTarget != 100+3*4 {
			t.Errorf("branch target expected %d, got %d", 100+3*4, token.BranchTarget)
		}
	})

	t.Run("BRANCH not taken", func(t *testing.T) {
		token := cpupipeline.NewToken()
		token.PC = 100
		d.Decode(EncodeBRANCH(1, 2, 3), token) // Rs1=10 != Rs2=20
		d.Execute(token, regFile)
		if token.BranchTaken {
			t.Error("branch should not be taken (10 != 20)")
		}
	})
}

// =========================================================================
// Register File Tests
// =========================================================================

// TestRegisterFileBasic verifies basic read/write operations.
func TestRegisterFileBasic(t *testing.T) {
	rf := NewRegisterFile(nil)

	rf.Write(1, 42)
	if rf.Read(1) != 42 {
		t.Errorf("R1 expected 42, got %d", rf.Read(1))
	}

	// Overwrite.
	rf.Write(1, 100)
	if rf.Read(1) != 100 {
		t.Errorf("R1 expected 100, got %d", rf.Read(1))
	}
}

// TestRegisterFileZeroRegister verifies that R0 is hardwired to zero.
func TestRegisterFileZeroRegister(t *testing.T) {
	cfg := RegisterFileConfig{Count: 16, Width: 32, ZeroRegister: true}
	rf := NewRegisterFile(&cfg)

	rf.Write(0, 999)
	if rf.Read(0) != 0 {
		t.Errorf("R0 should always read 0, got %d", rf.Read(0))
	}
}

// TestRegisterFileNoZeroRegister verifies R0 is writable when zero register is disabled.
func TestRegisterFileNoZeroRegister(t *testing.T) {
	cfg := RegisterFileConfig{Count: 16, Width: 32, ZeroRegister: false}
	rf := NewRegisterFile(&cfg)

	rf.Write(0, 999)
	if rf.Read(0) != 999 {
		t.Errorf("R0 expected 999 (no zero register), got %d", rf.Read(0))
	}
}

// TestRegisterFileOutOfRange verifies that out-of-range access is safe.
func TestRegisterFileOutOfRange(t *testing.T) {
	rf := NewRegisterFile(nil)

	// Read out of range should return 0.
	if rf.Read(100) != 0 {
		t.Errorf("out-of-range read should return 0, got %d", rf.Read(100))
	}
	if rf.Read(-1) != 0 {
		t.Errorf("negative index read should return 0, got %d", rf.Read(-1))
	}

	// Write out of range should not panic.
	rf.Write(100, 42)
	rf.Write(-1, 42)
}

// TestRegisterFileValues verifies the Values() method.
func TestRegisterFileValues(t *testing.T) {
	rf := NewRegisterFile(nil)
	rf.Write(1, 10)
	rf.Write(2, 20)

	vals := rf.Values()
	if len(vals) != rf.Count() {
		t.Errorf("values length expected %d, got %d", rf.Count(), len(vals))
	}
	if vals[1] != 10 {
		t.Errorf("vals[1] expected 10, got %d", vals[1])
	}
	if vals[2] != 20 {
		t.Errorf("vals[2] expected 20, got %d", vals[2])
	}
}

// TestRegisterFileReset verifies that Reset clears all registers.
func TestRegisterFileReset(t *testing.T) {
	rf := NewRegisterFile(nil)
	rf.Write(1, 42)
	rf.Write(5, 99)
	rf.Reset()

	if rf.Read(1) != 0 {
		t.Errorf("R1 should be 0 after reset, got %d", rf.Read(1))
	}
	if rf.Read(5) != 0 {
		t.Errorf("R5 should be 0 after reset, got %d", rf.Read(5))
	}
}

// TestRegisterFileString verifies the String() method.
func TestRegisterFileString(t *testing.T) {
	rf := NewRegisterFile(nil)
	rf.Write(1, 42)
	s := rf.String()
	if s == "" {
		t.Error("string should not be empty")
	}
}

// TestRegisterFileBitWidth verifies that values are masked to bit width.
func TestRegisterFileBitWidth(t *testing.T) {
	cfg := RegisterFileConfig{Count: 4, Width: 8, ZeroRegister: false}
	rf := NewRegisterFile(&cfg)

	rf.Write(1, 0xABCD) // only low 8 bits should be stored
	val := rf.Read(1)
	if val != 0xCD {
		t.Errorf("8-bit register expected 0xCD, got 0x%X", val)
	}
}

// =========================================================================
// Memory Controller Tests
// =========================================================================

// TestMemoryControllerReadWrite verifies immediate read/write operations.
func TestMemoryControllerReadWrite(t *testing.T) {
	mem := make([]byte, 4096)
	mc := NewMemoryController(mem, 10)

	mc.WriteWord(100, 0x1234ABCD)
	val := mc.ReadWord(100)
	if val != 0x1234ABCD {
		t.Errorf("expected 0x1234ABCD, got 0x%X", val)
	}
}

// TestMemoryControllerLoadProgram verifies LoadProgram writes bytes correctly.
func TestMemoryControllerLoadProgram(t *testing.T) {
	mem := make([]byte, 4096)
	mc := NewMemoryController(mem, 10)

	program := []byte{0x01, 0x02, 0x03, 0x04}
	mc.LoadProgram(program, 0)

	// Read back.
	word := mc.ReadWord(0)
	expected := 0x04030201 // little-endian
	if word != expected {
		t.Errorf("expected 0x%X, got 0x%X", expected, word)
	}
}

// TestMemoryControllerPendingRequests verifies async request processing.
func TestMemoryControllerPendingRequests(t *testing.T) {
	mem := make([]byte, 4096)
	mc := NewMemoryController(mem, 3) // 3-cycle latency

	// Write data directly first.
	mc.WriteWord(0, 42)

	// Submit an async read request.
	mc.RequestRead(0, 4, 0)

	if mc.PendingCount() != 1 {
		t.Errorf("pending count expected 1, got %d", mc.PendingCount())
	}

	// Tick 1 and 2: not ready yet.
	result1 := mc.Tick()
	if len(result1) != 0 {
		t.Error("read should not be ready after 1 tick")
	}
	result2 := mc.Tick()
	if len(result2) != 0 {
		t.Error("read should not be ready after 2 ticks")
	}

	// Tick 3: ready.
	result3 := mc.Tick()
	if len(result3) != 1 {
		t.Fatalf("expected 1 completed read after 3 ticks, got %d", len(result3))
	}
	if result3[0].RequesterID != 0 {
		t.Errorf("requester ID expected 0, got %d", result3[0].RequesterID)
	}
}

// TestMemoryControllerBoundsCheck verifies out-of-bounds access is safe.
func TestMemoryControllerBoundsCheck(t *testing.T) {
	mem := make([]byte, 64)
	mc := NewMemoryController(mem, 1)

	// These should not panic.
	mc.ReadWord(1000)
	mc.WriteWord(1000, 42)
	mc.LoadProgram([]byte{1, 2, 3, 4}, 1000)
}

// =========================================================================
// Interrupt Controller Tests
// =========================================================================

// TestInterruptControllerBasic verifies basic interrupt routing.
func TestInterruptControllerBasic(t *testing.T) {
	ic := NewInterruptController(4)

	ic.RaiseInterrupt(1, 2) // interrupt 1 to core 2
	if ic.PendingCount() != 1 {
		t.Errorf("pending count expected 1, got %d", ic.PendingCount())
	}

	pending := ic.PendingForCore(2)
	if len(pending) != 1 {
		t.Fatalf("expected 1 pending interrupt for core 2, got %d", len(pending))
	}
	if pending[0].InterruptID != 1 {
		t.Errorf("interrupt ID expected 1, got %d", pending[0].InterruptID)
	}

	// Acknowledge.
	ic.Acknowledge(2, 1)
	if ic.PendingCount() != 0 {
		t.Errorf("pending count expected 0 after acknowledge, got %d", ic.PendingCount())
	}
	if ic.AcknowledgedCount() != 1 {
		t.Errorf("acknowledged count expected 1, got %d", ic.AcknowledgedCount())
	}
}

// TestInterruptControllerDefaultRouting verifies that -1 routes to core 0.
func TestInterruptControllerDefaultRouting(t *testing.T) {
	ic := NewInterruptController(4)

	ic.RaiseInterrupt(5, -1) // should route to core 0
	pending := ic.PendingForCore(0)
	if len(pending) != 1 {
		t.Errorf("expected 1 pending for core 0, got %d", len(pending))
	}
}

// TestInterruptControllerReset verifies Reset clears all state.
func TestInterruptControllerReset(t *testing.T) {
	ic := NewInterruptController(4)
	ic.RaiseInterrupt(1, 0)
	ic.Acknowledge(0, 1)
	ic.Reset()

	if ic.PendingCount() != 0 {
		t.Errorf("pending should be 0 after reset, got %d", ic.PendingCount())
	}
	if ic.AcknowledgedCount() != 0 {
		t.Errorf("acknowledged should be 0 after reset, got %d", ic.AcknowledgedCount())
	}
}

// =========================================================================
// Configuration Tests
// =========================================================================

// TestSimpleConfigFields verifies all fields of SimpleConfig.
func TestSimpleConfigFields(t *testing.T) {
	cfg := SimpleConfig()

	if cfg.Name != "Simple" {
		t.Errorf("name expected 'Simple', got '%s'", cfg.Name)
	}
	if len(cfg.Pipeline.Stages) != 5 {
		t.Errorf("pipeline stages expected 5, got %d", len(cfg.Pipeline.Stages))
	}
	if cfg.BranchPredictorType != "static_always_not_taken" {
		t.Errorf("predictor type expected 'static_always_not_taken', got '%s'", cfg.BranchPredictorType)
	}
	if cfg.RegisterFile == nil || cfg.RegisterFile.Count != 16 {
		t.Error("register file should have 16 registers")
	}
	if cfg.FPUnit != nil {
		t.Error("simple config should not have FP unit")
	}
	if cfg.L1ICache == nil || cfg.L1ICache.TotalSize != 4096 {
		t.Error("L1I cache should be 4KB")
	}
	if cfg.L1DCache == nil || cfg.L1DCache.TotalSize != 4096 {
		t.Error("L1D cache should be 4KB")
	}
	if cfg.L2Cache != nil {
		t.Error("simple config should not have L2 cache")
	}
}

// TestCortexA78LikeConfigFields verifies CortexA78-like config.
func TestCortexA78LikeConfigFields(t *testing.T) {
	cfg := CortexA78LikeConfig()

	if cfg.Name != "CortexA78Like" {
		t.Errorf("name expected 'CortexA78Like', got '%s'", cfg.Name)
	}
	if len(cfg.Pipeline.Stages) != 13 {
		t.Errorf("pipeline stages expected 13, got %d", len(cfg.Pipeline.Stages))
	}
	if cfg.BranchPredictorType != "two_bit" {
		t.Errorf("predictor type expected 'two_bit', got '%s'", cfg.BranchPredictorType)
	}
	if cfg.BranchPredictorSize != 4096 {
		t.Errorf("predictor size expected 4096, got %d", cfg.BranchPredictorSize)
	}
	if cfg.RegisterFile == nil || cfg.RegisterFile.Count != 31 {
		t.Error("register file should have 31 registers")
	}
	if cfg.FPUnit == nil {
		t.Error("Cortex-A78-like config should have FP unit")
	}
	if cfg.L1ICache == nil || cfg.L1ICache.TotalSize != 65536 {
		t.Error("L1I cache should be 64KB")
	}
	if cfg.L2Cache == nil || cfg.L2Cache.TotalSize != 262144 {
		t.Error("L2 cache should be 256KB")
	}
}

// TestDefaultCoreConfigFields verifies defaults.
func TestDefaultCoreConfigFields(t *testing.T) {
	cfg := DefaultCoreConfig()

	if cfg.Name != "Default" {
		t.Errorf("name expected 'Default', got '%s'", cfg.Name)
	}
	if cfg.HazardDetection != true {
		t.Error("hazard detection should be enabled")
	}
	if cfg.Forwarding != true {
		t.Error("forwarding should be enabled")
	}
}

// TestDefaultMultiCoreConfig verifies multi-core defaults.
func TestDefaultMultiCoreConfig(t *testing.T) {
	cfg := DefaultMultiCoreConfig()

	if cfg.NumCores != 2 {
		t.Errorf("num cores expected 2, got %d", cfg.NumCores)
	}
	if cfg.MemorySize != 1048576 {
		t.Errorf("memory size expected 1MB, got %d", cfg.MemorySize)
	}
}

// =========================================================================
// Branch Predictor Factory Tests
// =========================================================================

// TestCreateBranchPredictor verifies all predictor types can be created.
func TestCreateBranchPredictor(t *testing.T) {
	types := []string{
		"static_always_taken",
		"static_always_not_taken",
		"static_btfnt",
		"one_bit",
		"two_bit",
		"unknown_type",
	}

	for _, typ := range types {
		t.Run(typ, func(t *testing.T) {
			p := createBranchPredictor(typ, 256)
			if p == nil {
				t.Errorf("createBranchPredictor(%q) returned nil", typ)
			}
			// Should implement the interface.
			var _ branchpredictor.BranchPredictor = p
		})
	}
}

// =========================================================================
// Multi-Core Tests
// =========================================================================

// TestMultiCoreConstruction verifies multi-core initialization.
func TestMultiCoreConstruction(t *testing.T) {
	config := DefaultMultiCoreConfig()
	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}

	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatalf("failed to create multi-core CPU: %v", err)
	}

	if len(mc.Cores()) != 2 {
		t.Errorf("expected 2 cores, got %d", len(mc.Cores()))
	}
	if mc.InterruptController() == nil {
		t.Error("interrupt controller should not be nil")
	}
	if mc.SharedMemoryController() == nil {
		t.Error("memory controller should not be nil")
	}
}

// TestMultiCoreIndependentPrograms verifies two cores run separate programs
// and both produce correct results.
func TestMultiCoreIndependentPrograms(t *testing.T) {
	config := DefaultMultiCoreConfig()
	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}

	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatalf("failed to create multi-core CPU: %v", err)
	}

	// Core 0: R1 = 10
	prog0 := EncodeProgram(
		EncodeADDI(1, 0, 10),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	// Core 1: R1 = 20, loaded at a different address
	prog1 := EncodeProgram(
		EncodeADDI(1, 0, 20),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)

	mc.LoadProgram(0, prog0, 0)
	mc.LoadProgram(1, prog1, 4096)

	mc.Run(200)

	if !mc.AllHalted() {
		t.Error("all cores should have halted")
	}

	// Core 0 should have R1=10.
	r1core0 := mc.Cores()[0].ReadRegister(1)
	if r1core0 != 10 {
		t.Errorf("Core 0 R1 expected 10, got %d", r1core0)
	}

	// Core 1 should have R1=20.
	r1core1 := mc.Cores()[1].ReadRegister(1)
	if r1core1 != 20 {
		t.Errorf("Core 1 R1 expected 20, got %d", r1core1)
	}
}

// TestMultiCoreSharedMemory verifies that one core's memory writes are
// visible to another core (through the shared memory controller).
func TestMultiCoreSharedMemory(t *testing.T) {
	config := DefaultMultiCoreConfig()
	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}

	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatalf("failed to create multi-core CPU: %v", err)
	}

	// Write a value to shared memory from the memory controller.
	// Use address 512 (fits in 12-bit unsigned immediate without sign-extension).
	mc.SharedMemoryController().WriteWord(512, 0xCAFE)

	// Core 0 loads from that address.
	prog0 := EncodeProgram(
		EncodeADDI(1, 0, 0),    // R1 = 0
		EncodeNOP(),
		EncodeNOP(),
		EncodeLOAD(2, 1, 512),  // R2 = Memory[512]
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeNOP(),
		EncodeHALT(),
	)
	mc.LoadProgram(0, prog0, 0)

	// Core 1 just halts.
	prog1 := EncodeProgram(EncodeHALT())
	mc.LoadProgram(1, prog1, 4096)

	mc.Run(200)

	r2 := mc.Cores()[0].ReadRegister(2)
	if r2 != 0xCAFE {
		t.Errorf("Core 0 R2 expected 0xCAFE, got 0x%X", r2)
	}
}

// TestMultiCoreStats verifies per-core statistics are collected.
func TestMultiCoreStats(t *testing.T) {
	config := DefaultMultiCoreConfig()
	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}

	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatalf("failed to create multi-core CPU: %v", err)
	}

	prog := EncodeProgram(EncodeADDI(1, 0, 1), EncodeHALT())
	mc.LoadProgram(0, prog, 0)
	mc.LoadProgram(1, prog, 4096)

	stats := mc.Run(200)

	if len(stats) != 2 {
		t.Fatalf("expected 2 stat entries, got %d", len(stats))
	}
	for i, s := range stats {
		if s.TotalCycles == 0 {
			t.Errorf("core %d should have cycles", i)
		}
	}
}

// TestMultiCoreStep verifies Step returns per-core snapshots.
func TestMultiCoreStep(t *testing.T) {
	config := DefaultMultiCoreConfig()
	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}

	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatalf("failed to create multi-core CPU: %v", err)
	}

	prog := EncodeProgram(EncodeHALT())
	mc.LoadProgram(0, prog, 0)
	mc.LoadProgram(1, prog, 4096)

	snapshots := mc.Step()
	if len(snapshots) != 2 {
		t.Errorf("expected 2 snapshots, got %d", len(snapshots))
	}
}

// TestMultiCoreCoreCountScaling verifies 1, 2, and 4 cores all work.
func TestMultiCoreCoreCountScaling(t *testing.T) {
	for _, numCores := range []int{1, 2, 4} {
		t.Run("cores="+string(rune('0'+numCores)), func(t *testing.T) {
			config := DefaultMultiCoreConfig()
			config.NumCores = numCores

			decoders := make([]ISADecoder, numCores)
			for i := range decoders {
				decoders[i] = NewMockDecoder()
			}

			mc, err := NewMultiCoreCPU(config, decoders)
			if err != nil {
				t.Fatalf("failed to create %d-core CPU: %v", numCores, err)
			}

			prog := EncodeProgram(EncodeHALT())
			for i := 0; i < numCores; i++ {
				mc.LoadProgram(i, prog, i*4096)
			}

			mc.Run(200)
			if !mc.AllHalted() {
				t.Errorf("%d-core CPU should have halted", numCores)
			}
		})
	}
}

// =========================================================================
// Performance Comparison Tests
// =========================================================================

// TestPredictorImpact verifies that different predictors give different stats.
func TestPredictorImpact(t *testing.T) {
	// Create a program with NOPs and HALT -- no branches, so predictor
	// differences should show up in stats even if not in IPC.
	prog := EncodeProgram(
		EncodeADDI(1, 0, 1),
		EncodeADDI(2, 0, 2),
		EncodeADDI(3, 0, 3),
		EncodeADDI(4, 0, 4),
		EncodeHALT(),
	)

	configs := []struct {
		name string
		typ  string
	}{
		{"static_always_taken", "static_always_taken"},
		{"static_always_not_taken", "static_always_not_taken"},
		{"two_bit", "two_bit"},
	}

	for _, cfg := range configs {
		t.Run(cfg.name, func(t *testing.T) {
			config := SimpleConfig()
			config.BranchPredictorType = cfg.typ
			config.BranchPredictorSize = 256

			c, err := NewCore(config, NewMockDecoder())
			if err != nil {
				t.Fatalf("failed to create core with %s predictor: %v", cfg.name, err)
			}

			c.LoadProgram(prog, 0)
			stats := c.Run(200)

			if stats.TotalCycles == 0 {
				t.Error("should have run at least one cycle")
			}
			// All should complete successfully.
			if stats.InstructionsCompleted == 0 {
				t.Error("should have completed at least one instruction")
			}
		})
	}
}

// TestCacheImpact verifies that different cache sizes affect stats.
func TestCacheImpact(t *testing.T) {
	prog := EncodeProgram(
		EncodeADDI(1, 0, 10),
		EncodeADDI(2, 0, 20),
		EncodeHALT(),
	)

	// Small cache: 4KB.
	t.Run("small_cache", func(t *testing.T) {
		config := SimpleConfig()
		c, err := NewCore(config, NewMockDecoder())
		if err != nil {
			t.Fatal(err)
		}
		c.LoadProgram(prog, 0)
		stats := c.Run(200)

		l1i := stats.CacheStats["L1I"]
		if l1i == nil {
			t.Fatal("L1I stats should exist")
		}
		if l1i.TotalAccesses() == 0 {
			t.Error("L1I should have been accessed")
		}
	})

	// Large cache: 64KB.
	t.Run("large_cache", func(t *testing.T) {
		config := SimpleConfig()
		l1iCfg := cache.CacheConfig{
			Name: "L1I", TotalSize: 65536, LineSize: 64,
			Associativity: 4, AccessLatency: 1, WritePolicy: "write-back",
		}
		config.L1ICache = &l1iCfg

		c, err := NewCore(config, NewMockDecoder())
		if err != nil {
			t.Fatal(err)
		}
		c.LoadProgram(prog, 0)
		stats := c.Run(200)

		l1i := stats.CacheStats["L1I"]
		if l1i == nil {
			t.Fatal("L1I stats should exist")
		}
		if l1i.TotalAccesses() == 0 {
			t.Error("L1I should have been accessed")
		}
	})
}

// =========================================================================
// Core Access Methods Tests
// =========================================================================

// TestCoreAccessors verifies all the accessor methods on Core.
func TestCoreAccessors(t *testing.T) {
	c := makeSimpleCore(t)

	if c.Config().Name != "Simple" {
		t.Errorf("config name expected 'Simple', got '%s'", c.Config().Name)
	}
	if c.Pipeline() == nil {
		t.Error("Pipeline() should not return nil")
	}
	if c.Predictor() == nil {
		t.Error("Predictor() should not return nil")
	}
	if c.CacheHierarchy() == nil {
		t.Error("CacheHierarchy() should not return nil")
	}
	if c.RegisterFile() == nil {
		t.Error("RegisterFile() should not return nil")
	}
	if c.MemoryController() == nil {
		t.Error("MemoryController() should not return nil")
	}
	if c.Cycle() != 0 {
		t.Errorf("initial cycle expected 0, got %d", c.Cycle())
	}
	if c.IsHalted() {
		t.Error("core should not be halted initially")
	}
}

// TestCoreReadWriteRegister verifies ReadRegister and WriteRegister.
func TestCoreReadWriteRegister(t *testing.T) {
	c := makeSimpleCore(t)

	c.WriteRegister(5, 123)
	if c.ReadRegister(5) != 123 {
		t.Errorf("R5 expected 123, got %d", c.ReadRegister(5))
	}
}

// TestCoreStepByStep verifies cycle-by-cycle execution.
func TestCoreStepByStep(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(EncodeHALT())
	c.LoadProgram(program, 0)

	// Step until halted.
	for i := 0; i < 20; i++ {
		c.Step()
		if c.IsHalted() {
			break
		}
	}

	if !c.IsHalted() {
		t.Error("core should have halted within 20 steps")
	}
	if c.Cycle() == 0 {
		t.Error("cycle count should be > 0")
	}
}

// TestCoreStepAfterHalt verifies that Step() is a no-op after halt.
func TestCoreStepAfterHalt(t *testing.T) {
	c := makeSimpleCore(t)

	program := EncodeProgram(EncodeHALT())
	c.LoadProgram(program, 0)
	c.Run(100)

	cycleBefore := c.Cycle()
	c.Step()
	if c.Cycle() != cycleBefore {
		t.Error("cycle should not advance after halt")
	}
}

// =========================================================================
// EncodeProgram Tests
// =========================================================================

// TestEncodeProgram verifies that EncodeProgram produces correct bytes.
func TestEncodeProgram(t *testing.T) {
	prog := EncodeProgram(0x01020304, 0x05060708)

	if len(prog) != 8 {
		t.Fatalf("expected 8 bytes, got %d", len(prog))
	}

	// First instruction: 0x01020304 in little-endian = 04 03 02 01.
	if prog[0] != 0x04 || prog[1] != 0x03 || prog[2] != 0x02 || prog[3] != 0x01 {
		t.Errorf("first instruction encoding wrong: %02X %02X %02X %02X",
			prog[0], prog[1], prog[2], prog[3])
	}
}

// =========================================================================
// Mock Instruction Encoding Tests
// =========================================================================

// TestMockInstructionEncodings verifies all instruction encoding helpers.
func TestMockInstructionEncodings(t *testing.T) {
	d := NewMockDecoder()

	t.Run("EncodeNOP", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeNOP(), tok)
		if tok.Opcode != "NOP" {
			t.Errorf("expected NOP, got %s", tok.Opcode)
		}
	})

	t.Run("EncodeADD", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeADD(3, 1, 2), tok)
		if tok.Opcode != "ADD" || tok.Rd != 3 || tok.Rs1 != 1 || tok.Rs2 != 2 {
			t.Errorf("ADD encoding wrong: %v", tok)
		}
	})

	t.Run("EncodeSUB", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeSUB(5, 3, 4), tok)
		if tok.Opcode != "SUB" || tok.Rd != 5 || tok.Rs1 != 3 || tok.Rs2 != 4 {
			t.Errorf("SUB encoding wrong: %v", tok)
		}
	})

	t.Run("EncodeADDI", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeADDI(2, 1, 100), tok)
		if tok.Opcode != "ADDI" || tok.Rd != 2 || tok.Rs1 != 1 || tok.Immediate != 100 {
			t.Errorf("ADDI encoding wrong: opcode=%s Rd=%d Rs1=%d imm=%d",
				tok.Opcode, tok.Rd, tok.Rs1, tok.Immediate)
		}
	})

	t.Run("EncodeLOAD", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeLOAD(1, 2, 200), tok)
		if tok.Opcode != "LOAD" || tok.Rd != 1 || tok.Rs1 != 2 || !tok.MemRead {
			t.Errorf("LOAD encoding wrong")
		}
	})

	t.Run("EncodeSTORE", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeSTORE(2, 3, 300), tok)
		if tok.Opcode != "STORE" || tok.Rs1 != 2 || tok.Rs2 != 3 || !tok.MemWrite {
			t.Errorf("STORE encoding wrong")
		}
	})

	t.Run("EncodeBRANCH", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeBRANCH(1, 2, 5), tok)
		if tok.Opcode != "BRANCH" || tok.Rs1 != 1 || tok.Rs2 != 2 || !tok.IsBranch {
			t.Errorf("BRANCH encoding wrong")
		}
	})

	t.Run("EncodeHALT", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		d.Decode(EncodeHALT(), tok)
		if tok.Opcode != "HALT" || !tok.IsHalt {
			t.Errorf("HALT encoding wrong")
		}
	})
}

// TestNegativeImmediate verifies sign extension of negative immediates.
func TestNegativeImmediate(t *testing.T) {
	d := NewMockDecoder()

	// Encode ADDI with negative immediate (-1 in 12-bit = 0xFFF).
	raw := EncodeADDI(1, 0, 0xFFF) // -1 in 12-bit two's complement
	tok := cpupipeline.NewToken()
	d.Decode(raw, tok)

	if tok.Immediate >= 0 {
		t.Errorf("negative immediate expected, got %d", tok.Immediate)
	}
}

// TestUnknownOpcode verifies unknown opcodes decode as NOP.
func TestUnknownOpcode(t *testing.T) {
	d := NewMockDecoder()

	raw := 0xFF << 24 // opcode 0xFF is unknown
	tok := cpupipeline.NewToken()
	d.Decode(raw, tok)

	if tok.Opcode != "NOP" {
		t.Errorf("unknown opcode should decode as NOP, got %s", tok.Opcode)
	}
}

// =========================================================================
// Hazard Callback Tests
// =========================================================================

// TestHazardDetectionDisabled verifies core works without hazard detection.
func TestHazardDetectionDisabled(t *testing.T) {
	config := SimpleConfig()
	config.HazardDetection = false

	c, err := NewCore(config, NewMockDecoder())
	if err != nil {
		t.Fatal(err)
	}

	program := EncodeProgram(
		EncodeADDI(1, 0, 10),
		EncodeHALT(),
	)
	c.LoadProgram(program, 0)
	c.Run(100)

	if !c.IsHalted() {
		t.Error("core should have halted")
	}
}

// =========================================================================
// CoreStats Edge Cases
// =========================================================================

// TestCoreStatsZeroCycles verifies IPC/CPI with zero data.
func TestCoreStatsZeroCycles(t *testing.T) {
	stats := CoreStats{}

	if stats.IPC() != 0.0 {
		t.Errorf("IPC with zero cycles should be 0.0, got %f", stats.IPC())
	}
	if stats.CPI() != 0.0 {
		t.Errorf("CPI with zero instructions should be 0.0, got %f", stats.CPI())
	}
}

// TestCoreWithL2Cache verifies core works with L2 cache enabled.
func TestCoreWithL2Cache(t *testing.T) {
	config := SimpleConfig()
	l2 := cache.CacheConfig{
		Name: "L2", TotalSize: 16384, LineSize: 64,
		Associativity: 4, AccessLatency: 10, WritePolicy: "write-back",
	}
	config.L2Cache = &l2

	c, err := NewCore(config, NewMockDecoder())
	if err != nil {
		t.Fatal(err)
	}

	program := EncodeProgram(EncodeADDI(1, 0, 5), EncodeHALT())
	c.LoadProgram(program, 0)
	stats := c.Run(200)

	// L2 stats should be present.
	if _, ok := stats.CacheStats["L2"]; !ok {
		t.Error("L2 cache stats should be present")
	}
}

// =========================================================================
// RegisterFileConfig Defaults Test
// =========================================================================

// TestDefaultRegisterFileConfig verifies default config values.
func TestDefaultRegisterFileConfig(t *testing.T) {
	cfg := DefaultRegisterFileConfig()

	if cfg.Count != 16 {
		t.Errorf("default count expected 16, got %d", cfg.Count)
	}
	if cfg.Width != 32 {
		t.Errorf("default width expected 32, got %d", cfg.Width)
	}
	if cfg.ZeroRegister != true {
		t.Error("default zero register should be true")
	}
}

// =========================================================================
// Token-to-Slot Conversion Test
// =========================================================================

// TestTokenToSlot verifies the token-to-slot conversion function.
func TestTokenToSlot(t *testing.T) {
	t.Run("nil token", func(t *testing.T) {
		slot := tokenToSlot(nil)
		if slot.Valid {
			t.Error("nil token should produce invalid slot")
		}
	})

	t.Run("bubble token", func(t *testing.T) {
		bubble := cpupipeline.NewBubble()
		slot := tokenToSlot(bubble)
		if slot.Valid {
			t.Error("bubble should produce invalid slot")
		}
	})

	t.Run("ADD token", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		tok.Opcode = "ADD"
		tok.PC = 100
		tok.Rs1 = 1
		tok.Rs2 = 2
		tok.Rd = 3
		tok.RegWrite = true
		tok.ALUResult = 42

		slot := tokenToSlot(tok)
		if !slot.Valid {
			t.Error("ADD token should produce valid slot")
		}
		if slot.PC != 100 {
			t.Errorf("PC expected 100, got %d", slot.PC)
		}
		if len(slot.SourceRegs) != 2 {
			t.Errorf("expected 2 source regs, got %d", len(slot.SourceRegs))
		}
		if slot.DestReg == nil || *slot.DestReg != 3 {
			t.Error("dest reg should be 3")
		}
	})

	t.Run("BRANCH token", func(t *testing.T) {
		tok := cpupipeline.NewToken()
		tok.Opcode = "BRANCH"
		tok.PC = 200
		tok.IsBranch = true
		tok.Rs1 = 1
		tok.Rs2 = 2

		slot := tokenToSlot(tok)
		if !slot.IsBranch {
			t.Error("branch token should have IsBranch=true")
		}
	})
}

// =========================================================================
// 64-bit Register Width Test
// =========================================================================

// TestRegisterFile64Bit verifies 64-bit register width.
func TestRegisterFile64Bit(t *testing.T) {
	cfg := RegisterFileConfig{Count: 32, Width: 64, ZeroRegister: true}
	rf := NewRegisterFile(&cfg)

	if rf.Width() != 64 {
		t.Errorf("width expected 64, got %d", rf.Width())
	}
	if rf.Count() != 32 {
		t.Errorf("count expected 32, got %d", rf.Count())
	}

	// Should handle large values.
	rf.Write(1, 0x7FFFFFFF+1) // 2^31
	val := rf.Read(1)
	if val != 0x80000000 {
		t.Errorf("expected 0x80000000, got 0x%X", val)
	}
}

// =========================================================================
// Memory Controller Write Request Test
// =========================================================================

// TestMemoryControllerWriteRequest verifies async write processing.
func TestMemoryControllerWriteRequest(t *testing.T) {
	mem := make([]byte, 4096)
	mc := NewMemoryController(mem, 2) // 2-cycle latency

	mc.RequestWrite(100, []byte{0xAA, 0xBB, 0xCC, 0xDD}, 0)

	// Tick 1: not committed yet.
	mc.Tick()
	word := mc.ReadWord(100)
	if word != 0 {
		t.Errorf("write should not be committed after 1 tick, got 0x%X", word)
	}

	// Tick 2: committed.
	mc.Tick()
	word = mc.ReadWord(100)
	if word&0xFF != 0xAA {
		t.Errorf("write should be committed after 2 ticks, got 0x%X", word)
	}
}

// =========================================================================
// Multi-Core with L3 Cache Test
// =========================================================================

// TestMultiCoreWithL3Cache verifies multi-core with shared L3.
func TestMultiCoreWithL3Cache(t *testing.T) {
	config := DefaultMultiCoreConfig()
	l3 := cache.CacheConfig{
		Name: "L3", TotalSize: 65536, LineSize: 64,
		Associativity: 8, AccessLatency: 30, WritePolicy: "write-back",
	}
	config.L3Cache = &l3

	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}
	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatalf("failed to create multi-core with L3: %v", err)
	}

	prog := EncodeProgram(EncodeHALT())
	mc.LoadProgram(0, prog, 0)
	mc.LoadProgram(1, prog, 4096)

	mc.Run(200)
	if !mc.AllHalted() {
		t.Error("all cores should have halted")
	}
}

// =========================================================================
// Multi-Core Cycle Counter Test
// =========================================================================

// TestMultiCoreCycle verifies the global cycle counter.
func TestMultiCoreCycle(t *testing.T) {
	config := DefaultMultiCoreConfig()
	decoders := []ISADecoder{NewMockDecoder(), NewMockDecoder()}

	mc, err := NewMultiCoreCPU(config, decoders)
	if err != nil {
		t.Fatal(err)
	}

	if mc.Cycle() != 0 {
		t.Errorf("initial cycle expected 0, got %d", mc.Cycle())
	}

	prog := EncodeProgram(EncodeHALT())
	mc.LoadProgram(0, prog, 0)
	mc.LoadProgram(1, prog, 4096)

	mc.Step()
	if mc.Cycle() != 1 {
		t.Errorf("after one step, cycle expected 1, got %d", mc.Cycle())
	}
}

// =========================================================================
// Interrupt Controller Overflow Test
// =========================================================================

// TestInterruptControllerOverflow verifies routing to non-existent core.
func TestInterruptControllerOverflow(t *testing.T) {
	ic := NewInterruptController(2) // only 2 cores

	ic.RaiseInterrupt(1, 99) // core 99 does not exist -> defaults to core 0
	pending := ic.PendingForCore(0)
	if len(pending) != 1 {
		t.Errorf("interrupt should route to core 0, got %d pending", len(pending))
	}
}
