import { describe, expect, it } from "vitest";
import { CorpusError } from "../src/errors.ts";
import { Compiler, MAX_DEPTH, MAX_IR_COST, type SubjectCall } from "../src/ir.ts";
import { parseType, type OpSig } from "../src/values.ts";

const OPS = new Map<string, OpSig>([
  ["eq", { name: "eq", params: [["a", parseType("Bytes")]], ret: parseType("Bool"), widthParams: [], fresh: false }],
  ["fixed", { name: "fixed", params: [["a", parseType("Bytes<N>")]], ret: parseType("Bool"), widthParams: ["N"], fresh: false }],
]);
const MODELS = {
  same: { params: ["a", "b"], body: ["eq", ["var", "a"], ["var", "b"]] },
  twice: { params: ["a"], body: ["model_call", "same", [["var", "a"], ["var", "a"]]] },
  loop: { params: ["a"], body: ["model_call", "loop", [["var", "a"]]] },
  uses: { params: ["a"], body: ["subject_op", "eq", [["var", "a"]]] },
};

function evaluate(node: unknown, scope: string[] = [], env: unknown[] = [], subject: SubjectCall | null = null): unknown {
  return new Compiler(MODELS, subject, OPS).compile(node, scope)(env);
}
const refuses = (node: unknown, scope: string[] = [], subject: SubjectCall | null = null, code?: string): void => {
  try {
    new Compiler(MODELS, subject, OPS).compile(node, scope);
  } catch (error) {
    expect(error).toBeInstanceOf(CorpusError);
    if (code) expect((error as CorpusError).code).toBe(code);
    return;
  }
  throw new Error(`expected a CorpusError for ${JSON.stringify(node)}`);
};

describe("literals, variables and structure", () => {
  it("evaluates literals and variables", () => {
    expect(evaluate(["lit", true])).toBe(true);
    expect(evaluate(["lit", 7])).toBe(7n);
    expect(evaluate(["var", "b"], ["a", "b"], [1n, 2n])).toBe(2n);
    expect(evaluate(["len", ["var", "a"]], ["a"], [Uint8Array.of(1, 2, 3)])).toBe(3n);
    expect(evaluate(["ok", ["lit", 1]])).toEqual(["ok", 1n]);
    expect(evaluate(["err", "BAD"])).toEqual(["err", "BAD"]);
    expect(evaluate(["name", "X"])).toEqual(["name", "X"]);
    expect(evaluate(["not", ["lit", false]])).toBe(true);
    expect(evaluate(["if", ["lit", true], ["lit", 1], ["lit", 2]])).toBe(1n);
    expect(evaluate(["if", ["lit", false], ["lit", 1], ["lit", 2]])).toBe(2n);
  });

  it("evaluates comparisons on integers, byte strings and results", () => {
    const e = (tag: string, left: unknown, right: unknown): unknown => evaluate([tag, ["var", "a"], ["var", "b"]], ["a", "b"], [left, right]);
    expect(e("eq", 1n, 1n)).toBe(true);
    expect(e("eq", 1n, true)).toBe(false);
    expect(e("ne", 1n, 2n)).toBe(true);
    expect(e("lt", 1n, 2n)).toBe(true);
    expect(e("le", 2n, 2n)).toBe(true);
    expect(e("gt", 3n, 2n)).toBe(true);
    expect(e("ge", 1n, 2n)).toBe(false);
    expect(e("lt", Uint8Array.of(1), Uint8Array.of(2))).toBe(true);
    expect(e("lt", Uint8Array.of(1), Uint8Array.of(1, 0))).toBe(true);
    expect(e("gt", Uint8Array.of(1, 0), Uint8Array.of(1))).toBe(true);
    expect(e("ge", Uint8Array.of(1), Uint8Array.of(1))).toBe(true);
    expect(e("eq", ["ok", 1n], ["ok", 1n])).toBe(true);
    expect(e("and", true, false)).toBe(false);
    expect(e("or", false, true)).toBe(true);
    expect(e("implies", false, false)).toBe(true);
    expect(e("implies", true, false)).toBe(false);
  });

  it("is strict about types at run time", () => {
    const e = (node: unknown, ...env: unknown[]): unknown => evaluate(node, ["a", "b"], env);
    expect(() => e(["lt", ["var", "a"], ["var", "b"]], 1n, Uint8Array.of(1))).toThrow(/ordering/);
    expect(() => e(["len", ["var", "a"]], 1n, 1n)).toThrow(/byte string/);
    expect(() => e(["not", ["var", "a"]], 1n, 1n)).toThrow(/boolean/);
    expect(() => e(["if", ["var", "a"], ["lit", 1], ["lit", 2]], 1n, 1n)).toThrow(CorpusError);
    expect(() => e(["and", ["var", "a"], ["var", "b"]], 1n, true)).toThrow(CorpusError);
  });
});

describe("checked arithmetic", () => {
  const arith = (tag: string, a: unknown, b: unknown): unknown => evaluate([tag, ["var", "a"], ["var", "b"]], ["a", "b"], [a, b]);
  it("adds, subtracts and multiplies within U64", () => {
    expect(arith("add", 2n, 3n)).toBe(5n);
    expect(arith("sub", 5n, 3n)).toBe(2n);
    expect(arith("mul", 4n, 3n)).toBe(12n);
  });

  it("refuses an out-of-range result", () => {
    expect(() => arith("sub", 1n, 2n)).toThrow(/left the U64 range/);
    expect(() => arith("add", 2n ** 64n - 1n, 1n)).toThrow(/left the U64 range/);
    expect(() => arith("mul", 2n ** 63n, 2n)).toThrow(/left the U64 range/);
  });

  it("range-checks operands before computing", () => {
    expect(() => arith("mul", 2n ** 1000n, 1n)).toThrow(/operand is outside U64/);
    expect(() => arith("add", -1n, 1n)).toThrow(/operand/);
    expect(() => arith("add", true, 1n)).toThrow(/operand/);
    expect(() => arith("add", 1, 1n)).toThrow(/operand/);
  });
});

describe("models and the subject hook", () => {
  it("calls a model, including one that calls another", () => {
    expect(evaluate(["model_call", "same", [["lit", 1], ["lit", 1]]])).toBe(true);
    expect(evaluate(["model_call", "twice", [["lit", 1]]])).toBe(true);
  });

  it("dispatches subject_op through the hook, with and without a width", () => {
    const seen: unknown[] = [];
    const hook: SubjectCall = (op, args, width) => {
      seen.push([op, args, width]);
      return true;
    };
    const buf = Uint8Array.of(1);
    expect(evaluate(["subject_op", "eq", [["var", "a"]]], ["a"], [buf], hook)).toBe(true);
    expect(evaluate(["subject_op", "fixed", [["var", "a"]], [4]], ["a"], [buf], hook)).toBe(true);
    expect(seen).toEqual([["eq", [buf], undefined], ["fixed", [buf], 4]]);
  });

  it("refuses malformed or forbidden nodes at compile time", () => {
    const hook: SubjectCall = () => true;
    for (const node of [
      "x", [], [1], ["bogus"], ["lit"], ["lit", "x"], ["lit", 1.5], ["lit", null], ["var"], ["var", "zzz"], ["var", 1],
      ["len"], ["ok"], ["err"], ["err", 1], ["not"], ["if", ["lit", true]], ["eq", ["lit", 1]], ["add", ["lit", 1]],
      ["model_call", "same"], ["model_call", "nope", []], ["model_call", "same", [["lit", 1]]], ["model_call", 1, []],
      ["model_call", "loop", [["lit", 1]]], ["model_call", "uses", [["lit", 1]]],
      ["subject_op", "eq"], ["subject_op", "nope", []], ["subject_op", 1, []],
      ["subject_op", "eq", [["lit", 1]], [1]], ["subject_op", "fixed", [["lit", 1]]],
      ["subject_op", "fixed", [["lit", 1]], [1, 2]], ["subject_op", "fixed", [["lit", 1]], [-1]],
      ["subject_op", "fixed", [["lit", 1]], ["x"]], ["subject_op", "fixed", [["lit", 1]], 4],
      ["subject_op", "eq", [["lit", 1]], [1], 5],
    ]) {
      refuses(node, [], hook);
    }
    // A model or an admission expression has no subject hook.
    refuses(["subject_op", "eq", [["lit", 1]]], [], null, "unsupported-ir");
  });

  it("refuses a malformed model table", () => {
    const run = (models: Record<string, unknown>): void => {
      expect(() => new Compiler(models, null, OPS).compile(["model_call", "m", []], [])).toThrow(CorpusError);
    };
    run({ m: [] });
    run({ m: null });
    run({ m: { params: [] } });
    run({ m: { params: [], body: ["lit", 1], extra: 1 } });
    run({ m: { params: [1], body: ["lit", 1] } });
    run({ m: { params: "a", body: ["lit", 1] } });
    run({ m: { params: [], body: ["bogus"] } });
  });
});

describe("work bounds", () => {
  it("refuses nesting deeper than MAX_DEPTH", () => {
    let node: unknown = ["lit", true];
    for (let level = 0; level < MAX_DEPTH + 1; level += 1) node = ["not", node];
    refuses(node, [], null, "ir-limit");
    let ok: unknown = ["lit", true];
    for (let level = 0; level < 10; level += 1) ok = ["not", ok];
    expect(evaluate(ok)).toBe(true);
  });

  it("refuses an exponential model chain statically, before any case runs", () => {
    const models: Record<string, unknown> = { m0: { params: ["a"], body: ["var", "a"] } };
    for (let level = 1; level <= 20; level += 1) {
      models[`m${level}`] = {
        params: ["a"],
        body: ["and", ["model_call", `m${level - 1}`, [["var", "a"]]], ["model_call", `m${level - 1}`, [["var", "a"]]]],
      };
    }
    expect(() => new Compiler(models, null, OPS).compile(["model_call", "m20", [["lit", true]]], [])).toThrow(/work limit/);
    expect(MAX_IR_COST).toBe(2000);
  });

  it("turns a stack overflow from an absurdly long model chain into a corpus error", () => {
    const models: Record<string, unknown> = { m0: { params: [], body: ["lit", 1] } };
    for (let level = 1; level <= 20_000; level += 1) {
      models[`m${level}`] = { params: [], body: ["model_call", `m${level - 1}`, []] };
    }
    expect(() => new Compiler(models, null, OPS).compile(["model_call", "m20000", []], [])).toThrow(CorpusError);
  });

  it("does not treat inherited names as nodes or models", () => {
    refuses(["constructor"]);
    refuses(["toString", ["lit", 1], ["lit", 2]]);
    expect(() => new Compiler(MODELS, null, OPS).compile(["model_call", "toString", []], [])).toThrow(CorpusError);
  });
});
