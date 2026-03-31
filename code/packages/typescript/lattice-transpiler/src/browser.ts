/**
 * Browser-compatible Lattice transpiler — backward-compatible re-export.
 *
 * This file previously contained a standalone browser transpiler with
 * embedded grammar strings and its own tokenization/parsing pipeline.
 * That workaround is no longer needed: the underlying lattice-lexer and
 * lattice-parser packages now use pre-compiled grammar objects (from
 * `_grammar.ts` files) instead of reading grammar files from disk.
 *
 * The main `transpileLattice()` function from `index.ts` works in all
 * environments — Node.js, browsers, edge runtimes — without any special
 * configuration. This file simply re-exports it under the old name for
 * backward compatibility with existing consumers.
 *
 * Migration
 * ---------
 *
 * Replace:
 *     import { transpileLatticeInBrowser } from
 *       "@coding-adventures/lattice-transpiler/src/browser.js";
 *
 * With:
 *     import { transpileLattice } from "@coding-adventures/lattice-transpiler";
 *
 * Both functions are now identical.
 */

import { transpileLattice } from "./index.js";
import type { TranspileOptions } from "./index.js";

/**
 * Transpile Lattice source text to CSS.
 *
 * @deprecated Use `transpileLattice` from the main entry point instead.
 *             This function now delegates directly to `transpileLattice`.
 */
export function transpileLatticeInBrowser(
  source: string,
  options: TranspileOptions = {}
): string {
  return transpileLattice(source, options);
}

// Re-export everything from the main entry point for backward compatibility
export type { TranspileOptions };

export {
  LatticeError,
  LatticeModuleNotFoundError,
  ReturnOutsideFunctionError,
  UndefinedVariableError,
  UndefinedMixinError,
  UndefinedFunctionError,
  WrongArityError,
  CircularReferenceError,
  TypeErrorInExpression,
  UnitMismatchError,
  MissingReturnError,
} from "@coding-adventures/lattice-ast-to-css";

export const VERSION = "0.1.0";
