package arm1simulator

import (
	"encoding/binary"
	"testing"
)

// =========================================================================
// Helper: load a program from uint32 instruction words
// =========================================================================

func loadProgram(cpu *ARM1, instructions []uint32) {
	code := make([]byte, len(instructions)*4)
	for i, inst := range instructions {
		binary.LittleEndian.PutUint32(code[i*4:], inst)
	}
	cpu.LoadProgram(code, 0)
}

// =========================================================================
// Types and Constants
// =========================================================================

func TestModeString(t *testing.T) {
	tests := []struct {
		mode int
		want string
	}{
		{ModeUSR, "USR"}, {ModeFIQ, "FIQ"}, {ModeIRQ, "IRQ"}, {ModeSVC, "SVC"},
		{99, "???"},
	}
	for _, tt := range tests {
		if got := ModeString(tt.mode); got != tt.want {
			t.Errorf("ModeString(%d) = %q, want %q", tt.mode, got, tt.want)
		}
	}
}

func TestOpString(t *testing.T) {
	if got := OpString(OpADD); got != "ADD" {
		t.Errorf("OpString(ADD) = %q, want ADD", got)
	}
	if got := OpString(OpMOV); got != "MOV" {
		t.Errorf("OpString(MOV) = %q, want MOV", got)
	}
	if got := OpString(99); got != "???" {
		t.Errorf("OpString(99) = %q, want ???", got)
	}
}

func TestIsTestOp(t *testing.T) {
	if !IsTestOp(OpTST) {
		t.Error("TST should be a test op")
	}
	if !IsTestOp(OpCMP) {
		t.Error("CMP should be a test op")
	}
	if IsTestOp(OpADD) {
		t.Error("ADD should not be a test op")
	}
}

func TestIsLogicalOp(t *testing.T) {
	if !IsLogicalOp(OpAND) {
		t.Error("AND should be logical")
	}
	if !IsLogicalOp(OpMOV) {
		t.Error("MOV should be logical")
	}
	if IsLogicalOp(OpADD) {
		t.Error("ADD should not be logical")
	}
}

// =========================================================================
// Condition Evaluator
// =========================================================================

func TestEvaluateCondition(t *testing.T) {
	tests := []struct {
		name  string
		cond  int
		flags Flags
		want  bool
	}{
		{"EQ when Z set", CondEQ, Flags{Z: true}, true},
		{"EQ when Z clear", CondEQ, Flags{}, false},
		{"NE when Z clear", CondNE, Flags{}, true},
		{"NE when Z set", CondNE, Flags{Z: true}, false},
		{"CS when C set", CondCS, Flags{C: true}, true},
		{"CC when C clear", CondCC, Flags{}, true},
		{"MI when N set", CondMI, Flags{N: true}, true},
		{"PL when N clear", CondPL, Flags{}, true},
		{"VS when V set", CondVS, Flags{V: true}, true},
		{"VC when V clear", CondVC, Flags{}, true},
		{"HI when C=1,Z=0", CondHI, Flags{C: true}, true},
		{"HI when C=1,Z=1", CondHI, Flags{C: true, Z: true}, false},
		{"LS when C=0", CondLS, Flags{}, true},
		{"LS when Z=1", CondLS, Flags{C: true, Z: true}, true},
		{"GE when N=V=0", CondGE, Flags{}, true},
		{"GE when N=V=1", CondGE, Flags{N: true, V: true}, true},
		{"GE when N!=V", CondGE, Flags{N: true}, false},
		{"LT when N!=V", CondLT, Flags{N: true}, true},
		{"LT when N=V", CondLT, Flags{}, false},
		{"GT when Z=0,N=V", CondGT, Flags{}, true},
		{"GT when Z=1", CondGT, Flags{Z: true}, false},
		{"LE when Z=1", CondLE, Flags{Z: true}, true},
		{"LE when N!=V", CondLE, Flags{N: true}, true},
		{"AL always", CondAL, Flags{}, true},
		{"NV never", CondNV, Flags{}, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := EvaluateCondition(tt.cond, tt.flags); got != tt.want {
				t.Errorf("EvaluateCondition(%d, %+v) = %v, want %v", tt.cond, tt.flags, got, tt.want)
			}
		})
	}
}

// =========================================================================
// Barrel Shifter
// =========================================================================

func TestBarrelShiftLSL(t *testing.T) {
	tests := []struct {
		name     string
		value    uint32
		amount   int
		wantVal  uint32
		wantC    bool
	}{
		{"LSL #0 (no shift)", 0xFF, 0, 0xFF, false},
		{"LSL #1", 0xFF, 1, 0x1FE, false},
		{"LSL #4", 0xFF, 4, 0xFF0, false},
		{"LSL #31", 1, 31, 0x80000000, false},
		{"LSL #32", 1, 32, 0, true},
		{"LSL #33", 1, 33, 0, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			val, c := BarrelShift(tt.value, ShiftLSL, tt.amount, false, false)
			if val != tt.wantVal {
				t.Errorf("value = 0x%X, want 0x%X", val, tt.wantVal)
			}
			if c != tt.wantC {
				t.Errorf("carry = %v, want %v", c, tt.wantC)
			}
		})
	}
}

func TestBarrelShiftLSR(t *testing.T) {
	tests := []struct {
		name    string
		value   uint32
		amount  int
		byReg   bool
		wantVal uint32
		wantC   bool
	}{
		{"LSR #1", 0xFF, 1, false, 0x7F, true},
		{"LSR #8", 0xFF00, 8, false, 0xFF, false},
		{"LSR #0 (encodes #32)", 0x80000000, 0, false, 0, true},
		{"LSR #32 by register", 0x80000000, 32, true, 0, true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			val, c := BarrelShift(tt.value, ShiftLSR, tt.amount, false, tt.byReg)
			if val != tt.wantVal {
				t.Errorf("value = 0x%X, want 0x%X", val, tt.wantVal)
			}
			if c != tt.wantC {
				t.Errorf("carry = %v, want %v", c, tt.wantC)
			}
		})
	}
}

func TestBarrelShiftASR(t *testing.T) {
	tests := []struct {
		name    string
		value   uint32
		amount  int
		wantVal uint32
		wantC   bool
	}{
		{"ASR #1 positive", 0x7FFFFFFE, 1, 0x3FFFFFFF, false},
		{"ASR #1 negative", 0x80000000, 1, 0xC0000000, false},
		{"ASR #0 (encodes #32) negative", 0x80000000, 0, 0xFFFFFFFF, true},
		{"ASR #0 (encodes #32) positive", 0x7FFFFFFF, 0, 0, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			val, c := BarrelShift(tt.value, ShiftASR, tt.amount, false, false)
			if val != tt.wantVal {
				t.Errorf("value = 0x%X, want 0x%X", val, tt.wantVal)
			}
			if c != tt.wantC {
				t.Errorf("carry = %v, want %v", c, tt.wantC)
			}
		})
	}
}

func TestBarrelShiftROR(t *testing.T) {
	tests := []struct {
		name    string
		value   uint32
		amount  int
		wantVal uint32
		wantC   bool
	}{
		{"ROR #4", 0x0000000F, 4, 0xF0000000, true},
		{"ROR #8", 0x000000FF, 8, 0xFF000000, true},
		{"ROR #16", 0x0000FFFF, 16, 0xFFFF0000, true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			val, c := BarrelShift(tt.value, ShiftROR, tt.amount, false, false)
			if val != tt.wantVal {
				t.Errorf("value = 0x%X, want 0x%X", val, tt.wantVal)
			}
			if c != tt.wantC {
				t.Errorf("carry = %v, want %v", c, tt.wantC)
			}
		})
	}
}

func TestBarrelShiftRRX(t *testing.T) {
	// RRX: ROR #0 with immediate encoding = 33-bit rotate through carry
	val, c := BarrelShift(0x00000001, ShiftROR, 0, true, false)
	if val != 0x80000000 {
		t.Errorf("RRX value = 0x%X, want 0x80000000", val)
	}
	if !c {
		t.Error("RRX carry should be true (old bit 0 was 1)")
	}

	val, c = BarrelShift(0x00000000, ShiftROR, 0, true, false)
	if val != 0x80000000 {
		t.Errorf("RRX value = 0x%X, want 0x80000000", val)
	}
	if c {
		t.Error("RRX carry should be false (old bit 0 was 0)")
	}
}

func TestDecodeImmediate(t *testing.T) {
	tests := []struct {
		imm8   uint32
		rotate uint32
		want   uint32
	}{
		{0xFF, 0, 0xFF},
		{0x01, 1, 0x40000000}, // 1 ROR 2 = 0x40000000
		{0xFF, 4, 0xFF000000}, // 0xFF ROR 8
	}
	for _, tt := range tests {
		val, _ := DecodeImmediate(tt.imm8, tt.rotate)
		if val != tt.want {
			t.Errorf("DecodeImmediate(%X, %d) = 0x%X, want 0x%X", tt.imm8, tt.rotate, val, tt.want)
		}
	}
}

// =========================================================================
// ALU
// =========================================================================

func TestALUAdd(t *testing.T) {
	r := ALUExecute(OpADD, 1, 2, false, false, false)
	if r.Result != 3 {
		t.Errorf("1 + 2 = %d, want 3", r.Result)
	}
	if r.N || r.Z || r.C || r.V {
		t.Errorf("flags should be clear for 1+2, got N=%v Z=%v C=%v V=%v", r.N, r.Z, r.C, r.V)
	}
}

func TestALUAddOverflow(t *testing.T) {
	// 0x7FFFFFFF + 1 = 0x80000000 (signed overflow)
	r := ALUExecute(OpADD, 0x7FFFFFFF, 1, false, false, false)
	if r.Result != 0x80000000 {
		t.Errorf("result = 0x%X, want 0x80000000", r.Result)
	}
	if !r.N {
		t.Error("N should be set (negative result)")
	}
	if !r.V {
		t.Error("V should be set (signed overflow)")
	}
}

func TestALUAddCarry(t *testing.T) {
	// 0xFFFFFFFF + 1 = 0 with carry
	r := ALUExecute(OpADD, 0xFFFFFFFF, 1, false, false, false)
	if r.Result != 0 {
		t.Errorf("result = 0x%X, want 0", r.Result)
	}
	if !r.C {
		t.Error("C should be set (unsigned overflow)")
	}
	if !r.Z {
		t.Error("Z should be set (result is zero)")
	}
}

func TestALUSub(t *testing.T) {
	r := ALUExecute(OpSUB, 5, 3, false, false, false)
	if r.Result != 2 {
		t.Errorf("5 - 3 = %d, want 2", r.Result)
	}
	if !r.C {
		t.Error("C should be set (no borrow in ARM subtraction)")
	}
}

func TestALUSubBorrow(t *testing.T) {
	// 3 - 5 = -2 (borrow occurs, carry cleared)
	r := ALUExecute(OpSUB, 3, 5, false, false, false)
	if r.Result != 0xFFFFFFFE {
		t.Errorf("result = 0x%X, want 0xFFFFFFFE", r.Result)
	}
	if r.C {
		t.Error("C should be clear (borrow occurred)")
	}
	if !r.N {
		t.Error("N should be set (negative result)")
	}
}

func TestALURSB(t *testing.T) {
	// RSB: Op2 - Rn = 5 - 3 = 2
	r := ALUExecute(OpRSB, 3, 5, false, false, false)
	if r.Result != 2 {
		t.Errorf("RSB 3,5 = %d, want 2", r.Result)
	}
}

func TestALUADC(t *testing.T) {
	// ADC: Rn + Op2 + Carry = 1 + 2 + 1 = 4
	r := ALUExecute(OpADC, 1, 2, true, false, false)
	if r.Result != 4 {
		t.Errorf("1 + 2 + 1 = %d, want 4", r.Result)
	}
}

func TestALUSBC(t *testing.T) {
	// SBC: Rn - Op2 - NOT(C) = 5 - 3 - 0 = 2 (when C=1)
	r := ALUExecute(OpSBC, 5, 3, true, false, false)
	if r.Result != 2 {
		t.Errorf("SBC 5,3,C=1 = %d, want 2", r.Result)
	}
}

func TestALULogical(t *testing.T) {
	tests := []struct {
		op   int
		a, b uint32
		want uint32
	}{
		{OpAND, 0xFF00FF00, 0x0FF00FF0, 0x0F000F00},
		{OpEOR, 0xFF00FF00, 0x0FF00FF0, 0xF0F0F0F0},
		{OpORR, 0xFF00FF00, 0x0FF00FF0, 0xFFF0FFF0},
		{OpBIC, 0xFFFFFFFF, 0x0000FF00, 0xFFFF00FF},
		{OpMOV, 0, 42, 42},
		{OpMVN, 0, 0, 0xFFFFFFFF},
	}
	for _, tt := range tests {
		t.Run(OpString(tt.op), func(t *testing.T) {
			r := ALUExecute(tt.op, tt.a, tt.b, false, false, false)
			if r.Result != tt.want {
				t.Errorf("%s(0x%X, 0x%X) = 0x%X, want 0x%X", OpString(tt.op), tt.a, tt.b, r.Result, tt.want)
			}
		})
	}
}

func TestALUTestOps(t *testing.T) {
	// TST: sets flags, does not write result
	r := ALUExecute(OpTST, 0xFF, 0x00, false, false, false)
	if r.WriteResult {
		t.Error("TST should not write result")
	}
	if !r.Z {
		t.Error("TST 0xFF & 0x00 should set Z")
	}

	// CMP: 5 - 5 = 0
	r = ALUExecute(OpCMP, 5, 5, false, false, false)
	if r.WriteResult {
		t.Error("CMP should not write result")
	}
	if !r.Z {
		t.Error("CMP 5,5 should set Z")
	}
	if !r.C {
		t.Error("CMP 5,5 should set C (no borrow)")
	}
}

// =========================================================================
// Decoder
// =========================================================================

func TestDecodeDataProcessing(t *testing.T) {
	// ADD R2, R0, R1 — E0802001
	inst := uint32(0xE0802001)
	d := Decode(inst)

	if d.Type != InstDataProcessing {
		t.Fatalf("type = %d, want DataProcessing", d.Type)
	}
	if d.Cond != CondAL {
		t.Errorf("cond = %d, want AL", d.Cond)
	}
	if d.Opcode != OpADD {
		t.Errorf("opcode = %d, want ADD", d.Opcode)
	}
	if d.S {
		t.Error("S should be false")
	}
	if d.Rn != 0 {
		t.Errorf("Rn = %d, want 0", d.Rn)
	}
	if d.Rd != 2 {
		t.Errorf("Rd = %d, want 2", d.Rd)
	}
	if d.Rm != 1 {
		t.Errorf("Rm = %d, want 1", d.Rm)
	}
}

func TestDecodeMovImmediate(t *testing.T) {
	// MOV R0, #42 — E3A0002A
	inst := uint32(0xE3A0002A)
	d := Decode(inst)

	if d.Type != InstDataProcessing {
		t.Fatalf("type = %d, want DataProcessing", d.Type)
	}
	if d.Opcode != OpMOV {
		t.Errorf("opcode = %d, want MOV", d.Opcode)
	}
	if !d.Immediate {
		t.Error("I should be true for immediate")
	}
	if d.Rd != 0 {
		t.Errorf("Rd = %d, want 0", d.Rd)
	}
	if d.Imm8 != 42 {
		t.Errorf("Imm8 = %d, want 42", d.Imm8)
	}
}

func TestDecodeBranch(t *testing.T) {
	// B +8 — EA000002 (offset = +8 bytes = 2 words, but encoded as offset/4)
	// Actually: EA000000 + offset/4 where offset is relative to PC+8
	inst := uint32(0xEA000002)
	d := Decode(inst)

	if d.Type != InstBranch {
		t.Fatalf("type = %d, want Branch", d.Type)
	}
	if d.Link {
		t.Error("Link should be false for B")
	}
	if d.BranchOffset != 8 {
		t.Errorf("BranchOffset = %d, want 8", d.BranchOffset)
	}
}

func TestDecodeBranchLink(t *testing.T) {
	// BL -4 — EBFFFFFE (offset = -4 from PC+8 = go back 1 instruction)
	inst := uint32(0xEBFFFFFE)
	d := Decode(inst)

	if d.Type != InstBranch {
		t.Fatalf("type = %d, want Branch", d.Type)
	}
	if !d.Link {
		t.Error("Link should be true for BL")
	}
	if d.BranchOffset != -8 {
		t.Errorf("BranchOffset = %d, want -8", d.BranchOffset)
	}
}

func TestDecodeSWI(t *testing.T) {
	inst := uint32(0xEF123456)
	d := Decode(inst)

	if d.Type != InstSWI {
		t.Fatalf("type = %d, want SWI", d.Type)
	}
	if d.SWIComment != 0x123456 {
		t.Errorf("SWIComment = 0x%X, want 0x123456", d.SWIComment)
	}
}

func TestDisassemble(t *testing.T) {
	tests := []struct {
		inst uint32
		want string
	}{
		{0xE3A0002A, "MOV R0, #42"},
		{0xE0802001, "ADD R2, R0, R1"},
		{0xE0912001, "ADDS R2, R1, R1"},
		{0x10802001, "ADDNE R2, R0, R1"},
		{0xEF123456, "HLT"},
	}
	for _, tt := range tests {
		d := Decode(tt.inst)
		got := d.Disassemble()
		if got != tt.want {
			t.Errorf("Disassemble(0x%08X) = %q, want %q", tt.inst, got, tt.want)
		}
	}
}

// =========================================================================
// CPU — Power-on state
// =========================================================================

func TestNewCPU(t *testing.T) {
	cpu := New(1024)

	// Should start in SVC mode with IRQ/FIQ disabled
	if cpu.Mode() != ModeSVC {
		t.Errorf("mode = %d, want SVC", cpu.Mode())
	}
	if cpu.PC() != 0 {
		t.Errorf("PC = 0x%X, want 0", cpu.PC())
	}
	flags := cpu.Flags()
	if flags.N || flags.Z || flags.C || flags.V {
		t.Error("flags should be clear on reset")
	}
}

// =========================================================================
// CPU — Basic programs
// =========================================================================

func TestMOVImmediate(t *testing.T) {
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 42), // MOV R0, #42
		EncodeHalt(),
	})

	cpu.Run(10)

	if cpu.ReadRegister(0) != 42 {
		t.Errorf("R0 = %d, want 42", cpu.ReadRegister(0))
	}
}

func TestOnePlusTwo(t *testing.T) {
	// x = 1 + 2 — the classic first test
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 1),            // MOV R0, #1
		EncodeMovImm(CondAL, 1, 2),            // MOV R1, #2
		EncodeALUReg(CondAL, OpADD, 0, 2, 0, 1), // ADD R2, R0, R1
		EncodeHalt(),
	})

	cpu.Run(10)

	if cpu.ReadRegister(0) != 1 {
		t.Errorf("R0 = %d, want 1", cpu.ReadRegister(0))
	}
	if cpu.ReadRegister(1) != 2 {
		t.Errorf("R1 = %d, want 2", cpu.ReadRegister(1))
	}
	if cpu.ReadRegister(2) != 3 {
		t.Errorf("R2 = %d, want 3", cpu.ReadRegister(2))
	}
}

func TestSUBSWithFlags(t *testing.T) {
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 5),                // MOV R0, #5
		EncodeMovImm(CondAL, 1, 5),                // MOV R1, #5
		EncodeALUReg(CondAL, OpSUB, 1, 2, 0, 1),  // SUBS R2, R0, R1
		EncodeHalt(),
	})

	cpu.Run(10)

	if cpu.ReadRegister(2) != 0 {
		t.Errorf("R2 = %d, want 0", cpu.ReadRegister(2))
	}
	flags := cpu.Flags()
	if !flags.Z {
		t.Error("Z should be set (5 - 5 = 0)")
	}
	if !flags.C {
		t.Error("C should be set (no borrow)")
	}
}

func TestConditionalExecution(t *testing.T) {
	// Test that conditional execution works:
	// Set R0=5, R1=5, SUBS to set Z, then ADDNE should NOT execute,
	// but ADDEQ should execute.
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 5),                 // MOV R0, #5
		EncodeMovImm(CondAL, 1, 5),                 // MOV R1, #5
		EncodeALUReg(CondAL, OpSUB, 1, 2, 0, 1),   // SUBS R2, R0, R1 (sets Z)
		EncodeMovImm(CondNE, 3, 99),                 // MOVNE R3, #99 (should NOT execute)
		EncodeMovImm(CondEQ, 4, 42),                 // MOVEQ R4, #42 (should execute)
		EncodeHalt(),
	})

	cpu.Run(20)

	if cpu.ReadRegister(3) != 0 {
		t.Errorf("R3 = %d, want 0 (MOVNE should not execute when Z set)", cpu.ReadRegister(3))
	}
	if cpu.ReadRegister(4) != 42 {
		t.Errorf("R4 = %d, want 42 (MOVEQ should execute when Z set)", cpu.ReadRegister(4))
	}
}

func TestBarrelShifterInInstruction(t *testing.T) {
	// R1 = R0 * 5 using barrel shifter: ADD R1, R0, R0, LSL #2
	// This is the ARM way to multiply without a MUL instruction.
	cpu := New(1024)

	// Encode: ADD R1, R0, R0, LSL #2
	// Operand2 = R0 (Rm=0), LSL (type=00), shift amount = 2 (bits 11:7 = 00010)
	addWithShift := uint32(CondAL)<<28 | // AL condition
		0<<25 | // I=0 (register operand)
		uint32(OpADD)<<21 | // ADD
		0<<20 | // S=0
		uint32(0)<<16 | // Rn=R0
		uint32(1)<<12 | // Rd=R1
		uint32(2)<<7 | // shift amount = 2
		uint32(ShiftLSL)<<5 | // LSL
		uint32(0) // Rm=R0

	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 7),  // MOV R0, #7
		addWithShift,                 // ADD R1, R0, R0, LSL #2 = 7 + 28 = 35
		EncodeHalt(),
	})

	cpu.Run(10)

	if cpu.ReadRegister(1) != 35 {
		t.Errorf("R1 = %d, want 35 (7 * 5 = 35)", cpu.ReadRegister(1))
	}
}

func TestLoopSumOneToTen(t *testing.T) {
	// Sum 1 to 10 using a countdown loop
	// R0 = sum (starts at 0)
	// R1 = counter (starts at 10, counts down)
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 0),                 // MOV R0, #0    (sum = 0)
		EncodeMovImm(CondAL, 1, 10),                // MOV R1, #10   (counter = 10)
		// loop:
		EncodeALUReg(CondAL, OpADD, 0, 0, 0, 1),   // ADD R0, R0, R1  (sum += counter)
		EncodeDataProcessing(CondAL, OpSUB, 1, 1, 1, (1<<25)|1), // SUBS R1, R1, #1
		EncodeBranch(CondNE, false, -16),            // BNE loop (target=0x08, from PC+8=0x18, offset=-16)
		EncodeHalt(),
	})

	cpu.Run(100)

	if cpu.ReadRegister(0) != 55 {
		t.Errorf("R0 = %d, want 55 (sum of 1..10)", cpu.ReadRegister(0))
	}
	if cpu.ReadRegister(1) != 0 {
		t.Errorf("R1 = %d, want 0 (counter should reach 0)", cpu.ReadRegister(1))
	}
}

// =========================================================================
// CPU — Load/Store
// =========================================================================

func TestLDRSTR(t *testing.T) {
	cpu := New(4096)

	// Store 42 to address 0x100, then load it back
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 42),                 // MOV R0, #42
		EncodeMovImm(CondAL, 1, 0),                  // MOV R1, #0
		// We need to encode MOV R1, #256 but 256 doesn't fit in 8-bit immediate
		// 256 = 1 ROR 24 = imm8=1, rotate=12
		EncodeDataProcessing(CondAL, OpMOV, 0, 0, 1, (1<<25)|(12<<8)|1), // MOV R1, #256
		EncodeSTR(CondAL, 0, 1, 0, true),            // STR R0, [R1]
		EncodeMovImm(CondAL, 0, 0),                  // MOV R0, #0 (clear R0)
		EncodeLDR(CondAL, 0, 1, 0, true),            // LDR R0, [R1]
		EncodeHalt(),
	})

	cpu.Run(20)

	if cpu.ReadRegister(0) != 42 {
		t.Errorf("R0 = %d, want 42 (loaded from memory)", cpu.ReadRegister(0))
	}
}

func TestLDRByte(t *testing.T) {
	cpu := New(4096)

	// Write 0xDEADBEEF to address 0x100, then load individual bytes
	cpu.WriteWord(0x100, 0xDEADBEEF)

	loadProgram(cpu, []uint32{
		EncodeDataProcessing(CondAL, OpMOV, 0, 0, 1, (1<<25)|(12<<8)|1), // MOV R1, #256
		// LDRB R0, [R1, #0]
		uint32(CondAL)<<28 | 0x05D00000 | uint32(1)<<16 | uint32(0)<<12 | 0, // LDRB R0, [R1]
		EncodeHalt(),
	})

	cpu.Run(10)

	// Byte 0 of 0xDEADBEEF in little-endian is 0xEF
	if cpu.ReadRegister(0) != 0xEF {
		t.Errorf("R0 = 0x%X, want 0xEF", cpu.ReadRegister(0))
	}
}

// =========================================================================
// CPU — Block Transfer (LDM/STM)
// =========================================================================

func TestSTMLDM(t *testing.T) {
	cpu := New(4096)

	// Set up registers R0-R3 with known values
	// Use STM to store them, clear registers, then LDM to load them back
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 10),  // R0 = 10
		EncodeMovImm(CondAL, 1, 20),  // R1 = 20
		EncodeMovImm(CondAL, 2, 30),  // R2 = 30
		EncodeMovImm(CondAL, 3, 40),  // R3 = 40
		// MOV R5, #256 (base address for stack)
		EncodeDataProcessing(CondAL, OpMOV, 0, 0, 5, (1<<25)|(12<<8)|1),
		// STMIA R5!, {R0-R3}
		EncodeSTM(CondAL, 5, 0x000F, true, "IA"),
		// Clear R0-R3
		EncodeMovImm(CondAL, 0, 0),
		EncodeMovImm(CondAL, 1, 0),
		EncodeMovImm(CondAL, 2, 0),
		EncodeMovImm(CondAL, 3, 0),
		// MOV R5, #256 (reset base)
		EncodeDataProcessing(CondAL, OpMOV, 0, 0, 5, (1<<25)|(12<<8)|1),
		// LDMIA R5!, {R0-R3}
		EncodeLDM(CondAL, 5, 0x000F, true, "IA"),
		EncodeHalt(),
	})

	cpu.Run(50)

	if cpu.ReadRegister(0) != 10 {
		t.Errorf("R0 = %d, want 10", cpu.ReadRegister(0))
	}
	if cpu.ReadRegister(1) != 20 {
		t.Errorf("R1 = %d, want 20", cpu.ReadRegister(1))
	}
	if cpu.ReadRegister(2) != 30 {
		t.Errorf("R2 = %d, want 30", cpu.ReadRegister(2))
	}
	if cpu.ReadRegister(3) != 40 {
		t.Errorf("R3 = %d, want 40", cpu.ReadRegister(3))
	}
}

// =========================================================================
// CPU — Branch and Link
// =========================================================================

func TestBranchAndLink(t *testing.T) {
	// Main: MOV R0, #7; BL double; HLT
	// double: ADD R0, R0, R0; MOVS PC, LR
	cpu := New(4096)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 7),     // 0x00: MOV R0, #7
		EncodeBranch(CondAL, true, 4),   // 0x04: BL double (skip 1 instruction to 0x10)
		EncodeHalt(),                    // 0x08: HLT (return here)
		0,                               // 0x0C: padding
		// double subroutine at 0x10:
		EncodeALUReg(CondAL, OpADD, 0, 0, 0, 0), // 0x10: ADD R0, R0, R0
		// MOVS PC, LR — return from subroutine
		// This is: MOV with S=1, Rd=R15(PC), Rm=R14(LR)
		EncodeDataProcessing(CondAL, OpMOV, 1, 0, 15, uint32(14)), // 0x14: MOVS PC, LR
	})

	cpu.Run(20)

	if cpu.ReadRegister(0) != 14 {
		t.Errorf("R0 = %d, want 14 (7 * 2)", cpu.ReadRegister(0))
	}
}

// =========================================================================
// CPU — Fibonacci
// =========================================================================

func TestFibonacci(t *testing.T) {
	// Compute fib(10) = 55
	// R0 = fib(n-2), R1 = fib(n-1), R2 = counter, R3 = temp
	cpu := New(4096)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 0),                       // R0 = 0 (fib_prev)
		EncodeMovImm(CondAL, 1, 1),                       // R1 = 1 (fib_curr)
		EncodeMovImm(CondAL, 2, 10),                      // R2 = 10 (counter)
		// loop:
		EncodeALUReg(CondAL, OpADD, 0, 3, 0, 1),         // ADD R3, R0, R1
		EncodeALUReg(CondAL, OpMOV, 0, 0, 0, 1),         // MOV R0, R1
		EncodeALUReg(CondAL, OpMOV, 0, 1, 0, 3),         // MOV R1, R3
		EncodeDataProcessing(CondAL, OpSUB, 1, 2, 2, (1<<25)|1), // SUBS R2, R2, #1
		EncodeBranch(CondNE, false, -24),                  // BNE loop (target=0x0C, from PC+8=0x24, offset=-24)
		EncodeHalt(),
	})

	cpu.Run(200)

	if cpu.ReadRegister(1) != 89 {
		t.Errorf("R1 = %d, want 89 (fib(11) after 10 iterations)", cpu.ReadRegister(1))
	}
}

// =========================================================================
// CPU — Register banking
// =========================================================================

func TestRegisterBanking(t *testing.T) {
	cpu := New(4096)

	// We start in SVC mode. Write to R13 (which is R13_svc)
	cpu.WriteRegister(13, 0xAA000000)

	// Manually switch to USR mode by writing R15
	r15 := cpu.regs[15]
	r15 = (r15 & ^uint32(ModeMask)) | ModeUSR
	cpu.regs[15] = r15

	// Write to R13 in USR mode (which is the base R13)
	cpu.WriteRegister(13, 0xBB000000)

	// Now R13 in USR should be different from R13 in SVC
	usrR13 := cpu.ReadRegister(13)

	// Switch back to SVC
	r15 = cpu.regs[15]
	r15 = (r15 & ^uint32(ModeMask)) | ModeSVC
	cpu.regs[15] = r15

	svcR13 := cpu.ReadRegister(13)

	if usrR13 == svcR13 {
		t.Errorf("USR R13 (0x%X) should differ from SVC R13 (0x%X)", usrR13, svcR13)
	}
	if usrR13 != 0xBB000000 {
		t.Errorf("USR R13 = 0x%X, want 0x%X", usrR13, 0xBB000000)
	}
	if svcR13 != 0xAA000000 {
		t.Errorf("SVC R13 = 0x%X, want 0x%X", svcR13, 0xAA000000)
	}
}

// =========================================================================
// CPU — Memory operations
// =========================================================================

func TestReadWriteWord(t *testing.T) {
	cpu := New(1024)
	cpu.WriteWord(0, 0xDEADBEEF)
	got := cpu.ReadWord(0)
	if got != 0xDEADBEEF {
		t.Errorf("ReadWord = 0x%X, want 0xDEADBEEF", got)
	}
}

func TestReadWriteByte(t *testing.T) {
	cpu := New(1024)
	cpu.WriteByte(0, 0xAB)
	got := cpu.ReadByte(0)
	if got != 0xAB {
		t.Errorf("ReadByte = 0x%X, want 0xAB", got)
	}
}

func TestCPUString(t *testing.T) {
	cpu := New(1024)
	s := cpu.String()
	if s == "" {
		t.Error("String() should not be empty")
	}
}

func TestHalt(t *testing.T) {
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeHalt(),
	})

	traces := cpu.Run(100)

	if !cpu.Halted() {
		t.Error("CPU should be halted after HLT")
	}
	if len(traces) != 1 {
		t.Errorf("expected 1 trace, got %d", len(traces))
	}
}

func TestTraceFields(t *testing.T) {
	cpu := New(1024)
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 42),
		EncodeHalt(),
	})

	trace := cpu.Step()

	if trace.Address != 0 {
		t.Errorf("trace address = 0x%X, want 0", trace.Address)
	}
	if !trace.ConditionMet {
		t.Error("condition should be met (AL)")
	}
	if trace.RegsAfter[0] != 42 {
		t.Errorf("R0 after = %d, want 42", trace.RegsAfter[0])
	}
}

// =========================================================================
// CPU — SWI
// =========================================================================

func TestSWI(t *testing.T) {
	cpu := New(4096)

	// Put a handler at the SWI vector (0x08) that just sets R5=99
	// For simplicity we put the handler inline and use a branch
	cpu.WriteWord(0x08, EncodeBranch(CondAL, false, 0xF0-0x08-8)) // B to 0xF0

	// Handler at 0xF0
	cpu.WriteWord(0xF0, EncodeMovImm(CondAL, 5, 99))
	cpu.WriteWord(0xF4, EncodeHalt())

	// Main program
	loadProgram(cpu, []uint32{
		EncodeMovImm(CondAL, 0, 1),  // 0x00: MOV R0, #1
		// SWI #1 (not our halt SWI)
		uint32(CondAL)<<28 | 0x0F000001, // 0x04: SWI #1
		EncodeHalt(),                     // 0x08: won't reach (overwritten by vector)
	})

	// Write the SWI vector branch
	cpu.WriteWord(0x08, EncodeBranch(CondAL, false, 0xF0-0x08-8))

	cpu.Run(20)

	if cpu.ReadRegister(5) != 99 {
		t.Errorf("R5 = %d, want 99 (SWI handler should have run)", cpu.ReadRegister(5))
	}
	if cpu.Mode() != ModeSVC {
		t.Errorf("mode = %d, want SVC after SWI", cpu.Mode())
	}
}

// =========================================================================
// Encoding helpers
// =========================================================================

func TestEncodeMovImm(t *testing.T) {
	inst := EncodeMovImm(CondAL, 0, 42)
	d := Decode(inst)
	if d.Opcode != OpMOV || d.Rd != 0 || d.Imm8 != 42 {
		t.Errorf("EncodeMovImm decoded wrong: opcode=%d Rd=%d Imm8=%d", d.Opcode, d.Rd, d.Imm8)
	}
}

func TestEncodeHalt(t *testing.T) {
	inst := EncodeHalt()
	d := Decode(inst)
	if d.Type != InstSWI || d.SWIComment != HaltSWI {
		t.Errorf("EncodeHalt decoded wrong: type=%d SWI=0x%X", d.Type, d.SWIComment)
	}
}
