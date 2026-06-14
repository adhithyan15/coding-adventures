/**
 * Brainfuck Interpreter -- Proving the Pluggable VM Works.
 *
 * ==========================================================================
 * What is Brainfuck?
 * ==========================================================================
 *
 * Brainfuck is a minimalist, Turing-complete programming language created by
 * Urban Mueller in 1993. It has only 8 commands -- each a single character --
 * yet it can compute anything a full programming language can.
 *
 * The entire language operates on:
 *
 * - A **tape** of 30,000 byte cells, all initialized to zero
 * - A **data pointer** that starts at cell 0
 * - An **input stream** and **output stream**
 *
 * The 8 commands:
 *
 *     >   Move the data pointer right
 *     <   Move the data pointer left
 *     +   Increment the byte at the data pointer
 *     -   Decrement the byte at the data pointer
 *     .   Output the byte at the data pointer as ASCII
 *     ,   Read one byte of input into the current cell
 *     [   If the byte at the data pointer is zero, jump past the matching ]
 *     ]   If the byte at the data pointer is nonzero, jump back to the matching [
 *
 * Everything else in the source code is ignored (treated as comments).
 *
 * ==========================================================================
 * Why Brainfuck for the Pluggable VM?
 * ==========================================================================
 *
 * Brainfuck is the **perfect test case** for our pluggable GenericVM architecture.
 * It's radically different from Starlark:
 *
 * - Starlark has 50+ opcodes; Brainfuck has 8
 * - Starlark has variables, functions, collections; Brainfuck has a tape
 * - Starlark needs a full parser and compiler; Brainfuck translates directly
 * - Starlark is a "real" language; Brainfuck is an esoteric toy
 *
 * If the same GenericVM chassis can run both Starlark and Brainfuck, the
 * pluggable design is validated. Different engines, same car.
 *
 * ==========================================================================
 * Package Structure
 * ==========================================================================
 *
 * - ``opcodes.ts`` -- The 9 Brainfuck opcodes (8 commands + HALT)
 * - ``translator.ts`` -- Source code -> CodeObject (no parser needed)
 * - ``handlers.ts`` -- Opcode handler functions registered with GenericVM
 * - ``vm.ts`` -- Factory function and convenience executor
 */

export { Op, CHAR_TO_OP } from "./opcodes.js";
export type { OpValue } from "./opcodes.js";
export { translate, TranslationError } from "./translator.js";
export { BrainfuckError, HANDLERS, TAPE_SIZE } from "./handlers.js";
export { createBrainfuckVm, executeBrainfuck } from "./vm.js";
export type { BrainfuckResult } from "./vm.js";
export { tokenizeBrainfuck } from "./lexer.js";
export { parseBrainfuck } from "./parser.js";
export { compileToIir, executeOnLangVm } from "./lang-vm.js";
export type { BrainfuckLangVmResult } from "./lang-vm.js";
