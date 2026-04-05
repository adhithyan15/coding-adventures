/**
 * @coding-adventures/wasm-validator
 *
 * Semantic validation for parsed WebAssembly modules.
 */

export const VERSION = "0.1.0";

export {
  ValidationError,
  ValidationErrorKind,
  validate,
  validateConstExpr,
  validateFunction,
  validateStructure,
} from "./wasm_validator.js";

export type { IndexSpaces, ValidatedModule } from "./wasm_validator.js";
