package intel4004simulator

import "testing"

func TestIntel4004(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	// Program computing: x = 1 + 2
	program := []byte{
		EncodeLdm(1),
		EncodeXch(0),
		EncodeLdm(2),
		EncodeAdd(0),
		EncodeXch(1),
		EncodeHlt(),
	}
	traces := sim.Run(program, 1000)
	// We expect 6 cycles vs RISC-V completing this sequence in 4 (setup/execution difference).
	if len(traces) != 6 {
		t.Fatalf("Expected 6 traces, got %d", len(traces))
	}
	if sim.Registers[1] != 3 {
		t.Errorf("R1 should be 3, got %d", sim.Registers[1])
	}
}

func TestSubBorrow(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	program := []byte{
		EncodeLdm(1),
		EncodeXch(0), // store 1 into R0
		EncodeLdm(0), // A=0
		EncodeSub(0), // A = A - R0 = 0 - 1 = -1 (which wraps around and becomes 15), borrow causes carry=true
		EncodeHlt(),
	}
	sim.Run(program, 10)
	if sim.Accumulator != 15 {
		t.Errorf("A should be 15, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Errorf("Carry (borrow) should be true")
	}
}

func TestUnknownInstruction(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	program := []byte{
		0xFF,
		EncodeHlt(),
	}
	traces := sim.Run(program, 10)
	if traces[0].Mnemonic == "" {
		t.Errorf("Unknown instruction shouldn't be blank")
	}
}

func TestHaltedPanic(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	sim.Halted = true
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Expected panic from stepping while halted")
		}
	}()
	sim.Step()
}
