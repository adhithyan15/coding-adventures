package riscvassembler

import (
	"testing"

	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

func TestAssembleRunsProgramOnSimulator(t *testing.T) {
	result, err := Assemble(`
.text
_start:
  li a0, 40
  addi a0, a0, 2
  halt
`)
	if err != nil {
		t.Fatalf("assemble failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	if got := sim.CPU.Registers.Read(10); got != 42 {
		t.Fatalf("expected a0 to contain 42, got %d", got)
	}
	if result.LabelOffsets["_start"] != 0 {
		t.Fatalf("expected _start at 0, got %d", result.LabelOffsets["_start"])
	}
}

func TestAssembleResolvesLabelsAndData(t *testing.T) {
	result, err := Assemble(`
.text
_start:
  la t0, value
  lbu a0, 0(t0)
  j done
  li a0, 99
done:
  halt

.data
value:
  .byte 65
  .zero 3
`)
	if err != nil {
		t.Fatalf("assemble failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	if got := sim.CPU.Registers.Read(10); got != 65 {
		t.Fatalf("expected a0 to load data byte 65, got %d", got)
	}
	if got := result.DataOffsets["value"]; got != result.TextSize {
		t.Fatalf("expected value at text size %d, got %d", result.TextSize, got)
	}
}

func TestAssembleBackwardBranch(t *testing.T) {
	result, err := Assemble(`
_start:
  li t0, 3
loop:
  addi t0, t0, -1
  bne t0, zero, loop
  halt
`)
	if err != nil {
		t.Fatalf("assemble failed: %v", err)
	}

	sim := riscv.NewRiscVSimulator(4096)
	sim.Run(result.Bytes)

	if got := sim.CPU.Registers.Read(5); got != 0 {
		t.Fatalf("expected t0 to count down to 0, got %d", got)
	}
}

func TestAssembleRejectsUnknownLabel(t *testing.T) {
	_, err := Assemble("j missing")
	if err == nil {
		t.Fatal("expected unknown label error")
	}
}
