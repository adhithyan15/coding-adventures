package compilerir

import "fmt"

// ──────────────────────────────────────────────────────────────────────────────
// IR Operand Types
//
// Every IR instruction operates on operands. There are three kinds:
//
//   IrRegister  — a virtual register (v0, v1, v2, ...)
//   IrImmediate — a literal integer value
//   IrLabel     — a named jump target or data label
//
// Operands are immutable value types. They implement the IrOperand
// interface so instructions can hold heterogeneous operand lists.
// ──────────────────────────────────────────────────────────────────────────────

// IrOperand is the interface satisfied by all operand types.
type IrOperand interface {
	// operandTag prevents external implementations — only the three
	// operand types in this package satisfy IrOperand.
	operandTag()

	// String returns a human-readable representation of the operand.
	String() string
}

// ──────────────────────────────────────────────────────────────────────────────
// IrRegister — a virtual register
//
// Virtual registers are named v0, v1, v2, ... (the Index field).
// There are infinitely many — the backend's register allocator maps
// them to physical registers.
//
// Example:
//   IrRegister{Index: 0}  →  "v0"
//   IrRegister{Index: 5}  →  "v5"
// ──────────────────────────────────────────────────────────────────────────────

type IrRegister struct {
	Index int
}

func (r IrRegister) operandTag() {}
func (r IrRegister) String() string {
	return fmt.Sprintf("v%d", r.Index)
}

// ──────────────────────────────────────────────────────────────────────────────
// IrImmediate — a literal integer value
//
// Immediates are signed integers that appear directly in instructions.
//
// Example:
//   IrImmediate{Value: 42}   →  "42"
//   IrImmediate{Value: -1}   →  "-1"
//   IrImmediate{Value: 255}  →  "255"
// ──────────────────────────────────────────────────────────────────────────────

type IrImmediate struct {
	Value int
}

func (i IrImmediate) operandTag() {}
func (i IrImmediate) String() string {
	return fmt.Sprintf("%d", i.Value)
}

// ──────────────────────────────────────────────────────────────────────────────
// IrLabel — a named target for jumps, branches, calls, or data references
//
// Labels are strings like "loop_0_start", "_start", "tape", "__trap_oob".
// They resolve to addresses during code generation.
//
// Example:
//   IrLabel{Name: "_start"}       →  "_start"
//   IrLabel{Name: "loop_0_end"}   →  "loop_0_end"
// ──────────────────────────────────────────────────────────────────────────────

type IrLabel struct {
	Name string
}

func (l IrLabel) operandTag() {}
func (l IrLabel) String() string {
	return l.Name
}

// ──────────────────────────────────────────────────────────────────────────────
// IrInstruction — a single IR instruction
//
// Every instruction has:
//   - Opcode:   what operation to perform (ADD_IMM, BRANCH_Z, etc.)
//   - Operands: the arguments (registers, immediates, labels)
//   - ID:       a unique monotonic integer for source mapping
//
// The ID field is the key that connects this instruction to the source
// map chain. Each instruction gets a unique ID assigned by the IDGenerator,
// and that ID flows through all pipeline stages.
//
// Examples:
//   {Opcode: OpAddImm, Operands: [v1, v1, 1], ID: 3}
//     →  ADD_IMM v1, v1, 1  ; #3
//
//   {Opcode: OpBranchZ, Operands: [v2, loop_0_end], ID: 7}
//     →  BRANCH_Z v2, loop_0_end  ; #7
// ──────────────────────────────────────────────────────────────────────────────

type IrInstruction struct {
	Opcode   IrOp
	Operands []IrOperand
	ID       int
}

// ──────────────────────────────────────────────────────────────────────────────
// IrDataDecl — a data segment declaration
//
// Declares a named region of memory with a given size and initial byte
// value. For Brainfuck, this is the tape:
//
//   IrDataDecl{Label: "tape", Size: 30000, Init: 0}
//     →  .data tape 30000 0
//
// The Init value is repeated for every byte in the region. Init=0 means
// zero-initialized (equivalent to .bss in most formats).
// ──────────────────────────────────────────────────────────────────────────────

type IrDataDecl struct {
	Label string
	Size  int
	Init  int // initial byte value (usually 0)
}

// ──────────────────────────────────────────────────────────────────────────────
// IrProgram — a complete IR program
//
// An IrProgram contains:
//   - Instructions: the linear sequence of IR instructions
//   - Data:         data segment declarations (.bss, .data)
//   - EntryLabel:   the label where execution begins
//   - Version:      IR version number (1 = Brainfuck subset)
//
// The Instructions slice is ordered — execution flows from index 0
// to len-1, with jumps/branches altering the flow.
// ──────────────────────────────────────────────────────────────────────────────

type IrProgram struct {
	Instructions []IrInstruction
	Data         []IrDataDecl
	EntryLabel   string
	Version      int // IR version (1 = v1 Brainfuck subset)
}

// NewIrProgram creates a new IR program with the given entry label
// and version 1.
func NewIrProgram(entryLabel string) *IrProgram {
	return &IrProgram{
		EntryLabel: entryLabel,
		Version:    1,
	}
}

// AddInstruction appends an instruction to the program.
func (p *IrProgram) AddInstruction(instr IrInstruction) {
	p.Instructions = append(p.Instructions, instr)
}

// AddData appends a data declaration to the program.
func (p *IrProgram) AddData(decl IrDataDecl) {
	p.Data = append(p.Data, decl)
}

// ──────────────────────────────────────────────────────────────────────────────
// IDGenerator — produces unique monotonic instruction IDs
//
// Every IR instruction in the pipeline needs a unique ID for source
// mapping. The IDGenerator ensures no two instructions ever share an ID,
// even across multiple compiler invocations within the same process.
//
// Usage:
//   gen := NewIDGenerator()
//   id1 := gen.Next()  // 0
//   id2 := gen.Next()  // 1
//   id3 := gen.Next()  // 2
// ──────────────────────────────────────────────────────────────────────────────

type IDGenerator struct {
	next int
}

// NewIDGenerator creates a new ID generator starting at 0.
func NewIDGenerator() *IDGenerator {
	return &IDGenerator{next: 0}
}

// NewIDGeneratorFrom creates a new ID generator starting at the given value.
// This is useful when multiple compilers contribute instructions to the
// same program and IDs must not collide.
func NewIDGeneratorFrom(start int) *IDGenerator {
	return &IDGenerator{next: start}
}

// Next returns the next unique ID and increments the counter.
func (g *IDGenerator) Next() int {
	id := g.next
	g.next++
	return id
}

// Current returns the current counter value without incrementing.
// This is the ID that will be returned by the next call to Next().
func (g *IDGenerator) Current() int {
	return g.next
}
