package gpucore

import "testing"

// =========================================================================
// Opcode tests
// =========================================================================

// TestOpcodeString verifies that all 16 opcodes have correct string names.
func TestOpcodeString(t *testing.T) {
	tests := []struct {
		op   Opcode
		want string
	}{
		{OpFADD, "FADD"}, {OpFSUB, "FSUB"}, {OpFMUL, "FMUL"},
		{OpFFMA, "FFMA"}, {OpFNEG, "FNEG"}, {OpFABS, "FABS"},
		{OpLOAD, "LOAD"}, {OpSTORE, "STORE"},
		{OpMOV, "MOV"}, {OpLIMM, "LIMM"},
		{OpBEQ, "BEQ"}, {OpBLT, "BLT"}, {OpBNE, "BNE"},
		{OpJMP, "JMP"}, {OpNOP, "NOP"}, {OpHALT, "HALT"},
	}
	for _, tt := range tests {
		if got := tt.op.String(); got != tt.want {
			t.Errorf("Opcode(%d).String() = %q, want %q", int(tt.op), got, tt.want)
		}
	}
}

// TestOpcodeStringUnknown verifies that an unknown opcode shows a numeric
// representation (defensive programming).
func TestOpcodeStringUnknown(t *testing.T) {
	unknown := Opcode(999)
	got := unknown.String()
	if got == "" {
		t.Error("unknown opcode should produce a non-empty string")
	}
	t.Logf("Unknown opcode string: %s", got)
}

// TestOpcodeCount verifies we have exactly 16 opcodes (from FADD=0 to HALT=15).
func TestOpcodeCount(t *testing.T) {
	if int(OpHALT) != 15 {
		t.Errorf("expected OpHALT=15 (16 opcodes total), got %d", int(OpHALT))
	}
}

// =========================================================================
// Instruction String tests
// =========================================================================

// TestInstructionStringArithmetic verifies pretty-printing for arithmetic
// instructions.
func TestInstructionStringArithmetic(t *testing.T) {
	tests := []struct {
		inst Instruction
		want string
	}{
		{Fadd(2, 0, 1), "FADD R2, R0, R1"},
		{Fsub(3, 1, 2), "FSUB R3, R1, R2"},
		{Fmul(4, 0, 1), "FMUL R4, R0, R1"},
		{Ffma(5, 0, 1, 2), "FFMA R5, R0, R1, R2"},
		{Fneg(1, 0), "FNEG R1, R0"},
		{Fabs(1, 0), "FABS R1, R0"},
	}
	for _, tt := range tests {
		if got := tt.inst.String(); got != tt.want {
			t.Errorf("String() = %q, want %q", got, tt.want)
		}
	}
}

// TestInstructionStringMemory verifies pretty-printing for memory instructions.
func TestInstructionStringMemory(t *testing.T) {
	load := Load(0, 1, 4.0)
	if got := load.String(); got != "LOAD R0, [R1+4]" {
		t.Errorf("Load string = %q, want %q", got, "LOAD R0, [R1+4]")
	}

	store := Store(1, 2, 8.0)
	if got := store.String(); got != "STORE [R1+8], R2" {
		t.Errorf("Store string = %q, want %q", got, "STORE [R1+8], R2")
	}
}

// TestInstructionStringDataMovement verifies pretty-printing for data
// movement instructions.
func TestInstructionStringDataMovement(t *testing.T) {
	m := Mov(1, 0)
	if got := m.String(); got != "MOV R1, R0" {
		t.Errorf("Mov string = %q, want %q", got, "MOV R1, R0")
	}

	l := Limm(0, 3.14)
	if got := l.String(); got != "LIMM R0, 3.14" {
		t.Errorf("Limm string = %q, want %q", got, "LIMM R0, 3.14")
	}
}

// TestInstructionStringControlFlow verifies pretty-printing for control flow
// instructions.
func TestInstructionStringControlFlow(t *testing.T) {
	beq := Beq(0, 1, 3)
	if got := beq.String(); got != "BEQ R0, R1, +3" {
		t.Errorf("Beq string = %q, want %q", got, "BEQ R0, R1, +3")
	}

	blt := Blt(0, 1, -2)
	if got := blt.String(); got != "BLT R0, R1, -2" {
		t.Errorf("Blt string = %q, want %q", got, "BLT R0, R1, -2")
	}

	bne := Bne(0, 1, 5)
	if got := bne.String(); got != "BNE R0, R1, +5" {
		t.Errorf("Bne string = %q, want %q", got, "BNE R0, R1, +5")
	}

	jmp := Jmp(10)
	if got := jmp.String(); got != "JMP 10" {
		t.Errorf("Jmp string = %q, want %q", got, "JMP 10")
	}

	nop := Nop()
	if got := nop.String(); got != "NOP" {
		t.Errorf("Nop string = %q, want %q", got, "NOP")
	}

	halt := Halt()
	if got := halt.String(); got != "HALT" {
		t.Errorf("Halt string = %q, want %q", got, "HALT")
	}
}

// TestInstructionStringDefaultCase covers the default branch in String().
func TestInstructionStringDefaultCase(t *testing.T) {
	inst := Instruction{Op: Opcode(999), Rd: 1, Rs1: 2, Rs2: 3}
	got := inst.String()
	if got == "" {
		t.Error("default case should produce non-empty string")
	}
	t.Logf("Default case string: %s", got)
}
