package gpucore

import (
	"math"
	"testing"
)

// =========================================================================
// End-to-end program tests
// =========================================================================
//
// These tests run complete GPU programs -- multi-instruction sequences that
// exercise the full pipeline from load to compute to store. They serve as
// integration tests proving that all the pieces (core, ISA, registers,
// memory) work together correctly.

// TestProgramMultiply verifies a simple multiplication program:
//
//	R0 = 3.0
//	R1 = 4.0
//	R2 = R0 * R1 = 12.0
func TestProgramMultiply(t *testing.T) {
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

	val, _ := core.Registers.ReadFloat(2)
	if val != 12.0 {
		t.Errorf("expected R2=12.0, got %g", val)
	}
}

// TestProgramDotProduct computes a 2-element dot product:
//
//	dot(a, b) = a[0]*b[0] + a[1]*b[1]
//	a = [2.0, 3.0], b = [4.0, 5.0]
//	result = 2*4 + 3*5 = 8 + 15 = 23
func TestProgramDotProduct(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 2.0), // a[0]
		Limm(1, 3.0), // a[1]
		Limm(2, 4.0), // b[0]
		Limm(3, 5.0), // b[1]
		Fmul(4, 0, 2), // a[0]*b[0] = 8
		Ffma(5, 1, 3, 4), // a[1]*b[1] + 8 = 15 + 8 = 23
		Halt(),
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(5)
	if val != 23.0 {
		t.Errorf("expected dot product=23.0, got %g", val)
	}
}

// TestProgramMemoryRoundTrip stores and loads a value through memory.
func TestProgramMemoryRoundTrip(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 0.0),    // R0 = 0 (base address)
		Limm(1, 42.0),   // R1 = 42 (value)
		Store(0, 1, 0),   // Mem[0] = 42
		Limm(1, 0.0),    // R1 = 0 (clear register)
		Load(2, 0, 0),    // R2 = Mem[0] = 42
		Halt(),
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(2)
	if val != 42.0 {
		t.Errorf("expected R2=42.0, got %g", val)
	}
}

// TestProgramBranchLoop counts from 0 to 4 using a loop with BLT:
//
//	R0 = counter (starts at 0)
//	R1 = limit (5.0)
//	R2 = increment (1.0)
//	Loop: R0 = R0 + R2; if R0 < R1: goto Loop
func TestProgramBranchLoop(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 0.0),       // 0: R0 = counter
		Limm(1, 5.0),       // 1: R1 = limit
		Limm(2, 1.0),       // 2: R2 = increment
		Fadd(0, 0, 2),      // 3: R0 = R0 + 1 (loop body)
		Blt(0, 1, -1),      // 4: if R0 < R1, go back to PC=3 (offset -1)
		Halt(),              // 5: done
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(0)
	if val != 5.0 {
		t.Errorf("expected R0=5.0 (counted to 5), got %g", val)
	}
}

// TestProgramNegateAndAbs tests the FNEG and FABS instructions together.
func TestProgramNegateAndAbs(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 7.0),
		Fneg(1, 0),     // R1 = -7
		Fabs(2, 1),     // R2 = |−7| = 7
		Halt(),
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	negVal, _ := core.Registers.ReadFloat(1)
	if negVal != -7.0 {
		t.Errorf("expected R1=-7.0, got %g", negVal)
	}

	absVal, _ := core.Registers.ReadFloat(2)
	if absVal != 7.0 {
		t.Errorf("expected R2=7.0, got %g", absVal)
	}
}

// TestProgramBeqSkip tests BEQ branch-over pattern.
func TestProgramBeqSkip(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 1.0),       // 0: R0 = 1
		Limm(1, 1.0),       // 1: R1 = 1
		Beq(0, 1, 2),       // 2: if R0 == R1, skip 2 instructions
		Limm(5, 99.0),      // 3: R5 = 99 (should be skipped)
		Limm(6, 99.0),      // 4: R6 = 99 (this is where BEQ lands: PC=2+2=4)
		Halt(),              // 5
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	// R5 should NOT have been set to 99 (it was skipped by BEQ)
	r5, _ := core.Registers.ReadFloat(5)
	if r5 == 99.0 {
		t.Error("R5 should not be 99 (instruction should have been skipped by BEQ)")
	}
}

// TestProgramBne tests BNE branch.
func TestProgramBne(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 1.0),       // 0: R0 = 1
		Limm(1, 2.0),       // 1: R1 = 2
		Bne(0, 1, 2),       // 2: if R0 != R1, skip 2
		Limm(5, 99.0),      // 3: should be skipped
		Halt(),              // 4: BNE lands here (PC=2+2=4)
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	r5, _ := core.Registers.ReadFloat(5)
	if r5 == 99.0 {
		t.Error("R5 should not be 99 (skipped by BNE)")
	}
}

// TestProgramMovAndSub tests MOV and FSUB together.
func TestProgramMovAndSub(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 10.0),
		Mov(1, 0),          // R1 = R0 = 10
		Limm(2, 3.0),
		Fsub(3, 1, 2),      // R3 = 10 - 3 = 7
		Halt(),
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(3)
	if val != 7.0 {
		t.Errorf("expected R3=7.0, got %g", val)
	}
}

// TestProgramStoreAndLoadWithOffset tests memory operations with offsets.
func TestProgramStoreAndLoadWithOffset(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 0.0),       // base address = 0
		Limm(1, 100.0),     // value = 100
		Store(0, 1, 8.0),   // Mem[0+8] = 100
		Load(2, 0, 8.0),    // R2 = Mem[0+8] = 100
		Halt(),
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(2)
	if val != 100.0 {
		t.Errorf("expected R2=100.0, got %g", val)
	}
}

// TestProgramJmpAbsolute tests an absolute jump.
func TestProgramJmpAbsolute(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Jmp(2),              // 0: jump to PC=2
		Limm(0, 99.0),      // 1: skipped!
		Halt(),              // 2: executed
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(0)
	if val == 99.0 {
		t.Error("R0 should not be 99 (instruction was skipped)")
	}
}

// TestProgramNop tests that NOP advances PC without side effects.
func TestProgramNop(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 1.0),
		Nop(),
		Nop(),
		Limm(1, 2.0),
		Halt(),
	})

	traces, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}
	if len(traces) != 5 {
		t.Errorf("expected 5 traces (including 2 NOPs), got %d", len(traces))
	}
}

// TestProgramTraceOutput verifies that traces contain meaningful data.
func TestProgramTraceOutput(t *testing.T) {
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

	// Print all traces for educational value
	for _, trace := range traces {
		t.Log(trace.Format())
	}

	// Verify cycle numbers are sequential
	for i, trace := range traces {
		if trace.Cycle != i+1 {
			t.Errorf("trace %d: expected cycle=%d, got %d", i, i+1, trace.Cycle)
		}
	}
}

// TestProgramResetAndRerun verifies that reset allows the same program
// to be run again with different initial register values.
func TestProgramResetAndRerun(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Fmul(2, 0, 1),
		Halt(),
	})

	// First run with R0=3, R1=4
	_ = core.Registers.WriteFloat(0, 3.0)
	_ = core.Registers.WriteFloat(1, 4.0)
	core.Run(100)
	val, _ := core.Registers.ReadFloat(2)
	if val != 12.0 {
		t.Errorf("first run: expected R2=12.0, got %g", val)
	}

	// Reset and run with R0=5, R1=6
	core.Reset()
	core.LoadProgram([]Instruction{
		Fmul(2, 0, 1),
		Halt(),
	})
	_ = core.Registers.WriteFloat(0, 5.0)
	_ = core.Registers.WriteFloat(1, 6.0)
	core.Run(100)
	val, _ = core.Registers.ReadFloat(2)
	if val != 30.0 {
		t.Errorf("second run: expected R2=30.0, got %g", val)
	}
}

// TestProgramQuadraticFormula computes b^2 - 4ac:
//
//	For a=1, b=5, c=6: discriminant = 25 - 24 = 1
func TestProgramQuadraticFormula(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 1.0),       // a = 1
		Limm(1, 5.0),       // b = 5
		Limm(2, 6.0),       // c = 6
		Limm(3, 4.0),       // constant 4
		Fmul(4, 1, 1),      // R4 = b^2 = 25
		Fmul(5, 3, 0),      // R5 = 4*a = 4
		Fmul(6, 5, 2),      // R6 = 4*a*c = 24
		Fsub(7, 4, 6),      // R7 = b^2 - 4ac = 1
		Halt(),
	})

	_, err := core.Run(100)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	val, _ := core.Registers.ReadFloat(7)
	if math.Abs(val-1.0) > 0.001 {
		t.Errorf("expected discriminant=1.0, got %g", val)
	}
}

// TestProgramAllOpcodes exercises all 16 opcodes in a single program.
func TestProgramAllOpcodes(t *testing.T) {
	core := NewGPUCore()
	core.LoadProgram([]Instruction{
		Limm(0, 2.0),       //  0: LIMM
		Limm(1, 3.0),       //  1: LIMM
		Limm(2, 1.0),       //  2: LIMM
		Fadd(3, 0, 1),      //  3: FADD  R3 = 2+3 = 5
		Fsub(4, 1, 0),      //  4: FSUB  R4 = 3-2 = 1
		Fmul(5, 0, 1),      //  5: FMUL  R5 = 2*3 = 6
		Ffma(6, 0, 1, 2),   //  6: FFMA  R6 = 2*3+1 = 7
		Fneg(7, 0),          //  7: FNEG  R7 = -2
		Fabs(8, 7),          //  8: FABS  R8 = |−2| = 2
		Mov(9, 3),           //  9: MOV   R9 = R3 = 5
		Store(10, 5, 0),     // 10: STORE Mem[0] = R5 = 6  (R10=0 as base)
		Load(11, 10, 0),     // 11: LOAD  R11 = Mem[0] = 6
		Nop(),               // 12: NOP
		Beq(0, 0, 2),       // 13: BEQ   R0==R0 -> skip to 15
		Limm(15, 999.0),    // 14: should be skipped
		Blt(0, 1, 2),       // 15: BLT   2 < 3 -> skip to 17
		Limm(15, 999.0),    // 16: should be skipped
		Bne(0, 1, 2),       // 17: BNE   2 != 3 -> skip to 19
		Limm(15, 999.0),    // 18: should be skipped
		Jmp(20),             // 19: JMP   -> goto 20
		Halt(),              // 20: HALT
	})

	_, err := core.Run(1000)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}

	// Verify results
	checks := map[int]float64{
		3: 5.0, 4: 1.0, 5: 6.0, 6: 7.0,
		7: -2.0, 8: 2.0, 9: 5.0, 11: 6.0,
	}
	for reg, want := range checks {
		got, _ := core.Registers.ReadFloat(reg)
		if math.Abs(got-want) > 0.001 {
			t.Errorf("R%d: expected %g, got %g", reg, want, got)
		}
	}

	// R15 should NOT be 999 (all branch/jump instructions should have
	// skipped the LIMM 999 instructions).
	r15, _ := core.Registers.ReadFloat(15)
	if r15 == 999.0 {
		t.Error("R15 should not be 999 (instructions should have been skipped)")
	}
}
