package starlarkcompiler

import "testing"

func TestOpValues(t *testing.T) {
	tests := map[Op]byte{
		LoadConst:     0x01,
		Add:           0x20,
		CmpEq:         0x30,
		Jump:          0x40,
		MakeFunction:  0x50,
		BuildList:     0x60,
		LoadSubscript: 0x70,
		GetIter:       0x80,
		LoadModule:    0x90,
		Print:         0xA0,
		Halt:          0xFF,
	}

	for op, want := range tests {
		if got := op.Byte(); got != want {
			t.Fatalf("%s byte = %#x, want %#x", op, got, want)
		}
	}
}

func TestOpFromByte(t *testing.T) {
	for _, op := range AllOps() {
		got, ok := OpFromByte(op.Byte())
		if !ok {
			t.Fatalf("OpFromByte(%#x) not found", op.Byte())
		}
		if got != op {
			t.Fatalf("OpFromByte(%#x) = %s, want %s", op.Byte(), got, op)
		}
	}

	if _, ok := OpFromByte(0xEE); ok {
		t.Fatal("invalid opcode resolved")
	}
}

func TestCategory(t *testing.T) {
	tests := map[Op]Category{
		LoadConst:      CategoryStack,
		LoadName:       CategoryVariable,
		RShift:         CategoryArithmetic,
		Not:            CategoryComparison,
		JumpIfTrue:     CategoryControlFlow,
		Return:         CategoryFunction,
		DictSet:        CategoryCollection,
		LoadSlice:      CategorySubscriptAttribute,
		UnpackSequence: CategoryIteration,
		ImportFrom:     CategoryModule,
		Print:          CategoryIO,
		Halt:           CategoryVMControl,
	}

	for op, want := range tests {
		got, ok := op.Category()
		if !ok {
			t.Fatalf("%s category not found", op)
		}
		if got != want {
			t.Fatalf("%s category = %s, want %s", op, got, want)
		}
	}

	if _, ok := Op(0xB0).Category(); ok {
		t.Fatal("unknown category resolved")
	}
	if _, ok := Op(0x07).Category(); ok {
		t.Fatal("undefined stack-range opcode category resolved")
	}
}

func TestOperatorMaps(t *testing.T) {
	binary := BinaryOpMap()
	if binary["+"] != Add || binary["//"] != FloorDiv || binary[">>"] != RShift {
		t.Fatalf("binary map mismatch: %#v", binary)
	}
	if len(binary) != 12 {
		t.Fatalf("binary map length = %d, want 12", len(binary))
	}
	if op, ok := BinaryOpcode("**"); !ok || op != Power {
		t.Fatalf("BinaryOpcode(**) = %s, %t", op, ok)
	}
	if _, ok := BinaryOpcode("???"); ok {
		t.Fatal("unknown binary operator resolved")
	}

	compare := CompareOpMap()
	if compare["=="] != CmpEq || compare["not in"] != CmpNotIn {
		t.Fatalf("compare map mismatch: %#v", compare)
	}
	if len(compare) != 8 {
		t.Fatalf("compare map length = %d, want 8", len(compare))
	}
	if op, ok := CompareOpcode("in"); !ok || op != CmpIn {
		t.Fatalf("CompareOpcode(in) = %s, %t", op, ok)
	}
	if _, ok := CompareOpcode("contains"); ok {
		t.Fatal("unknown compare operator resolved")
	}

	augmented := AugmentedAssignMap()
	if augmented["+="] != Add || augmented["**="] != Power {
		t.Fatalf("augmented map mismatch: %#v", augmented)
	}
	if len(augmented) != 12 {
		t.Fatalf("augmented map length = %d, want 12", len(augmented))
	}
	if op, ok := AugmentedAssignOpcode("<<="); !ok || op != LShift {
		t.Fatalf("AugmentedAssignOpcode(<<=) = %s, %t", op, ok)
	}
	if _, ok := AugmentedAssignOpcode("="); ok {
		t.Fatal("unknown augmented operator resolved")
	}

	unary := UnaryOpMap()
	if unary["-"] != Negate || unary["~"] != BitNot {
		t.Fatalf("unary map mismatch: %#v", unary)
	}
	if len(unary) != 2 {
		t.Fatalf("unary map length = %d, want 2", len(unary))
	}
	if op, ok := UnaryOpcode("~"); !ok || op != BitNot {
		t.Fatalf("UnaryOpcode(~) = %s, %t", op, ok)
	}
	if _, ok := UnaryOpcode("not"); ok {
		t.Fatal("unknown unary operator resolved")
	}
}

func TestMapsAndAllOpsReturnCopies(t *testing.T) {
	ops := AllOps()
	ops[0] = Halt
	if AllOps()[0] != LoadConst {
		t.Fatal("AllOps returned shared storage")
	}

	binary := BinaryOpMap()
	binary["custom"] = Halt
	if _, ok := BinaryOpMap()["custom"]; ok {
		t.Fatal("BinaryOpMap returned shared storage")
	}
}

func TestStringUnknown(t *testing.T) {
	if got := Op(0xB0).String(); got != "Op(unknown)" {
		t.Fatalf("unknown string = %q", got)
	}
}
