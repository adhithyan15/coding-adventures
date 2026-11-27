/**
 * # The `ir.fixed` built-ins: one function per tag (VER04r Appendix A.1)
 *
 * Every fixed-width value of this lane is held in the host form of
 * `values.ts`: a `number` for a type of at most 32 bits, a `bigint` for `U64`
 * and `I64`.  A type of at most 32 bits therefore never leaves the double's
 * exact range, and a 64-bit one never touches a double at all (a double would
 * make `MAX` and `MAX ^ 1` the same number).
 *
 * ## The two ideas every operation here rests on
 *
 * **The pattern.**  A `w`-bit value is a `w`-bit pattern read in two's
 * complement when the type is signed.  `-1i8` and `255u8` are the same eight
 * bits: `11111111`.  The bitwise operations act on the *pattern* and re-read the
 * result in the operation's own type, so
 *
 *     shr(-1i8, 1):   pattern 11111111 -> logical shift -> 01111111 -> read as I8 = 127
 *
 * and `sar` (the arithmetic shift) is the one that copies the sign bit
 * instead.  A signed value read in JavaScript's own 32-bit integers is already
 * sign-extended, which is why `a & b` of two signed small values is correct
 * without any masking, and why an *unsigned* 32-bit result must be pulled
 * back with `>>> 0`.
 *
 * **Counts are exact.**  A shift count is a `Nat`, a `bigint` that may be as
 * large as 2^127, so it is compared with the width **before** any host shift
 * (`n >= w` gives 0, or the sign for `sar`; a rotation reduces `n mod w` first).
 * A host shift by 40 would silently wrap to 8.
 *
 * ## `Option` is an intermediate here
 *
 * The nine producers (`try checked_add checked_sub checked_mul div rem
 * div_trunc rem_trunc at`) return `undefined` for `None` and the value itself
 * for `Some(v)`.  Their only consumer is `unwrap_or`, directly (Appendix A.1,
 * [D23]), so no `Option` value is ever held, streamed or compared by this lane.
 */

import { type Width, type Ty } from "./types.ts";
import { fixedRange } from "./values.ts";

/** A fixed-width integer type. */
export interface Fx {
  readonly k: "U" | "I";
  readonly w: Width;
}

/** An operand or a result of a built-in: a `number`, a `bigint`, a `Uint8Array`, or `undefined` for `None`. */
export type Raw = number | bigint | Uint8Array | boolean | undefined;

/** The host form of the exact integer `v` as a value of `ty`. */
function host(ty: Fx, v: bigint): number | bigint {
  return ty.w <= 32 ? Number(v) : v;
}

function big(v: number | bigint): bigint {
  return typeof v === "bigint" ? v : BigInt(v);
}

/** The wrapping function of a narrow integer type: the value is re-read in the type after each node. */
export function smallWrap(ty: Ty): (x: number) => number {
  const t = ty as { k: "U" | "I"; w: number };
  if (t.k === "U") {
    if (t.w === 8) return (x) => x & 0xff;
    if (t.w === 16) return (x) => x & 0xffff;
    return (x) => x >>> 0;
  }
  if (t.w === 8) return (x) => (x << 24) >> 24;
  if (t.w === 16) return (x) => (x << 16) >> 16;
  return (x) => x | 0;
}

/** The unsigned pattern of a small value, as a non-negative `number`. */
function smallPattern(w: number): (v: number) => number {
  if (w === 32) return (v) => v >>> 0;
  const mask = (1 << w) - 1;
  return (v) => v & mask;
}

/** A small pattern re-read in the type. */
function smallRead(ty: Fx): (p: number) => number {
  if (ty.k === "U") return (p) => p;
  if (ty.w === 32) return (p) => p | 0;
  const shift = 32 - ty.w;
  return (p) => (p << shift) >> shift;
}

/** The 64-bit pattern of `v` and the re-reading of one. */
const pattern64 = (v: bigint): bigint => BigInt.asUintN(64, v);
const read64 = (ty: Fx): ((p: bigint) => bigint) => (ty.k === "U" ? (p) => p : (p) => BigInt.asIntN(64, p));

function popcount32(p: number): number {
  let x = p - ((p >>> 1) & 0x55555555);
  x = (x & 0x33333333) + ((x >>> 2) & 0x33333333);
  x = (x + (x >>> 4)) & 0x0f0f0f0f;
  return Math.imul(x, 0x01010101) >>> 24;
}

function popcount64(p: bigint): number {
  return popcount32(Number(p & 0xffffffffn)) + popcount32(Number(p >> 32n));
}

/** Bit length of a non-negative bigint (0 for 0). */
function bitLength(p: bigint): number {
  return p === 0n ? 0 : p.toString(2).length;
}

function ctz64(p: bigint): number {
  if (p === 0n) return 64;
  let n = 0;
  let rest = p;
  while ((rest & 1n) === 0n) {
    rest >>= 1n;
    n += 1;
  }
  return n;
}

// ---------------------------------------------------------------------------
// One operand
// ---------------------------------------------------------------------------

/**
 * The function of a one-operand tag whose operand has type `ty`.
 *
 * `target` is the static type argument of `widen wrap try from_be from_le`.
 * `Nat` results (`popcount clz ctz`) are `bigint`s, as every `Nat` is.
 */
export function unaryFixed(tag: string, ty: Fx, target?: Fx): (a: never) => Raw {
  const small = ty.w <= 32;
  const w = ty.w;
  switch (tag) {
    case "bit_not": {
      if (small) {
        const wrap = smallWrap(ty as Ty);
        return ((a: number) => wrap(~a)) as (a: never) => Raw;
      }
      const read = read64(ty);
      return ((a: bigint) => read(pattern64(~a))) as (a: never) => Raw;
    }
    case "popcount": {
      if (small) {
        const pat = smallPattern(w);
        return ((a: number) => BigInt(popcount32(pat(a)))) as (a: never) => Raw;
      }
      return ((a: bigint) => BigInt(popcount64(pattern64(a)))) as (a: never) => Raw;
    }
    case "clz": {
      if (small) {
        const pat = smallPattern(w);
        // Math.clz32 counts in 32 bits; a narrower pattern has 32 - w extra leading zeros.
        return ((a: number) => BigInt(Math.clz32(pat(a)) - (32 - w))) as (a: never) => Raw;
      }
      return ((a: bigint) => BigInt(64 - bitLength(pattern64(a)))) as (a: never) => Raw;
    }
    case "ctz": {
      if (small) {
        const pat = smallPattern(w);
        return ((a: number) => {
          const p = pat(a);
          return p === 0 ? BigInt(w) : BigInt(31 - Math.clz32(p & -p));
        }) as (a: never) => Raw;
      }
      return ((a: bigint) => BigInt(ctz64(pattern64(a)))) as (a: never) => Raw;
    }
    case "to_int":
    case "to_nat":
      return ((a: number | bigint) => big(a)) as (a: never) => Raw;
    case "be_bytes":
    case "le_bytes": {
      const count = w / 8;
      const little = tag === "le_bytes";
      return ((a: number | bigint) => {
        let p = pattern64(big(a)) & ((1n << BigInt(w)) - 1n);
        const out = new Uint8Array(count);
        for (let i = 0; i < count; i += 1) {
          out[little ? i : count - 1 - i] = Number(p & 0xffn);
          p >>= 8n;
        }
        return out;
      }) as (a: never) => Raw;
    }
    case "widen": {
      const to = target as Fx;
      return ((a: number | bigint) => host(to, big(a))) as (a: never) => Raw;
    }
    case "wrap": {
      const to = target as Fx;
      const norm = to.k === "U" ? (v: bigint) => BigInt.asUintN(to.w, v) : (v: bigint) => BigInt.asIntN(to.w, v);
      return ((a: number | bigint) => host(to, norm(big(a)))) as (a: never) => Raw;
    }
    case "try": {
      const to = target as Fx;
      const [low, high] = fixedRange(to.k, to.w);
      return ((a: number | bigint) => {
        const v = big(a);
        return v < low || v > high ? undefined : host(to, v);
      }) as (a: never) => Raw;
    }
    case "from_be":
    case "from_le": {
      const to = target as Fx;
      const little = tag === "from_le";
      const count = to.w / 8;
      return ((a: Uint8Array) => {
        // The static width is exactly `w/8` by the typer; a length that is not is a corpus error, never a truncation.
        if (a.length !== count) throw new RangeError("from_be/from_le of a value that is not w/8 bytes");
        let p = 0n;
        for (let i = 0; i < count; i += 1) p = (p << 8n) | BigInt(a[little ? count - 1 - i : i] as number);
        return host(to, to.k === "U" ? p : BigInt.asIntN(to.w, p));
      }) as (a: never) => Raw;
    }
    default:
      throw new Error(`no unary fixed-width tag ${tag}`);
  }
}

// ---------------------------------------------------------------------------
// Two operands
// ---------------------------------------------------------------------------

/** Floor division and the remainder that goes with it (the remainder has the divisor's sign). */
function floorDivMod(a: bigint, b: bigint): [bigint, bigint] {
  let q = a / b;
  let r = a % b;
  if (r !== 0n && r < 0n !== b < 0n) {
    q -= 1n;
    r += b;
  }
  return [q, r];
}

/**
 * The function of a two-operand tag whose first operand has type `ty`.  (For `div rem imin imax` on `Nat` and `Int` pass
 * `undefined`: the operands are `bigint`s.)  `shl shr sar rotl rotr bit` take an exact `Nat` count as a `bigint`.
 */
export function binaryFixed(tag: string, ty: Fx | undefined): (a: never, b: never) => Raw {
  if (tag === "imin") return ((a: bigint, b: bigint) => (a < b ? a : b)) as (a: never, b: never) => Raw;
  if (tag === "imax") return ((a: bigint, b: bigint) => (a > b ? a : b)) as (a: never, b: never) => Raw;
  if (tag === "at") {
    return ((a: Uint8Array, i: bigint) => (i < 0n || i >= BigInt(a.length) ? undefined : (a[Number(i)] as number))) as (a: never, b: never) => Raw;
  }
  if (ty === undefined) {
    // `div` and `rem` on `Nat` and `Int`: floor, `None` iff the divisor is 0.
    if (tag === "div") return ((a: bigint, b: bigint) => (b === 0n ? undefined : floorDivMod(a, b)[0])) as (a: never, b: never) => Raw;
    if (tag === "rem") return ((a: bigint, b: bigint) => (b === 0n ? undefined : floorDivMod(a, b)[1])) as (a: never, b: never) => Raw;
    throw new Error(`no exact-integer tag ${tag}`);
  }
  const small = ty.w <= 32;
  const w = ty.w;
  const W = BigInt(w);
  switch (tag) {
    case "bit_and":
    case "bit_or":
    case "bit_xor": {
      if (small) {
        const wrap = smallWrap(ty as Ty);
        if (tag === "bit_and") return ((a: number, b: number) => wrap(a & b)) as (a: never, b: never) => Raw;
        if (tag === "bit_or") return ((a: number, b: number) => wrap(a | b)) as (a: never, b: never) => Raw;
        return ((a: number, b: number) => wrap(a ^ b)) as (a: never, b: never) => Raw;
      }
      // For two values of one 64-bit type the infinite two's-complement result already lies in the type.
      if (tag === "bit_and") return ((a: bigint, b: bigint) => a & b) as (a: never, b: never) => Raw;
      if (tag === "bit_or") return ((a: bigint, b: bigint) => a | b) as (a: never, b: never) => Raw;
      return ((a: bigint, b: bigint) => a ^ b) as (a: never, b: never) => Raw;
    }
    case "shl": {
      if (small) {
        const wrap = smallWrap(ty as Ty);
        return ((a: number, n: bigint) => (n >= W ? 0 : wrap(a << Number(n)))) as (a: never, b: never) => Raw;
      }
      const norm = ty.k === "U" ? (v: bigint) => BigInt.asUintN(64, v) : (v: bigint) => BigInt.asIntN(64, v);
      return ((a: bigint, n: bigint) => (n >= W ? 0n : norm(a << n))) as (a: never, b: never) => Raw;
    }
    case "shr": {
      if (small) {
        const pat = smallPattern(w);
        const read = smallRead(ty);
        return ((a: number, n: bigint) => (n >= W ? 0 : read(pat(a) >>> Number(n)))) as (a: never, b: never) => Raw;
      }
      const read = read64(ty);
      return ((a: bigint, n: bigint) => (n >= W ? 0n : read(pattern64(a) >> n))) as (a: never, b: never) => Raw;
    }
    case "sar": {
      if (small) return ((a: number, n: bigint) => (n >= W ? (a < 0 ? -1 : 0) : a >> Number(n))) as (a: never, b: never) => Raw;
      return ((a: bigint, n: bigint) => (n >= W ? (a < 0n ? -1n : 0n) : a >> n)) as (a: never, b: never) => Raw;
    }
    case "rotl":
    case "rotr": {
      const left = tag === "rotl";
      if (small) {
        const pat = smallPattern(w);
        const read = smallRead(ty);
        const mask = w === 32 ? 0xffffffff : (1 << w) - 1;
        return ((a: number, n: bigint) => {
          const k = Number(n % W);
          if (k === 0) return a;
          // rotr by k is rotl by w - k.
          const by = left ? k : w - k;
          const p = pat(a);
          return read((((p << by) | (p >>> (w - by))) & mask) >>> 0);
        }) as (a: never, b: never) => Raw;
      }
      const read = read64(ty);
      return ((a: bigint, n: bigint) => {
        const k = n % W;
        if (k === 0n) return a;
        const by = left ? k : W - k;
        const p = pattern64(a);
        return read(pattern64((p << by) | (p >> (W - by))));
      }) as (a: never, b: never) => Raw;
    }
    case "bit": {
      if (small) {
        const pat = smallPattern(w);
        return ((a: number, i: bigint) => (i >= W ? false : ((pat(a) >>> Number(i)) & 1) === 1)) as (a: never, b: never) => Raw;
      }
      return ((a: bigint, i: bigint) => (i >= W ? false : ((pattern64(a) >> i) & 1n) === 1n)) as (a: never, b: never) => Raw;
    }
    case "checked_add":
    case "checked_sub":
    case "checked_mul": {
      const [low, high] = fixedRange(ty.k, ty.w);
      const wide = tag === "checked_mul" && w >= 32;
      if (small && !wide) {
        const lo = Number(low);
        const hi = Number(high);
        // A sum, a difference and a product of two values of at most 16 bits are exact in a double.
        const run = tag === "checked_add" ? (a: number, b: number) => a + b : tag === "checked_sub" ? (a: number, b: number) => a - b : (a: number, b: number) => a * b;
        return ((a: number, b: number) => {
          const r = run(a, b);
          // `r + 0` turns a negative zero (0 * -5) into 0: no value of this lane is ever `-0`.
          return r < lo || r > hi ? undefined : r + 0;
        }) as (a: never, b: never) => Raw;
      }
      const run = tag === "checked_add" ? (a: bigint, b: bigint) => a + b : tag === "checked_sub" ? (a: bigint, b: bigint) => a - b : (a: bigint, b: bigint) => a * b;
      return ((a: number | bigint, b: number | bigint) => {
        const r = run(big(a), big(b));
        return r < low || r > high ? undefined : host(ty, r);
      }) as (a: never, b: never) => Raw;
    }
    case "div":
    case "rem": {
      // Unsigned fixed-width: both operands non-negative, so floor and truncation agree.
      const isDiv = tag === "div";
      return ((a: number | bigint, b: number | bigint) => {
        const y = big(b);
        if (y === 0n) return undefined;
        const x = big(a);
        return host(ty, isDiv ? x / y : x % y);
      }) as (a: never, b: never) => Raw;
    }
    case "div_trunc":
    case "rem_trunc": {
      const isDiv = tag === "div_trunc";
      const [min] = fixedRange("I", ty.w);
      return ((a: number | bigint, b: number | bigint) => {
        const y = big(b);
        const x = big(a);
        if (y === 0n || (x === min && y === -1n)) return undefined;
        return host(ty, isDiv ? x / y : x % y);
      }) as (a: never, b: never) => Raw;
    }
    default:
      throw new Error(`no binary fixed-width tag ${tag}`);
  }
}

/** `max_value` and `min_value` of a type, as a host value. */
export function boundValue(tag: "max_value" | "min_value", ty: Fx): number | bigint {
  const [low, high] = fixedRange(ty.k, ty.w);
  return host(ty, tag === "max_value" ? high : low);
}
