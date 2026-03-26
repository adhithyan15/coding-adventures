// Package brainfuck implements a Brainfuck interpreter built on top of the
// virtual-machine package's types (OpCode, Instruction, CodeObject, VMTrace).
//
// ==========================================================================
// From 8 Characters to 9 Opcodes
// ==========================================================================
//
// Brainfuck has 8 commands. We map each to a numeric opcode, plus HALT to
// mark the end of the program. These opcodes reuse the vm.OpCode type so
// that Brainfuck bytecode lives in the same CodeObject format as any other
// language targeting the virtual machine.
//
// Why numeric opcodes instead of characters? Because the VM dispatches on
// integers — it's a *bytecode* interpreter, not a character interpreter.
// This also means the same CodeObject that holds Starlark's 0x01-0xFF
// opcodes can hold Brainfuck's 0x01-0x08 opcodes. Different opcode
// *numbers*, different *handlers*, same data structures.
//
// ==========================================================================
// Opcode Table
// ==========================================================================
//
//	Opcode       Hex    BF    Description
//	─────────────────────────────────────────────────────────────
//	OpRight      0x01   >     Move data pointer right
//	OpLeft       0x02   <     Move data pointer left
//	OpInc        0x03   +     Increment current cell
//	OpDec        0x04   -     Decrement current cell
//	OpOutput     0x05   .     Print cell as ASCII
//	OpInput      0x06   ,     Read byte into cell
//	OpLoopStart  0x07   [     Jump forward if cell == 0
//	OpLoopEnd    0x08   ]     Jump backward if cell != 0
//	OpHalt       0xFF   —     Stop execution
//
// Note that Brainfuck opcodes have **no stack effect**. Unlike a stack-based
// language (push, push, add, pop result), Brainfuck operates entirely on the
// tape. The VM's operand stack goes unused — but the CodeObject structure
// still carries it, ready for any language that needs it.
package brainfuck

import (
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// =========================================================================
// Opcode constants
// =========================================================================
//
// Each constant maps one Brainfuck character to a numeric opcode. The hex
// values are arbitrary but chosen to be sequential starting at 0x01, with
// HALT at 0xFF (matching the virtual-machine package's convention).

const (
	// OpRight is ">" — move the data pointer one cell to the right.
	OpRight vm.OpCode = 0x01

	// OpLeft is "<" — move the data pointer one cell to the left.
	OpLeft vm.OpCode = 0x02

	// OpInc is "+" — increment the byte at the data pointer (wraps 255 → 0).
	OpInc vm.OpCode = 0x03

	// OpDec is "-" — decrement the byte at the data pointer (wraps 0 → 255).
	OpDec vm.OpCode = 0x04

	// OpOutput is "." — output the byte at the data pointer as ASCII.
	OpOutput vm.OpCode = 0x05

	// OpInput is "," — read one byte of input into the current cell.
	OpInput vm.OpCode = 0x06

	// OpLoopStart is "[" — if the current cell is zero, jump forward past
	// the matching "]". The operand holds the target instruction index.
	OpLoopStart vm.OpCode = 0x07

	// OpLoopEnd is "]" — if the current cell is nonzero, jump backward to
	// the matching "[". The operand holds the target instruction index.
	OpLoopEnd vm.OpCode = 0x08

	// OpHalt stops execution. Every translated program ends with this.
	OpHalt vm.OpCode = 0xFF
)

// =========================================================================
// Character-to-opcode mapping
// =========================================================================

// CharToOp maps each Brainfuck source character to its opcode. Characters
// not present in this map are ignored during translation — they're treated
// as comments, which is how Brainfuck handles all non-command characters.
//
// For example, the program "Hello, +++World!" contains only three actual
// commands: +, +, +. Everything else is commentary.
var CharToOp = map[byte]vm.OpCode{
	'>': OpRight,
	'<': OpLeft,
	'+': OpInc,
	'-': OpDec,
	'.': OpOutput,
	',': OpInput,
	'[': OpLoopStart,
	']': OpLoopEnd,
}
