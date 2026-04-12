package compilerir

import (
	"strings"
	"testing"
)

// ──────────────────────────────────────────────────────────────────────────────
// Opcode tests
// ──────────────────────────────────────────────────────────────────────────────

func TestOpString(t *testing.T) {
	tests := []struct {
		op   IrOp
		want string
	}{
		{OpLoadImm, "LOAD_IMM"},
		{OpLoadAddr, "LOAD_ADDR"},
		{OpLoadByte, "LOAD_BYTE"},
		{OpStoreByte, "STORE_BYTE"},
		{OpLoadWord, "LOAD_WORD"},
		{OpStoreWord, "STORE_WORD"},
		{OpAdd, "ADD"},
		{OpAddImm, "ADD_IMM"},
		{OpSub, "SUB"},
		{OpAnd, "AND"},
		{OpAndImm, "AND_IMM"},
		{OpCmpEq, "CMP_EQ"},
		{OpCmpNe, "CMP_NE"},
		{OpCmpLt, "CMP_LT"},
		{OpCmpGt, "CMP_GT"},
		{OpLabel, "LABEL"},
		{OpJump, "JUMP"},
		{OpBranchZ, "BRANCH_Z"},
		{OpBranchNz, "BRANCH_NZ"},
		{OpCall, "CALL"},
		{OpRet, "RET"},
		{OpSyscall, "SYSCALL"},
		{OpHalt, "HALT"},
		{OpNop, "NOP"},
		{OpComment, "COMMENT"},
	}

	for _, tt := range tests {
		got := tt.op.String()
		if got != tt.want {
			t.Errorf("IrOp(%d).String() = %q, want %q", tt.op, got, tt.want)
		}
	}
}

func TestOpStringUnknown(t *testing.T) {
	got := IrOp(9999).String()
	if got != "UNKNOWN" {
		t.Errorf("unknown opcode should return %q, got %q", "UNKNOWN", got)
	}
}

func TestParseOp(t *testing.T) {
	for op, name := range opNames {
		parsed, ok := ParseOp(name)
		if !ok {
			t.Errorf("ParseOp(%q) returned false", name)
			continue
		}
		if parsed != op {
			t.Errorf("ParseOp(%q) = %d, want %d", name, parsed, op)
		}
	}
}

func TestParseOpUnknown(t *testing.T) {
	_, ok := ParseOp("NONSENSE")
	if ok {
		t.Error("ParseOp(\"NONSENSE\") should return false")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Operand tests
// ──────────────────────────────────────────────────────────────────────────────

func TestRegisterString(t *testing.T) {
	r := IrRegister{Index: 3}
	if r.String() != "v3" {
		t.Errorf("expected v3, got %s", r.String())
	}
}

func TestImmediateString(t *testing.T) {
	tests := []struct {
		val  int
		want string
	}{
		{42, "42"},
		{-1, "-1"},
		{0, "0"},
		{255, "255"},
	}
	for _, tt := range tests {
		got := IrImmediate{Value: tt.val}.String()
		if got != tt.want {
			t.Errorf("IrImmediate{%d}.String() = %q, want %q", tt.val, got, tt.want)
		}
	}
}

func TestLabelString(t *testing.T) {
	l := IrLabel{Name: "loop_0_start"}
	if l.String() != "loop_0_start" {
		t.Errorf("expected loop_0_start, got %s", l.String())
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// IDGenerator tests
// ──────────────────────────────────────────────────────────────────────────────

func TestIDGeneratorSequence(t *testing.T) {
	gen := NewIDGenerator()
	for i := 0; i < 5; i++ {
		id := gen.Next()
		if id != i {
			t.Errorf("expected ID %d, got %d", i, id)
		}
	}
}

func TestIDGeneratorFrom(t *testing.T) {
	gen := NewIDGeneratorFrom(100)
	if gen.Next() != 100 {
		t.Error("expected first ID to be 100")
	}
	if gen.Next() != 101 {
		t.Error("expected second ID to be 101")
	}
}

func TestIDGeneratorCurrent(t *testing.T) {
	gen := NewIDGenerator()
	if gen.Current() != 0 {
		t.Error("expected current to be 0")
	}
	gen.Next()
	if gen.Current() != 1 {
		t.Error("expected current to be 1 after one Next()")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// IrProgram tests
// ──────────────────────────────────────────────────────────────────────────────

func TestNewIrProgram(t *testing.T) {
	p := NewIrProgram("_start")
	if p.EntryLabel != "_start" {
		t.Errorf("expected entry label _start, got %s", p.EntryLabel)
	}
	if p.Version != 1 {
		t.Errorf("expected version 1, got %d", p.Version)
	}
	if len(p.Instructions) != 0 {
		t.Error("expected no instructions initially")
	}
	if len(p.Data) != 0 {
		t.Error("expected no data declarations initially")
	}
}

func TestIrProgramAddInstruction(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadImm,
		Operands: []IrOperand{IrRegister{0}, IrImmediate{42}},
		ID:       0,
	})
	if len(p.Instructions) != 1 {
		t.Fatalf("expected 1 instruction, got %d", len(p.Instructions))
	}
	if p.Instructions[0].Opcode != OpLoadImm {
		t.Error("expected LOAD_IMM opcode")
	}
}

func TestIrProgramAddData(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddData(IrDataDecl{Label: "tape", Size: 30000, Init: 0})
	if len(p.Data) != 1 {
		t.Fatalf("expected 1 data decl, got %d", len(p.Data))
	}
	if p.Data[0].Label != "tape" {
		t.Errorf("expected label 'tape', got %s", p.Data[0].Label)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Printer tests
// ──────────────────────────────────────────────────────────────────────────────

func TestPrintMinimalProgram(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddInstruction(IrInstruction{
		Opcode:   OpLabel,
		Operands: []IrOperand{IrLabel{"_start"}},
		ID:       -1,
	})
	p.AddInstruction(IrInstruction{
		Opcode: OpHalt,
		ID:     0,
	})

	text := Print(p)
	if !strings.Contains(text, ".version 1") {
		t.Error("expected .version 1 in output")
	}
	if !strings.Contains(text, ".entry _start") {
		t.Error("expected .entry _start in output")
	}
	if !strings.Contains(text, "_start:") {
		t.Error("expected _start: label in output")
	}
	if !strings.Contains(text, "HALT") {
		t.Error("expected HALT instruction in output")
	}
}

func TestPrintWithData(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddData(IrDataDecl{Label: "tape", Size: 30000, Init: 0})

	text := Print(p)
	if !strings.Contains(text, ".data tape 30000 0") {
		t.Error("expected .data tape 30000 0 in output")
	}
}

func TestPrintWithOperands(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddInstruction(IrInstruction{
		Opcode:   OpAddImm,
		Operands: []IrOperand{IrRegister{1}, IrRegister{1}, IrImmediate{1}},
		ID:       3,
	})

	text := Print(p)
	if !strings.Contains(text, "ADD_IMM") {
		t.Error("expected ADD_IMM in output")
	}
	if !strings.Contains(text, "v1, v1, 1") {
		t.Error("expected 'v1, v1, 1' in operands")
	}
	if !strings.Contains(text, "; #3") {
		t.Error("expected '; #3' ID comment")
	}
}

func TestPrintComment(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddInstruction(IrInstruction{
		Opcode:   OpComment,
		Operands: []IrOperand{IrLabel{Name: "load tape base"}},
		ID:       -1,
	})

	text := Print(p)
	if !strings.Contains(text, "; load tape base") {
		t.Errorf("expected '; load tape base' in output, got:\n%s", text)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Parser tests
// ──────────────────────────────────────────────────────────────────────────────

func TestParseMinimalProgram(t *testing.T) {
	text := `.version 1

.entry _start

_start:
  HALT  ; #0
`
	p, err := Parse(text)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	if p.Version != 1 {
		t.Errorf("expected version 1, got %d", p.Version)
	}
	if p.EntryLabel != "_start" {
		t.Errorf("expected entry _start, got %s", p.EntryLabel)
	}
	// Should have 2 instructions: LABEL + HALT
	if len(p.Instructions) != 2 {
		t.Fatalf("expected 2 instructions, got %d", len(p.Instructions))
	}
	if p.Instructions[0].Opcode != OpLabel {
		t.Error("first instruction should be LABEL")
	}
	if p.Instructions[1].Opcode != OpHalt {
		t.Error("second instruction should be HALT")
	}
	if p.Instructions[1].ID != 0 {
		t.Errorf("HALT instruction ID should be 0, got %d", p.Instructions[1].ID)
	}
}

func TestParseWithData(t *testing.T) {
	text := `.version 1

.data tape 30000 0

.entry _start

_start:
  HALT  ; #0
`
	p, err := Parse(text)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	if len(p.Data) != 1 {
		t.Fatalf("expected 1 data decl, got %d", len(p.Data))
	}
	if p.Data[0].Label != "tape" || p.Data[0].Size != 30000 || p.Data[0].Init != 0 {
		t.Errorf("unexpected data decl: %+v", p.Data[0])
	}
}

func TestParseOperands(t *testing.T) {
	text := `.version 1

.entry _start

_start:
  LOAD_IMM   v0, 42  ; #0
  ADD_IMM    v1, v1, -1  ; #1
  LOAD_ADDR  v0, tape  ; #2
`
	p, err := Parse(text)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}

	// LOAD_IMM v0, 42
	loadImm := p.Instructions[1] // [0] is LABEL
	if loadImm.Opcode != OpLoadImm {
		t.Errorf("expected LOAD_IMM, got %s", loadImm.Opcode)
	}
	if len(loadImm.Operands) != 2 {
		t.Fatalf("expected 2 operands, got %d", len(loadImm.Operands))
	}
	reg, ok := loadImm.Operands[0].(IrRegister)
	if !ok || reg.Index != 0 {
		t.Errorf("expected v0, got %v", loadImm.Operands[0])
	}
	imm, ok := loadImm.Operands[1].(IrImmediate)
	if !ok || imm.Value != 42 {
		t.Errorf("expected 42, got %v", loadImm.Operands[1])
	}

	// ADD_IMM v1, v1, -1
	addImm := p.Instructions[2]
	if addImm.Opcode != OpAddImm {
		t.Errorf("expected ADD_IMM, got %s", addImm.Opcode)
	}
	negImm, ok := addImm.Operands[2].(IrImmediate)
	if !ok || negImm.Value != -1 {
		t.Errorf("expected -1, got %v", addImm.Operands[2])
	}

	// LOAD_ADDR v0, tape
	loadAddr := p.Instructions[3]
	if loadAddr.Opcode != OpLoadAddr {
		t.Errorf("expected LOAD_ADDR, got %s", loadAddr.Opcode)
	}
	label, ok := loadAddr.Operands[1].(IrLabel)
	if !ok || label.Name != "tape" {
		t.Errorf("expected label 'tape', got %v", loadAddr.Operands[1])
	}
}

func TestParseUnknownOpcode(t *testing.T) {
	text := `.version 1

.entry _start

_start:
  NONSENSE v0, v1  ; #0
`
	_, err := Parse(text)
	if err == nil {
		t.Fatal("expected parse error for unknown opcode")
	}
	if !strings.Contains(err.Error(), "unknown opcode") {
		t.Errorf("expected 'unknown opcode' in error, got: %v", err)
	}
}

func TestParseInvalidVersion(t *testing.T) {
	_, err := Parse(".version abc")
	if err == nil {
		t.Fatal("expected error for invalid version")
	}
}

func TestParseInvalidData(t *testing.T) {
	_, err := Parse(".data tape abc 0")
	if err == nil {
		t.Fatal("expected error for invalid data size")
	}
}

func TestParseInvalidDataInit(t *testing.T) {
	_, err := Parse(".data tape 100 abc")
	if err == nil {
		t.Fatal("expected error for invalid data init")
	}
}

func TestParseInvalidVersionFieldCount(t *testing.T) {
	_, err := Parse(".version 1 2")
	if err == nil {
		t.Fatal("expected error for too many version fields")
	}
}

func TestParseInvalidDataFieldCount(t *testing.T) {
	_, err := Parse(".data tape 100")
	if err == nil {
		t.Fatal("expected error for too few data fields")
	}
}

func TestParseInvalidEntryFieldCount(t *testing.T) {
	_, err := Parse(".entry")
	if err == nil {
		t.Fatal("expected error for missing entry label")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Roundtrip test — the most important test
//
// This verifies that parse(print(program)) produces a structurally
// equivalent program. This is the key invariant that makes golden-file
// testing reliable.
// ──────────────────────────────────────────────────────────────────────────────

func TestPrintParseRoundtrip(t *testing.T) {
	// Build a non-trivial program
	gen := NewIDGenerator()
	p := NewIrProgram("_start")
	p.AddData(IrDataDecl{Label: "tape", Size: 30000, Init: 0})

	p.AddInstruction(IrInstruction{
		Opcode:   OpLabel,
		Operands: []IrOperand{IrLabel{"_start"}},
		ID:       -1,
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadAddr,
		Operands: []IrOperand{IrRegister{0}, IrLabel{"tape"}},
		ID:       gen.Next(), // 0
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadImm,
		Operands: []IrOperand{IrRegister{1}, IrImmediate{0}},
		ID:       gen.Next(), // 1
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadByte,
		Operands: []IrOperand{IrRegister{2}, IrRegister{0}, IrRegister{1}},
		ID:       gen.Next(), // 2
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpAddImm,
		Operands: []IrOperand{IrRegister{2}, IrRegister{2}, IrImmediate{1}},
		ID:       gen.Next(), // 3
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpAndImm,
		Operands: []IrOperand{IrRegister{2}, IrRegister{2}, IrImmediate{255}},
		ID:       gen.Next(), // 4
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpStoreByte,
		Operands: []IrOperand{IrRegister{2}, IrRegister{0}, IrRegister{1}},
		ID:       gen.Next(), // 5
	})
	p.AddInstruction(IrInstruction{
		Opcode: OpHalt,
		ID:     gen.Next(), // 6
	})

	// Print → Parse → compare
	text := Print(p)
	parsed, err := Parse(text)
	if err != nil {
		t.Fatalf("roundtrip parse failed: %v\n\nText:\n%s", err, text)
	}

	// Verify structural equivalence
	if parsed.Version != p.Version {
		t.Errorf("version: got %d, want %d", parsed.Version, p.Version)
	}
	if parsed.EntryLabel != p.EntryLabel {
		t.Errorf("entry: got %s, want %s", parsed.EntryLabel, p.EntryLabel)
	}
	if len(parsed.Data) != len(p.Data) {
		t.Fatalf("data decls: got %d, want %d", len(parsed.Data), len(p.Data))
	}
	for i, d := range parsed.Data {
		if d.Label != p.Data[i].Label || d.Size != p.Data[i].Size || d.Init != p.Data[i].Init {
			t.Errorf("data[%d]: got %+v, want %+v", i, d, p.Data[i])
		}
	}
	if len(parsed.Instructions) != len(p.Instructions) {
		t.Fatalf("instructions: got %d, want %d", len(parsed.Instructions), len(p.Instructions))
	}
	for i, instr := range parsed.Instructions {
		orig := p.Instructions[i]
		if instr.Opcode != orig.Opcode {
			t.Errorf("instr[%d] opcode: got %s, want %s", i, instr.Opcode, orig.Opcode)
		}
		if len(instr.Operands) != len(orig.Operands) {
			t.Errorf("instr[%d] operands: got %d, want %d", i, len(instr.Operands), len(orig.Operands))
			continue
		}
		for j, op := range instr.Operands {
			if op.String() != orig.Operands[j].String() {
				t.Errorf("instr[%d] operand[%d]: got %s, want %s", i, j, op.String(), orig.Operands[j].String())
			}
		}
	}
}

func TestPrintParseRoundtripWithBranches(t *testing.T) {
	gen := NewIDGenerator()
	p := NewIrProgram("_start")

	p.AddInstruction(IrInstruction{Opcode: OpLabel, Operands: []IrOperand{IrLabel{"_start"}}, ID: -1})
	p.AddInstruction(IrInstruction{Opcode: OpLabel, Operands: []IrOperand{IrLabel{"loop_0_start"}}, ID: -1})
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadByte,
		Operands: []IrOperand{IrRegister{2}, IrRegister{0}, IrRegister{1}},
		ID:       gen.Next(),
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpBranchZ,
		Operands: []IrOperand{IrRegister{2}, IrLabel{"loop_0_end"}},
		ID:       gen.Next(),
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpJump,
		Operands: []IrOperand{IrLabel{"loop_0_start"}},
		ID:       gen.Next(),
	})
	p.AddInstruction(IrInstruction{Opcode: OpLabel, Operands: []IrOperand{IrLabel{"loop_0_end"}}, ID: -1})
	p.AddInstruction(IrInstruction{Opcode: OpHalt, ID: gen.Next()})

	text := Print(p)
	parsed, err := Parse(text)
	if err != nil {
		t.Fatalf("roundtrip parse failed: %v\n\nText:\n%s", err, text)
	}

	if len(parsed.Instructions) != len(p.Instructions) {
		t.Errorf("instructions: got %d, want %d", len(parsed.Instructions), len(p.Instructions))
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Full program text test — verify exact printer output
// ──────────────────────────────────────────────────────────────────────────────

func TestPrintFullProgram(t *testing.T) {
	p := NewIrProgram("_start")
	p.AddData(IrDataDecl{Label: "tape", Size: 30000, Init: 0})

	p.AddInstruction(IrInstruction{
		Opcode:   OpLabel,
		Operands: []IrOperand{IrLabel{"_start"}},
		ID:       -1,
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadAddr,
		Operands: []IrOperand{IrRegister{0}, IrLabel{"tape"}},
		ID:       0,
	})
	p.AddInstruction(IrInstruction{
		Opcode:   OpLoadImm,
		Operands: []IrOperand{IrRegister{1}, IrImmediate{0}},
		ID:       1,
	})
	p.AddInstruction(IrInstruction{
		Opcode: OpHalt,
		ID:     2,
	})

	text := Print(p)

	// Check key elements are present in the right order
	lines := strings.Split(text, "\n")
	foundVersion := false
	foundData := false
	foundEntry := false
	foundLabel := false
	foundHalt := false

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, ".version") {
			foundVersion = true
		}
		if strings.HasPrefix(trimmed, ".data") {
			if !foundVersion {
				t.Error(".data should come after .version")
			}
			foundData = true
		}
		if strings.HasPrefix(trimmed, ".entry") {
			if !foundData {
				t.Error(".entry should come after .data")
			}
			foundEntry = true
		}
		if trimmed == "_start:" {
			if !foundEntry {
				t.Error("label should come after .entry")
			}
			foundLabel = true
		}
		if strings.Contains(trimmed, "HALT") {
			if !foundLabel {
				t.Error("HALT should come after label")
			}
			foundHalt = true
		}
	}

	if !foundVersion || !foundData || !foundEntry || !foundLabel || !foundHalt {
		t.Errorf("missing elements in output:\nversion=%v data=%v entry=%v label=%v halt=%v\n\n%s",
			foundVersion, foundData, foundEntry, foundLabel, foundHalt, text)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Instruction with no operands (HALT, RET, NOP)
// ──────────────────────────────────────────────────────────────────────────────

func TestParseNoOperandInstructions(t *testing.T) {
	text := `.version 1
.entry _start
_start:
  NOP  ; #0
  RET  ; #1
  HALT  ; #2
`
	p, err := Parse(text)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	// LABEL + NOP + RET + HALT = 4
	if len(p.Instructions) != 4 {
		t.Fatalf("expected 4 instructions, got %d", len(p.Instructions))
	}
	if p.Instructions[1].Opcode != OpNop {
		t.Error("expected NOP")
	}
	if p.Instructions[2].Opcode != OpRet {
		t.Error("expected RET")
	}
	if p.Instructions[3].Opcode != OpHalt {
		t.Error("expected HALT")
	}
}

func TestParseTooManyOperands(t *testing.T) {
	// Build a line with more than maxOperandsPerInstr operands
	operands := make([]string, 20)
	for i := range operands {
		operands[i] = "v0"
	}
	text := ".version 1\n.entry _start\n_start:\n  ADD " + strings.Join(operands, ", ") + "  ; #0\n"
	_, err := Parse(text)
	if err == nil {
		t.Fatal("expected error for too many operands")
	}
	if !strings.Contains(err.Error(), "too many operands") {
		t.Errorf("expected 'too many operands' error, got: %v", err)
	}
}

func TestParseRegisterIndexTooLarge(t *testing.T) {
	text := ".version 1\n.entry _start\n_start:\n  LOAD_IMM v99999999, 0  ; #0\n"
	_, err := Parse(text)
	if err == nil {
		t.Fatal("expected error for register index too large")
	}
	if !strings.Contains(err.Error(), "out of range") {
		t.Errorf("expected 'out of range' error, got: %v", err)
	}
}

func TestParseSyscall(t *testing.T) {
	text := `.version 1
.entry _start
_start:
  SYSCALL    1  ; #0
`
	p, err := Parse(text)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	syscall := p.Instructions[1] // [0] is LABEL
	if syscall.Opcode != OpSyscall {
		t.Errorf("expected SYSCALL, got %s", syscall.Opcode)
	}
	if len(syscall.Operands) != 1 {
		t.Fatalf("expected 1 operand, got %d", len(syscall.Operands))
	}
	imm, ok := syscall.Operands[0].(IrImmediate)
	if !ok || imm.Value != 1 {
		t.Errorf("expected immediate 1, got %v", syscall.Operands[0])
	}
}
