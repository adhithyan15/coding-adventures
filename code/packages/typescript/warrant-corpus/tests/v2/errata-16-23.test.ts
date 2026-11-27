/**
 * VER04r section 16.23 ([D77], R119 (c), R120 (b), R126): the `v2-fault` row is read step by step over every expression
 * of the row, a bare `model_call` is `model-order`, an `env` entry or a fault row that breaks the case's own schema is a
 * kit error (a harness error, never an outcome), and a type text the reader refuses is still `ill-typed`.
 */
import { describe, expect, it } from "vitest";
import { runCase } from "../../src/v2/kit.ts";
import { kitCase } from "./support.ts";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any
const clone = <X>(value: X): X => JSON.parse(JSON.stringify(value)) as X;

const post = (): Json => clone(kitCase("v2-fault/invert-true-result"));
const pre = (rewrites: Json[], requires: string[] = ["ir.core"]): Json => ({
  ...post(),
  fault: { fault: "f", kind: "pre", op: "cmp2", requires, rewrites },
  expect: {},
});
const rw = (param: string, expr: Json): Json => ({ param, expr });
const code = (c: Json): unknown => (runCase("v2-fault", c) as Json)["error"];
const subject: Json = ["subject_op", "cmp2", [["var", "a"], ["var", "b"]]];
const illTyped: Json = ["lit", true];
const deep = (n: number): Json => {
  let tree: Json = ["var", "a"];
  for (let i = 1; i < n; i += 1) tree = ["concat", tree, ["var", "b"]];
  return tree;
};

describe("R119 (c): each step runs over every expression of a pre row before the next", () => {
  it("unknown-tag, depth and model-order in a later rewrite beat a typing defect in an earlier one", () => {
    expect(code(pre([rw("a", illTyped), rw("b", ["bogus"])]))).toBe("unknown-tag");
    expect(code(pre([rw("a", illTyped), rw("b", deep(70))]))).toBe("depth");
    expect(code(pre([rw("a", illTyped), rw("b", ["model_call", "m", [["var", "a"]]])]))).toBe("model-order");
    expect(code(pre([rw("a", illTyped), rw("b", ["model_call"])]))).toBe("model-order");
  });
  it("the three codes beat a subject_op in an earlier rewrite", () => {
    expect(code(pre([rw("a", subject), rw("b", ["bogus"])]))).toBe("unknown-tag");
    expect(code(pre([rw("a", subject), rw("b", deep(70))]))).toBe("depth");
    expect(code(pre([rw("a", subject), rw("b", ["model_call", "m", []])]))).toBe("model-order");
  });
  it("level-missing of a later rewrite beats a subject_op or a typing defect before it", () => {
    expect(code(pre([rw("a", subject), rw("b", ["bit_not", ["var", "b"]])]))).toBe("level-missing");
    expect(code(pre([rw("a", illTyped), rw("b", ["bit_not", ["var", "b"]])]))).toBe("level-missing");
  });
  it("subject-forbidden of a later rewrite beats a typing defect in an earlier one", () => {
    expect(code(pre([rw("a", illTyped), rw("b", subject)]))).toBe("subject-forbidden");
  });
  it("a typing defect is reported when nothing earlier applies, in either position", () => {
    expect(code(pre([rw("a", ["var", "a"]), rw("b", illTyped)]))).toBe("ill-typed");
    expect(code(pre([rw("a", illTyped), rw("b", ["var", "b"])]))).toBe("ill-typed");
  });
  it("a clean pre row still rewrites", () => {
    const out = runCase("v2-fault", { ...pre([rw("a", ["var", "b"]), rw("b", ["var", "a"])]), args: ["01", "02"] }) as Json;
    expect(out).toEqual({ args: ["02", "01"] });
  });
  it("a post row has the same order", () => {
    expect(code({ ...post(), fault: { ...post().fault, expr: subject } })).toBe("subject-forbidden");
    expect(code({ ...post(), fault: { ...post().fault, expr: ["not", ["bogus"]] } })).toBe("unknown-tag");
  });
});

describe("R119 (c): level-missing (2) precedes the three codes (3)", () => {
  const fixed: Json = ["bit_not", ["var", "b"]];
  it("a tag of a missing level beats an unknown tag, in one expression and across a pre row", () => {
    expect(code({ ...post(), fault: { ...post().fault, expr: ["concat", fixed, ["bogus"]] } })).toBe("level-missing");
    expect(code({ ...post(), fault: { ...post().fault, expr: ["concat", ["bogus"], fixed] } })).toBe("level-missing");
    expect(code(pre([rw("a", ["bogus"]), rw("b", fixed)]))).toBe("level-missing");
    expect(code(pre([rw("a", fixed), rw("b", ["bogus"])]))).toBe("level-missing");
  });
  it("a tag of a missing level beats a nesting over 64 and a model_call", () => {
    expect(code(pre([rw("a", deep(70)), rw("b", fixed)]))).toBe("level-missing");
    expect(code(pre([rw("a", ["model_call"]), rw("b", fixed)]))).toBe("level-missing");
  });
  it("the tag list belongs to ir.list (R57), a level this runner lacks or has, never an unknown tag", () => {
    const out = code({ ...post(), fault: { ...post().fault, expr: ["list"] } });
    expect(out).not.toBe("unknown-tag");
  });
  it("an unknown tag alone is still unknown-tag", () => {
    expect(code({ ...post(), fault: { ...post().fault, expr: ["not", ["bogus"]] } })).toBe("unknown-tag");
    expect(code(pre([rw("a", ["var", "a"]), rw("b", ["bogus"])]))).toBe("unknown-tag");
  });
});

describe("a bare model_call in a fault expression", () => {
  it("is model-order, as is any model_call there (R119 (a))", () => {
    expect(code({ ...post(), fault: { ...post().fault, expr: ["model_call"] } })).toBe("model-order");
    expect(code({ ...post(), fault: { ...post().fault, expr: ["model_call", 7] } })).toBe("model-order");
    expect(code({ ...post(), fault: { ...post().fault, expr: ["model_call", "m", []] } })).toBe("model-order");
  });
});

describe("R120 (b): an env entry that breaks the case's schema is a kit error", () => {
  const base = (): Json => {
    const c = clone(kitCase("v2-ir/lit-bool-true"));
    return c;
  };
  const ir = (env: Json): Json => ({ ...base(), env, ir: ["lit", true], expect: {} });
  it("the base case is a value", () => {
    expect(runCase("v2-ir", ir([["x", { type: "U8", value: "7" }]]))).toMatchObject({ value: true });
  });
  it.each([
    ["no value", [["x", { type: "U8" }]]],
    ["no type", [["x", { value: 1 }]]],
    ["a non-text type", [["x", { type: 5, value: 1 }]]],
    ["an entry that is not a pair", [["x"]]],
    ["an entry that is not a list", ["x"]],
    ["a non-object body", [["x", 5]]],
    ["an undecodable value", [["x", { type: "U8", value: 999 }]]],
  ])("%s", (_name, env) => {
    expect(() => runCase("v2-ir", ir(env))).toThrow(Error);
  });
  it("a type text the reader refuses stays ill-typed", () => {
    expect(runCase("v2-ir", ir([["x", { type: "Int", value: 1 }]]))).toEqual({ error: "ill-typed" });
    expect(runCase("v2-ir", ir([["x", { type: "Ghost", value: 1 }]]))).toEqual({ error: "ill-typed" });
  });
});

describe("R126: a malformed fault row is a kit error, never a vector", () => {
  const withFault = (edit: (fault: Json) => void): Json => {
    const c = post();
    edit(c.fault);
    return c;
  };
  it.each<[string, (fault: Json) => void]>([
    ["an extra key", (f) => (f.extra = 1)],
    ["a missing expr", (f) => delete f.expr],
    ["a bad kind", (f) => (f.kind = "swap")],
    ["an undefined level in requires", (f) => (f.requires = ["ir.bogus"])],
    ["requires that is not a list", (f) => (f.requires = "ir.core")],
    ["an op that is not a name", (f) => (f.op = 5)],
    ["a fault that is not text", (f) => (f.fault = 5)],
  ])("%s", (_name, edit) => {
    expect(() => runCase("v2-fault", withFault(edit))).toThrow(Error);
  });
  it.each<[string, Json]>([
    ["no rewrites", []],
    ["rewrites that is not a list", "x"],
    ["a rewrite without expr", [{ param: "a" }]],
    ["a rewrite of no parameter", [rw("zzz", ["var", "a"])]],
    ["a rewrite naming a parameter twice", [rw("a", ["var", "a"]), rw("a", ["var", "b"])]],
    ["a rewrite that is not an object", [5]],
  ])("a pre row with %s", (_name, rewrites) => {
    expect(() => runCase("v2-fault", pre(rewrites))).toThrow(Error);
  });
  it("a malformed row beats a level defect (the shape is read whole)", () => {
    expect(() => runCase("v2-fault", { ...pre([rw("zzz", ["bit_not", ["var", "a"]])]), levels: [] })).toThrow(Error);
  });
});
