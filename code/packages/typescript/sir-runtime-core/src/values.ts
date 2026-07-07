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

// ── source-language display convention (SIR display-convention spec) ──
//
// The default convention is "lisp" (Twig/Scheme: booleans as `#t`/`#f`),
// matching this library's original behaviour. A Ruby-sourced emitted program
// calls `setDisplayConvention("ruby")` once at startup so `puts true` prints
// `true`. Module-level state (each emitted program is its own process) keeps
// `toDisplay` convention-aware without threading a parameter through the whole
// display path.
let _displayConvention: "ruby" | "lisp" = "lisp";

/**
 * Select the value-display convention: `"ruby"` or `"lisp"` (default).
 * An unrecognised name falls back to the `"lisp"` default rather than
 * throwing, so a forward-compatible emitter can never crash an older runtime.
 */
export function setDisplayConvention(name: string): void {
  _displayConvention = name === "ruby" ? "ruby" : "lisp";
}

/**
 * SIR display form. Distinct from JSON: `nil` prints as `nil`, a symbol as its
 * bare name, a pair as a Lisp list. Booleans follow the active display
 * convention (see `setDisplayConvention`): `true`/`false` under `"ruby"`, else
 * the default Lisp `#t`/`#f`. Everything else falls back to `String(v)`.
 */
export function toDisplay(v: Val): string {
  if (v === null) {
    return "nil";
  }
  if (v === true) {
    return _displayConvention === "ruby" ? "true" : "#t";
  }
  if (v === false) {
    return _displayConvention === "ruby" ? "false" : "#f";
  }
  if (v instanceof Sym) {
    return v.name;
  }
  if (v instanceof Pair) {
    return v.toString();
  }
  return String(v);
}
