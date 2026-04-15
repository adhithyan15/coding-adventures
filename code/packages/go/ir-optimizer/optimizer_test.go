package iroptimizer

import (
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

func TestOptimizerProducesStats(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 1}}, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAddImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 0}, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 2}}, ID: 2})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpHalt, Operands: nil, ID: 3})

	result := DefaultPasses().Optimize(program)
	if result.InstructionsBefore == 0 || result.Program == nil {
		t.Fatal("expected optimization result")
	}
}
