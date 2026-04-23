package irtointel4004compiler

import (
	"strings"
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

func TestCompileProducesAssembly(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1},
		{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 2}, ir.IrImmediate{Value: 5}}, ID: 1},
		{Opcode: ir.OpHalt, Operands: nil, ID: 2},
	}
	assembly, err := NewIrToIntel4004Compiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if !strings.Contains(assembly, "HLT") {
		t.Fatalf("expected HLT in assembly, got %s", assembly)
	}
}
