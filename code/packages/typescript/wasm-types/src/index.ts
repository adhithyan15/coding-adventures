/**
 * @coding-adventures/wasm-types
 *
 * WASM 1.0 type system: ValueType, FuncType, Limits, MemoryType, TableType,
 * GlobalType, WasmModule, and all associated data structures.
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 *
 * Re-exports everything from the main implementation module so consumers
 * can simply write:
 *
 *   import { ValueType, WasmModule, makeFuncType } from "@coding-adventures/wasm-types";
 */

export const VERSION = "0.1.0";

export {
  ValueType,
  BlockType,
  ExternalKind,
  FUNCREF,
  makeFuncType,
} from "./wasm_types.js";

export type {
  FuncType,
  Limits,
  MemoryType,
  TableType,
  GlobalType,
  Import,
  Export,
  Global,
  Element,
  DataSegment,
  FunctionBody,
  CustomSection,
} from "./wasm_types.js";

export { WasmModule } from "./wasm_types.js";
