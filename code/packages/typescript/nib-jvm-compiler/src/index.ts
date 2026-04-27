export const VERSION = "0.1.0";

export type {
  NibJvmCompilerOptions,
  PackageResult,
} from "./compiler.js";
export {
  NibJvmCompiler,
  PackageError,
  compileSource,
  packSource,
  writeClassFile,
} from "./compiler.js";
