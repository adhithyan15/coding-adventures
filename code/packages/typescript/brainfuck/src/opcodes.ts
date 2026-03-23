/**
 * Brainfuck Opcodes -- The Simplest Instruction Set.
 *
 * ==========================================================================
 * From 8 Characters to 9 Opcodes
 * ==========================================================================
 *
 * Brainfuck has 8 commands. We map each to a numeric opcode, plus HALT to
 * mark the end of the program. These opcodes are registered with the GenericVM
 * via ``registerOpcode()``.
 *
 * Why numeric opcodes instead of characters? Because the GenericVM dispatches
 * on integers -- it's a *bytecode* interpreter, not a character interpreter.
 * This also means the same GenericVM that runs Starlark's 0x01-0xFF opcodes
 * can run Brainfuck's 0x01-0x08 opcodes. Different opcode *numbers*, different
 * *handlers*, same execution engine.
 *
 * ==========================================================================
 * Opcode Table
 * ==========================================================================
 *
 *     Opcode       Hex    BF    Stack Effect    Description
 *     -----------------------------------------------------------
 *     RIGHT        0x01   >     --              Move data pointer right
 *     LEFT         0x02   <     --              Move data pointer left
 *     INC          0x03   +     --              Increment current cell
 *     DEC          0x04   -     --              Decrement current cell
 *     OUTPUT       0x05   .     --              Print cell as ASCII
 *     INPUT        0x06   ,     --              Read byte into cell
 *     LOOP_START   0x07   [     --              Jump forward if cell == 0
 *     LOOP_END     0x08   ]     --              Jump backward if cell != 0
 *     HALT         0xFF   --    --              Stop execution
 *
 * Note that Brainfuck opcodes have **no stack effect**. Unlike Starlark's
 * stack-based arithmetic (push, push, add, pop result), Brainfuck operates
 * entirely on the tape. The GenericVM's operand stack goes unused -- but
 * it's still there, available if a future language needs it.
 */

// =========================================================================
// Opcode Constants
// =========================================================================

/**
 * Brainfuck opcodes.
 *
 * Each opcode corresponds to one of the 8 Brainfuck commands, plus
 * HALT to mark end-of-program. The numeric values are arbitrary but
 * chosen to avoid collision with Starlark's opcode space (which starts
 * at 0x01 for LOAD_CONST). In practice, each language plugin gets its
 * own GenericVM instance, so collisions don't matter -- but distinct
 * values make debugging clearer.
 *
 * We use a ``const`` object (with ``as const``) instead of a TypeScript
 * enum. This produces cleaner JavaScript output and avoids the quirks
 * of TypeScript's numeric enums (like reverse mappings). The ``as const``
 * assertion makes every value a literal type, so ``Op.RIGHT`` is typed
 * as ``0x01``, not ``number``.
 */
export const Op = {
  // -- Pointer movement --------------------------------------------------

  /** ``>`` -- Move the data pointer one cell to the right. */
  RIGHT: 0x01,

  /** ``<`` -- Move the data pointer one cell to the left. */
  LEFT: 0x02,

  // -- Cell modification -------------------------------------------------

  /** ``+`` -- Increment the byte at the data pointer (wraps 255 -> 0). */
  INC: 0x03,

  /** ``-`` -- Decrement the byte at the data pointer (wraps 0 -> 255). */
  DEC: 0x04,

  // -- I/O ---------------------------------------------------------------

  /** ``.`` -- Output the byte at the data pointer as an ASCII character. */
  OUTPUT: 0x05,

  /** ``,`` -- Read one byte of input into the current cell. */
  INPUT: 0x06,

  // -- Control flow ------------------------------------------------------

  /**
   * ``[`` -- If the current cell is zero, jump forward past the matching ``]``.
   *
   * The operand contains the instruction index to jump to (one past the
   * matching LOOP_END). If the cell is nonzero, execution falls through
   * to the next instruction.
   *
   * This is how Brainfuck implements loops: ``[`` and ``]`` form a
   * while-loop that repeats as long as the current cell is nonzero.
   */
  LOOP_START: 0x07,

  /**
   * ``]`` -- If the current cell is nonzero, jump backward to the matching ``[``.
   *
   * The operand contains the instruction index of the matching LOOP_START.
   * If the cell is zero, execution falls through (exiting the loop).
   */
  LOOP_END: 0x08,

  // -- VM control --------------------------------------------------------

  /** End of program -- stop the VM. */
  HALT: 0xff,
} as const;

/**
 * The type of any Brainfuck opcode value.
 *
 * This is a union of all literal opcode values: ``0x01 | 0x02 | ... | 0xFF``.
 * It provides type safety: you can't accidentally pass an arbitrary number
 * where an opcode is expected.
 */
export type OpValue = (typeof Op)[keyof typeof Op];

// =========================================================================
// Character-to-opcode mapping
// =========================================================================

/**
 * Maps each Brainfuck character to its opcode.
 *
 * Characters not in this map are ignored (they're comments). This is one
 * of Brainfuck's most charming features: any text that isn't one of the
 * 8 command characters is simply a comment. You can write entire essays
 * between commands and the interpreter won't care.
 *
 * Example:
 *     ``CHAR_TO_OP.get(">")``  -> ``Op.RIGHT`` (0x01)
 *     ``CHAR_TO_OP.get("a")``  -> ``undefined`` (comment)
 */
export const CHAR_TO_OP: ReadonlyMap<string, OpValue> = new Map<string, OpValue>([
  [">", Op.RIGHT],
  ["<", Op.LEFT],
  ["+", Op.INC],
  ["-", Op.DEC],
  [".", Op.OUTPUT],
  [",", Op.INPUT],
  ["[", Op.LOOP_START],
  ["]", Op.LOOP_END],
]);
