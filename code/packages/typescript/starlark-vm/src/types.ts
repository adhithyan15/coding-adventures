/**
 * Starlark VM Types -- Type definitions for the Starlark virtual machine.
 *
 * ==========================================================================
 * Chapter 1: Why Separate Types?
 * ==========================================================================
 *
 * This module defines the core types used throughout the Starlark VM:
 *
 * - **StarlarkFunction** -- A user-defined function created by ``def`` statements.
 * - **StarlarkIterator** -- A wrapper around JavaScript iterators for ``for`` loops.
 * - **StarlarkResult** -- The result of executing a Starlark program.
 * - **Op** -- The opcode enumeration (46 opcodes organized by category).
 *
 * These types are the "vocabulary" of the VM -- every handler, builtin, and
 * the factory function all speak in terms of these types.
 *
 * ==========================================================================
 * Chapter 2: Starlark's Type System
 * ==========================================================================
 *
 * Starlark is dynamically typed with a small, well-defined set of types:
 *
 * | Starlark Type | TypeScript Representation        |
 * |---------------|----------------------------------|
 * | int           | number (integer values)          |
 * | float         | number (decimal values)          |
 * | string        | string                           |
 * | bool          | boolean                          |
 * | None          | null                             |
 * | list          | unknown[]                        |
 * | dict          | Map or Record<string, unknown>   |
 * | tuple         | readonly unknown[] (as array)    |
 * | function      | StarlarkFunction                 |
 *
 * Since TypeScript's ``number`` type conflates int and float, we use
 * ``Number.isInteger()`` to distinguish them at runtime when needed
 * (e.g., for ``type()`` to return "int" vs "float").
 *
 * @module
 */

import type { CodeObject, VMTrace, VMValue } from "@coding-adventures/virtual-machine";

// =========================================================================
// Opcodes -- The Starlark bytecode instruction set
// =========================================================================

/**
 * Starlark bytecode opcodes.
 *
 * Each value is a single byte (0x00-0xFF). The high nibble groups opcodes
 * by category, making it easy to identify an instruction's purpose at a glance:
 *
 *     0x0_ = Stack operations      (push, pop, dup, load constants)
 *     0x1_ = Variable operations   (store/load by name or slot)
 *     0x2_ = Arithmetic            (add, sub, mul, div, bitwise)
 *     0x3_ = Comparison & boolean  (==, !=, <, >, in, not)
 *     0x4_ = Control flow          (jump, branch)
 *     0x5_ = Functions             (make, call, return)
 *     0x6_ = Collections           (build list, dict, tuple)
 *     0x7_ = Subscript & attribute (indexing, slicing, dot access)
 *     0x8_ = Iteration             (get_iter, for_iter, unpack)
 *     0x9_ = Module                (load statement)
 *     0xA_ = I/O                   (print)
 *     0xF_ = VM control            (halt)
 *
 * This mirrors the JVM's opcode organization and the Python reference
 * implementation's ``opcodes.py``.
 */
export const Op = {
  // Stack Operations (0x0_)
  LOAD_CONST: 0x01,
  POP: 0x02,
  DUP: 0x03,
  LOAD_NONE: 0x04,
  LOAD_TRUE: 0x05,
  LOAD_FALSE: 0x06,

  // Variable Operations (0x1_)
  STORE_NAME: 0x10,
  LOAD_NAME: 0x11,
  STORE_LOCAL: 0x12,
  LOAD_LOCAL: 0x13,
  STORE_CLOSURE: 0x14,
  LOAD_CLOSURE: 0x15,

  // Arithmetic Operations (0x2_)
  ADD: 0x20,
  SUB: 0x21,
  MUL: 0x22,
  DIV: 0x23,
  FLOOR_DIV: 0x24,
  MOD: 0x25,
  POWER: 0x26,
  NEGATE: 0x27,
  BIT_AND: 0x28,
  BIT_OR: 0x29,
  BIT_XOR: 0x2a,
  BIT_NOT: 0x2b,
  LSHIFT: 0x2c,
  RSHIFT: 0x2d,

  // Comparison Operations (0x3_)
  CMP_EQ: 0x30,
  CMP_NE: 0x31,
  CMP_LT: 0x32,
  CMP_GT: 0x33,
  CMP_LE: 0x34,
  CMP_GE: 0x35,
  CMP_IN: 0x36,
  CMP_NOT_IN: 0x37,

  // Boolean Operations (0x38)
  NOT: 0x38,

  // Control Flow (0x4_)
  JUMP: 0x40,
  JUMP_IF_FALSE: 0x41,
  JUMP_IF_TRUE: 0x42,
  JUMP_IF_FALSE_OR_POP: 0x43,
  JUMP_IF_TRUE_OR_POP: 0x44,

  // Function Operations (0x5_)
  MAKE_FUNCTION: 0x50,
  CALL_FUNCTION: 0x51,
  CALL_FUNCTION_KW: 0x52,
  RETURN: 0x53,

  // Collection Operations (0x6_)
  BUILD_LIST: 0x60,
  BUILD_DICT: 0x61,
  BUILD_TUPLE: 0x62,
  LIST_APPEND: 0x63,
  DICT_SET: 0x64,

  // Subscript & Attribute Operations (0x7_)
  LOAD_SUBSCRIPT: 0x70,
  STORE_SUBSCRIPT: 0x71,
  LOAD_ATTR: 0x72,
  STORE_ATTR: 0x73,
  LOAD_SLICE: 0x74,

  // Iteration Operations (0x8_)
  GET_ITER: 0x80,
  FOR_ITER: 0x81,
  UNPACK_SEQUENCE: 0x82,

  // Module Operations (0x9_)
  LOAD_MODULE: 0x90,
  IMPORT_FROM: 0x91,

  // I/O Operations (0xA_)
  PRINT: 0xa0,

  // VM Control (0xF_)
  HALT: 0xff,
} as const;

/** The type of any opcode value from the Op enum. */
export type OpValue = (typeof Op)[keyof typeof Op];

// =========================================================================
// StarlarkFunction -- User-defined functions
// =========================================================================

/**
 * A user-defined Starlark function.
 *
 * Created by the ``MAKE_FUNCTION`` opcode handler when the VM encounters
 * a ``def`` statement. Contains:
 *
 * - The function's compiled body (a CodeObject)
 * - Parameter metadata (names, count)
 * - Default parameter values
 *
 * This is analogous to Python's ``PyFunctionObject`` or the JVM's
 * ``MethodHandle`` -- it's a first-class value that can be stored in
 * variables, passed as arguments, and called later.
 *
 * Example of how a function flows through the system:
 *
 * ```
 *   Source:    def add(x, y): return x + y
 *
 *   Compiler:  LOAD_CONST <CodeObject for add>
 *              LOAD_CONST ("x", "y")       # param names
 *              MAKE_FUNCTION 0x08          # flag: has param_names
 *              STORE_NAME "add"
 *
 *   VM:        MAKE_FUNCTION creates a StarlarkFunction and pushes it.
 *              STORE_NAME binds it to the name "add".
 *              Later, CALL_FUNCTION pops it and executes its code.
 * ```
 */
export class StarlarkFunction {
  /** The compiled bytecode for this function's body. */
  readonly code: CodeObject;

  /** Default values for parameters (right-to-left). */
  readonly defaults: VMValue[];

  /** The function's name (e.g., "add", or "<lambda>" for lambdas). */
  readonly name: string;

  /** How many parameters this function expects. */
  readonly paramCount: number;

  /** Ordered parameter names (e.g., ["x", "y", "z"]). */
  readonly paramNames: string[];

  constructor(
    code: CodeObject,
    defaults: VMValue[] = [],
    name: string = "<lambda>",
    paramCount: number = 0,
    paramNames: string[] = [],
  ) {
    this.code = code;
    this.defaults = defaults;
    this.name = name;
    this.paramCount = paramCount;
    this.paramNames = paramNames;
  }

  toString(): string {
    return `<function ${this.name}>`;
  }
}

// =========================================================================
// StarlarkIterator -- For-loop iteration
// =========================================================================

/**
 * Wraps a JavaScript iterator for use in the Starlark VM.
 *
 * Starlark's ``for`` loops use an iterator protocol (same as Python's):
 *
 * 1. ``GET_ITER`` -- Convert an iterable (list, dict, string, etc.) to
 *    a ``StarlarkIterator``.
 * 2. ``FOR_ITER`` -- Call ``next()`` on the iterator to get the next value.
 *    When the iterator is exhausted (returns ``done: true``), the loop jumps
 *    to its end.
 *
 * We use JavaScript's built-in iterator protocol under the hood. Arrays,
 * strings, Maps, and Sets all implement ``Symbol.iterator``, so we can
 * wrap them uniformly.
 *
 * Example:
 *
 * ```
 *   for x in [1, 2, 3]:   # GET_ITER converts [1,2,3] to StarlarkIterator
 *       print(x)           # FOR_ITER yields 1, then 2, then 3, then jumps out
 * ```
 */
export class StarlarkIterator {
  /** The underlying JavaScript iterator. */
  private readonly iterator: Iterator<VMValue>;

  /** Whether the iterator has been exhausted. */
  private done: boolean = false;

  constructor(iterable: Iterable<VMValue>) {
    this.iterator = iterable[Symbol.iterator]();
  }

  /**
   * Get the next value from the iterator.
   *
   * Returns ``{ value, done }`` following the JavaScript iterator protocol.
   * When ``done`` is true, the iterator is exhausted and FOR_ITER should
   * jump to the loop's end.
   */
  next(): IteratorResult<VMValue> {
    if (this.done) {
      return { value: undefined, done: true };
    }
    const result = this.iterator.next();
    if (result.done) {
      this.done = true;
    }
    return result;
  }

  toString(): string {
    return "<starlark_iterator>";
  }
}

// =========================================================================
// StarlarkResult -- Execution output
// =========================================================================

/**
 * The result of executing a Starlark program.
 *
 * Contains all the information about the execution:
 *
 * - **variables** -- The final state of all named variables after execution.
 *   This is how you inspect what the program computed.
 *
 * - **output** -- Captured ``print()`` output, one entry per ``print()`` call.
 *   Instead of going to stdout, all output is captured here for easy inspection.
 *
 * - **traces** -- Step-by-step execution trace (for debugging/visualization).
 *   Each trace captures the state before and after one instruction.
 *
 * Example:
 *
 * ```typescript
 *   const result = executeStarlark("x = 1 + 2\nprint(x)\n");
 *   result.variables["x"]  // 3
 *   result.output           // ["3"]
 *   result.traces.length    // ~6 instructions
 * ```
 */
export interface StarlarkResult {
  /** Final variable state after execution. */
  readonly variables: Record<string, VMValue>;

  /** Captured print output, one entry per print() call. */
  readonly output: string[];

  /** Step-by-step execution trace. */
  readonly traces: VMTrace[];
}

// =========================================================================
// Helper: Starlark truthiness
// =========================================================================

/**
 * Determine if a value is truthy in Starlark.
 *
 * Starlark's truthiness rules follow Python exactly:
 *
 * | Value                            | Truthy? |
 * |----------------------------------|---------|
 * | null (None)                      | false   |
 * | false                            | false   |
 * | 0, 0.0                           | false   |
 * | "" (empty string)                | false   |
 * | [] (empty list)                  | false   |
 * | {} (empty dict)                  | false   |
 * | () (empty tuple, represented []) | false   |
 * | Everything else                  | true    |
 *
 * This function is used by control flow handlers (JUMP_IF_FALSE, NOT, etc.)
 * and by the ``bool()`` builtin.
 */
export function isTruthy(value: VMValue): boolean {
  if (value === null || value === undefined) return false;
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value !== 0;
  if (typeof value === "string") return value.length > 0;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object" && value !== null) {
    // Dict (plain object) -- check if it has any keys
    if (value.constructor === Object) {
      return Object.keys(value as Record<string, unknown>).length > 0;
    }
    // Map (for dict representation)
    if (value instanceof Map) return value.size > 0;
  }
  return true;
}

// =========================================================================
// Helper: Starlark type name
// =========================================================================

/**
 * Get the Starlark type name of a value.
 *
 * Unlike Python's ``type()`` which returns a type object, Starlark's
 * ``type()`` returns a plain string. This maps JavaScript/TypeScript
 * runtime types to their Starlark names.
 *
 * Examples:
 *   starlarkTypeName(42)        --> "int"
 *   starlarkTypeName(3.14)      --> "float"
 *   starlarkTypeName("hello")   --> "string"
 *   starlarkTypeName([1, 2])    --> "list"
 *   starlarkTypeName(null)      --> "NoneType"
 */
export function starlarkTypeName(value: VMValue): string {
  if (value === null || value === undefined) return "NoneType";
  if (typeof value === "boolean") return "bool";
  if (typeof value === "number") {
    return Number.isInteger(value) ? "int" : "float";
  }
  if (typeof value === "string") return "string";
  if (Array.isArray(value)) return "list";
  if (value instanceof StarlarkFunction) return "function";
  if (value instanceof StarlarkIterator) return "iterator";
  if (typeof value === "object") {
    // CodeObject check
    if ("instructions" in value && "constants" in value) return "code";
    return "dict";
  }
  return typeof value;
}

// =========================================================================
// Helper: Starlark repr
// =========================================================================

/**
 * Format a value for Starlark print output.
 *
 * Starlark's print representation follows Python conventions:
 * - Strings are printed without quotes (``print("hi")`` outputs ``hi``)
 * - None prints as ``None``
 * - Booleans print as ``True``/``False``
 * - Lists print as ``[1, 2, 3]``
 * - Dicts print as ``{"a": 1}``
 */
export function starlarkRepr(value: VMValue): string {
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "string") return value; // print() shows strings without quotes
  if (typeof value === "number") return String(value);
  if (Array.isArray(value)) {
    const items = value.map((v) => starlarkValueRepr(v));
    return `[${items.join(", ")}]`;
  }
  if (typeof value === "object" && value !== null) {
    if (value instanceof StarlarkFunction) return value.toString();
    if (value instanceof StarlarkIterator) return value.toString();
    // Dict
    const entries = Object.entries(value as Record<string, unknown>).map(
      ([k, v]) => `${starlarkValueRepr(k)}: ${starlarkValueRepr(v)}`,
    );
    return `{${entries.join(", ")}}`;
  }
  return String(value);
}

/**
 * Format a value as a Starlark repr (with quotes around strings).
 *
 * Unlike ``starlarkRepr()`` which formats for ``print()`` (no quotes on strings),
 * this formats for ``repr()`` and for values inside containers (strings get quotes).
 */
export function starlarkValueRepr(value: VMValue): string {
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "string") return `"${value}"`;
  if (typeof value === "number") return String(value);
  if (Array.isArray(value)) {
    const items = value.map((v) => starlarkValueRepr(v));
    return `[${items.join(", ")}]`;
  }
  if (typeof value === "object" && value !== null) {
    if (value instanceof StarlarkFunction) return value.toString();
    const entries = Object.entries(value as Record<string, unknown>).map(
      ([k, v]) => `${starlarkValueRepr(k)}: ${starlarkValueRepr(v)}`,
    );
    return `{${entries.join(", ")}}`;
  }
  return String(value);
}
