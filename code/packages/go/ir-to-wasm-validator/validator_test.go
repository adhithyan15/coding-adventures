package irtowasmvalidator

import (
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

func TestValidateReportsUnsupportedSyscalls(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.AddInstruction(ir.IrInstruction{
		Opcode:   ir.OpLabel,
		Operands: []ir.IrOperand{ir.IrLabel{Name: "_start"}},
		ID:       -1,
	})
	program.AddInstruction(ir.IrInstruction{
		Opcode:   ir.OpSyscall,
		Operands: []ir.IrOperand{ir.IrImmediate{Value: 999}},
		ID:       1,
	})

	errors := Validate(program)
	if len(errors) != 1 {
		t.Fatalf("expected one validation error, got %d", len(errors))
	}
	if errors[0].Rule != "lowering" {
		t.Fatalf("expected lowering rule, got %q", errors[0].Rule)
	}
	if errors[0].Message != "unsupported SYSCALL number(s): 999" {
		t.Fatalf("unexpected message: %q", errors[0].Message)
	}
}
