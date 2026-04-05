/**
 * table.ts --- WASM table implementation for indirect function calls.
 *
 * ===========================================================================
 * WHAT IS A TABLE?
 * ===========================================================================
 *
 * A WASM table is an array of opaque references --- in WASM 1.0, these are
 * always function references (``funcref``). Tables enable *indirect* function
 * calls: instead of calling a function by its index directly, code can look
 * up a function reference in a table at runtime.
 *
 * This is how WASM implements C-style function pointers, virtual method
 * dispatch, and dynamic linking --- anywhere you need to decide *which*
 * function to call based on runtime data rather than compile-time constants.
 *
 * ===========================================================================
 * WHY NOT STORE FUNCTION POINTERS IN MEMORY?
 * ===========================================================================
 *
 * In C, function pointers are raw memory addresses. A bug (buffer overflow,
 * use-after-free) could corrupt a function pointer and redirect execution
 * to arbitrary code --- this is a classic security vulnerability.
 *
 * WASM tables are *opaque*: code cannot manufacture a function reference
 * from an integer. It can only read references that the module or host
 * placed in the table. This is a form of capability-based security ---
 * you can only call functions you were explicitly given access to.
 *
 * ===========================================================================
 * HOW call_indirect USES THE TABLE
 * ===========================================================================
 *
 *   1. Pop an i32 index from the stack.
 *   2. Look up table[index]. If out of bounds or null, TRAP.
 *   3. Verify the function's type signature matches the expected type.
 *      If mismatch, TRAP.
 *   4. Call the function with arguments from the stack.
 *
 *   +----+----+------+----+----+------+
 *   | f0 | f3 | null | f1 | f5 | null |   <- Table elements
 *   +----+----+------+----+----+------+
 *     0    1     2     3    4     5
 *                      ^
 *              call_indirect(3) -> calls f1
 *
 * @module
 */

import { TrapError } from "./host_interface.js";

/**
 * Table --- a resizable array of nullable function indices.
 *
 * In WASM 1.0, table elements are either a valid function index or ``null``
 * (uninitialized). Accessing a null element via ``call_indirect`` causes a
 * trap. Accessing an out-of-bounds index also traps.
 */
export class Table {
  /** The table elements: function index or null. */
  private elements: (number | null)[];

  /** Maximum number of elements (null = no limit). */
  private readonly maxSize: number | null;

  /**
   * Create a new Table.
   *
   * @param initialSize - Number of entries, all initialized to null.
   * @param maxSize     - Optional upper bound on table size.
   */
  constructor(initialSize: number, maxSize?: number) {
    this.elements = new Array<number | null>(initialSize).fill(null);
    this.maxSize = maxSize !== undefined ? maxSize : null;
  }

  /**
   * Get the function index at the given table index.
   *
   * @param index - The table index.
   * @returns The function index, or null if the entry is empty.
   * @throws TrapError if the index is out of bounds.
   */
  get(index: number): number | null {
    if (index < 0 || index >= this.elements.length) {
      throw new TrapError(
        `Out of bounds table access: index=${index}, table size=${this.elements.length}`
      );
    }
    return this.elements[index];
  }

  /**
   * Set the function index at the given table index.
   *
   * @param index     - The table index.
   * @param funcIndex - The function index to store (or null to clear).
   * @throws TrapError if the index is out of bounds.
   */
  set(index: number, funcIndex: number | null): void {
    if (index < 0 || index >= this.elements.length) {
      throw new TrapError(
        `Out of bounds table access: index=${index}, table size=${this.elements.length}`
      );
    }
    this.elements[index] = funcIndex;
  }

  /** Return the current table size (number of entries). */
  size(): number {
    return this.elements.length;
  }

  /**
   * Grow the table by ``delta`` entries (initialized to null).
   *
   * Returns the old size on success, or -1 if growth would exceed
   * the maximum. Like memory.grow, failure is a normal result.
   *
   * @param delta - Number of entries to add.
   * @returns The old table size, or -1 on failure.
   */
  grow(delta: number): number {
    const oldSize = this.elements.length;
    const newSize = oldSize + delta;

    if (this.maxSize !== null && newSize > this.maxSize) {
      return -1;
    }

    for (let i = 0; i < delta; i++) {
      this.elements.push(null);
    }
    return oldSize;
  }
}
