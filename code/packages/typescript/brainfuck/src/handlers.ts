/**
 * Brainfuck Opcode Handlers -- Teaching the GenericVM a New Language.
 *
 * ==========================================================================
 * How Handlers Plug Into the GenericVM
 * ==========================================================================
 *
 * The GenericVM is a blank slate -- it knows how to fetch-decode-execute
 * instructions, but it doesn't know what any opcode *means*. That's where
 * handlers come in.
 *
 * Each handler is a function with the signature:
 *
 *     (vm: GenericVM, instruction: Instruction, code: CodeObject) => string | null
 *
 * The handler receives:
 *
 * - **vm** -- The GenericVM instance. We use Brainfuck-specific state that
 *   has been attached to the VM instance (tape, dp, inputBuffer, inputPos)
 *   plus the VM's built-in methods (advancePc, jumpTo, output).
 * - **instruction** -- The current instruction (opcode + optional operand).
 * - **code** -- The CodeObject (unused by most Brainfuck handlers, since
 *   Brainfuck has no constant or name pools).
 *
 * The handler returns a string if it produces output (the ``.`` command),
 * otherwise null.
 *
 * ==========================================================================
 * Brainfuck's Extra State
 * ==========================================================================
 *
 * The GenericVM provides a stack, variables, and locals -- none of which
 * Brainfuck uses. Instead, Brainfuck needs:
 *
 * - **tape** -- An array of 30,000 byte cells, initialized to 0.
 * - **dp** (data pointer) -- Index into the tape, starts at 0.
 * - **inputBuffer** -- String to read from (simulates stdin).
 * - **inputPos** -- Current position in the input buffer.
 *
 * These are attached as properties on the GenericVM instance in the
 * factory function (``createBrainfuckVm()``). TypeScript doesn't allow
 * adding arbitrary properties to a typed object, so we use ``(vm as any)``
 * to access them. This is the same approach used in the Python version
 * with dynamic attribute assignment.
 *
 * ==========================================================================
 * Cell Wrapping
 * ==========================================================================
 *
 * Brainfuck cells are unsigned bytes: values 0--255. Incrementing 255
 * wraps to 0; decrementing 0 wraps to 255. This is modular arithmetic:
 *
 *     cell = (cell + 1) % 256   // INC
 *     cell = (cell + 255) % 256 // DEC
 *
 * **Why +255 instead of -1?** JavaScript's ``%`` operator is a *remainder*,
 * not a true *modulo*. For negative numbers, ``(-1) % 256 === -1`` in JS,
 * but we want 255. Adding 255 instead of subtracting 1 avoids the negative
 * modulo problem entirely:
 *
 *     (0 + 255) % 256 === 255  // correct!
 *     (0 - 1) % 256 === -1     // wrong in JS!
 *
 * Python's ``%`` handles this correctly (``(-1) % 256 == 255``), but
 * JavaScript, Ruby, C, and many other languages don't. This is a classic
 * porting gotcha.
 */

import type { GenericVM, OpcodeHandler } from "@coding-adventures/virtual-machine";
import type { Instruction, CodeObject } from "@coding-adventures/virtual-machine";

import { Op } from "./opcodes.js";

// =========================================================================
// Tape size constant
// =========================================================================

/**
 * The number of cells on the Brainfuck tape.
 *
 * The original Brainfuck specification uses 30,000 cells. Some implementations
 * use more (or even dynamically grow), but 30,000 is the classic size. At one
 * byte per cell, this is about 30 KB of memory -- tiny by modern standards,
 * but enough for Turing completeness.
 */
export const TAPE_SIZE = 30_000;

// =========================================================================
// Error type
// =========================================================================

/**
 * Runtime error during Brainfuck execution.
 *
 * This is thrown when a Brainfuck program does something illegal at runtime,
 * such as moving the data pointer past the boundaries of the tape. It's
 * distinct from TranslationError (which is a compile-time error for bad
 * bracket matching).
 */
export class BrainfuckError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BrainfuckError";
  }
}

// =========================================================================
// Helper type for accessing BF-specific state on the GenericVM
// =========================================================================

/**
 * We need to read/write Brainfuck-specific properties (tape, dp, etc.) that
 * are dynamically attached to the GenericVM instance. Since TypeScript's type
 * system doesn't know about these properties, we cast ``vm`` to ``any`` when
 * accessing them. This helper type documents what properties we expect.
 *
 * In a more type-safe design, we'd create a BrainfuckVM subclass. But the
 * whole point of the GenericVM architecture is that the *same class* runs
 * different languages -- no subclassing needed. The trade-off is these
 * ``as any`` casts, which are confined to this file.
 */
interface BfState {
  tape: number[];
  dp: number;
  inputBuffer: string;
  inputPos: number;
}

/** Safely access the Brainfuck-specific state from a GenericVM instance. */
function bf(vm: GenericVM): BfState {
  return vm as unknown as BfState;
}

// =========================================================================
// Pointer movement handlers
// =========================================================================

/**
 * ``>`` -- Move the data pointer one cell to the right.
 *
 * If the pointer is already at the last cell (index 29,999), this raises
 * a BrainfuckError. Some Brainfuck implementations wrap around; we choose
 * to error because silent wrapping hides bugs in BF programs.
 *
 * After moving, we advance the PC to the next instruction.
 */
const handleRight: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  bf(vm).dp += 1;
  if (bf(vm).dp >= TAPE_SIZE) {
    throw new BrainfuckError(
      `Data pointer moved past end of tape (position ${bf(vm).dp}). ` +
      `The tape has ${TAPE_SIZE} cells (indices 0--${TAPE_SIZE - 1}).`,
    );
  }
  vm.advancePc();
  return null;
};

/**
 * ``<`` -- Move the data pointer one cell to the left.
 *
 * If the pointer is already at cell 0, this raises a BrainfuckError.
 */
const handleLeft: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  bf(vm).dp -= 1;
  if (bf(vm).dp < 0) {
    throw new BrainfuckError(
      "Data pointer moved before start of tape (position -1). " +
      "The tape starts at index 0.",
    );
  }
  vm.advancePc();
  return null;
};

// =========================================================================
// Cell modification handlers
// =========================================================================

/**
 * ``+`` -- Increment the byte at the data pointer.
 *
 * Wraps from 255 to 0 (unsigned byte arithmetic). We use modulo 256
 * to enforce this wrapping.
 */
const handleInc: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  const state = bf(vm);
  state.tape[state.dp] = (state.tape[state.dp] + 1) % 256;
  vm.advancePc();
  return null;
};

/**
 * ``-`` -- Decrement the byte at the data pointer.
 *
 * Wraps from 0 to 255 (unsigned byte arithmetic). We add 255 instead
 * of subtracting 1 to avoid JavaScript's negative modulo behavior:
 *
 *     (0 - 1) % 256 === -1   // WRONG in JavaScript!
 *     (0 + 255) % 256 === 255 // Correct!
 *
 * This is mathematically equivalent: subtracting 1 mod 256 is the same
 * as adding 255 mod 256, because 255 === -1 (mod 256).
 */
const handleDec: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  const state = bf(vm);
  state.tape[state.dp] = (state.tape[state.dp] + 255) % 256;
  vm.advancePc();
  return null;
};

// =========================================================================
// I/O handlers
// =========================================================================

/**
 * ``.`` -- Output the current cell's value as an ASCII character.
 *
 * The character is appended to ``vm.output`` (the GenericVM's output
 * capture list) and also returned as the handler's output string.
 * This dual output mechanism allows both programmatic inspection
 * (via ``vm.output``) and trace recording (via the return value).
 */
const handleOutput: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  const state = bf(vm);
  const char = String.fromCharCode(state.tape[state.dp]);
  vm.output.push(char);
  vm.advancePc();
  return char;
};

/**
 * ``,`` -- Read one byte of input into the current cell.
 *
 * Reads from ``vm.inputBuffer`` at position ``vm.inputPos``.
 * If the input is exhausted (EOF), the cell is set to 0.
 *
 * Different Brainfuck implementations handle EOF differently:
 * - Set cell to 0 (our choice -- clean and predictable)
 * - Set cell to -1 (255 in unsigned)
 * - Leave cell unchanged
 *
 * We chose 0 because it makes the common ``,[.,]`` (cat program)
 * pattern work naturally: the loop exits when EOF produces 0.
 */
const handleInput: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  const state = bf(vm);
  if (state.inputPos < state.inputBuffer.length) {
    state.tape[state.dp] = state.inputBuffer.charCodeAt(state.inputPos);
    state.inputPos += 1;
  } else {
    // EOF: set cell to 0
    state.tape[state.dp] = 0;
  }
  vm.advancePc();
  return null;
};

// =========================================================================
// Control flow handlers
// =========================================================================

/**
 * ``[`` -- Jump forward past the matching ``]`` if the current cell is zero.
 *
 * If the cell is **nonzero**, execution continues to the next instruction
 * (entering the loop body). If the cell is **zero**, the VM jumps to the
 * instruction index stored in the operand (one past the matching ``]``),
 * effectively skipping the loop entirely.
 *
 * This is the "while" test: ``while (tape[dp] !== 0) { ... }``
 *
 * The operand is set by the translator during bracket matching. It always
 * points to the instruction after the matching LOOP_END.
 */
const handleLoopStart: OpcodeHandler = (
  vm: GenericVM,
  instruction: Instruction,
  _code: CodeObject,
): string | null => {
  if (bf(vm).tape[bf(vm).dp] === 0) {
    // Cell is zero -- skip the loop.
    vm.jumpTo(instruction.operand as number);
  } else {
    // Cell is nonzero -- enter the loop.
    vm.advancePc();
  }
  return null;
};

/**
 * ``]`` -- Jump backward to the matching ``[`` if the current cell is nonzero.
 *
 * If the cell is **nonzero**, jump back to the matching ``[`` (which will
 * re-test the condition). If the cell is **zero**, fall through to the next
 * instruction (exiting the loop).
 *
 * Together with LOOP_START, this implements:
 *
 *     while (tape[dp] !== 0) {
 *         <loop body>
 *     }
 */
const handleLoopEnd: OpcodeHandler = (
  vm: GenericVM,
  instruction: Instruction,
  _code: CodeObject,
): string | null => {
  if (bf(vm).tape[bf(vm).dp] !== 0) {
    // Cell is nonzero -- loop again.
    vm.jumpTo(instruction.operand as number);
  } else {
    // Cell is zero -- exit loop.
    vm.advancePc();
  }
  return null;
};

// =========================================================================
// HALT handler
// =========================================================================

/**
 * Stop the VM.
 *
 * Sets ``vm.halted = true``, which causes the eval loop in
 * ``GenericVM.execute()`` to stop. The PC is not advanced -- there's
 * nowhere to go after halting.
 */
const handleHalt: OpcodeHandler = (
  vm: GenericVM,
  _instruction: Instruction,
  _code: CodeObject,
): string | null => {
  vm.halted = true;
  return null;
};

// =========================================================================
// Handler registry -- maps opcode numbers to handler functions
// =========================================================================

/**
 * All Brainfuck opcode handlers, keyed by opcode number.
 *
 * Used by ``createBrainfuckVm()`` to register all handlers at once.
 * Each entry maps an opcode (like ``Op.RIGHT = 0x01``) to the function
 * that implements it (like ``handleRight``).
 *
 * Having all handlers in one registry makes it easy to:
 * 1. Register them all with the GenericVM in a single loop.
 * 2. Test that all expected opcodes are covered.
 * 3. See the complete "language definition" in one place.
 */
export const HANDLERS: ReadonlyMap<number, OpcodeHandler> = new Map<number, OpcodeHandler>([
  [Op.RIGHT, handleRight],
  [Op.LEFT, handleLeft],
  [Op.INC, handleInc],
  [Op.DEC, handleDec],
  [Op.OUTPUT, handleOutput],
  [Op.INPUT, handleInput],
  [Op.LOOP_START, handleLoopStart],
  [Op.LOOP_END, handleLoopEnd],
  [Op.HALT, handleHalt],
]);
