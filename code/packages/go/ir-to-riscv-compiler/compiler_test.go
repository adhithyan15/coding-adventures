package irtoriscvcompiler

import (
	"bytes"
	"strings"
	"testing"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	riscvassembler "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

func TestCompileRunsArithmeticOnRiscVSimulator(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpLoadImm, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 2}),
		instr(2, ir.OpLoadImm, ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 40}),
		instr(3, ir.OpAdd, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 0}, ir.IrRegister{Index: 1}),
		instr(4, ir.OpHalt),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	if got := sim.CPU.Registers.Read(7); got != 42 {
		t.Fatalf("expected v2/x7 to contain 42, got %d", got)
	}
	if offset, length := result.IrToMachineCode.LookupByIrID(3); offset != 8 || length != 4 {
		t.Fatalf("expected ADD mapping at offset=8 length=4, got offset=%d length=%d", offset, length)
	}
	if !strings.Contains(result.Assembly, "add x7, x5, x6") {
		t.Fatalf("expected assembly to contain lowered add, got:\n%s", result.Assembly)
	}

	assembled, err := riscvassembler.Assemble(result.Assembly)
	if err != nil {
		t.Fatalf("assemble emitted assembly failed: %v\n%s", err, result.Assembly)
	}
	if !bytes.Equal(assembled.Bytes, result.Bytes) {
		t.Fatalf("expected emitted assembly to reassemble to compiler bytes")
	}
}

func TestCompileLowersDataAddressAndMemoryOps(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Data = []ir.IrDataDecl{{Label: "tape", Size: 4, Init: 0}}
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpLoadAddr, ir.IrRegister{Index: 0}, ir.IrLabel{Name: "tape"}),
		instr(2, ir.OpLoadImm, ir.IrRegister{Index: 1}, ir.IrImmediate{Value: 0}),
		instr(3, ir.OpLoadImm, ir.IrRegister{Index: 2}, ir.IrImmediate{Value: 65}),
		instr(4, ir.OpStoreByte, ir.IrRegister{Index: 2}, ir.IrRegister{Index: 0}, ir.IrRegister{Index: 1}),
		instr(5, ir.OpLoadByte, ir.IrRegister{Index: 3}, ir.IrRegister{Index: 0}, ir.IrRegister{Index: 1}),
		instr(6, ir.OpHalt),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	tapeOffset := result.DataOffsets["tape"]
	if got := sim.CPU.Registers.Read(28); got != 65 {
		t.Fatalf("expected v3/x28 to reload byte 65, got %d", got)
	}
	if got := sim.CPU.Memory.ReadByte(tapeOffset); got != 65 {
		t.Fatalf("expected tape byte at %d to be 65, got %d", tapeOffset, got)
	}
}

func TestCompileResolvesBackwardBranch(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpLoadImm, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 3}),
		label("loop"),
		instr(2, ir.OpAddImm, ir.IrRegister{Index: 0}, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: -1}),
		instr(3, ir.OpBranchNz, ir.IrRegister{Index: 0}, ir.IrLabel{Name: "loop"}),
		instr(4, ir.OpHalt),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	if got := sim.CPU.Registers.Read(5); got != 0 {
		t.Fatalf("expected loop to count v0/x5 down to 0, got %d", got)
	}
	if result.LabelOffsets["loop"] != 4 {
		t.Fatalf("expected loop label at byte offset 4, got %d", result.LabelOffsets["loop"])
	}
}

func TestCompileHonorsEntryLabelWithTrampoline(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("helper"),
		instr(1, ir.OpLoadImm, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 99}),
		instr(2, ir.OpHalt),
		label("_start"),
		instr(3, ir.OpLoadImm, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 7}),
		instr(4, ir.OpHalt),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	if got := sim.CPU.Registers.Read(5); got != 7 {
		t.Fatalf("expected entry trampoline to skip helper code and set v0/x5 to 7, got %d", got)
	}
	if result.LabelOffsets["_start"] == 0 {
		t.Fatal("expected _start to live after the entry trampoline")
	}
}

func TestCompileRejectsUnknownLabel(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpJump, ir.IrLabel{Name: "missing"}),
	}

	_, err := NewIrToRiscVCompiler().Compile(program)
	if err == nil {
		t.Fatal("expected missing label error")
	}
}

func instr(id int, opcode ir.IrOp, operands ...ir.IrOperand) ir.IrInstruction {
	return ir.IrInstruction{ID: id, Opcode: opcode, Operands: operands}
}

func label(name string) ir.IrInstruction {
	return ir.IrInstruction{ID: -1, Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: name}}}
}
