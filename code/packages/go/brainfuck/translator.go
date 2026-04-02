package brainfuck

// ==========================================================================
// Brainfuck Translator — Source Code to Bytecode in One Pass
// ==========================================================================
//
// Why "Translator" and not "Compiler"?
// ------------------------------------
//
// A compiler transforms a high-level *structured* representation (an AST)
// into lower-level instructions. It handles scoping, type checking, operator
// precedence, and all the complexity that comes with real languages.
//
// Brainfuck doesn't have any of that. There's no AST, no scoping, no types.
// Each source character maps directly to one instruction. The only non-trivial
// step is **bracket matching** — connecting "[" to its matching "]" so the
// VM knows where to jump.
//
// So we call this a "translator" rather than a "compiler": it translates
// characters to opcodes, with bracket matching as the sole transformation.
//
// ==========================================================================
// How Bracket Matching Works
// ==========================================================================
//
// Bracket matching is a classic stack problem:
//
//  1. Scan the source left to right.
//  2. When we see "[", emit a LOOP_START with a placeholder target (0),
//     and push its instruction index onto a stack.
//  3. When we see "]", pop the matching "[" index from the stack.
//     - Patch the "[" instruction to jump to one past the current "]".
//     - Emit a LOOP_END that jumps back to the "[".
//  4. After scanning, if the stack isn't empty, we have unmatched "[".
//
// This is identical to how a compiler's emit_jump() / patch_jump() work,
// but we do it by hand since Brainfuck is too simple to need the full
// compiler framework.
//
// ==========================================================================
// Example
// ==========================================================================
//
// Source: "++[>+<-]"
//
// Translation:
//
//	Index  Opcode       Operand   Source
//	─────────────────────────────────────
//	0      INC          —         +
//	1      INC          —         +
//	2      LOOP_START   8         [  (jump to 8 if cell==0)
//	3      RIGHT        —         >
//	4      INC          —         +
//	5      LEFT         —         <
//	6      DEC          —         -
//	7      LOOP_END     2         ]  (jump to 2 if cell!=0)
//	8      HALT         —         (end)
//
// When cell 0 reaches zero, LOOP_START at index 2 jumps to index 8
// (one past the LOOP_END at index 7, which is HALT). When the cell is
// still nonzero, LOOP_END at index 7 jumps back to index 2.

import (
	"fmt"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// Translate converts a Brainfuck source string into a vm.CodeObject.
//
// Each Brainfuck command character (> < + - . , [ ]) becomes one Instruction.
// Non-command characters are silently ignored (they're comments). A HALT
// instruction is appended at the end.
//
// Translate panics if brackets are mismatched — either an unmatched "[" or
// an unmatched "]". This matches the existing virtual-machine convention of
// using panics for translation-time errors.
//
// The returned CodeObject has empty Constants and Names slices, since
// Brainfuck has no variables or literal values.
//
// Example:
//
//	code := Translate("+++.")
//	// code.Instructions has 5 entries: INC, INC, INC, OUTPUT, HALT
//	// code.Constants is []
//	// code.Names is []
func Translate(source string) vm.CodeObject {
	result, _ := StartNew[vm.CodeObject]("brainfuck.Translate", vm.CodeObject{},
		func(op *Operation[vm.CodeObject], rf *ResultFactory[vm.CodeObject]) *OperationResult[vm.CodeObject] {
			op.AddProperty("sourceLen", len(source))
			// The instructions we're building up, one per BF command.
			instructions := []vm.Instruction{}

			// bracketStack tracks the instruction indices of unmatched "[" opcodes.
			bracketStack := []int{}

			for i := 0; i < len(source); i++ {
				ch := source[i]
				bfop, ok := CharToOp[ch]
				if !ok {
					continue
				}

				switch bfop {
				case OpLoopStart:
					index := len(instructions)
					instructions = append(instructions, vm.Instruction{
						Opcode:  OpLoopStart,
						Operand: 0,
					})
					bracketStack = append(bracketStack, index)

				case OpLoopEnd:
					if len(bracketStack) == 0 {
						panic("TranslationError: Unmatched ']' — no matching '[' found")
					}
					startIndex := bracketStack[len(bracketStack)-1]
					bracketStack = bracketStack[:len(bracketStack)-1]

					endIndex := len(instructions)

					instructions[startIndex] = vm.Instruction{
						Opcode:  OpLoopStart,
						Operand: endIndex + 1,
					}

					instructions = append(instructions, vm.Instruction{
						Opcode:  OpLoopEnd,
						Operand: startIndex,
					})

				default:
					instructions = append(instructions, vm.Instruction{
						Opcode:  bfop,
						Operand: nil,
					})
				}
			}

			if len(bracketStack) > 0 {
				panic(fmt.Sprintf(
					"TranslationError: Unmatched '[' — %d unclosed bracket(s)",
					len(bracketStack),
				))
			}

			instructions = append(instructions, vm.Instruction{
				Opcode:  OpHalt,
				Operand: nil,
			})

			return rf.Generate(true, false, vm.CodeObject{
				Instructions: instructions,
				Constants:    []interface{}{},
				Names:        []string{},
			})
		}).GetResult()
	return result
}
