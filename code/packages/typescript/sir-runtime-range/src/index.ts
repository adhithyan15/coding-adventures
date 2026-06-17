/**
 * Ranges — the SIR `Range` value type (`a..b` / `a...b`, begin/endless).
 *
 * A Ruby *range* is a first-class object, not a loop: `1..5` is a value you can
 * iterate, test membership against (`r.include?(3)`), or materialise
 * (`r.to_a`). JavaScript has no range type at all, so the SIR `Range` is a
 * quirk that lives here as a per-concern runtime, exactly like the cons cell in
 * `@coding-adventures/sir-runtime-pairs`.
 *
 * A range carries three fields:
 *
 * | field       | meaning                                                       |
 * |-------------|---------------------------------------------------------------|
 * | `start`     | the low bound, or `null` for a *beginless* range `..b`         |
 * | `stop`      | the high bound, or `null` for an *endless* range `a..`         |
 * | `exclusive` | `false` for `a..b` (includes `b`); `true` for `a...b` (excl.)  |
 *
 * Membership truth table (`s` = start, `e` = stop):
 *
 * | form    | example  | `includes(v)` is true when…       |
 * |---------|----------|-----------------------------------|
 * | `s..e`  | `1..5`   | `s <= v <= e`                     |
 * | `s...e` | `1...5`  | `s <= v <  e`                     |
 * | `s..`   | `1..`    | `s <= v`            (endless)     |
 * | `..e`   | `..5`    | `v <= e`            (beginless)   |
 *
 * Iteration walks integers from `start` upward. An *endless* range yields
 * forever (consume it lazily); a *beginless* range has no first element, so
 * iterating one (or calling {@link toList} on any unbounded range) throws a
 * `TypeError` rather than hanging — mirroring Ruby, where `(..5).each` raises.
 *
 * This package depends on **nothing** (numeric ranges need no richer display).
 *
 * See `code/specs/sir-runtime.md`.
 */

/** The SIR universal value type at this package's boundary. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Val = any;

/**
 * An immutable Ruby-style range value.
 *
 * Construct via {@link range} (the SIR backend emits a call to it). A `Range`
 * is iterable (`for (const x of r)`), supports membership via {@link includes},
 * and renders in Ruby's `a..b` / `a...b` notation through `toString`.
 */
export class Range {
  readonly start: Val;
  readonly stop: Val;
  readonly exclusive: boolean;

  constructor(start: Val, stop: Val, exclusive: Val) {
    this.start = start;
    this.stop = stop;
    // Coerce to a real boolean so `range(1, 5, null)` behaves like `..`.
    this.exclusive = Boolean(exclusive);
  }

  /**
   * Iterate integers upward from `start`. A beginless range has no first
   * element and throws; an endless range yields forever (consume lazily).
   */
  *[Symbol.iterator](): Iterator<Val> {
    if (this.start === null) {
      throw new TypeError("cannot iterate a beginless range (no start)");
    }
    let value = this.start;
    if (this.stop === null) {
      // Endless range: yield forever. Callers must consume lazily.
      for (;;) {
        yield value;
        value += 1;
      }
    } else if (this.exclusive) {
      while (value < this.stop) {
        yield value;
        value += 1;
      }
    } else {
      while (value <= this.stop) {
        yield value;
        value += 1;
      }
    }
  }

  /**
   * True iff `value` falls within the range (see the module truth table). The
   * `null` bounds of begin/endless ranges drop the corresponding comparison.
   */
  includes(value: Val): boolean {
    if (this.start !== null && value < this.start) {
      return false;
    }
    if (this.stop !== null) {
      if (this.exclusive) {
        if (value >= this.stop) {
          return false;
        }
      } else if (value > this.stop) {
        return false;
      }
    }
    return true;
  }

  /**
   * Materialise the range as an array (Ruby `to_a`). Throws for an unbounded
   * range (beginless **or** endless), since neither can produce a finite list.
   */
  toList(): Val[] {
    if (this.start === null) {
      throw new TypeError("cannot convert a beginless range to a list");
    }
    if (this.stop === null) {
      throw new TypeError("cannot convert an endless range to a list");
    }
    return [...this];
  }

  toString(): string {
    // Ruby notation: ".." inclusive, "..." exclusive; an absent bound (the
    // begin/endless forms) renders as the empty string.
    const op = this.exclusive ? "..." : "..";
    const left = this.start === null ? "" : String(this.start);
    const right = this.stop === null ? "" : String(this.stop);
    return `${left}${op}${right}`;
  }
}

/**
 * Construct a {@link Range} `start..stop` (or `start...stop`).
 *
 * This is the entry point the SIR TypeScript backend targets: a Ruby `a..b`
 * lowers to `BuiltinCall("range", [a, b, false])` and the emitter renders
 * `__SirRange.range(a, b, false)`. Either bound may be `null` for the
 * begin/endless forms.
 */
export function range(start: Val, stop: Val, exclusive: Val): Range {
  return new Range(start, stop, exclusive);
}

/** Free-function form of {@link Range.includes} (Ruby `r.include?(v)`). */
export function includes(r: Range, value: Val): boolean {
  return r.includes(value);
}

/** Free-function form of {@link Range.toList} (Ruby `r.to_a`). */
export function toList(r: Range): Val[] {
  return r.toList();
}

/** True iff `value` is a {@link Range}. */
export function isRange(value: Val): boolean {
  return value instanceof Range;
}
