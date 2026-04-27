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

func TestConstantFolderFoldsStraightLineSelfIncrement(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 1}}, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAddImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 0}, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 2}}, ID: 2})

	result := ConstantFolder{}.Run(program)
	if got := result.Instructions[1].Opcode; got != ir.OpLoadImm {
		t.Fatalf("expected straight-line addi to fold into LOAD_IMM, got %s", got.String())
	}
	imm, ok := result.Instructions[1].Operands[1].(ir.IrImmediate)
	if !ok || imm.Value != 3 {
		t.Fatalf("expected folded immediate value 3, got %#v", result.Instructions[1].Operands[1])
	}
}

func TestConstantFolderDoesNotFoldAcrossLabelBoundary(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLoadImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 0}}, ID: 1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: "loop"}}, ID: -1})
	program.AddInstruction(ir.IrInstruction{Opcode: ir.OpAddImm, Operands: []ir.IrOperand{ir.IrRegister{Index: 0}, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 1}}, ID: 2})

	result := ConstantFolder{}.Run(program)
	if got := result.Instructions[2].Opcode; got != ir.OpAddImm {
		t.Fatalf("expected addi after label to stay as ADD_IMM, got %s", got.String())
	}
}
