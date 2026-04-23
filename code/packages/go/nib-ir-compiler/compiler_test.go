package nibircompiler

import (
	"strings"
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
)

func TestCompileSourceProducesIR(t *testing.T) {
	typed := nibtypechecker.CheckSource("fn main() { let x: u4 = 5; }")
	if !typed.OK {
		t.Fatalf("type check failed: %#v", typed.Errors)
	}
	result := CompileNib(typed.TypedAST, ReleaseConfig())
	if result.Program == nil || len(result.Program.Instructions) == 0 {
		t.Fatal("expected instructions")
	}
}

func TestParseLiteralRejectsOverflow(t *testing.T) {
	if parsed, ok := parseLiteral("70000", "INT_LIT"); ok {
		t.Fatalf("expected overflow to be rejected, got %d", parsed)
	}
}

func TestCallSafeConfigCopiesParametersPastLiveArgumentRegisters(t *testing.T) {
	typed := nibtypechecker.CheckSource(strings.Join([]string{
		"fn sum7(a: u4, b: u4, c: u4, d: u4, e: u4, f: u4, g: u4) -> u4 {",
		"  return a;",
		"}",
	}, " "))
	if !typed.OK {
		t.Fatalf("type check failed: %#v", typed.Errors)
	}

	result := CompileNib(typed.TypedAST, CallSafeConfig())
	instructions := result.Program.Instructions
	labelIndex := -1
	for index, instruction := range instructions {
		if instruction.Opcode != ir.OpLabel {
			continue
		}
		label, ok := instruction.Operands[0].(ir.IrLabel)
		if ok && label.Name == "_fn_sum7" {
			labelIndex = index
			break
		}
	}
	if labelIndex < 0 {
		t.Fatal("expected _fn_sum7 label in IR")
	}

	expectedCopies := [][2]int{
		{9, 2},
		{10, 3},
		{11, 4},
		{12, 5},
		{13, 6},
		{14, 7},
		{15, 8},
	}
	for index, want := range expectedCopies {
		instruction := instructions[labelIndex+1+index]
		if instruction.Opcode != ir.OpAddImm {
			t.Fatalf("expected parameter copy %d to be ADD_IMM, got %s", index, instruction.Opcode)
		}
		dst, ok := instruction.Operands[0].(ir.IrRegister)
		if !ok || dst.Index != want[0] {
			t.Fatalf("expected parameter copy %d to target v%d, got %#v", index, want[0], instruction.Operands[0])
		}
		src, ok := instruction.Operands[1].(ir.IrRegister)
		if !ok || src.Index != want[1] {
			t.Fatalf("expected parameter copy %d to read v%d, got %#v", index, want[1], instruction.Operands[1])
		}
	}
}
