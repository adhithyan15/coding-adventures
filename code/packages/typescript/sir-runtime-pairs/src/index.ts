/**
 * Cons pairs — the SIR `Pair` value type (`cons` / `car` / `cdr`).
 *
 * A *pair* is the Lisp cons cell: an immutable two-field record holding a
 * `car` (first) and `cdr` (rest). Linked pairs build lists. JavaScript has
 * no native cons cell, so pairs are an SIR quirk that lives here.
 *
 * A proper list `(1 2 3)` is `cons(1, cons(2, cons(3, null)))`. Display
 * follows Lisp convention, with a dotted tail when the final `cdr` is not
 * `null`:
 *
 *     cons(1, cons(2, null))  ->  "(1 2)"
 *     cons(1, 2)              ->  "(1 . 2)"   (improper / dotted pair)
 *
 * **Extraction + injection design.** The general SIR value display lives in
 * `@coding-adventures/sir-runtime-core` (`toDisplay`). A pair wants to render
 * its elements with that richer display so a boolean inside a list prints as
 * `#t`/`#f` rather than `true`/`false`. But core *also* needs to display
 * pairs (a pair nested in some other value), so the two importing each other
 * would form a load-time cycle.
 *
 * We break the cycle by *inverting the dependency*: this package depends on
 * **nothing** and exposes a module-level display *hook*, defaulting to
 * `String`. When core is present it calls {@link setDisplay} once at import to
 * inject its `toDisplay`, and from then on pairs render as proper Lisp lists.
 * Used standalone, a pair still prints sensibly — just with `String` for each
 * element. Pairs never import core.
 *
 * ```text
 * pairs ◀───── setDisplay(toDisplay) ───── core   (core knows pairs;
 *   │                                              pairs never imports
 *   └─ depends on nothing ──────────────────────── core)
 * ```
 */

/** The SIR universal value type at this package's boundary. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Val = any;

/**
 * The current element renderer. {@link Pair.toString} calls this for every
 * element it prints. Defaults to `String`; core overwrites it via
 * {@link setDisplay}.
 */
let _display: (v: Val) => string = (v: Val): string => String(v);

/**
 * Inject the element renderer that {@link Pair} uses for its display.
 *
 * `@coding-adventures/sir-runtime-core` calls this once at import time with
 * its richer `toDisplay` so pairs render as proper Lisp lists (booleans as
 * `#t`/`#f`, nested pairs recursively, and so on). Without that injection the
 * renderer falls back to `String`, which is why this package can be used on
 * its own with no dependency on core.
 */
export function setDisplay(fn: (v: Val) => string): void {
  _display = fn;
}

/**
 * An immutable cons cell.
 *
 * `toString` renders the Lisp list display, calling the injected
 * {@link _display} hook for each element (so the same pair prints richly under
 * core and plainly standalone — see the module doc comment).
 */
export class Pair {
  constructor(
    public readonly car: Val,
    public readonly cdr: Val,
  ) {}

  toString(): string {
    // Open paren + the first element, then walk the `cdr` chain appending each
    // subsequent `car`. A non-`null` final tail is an improper (dotted) pair
    // and prints with the Lisp ` . ` separator.
    const parts: string[] = ["(", _display(this.car)];
    let rest: Val = this.cdr;
    while (rest instanceof Pair) {
      parts.push(" ", _display(rest.car));
      rest = rest.cdr;
    }
    if (rest !== null) {
      parts.push(" . ", _display(rest));
    }
    parts.push(")");
    return parts.join("");
  }
}

/** Construct a pair `(a . b)`. */
export function cons(a: Val, b: Val): Pair {
  return new Pair(a, b);
}

/** First field of a pair. Throws on a non-pair. */
export function car(p: Val): Val {
  if (!(p instanceof Pair)) {
    throw new TypeError("car on non-pair");
  }
  return p.car;
}

/** Rest field of a pair. Throws on a non-pair. */
export function cdr(p: Val): Val {
  if (!(p instanceof Pair)) {
    throw new TypeError("cdr on non-pair");
  }
  return p.cdr;
}

/** True iff `v` is a pair. */
export function isPair(v: Val): boolean {
  return v instanceof Pair;
}
