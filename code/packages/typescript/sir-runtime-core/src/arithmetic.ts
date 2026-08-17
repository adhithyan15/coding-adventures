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

import { raiseError } from "@coding-adventures/sir-runtime-exceptions";
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
 * Extracts a shift-amount operand as a plain number, truncated toward zero
 * (matching real Ruby's own Float-shift-amount truncation). A non-number
 * operand contributes a `0` shift rather than throwing.
 */
function shiftAmountArg(v: Val): number {
  return typeof v === "number" ? Math.trunc(v) : 0;
}

/**
 * Ruby's `<<` (shift operator) — polymorphic like `add`, but dispatched
 * explicitly on the runtime tag since native `<<`/`>>` don't line up for
 * every receiver:
 *
 * | Receiver | Behaviour                                                    |
 * |----------|---------------------------------------------------------------|
 * | array    | push each RHS operand IN PLACE (never flattened — unlike      |
 * |          | `add`'s array arm, which concatenates), returns the mutated   |
 * |          | receiver. Chains left-to-right: the frontend lowers a `<<`    |
 * |          | chain (`a << 1 << 2`) to NESTED binary calls, not one flat    |
 * |          | variadic one — but since `<<` mutates and returns the SAME    |
 * |          | receiver, nesting composes exactly like a fold, so this stays |
 * |          | variadic-capable (`...args`) for a hand-built module that     |
 * |          | constructs a flat call directly.                              |
 * | string   | concatenates via the display helper (the SAME tolerant        |
 * |          | convention `add`'s string arm already uses — never throws for |
 * |          | a non-string operand).                                        |
 * | number   | bitwise shift, implemented via MULTIPLICATION/DIVISION by a   |
 * |          | power of two rather than native `<<`/`>>`: JS's native        |
 * |          | bitwise operators coerce both operands to a 32-bit integer and|
 * |          | mask the shift count to 5 bits, so `1 << 40` would silently   |
 * |          | give the wrong answer (not even close) rather than the        |
 * |          | correct `1099511627776`. This runtime's numeric model is a    |
 * |          | plain `number` everywhere (see the module doc comment), so    |
 * |          | precision degrades past `Number.MAX_SAFE_INTEGER` like `+`/`*`|
 * |          | already do, rather than saturating like the fixed-width       |
 * |          | C/Go/Rust backends. A negative amount REVERSES direction (a   |
 * |          | right shift by the absolute value, matching Ruby's `5 << -1 ==|
 * |          | 5 >> 1 == 2`); `Math.floor` on the division correctly         |
 * |          | replicates ARITHMETIC (sign-extending) right shift for a      |
 * |          | negative receiver too (floor division by a power of two IS    |
 * |          | arithmetic right shift). The zero-receiver short-circuit both |
 * |          | matches "0 shifted by anything is 0" and avoids               |
 * |          | `0 * Infinity === NaN` once `Math.pow(2, amount)` overflows to|
 * |          | `Infinity` for an extreme shift amount.                       |
 */
export function shiftLeft(...args: Val[]): Val {
  if (args.length === 0) {
    return 0;
  }
  const first = args[0]!;
  if (Array.isArray(first)) {
    for (let i = 1; i < args.length; i++) {
      first.push(args[i]!);
    }
    return first;
  }
  if (typeof first === "string") {
    let s = first;
    for (let i = 1; i < args.length; i++) {
      const a = args[i]!;
      s += typeof a === "string" ? a : toDisplay(a);
    }
    return s;
  }
  let acc = num(first);
  for (let i = 1; i < args.length; i++) {
    if (acc === 0) {
      continue;
    }
    const amount = shiftAmountArg(args[i]!);
    acc = amount < 0 ? Math.floor(acc / Math.pow(2, -amount)) : acc * Math.pow(2, amount);
  }
  return acc;
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

/**
 * Variadic quotient with truncating-integer division (toward zero).
 *
 * **Division by zero raises `ZeroDivisionError` (T2).** Ruby raises for *both*
 * integer and float division by 0 (`1 / 0` and `1.0 / 0` both raise
 * `ZeroDivisionError: divided by 0`) — unlike bare JavaScript, where `1 / 0`
 * silently yields `Infinity` (and `0 / 0` yields `NaN`). Native `/` therefore
 * never surfaces the fault, so we ADD an explicit zero-divisor check *before*
 * each division step and raise the typed `SirError` through the exceptions
 * runtime's existing `raiseError` entry point, so a Ruby
 * `rescue ZeroDivisionError` catches it identically to the reference.
 *
 * The guard sits inside the fold, so the divisor of *every* step is checked
 * (`div(10, 2, 0)` raises on the trailing 0). We test `=== 0` rather than
 * "falsy" so a legitimate non-zero divisor is never mistaken; the message is
 * Ruby's exact `"divided by 0"`.
 *
 * **SIR21 T3b-2 `div_floor` dispatches here unchanged (known limitation).**
 * The spec's `div_floor` is Ruby's own `/`: `Integer#/` FLOORS toward −∞
 * (`-7 / 2 == -4` in real Ruby), while `Float#/` true-divides. This
 * function always TRUNCATES instead (`div(-7, 2) === -3`, not `-4`) —
 * every sibling backend's `div_floor` is either a bare rename of
 * already-Ruby-floor-faithful logic, or (for `semantic-ir-to-javascript`,
 * the closest sibling to this one) built on a boxed `SirFloat`/`isFloat`
 * runtime tag that lets it dispatch floor-vs-true-divide correctly. This
 * runtime's `Val` (see the module doc comment) has NO such tag — every
 * number, whether it came from an `IntLit` or a `FloatLit`, is an
 * indistinguishable plain JS `number` by the time it reaches here — so
 * there is no way to tell "this operand is a Ruby Integer" from "this
 * operand is a Ruby Float holding a whole number" at this point, and
 * `div_floor` cannot be made floor-vs-true-divide-correct without first
 * adding value-level float tagging throughout this runtime (mirroring
 * `semantic-ir-to-javascript`'s `SirFloat`/`mkFloat`/`isFloat` — a
 * runtime-wide change touching `add`/`sub`/`mul`/comparisons/display, not
 * a division-only fix). That is out of scope for the additive SIR21
 * T3b-2 Slice 2 rollout (backend dispatch-table wiring only); `div_floor`
 * therefore deliberately inherits this pre-existing truncating behavior
 * rather than silently changing it here — matching the precedent set by
 * this same arc's `semantic-ir-to-c`/`semantic-ir-to-ruby` PRs, whose own
 * `div_floor` aliases likewise inherited THEIR backends' pre-existing
 * zero-divisor quirks rather than being retroactively "fixed" mid-slice.
 * `div_trunc`/`udiv_trunc`/`div_true` below do NOT have this problem —
 * see their own doc comments for why.
 */
export function div(...args: Val[]): Val {
  if (args.length === 0) {
    return 0;
  }
  let acc = num(args[0]!);
  for (let i = 1; i < args.length; i++) {
    const divisor = num(args[i]!);
    if (divisor === 0) {
      raiseError("ZeroDivisionError", "divided by 0");
    }
    acc = Math.trunc(acc / divisor);
  }
  return acc;
}

/**
 * SIR21 T3b-2 `div_trunc` / `udiv_trunc` — signed/unsigned truncating
 * division (rounds toward zero, matching C's integer `/`). This is
 * exactly what {@link div} above already computes — exported under its
 * own correctly-scoped name purely so this backend exposes the same four
 * division-op names every sibling backend does.
 *
 * Unlike `div_floor` (see the dispatch-table comment in `emit.rs` for why
 * that name is NOT fixed here), `div_trunc`'s spec — "always round toward
 * zero" — needs no int/float distinction to be correct: truncation
 * toward zero is the SAME operation regardless of whether an operand is
 * semantically a Ruby Integer or Float. That is what makes this a
 * genuinely well-defined function in this runtime, where {@link div}'s
 * int-vs-float polymorphism is not.
 *
 * `udiv_trunc` (the twin C/Go/Rust need bit-reinterpretation for, since a
 * tagged `u64` >= 2^63 misreads as negative in their fixed-width models)
 * computes IDENTICALLY here: this runtime's `Val` has no fixed width and
 * no separate signed/unsigned representation (see the module doc comment
 * — every number is a plain JS `number`), so there is nothing to
 * reinterpret. Both SIR names route to this one function.
 */
export function truncDiv(a: Val, b: Val): Val {
  const bn = num(b);
  if (bn === 0) {
    raiseError("ZeroDivisionError", "divided by 0");
  }
  return Math.trunc(num(a) / bn);
}

/**
 * SIR21 T3b-2 `div_true` — ALWAYS true-divides, even when both operands
 * are meant as Ruby Integers (`trueDiv(6, 3) === 2`, the plain JS number
 * — this runtime has no boxed-float type to re-tag the result with, see
 * the module doc comment). Models Python's `/`.
 *
 * Unlike {@link div}/`div_floor`, this needs no int/float distinction
 * either: it unconditionally treats both operands as numeric and
 * true-divides, so it is fully and correctly implementable in this
 * runtime's untagged numeric model — genuinely new, not a rename.
 */
export function trueDiv(a: Val, b: Val): Val {
  const bn = num(b);
  if (bn === 0) {
    raiseError("ZeroDivisionError", "divided by 0");
  }
  return num(a) / bn;
}

/** Less-than. */
export function lt(a: Val, b: Val): boolean {
  return num(a) < num(b);
}

/** Greater-than. */
export function gt(a: Val, b: Val): boolean {
  return num(a) > num(b);
}
