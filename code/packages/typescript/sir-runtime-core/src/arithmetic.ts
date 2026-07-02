/**
 * SIR arithmetic and comparison.
 *
 * These differ from JavaScript's native operators in three ways that justify
 * a runtime helper rather than a bare `a + b`:
 *
 * 1. **Variadic** — `add`/`sub`/`mul` fold over any number of arguments
 *    (`add()` is `0`, `mul()` is `1`), matching the SIR builtin contract.
 * 2. **Truncating integer division** — `div` truncates toward zero
 *    (`div(7, 2) === 3`, `div(-7, 2) === -3`), where JS `/` yields a float.
 * 3. **Type-polymorphic `+`/`*`** — Ruby overloads these operators by the
 *    *runtime type of the first operand*, and all cases lower to the same SIR
 *    `_sir_plus`/`_sir_times` builtins, so the runtime helper must dispatch:
 *
 *    | Expr          | Ruby result   | Arm                         |
 *    |---------------|---------------|-----------------------------|
 *    | `1 + 2`       | `3`           | numeric fold (below)        |
 *    | `"a" + "b"`   | `"ab"`        | string concat               |
 *    | `[1] + [2]`   | `[1, 2]`      | array concat (fresh array)  |
 *    | `"ab" * 3`    | `"ababab"`    | string repeat               |
 *    | `[0] * 3`     | `[0, 0, 0]`   | array repeat                |
 *    | `[1,2] * ", "`| `"1, 2"`      | array join (separator)      |
 *
 *    Dispatch is on the concrete JS runtime tag (`typeof x === "string"`,
 *    `Array.isArray(x)`) — **never** reflection / `eval` / source-derived
 *    property access (see the dynamic-dispatch-RCE lesson). Anything that is
 *    neither a string nor an array falls through to the numeric fold unchanged.
 */

import type { Val } from "./values.js";
import { toDisplay } from "./values.js";

const num = (v: Val): number => v as number;

/**
 * Maximum element/character count a single `*` repeat may produce.
 *
 * `"ab" * 3` and `[0] * 3` are cheap, but `[0] * 1e18` (or a string times a
 * huge count) would try to allocate an astronomically large result and hang /
 * OOM the process — an uncontrolled-resource-consumption DoS (CWE-1284/CWE-770).
 * The Go and Rust sibling backends hit exactly this, so we front-load the guard
 * here: any repeat whose resulting length would exceed this cap is rejected with
 * a Ruby-shaped `ArgumentError` ("argument too big") rather than attempted.
 *
 * `Number.MAX_SAFE_INTEGER` is the ceiling past which integer arithmetic on the
 * length itself becomes lossy, so it is the natural sane cap.
 */
const MAX_REPEAT_LEN = Number.MAX_SAFE_INTEGER;

/**
 * Normalise a `*` repeat count and clamp against the size cap.
 *
 * Returns the effective (non-negative integer) count, or `0` when the repeat
 * should produce an empty result. Ruby treats a non-positive count as "empty"
 * (`"ab" * 0 == ""`, `"ab" * -1 == ""`), so a count `<= 0`, `NaN`, or a
 * non-integer/non-finite float all collapse to `0`.
 *
 * When the count is a large positive integer, we reject *before* allocating if
 * `baseLen * count` would exceed {@link MAX_REPEAT_LEN} — but only when `baseLen`
 * is non-zero (an empty receiver repeated any number of times is still empty, so
 * a huge count over an empty base does no work and must not throw).
 */
function repeatCount(rawCount: Val, baseLen: number): number {
  const n = num(rawCount);
  // Non-finite or non-integer or non-positive → empty result (Ruby semantics).
  if (!Number.isFinite(n) || !Number.isInteger(n) || n <= 0) {
    return 0;
  }
  // Empty receiver short-circuits: no work regardless of how large `n` is.
  if (baseLen === 0) {
    return n;
  }
  // Guard the multiply itself: reject anything that would overflow the cap.
  if (n > MAX_REPEAT_LEN / baseLen) {
    throw new Error("argument too big");
  }
  return n;
}

/**
 * Variadic sum; `add()` is `0`.
 *
 * When the **first** operand is a string or array, `+` is Ruby's polymorphic
 * concat rather than numeric addition (see the module truth table). Both concat
 * arms fold left-associatively over all operands, preserving the variadic
 * contract while matching Ruby's binary `+`.
 */
export function add(...args: Val[]): Val {
  if (args.length > 0) {
    const first = args[0]!;
    // String concat: render every operand through the display helper and join.
    if (typeof first === "string") {
      let s = "";
      for (const a of args) {
        s += typeof a === "string" ? a : toDisplay(a);
      }
      return s;
    }
    // Array concat: build a FRESH array (no aliasing of any input) by spreading
    // each operand's elements. `[]+[]` in bare JS yields `""` — wrong — so we
    // never rely on native `+`; we concatenate element lists explicitly.
    if (Array.isArray(first)) {
      const out: Val[] = [];
      for (const a of args) {
        if (Array.isArray(a)) {
          for (const el of a) {
            out.push(el);
          }
        } else {
          out.push(a);
        }
      }
      return out;
    }
  }
  // Numeric fold (unchanged).
  let total = 0;
  for (const a of args) {
    total += num(a);
  }
  return total;
}

/** Variadic difference; `sub(x)` negates, `sub()` is `0`. */
export function sub(...args: Val[]): Val {
  if (args.length === 0) {
    return 0;
  }
  if (args.length === 1) {
    return -num(args[0]!);
  }
  let acc = num(args[0]!);
  for (let i = 1; i < args.length; i++) {
    acc -= num(args[i]!);
  }
  return acc;
}

/**
 * Variadic product; `mul()` is `1`.
 *
 * Ruby's `*` is binary and polymorphic on the receiver (first operand):
 * - **String × Integer** → repeat the string (`"ab" * 3 == "ababab"`).
 * - **Array × Integer** → repeat the element list (`[0] * 3 == [0, 0, 0]`),
 *   producing a fresh array (no aliasing of the receiver's elements — though
 *   element *references* are shared, matching Ruby).
 * - **Array × String** → join the elements with the string separator, each
 *   element rendered via the display helper (`[1, 2] * ", " == "1, 2"`).
 * - otherwise → the existing variadic numeric fold, unchanged.
 *
 * The string/array arms use only `args[0]`/`args[1]` (binary), consistent with
 * Ruby; the numeric fold remains variadic to preserve the SIR builtin contract.
 * Both repeat arms are guarded against oversize allocation via {@link repeatCount}.
 */
export function mul(...args: Val[]): Val {
  if (args.length >= 2) {
    const first = args[0]!;
    const second = args[1]!;
    // String × Integer → repeat.
    if (typeof first === "string" && typeof second === "number") {
      // Short-circuit an empty base first: `"".repeat(hugeCount)` throws a JS
      // `RangeError` for counts past ~2^28 even though the result is "", so we
      // never call `.repeat` when there is no work to do.
      if (first.length === 0) {
        return "";
      }
      const count = repeatCount(second, first.length);
      return count <= 0 ? "" : first.repeat(count);
    }
    if (Array.isArray(first)) {
      // Array × String → join with separator.
      if (typeof second === "string") {
        return first.map((el) => toDisplay(el)).join(second);
      }
      // Array × Integer → repeat the element list into a fresh array.
      if (typeof second === "number") {
        // Short-circuit an empty receiver: repeating `[]` any number of times is
        // still `[]`, and looping `count` times (which may be huge) doing no
        // per-iteration work would be a pointless DoS.
        if (first.length === 0) {
          return [];
        }
        const count = repeatCount(second, first.length);
        if (count <= 0) {
          return [];
        }
        const out: Val[] = [];
        for (let i = 0; i < count; i++) {
          for (const el of first) {
            out.push(el);
          }
        }
        return out;
      }
    }
  }
  // Numeric fold (unchanged).
  let acc = 1;
  for (const a of args) {
    acc *= num(a);
  }
  return acc;
}

/** Variadic quotient with truncating-integer division (toward zero). */
export function div(...args: Val[]): Val {
  if (args.length === 0) {
    return 0;
  }
  let acc = num(args[0]!);
  for (let i = 1; i < args.length; i++) {
    acc = Math.trunc(acc / num(args[i]!));
  }
  return acc;
}

/** Less-than. */
export function lt(a: Val, b: Val): boolean {
  return num(a) < num(b);
}

/** Greater-than. */
export function gt(a: Val, b: Val): boolean {
  return num(a) > num(b);
}
