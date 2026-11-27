/**
 * `ir.fixed` (VER04r Appendix A.1; slice S10f): every one of the 35 tags, its typing, its evaluation and its cost.
 *
 * The kit pins the outcomes of 380 + 19 + 15 cases; these tests pin the *rules* around them, and they check the
 * implementation against a second, deliberately naive one.  The reference below is written from the spec's own
 * sentences on exact `bigint`s only (`shr(-1i8, 1) = 127` is "logical on the pattern, then re-read"): no host shift, no
 * `number` arithmetic, no fast path.  Every operation of every 8-bit type is compared on **every** operand pair, and the
 * wider widths on the edge values of section 5.1 and every shift count that matters.  A fast path of `fixed.ts` that is
 * wrong at one value of one width cannot hide in a kit of a few hundred cases; here it has nowhere to go.
 */
import { describe, expect, it } from "vitest";
import { KitError } from "../../src/v2/errors.ts";
import { type Fx, binaryFixed, boundValue, unaryFixed } from "../../src/v2/fixed.ts";
import { type Param, Program, type ProgramInput } from "../../src/v2/ir.ts";
import { type Ty, parseType, showType } from "../../src/v2/types.ts";
import { type Value, fixedRange } from "../../src/v2/values.ts";

type Json = unknown;
const T = (text: string): Ty => parseType(text, { allowInt: true });
const LEVELS = ["ir.core", "ir.fixed", "ir.tuple"];
const lit = (type: string, value: string | number): Json => ["lit", { type, value: String(value) }];
const ERRS = { E: { kind: "error", cases: ["BAD", "WORSE"] } };

function program(over: Partial<{ ops: Json; models: Json; order: Json; levels: string[] }> = {}): Program {
  const input: ProgramInput = {
    types: ERRS,
    ops: over.ops ?? {},
    models: over.models ?? {},
    modelOrder: over.order ?? Object.keys((over.models ?? {}) as object),
    levels: new Set(over.levels ?? LEVELS),
  };
  return new Program(input);
}

function outcome(run: () => unknown): string {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.overflowClass === undefined ? error.code : `${error.code}:${error.overflowClass}`;
    throw error;
  }
  return "ok";
}

function evaluate(ir: Json, vars: [string, string, Value][] = [], p: Program = program(), role: "body" | "pure" = "pure"): { value: Value; type: string; cost: number } {
  const params: Param[] = vars.map(([name, ty]) => ({ name, ty: T(ty) }));
  const compiled = p.compileExpr(ir, params, role);
  return { value: compiled.run(vars.map(([, , v]) => v)), type: showType(compiled.ty), cost: compiled.cost };
}

// ---------------------------------------------------------------------------
// The reference: exact integers, the spec's sentences, nothing clever
// ---------------------------------------------------------------------------

interface Kind {
  readonly k: "U" | "I";
  readonly w: 8 | 16 | 32 | 64;
}
const name = (ty: Kind): string => `${ty.k}${ty.w}`;
const small = (ty: Kind): boolean => ty.w <= 32;
/** The host form of the exact value `v`. */
const hostOf = (ty: Kind, v: bigint): number | bigint => (small(ty) ? Number(v) : v);
const big = (v: number | bigint): bigint => BigInt(v);
const pat = (ty: Kind, v: bigint): bigint => BigInt.asUintN(ty.w, v);
const read = (ty: Kind, p: bigint): bigint => (ty.k === "U" ? p : BigInt.asIntN(ty.w, p));
const WIDTH = (ty: Kind): bigint => BigInt(ty.w);

const refBitwise = (tag: string, ty: Kind, a: bigint, b: bigint): bigint => {
  const x = pat(ty, a);
  const y = pat(ty, b);
  return read(ty, tag === "bit_and" ? x & y : tag === "bit_or" ? x | y : x ^ y);
};
const refShift = (tag: string, ty: Kind, a: bigint, n: bigint): bigint => {
  const w = WIDTH(ty);
  if (tag === "shl") return n >= w ? 0n : read(ty, pat(ty, a << n));
  if (tag === "shr") return n >= w ? 0n : read(ty, pat(ty, a) >> n);
  if (tag === "sar") return n >= w ? (a < 0n ? -1n : 0n) : a >> n;
  const k = n % w;
  const p = pat(ty, a);
  const left = tag === "rotl" ? k : (w - k) % w;
  return read(ty, pat(ty, (p << left) | (p >> (w - left))));
};
const bitsOf = (ty: Kind, a: bigint): string => pat(ty, a).toString(2).padStart(ty.w, "0");
const refCount = (tag: string, ty: Kind, a: bigint): bigint => {
  const bits = bitsOf(ty, a);
  if (tag === "popcount") return BigInt([...bits].filter((c) => c === "1").length);
  if (tag === "clz") return BigInt(bits.indexOf("1") < 0 ? ty.w : bits.indexOf("1"));
  return BigInt(bits.lastIndexOf("1") < 0 ? ty.w : ty.w - 1 - bits.lastIndexOf("1"));
};
const refChecked = (tag: string, ty: Kind, a: bigint, b: bigint): bigint | undefined => {
  const [low, high] = fixedRange(ty.k, ty.w);
  const r = tag === "checked_add" ? a + b : tag === "checked_sub" ? a - b : a * b;
  return r < low || r > high ? undefined : r;
};
/** Floor division with the remainder of the divisor's sign, written from the definition `a = q*b + r`, `0 <= |r| < |b|`. */
const refFloor = (a: bigint, b: bigint): [bigint, bigint] => {
  // floor(a / b) for b > 0 from the definition q*b <= a < (q+1)*b; a negative divisor flips both signs.
  const [x, y] = b < 0n ? [-a, -b] : [a, b];
  const q = x >= 0n ? x / y : -((-x + y - 1n) / y);
  return [q, a - q * b];
};

const edges = (ty: Kind): bigint[] => {
  const w = BigInt(ty.w);
  const max = (1n << w) - 1n;
  const set = new Set<bigint>();
  for (let v = 0n; v < 20n; v += 1n) set.add(v);
  for (let s = 0n; s < w; s += 1n) {
    set.add(1n << s);
    set.add(max ^ (1n << s));
    set.add((1n << s) - 1n);
  }
  set.add(max);
  const [low] = fixedRange(ty.k, ty.w);
  return [...set].map((p) => read(ty, p)).concat(ty.k === "I" ? [-1n, -2n, -3n, -255n].filter((v) => v >= low) : []);
};
const everything = (ty: Kind): bigint[] => {
  const [low, high] = fixedRange(ty.k, ty.w);
  const out: bigint[] = [];
  for (let v = low; v <= high; v += 1n) out.push(v);
  return out;
};
const COUNTS = [0n, 1n, 2n, 3n, 7n, 8n, 9n, 15n, 16n, 17n, 31n, 32n, 33n, 63n, 64n, 65n, 127n, 128n, 255n, 65536n, (1n << 127n) - 1n];

const WIDTHS = [8, 16, 32, 64] as const;
const KINDS: Kind[] = WIDTHS.flatMap((w) => [{ k: "U" as const, w }, { k: "I" as const, w }]);

describe("the reference model agrees with the spec's own examples (so the cross-checks below mean something)", () => {
  it("states the sentences of Appendix A.1", () => {
    const i8: Kind = { k: "I", w: 8 };
    expect(refShift("shr", i8, -1n, 1n)).toBe(127n);
    expect(refShift("sar", i8, -128n, 7n)).toBe(-1n);
    expect(refShift("shl", i8, 1n, 8n)).toBe(0n);
    expect(refShift("sar", i8, -5n, 100n)).toBe(-1n);
    expect(refShift("sar", i8, 5n, 100n)).toBe(0n);
    expect(refShift("rotl", { k: "U", w: 8 }, 0x81n, 1n)).toBe(0x03n);
    expect(refShift("rotr", { k: "U", w: 8 }, 0x81n, 1n)).toBe(0xc0n);
    expect(refCount("clz", i8, 0n)).toBe(8n);
    expect(refCount("ctz", i8, 0n)).toBe(8n);
    expect(refCount("popcount", i8, -1n)).toBe(8n);
    expect(refBitwise("bit_xor", i8, -1n, 1n)).toBe(-2n);
    expect(refFloor(-7n, 2n)).toEqual([-4n, 1n]);
    expect(refFloor(7n, -2n)).toEqual([-4n, -1n]);
    expect(refFloor(-7n, -2n)).toEqual([3n, -1n]);
    expect(refFloor(6n, 3n)).toEqual([2n, 0n]);
  });
});

// ---------------------------------------------------------------------------
// Every operation against the reference
// ---------------------------------------------------------------------------

describe("bitwise operations", () => {
  const ops = ["bit_and", "bit_or", "bit_xor"] as const;
  it("agree with the reference on every pair of every 8-bit type", () => {
    for (const ty of KINDS.filter((t) => t.w === 8)) {
      const values = everything(ty);
      for (const tag of ops) {
        const f = binaryFixed(tag, ty as Fx) as (a: number, b: number) => number;
        for (const a of values) for (const b of values) expect(f(Number(a), Number(b))).toBe(Number(refBitwise(tag, ty, a, b)));
      }
      const not = unaryFixed("bit_not", ty as Fx) as (a: number) => number;
      for (const a of values) expect(not(Number(a))).toBe(Number(read(ty, pat(ty, ~a))));
    }
  });

  it("agree with the reference on the edge values of the wider types", () => {
    for (const ty of KINDS.filter((t) => t.w > 8)) {
      const values = edges(ty);
      for (const tag of ops) {
        const f = binaryFixed(tag, ty as Fx) as (a: number | bigint, b: number | bigint) => number | bigint;
        for (const a of values) for (const b of values) expect(f(hostOf(ty, a), hostOf(ty, b)), `${tag} ${name(ty)} ${a} ${b}`).toBe(hostOf(ty, refBitwise(tag, ty, a, b)));
      }
      const not = unaryFixed("bit_not", ty as Fx) as (a: number | bigint) => number | bigint;
      for (const a of values) expect(not(hostOf(ty, a)), `bit_not ${name(ty)} ${a}`).toBe(hostOf(ty, read(ty, pat(ty, ~a))));
    }
  });

  it("evaluates through the typer with the spec's examples", () => {
    expect(evaluate(["bit_not", lit("I8", 0)])).toMatchObject({ value: -1, type: "I8" });
    expect(evaluate(["bit_and", lit("U8", 0xf0), lit("U8", 0x3c)])).toMatchObject({ value: 0x30, type: "U8" });
    expect(evaluate(["bit_xor", lit("U64", "18446744073709551615"), lit("U64", 1)])).toMatchObject({ value: 18446744073709551614n, type: "U64" });
    expect(evaluate(["bit_or", lit("I64", "-9223372036854775808"), lit("I64", 1)])).toMatchObject({ value: -9223372036854775807n, type: "I64" });
    expect(evaluate(["bit_and", lit("U32", "4294967295"), lit("U32", "4294967294")])).toMatchObject({ value: 4294967294, type: "U32" });
  });
});

describe("shifts and rotations", () => {
  const tags = ["shl", "shr", "sar", "rotl", "rotr"] as const;
  const f = (tag: string, ty: Kind) => binaryFixed(tag, ty as Fx) as (a: number | bigint, n: bigint) => number | bigint;

  it("agree with the reference on every value of every 8-bit type and every count that matters", () => {
    for (const ty of KINDS.filter((t) => t.w === 8)) {
      for (const tag of tags) {
        if (tag === "sar" && ty.k === "U") continue;
        const run = f(tag, ty);
        for (const a of everything(ty)) for (const n of COUNTS) expect(run(Number(a), n), `${tag} ${name(ty)} ${a} ${n}`).toBe(Number(refShift(tag, ty, a, n)));
      }
    }
  });

  it("agree with the reference on the edge values of the wider types and counts around the width", () => {
    for (const ty of KINDS.filter((t) => t.w > 8)) {
      for (const tag of tags) {
        if (tag === "sar" && ty.k === "U") continue;
        const run = f(tag, ty);
        for (const a of edges(ty)) for (const n of COUNTS) expect(run(hostOf(ty, a), n), `${tag} ${name(ty)} ${a} ${n}`).toBe(hostOf(ty, refShift(tag, ty, a, n)));
      }
    }
  });

  it("states the spec's cases: n >= w is zero (the sign for sar), a rotation reduces n mod w first", () => {
    expect(evaluate(["shr", lit("I8", -1), lit("Nat", 1)]).value).toBe(127);
    expect(evaluate(["shl", lit("U8", 1), lit("Nat", 8)]).value).toBe(0);
    expect(evaluate(["shl", lit("U64", 1), lit("Nat", 64)]).value).toBe(0n);
    expect(evaluate(["shl", lit("U64", 1), lit("Nat", 63)]).value).toBe(1n << 63n);
    expect(evaluate(["sar", lit("I8", -128), lit("Nat", 200)]).value).toBe(-1);
    expect(evaluate(["sar", lit("I64", "-9223372036854775808"), lit("Nat", 63)]).value).toBe(-1n);
    expect(evaluate(["rotl", lit("U8", 0x81), lit("Nat", 9)]).value).toBe(0x03);
    expect(evaluate(["rotr", lit("U32", 1), lit("Nat", 33)]).value).toBe(0x80000000);
    // a count past 2^64 still compares exactly (a host shift would wrap): the count is a bigint to 2^127
    expect(evaluate(["shl", lit("U64", 1), lit("Nat", "18446744073709551615")]).value).toBe(0n);
    expect(evaluate(["rotl", lit("U64", 1), lit("Nat", "18446744073709551615")]).value).toBe(1n << 63n);
  });
});

describe("typing: every operand rule of Appendix A.1", () => {
  it("refuses mixed widths, mixed signs, a non-integer, a signed sar, and a count that is not a Nat", () => {
    for (const bad of [
      ["bit_and", lit("U8", 1), lit("U16", 1)], ["bit_xor", lit("I8", 1), lit("U8", 1)], ["bit_or", ["lit", true], ["lit", true]], ["bit_and", lit("Nat", 1), lit("Nat", 1)],
      ["bit_and", lit("U8", 1)], ["bit_not", ["lit", true]], ["bit_not", lit("Nat", 1)], ["bit_not", lit("U8", 1), lit("U8", 1)],
      ["sar", lit("U8", 1), lit("Nat", 1)], ["sar", lit("U64", 1), lit("Nat", 1)],
      ["shl", lit("U8", 1), lit("Int", 1)], ["shr", lit("U8", 1), lit("U8", 1)], ["rotl", lit("U8", 1), lit("I8", 1)], ["rotr", lit("U8", 1), ["lit", true]],
      ["shl", lit("Nat", 1), lit("Nat", 1)], ["shl", lit("U8", 1), ["add", lit("Nat", 1), lit("Nat", 1)]],
      ["popcount", lit("Nat", 1)], ["clz", ["lit", true]], ["ctz", lit("Int", 1)],
      ["bit", lit("U8", 1), lit("Int", 1)], ["bit", lit("Nat", 1), lit("Nat", 1)], ["bit", lit("U8", 1), ["lit", true]],
      ["checked_sub", lit("I8", 1), lit("U8", 1)], ["checked_mul", ["lit", true], ["lit", true]], ["div_trunc", lit("I8", 1)], ["be_bytes"],
    ]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
    expect(evaluate(["sar", lit("I8", -4), lit("Nat", 1)]).value).toBe(-2);
  });
});

describe("popcount, clz, ctz and bit", () => {
  it("agree with the reference on every 8-bit value and the edge values of the wider types", () => {
    for (const ty of KINDS) {
      const values = ty.w === 8 ? everything(ty) : edges(ty);
      for (const tag of ["popcount", "clz", "ctz"]) {
        const run = unaryFixed(tag, ty as Fx) as (a: number | bigint) => bigint;
        for (const a of values) expect(run(hostOf(ty, a)), `${tag} ${name(ty)} ${a}`).toBe(refCount(tag, ty, a));
      }
      const bit = binaryFixed("bit", ty as Fx) as (a: number | bigint, i: bigint) => boolean;
      for (const a of values) {
        for (const i of [0n, 1n, 7n, 8n, 15n, 16n, 31n, 32n, 63n, 64n, 65n, 1000n, (1n << 100n)]) {
          const expected = i >= WIDTH(ty) ? false : ((pat(ty, a) >> i) & 1n) === 1n;
          expect(bit(hostOf(ty, a), i), `bit ${name(ty)} ${a} ${i}`).toBe(expected);
        }
      }
    }
  });

  it("types popcount, clz and ctz as Nat and bit as Bool", () => {
    expect(evaluate(["popcount", lit("U8", 255)])).toMatchObject({ value: 8n, type: "Nat" });
    expect(evaluate(["clz", lit("U16", 0)])).toMatchObject({ value: 16n, type: "Nat" });
    expect(evaluate(["ctz", lit("I32", "-2147483648")])).toMatchObject({ value: 31n, type: "Nat" });
    expect(evaluate(["bit", lit("I8", -1), lit("Nat", 7)])).toMatchObject({ value: true, type: "Bool" });
    expect(evaluate(["bit", lit("U8", 0), lit("Nat", 8)])).toMatchObject({ value: false, type: "Bool" });
  });
});

describe("widen, wrap, try, to_int, to_nat, max_value and min_value", () => {
  it("widen keeps the value and needs a range inside the target", () => {
    expect(evaluate(["widen", "U32", lit("U8", 200)])).toMatchObject({ value: 200, type: "U32" });
    expect(evaluate(["widen", "I16", lit("I8", -128)])).toMatchObject({ value: -128, type: "I16" });
    expect(evaluate(["widen", "I16", lit("U8", 255)])).toMatchObject({ value: 255, type: "I16" });
    expect(evaluate(["widen", "U64", lit("U32", "4294967295")])).toMatchObject({ value: 4294967295n, type: "U64" });
    expect(evaluate(["widen", "I64", lit("I32", "-2147483648")])).toMatchObject({ value: -2147483648n, type: "I64" });
    expect(evaluate(["widen", "U8", lit("U8", 7)])).toMatchObject({ value: 7, type: "U8" });
    for (const bad of [
      ["widen", "U8", lit("U16", 1)], ["widen", "U32", lit("I8", 1)], ["widen", "I8", lit("U8", 1)], ["widen", "U16", lit("I16", 1)],
      ["widen", "Bool", lit("U8", 1)], ["widen", "Nat", lit("U8", 1)], ["widen", "U32", lit("Nat", 1)], ["widen", "U32", ["lit", true]],
      ["widen", 5, lit("U8", 1)], ["widen", "U32"], ["widen", "U32", lit("U8", 1), lit("U8", 2)], ["widen", "(U8,U8)", lit("U8", 1)],
      ["widen", "Int", lit("U8", 1)],
    ]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("wrap is the value mod 2^w re-read in the target, from a fixed-width type, Int or Nat", () => {
    expect(evaluate(["wrap", "I8", lit("Int", 200)]).value).toBe(-56);
    expect(evaluate(["wrap", "U8", lit("Int", -1)]).value).toBe(255);
    expect(evaluate(["wrap", "U8", lit("Nat", "18446744073709551615")]).value).toBe(255);
    expect(evaluate(["wrap", "U64", lit("Int", -1)]).value).toBe(18446744073709551615n);
    expect(evaluate(["wrap", "I64", lit("U64", "18446744073709551615")])).toMatchObject({ value: -1n, type: "I64" });
    expect(evaluate(["wrap", "U8", lit("I32", -2)]).value).toBe(254);
    expect(evaluate(["wrap", "I16", lit("U16", 40000)]).value).toBe(-25536);
    // an exact value far outside any fixed width: the 2^127 window
    expect(evaluate(["wrap", "U8", ["mul", lit("Nat", "4294967296"), lit("Nat", "4294967297")]]).value).toBe(0);
    expect(evaluate(["wrap", "U32", ["mul", lit("Nat", "4294967296"), lit("Nat", "4294967297")]]).value).toBe(0);
    expect(evaluate(["wrap", "U64", ["mul", lit("Nat", "4294967296"), lit("Nat", "4294967297")]]).value).toBe(4294967296n);
    for (const bad of [["wrap", "Nat", lit("U8", 1)], ["wrap", "Bool", lit("U8", 1)], ["wrap", "U8", ["lit", true]], ["wrap", "Nope", lit("U8", 1)], ["wrap", "U8", ["lit", { type: "Bytes", value: "00" }]]]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("try is Some exactly when the value is in range (the Option is consumed by unwrap_or)", () => {
    const d = lit("U8", 99);
    expect(evaluate(["unwrap_or", ["try", "U8", lit("Int", 255)], d]).value).toBe(255);
    expect(evaluate(["unwrap_or", ["try", "U8", lit("Int", 256)], d]).value).toBe(99);
    expect(evaluate(["unwrap_or", ["try", "U8", lit("Int", -1)], d]).value).toBe(99);
    expect(evaluate(["unwrap_or", ["try", "I8", lit("Int", -128)], lit("I8", 1)]).value).toBe(-128);
    expect(evaluate(["unwrap_or", ["try", "I8", lit("I16", -129)], lit("I8", 1)]).value).toBe(1);
    expect(evaluate(["unwrap_or", ["try", "U64", lit("Nat", "18446744073709551615")], lit("U64", 5)]).value).toBe(18446744073709551615n);
    expect(evaluate(["unwrap_or", ["try", "U8", lit("U64", "256")], d]).value).toBe(99);
    expect(outcome(() => evaluate(["try", "Bytes", lit("U8", 1)]))).toBe("ill-typed");
  });

  it("to_int is any fixed width and to_nat an unsigned one", () => {
    expect(evaluate(["to_int", lit("I8", -3)])).toMatchObject({ value: -3n, type: "Int" });
    expect(evaluate(["to_int", lit("U64", "18446744073709551615")])).toMatchObject({ value: 18446744073709551615n, type: "Int" });
    expect(evaluate(["to_nat", lit("U32", 7)])).toMatchObject({ value: 7n, type: "Nat" });
    for (const bad of [["to_nat", lit("I8", 1)], ["to_int", ["lit", true]], ["to_nat", lit("Nat", 1)], ["to_int", lit("Nat", 1)], ["to_int"], ["to_nat", lit("U8", 1), lit("U8", 1)]]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("max_value and min_value are nodes of their own, cost 1, never folded", () => {
    for (const ty of KINDS) {
      const [low, high] = fixedRange(ty.k, ty.w);
      expect(evaluate(["max_value", name(ty)])).toMatchObject({ value: hostOf(ty, high), type: name(ty), cost: 1 });
      expect(evaluate(["min_value", name(ty)])).toMatchObject({ value: hostOf(ty, low), type: name(ty), cost: 1 });
      expect(boundValue("max_value", ty as Fx)).toBe(hostOf(ty, high));
      expect(boundValue("min_value", ty as Fx)).toBe(hostOf(ty, low));
    }
    for (const bad of [["max_value", "Nat"], ["min_value", "Bool"], ["max_value"], ["max_value", 3], ["max_value", "I64", "I8"], ["max_value", "(U8,U8)"]]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });
});

describe("checked_add, checked_sub, checked_mul", () => {
  const tags = ["checked_add", "checked_sub", "checked_mul"] as const;
  it("agree with the reference on every pair of the 8-bit types and the edges of the wider ones", () => {
    for (const ty of KINDS) {
      const values = ty.w === 8 ? everything(ty) : edges(ty);
      for (const tag of tags) {
        const f = binaryFixed(tag, ty as Fx) as (a: number | bigint, b: number | bigint) => number | bigint | undefined;
        for (const a of values) {
          for (const b of values) {
            const expected = refChecked(tag, ty, a, b);
            expect(f(hostOf(ty, a), hostOf(ty, b)), `${tag} ${name(ty)} ${a} ${b}`).toBe(expected === undefined ? undefined : hostOf(ty, expected));
          }
        }
      }
    }
  });

  it("never returns a negative zero", () => {
    const f = binaryFixed("checked_mul", { k: "I", w: 16 }) as (a: number, b: number) => number | undefined;
    expect(Object.is(f(0, -5), 0)).toBe(true);
    expect(Object.is(f(-5, 0), 0)).toBe(true);
  });

  it("is an Option, consumed by unwrap_or", () => {
    expect(evaluate(["unwrap_or", ["checked_add", lit("U8", 200), lit("U8", 55)], lit("U8", 0)]).value).toBe(255);
    expect(evaluate(["unwrap_or", ["checked_add", lit("U8", 200), lit("U8", 56)], lit("U8", 7)]).value).toBe(7);
    expect(evaluate(["unwrap_or", ["checked_sub", lit("U8", 0), lit("U8", 1)], lit("U8", 7)]).value).toBe(7);
    expect(evaluate(["unwrap_or", ["checked_mul", lit("I64", "-9223372036854775808"), lit("I64", -1)], lit("I64", 5)]).value).toBe(5n);
    expect(outcome(() => evaluate(["checked_add", lit("U8", 1), lit("U16", 1)]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["checked_add", lit("Nat", 1), lit("Nat", 1)]))).toBe("ill-typed");
  });
});

describe("div, rem, div_trunc, rem_trunc, imin, imax", () => {
  it("div and rem on unsigned fixed-width operands: None iff the divisor is 0", () => {
    for (const ty of KINDS.filter((t) => t.k === "U")) {
      const values = ty.w === 8 ? everything(ty) : edges(ty);
      for (const tag of ["div", "rem"] as const) {
        const f = binaryFixed(tag, ty as Fx) as (a: number | bigint, b: number | bigint) => number | bigint | undefined;
        for (const a of values) for (const b of values) {
          const expected = b === 0n ? undefined : hostOf(ty, tag === "div" ? a / b : a % b);
          expect(f(hostOf(ty, a), hostOf(ty, b)), `${tag} ${name(ty)} ${a} ${b}`).toBe(expected);
        }
      }
    }
  });

  it("div_trunc and rem_trunc round toward zero: None iff the divisor is 0 or MIN / -1", () => {
    for (const ty of KINDS.filter((t) => t.k === "I")) {
      const values = ty.w === 8 ? everything(ty) : edges(ty);
      const [min] = fixedRange("I", ty.w);
      for (const tag of ["div_trunc", "rem_trunc"] as const) {
        const f = binaryFixed(tag, ty as Fx) as (a: number | bigint, b: number | bigint) => number | bigint | undefined;
        for (const a of values) for (const b of values) {
          const expected = b === 0n || (a === min && b === -1n) ? undefined : hostOf(ty, tag === "div_trunc" ? a / b : a % b);
          expect(f(hostOf(ty, a), hostOf(ty, b)), `${tag} ${name(ty)} ${a} ${b}`).toBe(expected);
        }
      }
    }
  });

  it("div and rem on Int and Nat are floor and the divisor's sign, and give Option<Int>", () => {
    const d = lit("Int", 99);
    const run = (tag: string, a: string, b: string): Value => evaluate(["unwrap_or", [tag, lit("Int", a), lit("Int", b)], d]).value;
    for (const [a, b] of [["7", "2"], ["-7", "2"], ["7", "-2"], ["-7", "-2"], ["6", "3"], ["-6", "3"], ["0", "5"], ["0", "-5"], ["5", "1"], ["-5", "-1"]]) {
      const [q, r] = refFloor(BigInt(a as string), BigInt(b as string));
      expect(run("div", a as string, b as string), `div ${a} ${b}`).toBe(q);
      expect(run("rem", a as string, b as string), `rem ${a} ${b}`).toBe(r);
    }
    expect(run("div", "5", "0")).toBe(99n);
    expect(run("rem", "5", "0")).toBe(99n);
    // two Nat operands give Option<Int>; the default is an Int literal, so no widening is needed
    expect(evaluate(["unwrap_or", ["rem", lit("Nat", 5), lit("Int", 2)], lit("Int", 0)])).toMatchObject({ value: 1n, type: "Int" });
    expect(evaluate(["unwrap_or", ["div", lit("Nat", "18446744073709551615"), lit("Nat", 2)], lit("Int", 0)]).value).toBe(9223372036854775807n);
    // a Nat default widens into the Int payload
    expect(evaluate(["unwrap_or", ["div", lit("Int", 5), lit("Int", 0)], lit("Nat", 4)])).toMatchObject({ value: 4n, type: "Int" });
  });

  it("refuses a signed pair for div/rem and an unsigned pair for the _trunc forms", () => {
    for (const bad of [
      ["div", lit("I8", 1), lit("I8", 1)], ["rem", lit("I64", 1), lit("I64", 1)], ["div", lit("U8", 1), lit("U16", 1)], ["rem", lit("U8", 1), lit("Nat", 1)],
      ["div", lit("U8", 1), lit("Int", 1)], ["div", ["lit", true], ["lit", true]],
      ["div_trunc", lit("U8", 1), lit("U8", 1)], ["rem_trunc", lit("I8", 1), lit("I16", 1)], ["div_trunc", lit("Int", 1), lit("Int", 1)],
    ]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("imin and imax are exact over Int and Nat and give Int", () => {
    expect(evaluate(["imin", lit("Int", -3), lit("Nat", 2)])).toMatchObject({ value: -3n, type: "Int" });
    expect(evaluate(["imax", lit("Int", -3), lit("Nat", 2)])).toMatchObject({ value: 2n, type: "Int" });
    expect(evaluate(["imax", lit("Nat", "18446744073709551615"), lit("Nat", 1)]).value).toBe(18446744073709551615n);
    expect(evaluate(["imin", lit("Nat", 4), lit("Nat", 4)]).value).toBe(4n);
    for (const bad of [["imin", lit("U8", 1), lit("U8", 1)], ["imax", lit("Nat", 1), lit("I8", 1)], ["imax", lit("Nat", 1)]]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });
});

describe("be_bytes, le_bytes, from_be, from_le and at", () => {
  it("be_bytes and le_bytes give the two's-complement bytes, typed Bytes<w/8>", () => {
    expect(evaluate(["be_bytes", lit("U32", "16909060")])).toMatchObject({ value: Uint8Array.of(1, 2, 3, 4), type: "Bytes<4>" });
    expect(evaluate(["le_bytes", lit("U32", "16909060")])).toMatchObject({ value: Uint8Array.of(4, 3, 2, 1), type: "Bytes<4>" });
    expect(evaluate(["be_bytes", lit("I16", -2)])).toMatchObject({ value: Uint8Array.of(0xff, 0xfe), type: "Bytes<2>" });
    expect(evaluate(["le_bytes", lit("I64", -2)])).toMatchObject({ value: Uint8Array.of(0xfe, 255, 255, 255, 255, 255, 255, 255), type: "Bytes<8>" });
    expect(evaluate(["be_bytes", lit("U8", 7)])).toMatchObject({ value: Uint8Array.of(7), type: "Bytes<1>" });
    expect(outcome(() => evaluate(["be_bytes", lit("Nat", 1)]))).toBe("ill-typed");
  });

  it("from_be and from_le invert them for every width, and read a signed target in two's complement", () => {
    for (const ty of KINDS) {
      for (const a of edges(ty).filter((v) => v >= fixedRange(ty.k, ty.w)[0] && v <= fixedRange(ty.k, ty.w)[1])) {
        const host = hostOf(ty, a);
        for (const [pack, unpack] of [["be_bytes", "from_be"], ["le_bytes", "from_le"]] as const) {
          const out = evaluate([unpack, name(ty), [pack, lit(name(ty), String(a))]]);
          expect(out.value, `${unpack} ${name(ty)} ${a}`).toBe(host);
          expect(out.type).toBe(name(ty));
        }
      }
    }
    expect(evaluate(["from_be", "I8", ["lit", { type: "Bytes", value: "ff" }]]).value).toBe(-1);
    expect(evaluate(["from_le", "U16", ["lit", { type: "Bytes", value: "0102" }]]).value).toBe(0x0201);
  });

  it("from_be needs a static width of exactly w/8: a plain Bytes, a wrong width and a non-bytes are ill-typed (R51)", () => {
    const plain: [string, string, Value][] = [["b", "Bytes", Uint8Array.of(1, 2, 3, 4)]];
    expect(outcome(() => evaluate(["from_be", "U32", ["var", "b"]], plain))).toBe("ill-typed");
    expect(outcome(() => evaluate(["from_be", "U32", ["var", "b"]], [["b", "Bytes<3>", Uint8Array.of(1, 2, 3)]]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["from_be", "U32", lit("U8", 1)]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["from_le", "Nat", ["var", "b"]], plain))).toBe("ill-typed");
    // a `concat` of two literal widths is Bytes<sum>; `take` is plain Bytes
    const four = ["concat", ["lit", { type: "Bytes", value: "0102" }], ["lit", { type: "Bytes", value: "0304" }]];
    expect(evaluate(["from_be", "U32", four]).value).toBe(0x01020304);
    expect(outcome(() => evaluate(["from_be", "U32", ["take", four, lit("Nat", 4)]]))).toBe("ill-typed");
    expect(evaluate(["from_le", "U32", ["concat", ["var", "t"], ["lit", { type: "Bytes", value: "000000" }]]], [["t", "Bytes<1>", Uint8Array.of(9)]]).value).toBe(9);
  });

  it("refuses a runtime length other than w/8 as an overflow, never a truncation", () => {
    // a subject that answers with 3 bytes where the op declares Bytes<4>: the static width promised 4
    const p = program({ ops: { four: { params: [], ret: "Bytes<4>", requires: ["ir.core"] } } });
    const compiled = p.compileExpr(["from_be", "U32", ["subject_op", "four", []]], [], "body");
    p.state.subject = () => Uint8Array.of(1, 2, 3);
    expect(outcome(() => compiled.run([]))).toBe("ir-overflow:case");
    p.state.subject = () => Uint8Array.of(0, 0, 0, 5);
    expect(compiled.run([])).toBe(5);
  });

  it("at is Option<U8>: None for a negative index and for one past the end", () => {
    const b: [string, string, Value][] = [["b", "Bytes", Uint8Array.of(10, 20, 30)]];
    const at = (i: Json): Value => evaluate(["unwrap_or", ["at", ["var", "b"], i], lit("U8", 99)], b).value;
    expect(at(lit("Nat", 0))).toBe(10);
    expect(at(lit("Nat", 2))).toBe(30);
    expect(at(lit("Nat", 3))).toBe(99);
    expect(at(lit("Int", -1))).toBe(99);
    expect(at(lit("Nat", "18446744073709551615"))).toBe(99);
    expect(evaluate(["unwrap_or", ["at", ["var", "b"], lit("Nat", 1)], lit("U8", 99)], [["b", "Bytes<3>", Uint8Array.of(10, 20, 30)]])).toMatchObject({ value: 20, type: "U8" });
    for (const bad of [["at", ["var", "b"], lit("U8", 0)], ["at", lit("U8", 1), lit("Nat", 0)], ["at", ["var", "b"]]]) {
      expect(outcome(() => evaluate(bad, b)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });
});

describe("unwrap_or and the intermediate Option", () => {
  it("evaluates both operands (no short circuit): an overflow in the default is an overflow", () => {
    const boom = ["add", ["mul", lit("Nat", "18446744073709551615"), lit("Nat", "18446744073709551615")], lit("Int", 0)];
    // the default is an Int overflow, though the Option is Some: both operands are evaluated
    expect(outcome(() => evaluate(["unwrap_or", ["try", "I64", lit("Int", 1)], ["wrap", "I64", ["mul", boom, boom]]]))).toBe("ir-overflow:corpus");
  });

  it("costs 1 plus the sum of its operands, as does every fixed tag", () => {
    expect(evaluate(["unwrap_or", ["try", "U8", lit("Int", 1)], lit("U8", 0)]).cost).toBe(1 + (1 + 1) + 1);
    expect(evaluate(["bit_xor", ["bit_and", lit("U8", 1), lit("U8", 2)], lit("U8", 3)]).cost).toBe(5);
    expect(evaluate(["shl", lit("U8", 1), lit("Nat", 2)]).cost).toBe(3);
    expect(evaluate(["from_be", "U16", ["lit", { type: "Bytes", value: "0102" }]]).cost).toBe(2);
    expect(evaluate(["widen", "U16", lit("U8", 1)]).cost).toBe(2);
  });

  it("is typed on the payload: the default must fit, and the first operand must be an Option", () => {
    expect(outcome(() => evaluate(["unwrap_or", ["try", "U8", lit("Int", 1)], lit("U16", 1)]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["unwrap_or", ["try", "U8", lit("Int", 1)], ["lit", true]]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["unwrap_or", lit("U8", 1), lit("U8", 1)]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["unwrap_or", ["try", "U8", lit("Int", 1)]]))).toBe("ill-typed");
    // a Nat default widens into an Option<Int>
    expect(evaluate(["unwrap_or", ["div", lit("Int", 1), lit("Int", 1)], lit("Nat", 0)]).type).toBe("Int");
  });

  it("an Option anywhere else is the real type of ir.data: level-missing, and the one level named is ir.data", () => {
    const refusal = (ir: Json): KitError | undefined => {
      try {
        evaluate(ir);
      } catch (error) {
        return error as KitError;
      }
      return undefined;
    };
    for (const ir of [
      ["try", "U8", lit("Int", 1)],
      ["at", ["lit", { type: "Bytes", value: "00" }], lit("Nat", 0)],
      ["unwrap_or", ["if", ["lit", true], ["try", "U8", lit("Int", 1)], ["try", "U8", lit("Int", 2)]], lit("U8", 0)],
      ["eq", ["try", "U8", lit("Int", 1)], ["try", "U8", lit("Int", 1)]],
      ["tuple", [["try", "U8", lit("Int", 1)], ["lit", true]]],
      ["ok", ["div", lit("Int", 1), lit("Int", 1)]],
    ]) {
      const error = refusal(ir);
      expect([error?.code, error?.level], JSON.stringify(ir)).toEqual(["level-missing", "ir.data"]);
    }
    // the consumer's own operand is fine, whatever stands under it
    expect(refusal(["unwrap_or", ["checked_add", lit("U8", 1), lit("U8", 1)], lit("U8", 0)])).toBeUndefined();
  });

  it("makes a model whose body is an Option not-live, and refuses (as level-missing) what reaches it; an uncalled one is harmless", () => {
    const models = {
      opt: { params: [["a", "U8"]], body: ["checked_add", ["var", "a"], lit("U8", 1)] },
      ok: { params: [["a", "U8"]], body: ["unwrap_or", ["checked_add", ["var", "a"], lit("U8", 1)], lit("U8", 0)] },
      calls: { params: [["a", "U8"]], body: ["unwrap_or", ["model_call", "opt", [["var", "a"]]], lit("U8", 0)] },
    };
    const p = program({ models, order: ["opt", "ok", "calls"] });
    // the model that is fine is compiled and runs
    expect(evaluate(["model_call", "ok", [lit("U8", 255)]], [], p).value).toBe(0);
    for (const target of ["opt", "calls"]) {
      const error = (() => {
        try {
          evaluate(["model_call", target, [lit("U8", 1)]], [], p);
        } catch (e) {
          return e as KitError;
        }
        return undefined;
      })();
      expect([error?.code, error?.level], target).toEqual(["level-missing", "ir.data"]);
    }
  });
});

describe("the levels of the tags", () => {
  it("a program without ir.fixed refuses every ir.fixed tag as level-missing, one without ir.tuple every ir.tuple tag", () => {
    const noFixed = program({ levels: ["ir.core", "ir.tuple"] });
    const noTuple = program({ levels: ["ir.core", "ir.fixed"] });
    for (const ir of [["bit_not", lit("U8", 1)], ["unwrap_or", ["try", "U8", lit("Int", 1)], lit("U8", 0)], ["max_value", "U8"], ["widen", "U16", lit("U8", 1)]]) {
      expect(outcome(() => evaluate(ir, [], noFixed)), JSON.stringify(ir)).toBe("level-missing");
    }
    expect(outcome(() => evaluate(["tuple", [lit("U8", 1), lit("U8", 2)]], [], noTuple))).toBe("level-missing");
    expect(outcome(() => evaluate(["bit_not", lit("U8", 1)], [], noTuple))).toBe("ok");
    // a static type argument counts for the level (R69 (b)): a tuple type named there needs ir.tuple before anything else is said
    expect(outcome(() => evaluate(["widen", "(U8,U8)", lit("U8", 1)], [], noTuple))).toBe("level-missing");
    expect(outcome(() => evaluate(["widen", "(U8,U8)", lit("U8", 1)]))).toBe("ill-typed");
  });

  it("the cost ceiling holds for a chain of fixed tags, and an overflow of an exact intermediate has its class (R71)", () => {
    const m = (n: number): Json => ({ params: [["a", "U8"]], body: n === 0 ? ["bit_not", ["var", "a"]] : ["bit_xor", ["model_call", `m${n - 1}`, [["var", "a"]]], ["model_call", `m${n - 1}`, [["var", "a"]]]] });
    const models = Object.fromEntries([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].map((n) => [`m${n}`, m(n)]));
    const order = Object.keys(models);
    expect(outcome(() => program({ models, order }))).toBe("ir-cost");
    const pure = ["mul", ["to_int", lit("U64", "18446744073709551615")], ["to_int", lit("U64", "18446744073709551615")]];
    const big = ["mul", pure, ["mul", pure, pure]];
    expect(outcome(() => evaluate(big))).toBe("ir-overflow:corpus");
  });
});
