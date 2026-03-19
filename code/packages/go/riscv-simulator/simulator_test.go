package riscvsimulator

import (
	"testing"
)

func TestSimulator(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 1),
		EncodeAddi(2, 0, 2),
		EncodeAdd(3, 1, 2),
		EncodeSub(4, 3, 1),
		EncodeEcall(),
	})
	
	traces := sim.Run(program)
	if len(traces) != 5 {
		t.Fatalf("Expected 5 instruction traces, got %d", len(traces))
	}
	
	r1 := sim.CPU.Registers.Read(1)
	if r1 != 1 { t.Errorf("Expected R1=1, got %d", r1) }
	
	r3 := sim.CPU.Registers.Read(3)
	if r3 != 3 { t.Errorf("Expected R3=3, got %d", r3) }
	
	r4 := sim.CPU.Registers.Read(4)
	if r4 != 2 { t.Errorf("Expected R4=2, got %d", r4) }
}

func TestRegisterZeroHardwired(t *testing.T) {
	sim := NewRiscVSimulator(1024)
	program := Assemble([]uint32{
		EncodeAddi(0, 0, 42),
		EncodeEcall(),
	})
	sim.Run(program)
	if sim.CPU.Registers.Read(0) != 0 {
		t.Errorf("x0 must be hardwired to zero, but allowed a write operation")
	}
}

func TestNegativeImmediate(t *testing.T) {
	sim := NewRiscVSimulator(1024)
	program := Assemble([]uint32{
		EncodeAddi(1, 0, -5),
		EncodeEcall(),
	})
	sim.Run(program)
	val := int32(sim.CPU.Registers.Read(1))
	if val != -5 {
		t.Errorf("Negative immediate decoding failed, got %d", val)
	}
}

func TestUnknownInstruction(t *testing.T) {
	sim := NewRiscVSimulator(1024)
	program := Assemble([]uint32{
		0xFFFFFFFF,
		EncodeEcall(),
	})
	traces := sim.Run(program)
	if traces[0].Execute.Description == "" {
		t.Errorf("Unknown instruction should have default safe description fallback")
	}
}
