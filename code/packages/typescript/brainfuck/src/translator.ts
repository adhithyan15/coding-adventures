/**
 * Brainfuck Translator -- Source Code to Bytecode in One Pass.
 *
 * ==========================================================================
 * Why "Translator" and not "Compiler"?
 * ==========================================================================
 *
 * A compiler transforms a high-level *structured* representation (an AST)
 * into lower-level instructions. It handles scoping, type checking, operator
 * precedence, and all the complexity that comes with real languages.
 *
 * Brainfuck doesn't have any of that. There's no AST, no scoping, no types.
 * Each source character maps directly to one instruction. The only non-trivial
 * step is **bracket matching** -- connecting ``[`` to its matching ``]`` so the
 * VM knows where to jump.
 *
 * So we call this a "translator" rather than a "compiler": it translates
 * characters to opcodes, with bracket matching as the sole transformation.
 *
 * ==========================================================================
 * How Bracket Matching Works
 * ==========================================================================
 *
 * Bracket matching is a classic stack problem:
 *
 * 1. Scan the source left to right.
 * 2. When we see ``[``, emit a ``LOOP_START`` with a placeholder target (0),
 *    and push its instruction index onto a stack.
 * 3. When we see ``]``, pop the matching ``[`` index from the stack.
 *    - Patch the ``[`` instruction to jump to one past the current ``]``.
 *    - Emit a ``LOOP_END`` that jumps back to the ``[``.
 * 4. After scanning, if the stack isn't empty, we have unmatched ``[``.
 *
 * This is identical to how the GenericCompiler's ``emit_jump()`` /
 * ``patch_jump()`` work -- but we do it by hand since Brainfuck is too
 * simple to need the full compiler framework.
 *
 * ==========================================================================
 * Example
 * ==========================================================================
 *
 * Source: ``++[>+<-]``
 *
 * Translation:
 *
 *     Index  Opcode       Operand   Source
 *     -----------------------------------------
 *     0      INC          --        +
 *     1      INC          --        +
 *     2      LOOP_START   8         [  (jump to 8 if cell==0)
 *     3      RIGHT        --        >
 *     4      INC          --        +
 *     5      LEFT         --        <
 *     6      DEC          --        -
 *     7      LOOP_END     2         ]  (jump to 2 if cell!=0)
 *     8      HALT         --        (end)
 *
 * When cell 0 reaches zero, LOOP_START at index 2 jumps to index 8
 * (one past the LOOP_END). When the cell is still nonzero, LOOP_END
 * at index 7 jumps back to index 2.
 */

import type { Instruction, CodeObject } from "@coding-adventures/virtual-machine";

import { CHAR_TO_OP, Op } from "./opcodes.js";

// =========================================================================
// Error type
// =========================================================================

/**
 * Raised when the Brainfuck source has mismatched brackets.
 *
 * This is the only error that can occur during translation, since
 * Brainfuck's "syntax" consists entirely of bracket matching. Every
 * other character either maps to an opcode or is a comment -- there
 * are no other error conditions.
 */
export class TranslationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TranslationError";
  }
}

// =========================================================================
// The Translator
// =========================================================================

/**
 * Translate Brainfuck source code into a CodeObject.
 *
 * This function performs a single-pass translation from BF source to bytecode.
 * Each BF command character becomes one instruction. Non-command characters
 * are silently ignored (they're comments). The only complex step is bracket
 * matching, which uses a stack to pair ``[`` with ``]``.
 *
 * @param source - The Brainfuck program. Non-command characters are ignored.
 * @returns A CodeObject with instructions ready for the GenericVM.
 *          The constant and name pools are empty (Brainfuck has no
 *          variables or literals).
 * @throws TranslationError if brackets are mismatched.
 *
 * @example
 * ```typescript
 * const code = translate("+++.");
 * console.log(code.instructions.length); // 5 (3 INCs + 1 OUTPUT + 1 HALT)
 * console.log(code.constants);           // []
 * console.log(code.names);               // []
 * ```
 */
export function translate(source: string): CodeObject {
  /**
   * We build a mutable array of instructions. Entries for LOOP_START are
   * initially created with a placeholder operand of 0, then patched when
   * the matching ``]`` is found.
   *
   * Why mutable? Because we need to patch LOOP_START operands after the
   * fact (we don't know where ``]`` is until we reach it). Using a plain
   * array lets us index back and update in place.
   */
  const instructions: Instruction[] = [];

  /**
   * The bracket stack -- holds the instruction index of each unmatched ``[``.
   *
   * When we encounter ``[``, we push its index. When we encounter ``]``,
   * we pop and patch. If the stack is empty when we see ``]``, that bracket
   * has no match. If the stack is non-empty at the end, those ``[`` have
   * no match.
   *
   * This is the same algorithm used to check balanced parentheses in any
   * introductory CS course -- one of the most elegant uses of a stack.
   */
  const bracketStack: number[] = [];

  for (const char of source) {
    const op = CHAR_TO_OP.get(char);
    if (op === undefined) {
      // Not a Brainfuck command -- skip (it's a comment).
      // This is why Brainfuck programs can contain arbitrary text:
      // everything except ><+-.,[] is a comment.
      continue;
    }

    if (op === Op.LOOP_START) {
      // Emit LOOP_START with a placeholder operand (will be patched
      // when we find the matching ``]``).
      const index = instructions.length;
      instructions.push({ opcode: Op.LOOP_START, operand: 0 });
      bracketStack.push(index);
    } else if (op === Op.LOOP_END) {
      if (bracketStack.length === 0) {
        throw new TranslationError(
          "Unmatched ']' -- no matching '[' found",
        );
      }

      // Pop the matching [ index.
      const startIndex = bracketStack.pop()!;

      // The LOOP_END instruction index.
      const endIndex = instructions.length;

      // Patch LOOP_START to jump past LOOP_END (endIndex + 1).
      // This is the "skip the loop" target: if the cell is zero when
      // we reach [, we jump to the instruction *after* the matching ].
      instructions[startIndex] = {
        opcode: Op.LOOP_START,
        operand: endIndex + 1,
      };

      // Emit LOOP_END that jumps back to LOOP_START.
      // This is the "loop again" target: if the cell is nonzero when
      // we reach ], we jump back to [ to re-test the condition.
      instructions.push({ opcode: Op.LOOP_END, operand: startIndex });
    } else {
      // Simple command -- no operand needed.
      // These instructions (RIGHT, LEFT, INC, DEC, OUTPUT, INPUT) don't
      // need an operand because their behavior is fully determined by
      // the opcode alone.
      instructions.push({ opcode: op });
    }
  }

  // After processing all characters, check for unmatched ``[``.
  if (bracketStack.length > 0) {
    throw new TranslationError(
      `Unmatched '[' -- ${bracketStack.length} unclosed bracket(s)`,
    );
  }

  // Append HALT to mark the end of the program.
  // Without HALT, the VM would run off the end of the instruction array
  // and stop naturally -- but an explicit HALT is cleaner and produces
  // a proper trace entry.
  instructions.push({ opcode: Op.HALT });

  return {
    instructions,
    constants: [],
    names: [],
  };
}
