package gpucore

import "testing"

// =========================================================================
// Helper constructor tests
// =========================================================================
//
// Each helper function creates an Instruction with the correct opcode and
// operands. These tests verify that the helpers produce the expected values.

func TestFaddHelper(t *testing.T) {
	inst := Fadd(2, 0, 1)
	if inst.Op != OpFADD || inst.Rd != 2 || inst.Rs1 != 0 || inst.Rs2 != 1 {
		t.Errorf("Fadd(2,0,1) = %+v", inst)
	}
}

func TestFsubHelper(t *testing.T) {
	inst := Fsub(3, 1, 2)
	if inst.Op != OpFSUB || inst.Rd != 3 || inst.Rs1 != 1 || inst.Rs2 != 2 {
		t.Errorf("Fsub(3,1,2) = %+v", inst)
	}
}

func TestFmulHelper(t *testing.T) {
	inst := Fmul(4, 0, 1)
	if inst.Op != OpFMUL || inst.Rd != 4 || inst.Rs1 != 0 || inst.Rs2 != 1 {
		t.Errorf("Fmul(4,0,1) = %+v", inst)
	}
}

func TestFfmaHelper(t *testing.T) {
	inst := Ffma(5, 0, 1, 2)
	if inst.Op != OpFFMA || inst.Rd != 5 || inst.Rs1 != 0 || inst.Rs2 != 1 || inst.Rs3 != 2 {
		t.Errorf("Ffma(5,0,1,2) = %+v", inst)
	}
}

func TestFnegHelper(t *testing.T) {
	inst := Fneg(1, 0)
	if inst.Op != OpFNEG || inst.Rd != 1 || inst.Rs1 != 0 {
		t.Errorf("Fneg(1,0) = %+v", inst)
	}
}

func TestFabsHelper(t *testing.T) {
	inst := Fabs(1, 0)
	if inst.Op != OpFABS || inst.Rd != 1 || inst.Rs1 != 0 {
		t.Errorf("Fabs(1,0) = %+v", inst)
	}
}

func TestLoadHelper(t *testing.T) {
	inst := Load(0, 1, 4.0)
	if inst.Op != OpLOAD || inst.Rd != 0 || inst.Rs1 != 1 || inst.Immediate != 4.0 {
		t.Errorf("Load(0,1,4.0) = %+v", inst)
	}
}

func TestStoreHelper(t *testing.T) {
	inst := Store(1, 2, 8.0)
	if inst.Op != OpSTORE || inst.Rs1 != 1 || inst.Rs2 != 2 || inst.Immediate != 8.0 {
		t.Errorf("Store(1,2,8.0) = %+v", inst)
	}
}

func TestMovHelper(t *testing.T) {
	inst := Mov(1, 0)
	if inst.Op != OpMOV || inst.Rd != 1 || inst.Rs1 != 0 {
		t.Errorf("Mov(1,0) = %+v", inst)
	}
}

func TestLimmHelper(t *testing.T) {
	inst := Limm(0, 3.14)
	if inst.Op != OpLIMM || inst.Rd != 0 || inst.Immediate != 3.14 {
		t.Errorf("Limm(0,3.14) = %+v", inst)
	}
}

func TestBeqHelper(t *testing.T) {
	inst := Beq(0, 1, 3)
	if inst.Op != OpBEQ || inst.Rs1 != 0 || inst.Rs2 != 1 || inst.Immediate != 3.0 {
		t.Errorf("Beq(0,1,3) = %+v", inst)
	}
}

func TestBltHelper(t *testing.T) {
	inst := Blt(0, 1, -2)
	if inst.Op != OpBLT || inst.Rs1 != 0 || inst.Rs2 != 1 || inst.Immediate != -2.0 {
		t.Errorf("Blt(0,1,-2) = %+v", inst)
	}
}

func TestBneHelper(t *testing.T) {
	inst := Bne(0, 1, 5)
	if inst.Op != OpBNE || inst.Rs1 != 0 || inst.Rs2 != 1 || inst.Immediate != 5.0 {
		t.Errorf("Bne(0,1,5) = %+v", inst)
	}
}

func TestJmpHelper(t *testing.T) {
	inst := Jmp(10)
	if inst.Op != OpJMP || inst.Immediate != 10.0 {
		t.Errorf("Jmp(10) = %+v", inst)
	}
}

func TestNopHelper(t *testing.T) {
	inst := Nop()
	if inst.Op != OpNOP {
		t.Errorf("Nop() = %+v", inst)
	}
}

func TestHaltHelper(t *testing.T) {
	inst := Halt()
	if inst.Op != OpHALT {
		t.Errorf("Halt() = %+v", inst)
	}
}
