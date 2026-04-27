export const VERSION = "0.1.0";

export type { FunctionSignature } from "./compiler.js";
export {
  IrToWasmCompiler,
  WasmLoweringError,
  inferFunctionSignaturesFromComments,
} from "./compiler.js";
