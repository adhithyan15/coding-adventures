package jvmsimulator

import "testing"

func TestJVMSimulator(t *testing.T) {
	sim := NewJVMSimulator()
	// x = 1 + 2
	prog := AssembleJvm([]Instr{
		{Opcode: OpIconst1},
		{Opcode: OpIconst2},
		{Opcode: OpIadd},
		{Opcode: OpIstore0},
		{Opcode: OpIload0},
		{Opcode: OpIreturn},
	})
	sim.Load(prog, nil, 16)
	traces := sim.Run(100)
	
	if len(traces) != 6 {
		t.Fatalf("Expected 6 traces, got %d", len(traces))
	}
	if sim.ReturnValue == nil || *sim.ReturnValue != 3 {
		t.Errorf("Return value should be 3")
	}
}

func TestJVMLdcAndBipush(t *testing.T) {
	sim := NewJVMSimulator()
	prog := AssembleJvm([]Instr{
		{Opcode: OpBipush, Params: []int{-42}},
		{Opcode: OpLdc, Params: []int{0}},
		{Opcode: OpIsub},
		{Opcode: OpIreturn},
	})
	
	sim.Load(prog, []interface{}{100}, 16)
	sim.Run(100)
	
	if sim.ReturnValue == nil || *sim.ReturnValue != -142 {
		t.Errorf("Should be -142, got %d", *sim.ReturnValue)
	}
}

func TestJVMIcmpGoto(t *testing.T) {
	sim := NewJVMSimulator()
	// push 5, push 5, cmp eq (+3 jump), push 1 (skipped), push 10, return.
	prog := AssembleJvm([]Instr{
		{Opcode: OpIconst5},
		{Opcode: OpIconst5},
		{Opcode: OpIfIcmpeq, Params: []int{4}}, // 4 bytes relative from PC 2 -> PC 6.
		{Opcode: OpIconst1}, // byte: 5
		{Opcode: OpIconst4}, // byte: 6 (target!)
		{Opcode: OpIreturn}, // byte: 7
	})
	// instruction lengths
	// 0: iconst_5 (1 byte) -> pc 1
	// 1: iconst_5 (1 byte) -> pc 2
	// 2: if_icmpeq (3 bytes) -> relative offset + 5 lands at pc 7
	// 5: iconst_1 (1 byte)
	// 6: iadd (1 byte) -- let's make it iconst_1 directly
	
	sim.Load(prog, nil, 16)
	traces := sim.Run(10)
	if *sim.ReturnValue != 4 {
		t.Errorf("Should return 4 because icmpeq skipped iconst_1. Returned %v", *sim.ReturnValue)
	}
	
	// Ensure jump was mapped correctly in traces
	found := false
	for _, trace := range traces {
		if trace.Opcode == "if_icmpeq" {
			found = true
		}
	}
	if !found {
		t.Fatalf("if_icmpeq not traced")
	}
}

func TestJVMidivZeroFail(t *testing.T) {
	sim := NewJVMSimulator()
	prog := AssembleJvm([]Instr{
		{Opcode: OpIconst5},
		{Opcode: OpIconst0},
		{Opcode: OpIdiv},
	})
	sim.Load(prog, nil, 16)
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Zero division should panic")
		}
	}()
	sim.Run(10)
}
