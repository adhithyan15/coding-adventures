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
 * The shape `@coding-adventures/sir-runtime-array`'s `NDArray` values carry
 * (`{ shape: number[], data: Float64Array }`). Declared structurally, not
 * imported — this package deliberately has NO dependency on
 * `sir-runtime-array` (a non-array-sourced program should pull in zero
 * array code), so an NDArray is recognised by its runtime SHAPE, not by
 * importing its type. Mirrors exactly how the JS backend's single
 * self-contained runtime blob duck-types the same shape in `formatSeen`.
 */
export interface NDArrayLike {
  readonly shape: readonly number[];
  readonly data: Float64Array;
}

/**
 * A SIR value.  Includes the SIR16 collection types — sequences
 * (`Val[]`) and maps (`Map<Val, Val>`) — so backends can emit native
 * arrays/maps that still type as `Val`, and (SIR22) `NDArrayLike` so the
 * TypeScript backend's emitted `__Sir.write(...)` calls type-check when one
 * of the values is an array/matrix result from `@coding-adventures/
 * sir-runtime-array`. All three are truthy under SIR truthiness and
 * display via {@link toDisplay} (`NDArrayLike` only under the `"apl"`
 * display convention today — see `toDisplay`'s own doc comment).
 */
export type Val =
  | number
  | boolean
  | null
  | string
  | Sym
  | Pair
  | ClosureLike
  | NDArrayLike
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
// `true`. An APL-sourced program calls `setDisplayConvention("apl")` so a
// negative number prints with APL's own high-minus glyph (`¯`) instead of
// ASCII `-` — see `toDisplay`'s NDArray branch below. Module-level state
// (each emitted program is its own process) keeps `toDisplay`
// convention-aware without threading a parameter through the whole display
// path.
let _displayConvention: "ruby" | "lisp" | "apl" = "lisp";

/**
 * Select the value-display convention: `"ruby"`, `"apl"`, or `"lisp"`
 * (default). An unrecognised name falls back to the `"lisp"` default rather
 * than throwing, so a forward-compatible emitter can never crash an older
 * runtime.
 */
export function setDisplayConvention(name: string): void {
  if (name === "ruby") {
    _displayConvention = "ruby";
  } else if (name === "apl") {
    _displayConvention = "apl";
  } else {
    _displayConvention = "lisp";
  }
}

/**
 * Render a number using APL's own console convention: a high-minus `¯`
 * glyph for negatives (ASCII `-` is APL's own dyadic subtraction operator,
 * so a real APL session reserves a distinct glyph for a negative
 * literal/result) — ported 1:1 from `apl_runtime::value::fmt_num` (see also
 * `semantic-ir-to-javascript`'s `ArrayRt.fmtNum`, the identical port already
 * shipped for the JS backend).
 */
function fmtNumApl(x: number): string {
  if (Number.isNaN(x)) {
    return "NaN";
  }
  if (!Number.isFinite(x)) {
    return x < 0 ? "¯∞" : "∞";
  }
  const body = String(Math.abs(x));
  return x < 0 ? "¯" + body : body;
}

/** Runtime shape guard for {@link NDArrayLike} — see that type's doc comment. */
function isNdArrayLike(v: unknown): v is NDArrayLike {
  return (
    v !== null &&
    typeof v === "object" &&
    Array.isArray((v as { shape?: unknown }).shape) &&
    (v as { data?: unknown }).data instanceof Float64Array
  );
}

/**
 * Render `a` the way an APL session echoes a bare (auto-printed) result —
 * ported 1:1 from `apl_runtime::value::display` (see
 * `semantic-ir-to-javascript`'s `ArrayRt.display`, the identical port
 * already shipped for the JS backend, including its column-major indexing
 * — `col * rows + row`, matching `sir-runtime-array`'s own `get`).
 *
 * - rank 0 (scalar): the one number.
 * - rank 1 (vector): elements, space-separated, on one line (the empty
 *   vector prints as the empty string).
 * - rank 2 (matrix): one row per line, elements space-separated and
 *   right-aligned to the widest cell's width in this display.
 * - rank > 2: no APL display convention is defined yet (matches the JS
 *   port's own current scope) — falls back to a flat space-separated list.
 */
function displayNdArrayApl(a: NDArrayLike): string {
  const shape = a.shape;
  if (shape.length === 0) {
    return fmtNumApl(a.data[0]);
  }
  if (shape.length === 1) {
    const n = shape[0];
    if (n === 0) {
      return "";
    }
    return Array.from(a.data, fmtNumApl).join(" ");
  }
  if (shape.length === 2) {
    const rows = shape[0];
    const cols = shape[1];
    const width = Array.from(a.data, fmtNumApl).reduce((w, s) => Math.max(w, s.length), 1);
    const lines: string[] = [];
    for (let row = 0; row < rows; row++) {
      const rowCells: string[] = [];
      for (let col = 0; col < cols; col++) {
        rowCells.push(fmtNumApl(a.data[col * rows + row]).padStart(width, " "));
      }
      lines.push(rowCells.join(" "));
    }
    return lines.join("\n");
  }
  return Array.from(a.data, fmtNumApl).join(" ");
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
  // SIR22/APL: a bare number under the "apl" convention still needs the
  // high-minus glyph — a rank-0 NDArray (see below) is what most APL
  // expressions degenerately unwrap to, but an already-unboxed scalar can
  // reach here too.
  if (typeof v === "number") {
    return _displayConvention === "apl" ? fmtNumApl(v) : String(v);
  }
  // SIR22/APL: an `NDArray` (the `{ shape, data }` value
  // `@coding-adventures/sir-runtime-array` constructs) has no Ruby/Scheme
  // display convention of its own — APL auto-prints a bare top-level
  // expression and has no bracket-indexing syntax to read a computed array
  // back with (see `apl-to-semantic-ir`'s "Auto-print, not MATLAB-style
  // suppression"), so a real APL program's `print` can only ever be made
  // to work by rendering the NDArray itself.
  if (_displayConvention === "apl" && isNdArrayLike(v)) {
    return displayNdArrayApl(v);
  }
  return String(v);
}
