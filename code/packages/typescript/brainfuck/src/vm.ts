/**
 * Brainfuck VM Factory -- Plugging Brainfuck Into the GenericVM.
 *
 * ==========================================================================
 * The Factory Pattern
 * ==========================================================================
 *
 * This module provides ``createBrainfuckVm()`` -- a factory function that
 * creates a GenericVM fully configured for Brainfuck. It:
 *
 * 1. Creates a fresh GenericVM instance.
 * 2. Attaches Brainfuck-specific state (tape, data pointer, input buffer).
 * 3. Registers all 9 opcode handlers.
 *
 * The result is a GenericVM that speaks Brainfuck -- same execution engine
 * as Starlark, different language semantics.
 *
 * ==========================================================================
 * Convenience Executor
 * ==========================================================================
 *
 * For simple use cases, ``executeBrainfuck()`` wraps the full pipeline:
 *
 *     const result = executeBrainfuck("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.");
 *
 * This translates the source, creates a VM, and executes in one call.
 *
 * ==========================================================================
 * Why Not a BrainfuckVM Subclass?
 * ==========================================================================
 *
 * You might wonder why we don't create a ``BrainfuckVM extends GenericVM``
 * class with typed ``tape``, ``dp``, etc. properties. The answer is
 * philosophical: the GenericVM's pluggable architecture is designed so that
 * *the same class* runs every language. Subclassing would defeat the purpose.
 *
 * Instead, we use the factory pattern to *configure* a GenericVM for
 * Brainfuck. The Brainfuck-specific state is attached dynamically, and
 * the handlers access it via ``(vm as any).tape`` etc. This mirrors the
 * Python version, which uses Python's dynamic attribute assignment.
 */

import type { VMTrace } from "@coding-adventures/virtual-machine";
import { GenericVM } from "@coding-adventures/virtual-machine";

import { HANDLERS, TAPE_SIZE } from "./handlers.js";
import { translate } from "./translator.js";

// =========================================================================
// Result type
// =========================================================================

/**
 * The result of executing a Brainfuck program.
 *
 * This bundles together everything you might want to know about a
 * completed execution: the program's output, the final tape state,
 * the final data pointer position, and the step-by-step execution
 * traces for debugging or visualization.
 */
export interface BrainfuckResult {
  /** The program's output (concatenation of all ``.`` commands). */
  readonly output: string;

  /**
   * The final state of the tape (all 30,000 cells).
   *
   * This is a copy, not a reference, so modifying it won't affect
   * anything. Useful for inspecting what the program computed.
   */
  readonly tape: readonly number[];

  /** The final data pointer position. */
  readonly dp: number;

  /**
   * Step-by-step execution traces (for debugging/visualization).
   *
   * Each entry records one instruction's execution: what the PC was,
   * what the stack looked like before and after, what variables changed.
   */
  readonly traces: readonly VMTrace[];

  /** Total number of instructions executed. */
  readonly steps: number;
}

// =========================================================================
// Factory function
// =========================================================================

/**
 * Create a GenericVM configured for Brainfuck execution.
 *
 * This is the factory function that wires up Brainfuck's handlers and
 * state. The returned VM is ready to execute any Brainfuck CodeObject.
 *
 * **What it does:**
 *
 * 1. Creates a new GenericVM instance.
 * 2. Attaches Brainfuck-specific state as dynamic properties:
 *    - ``tape``: an array of 30,000 zeros (the Brainfuck tape)
 *    - ``dp``: the data pointer, starting at 0
 *    - ``inputBuffer``: the string to read from for ``,`` commands
 *    - ``inputPos``: the current position in the input buffer
 * 3. Registers all 9 opcode handlers from ``HANDLERS``.
 *
 * @param inputData - Input to feed to ``,`` commands. Each character is
 *                    one byte. Default is empty (all ``,`` commands produce
 *                    0 / EOF).
 * @returns A GenericVM with Brainfuck handlers registered and tape initialized.
 *
 * @example
 * ```typescript
 * import { translate, createBrainfuckVm } from "@coding-adventures/brainfuck";
 *
 * const code = translate("+++.");
 * const vm = createBrainfuckVm();
 * const traces = vm.execute(code);
 * console.log(vm.output.join("")); // "\x03"
 * ```
 */
export function createBrainfuckVm(inputData: string = ""): GenericVM {
  const vm = new GenericVM();

  // -- Attach Brainfuck-specific state ----------------------------------
  // TypeScript doesn't allow adding arbitrary properties to a typed object,
  // so we use ``(vm as any)`` to attach the Brainfuck state. The handlers
  // access these properties the same way. This is the TypeScript equivalent
  // of Python's dynamic attribute assignment (``vm.tape = [0] * TAPE_SIZE``).
  (vm as any).tape = new Array<number>(TAPE_SIZE).fill(0);
  (vm as any).dp = 0;
  (vm as any).inputBuffer = inputData;
  (vm as any).inputPos = 0;

  // -- Register all opcode handlers -------------------------------------
  // Each handler is a function that knows how to execute one Brainfuck
  // opcode. The GenericVM's eval loop will dispatch to these handlers
  // based on the opcode number in each instruction.
  for (const [opcode, handler] of HANDLERS) {
    vm.registerOpcode(opcode, handler);
  }

  return vm;
}

// =========================================================================
// Convenience executor
// =========================================================================

/**
 * Translate and execute a Brainfuck program in one call.
 *
 * This is the convenience function for quick execution. It handles
 * the full pipeline: source -> translate -> create VM -> execute -> result.
 *
 * @param source - The Brainfuck source code.
 * @param inputData - Input bytes for ``,`` commands.
 * @returns The program's output, final tape state, and execution traces.
 *
 * @example
 * ```typescript
 * // Simple addition: 2 + 5 = 7
 * const result = executeBrainfuck("++>+++++[<+>-]");
 * console.log(result.tape[0]); // 7
 *
 * // Hello character (ASCII 72 = 'H')
 * const result2 = executeBrainfuck("+++++++++[>++++++++<-]>.");
 * console.log(result2.output); // "H"
 * ```
 */
export function executeBrainfuck(
  source: string,
  inputData: string = "",
): BrainfuckResult {
  const code = translate(source);
  const vm = createBrainfuckVm(inputData);
  const traces = vm.execute(code);

  return {
    output: vm.output.join(""),
    tape: [...(vm as any).tape],
    dp: (vm as any).dp,
    traces,
    steps: traces.length,
  };
}
