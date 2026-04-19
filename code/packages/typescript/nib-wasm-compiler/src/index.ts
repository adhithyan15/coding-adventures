export const VERSION = "0.1.0";

export type {
  NibWasmCompilerOptions,
  PackageResult,
} from "./compiler.js";
export {
  NibWasmCompiler,
  PackageError,
  compileSource,
  extractSignatures,
  packSource,
  writeWasmFile,
} from "./compiler.js";
