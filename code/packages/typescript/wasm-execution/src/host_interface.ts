/**
 * host_interface.ts --- Host function interface and TrapError for WASM execution
 *
 * ===========================================================================
 * WHAT IS THE HOST INTERFACE?
 * ===========================================================================
 *
 * WebAssembly modules do not run in isolation. They interact with the outside
 * world through *imports* --- functions, globals, memories, and tables that
 * the host environment provides. Think of the host as the operating system
 * and the WASM module as a user-space program: the module can only access
 * resources that the host explicitly grants.
 *
 * For example, when a WASM module running in a browser calls ``console.log``,
 * it is invoking a *host function* that the JavaScript runtime imported into
 * the module's namespace.
 *
 * The HostInterface is the contract that any host environment must implement
 * to provide these imported values. It has four resolve methods, one for each
 * kind of importable definition in WASM 1.0:
 *
 *   1. **Functions** --- executable code provided by the host
 *   2. **Globals**   --- mutable or immutable scalar values
 *   3. **Memories**  --- byte-addressable linear memory regions
 *   4. **Tables**    --- arrays of function references (for indirect calls)
 *
 * ===========================================================================
 * WHAT IS A TRAP?
 * ===========================================================================
 *
 * In WASM, a *trap* is an unrecoverable runtime error. When a trap occurs,
 * execution of the current module immediately halts. There is no exception
 * handling within WASM 1.0 --- traps propagate to the host, which decides
 * what to do (typically throwing an exception in the host language).
 *
 * Common causes of traps:
 *
 *   - Out-of-bounds memory access (reading past the end of linear memory)
 *   - Out-of-bounds table access (call_indirect with an invalid index)
 *   - Division by zero (integer division only; float division yields NaN)
 *   - Integer overflow in division (e.g., i32.div_s(-2147483648, -1))
 *   - Unreachable instruction executed (explicit trap via ``unreachable``)
 *   - Type mismatch in call_indirect (signature does not match)
 *
 * We model traps as a TrapError class that extends JavaScript's Error. This
 * lets host code use standard try/catch to handle traps, while keeping them
 * distinct from other error types (like stack overflow or validation errors).
 *
 *   ┌────────────────────────────────────────────────────────────────────────┐
 *   │                         Trap Error Flow                               │
 *   │                                                                       │
 *   │   WASM Module                     Host Environment                    │
 *   │  ┌──────────────┐               ┌──────────────────┐                  │
 *   │  │  i32.div_s   │──── trap! ───>│  try { ... }     │                  │
 *   │  │  (n / 0)     │               │  catch (e) {     │                  │
 *   │  └──────────────┘               │    // TrapError!  │                  │
 *   │                                 │  }                │                  │
 *   │                                 └──────────────────┘                  │
 *   └────────────────────────────────────────────────────────────────────────┘
 */

import type { FuncType } from "@coding-adventures/wasm-types";
import type { WasmValue } from "./values.js";
import type { LinearMemory } from "./linear_memory.js";
import type { Table } from "./table.js";

// ===========================================================================
// TrapError
// ===========================================================================

/**
 * TrapError --- an unrecoverable WASM runtime error (a "trap").
 *
 * This is the WASM equivalent of a fatal error. When execution encounters
 * an illegal operation (out-of-bounds access, division by zero, etc.), we
 * throw a TrapError and immediately unwind the entire call stack.
 *
 * Why a custom class instead of just ``new Error(...)``?
 *   - Allows host code to distinguish traps from other errors via instanceof.
 *   - The ``name`` property shows "TrapError" in stack traces, making
 *     debugging easier.
 *   - Matches the WASM spec's concept of traps as a distinct error category.
 */
export class TrapError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TrapError";
  }
}

// ===========================================================================
// HostFunction
// ===========================================================================

/**
 * HostFunction --- a callable function provided by the host environment.
 *
 * When a WASM module imports a function (e.g., ``(import "env" "print"
 * (func (param i32))``), the host must provide an object implementing this
 * interface. The ``type`` field describes the expected parameter and return
 * types (so the runtime can validate calls), and the ``call`` method
 * performs the actual work.
 *
 * Example: a host function that prints an i32 to the console:
 *
 *   const printI32: HostFunction = {
 *     type: { params: [ValueType.I32], results: [] },
 *     call(args) {
 *       console.log("WASM says:", asI32(args[0]));
 *       return [];
 *     },
 *   };
 *
 * The ``call`` method receives an array of WasmValues (one per parameter)
 * and returns an array of WasmValues (one per result). WASM 1.0 allows at
 * most one result value, but the interface uses an array for forward
 * compatibility with the multi-value proposal.
 */
export interface HostFunction {
  /** The function signature: parameter types and result types. */
  readonly type: FuncType;

  /**
   * Invoke this host function with the given arguments.
   *
   * @param args - One WasmValue per parameter, in declaration order.
   * @returns    An array of WasmValues (empty for void functions, or a
   *             single-element array for functions with one result).
   */
  call(args: WasmValue[]): WasmValue[];
}

// ===========================================================================
// HostInterface
// ===========================================================================

/**
 * HostInterface --- the contract for resolving WASM imports.
 *
 * When instantiating a WASM module, the runtime walks through every import
 * declaration and calls the appropriate ``resolve*`` method to obtain the
 * imported definition. If the host returns ``undefined``, instantiation fails
 * with a link error (the import could not be satisfied).
 *
 * The two-level namespace (moduleName + name) mirrors the WASM binary format:
 *
 *   (import "env" "memory" (memory 1))
 *            ^^^   ^^^^^^
 *            |     |
 *            |     name (the specific definition)
 *            moduleName (the "namespace" or "library")
 *
 * A typical host might implement this by maintaining a registry:
 *
 *   const host: HostInterface = {
 *     resolveFunction(mod, name) {
 *       if (mod === "env" && name === "print") return printI32;
 *       return undefined; // import not found
 *     },
 *     resolveGlobal(mod, name) { return undefined; },
 *     resolveMemory(mod, name) {
 *       if (mod === "env" && name === "memory") return memory;
 *       return undefined;
 *     },
 *     resolveTable(mod, name) { return undefined; },
 *   };
 */
export interface HostInterface {
  /**
   * Resolve an imported function.
   *
   * @param moduleName - The import's module namespace (e.g., "env").
   * @param name       - The import's field name (e.g., "print").
   * @returns The host function, or undefined if not found.
   */
  resolveFunction(
    moduleName: string,
    name: string
  ): HostFunction | undefined;

  /**
   * Resolve an imported global variable.
   *
   * Returns both the global's type descriptor (value type + mutability)
   * and its current value.
   *
   * @param moduleName - The import's module namespace.
   * @param name       - The import's field name.
   * @returns The global definition, or undefined if not found.
   */
  resolveGlobal(
    moduleName: string,
    name: string
  ): { type: { valueType: number; mutable: boolean }; value: WasmValue } | undefined;

  /**
   * Resolve an imported linear memory.
   *
   * @param moduleName - The import's module namespace.
   * @param name       - The import's field name.
   * @returns The LinearMemory instance, or undefined if not found.
   */
  resolveMemory(
    moduleName: string,
    name: string
  ): LinearMemory | undefined;

  /**
   * Resolve an imported table.
   *
   * @param moduleName - The import's module namespace.
   * @param name       - The import's field name.
   * @returns The Table instance, or undefined if not found.
   */
  resolveTable(
    moduleName: string,
    name: string
  ): Table | undefined;
}
