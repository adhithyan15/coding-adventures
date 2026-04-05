/**
 * @coding-adventures/wasm-execution
 *
 * WebAssembly 1.0 execution engine with typed stack machine and 182 instruction
 * handlers, built on the GenericVM infrastructure.
 *
 * @module
 */

export const VERSION = "0.1.0";

// Core value types and constructors
export type { WasmValue } from "./values.js";
export { i32, i64, f32, f64, defaultValue, asI32, asI64, asF32, asF64 } from "./values.js";

// Runtime data structures
export { LinearMemory } from "./linear_memory.js";
export { Table } from "./table.js";

// Host interface
export { TrapError } from "./host_interface.js";
export type { HostFunction, HostInterface } from "./host_interface.js";

// Constant expression evaluation
export { evaluateConstExpr } from "./const_expr.js";

// Execution engine
export { WasmExecutionEngine } from "./wasm_execution.js";

// Shared types
export type { WasmExecutionContext, Label, ControlTarget, SavedFrame } from "./types.js";

// Decoder
export { decodeFunctionBody, buildControlFlowMap, toVmInstructions } from "./decoder.js";
