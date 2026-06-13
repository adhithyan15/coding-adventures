/**
 * Value-level SIR semantics: the `Val` type, truthiness, equality,
 * display, and predicates.
 *
 * **SIR truthiness is false/nil-only.** Only `false` and `nil` (`null`)
 * are falsy. Everything else — including `0`, `""`, `[]`, `{}`, a symbol,
 * a pair — is **truthy**. This is the Lisp/Ruby convention and the single
 * most important reason this library exists: JavaScript's native coercion
 * would (wrongly, for SIR) call `0`/`""`/`NaN` falsy.
 *
 *     truthy(false) -> false    truthy(null) -> false
 *     truthy(0)     -> true      truthy("")   -> true
 */

import { Pair } from "./pairs.js";
import { Sym } from "./symbols.js";

/** Forward declaration of the closure handle (defined in runtime.ts). */
export interface ClosureLike {
  readonly __sirClosure: true;
}

/**
 * A SIR value.  Includes the SIR16 collection types — sequences
 * (`Val[]`) and maps (`Map<Val, Val>`) — so backends can emit native
 * arrays/maps that still type as `Val`.  Both are truthy under SIR
 * truthiness and display via `String(v)` in {@link toDisplay}.
 */
export type Val =
  | number
  | boolean
  | null
  | string
  | Sym
  | Pair
  | ClosureLike
  | Val[]
  | Map<Val, Val>;

/** SIR truthiness: everything is true except `false` and `nil`. */
export function truthy(v: Val): boolean {
  return v !== false && v !== null;
}

/** True iff `v` is `nil` (`null`). */
export function isNull(v: Val): boolean {
  return v === null;
}

/** True iff `v` is a number. */
export function isNumber(v: Val): boolean {
  return typeof v === "number";
}

/** True iff `v` is a {@link Sym}. */
export function isSymbol(v: Val): boolean {
  return v instanceof Sym;
}

/** SIR equality. Symbol-aware (two symbols are equal iff their names
 * match); otherwise native `===`. */
export function eq(a: Val, b: Val): boolean {
  if (a instanceof Sym && b instanceof Sym) {
    return a.name === b.name;
  }
  return a === b;
}

/**
 * SIR display form. Distinct from JSON: `nil` prints as `nil`, booleans
 * as `#t`/`#f`, a symbol as its bare name, a pair as a Lisp list.
 * Everything else falls back to `String(v)`.
 */
export function toDisplay(v: Val): string {
  if (v === null) {
    return "nil";
  }
  if (v === true) {
    return "#t";
  }
  if (v === false) {
    return "#f";
  }
  if (v instanceof Sym) {
    return v.name;
  }
  if (v instanceof Pair) {
    return v.toString();
  }
  return String(v);
}
