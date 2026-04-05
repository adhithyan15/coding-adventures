/**
 * WasmExecutionEngine — The Core WASM Interpreter.
 *
 * ==========================================================================
 * Chapter 1: How It All Fits Together
 * ==========================================================================
 *
 * The WasmExecutionEngine is the centerpiece of the WASM execution system.
 * It takes a validated WASM module's runtime state (memory, tables, globals,
 * functions) and executes function calls using the GenericVM infrastructure.
 *
 * The flow for calling a function:
 *
 * ```
 * WasmExecutionEngine.callFunction(funcIndex, args)
 *   │
 *   ├── 1. Look up the function body
 *   ├── 2. Decode the bytecodes into Instruction[]
 *   ├── 3. Build the control flow map
 *   ├── 4. Initialize locals (args + zero-initialized declared locals)
 *   ├── 5. Create the WasmExecutionContext
 *   ├── 6. Run GenericVM.executeWithContext(code, context)
 *   │       │
 *   │       └── For each instruction:
 *   │           ├── GenericVM dispatches to registered handler
 *   │           └── Handler reads/modifies context (stack, memory, etc.)
 *   │
 *   └── 7. Collect return values from the typed stack
 * ```
 *
 * The GenericVM provides the eval loop and opcode dispatch table. The WASM
 * instruction handlers (registered during engine construction) provide the
 * instruction semantics. The execution context carries the WASM-specific
 * state (memory, tables, globals, labels).
 *
 * ==========================================================================
 * Chapter 2: Recursive Function Calls
 * ==========================================================================
 *
 * When a WASM function calls another WASM function (via ``call`` or
 * ``call_indirect``), the control flow handler saves the caller's state
 * and sets up a new frame. The GenericVM continues executing, now in the
 * callee's context. When the callee returns (via ``return`` or ``end``),
 * the engine restores the caller's state and resumes.
 *
 * This is similar to how a real CPU handles function calls:
 * - Push return address and saved registers onto the call stack.
 * - Jump to the callee.
 * - On return, pop saved registers and jump to the return address.
 *
 * @module
 */

import { GenericVM } from "@coding-adventures/virtual-machine";
import type { CodeObject, Instruction } from "@coding-adventures/virtual-machine";
import { ValueType } from "@coding-adventures/wasm-types";
import type { FuncType, FunctionBody, GlobalType } from "@coding-adventures/wasm-types";
import type { WasmValue } from "./values.js";
import { defaultValue } from "./values.js";
import type { LinearMemory } from "./linear_memory.js";
import type { Table } from "./table.js";
import type { HostFunction } from "./host_interface.js";
import { TrapError } from "./host_interface.js";
import type { WasmExecutionContext } from "./types.js";
import {
  decodeFunctionBody,
  buildControlFlowMap,
  toVmInstructions,
} from "./decoder.js";
import type { DecodedInstruction } from "./decoder.js";
import { registerAllInstructions } from "./instructions/dispatch.js";
import { registerControl } from "./instructions/control.js";

// =========================================================================
// Engine Configuration
// =========================================================================

/** Maximum call stack depth to prevent infinite recursion. */
const MAX_CALL_DEPTH = 1024;

// =========================================================================
// Execution Engine
// =========================================================================

/**
 * The WASM execution engine — interprets validated WASM modules.
 *
 * Usage:
 *
 * ```typescript
 * const engine = new WasmExecutionEngine({
 *   memory: linearMemory,
 *   tables: [funcTable],
 *   globals: [i32(0)],
 *   globalTypes: [{ valueType: ValueType.I32, mutable: true }],
 *   funcTypes: [{ params: [ValueType.I32], results: [ValueType.I32] }],
 *   funcBodies: [functionBody],
 *   hostFunctions: [null],  // no imports
 * });
 *
 * const result = engine.callFunction(0, [i32(5)]);
 * // result = [i32(25)] for a square function
 * ```
 */
export class WasmExecutionEngine {
  private readonly vm: GenericVM;
  private readonly memory: LinearMemory | null;
  private readonly tables: Table[];
  private readonly globals: WasmValue[];
  private readonly globalTypes: GlobalType[];
  private readonly funcTypes: FuncType[];
  private readonly funcBodies: (FunctionBody | null)[];
  private readonly hostFunctions: (HostFunction | null)[];

  /** Cache of decoded function bodies (decoded once, reused on subsequent calls). */
  private readonly decodedCache: Map<number, DecodedInstruction[]> = new Map();

  constructor(config: {
    memory: LinearMemory | null;
    tables: Table[];
    globals: WasmValue[];
    globalTypes: GlobalType[];
    funcTypes: FuncType[];
    funcBodies: (FunctionBody | null)[];
    hostFunctions: (HostFunction | null)[];
  }) {
    this.memory = config.memory;
    this.tables = config.tables;
    this.globals = config.globals;
    this.globalTypes = config.globalTypes;
    this.funcTypes = config.funcTypes;
    this.funcBodies = config.funcBodies;
    this.hostFunctions = config.hostFunctions;

    // Create and configure the GenericVM.
    this.vm = new GenericVM();
    this.vm.setMaxRecursionDepth(MAX_CALL_DEPTH);

    // Register all WASM instruction handlers.
    registerAllInstructions(this.vm);
    registerControl(this.vm);
  }

  /**
   * Call a WASM function by index.
   *
   * @param funcIndex - The function index (in the combined import + module space).
   * @param args      - The function arguments as WasmValues.
   * @returns         The function's return values as WasmValues.
   * @throws {TrapError} On runtime errors (div by zero, OOB memory, etc.).
   */
  callFunction(funcIndex: number, args: WasmValue[]): WasmValue[] {
    const funcType = this.funcTypes[funcIndex];
    if (!funcType) {
      throw new TrapError(`undefined function index ${funcIndex}`);
    }

    // Validate argument count.
    if (args.length !== funcType.params.length) {
      throw new TrapError(
        `function ${funcIndex} expects ${funcType.params.length} arguments, got ${args.length}`,
      );
    }

    // Check if this is a host (imported) function.
    const hostFunc = this.hostFunctions[funcIndex];
    if (hostFunc) {
      return hostFunc.call(args);
    }

    // Module-defined function.
    const body = this.funcBodies[funcIndex];
    if (!body) {
      throw new TrapError(`no body for function ${funcIndex}`);
    }

    // Decode the function body (cached).
    let decoded = this.decodedCache.get(funcIndex);
    if (!decoded) {
      decoded = decodeFunctionBody(body);
      this.decodedCache.set(funcIndex, decoded);
    }

    // Build the control flow map.
    const controlFlowMap = buildControlFlowMap(decoded);

    // Convert to GenericVM instruction format.
    const vmInstructions = toVmInstructions(decoded);

    // Initialize locals: arguments + zero-initialized declared locals.
    const typedLocals: WasmValue[] = [
      ...args,
      ...body.locals.map(t => defaultValue(t)),
    ];

    // Build the execution context.
    const ctx: WasmExecutionContext = {
      memory: this.memory,
      tables: this.tables,
      globals: this.globals,
      globalTypes: this.globalTypes,
      funcTypes: this.funcTypes,
      funcBodies: this.funcBodies,
      hostFunctions: this.hostFunctions,
      typedLocals,
      labelStack: [],
      controlFlowMap,
      savedFrames: [],
      returned: false,
      returnValues: [],
    };

    // Build the CodeObject.
    const code: CodeObject = {
      instructions: vmInstructions,
      constants: [],
      names: [],
    };

    // Reset the VM and execute.
    this.vm.reset();

    // Re-register handlers after reset (reset clears state but preserves handlers).
    // Actually, GenericVM.reset() preserves handlers, so this is fine.

    this.vm.executeWithContext(code, ctx);

    // Collect return values from the typed stack.
    const resultCount = funcType.results.length;
    const results: WasmValue[] = [];
    for (let i = 0; i < resultCount; i++) {
      if (this.vm.typedStack.length > 0) {
        results.unshift(this.vm.popTyped() as WasmValue);
      }
    }

    return results;
  }
}
