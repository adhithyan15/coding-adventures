package gpucore

import "testing"

// =========================================================================
// ExecuteResult tests
// =========================================================================

// TestNewExecuteResult verifies that the default constructor sets sensible
// defaults: PC advances by 1, no halt, no register/memory changes.
func TestNewExecuteResult(t *testing.T) {
	result := NewExecuteResult("test description")

	if result.Description != "test description" {
		t.Errorf("expected description 'test description', got %q", result.Description)
	}
	if result.NextPCOffset != 1 {
		t.Errorf("expected NextPCOffset=1, got %d", result.NextPCOffset)
	}
	if result.AbsoluteJump {
		t.Error("expected AbsoluteJump=false")
	}
	if result.Halted {
		t.Error("expected Halted=false")
	}
	if result.RegistersChanged != nil {
		t.Error("expected RegistersChanged=nil")
	}
	if result.MemoryChanged != nil {
		t.Error("expected MemoryChanged=nil")
	}
}

// TestExecuteResultWithAllFields verifies that all fields can be set.
func TestExecuteResultWithAllFields(t *testing.T) {
	result := ExecuteResult{
		Description:      "halted",
		NextPCOffset:     0,
		AbsoluteJump:     true,
		RegistersChanged: map[string]float64{"R0": 1.0},
		MemoryChanged:    map[int]float64{0: 3.14},
		Halted:           true,
	}

	if !result.Halted {
		t.Error("expected Halted=true")
	}
	if !result.AbsoluteJump {
		t.Error("expected AbsoluteJump=true")
	}
	if result.RegistersChanged["R0"] != 1.0 {
		t.Errorf("expected R0=1.0, got %g", result.RegistersChanged["R0"])
	}
	if result.MemoryChanged[0] != 3.14 {
		t.Errorf("expected Mem[0]=3.14, got %g", result.MemoryChanged[0])
	}
}

// =========================================================================
// Interface compliance tests
// =========================================================================

// TestGenericISAImplementsInstructionSet is a compile-time check that
// GenericISA satisfies the InstructionSet interface.
func TestGenericISAImplementsInstructionSet(t *testing.T) {
	var _ InstructionSet = GenericISA{}
	t.Log("GenericISA satisfies InstructionSet interface")
}

// TestGPUCoreImplementsProcessingElement is a compile-time check that
// *GPUCore satisfies the ProcessingElement interface.
func TestGPUCoreImplementsProcessingElement(t *testing.T) {
	var _ ProcessingElement = &GPUCore{}
	t.Log("*GPUCore satisfies ProcessingElement interface")
}

// TestStepOneWrapper verifies that StepOne wraps Step correctly.
func TestStepOneWrapper(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{Limm(0, 1.0), Halt()})

	result, err := core.StepOne()
	if err != nil {
		t.Fatalf("StepOne error: %v", err)
	}
	trace, ok := result.(GPUCoreTrace)
	if !ok {
		t.Fatal("StepOne should return a GPUCoreTrace")
	}
	if trace.Cycle != 1 {
		t.Errorf("expected cycle=1, got %d", trace.Cycle)
	}
}
