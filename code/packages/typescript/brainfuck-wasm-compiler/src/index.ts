export const VERSION = "0.1.0";

export type {
  BrainfuckWasmCompilerOptions,
  PackageResult,
} from "./compiler.js";
export {
  BrainfuckWasmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeWasmFile,
} from "./compiler.js";
