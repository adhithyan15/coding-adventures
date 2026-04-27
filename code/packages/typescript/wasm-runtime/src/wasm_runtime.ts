/**
 * WasmRuntime — The Complete WebAssembly Runtime.
 *
 * ==========================================================================
 * Chapter 1: What Is a Runtime?
 * ==========================================================================
 *
 * A WASM runtime is the user-facing entry point that composes all the lower-
 * level packages into a single, easy-to-use API. It handles the full pipeline:
 *
 * ```
 *   .wasm bytes  →  Parse  →  Validate  →  Instantiate  →  Execute
 *       │              │           │             │              │
 *   Uint8Array   WasmModule  ValidatedModule  WasmInstance  WasmValue[]
 *       │              │           │             │              │
 *   (input)    (module-parser) (validator)  (this file)   (execution)
 * ```
 *
 * The convenience method ``loadAndRun()`` does all four steps in one call:
 *
 * ```typescript
 * const runtime = new WasmRuntime();
 * const result = runtime.loadAndRun(squareWasm, "square", [5]);
 * // result = [25]
 * ```
 *
 * ==========================================================================
 * Chapter 2: Instantiation
 * ==========================================================================
 *
 * Instantiation is the most complex step. It transforms a static module
 * definition into a live, executable instance:
 *
 * 1. **Resolve imports**: For each import in the module, ask the host
 *    interface to provide the function/memory/table/global.
 *
 * 2. **Allocate memory**: Create LinearMemory with the size specified in
 *    the memory section.
 *
 * 3. **Allocate tables**: Create Tables with the sizes from the table section.
 *
 * 4. **Initialize globals**: Evaluate constant expressions for each global's
 *    initial value.
 *
 * 5. **Apply data segments**: Copy bytes from data segments into memory at
 *    the offsets specified by their constant expressions.
 *
 * 6. **Apply element segments**: Copy function references from element segments
 *    into tables at their specified offsets.
 *
 * 7. **Call start function**: If the module declares a start function, call it.
 *
 * @module
 */

import { WasmModuleParser } from "@coding-adventures/wasm-module-parser";
import { validate } from "@coding-adventures/wasm-validator";
import type { ValidatedModule } from "@coding-adventures/wasm-validator";
import { ExternalKind, ValueType } from "@coding-adventures/wasm-types";
import type { WasmModule, FuncType, GlobalType, FunctionBody } from "@coding-adventures/wasm-types";
import {
  WasmExecutionEngine,
  LinearMemory,
  Table,
  TrapError,
  evaluateConstExpr,
  i32, i64, f32, f64, defaultValue,
} from "@coding-adventures/wasm-execution";
import type { WasmValue, HostFunction, HostInterface } from "@coding-adventures/wasm-execution";
import type { WasmInstance } from "./wasm_instance.js";

// =========================================================================
// The Runtime
// =========================================================================

/**
 * Complete WebAssembly 1.0 runtime.
 *
 * Composes the parser, validator, and execution engine into a single
 * user-facing API. Optionally accepts a host interface for import resolution
 * (e.g., a WASI implementation).
 *
 * **Usage:**
 *
 * ```typescript
 * // Simple: compute square(5) from a .wasm binary
 * const runtime = new WasmRuntime();
 * const result = runtime.loadAndRun(squareWasm, "square", [5]);
 * console.log(result); // [25]
 *
 * // With WASI for programs that do I/O:
 * const wasi = new WasiStub({ stdout: (text) => console.log(text) });
 * const runtime = new WasmRuntime(wasi);
 * runtime.loadAndRun(helloWorldWasm);
 * ```
 */
export class WasmRuntime {
  private readonly parser: WasmModuleParser;
  private readonly host: HostInterface | null;

  constructor(host?: HostInterface) {
    this.parser = new WasmModuleParser();
    this.host = host ?? null;
  }

  // ─── Parse ──────────────────────────────────────────────────────────

  /**
   * Parse a .wasm binary into a WasmModule.
   *
   * @param wasmBytes - The raw .wasm binary data.
   * @returns         The parsed module structure.
   * @throws {WasmParseError} On malformed binary data.
   */
  load(wasmBytes: Uint8Array): WasmModule {
    return this.parser.parse(wasmBytes);
  }

  // ─── Validate ───────────────────────────────────────────────────────

  /**
   * Validate a parsed module for semantic correctness.
   *
   * @param module - The parsed WASM module.
   * @returns      The validated module with resolved type information.
   * @throws {ValidationError} On validation failures.
   */
  validate(module: WasmModule): ValidatedModule {
    return validate(module);
  }

  // ─── Instantiate ────────────────────────────────────────────────────

  /**
   * Create a live instance from a parsed (and optionally validated) module.
   *
   * This allocates all runtime resources: memory, tables, globals.
   * Resolves imports, applies data/element segments, and calls the start
   * function if one is declared.
   *
   * @param module - The parsed WASM module.
   * @returns      A live, executable instance.
   */
  instantiate(module: WasmModule): WasmInstance {
    // Step 1: Build the combined function type array (imports + module functions).
    const funcTypes: FuncType[] = [];
    const funcBodies: (FunctionBody | null)[] = [];
    const hostFunctions: (HostFunction | null)[] = [];
    const globalTypes: GlobalType[] = [];
    const globals: WasmValue[] = [];

    // Step 2: Resolve imports.
    let memory: LinearMemory | null = null;
    const tables: Table[] = [];

    for (const imp of module.imports) {
      switch (imp.kind) {
        case ExternalKind.FUNCTION: {
          // imp.typeInfo is the type index for function imports.
          const typeIdx = imp.typeInfo as number;
          const funcType = module.types[typeIdx];
          funcTypes.push(funcType);
          funcBodies.push(null); // No body for imports.

          // Resolve the host function.
          const hostFunc = this.host?.resolveFunction(imp.moduleName, imp.name);
          hostFunctions.push(hostFunc ?? null);
          break;
        }
        case ExternalKind.MEMORY: {
          // Resolve imported memory.
          const importedMem = this.host?.resolveMemory(imp.moduleName, imp.name);
          if (importedMem) {
            memory = importedMem;
          }
          break;
        }
        case ExternalKind.TABLE: {
          const importedTable = this.host?.resolveTable(imp.moduleName, imp.name);
          if (importedTable) {
            tables.push(importedTable);
          }
          break;
        }
        case ExternalKind.GLOBAL: {
          const importedGlobal = this.host?.resolveGlobal(imp.moduleName, imp.name);
          if (importedGlobal) {
            globalTypes.push(importedGlobal.type as GlobalType);
            globals.push(importedGlobal.value);
          }
          break;
        }
      }
    }

    // Step 3: Add module-defined functions.
    for (let i = 0; i < module.functions.length; i++) {
      const typeIdx = module.functions[i];
      funcTypes.push(module.types[typeIdx]);
      funcBodies.push(module.code[i] ?? null);
      hostFunctions.push(null);
    }

    // Step 4: Allocate memory (from memory section, if not imported).
    if (!memory && module.memories.length > 0) {
      const memType = module.memories[0];
      memory = new LinearMemory(
        memType.limits.min,
        memType.limits.max ?? undefined,
      );
    }

    // Step 5: Allocate tables (from table section, if not imported).
    for (const tableType of module.tables) {
      tables.push(new Table(
        tableType.limits.min,
        tableType.limits.max ?? undefined,
      ));
    }

    // Step 6: Initialize globals (from global section).
    for (const global of module.globals) {
      globalTypes.push(global.globalType);
      const value = evaluateConstExpr(global.initExpr, globals);
      globals.push(value);
    }

    // Step 7: Apply data segments (copy bytes to memory).
    if (memory) {
      for (const seg of module.data) {
        const offset = evaluateConstExpr(seg.offsetExpr, globals);
        const offsetNum = offset.value as number;
        memory.writeBytes(offsetNum, seg.data);
      }
    }

    // Step 8: Apply element segments (copy func refs to tables).
    for (const elem of module.elements) {
      const table = tables[elem.tableIndex];
      if (table) {
        const offset = evaluateConstExpr(elem.offsetExpr, globals);
        const offsetNum = offset.value as number;
        for (let j = 0; j < elem.functionIndices.length; j++) {
          table.set(offsetNum + j, elem.functionIndices[j]);
        }
      }
    }

    // Build the export map.
    const exports = new Map<string, { kind: number; index: number }>();
    for (const exp of module.exports) {
      exports.set(exp.name, { kind: exp.kind, index: exp.index });
    }

    const instance: WasmInstance = {
      module,
      memory,
      tables,
      globals,
      globalTypes,
      funcTypes,
      funcBodies,
      hostFunctions,
      exports,
      host: this.host,
    };

    const memoryBinder = this.host as
      | (HostInterface & {
          setMemory?: (memory: LinearMemory) => void;
        })
      | null;
    if (memory && memoryBinder && typeof memoryBinder.setMemory === "function") {
      memoryBinder.setMemory(memory);
    }

    // Step 9: Call start function (if present).
    if (module.start !== null && module.start !== undefined) {
      const engine = new WasmExecutionEngine({
        memory: instance.memory,
        tables: instance.tables,
        globals: instance.globals,
        globalTypes: instance.globalTypes,
        funcTypes: instance.funcTypes,
        funcBodies: instance.funcBodies,
        hostFunctions: instance.hostFunctions,
      });
      engine.callFunction(module.start, []);
    }

    return instance;
  }

  // ─── Call ───────────────────────────────────────────────────────────

  /**
   * Call an exported function by name.
   *
   * @param instance - The live WASM instance.
   * @param name     - The export name (e.g., "square", "add", "_start").
   * @param args     - Arguments as plain numbers (converted to WasmValues).
   * @returns        Return values as plain numbers.
   * @throws {TrapError} If the export doesn't exist or on runtime errors.
   */
  call(instance: WasmInstance, name: string, args: number[]): number[] {
    const exp = instance.exports.get(name);
    if (!exp) {
      throw new TrapError(`export "${name}" not found`);
    }
    if (exp.kind !== ExternalKind.FUNCTION) {
      throw new TrapError(`export "${name}" is not a function`);
    }

    const funcType = instance.funcTypes[exp.index];
    if (!funcType) {
      throw new TrapError(`function type not found for export "${name}"`);
    }

    // Convert plain numbers to WasmValues based on the function's parameter types.
    const wasmArgs: WasmValue[] = args.map((arg, i) => {
      const paramType = funcType.params[i];
      switch (paramType) {
        case ValueType.I32: return i32(arg);
        case ValueType.I64: return i64(BigInt(arg));
        case ValueType.F32: return f32(arg);
        case ValueType.F64: return f64(arg);
        default: return i32(arg);
      }
    });

    // Create an execution engine and call the function.
    const engine = new WasmExecutionEngine({
      memory: instance.memory,
      tables: instance.tables,
      globals: instance.globals,
      globalTypes: instance.globalTypes,
      funcTypes: instance.funcTypes,
      funcBodies: instance.funcBodies,
      hostFunctions: instance.hostFunctions,
    });

    const results = engine.callFunction(exp.index, wasmArgs);

    // Convert WasmValues back to plain numbers.
    return results.map(r => {
      if (typeof r.value === "bigint") {
        return Number(r.value);
      }
      return r.value as number;
    });
  }

  // ─── Convenience ────────────────────────────────────────────────────

  /**
   * Parse, validate, instantiate, and call in one step.
   *
   * This is the easiest way to run a WASM module:
   *
   * ```typescript
   * const result = runtime.loadAndRun(wasmBytes, "square", [5]);
   * // result = [25]
   * ```
   *
   * @param wasmBytes - The raw .wasm binary.
   * @param entry     - The export function name to call (default: "_start").
   * @param args      - Arguments as plain numbers (default: []).
   * @returns         Return values as plain numbers.
   */
  loadAndRun(
    wasmBytes: Uint8Array,
    entry: string = "_start",
    args: number[] = [],
  ): number[] {
    const module = this.load(wasmBytes);
    this.validate(module);
    const instance = this.instantiate(module);
    return this.call(instance, entry, args);
  }
}
