package intel4004irvalidator

import (
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

func TestValidatorRejectsDeepCallGraph(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}}, ID: -1},
		{Opcode: ir.OpCall, Operands: []ir.IrOperand{ir.IrLabel{Name: "a"}}, ID: 1},
		{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "a"}}, ID: -1},
		{Opcode: ir.OpCall, Operands: []ir.IrOperand{ir.IrLabel{Name: "b"}}, ID: 2},
		{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "b"}}, ID: -1},
		{Opcode: ir.OpCall, Operands: []ir.IrOperand{ir.IrLabel{Name: "c"}}, ID: 3},
		{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "c"}}, ID: -1},
	}
	if len(IrValidator{}.Validate(program)) == 0 {
		t.Fatal("expected validation errors")
	}
}
