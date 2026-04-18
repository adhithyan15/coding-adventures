export const VERSION = "0.1.0";

export type {
  BrainfuckJvmCompilerOptions,
  PackageResult,
} from "./compiler.js";
export {
  BrainfuckJvmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeClassFile,
} from "./compiler.js";
