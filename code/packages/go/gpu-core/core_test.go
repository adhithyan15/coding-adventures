package gpucore

import (
	"testing"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// =========================================================================
// Constructor tests
// =========================================================================

// TestNewGPUCoreDefaults verifies the default configuration.
func TestNewGPUCoreDefaults(t *testing.T) {
	core := NewGPUCore()

	if core.ISA.Name() != "Generic" {
		t.Errorf("expected ISA name 'Generic', got %q", core.ISA.Name())
	}
	if core.Registers.NumRegisters != 32 {
		t.Errorf("expected 32 registers, got %d", core.Registers.NumRegisters)
	}
	if core.Memory.Size != 4096 {
		t.Errorf("expected 4096 bytes memory, got %d", core.Memory.Size)
	}
	if core.PC != 0 {
		t.Errorf("expected PC=0, got %d", core.PC)
	}
	if core.Cycle != 0 {
		t.Errorf("expected Cycle=0, got %d", core.Cycle)
	}
	if core.IsHalted() {
		t.Error("expected core not halted")
	}
}

// TestNewGPUCoreWithOptions verifies custom configuration.
func TestNewGPUCoreWithOptions(t *testing.T) {
	core := NewGPUCore(
		WithNumRegisters(64),
		WithMemorySize(8192),
	)

	if core.Registers.NumRegisters != 64 {
		t.Errorf("expected 64 registers, got %d", core.Registers.NumRegisters)
	}
	if core.Memory.Size != 8192 {
		t.Errorf("expected 8192 bytes memory, got %d", core.Memory.Size)
	}
}

// TestNewGPUCoreWithISA verifies custom ISA injection.
func TestNewGPUCoreWithISA(t *testing.T) {
	core := NewGPUCore(WithISA(GenericISA{}))
	if core.ISA.Name() != "Generic" {
		t.Errorf("expected ISA name 'Generic', got %q", core.ISA.Name())
	}
}

// TestNewGPUCorePanicsOnBadConfig verifies that invalid config panics.
func TestNewGPUCorePanicsOnBadRegisterConfig(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic for invalid register count")
		}
	}()
	NewGPUCore(WithNumRegisters(0))
}

func TestNewGPUCorePanicsOnBadMemoryConfig(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic for invalid memory size")
		}
	}()
	NewGPUCore(WithMemorySize(0))
}

// =========================================================================
// LoadProgram tests
// =========================================================================

// TestLoadProgram verifies that loading a program resets PC and cycle.
func TestLoadProgram(t *testing.T) {
	core := NewGPUCore()
	program := []Instruction{Limm(0, 1.0), Halt()}
	core.LoadProgram(program)

	if core.PC != 0 {
		t.Errorf("expected PC=0 after LoadProgram, got %d", core.PC)
	}
	if core.Cycle != 0 {
		t.Errorf("expected Cycle=0 after LoadProgram, got %d", core.Cycle)
	}
	if core.IsHalted() {
		t.Error("expected core not halted after LoadProgram")
	}
}

// =========================================================================
// Step tests
// =========================================================================

// TestStepBasic verifies a single step execution.
func TestStepBasic(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{Limm(0, 42.0), Halt()})

	trace, err := core.Step()
	if err != nil {
		t.Fatalf("Step error: %v", err)
	}

	if trace.Cycle != 1 {
		t.Errorf("expected cycle=1, got %d", trace.Cycle)
	}
	if trace.PC != 0 {
		t.Errorf("expected PC=0, got %d", trace.PC)
	}
	if core.PC != 1 {
		t.Errorf("expected PC=1 after step, got %d", core.PC)
	}

	val, _ := core.Registers.ReadFloat(0)
	if val != 42.0 {
		t.Errorf("expected R0=42.0, got %g", val)
	}
}

// TestStepHalted verifies that stepping a halted core returns an error.
func TestStepHalted(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{Halt()})

	_, err := core.Step()
	if err != nil {
		t.Fatalf("first Step error: %v", err)
	}

	if !core.IsHalted() {
		t.Error("expected core to be halted")
	}

	_, err = core.Step()
	if err == nil {
		t.Error("expected error when stepping halted core")
	}
}

// TestStepPCOutOfRange verifies that stepping with PC out of range errors.
func TestStepPCOutOfRange(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{Nop()})
	core.PC = 5 // beyond program length

	_, err := core.Step()
	if err == nil {
		t.Error("expected error for PC out of range")
	}
}

// TestStepAbsoluteJump verifies that JMP sets PC to an absolute address.
func TestStepAbsoluteJump(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Jmp(2),      // 0: jump to PC=2
		Limm(0, 1.0), // 1: should be skipped
		Halt(),      // 2: should land here
	})

	trace, _ := core.Step()
	if trace.NextPC != 2 {
		t.Errorf("expected NextPC=2, got %d", trace.NextPC)
	}
	if core.PC != 2 {
		t.Errorf("expected PC=2 after JMP, got %d", core.PC)
	}
}

// TestStepHaltDoesNotAdvancePC verifies that HALT keeps PC at current position.
func TestStepHaltDoesNotAdvancePC(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{Halt()})

	trace, _ := core.Step()
	if trace.NextPC != 0 {
		t.Errorf("expected NextPC=0 (halt), got %d", trace.NextPC)
	}
	if trace.Halted != true {
		t.Error("expected trace.Halted=true")
	}
}

// =========================================================================
// Run tests
// =========================================================================

// TestRunSimple verifies running a complete program.
func TestRunSimple(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 3.0),
		Limm(1, 4.0),
		Fmul(2, 0, 1),
		Halt(),
	})

	traces, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	if len(traces) != 4 {
		t.Errorf("expected 4 traces, got %d", len(traces))
	}
	if !core.IsHalted() {
		t.Error("expected core to be halted")
	}

	val, _ := core.Registers.ReadFloat(2)
	if val != 12.0 {
		t.Errorf("expected R2=12.0, got %g", val)
	}
}

// TestRunMaxSteps verifies that the max steps limit is enforced.
func TestRunMaxSteps(t *testing.T) {
	core := NewGPUCore()
	// Infinite loop: NOP then jump back to start
	core.LoadProgram([]Instruction{
		Nop(),
		Jmp(0),
	})

	_, err := core.Run(10)
	if err == nil {
		t.Error("expected error for exceeding max steps")
	}
}

// =========================================================================
// Reset tests
// =========================================================================

// TestReset verifies that Reset() clears all state but preserves the program.
func TestReset(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 42.0),
		Halt(),
	})
	core.Run(100)

	// Verify state after run
	if !core.IsHalted() {
		t.Error("expected halted before reset")
	}
	val, _ := core.Registers.ReadFloat(0)
	if val != 42.0 {
		t.Errorf("expected R0=42.0 before reset, got %g", val)
	}

	// Reset
	core.Reset()

	if core.IsHalted() {
		t.Error("expected not halted after reset")
	}
	if core.PC != 0 {
		t.Errorf("expected PC=0 after reset, got %d", core.PC)
	}
	if core.Cycle != 0 {
		t.Errorf("expected Cycle=0 after reset, got %d", core.Cycle)
	}
	val, _ = core.Registers.ReadFloat(0)
	if val != 0.0 {
		t.Errorf("expected R0=0.0 after reset, got %g", val)
	}

	// Program should still be loaded -- run it again
	traces, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run after reset error: %v", err)
	}
	if len(traces) != 2 {
		t.Errorf("expected 2 traces after re-run, got %d", len(traces))
	}
}

// =========================================================================
// String tests
// =========================================================================

// TestGPUCoreString verifies the string representation.
func TestGPUCoreString(t *testing.T) {
	core := NewGPUCore()
	s := core.String()
	if s == "" {
		t.Error("expected non-empty string")
	}
	t.Logf("Core string (running): %s", s)

	core.LoadProgram([]Instruction{Halt()})
	core.Run(100)
	s = core.String()
	t.Logf("Core string (halted): %s", s)
}

// =========================================================================
// Halted/ProcessingElement interface tests
// =========================================================================

func TestHaltedMethod(t *testing.T) {
	core := NewGPUCore()
	if core.Halted() {
		t.Error("expected not halted initially")
	}
	core.LoadProgram([]Instruction{Halt()})
	core.Run(100)
	if !core.Halted() {
		t.Error("expected halted after running HALT program")
	}
}

// TestWithFormat verifies the WithFormat option.
func TestWithFormat(t *testing.T) {
	// Just ensure it doesn't panic and creates a working core.
	core := NewGPUCore(WithFormat(fp.FP32))
	core.LoadProgram([]Instruction{Limm(0, 1.0), Halt()})
	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error with FP32 format: %v", err)
	}
}
