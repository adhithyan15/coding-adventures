package arm1gatelevel

import (
	"encoding/binary"
	"testing"

	sim "github.com/adhithyan15/coding-adventures/code/packages/go/arm1-simulator"
)

// =========================================================================
// Helper: load program from instruction words
// =========================================================================

func loadProgram(cpu *ARM1GateLevel, instructions []uint32) {
	code := make([]byte, len(instructions)*4)
	for i, inst := range instructions {
		binary.LittleEndian.PutUint32(code[i*4:], inst)
	}
	cpu.LoadProgram(code, 0)
}

func loadBehavioral(cpu *sim.ARM1, instructions []uint32) {
	code := make([]byte, len(instructions)*4)
	for i, inst := range instructions {
		binary.LittleEndian.PutUint32(code[i*4:], inst)
	}
	cpu.LoadProgram(code, 0)
}

// =========================================================================
// Bit conversion
// =========================================================================

func TestIntToBits(t *testing.T) {
	bits := IntToBits(5, 32)
	if bits[0] != 1 || bits[2] != 1 {
		t.Errorf("IntToBits(5) bit0=%d bit2=%d, want 1,1", bits[0], bits[2])
	}
	if BitsToInt(bits) != 5 {
		t.Errorf("round-trip failed: got %d want 5", BitsToInt(bits))
	}
}

func TestBitsRoundTrip(t *testing.T) {
	values := []uint32{0, 1, 42, 0xFF, 0xDEADBEEF, 0xFFFFFFFF}
	for _, v := range values {
		bits := IntToBits(v, 32)
		got := BitsToInt(bits)
		if got != v {
			t.Errorf("round-trip(%d): got %d", v, got)
		}
	}
}

// =========================================================================
// Gate-level ALU
// =========================================================================

func TestGateALUAdd(t *testing.T) {
	a := IntToBits(1, 32)
	b := IntToBits(2, 32)
	r := GateALUExecute(sim.OpADD, a, b, 0, 0, 0)
	result := BitsToInt(r.Result)
	if result != 3 {
		t.Errorf("1 + 2 = %d, want 3", result)
	}
	if r.N != 0 || r.Z != 0 || r.C != 0 || r.V != 0 {
		t.Errorf("flags wrong: N=%d Z=%d C=%d V=%d", r.N, r.Z, r.C, r.V)
	}
}

func TestGateALUSubZero(t *testing.T) {
	a := IntToBits(5, 32)
	b := IntToBits(5, 32)
	r := GateALUExecute(sim.OpSUB, a, b, 0, 0, 0)
	result := BitsToInt(r.Result)
	if result != 0 {
		t.Errorf("5 - 5 = %d, want 0", result)
	}
	if r.Z != 1 {
		t.Error("Z should be set for 5-5=0")
	}
	if r.C != 1 {
		t.Error("C should be set (no borrow)")
	}
}

func TestGateALULogical(t *testing.T) {
	a := IntToBits(0xFF00FF00, 32)
	b := IntToBits(0x0FF00FF0, 32)

	// AND
	r := GateALUExecute(sim.OpAND, a, b, 0, 0, 0)
	if BitsToInt(r.Result) != 0x0F000F00 {
		t.Errorf("AND = 0x%X, want 0x0F000F00", BitsToInt(r.Result))
	}

	// EOR
	r = GateALUExecute(sim.OpEOR, a, b, 0, 0, 0)
	if BitsToInt(r.Result) != 0xF0F0F0F0 {
		t.Errorf("EOR = 0x%X, want 0xF0F0F0F0", BitsToInt(r.Result))
	}

	// ORR
	r = GateALUExecute(sim.OpORR, a, b, 0, 0, 0)
	if BitsToInt(r.Result) != 0xFFF0FFF0 {
		t.Errorf("ORR = 0x%X, want 0xFFF0FFF0", BitsToInt(r.Result))
	}
}

// =========================================================================
// Gate-level barrel shifter
// =========================================================================

func TestGateBarrelShiftLSL(t *testing.T) {
	value := IntToBits(0xFF, 32)
	result, _ := GateBarrelShift(value, 0, 4, 0, false)
	got := BitsToInt(result)
	if got != 0xFF0 {
		t.Errorf("LSL #4 of 0xFF = 0x%X, want 0xFF0", got)
	}
}

func TestGateBarrelShiftLSR(t *testing.T) {
	value := IntToBits(0xFF00, 32)
	result, _ := GateBarrelShift(value, 1, 8, 0, false)
	got := BitsToInt(result)
	if got != 0xFF {
		t.Errorf("LSR #8 of 0xFF00 = 0x%X, want 0xFF", got)
	}
}

func TestGateBarrelShiftROR(t *testing.T) {
	value := IntToBits(0x0000000F, 32)
	result, _ := GateBarrelShift(value, 3, 4, 0, false)
	got := BitsToInt(result)
	if got != 0xF0000000 {
		t.Errorf("ROR #4 of 0xF = 0x%X, want 0xF0000000", got)
	}
}

func TestGateBarrelShiftRRX(t *testing.T) {
	value := IntToBits(0x00000001, 32)
	result, carry := GateBarrelShift(value, 3, 0, 1, false)
	got := BitsToInt(result)
	if got != 0x80000000 {
		t.Errorf("RRX of 1 with C=1: value = 0x%X, want 0x80000000", got)
	}
	if carry != 1 {
		t.Error("RRX carry should be 1 (old bit 0 was 1)")
	}
}

// =========================================================================
// Cross-validation: Gate-level vs Behavioral
// =========================================================================
//
// This is the ultimate correctness guarantee. We run the same program on
// both simulators and verify they produce identical results.

func crossValidate(t *testing.T, name string, instructions []uint32) {
	t.Helper()

	behavioral := sim.New(4096)
	gateLev := NewGateLevel(4096)

	loadBehavioral(behavioral, instructions)
	loadProgram(gateLev, instructions)

	bTraces := behavioral.Run(200)
	gTraces := gateLev.Run(200)

	if len(bTraces) != len(gTraces) {
		t.Fatalf("%s: trace count mismatch: behavioral=%d gate-level=%d",
			name, len(bTraces), len(gTraces))
	}

	for i := range bTraces {
		bt := bTraces[i]
		gt := gTraces[i]

		if bt.Address != gt.Address {
			t.Errorf("%s step %d: address mismatch: B=0x%X G=0x%X", name, i, bt.Address, gt.Address)
		}
		if bt.ConditionMet != gt.ConditionMet {
			t.Errorf("%s step %d: condition mismatch: B=%v G=%v", name, i, bt.ConditionMet, gt.ConditionMet)
		}

		// Compare final register state
		for r := 0; r < 16; r++ {
			if bt.RegsAfter[r] != gt.RegsAfter[r] {
				t.Errorf("%s step %d: R%d mismatch: B=0x%X G=0x%X",
					name, i, r, bt.RegsAfter[r], gt.RegsAfter[r])
			}
		}

		// Compare flags
		if bt.FlagsAfter != gt.FlagsAfter {
			t.Errorf("%s step %d: flags mismatch: B=%+v G=%+v",
				name, i, bt.FlagsAfter, gt.FlagsAfter)
		}
	}

	t.Logf("%s: %d steps validated, gate-level used ~%d gate ops",
		name, len(bTraces), gateLev.GateOps())
}

func TestCrossValidateOnePlusTwo(t *testing.T) {
	crossValidate(t, "1+2", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 1),
		sim.EncodeMovImm(sim.CondAL, 1, 2),
		sim.EncodeALUReg(sim.CondAL, sim.OpADD, 0, 2, 0, 1),
		sim.EncodeHalt(),
	})
}

func TestCrossValidateSUBSWithFlags(t *testing.T) {
	crossValidate(t, "SUBS", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 5),
		sim.EncodeMovImm(sim.CondAL, 1, 5),
		sim.EncodeALUReg(sim.CondAL, sim.OpSUB, 1, 2, 0, 1),
		sim.EncodeHalt(),
	})
}

func TestCrossValidateConditional(t *testing.T) {
	crossValidate(t, "conditional", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 5),
		sim.EncodeMovImm(sim.CondAL, 1, 5),
		sim.EncodeALUReg(sim.CondAL, sim.OpSUB, 1, 2, 0, 1),
		sim.EncodeMovImm(sim.CondNE, 3, 99),
		sim.EncodeMovImm(sim.CondEQ, 4, 42),
		sim.EncodeHalt(),
	})
}

func TestCrossValidateBarrelShifter(t *testing.T) {
	// ADD R1, R0, R0, LSL #2 (multiply by 5)
	addWithShift := uint32(sim.CondAL)<<28 |
		uint32(sim.OpADD)<<21 |
		uint32(0)<<16 |
		uint32(1)<<12 |
		uint32(2)<<7 |
		uint32(sim.ShiftLSL)<<5 |
		uint32(0)

	crossValidate(t, "barrel_shifter", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 7),
		addWithShift,
		sim.EncodeHalt(),
	})
}

func TestCrossValidateLoop(t *testing.T) {
	crossValidate(t, "loop_sum_1_to_10", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 0),
		sim.EncodeMovImm(sim.CondAL, 1, 10),
		sim.EncodeALUReg(sim.CondAL, sim.OpADD, 0, 0, 0, 1),
		sim.EncodeDataProcessing(sim.CondAL, sim.OpSUB, 1, 1, 1, (1<<25)|1),
		sim.EncodeBranch(sim.CondNE, false, -16),
		sim.EncodeHalt(),
	})
}

func TestCrossValidateLDRSTR(t *testing.T) {
	crossValidate(t, "ldr_str", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 42),
		sim.EncodeDataProcessing(sim.CondAL, sim.OpMOV, 0, 0, 1, (1<<25)|(12<<8)|1),
		sim.EncodeSTR(sim.CondAL, 0, 1, 0, true),
		sim.EncodeMovImm(sim.CondAL, 0, 0),
		sim.EncodeLDR(sim.CondAL, 0, 1, 0, true),
		sim.EncodeHalt(),
	})
}

func TestCrossValidateSTMLDM(t *testing.T) {
	crossValidate(t, "stm_ldm", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 10),
		sim.EncodeMovImm(sim.CondAL, 1, 20),
		sim.EncodeMovImm(sim.CondAL, 2, 30),
		sim.EncodeMovImm(sim.CondAL, 3, 40),
		sim.EncodeDataProcessing(sim.CondAL, sim.OpMOV, 0, 0, 5, (1<<25)|(12<<8)|1),
		sim.EncodeSTM(sim.CondAL, 5, 0x000F, true, "IA"),
		sim.EncodeMovImm(sim.CondAL, 0, 0),
		sim.EncodeMovImm(sim.CondAL, 1, 0),
		sim.EncodeMovImm(sim.CondAL, 2, 0),
		sim.EncodeMovImm(sim.CondAL, 3, 0),
		sim.EncodeDataProcessing(sim.CondAL, sim.OpMOV, 0, 0, 5, (1<<25)|(12<<8)|1),
		sim.EncodeLDM(sim.CondAL, 5, 0x000F, true, "IA"),
		sim.EncodeHalt(),
	})
}

func TestCrossValidateBranchAndLink(t *testing.T) {
	crossValidate(t, "branch_and_link", []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 7),
		sim.EncodeBranch(sim.CondAL, true, 4),
		sim.EncodeHalt(),
		0,
		sim.EncodeALUReg(sim.CondAL, sim.OpADD, 0, 0, 0, 0),
		sim.EncodeDataProcessing(sim.CondAL, sim.OpMOV, 1, 0, 15, uint32(14)),
	})
}

// =========================================================================
// Gate-level specific tests
// =========================================================================

func TestGateLevelNewAndReset(t *testing.T) {
	cpu := NewGateLevel(1024)
	if cpu.Mode() != sim.ModeSVC {
		t.Errorf("mode = %d, want SVC", cpu.Mode())
	}
	if cpu.PC() != 0 {
		t.Errorf("PC = 0x%X, want 0", cpu.PC())
	}
}

func TestGateLevelHalt(t *testing.T) {
	cpu := NewGateLevel(1024)
	loadProgram(cpu, []uint32{sim.EncodeHalt()})
	traces := cpu.Run(10)
	if !cpu.Halted() {
		t.Error("should be halted")
	}
	if len(traces) != 1 {
		t.Errorf("expected 1 trace, got %d", len(traces))
	}
}

func TestGateLevelGateOpsTracking(t *testing.T) {
	cpu := NewGateLevel(1024)
	loadProgram(cpu, []uint32{
		sim.EncodeMovImm(sim.CondAL, 0, 42),
		sim.EncodeHalt(),
	})
	cpu.Run(10)
	if cpu.GateOps() == 0 {
		t.Error("gate ops should be non-zero after execution")
	}
}
