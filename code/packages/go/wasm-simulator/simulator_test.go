package wasmsimulator

import "testing"

func TestWasmSimulator(t *testing.T) {
	sim := NewWasmSimulator(4)
	program := AssembleWasm([][]byte{
		EncodeI32Const(1),
		EncodeI32Const(2),
		EncodeI32Add(),
		EncodeLocalSet(0),
		EncodeLocalGet(0),
		EncodeI32Const(5),
		EncodeI32Sub(),
		EncodeEnd(),
	})

	traces := sim.Run(program, 1000)
	// Program breakdown:
	// 1. push 1 (stack: [1])
	// 2. push 2 (stack: [1, 2])
	// 3. add    (stack: [3])
	// 4. set l0 (stack: []) locals[0] = 3
	// 5. get l0 (stack: [3])
	// 6. push 5 (stack: [3, 5])
	// 7. sub    (stack: [-2/truncated])
	// 8. end
	if len(traces) != 8 {
		t.Fatalf("Expected 8 traces, got %d", len(traces))
	}

	if sim.Locals[0] != 3 {
		t.Errorf("Local 0 should be 3")
	}
	
	val := 4294967294 // Equivalent to -2 masked to 32-bit unsigned
	if len(sim.Stack) != 1 || sim.Stack[0] != val {
		t.Errorf("Stack top should be %d, got %+v", val, sim.Stack)
	}
}

func TestHaltedPanic(t *testing.T) {
	sim := NewWasmSimulator(1)
	program := AssembleWasm([][]byte{
		EncodeEnd(),
	})
	sim.Run(program, 10)
	
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Expected panic after halted")
		}
	}()
	sim.Step()
}

func TestUnknownOpcode(t *testing.T) {
	sim := NewWasmSimulator(1)
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Should panic on unknown opcode")
		}
	}()
	program := AssembleWasm([][]byte{
		{0xFF},
	})
	sim.Run(program, 10)
}

func TestExecutorPanicFail(t *testing.T) {
	executor := &WasmExecutor{}
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Should panic on unknown instruction mnemonic")
		}
	}()
	instruction := WasmInstruction{Mnemonic: "unknown.command"}
	stack := []int{1, 2}
	locals := []int{0, 0}
	
	executor.Execute(instruction, &stack, locals, 0)
}
