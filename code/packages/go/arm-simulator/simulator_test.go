package armsimulator

import "testing"

func TestARMSimulator(t *testing.T) {
	sim := NewARMSimulator(65536)
	program := Assemble([]uint32{
		EncodeMovImm(0, 1),
		EncodeMovImm(1, 2),
		EncodeAdd(2, 0, 1),
		EncodeSub(3, 2, 0),
		EncodeHlt(),
	})
	
	traces := sim.Run(program)
	if len(traces) != 5 {
		t.Fatalf("Expected 5 traces, got %d", len(traces))
	}
	
	if sim.CPU.Registers.Read(0) != 1 { t.Errorf("R0 wrong") }
	if sim.CPU.Registers.Read(1) != 2 { t.Errorf("R1 wrong") }
	if sim.CPU.Registers.Read(2) != 3 { t.Errorf("R2 wrong") }
	if sim.CPU.Registers.Read(3) != 2 { t.Errorf("R3 wrong") }
}

func TestARMRotateDecode(t *testing.T) {
	sim := NewARMSimulator(1024)
	
	// Create an instruction with a rotate to test the rotate decode logic
	// e.g. imm = 1, rotate = 1 -> shifted right by 2 positions = 0x40000000
	cond := uint32(CondAL)
	raw := (cond << 28) | (1 << 25) | (OpcodeMov << 21) | (1 << 12) | (1 << 8) | 1
	program := Assemble([]uint32{raw, EncodeHlt()})
	
	sim.Run(program)
	val := sim.CPU.Registers.Read(1)
	if val != 0x40000000 {
		t.Errorf("Rotate right by 2 of 1 should be 0x40000000, got %08X", val)
	}
}

func TestUnknownOpcode(t *testing.T) {
    sim := NewARMSimulator(1024)
    program := Assemble([]uint32{
        (CondAL << 28) | (0xF << 21),
        EncodeHlt(),
    })
    traces := sim.Run(program)
    if traces[0].Decode.Mnemonic == "" {
		t.Errorf("Unknown instruction should not be blank")
	}
}
