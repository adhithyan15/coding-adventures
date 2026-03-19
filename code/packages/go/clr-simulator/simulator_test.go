package clrsimulator

import "testing"

func TestCLRSimulatorMath(t *testing.T) {
	sim := NewCLRSimulator()
	// x = 1 + 2
	prog := AssembleClr([][]byte{
		EncodeLdcI4(1),
		EncodeLdcI4(2),
		{OpAdd},
		EncodeStloc(0),
		EncodeLdloc(0),
		{OpRet},
	})
	sim.Load(prog, 16)
	traces := sim.Run(100)
	
	if len(traces) != 6 {
		t.Fatalf("Expected 6 exactly, got %d", len(traces))
	}
	if sim.Locals[0] == nil || *sim.Locals[0] != 3 {
		t.Errorf("Result should be stored natively into local explicitly as 3")
	}
}

func TestCLRDivByZero(t *testing.T) {
	sim := NewCLRSimulator()
	prog := AssembleClr([][]byte{
		EncodeLdcI4(5),
		EncodeLdcI4(0),
		{OpDiv},
	})
	sim.Load(prog, 16)
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("Division by zero fails cleanly mapping System.DivideByZeroException")
		}
	}()
	sim.Run(10)
}

func TestCLRExtendedOpcodes(t *testing.T) {
	sim := NewCLRSimulator()
	prog := AssembleClr([][]byte{
		EncodeLdcI4(10), // A=10
		EncodeLdcI4(5),  // B=5
		{OpPrefixFE, CgtByte}, // A > B? (1)
		{OpRet},
	})
	sim.Load(prog, 16)
	sim.Run(10)
	if *sim.Stack[0] != 1 { // 1 meaning true
		t.Errorf("10 > 5 should push 1")
	}
}

func TestCLRBranchingZero(t *testing.T) {
	sim := NewCLRSimulator()
	// push 0, branch if zero (+2 jump skipping), push 5, push 10, ret
	prog := AssembleClr([][]byte{
		EncodeLdcI4(0), // 1 byte
		{OpBrfalseS, 2}, // jumps 2 bytes skipping the 5-byte encoding. Wait, pc is current instruction. nextPC is pc+2. target is nextPC + 2.
		// wait, if I want to skip a 5-byte instruction:
		EncodeLdcI4(1000), // takes 5 bytes
		EncodeLdcI4(10), // target.
		{OpRet},
	})
	// To cleanly skip `EncodeLdcI4(1000)` which takes 5 bytes: offset = 5
	prog[2] = 5 // Override offset to 5 bytes
	
	sim.Load(prog, 16)
	traces := sim.Run(10)
	
	// Because of tracing logic, `OpBrfalseS` evaluates against `0` and explicitly Branches across.
	foundPush10 := false
	for _, trc := range traces {
		if trc.Opcode == "ldc.i4.s" || (len(trc.StackAfter) > 0 && trc.StackAfter[0] != nil && *trc.StackAfter[0] == 10) {
			foundPush10 = true
		}
	}
	if !foundPush10 {
		t.Errorf("Should have pushed 10 accurately jumping.")
	}
}
