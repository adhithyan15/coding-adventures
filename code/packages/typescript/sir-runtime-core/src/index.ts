/**
 * @coding-adventures/sir-runtime-core — core runtime for SIR-emitted
 * TypeScript / JavaScript.
 *
 * Semantic-IR backends translate most constructs to **native** code (a
 * sequence is an `Array`, a loop is a `for`, a class is a `class`). The
 * handful of SIR semantics with **no faithful native equivalent** live
 * here and are imported by the emitted module:
 *
 *     import * as _sir from "@coding-adventures/sir-runtime-core";
 *     _sir.truthy(x);     // SIR truthiness: only false / nil are falsy
 *     _sir.cons(a, b);    // cons pairs
 *     _sir.print(v);      // SIR display + newline
 *
 * It implements **SIR** semantics (not any one source language's), so a
 * Ruby frontend today and a JavaScript or Python frontend tomorrow all
 * reuse it. See `code/specs/sir-runtime.md`.
 */

export { Sym, intern } from "./symbols.js";
export { Pair, cons, car, cdr, isPair } from "./pairs.js";
export { truthy, isNull, isNumber, isSymbol, eq, toDisplay } from "./values.js";
export type { Val, ClosureLike } from "./values.js";
export { add, sub, mul, div, lt, gt } from "./arithmetic.js";
export {
  Closure,
  LocalJumpError,
  apply,
  makeClosure,
  globalSet,
  globalGet,
  globalGetStatic,
  print,
  callBuiltin,
  builtinClosure,
  doubleSplatMerge,
} from "./runtime.js";

// Inject core's richer `toDisplay` into the (dependency-free) pairs package so
// a `Pair` renders as a Lisp list (`(1 2 3)`, `#t`/`nil`/symbols) rather than
// via plain `String`. Runs once when this module is first evaluated; emitted
// code imports core, so the hook is wired before any pair is displayed.
import { setDisplay as _setPairsDisplay } from "./pairs.js";
import { toDisplay as _toDisplayHook } from "./values.js";
_setPairsDisplay(_toDisplayHook);
