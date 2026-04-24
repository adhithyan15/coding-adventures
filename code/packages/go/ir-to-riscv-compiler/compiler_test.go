package irtoriscvcompiler

import (
	"bytes"
	"fmt"
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

	resultRegister, _ := physicalRegister(2)
	if got := sim.CPU.Registers.Read(resultRegister); got != 42 {
		t.Fatalf("expected v2/x%d to contain 42, got %d", resultRegister, got)
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
	loadRegister, _ := physicalRegister(3)
	if got := sim.CPU.Registers.Read(loadRegister); got != 65 {
		t.Fatalf("expected v3/x%d to reload byte 65, got %d", loadRegister, got)
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

	loopRegister, _ := physicalRegister(0)
	if got := sim.CPU.Registers.Read(loopRegister); got != 0 {
		t.Fatalf("expected loop to count v0/x%d down to 0, got %d", loopRegister, got)
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

	entryRegister, _ := physicalRegister(0)
	if got := sim.CPU.Registers.Read(entryRegister); got != 7 {
		t.Fatalf("expected entry trampoline to skip helper code and set v0/x%d to 7, got %d", entryRegister, got)
	}
	if result.LabelOffsets["_start"] == 0 {
		t.Fatal("expected _start to live after the entry trampoline")
	}
}

func TestCompileUsesA0ForSyscallArguments(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpLoadImm, ir.IrRegister{Index: 4}, ir.IrImmediate{Value: 65}),
		instr(2, ir.OpSyscall, ir.IrImmediate{Value: riscv.SyscallWriteByte}),
		instr(3, ir.OpLoadImm, ir.IrRegister{Index: 4}, ir.IrImmediate{Value: 0}),
		instr(4, ir.OpSyscall, ir.IrImmediate{Value: riscv.SyscallExit}),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	host := riscv.NewHostIO(nil)
	sim := riscv.NewRiscVSimulatorWithHost(4096, host)
	sim.Run(result.Bytes)

	if got := host.OutputString(); got != "A" {
		t.Fatalf("expected syscall write to emit %q, got %q", "A", got)
	}
	if !host.Exited {
		t.Fatal("expected syscall exit to halt host execution")
	}
	if got := sim.CPU.Registers.Read(10); got != 0 {
		t.Fatalf("expected a0/x10 to contain exit code 0, got %d", got)
	}
}

func TestCompilePreservesReturnAddressAcrossNestedCalls(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpCall, ir.IrLabel{Name: "first"}),
		instr(2, ir.OpHalt),
		label("first"),
		instr(3, ir.OpCall, ir.IrLabel{Name: "second"}),
		instr(4, ir.OpRet),
		label("second"),
		instr(5, ir.OpLoadImm, ir.IrRegister{Index: 0}, ir.IrImmediate{Value: 42}),
		instr(6, ir.OpRet),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	resultRegister, _ := physicalRegister(0)
	if got := sim.CPU.Registers.Read(resultRegister); got != 42 {
		t.Fatalf("expected nested call result in v0/x%d to be 42, got %d", resultRegister, got)
	}
	expectedStackTop := uint32(result.DataOffsets[callFrameStackLabel] + callFrameStackSize)
	if got := sim.CPU.Registers.Read(2); got != expectedStackTop {
		t.Fatalf("expected stack pointer to be restored to %d, got %d", expectedStackTop, got)
	}
	if !strings.Contains(result.Assembly, "sw ra, 0(sp)") ||
		!strings.Contains(result.Assembly, "lw ra, 0(sp)") {
		t.Fatalf("expected call frame save/restore in assembly, got:\n%s", result.Assembly)
	}

	assembled, err := riscvassembler.Assemble(result.Assembly)
	if err != nil {
		t.Fatalf("assemble emitted assembly failed: %v\n%s", err, result.Assembly)
	}
	if !bytes.Equal(assembled.Bytes, result.Bytes) {
		t.Fatalf("expected emitted assembly to reassemble to compiler bytes")
	}
}

func TestCompilePreservesCallerRegistersAroundCalls(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpCall, ir.IrLabel{Name: "caller"}),
		instr(2, ir.OpHalt),
		label("caller"),
		instr(3, ir.OpLoadImm, ir.IrRegister{Index: 8}, ir.IrImmediate{Value: 5}),
		instr(4, ir.OpLoadImm, ir.IrRegister{Index: 9}, ir.IrImmediate{Value: 9}),
		instr(5, ir.OpCall, ir.IrLabel{Name: "clobber"}),
		instr(6, ir.OpAdd, ir.IrRegister{Index: 1}, ir.IrRegister{Index: 8}, ir.IrRegister{Index: 9}),
		instr(7, ir.OpRet),
		label("clobber"),
		instr(8, ir.OpLoadImm, ir.IrRegister{Index: 8}, ir.IrImmediate{Value: 100}),
		instr(9, ir.OpLoadImm, ir.IrRegister{Index: 9}, ir.IrImmediate{Value: 200}),
		instr(10, ir.OpRet),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	returnRegister, _ := physicalRegister(1)
	if got := sim.CPU.Registers.Read(returnRegister); got != 14 {
		t.Fatalf("expected caller return register v1/x%d to preserve 5+9 across call, got %d", returnRegister, got)
	}
	expectedStackTop := uint32(result.DataOffsets[callFrameStackLabel] + callFrameStackSize)
	if got := sim.CPU.Registers.Read(2); got != expectedStackTop {
		t.Fatalf("expected stack pointer to be restored to %d, got %d", expectedStackTop, got)
	}
	callerSavedRegister, _ := physicalRegister(8)
	callerSavedName := fmt.Sprintf("x%d", callerSavedRegister)
	if !strings.Contains(result.Assembly, "sw "+callerSavedName+", 0(sp)") ||
		!strings.Contains(result.Assembly, "lw "+callerSavedName+", 0(sp)") {
		t.Fatalf("expected caller-save spill/reload in assembly, got:\n%s", result.Assembly)
	}

	assembled, err := riscvassembler.Assemble(result.Assembly)
	if err != nil {
		t.Fatalf("assemble emitted assembly failed: %v\n%s", err, result.Assembly)
	}
	if !bytes.Equal(assembled.Bytes, result.Bytes) {
		t.Fatalf("expected emitted assembly to reassemble to compiler bytes")
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

func TestCompileSpillsVirtualRegistersBeyondStarterMap(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpLoadImm, ir.IrRegister{Index: 20}, ir.IrImmediate{Value: 20}),
		instr(2, ir.OpLoadImm, ir.IrRegister{Index: 21}, ir.IrImmediate{Value: 22}),
		instr(3, ir.OpAdd, ir.IrRegister{Index: 22}, ir.IrRegister{Index: 20}, ir.IrRegister{Index: 21}),
		instr(4, ir.OpAddImm, ir.IrRegister{Index: 1}, ir.IrRegister{Index: 22}, ir.IrImmediate{Value: 0}),
		instr(5, ir.OpHalt),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	returnRegister, _ := physicalRegister(1)
	if got := sim.CPU.Registers.Read(returnRegister); got != 42 {
		t.Fatalf("expected spilled add result to round-trip into v1/x%d as 42, got %d", returnRegister, got)
	}
	if !strings.Contains(result.Assembly, "sw x28, 8(sp)") ||
		!strings.Contains(result.Assembly, "lw x29, 8(sp)") {
		t.Fatalf("expected spill-slot store/load in assembly, got:\n%s", result.Assembly)
	}

	assembled, err := riscvassembler.Assemble(result.Assembly)
	if err != nil {
		t.Fatalf("assemble emitted assembly failed: %v\n%s", err, result.Assembly)
	}
	if !bytes.Equal(assembled.Bytes, result.Bytes) {
		t.Fatalf("expected emitted assembly to reassemble to compiler bytes")
	}
}

func TestCompilePreservesSpilledRegistersAcrossCalls(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpCall, ir.IrLabel{Name: "caller"}),
		instr(2, ir.OpHalt),
		label("caller"),
		instr(3, ir.OpLoadImm, ir.IrRegister{Index: 20}, ir.IrImmediate{Value: 5}),
		instr(4, ir.OpLoadImm, ir.IrRegister{Index: 21}, ir.IrImmediate{Value: 9}),
		instr(5, ir.OpCall, ir.IrLabel{Name: "clobber"}),
		instr(6, ir.OpAdd, ir.IrRegister{Index: 1}, ir.IrRegister{Index: 20}, ir.IrRegister{Index: 21}),
		instr(7, ir.OpRet),
		label("clobber"),
		instr(8, ir.OpLoadImm, ir.IrRegister{Index: 20}, ir.IrImmediate{Value: 100}),
		instr(9, ir.OpLoadImm, ir.IrRegister{Index: 21}, ir.IrImmediate{Value: 200}),
		instr(10, ir.OpRet),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	returnRegister, _ := physicalRegister(1)
	if got := sim.CPU.Registers.Read(returnRegister); got != 14 {
		t.Fatalf("expected spilled caller locals to survive nested call and sum to 14, got %d", got)
	}
	expectedStackTop := uint32(result.DataOffsets[callFrameStackLabel] + callFrameStackSize)
	if got := sim.CPU.Registers.Read(2); got != expectedStackTop {
		t.Fatalf("expected stack pointer to be restored to %d, got %d", expectedStackTop, got)
	}
	if !strings.Contains(result.Assembly, "addi sp, sp, -12") {
		t.Fatalf("expected function frame allocation in assembly, got:\n%s", result.Assembly)
	}

	assembled, err := riscvassembler.Assemble(result.Assembly)
	if err != nil {
		t.Fatalf("assemble emitted assembly failed: %v\n%s", err, result.Assembly)
	}
	if !bytes.Equal(assembled.Bytes, result.Bytes) {
		t.Fatalf("expected emitted assembly to reassemble to compiler bytes")
	}
}

func TestCompileUsesCorrectBranchPcAfterSpillReload(t *testing.T) {
	program := ir.NewIrProgram("_start")
	program.Instructions = []ir.IrInstruction{
		label("_start"),
		instr(1, ir.OpLoadImm, ir.IrRegister{Index: 20}, ir.IrImmediate{Value: 3}),
		label("loop"),
		instr(2, ir.OpAddImm, ir.IrRegister{Index: 20}, ir.IrRegister{Index: 20}, ir.IrImmediate{Value: -1}),
		instr(3, ir.OpBranchNz, ir.IrRegister{Index: 20}, ir.IrLabel{Name: "loop"}),
		instr(4, ir.OpAddImm, ir.IrRegister{Index: 1}, ir.IrRegister{Index: 20}, ir.IrImmediate{Value: 0}),
		instr(5, ir.OpHalt),
	}

	result, err := NewIrToRiscVCompiler().Compile(program)
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	assembled, err := riscvassembler.Assemble(result.Assembly)
	if err != nil {
		t.Fatalf("assemble emitted assembly failed: %v\n%s", err, result.Assembly)
	}
	if !bytes.Equal(assembled.Bytes, result.Bytes) {
		t.Fatalf("expected emitted assembly to reassemble to compiler bytes")
	}

	loopRegister, _ := physicalRegister(1)
	if got := sim.CPU.Registers.Read(loopRegister); got != 0 {
		t.Fatalf("expected spilled loop counter to reach 0, got %d", got)
	}
}

func instr(id int, opcode ir.IrOp, operands ...ir.IrOperand) ir.IrInstruction {
	return ir.IrInstruction{ID: id, Opcode: opcode, Operands: operands}
}

func label(name string) ir.IrInstruction {
	return ir.IrInstruction{ID: -1, Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: name}}}
}
