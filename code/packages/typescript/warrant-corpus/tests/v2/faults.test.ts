/** Fault wrappers (VER04r section 8; R70, R94). */
import { describe, expect, it } from "vitest";
import { KitError } from "../../src/v2/errors.ts";
import { compileFault } from "../../src/v2/faults.ts";
import { type OpSig, Program, readOp } from "../../src/v2/ir.ts";
import type { Value } from "../../src/v2/values.ts";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any
const TYPES = { E: { kind: "error", cases: ["BAD", "WORSE"] } };
const program = (levels = ["ir.core"]): Program => new Program({ types: TYPES, ops: {}, models: {}, modelOrder: [], levels: new Set(levels) });
const code = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.code;
    throw error;
  }
  return "none";
};
const U8 = (v: number): Json => ["lit", { type: "U8", value: String(v) }];
const op = (params: [string, string][], ret: string, extra: Json = {}): OpSig => readOp("f", { params, ret, requires: ["ir.core"], ...extra });
const row = (kind: string, fields: Json, requires = ["ir.core"]): Json => ({ fault: "x", op: "f", kind, requires, ...fields });

describe("the three wrappers", () => {
  const two = op([["a", "U8"], ["b", "U8"]], "U8");

  it("pre rewrites every parameter from the arguments as passed in, simultaneously", () => {
    const fault = compileFault(program(), row("pre", { rewrites: [{ param: "a", expr: ["var", "b"] }, { param: "b", expr: ["var", "a"] }] }), two);
    expect(fault.kind).toBe("pre");
    expect(fault.rewriteArgs([3, 4])).toEqual([4, 3]);
    expect(fault.rewriteResult([3, 4], 7)).toBe(7);
    const one = compileFault(program(), row("pre", { rewrites: [{ param: "b", expr: ["add", ["var", "a"], ["var", "b"]] }] }), two);
    expect(one.rewriteArgs([3, 4])).toEqual([3, 7]);
  });

  it("post and replace read the parameters and the result", () => {
    const post = compileFault(program(), row("post", { expr: ["add", ["var", "result"], ["var", "a"]] }), two);
    expect(post.rewriteResult([3, 4], 10)).toBe(13);
    expect(post.rewriteArgs([3, 4])).toEqual([3, 4]);
    const replace = compileFault(program(), row("replace", { expr: ["var", "b"] }), two);
    expect(replace.rewriteResult([3, 4], 10)).toBe(4);
  });

  it("types a fault with a common type, not a flow (R94), and leaves named widths unstated (R70)", () => {
    const fixed = op([["a", "Bytes<N>"]], "Bytes<N>", { width_params: ["N"] });
    const take = compileFault(program(), row("post", { expr: ["take", ["var", "result"], ["lit", { type: "Nat", value: "1" }]] }), fixed);
    expect(take.rewriteResult([Uint8Array.of(1, 2)], Uint8Array.of(1, 2))).toEqual(Uint8Array.of(1));
    const nat = op([["a", "U8"]], "Nat");
    const off = compileFault(program(), row("post", { expr: ["add", ["var", "result"], ["lit", { type: "Nat", value: "1" }]] }), nat);
    expect(off.rewriteResult([1], 5n)).toBe(6n);
    expect(code(() => compileFault(program(), row("post", { expr: ["lit", true] }), two))).toBe("ill-typed");
    expect(code(() => compileFault(program(), row("pre", { rewrites: [{ param: "a", expr: ["lit", true] }] }), two))).toBe("ill-typed");
    expect(code(() => compileFault(program(), row("replace", { expr: U8(1) }), fixed))).toBe("ill-typed");
  });

  it("refuses a malformed row with `corpus`", () => {
    for (const bad of [
      null, [], row("shred", {}), row("post", {}), row("post", { expr: U8(1), extra: 1 }), row("pre", { expr: U8(1) }), { ...row("post", { expr: U8(1) }), fault: 5 },
      { ...row("post", { expr: U8(1) }), op: "g" }, { ...row("post", { expr: U8(1) }), requires: "ir.core" }, { ...row("post", { expr: U8(1) }), requires: [1] },
      row("pre", { rewrites: [] }), row("pre", { rewrites: "a" }), row("pre", { rewrites: [5] }), row("pre", { rewrites: [{ param: "a" }] }),
      row("pre", { rewrites: [{ param: "z", expr: U8(1) }] }), row("pre", { rewrites: [{ param: "a", expr: U8(1) }, { param: "a", expr: U8(2) }] }),
      row("pre", { rewrites: [{ param: 1, expr: U8(1) }] }), { ...row("post", { expr: U8(1) }), requires: ["ir.fancy"] },
    ]) {
      expect(code(() => compileFault(program(), bad, two)), JSON.stringify(bad)).toBe("corpus");
    }
    const clash = readOp("f", { params: [["result", "U8"]], ret: "U8", requires: ["ir.core"] });
    expect(code(() => compileFault(program(), row("post", { expr: U8(1) }), clash))).toBe("corpus");
  });
});

describe("levels and the subject", () => {
  const two = op([["a", "U8"], ["b", "U8"]], "U8");
  it("is level-missing for a declared level, a tag or an op type of a level this lane lacks", () => {
    expect(code(() => compileFault(program(), row("post", { expr: U8(1) }, ["ir.core", "ir.fixed"]), two))).toBe("level-missing");
    expect(code(() => compileFault(program(), row("post", { expr: ["bit_not", ["var", "result"]] }), two))).toBe("level-missing");
    expect(code(() => compileFault(program(), row("pre", { rewrites: [{ param: "a", expr: ["bit_not", ["var", "a"]] }] }), two))).toBe("level-missing");
    expect(code(() => compileFault(program(), row("post", { expr: U8(1) }), op([["a", "(U8,U8)"]], "U8")))).toBe("level-missing");
    expect(code(() => compileFault(program(["ir.core", "ir.fixed"]), row("post", { expr: U8(1) }, ["ir.core", "ir.fixed"]), two))).toBe("none");
  });

  it("is subject-forbidden for a subject_op, and the level wins when both hold", () => {
    const call: Json = ["subject_op", "f", [U8(1), U8(2)]];
    expect(code(() => compileFault(program(), row("post", { expr: call }), two))).toBe("subject-forbidden");
    expect(code(() => compileFault(program(), row("pre", { rewrites: [{ param: "a", expr: call }] }), two))).toBe("subject-forbidden");
    expect(code(() => compileFault(program(), row("post", { expr: call }, ["ir.core", "ir.fixed"]), two))).toBe("level-missing");
  });

  it("gives a bare ok a common type with whatever stands opposite it (R97) and compares every branch (R98 a)", () => {
    const fixed = op([["a", "Bytes"]], "Result<Bytes<32>,E>");
    // a bare ok of plain Bytes against a declared Result<Bytes<32>,E>: a common type, not a flow
    const missing = compileFault(program(), row("replace", { expr: ["if", ["is_err", ["var", "result"]], ["ok", ["var", "a"]], ["var", "result"]] }), fixed);
    expect(missing.rewriteResult([Uint8Array.of(1)], { err: "BAD" })).toEqual({ ok: Uint8Array.of(1) });
    // a branch whose payload has no common type with the declared one is refused, wherever the other branch is fine
    const u8 = op([["a", "U8"]], "Result<U8,E>");
    expect(code(() => compileFault(program(), row("post", { expr: ["if", ["is_err", ["var", "result"]], ["var", "result"], ["ok", ["lit", true]]] }), u8))).toBe("ill-typed");
    expect(code(() => compileFault(program(), row("post", { expr: ["if", ["is_err", ["var", "result"]], ["ok", ["lit", true]], ["var", "result"]] }), u8))).toBe("ill-typed");
    expect(code(() => compileFault(program(), row("post", { expr: ["if", ["is_err", ["var", "result"]], ["var", "result"], ["ok", U8(1)]] }), u8))).toBe("none");
    expect(code(() => compileFault(program(), row("post", { expr: ["ok", U8(1)] }), op([["a", "U8"]], "Bool")))).toBe("ill-typed");
  });

  it("applies to values of every host form", () => {
    const bytes = op([["a", "Bytes"]], "Result<Bytes,E>");
    const missing = compileFault(program(), row("replace", { expr: ["if", ["is_err", ["var", "result"]], ["ok", ["var", "a"]], ["var", "result"]] }), bytes);
    const value = (v: Value): Value => missing.rewriteResult([Uint8Array.of(1)], v);
    expect(value({ err: "BAD" })).toEqual({ ok: Uint8Array.of(1) });
    expect(value({ ok: Uint8Array.of(2) })).toEqual({ ok: Uint8Array.of(2) });
  });
});
