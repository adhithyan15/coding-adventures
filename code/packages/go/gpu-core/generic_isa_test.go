package gpucore

import (
	"testing"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// helper creates a standard test rig: a GenericISA, register file, and memory.
func newTestRig() (GenericISA, *FPRegisterFile, *LocalMemory) {
	isa := GenericISA{}
	regs, _ := NewFPRegisterFile(32, fp.FP32)
	mem, _ := NewLocalMemory(4096)
	return isa, regs, mem
}

// =========================================================================
// ISA metadata
// =========================================================================

func TestGenericISAName(t *testing.T) {
	isa := GenericISA{}
	if isa.Name() != "Generic" {
		t.Errorf("expected name 'Generic', got %q", isa.Name())
	}
}

// =========================================================================
// Arithmetic instruction tests
// =========================================================================

// TestExecFadd verifies floating-point addition.
func TestExecFadd(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 3.0)
	_ = regs.WriteFloat(1, 4.0)

	result := isa.Execute(Fadd(2, 0, 1), regs, mem)

	val, _ := regs.ReadFloat(2)
	if val != 7.0 {
		t.Errorf("FADD: expected R2=7.0, got %g", val)
	}
	if result.NextPCOffset != 1 {
		t.Errorf("FADD: expected NextPCOffset=1, got %d", result.NextPCOffset)
	}
	if result.RegistersChanged["R2"] != 7.0 {
		t.Errorf("FADD: expected RegistersChanged[R2]=7.0, got %g", result.RegistersChanged["R2"])
	}
}

// TestExecFsub verifies floating-point subtraction.
func TestExecFsub(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 10.0)
	_ = regs.WriteFloat(1, 3.0)

	result := isa.Execute(Fsub(2, 0, 1), regs, mem)

	val, _ := regs.ReadFloat(2)
	if val != 7.0 {
		t.Errorf("FSUB: expected R2=7.0, got %g", val)
	}
	if result.Description == "" {
		t.Error("FSUB: expected non-empty description")
	}
}

// TestExecFmul verifies floating-point multiplication.
func TestExecFmul(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 3.0)
	_ = regs.WriteFloat(1, 4.0)

	isa.Execute(Fmul(2, 0, 1), regs, mem)

	val, _ := regs.ReadFloat(2)
	if val != 12.0 {
		t.Errorf("FMUL: expected R2=12.0, got %g", val)
	}
}

// TestExecFfma verifies fused multiply-add: Rd = Rs1 * Rs2 + Rs3.
func TestExecFfma(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 2.0)
	_ = regs.WriteFloat(1, 3.0)
	_ = regs.WriteFloat(2, 1.0)

	isa.Execute(Ffma(3, 0, 1, 2), regs, mem)

	val, _ := regs.ReadFloat(3)
	if val != 7.0 {
		t.Errorf("FFMA: expected R3=7.0 (2*3+1), got %g", val)
	}
}

// TestExecFneg verifies negation.
func TestExecFneg(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 5.0)

	isa.Execute(Fneg(1, 0), regs, mem)

	val, _ := regs.ReadFloat(1)
	if val != -5.0 {
		t.Errorf("FNEG: expected R1=-5.0, got %g", val)
	}
}

// TestExecFabs verifies absolute value.
func TestExecFabs(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, -5.0)

	isa.Execute(Fabs(1, 0), regs, mem)

	val, _ := regs.ReadFloat(1)
	if val != 5.0 {
		t.Errorf("FABS: expected R1=5.0, got %g", val)
	}
}

// =========================================================================
// Memory instruction tests
// =========================================================================

// TestExecLoad verifies loading a float from memory into a register.
func TestExecLoad(t *testing.T) {
	isa, regs, mem := newTestRig()

	// Store 3.14 at address 0
	_ = mem.StoreGoFloat(0, 3.14, fp.FP32)

	result := isa.Execute(Load(0, 0, 0), regs, mem)

	val, _ := regs.ReadFloat(0)
	diff := val - 3.14
	if diff < 0 {
		diff = -diff
	}
	if diff > 0.001 {
		t.Errorf("LOAD: expected R0~=3.14, got %g", val)
	}
	if result.RegistersChanged == nil {
		t.Error("LOAD: expected RegistersChanged to be set")
	}
}

// TestExecLoadWithOffset verifies loading with a base register + offset.
func TestExecLoadWithOffset(t *testing.T) {
	isa, regs, mem := newTestRig()

	// Store 42.0 at address 8
	_ = mem.StoreGoFloat(8, 42.0, fp.FP32)
	// Set R1 = 4 (base address)
	_ = regs.WriteFloat(1, 4.0)

	isa.Execute(Load(0, 1, 4.0), regs, mem)

	val, _ := regs.ReadFloat(0)
	if val != 42.0 {
		t.Errorf("LOAD with offset: expected R0=42.0, got %g", val)
	}
}

// TestExecStore verifies storing a register value to memory.
func TestExecStore(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(1, 7.5)

	result := isa.Execute(Store(0, 1, 0), regs, mem)

	val, _ := mem.LoadFloatAsGo(0, fp.FP32)
	if val != 7.5 {
		t.Errorf("STORE: expected Mem[0]=7.5, got %g", val)
	}
	if result.MemoryChanged == nil {
		t.Error("STORE: expected MemoryChanged to be set")
	}
	if result.MemoryChanged[0] != 7.5 {
		t.Errorf("STORE: expected MemoryChanged[0]=7.5, got %g", result.MemoryChanged[0])
	}
}

// TestExecStoreWithOffset verifies storing with a base register + offset.
func TestExecStoreWithOffset(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 4.0)  // base address
	_ = regs.WriteFloat(1, 99.0) // value to store

	isa.Execute(Store(0, 1, 8.0), regs, mem)

	val, _ := mem.LoadFloatAsGo(12, fp.FP32)
	if val != 99.0 {
		t.Errorf("STORE with offset: expected Mem[12]=99.0, got %g", val)
	}
}

// =========================================================================
// Data movement tests
// =========================================================================

// TestExecMov verifies register copy.
func TestExecMov(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 42.0)

	result := isa.Execute(Mov(1, 0), regs, mem)

	val, _ := regs.ReadFloat(1)
	if val != 42.0 {
		t.Errorf("MOV: expected R1=42.0, got %g", val)
	}
	if result.RegistersChanged["R1"] != 42.0 {
		t.Errorf("MOV: expected RegistersChanged[R1]=42.0")
	}
}

// TestExecLimm verifies immediate load.
func TestExecLimm(t *testing.T) {
	isa, regs, mem := newTestRig()

	result := isa.Execute(Limm(0, 3.14), regs, mem)

	val, _ := regs.ReadFloat(0)
	diff := val - 3.14
	if diff < 0 {
		diff = -diff
	}
	if diff > 0.001 {
		t.Errorf("LIMM: expected R0~=3.14, got %g", val)
	}
	if result.RegistersChanged == nil {
		t.Error("LIMM: expected RegistersChanged to be set")
	}
}

// =========================================================================
// Control flow tests
// =========================================================================

// TestExecBeqTaken verifies BEQ when equal (branch taken).
func TestExecBeqTaken(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 5.0)
	_ = regs.WriteFloat(1, 5.0)

	result := isa.Execute(Beq(0, 1, 3), regs, mem)

	if result.NextPCOffset != 3 {
		t.Errorf("BEQ taken: expected NextPCOffset=3, got %d", result.NextPCOffset)
	}
}

// TestExecBeqNotTaken verifies BEQ when not equal (fall through).
func TestExecBeqNotTaken(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 5.0)
	_ = regs.WriteFloat(1, 3.0)

	result := isa.Execute(Beq(0, 1, 3), regs, mem)

	if result.NextPCOffset != 1 {
		t.Errorf("BEQ not taken: expected NextPCOffset=1, got %d", result.NextPCOffset)
	}
}

// TestExecBltTaken verifies BLT when less than (branch taken).
func TestExecBltTaken(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 2.0)
	_ = regs.WriteFloat(1, 5.0)

	result := isa.Execute(Blt(0, 1, 4), regs, mem)

	if result.NextPCOffset != 4 {
		t.Errorf("BLT taken: expected NextPCOffset=4, got %d", result.NextPCOffset)
	}
}

// TestExecBltNotTaken verifies BLT when not less than (fall through).
func TestExecBltNotTaken(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 5.0)
	_ = regs.WriteFloat(1, 2.0)

	result := isa.Execute(Blt(0, 1, 4), regs, mem)

	if result.NextPCOffset != 1 {
		t.Errorf("BLT not taken: expected NextPCOffset=1, got %d", result.NextPCOffset)
	}
}

// TestExecBneTaken verifies BNE when not equal (branch taken).
func TestExecBneTaken(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 3.0)
	_ = regs.WriteFloat(1, 5.0)

	result := isa.Execute(Bne(0, 1, 2), regs, mem)

	if result.NextPCOffset != 2 {
		t.Errorf("BNE taken: expected NextPCOffset=2, got %d", result.NextPCOffset)
	}
}

// TestExecBneNotTaken verifies BNE when equal (fall through).
func TestExecBneNotTaken(t *testing.T) {
	isa, regs, mem := newTestRig()
	_ = regs.WriteFloat(0, 5.0)
	_ = regs.WriteFloat(1, 5.0)

	result := isa.Execute(Bne(0, 1, 2), regs, mem)

	if result.NextPCOffset != 1 {
		t.Errorf("BNE not taken: expected NextPCOffset=1, got %d", result.NextPCOffset)
	}
}

// TestExecJmp verifies unconditional jump.
func TestExecJmp(t *testing.T) {
	isa, regs, mem := newTestRig()

	result := isa.Execute(Jmp(10), regs, mem)

	if result.NextPCOffset != 10 {
		t.Errorf("JMP: expected NextPCOffset=10, got %d", result.NextPCOffset)
	}
	if !result.AbsoluteJump {
		t.Error("JMP: expected AbsoluteJump=true")
	}
}

// TestExecNop verifies no-operation.
func TestExecNop(t *testing.T) {
	isa, regs, mem := newTestRig()

	result := isa.Execute(Nop(), regs, mem)

	if result.NextPCOffset != 1 {
		t.Errorf("NOP: expected NextPCOffset=1, got %d", result.NextPCOffset)
	}
	if result.Description != "No operation" {
		t.Errorf("NOP: unexpected description: %q", result.Description)
	}
}

// TestExecHalt verifies halt.
func TestExecHalt(t *testing.T) {
	isa, regs, mem := newTestRig()

	result := isa.Execute(Halt(), regs, mem)

	if !result.Halted {
		t.Error("HALT: expected Halted=true")
	}
	if result.Description != "Halted" {
		t.Errorf("HALT: unexpected description: %q", result.Description)
	}
}

// TestExecUnknownOpcode verifies the default case.
func TestExecUnknownOpcode(t *testing.T) {
	isa, regs, mem := newTestRig()

	result := isa.Execute(Instruction{Op: Opcode(999)}, regs, mem)

	if result.Description == "" {
		t.Error("unknown opcode should produce a description")
	}
	t.Logf("Unknown opcode result: %s", result.Description)
}
