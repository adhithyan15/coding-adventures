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
 */

import { toDisplay } from "./values.js";
import type { Val } from "./values.js";

/** An immutable cons cell. */
export class Pair {
  constructor(
    public readonly car: Val,
    public readonly cdr: Val,
  ) {}

  toString(): string {
    const parts: string[] = ["(", toDisplay(this.car)];
    let rest: Val = this.cdr;
    while (rest instanceof Pair) {
      parts.push(" ", toDisplay(rest.car));
      rest = rest.cdr;
    }
    if (rest !== null) {
      parts.push(" . ", toDisplay(rest));
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
