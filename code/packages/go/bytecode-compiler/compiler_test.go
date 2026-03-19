package bytecodecompiler

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

func TestCompilerVariablesAndArithmetic(t *testing.T) {
	// Represents: x = 1 + 2
	stmt := parser.Assignment{
		Target: parser.Name{Name: "x"},
		Value: parser.BinaryOp{
			Left:  parser.NumberLiteral{Value: 1},
			Op:    "+",
			Right: parser.NumberLiteral{Value: 2},
		},
	}
	
	prog := parser.Program{
		Statements: []parser.Statement{stmt},
	}

	compiler := NewBytecodeCompiler()
	code := compiler.Compile(prog)

	expectedInstructions := []vm.Instruction{
		{Opcode: vm.OpLoadConst, Operand: 0},
		{Opcode: vm.OpLoadConst, Operand: 1},
		{Opcode: vm.OpAdd},
		{Opcode: vm.OpStoreName, Operand: 0},
		{Opcode: vm.OpHalt},
	}

	if len(code.Instructions) != len(expectedInstructions) {
		t.Fatalf("Expected %d instructions, got %d", len(expectedInstructions), len(code.Instructions))
	}

	for i, instr := range code.Instructions {
		expected := expectedInstructions[i]
		if instr.Opcode != expected.Opcode || instr.Operand != expected.Operand {
			t.Errorf("Instruction %d: expected %v, got %v", i, expected, instr)
		}
	}

	if !reflect.DeepEqual(code.Constants, []interface{}{1, 2}) {
		t.Errorf("Constants pool expected [1, 2] got %v", code.Constants)
	}

	if !reflect.DeepEqual(code.Names, []string{"x"}) {
		t.Errorf("Names pool expected ['x'] got %v", code.Names)
	}
}
