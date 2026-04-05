/**
 * WasmInstance — A Runtime Instance of a WASM Module.
 *
 * ==========================================================================
 * What Is an Instance?
 * ==========================================================================
 *
 * A WASM **module** is a static artifact — it describes types, functions,
 * memory layout, and initialization data, but it doesn't "run" anything.
 * An **instance** is a module that has been "brought to life":
 *
 * - Memory has been allocated and initialized with data segments.
 * - Tables have been created and filled with element segments.
 * - Globals have been initialized with constant expressions.
 * - Imports have been resolved to actual host functions.
 * - The start function (if any) has been called.
 *
 * Think of a module as a class and an instance as an object: the module
 * defines the blueprint, the instance is a live, running entity with its
 * own state.
 *
 * A single module can be instantiated multiple times, each with independent
 * memory, tables, and globals. This is how WASM achieves sandboxing — each
 * instance is isolated from others.
 *
 * @module
 */

import type { WasmModule, FuncType, GlobalType, Export, ExternalKind } from "@coding-adventures/wasm-types";
import type { WasmValue } from "@coding-adventures/wasm-execution";
import type { LinearMemory } from "@coding-adventures/wasm-execution";
import type { Table } from "@coding-adventures/wasm-execution";
import type { HostFunction, HostInterface } from "@coding-adventures/wasm-execution";
import type { FunctionBody } from "@coding-adventures/wasm-types";

/**
 * A live, executable instance of a WASM module.
 *
 * Contains all allocated runtime state and provides access to exports.
 */
export interface WasmInstance {
  /** The original parsed module. */
  readonly module: WasmModule;

  /** Allocated linear memory (null if module has no memory). */
  readonly memory: LinearMemory | null;

  /** Allocated tables. */
  readonly tables: Table[];

  /** Current global variable values (mutable for var globals). */
  readonly globals: WasmValue[];

  /** Global type descriptors. */
  readonly globalTypes: GlobalType[];

  /** All function type signatures (imports + module functions). */
  readonly funcTypes: FuncType[];

  /** Function bodies (null for imported functions). */
  readonly funcBodies: (FunctionBody | null)[];

  /** Host function implementations (null for module-defined functions). */
  readonly hostFunctions: (HostFunction | null)[];

  /** Export lookup table: name → { kind, index }. */
  readonly exports: Map<string, { kind: number; index: number }>;

  /** The host interface used to resolve imports. */
  readonly host: HostInterface | null;
}
