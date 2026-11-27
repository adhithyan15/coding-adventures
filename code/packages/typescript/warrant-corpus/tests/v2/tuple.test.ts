/**
 * `ir.tuple` (VER04r sections 2.1 to 2.5, 4.1, 5.1; Appendix A.1; R76, R94 to R99; slice S10f): the tuple literal, `project`,
 * structural `==`, `match` on a `Result`, the tuple value (JSON, TFV, stream tag `64`), the `tuple` descriptor, and the
 * common type of a fault row under a tuple.
 */
import { describe, expect, it } from "vitest";
import { buildDomain } from "../../src/v2/domains.ts";
import { KitError } from "../../src/v2/errors.ts";
import { compileFault } from "../../src/v2/faults.ts";
import { type OpSig, type Param, Program, type ProgramInput, readOp } from "../../src/v2/ir.ts";
import { type Ty, faultCommon, hasEquality, parseType, readTypeTable, showType, typeLevels } from "../../src/v2/types.ts";
import { type Value, decodeJson, decodeTfv, encodeJson, makeWriter, streamBytes, toHex, valuesEqual } from "../../src/v2/values.ts";
import { StreamSink } from "../../src/v2/values.ts";

type Json = unknown;
const T = (text: string): Ty => parseType(text, { allowInt: true });
const LEVELS = ["ir.core", "ir.fixed", "ir.tuple"];
const lit = (type: string, value: string | number): Json => ["lit", { type, value: String(value) }];
const U8 = (v: number): Json => lit("U8", v);
const TRUE: Json = ["lit", true];
const FALSE: Json = ["lit", false];
const TYPES = { E: { kind: "error", cases: ["BAD", "WORSE"] }, F: { kind: "error", cases: ["WORSE", "OTHER"] } };
const table = readTypeTable(TYPES);

function program(over: Partial<{ ops: Json; models: Json; order: Json; levels: string[]; types: Json }> = {}): Program {
  const input: ProgramInput = {
    types: over.types ?? TYPES,
    ops: over.ops ?? {},
    models: over.models ?? {},
    modelOrder: over.order ?? Object.keys((over.models ?? {}) as object),
    levels: new Set(over.levels ?? LEVELS),
  };
  return new Program(input);
}

function refusal(run: () => unknown): KitError | undefined {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error;
    throw error;
  }
  return undefined;
}
const outcome = (run: () => unknown): string => {
  const error = refusal(run);
  return error === undefined ? "ok" : error.overflowClass === undefined ? error.code : `${error.code}:${error.overflowClass}`;
};

function evaluate(ir: Json, vars: [string, string, Json][] = [], p: Program = program(), role: "body" | "pure" = "pure"): { value: Value; type: string; cost: number; calls: number } {
  const params: Param[] = vars.map(([name, ty]) => ({ name, ty: T(ty) }));
  const compiled = p.compileExpr(ir, params, role);
  const values = vars.map(([, ty, raw]) => decodeJson(raw, T(ty), p.table));
  p.state.calls = 0;
  const value = compiled.run(values);
  return { value, type: showType(compiled.ty), cost: compiled.cost, calls: p.state.calls };
}
const shown = (ir: Json, vars: [string, string, Json][] = [], p?: Program): Json => {
  const out = evaluate(ir, vars, p);
  return encodeJson(out.value, T(out.type));
};

describe("tuple types", () => {
  it("is a type of 2 to 8 components and a level of its own; the components count for the level", () => {
    expect(showType(T("(U8,(Bool,Bytes<2>),Result<U8,E>)"))).toBe("(U8,(Bool,Bytes<2>),Result<U8,E>)");
    for (const bad of ["(U8)", "()", "(U8,U8,U8,U8,U8,U8,U8,U8,U8)", "(U8,)", "(,U8)", "(U8,U8", "U8,U8)"]) expect(() => T(bad), bad).toThrow(KitError);
    const levels = (text: string): string[] => {
      const out = new Set<string>();
      typeLevels(T(text), out);
      return [...out].sort();
    };
    expect(levels("(U8,Bool)")).toEqual(["ir.core", "ir.tuple"]);
    expect(levels("(U8,Option<U8>)")).toEqual(["ir.core", "ir.data", "ir.tuple"]);
    expect(levels("Result<(U8,U8),E>")).toEqual(["ir.core", "ir.tuple"]);
  });

  it("has an equality exactly when every component has one", () => {
    expect(hasEquality(T("(U8,(Bool,Bytes))"))).toBe(true);
    expect(hasEquality(T("(Result<U8,E>,Nat)"))).toBe(true);
    expect(hasEquality(T("(U8,Option<U8>)"))).toBe(false);
    expect(hasEquality(T("(U8,List<U8>)"))).toBe(false);
    expect(hasEquality(T("Rec"))).toBe(false);
  });
});

describe("tuple values: JSON, TFV and the stream", () => {
  it("decodes a JSON array by component type, range-checking every component", () => {
    expect(decodeJson(["7", true], T("(U8,Bool)"), table)).toEqual([7, true]);
    expect(decodeJson([["1", "2"], "18446744073709551615"], T("((U8,I8),U64)"), table)).toEqual([[1, 2], 18446744073709551615n]);
    expect(decodeJson(["0a0b", { err: "BAD" }], T("(Bytes<2>,Result<U8,E>)"), table)).toEqual([Uint8Array.of(10, 11), { err: "BAD" }]);
    for (const [raw, ty] of [
      [["1"], "(U8,Bool)"], [["1", true, true], "(U8,Bool)"], [[], "(U8,Bool)"], [{ 0: "1", 1: true }, "(U8,Bool)"], ["17", "(U8,Bool)"], [null, "(U8,Bool)"],
      [["256", true], "(U8,Bool)"], [["1", "true"], "(U8,Bool)"], [["01", true], "(U8,Bool)"], [["-1", true], "(U8,Bool)"], [["0a", true], "(Bytes<2>,Bool)"],
      [[["1"], true], "((U8,U8),Bool)"], [["1", true], "((U8,U8),Bool)"], [["1", { err: "NOPE" }], "(U8,Result<U8,E>)"],
    ] as const) {
      expect(refusal(() => decodeJson(raw, T(ty), table))?.code, `${JSON.stringify(raw)} as ${ty}`).toBe("value");
    }
    // a component of a level this lane lacks names that level
    expect(refusal(() => decodeJson(["1", []], T("(U8,Option<U8>)"), table))?.level).toBe("ir.data");
  });

  it("decodes the TFV form L<n> v1..vn", () => {
    const list = (...items: unknown[]): never => ["list", items] as never;
    expect(decodeTfv(list(["int", 7n], ["bool", true]), T("(U8,Bool)"), table)).toEqual([7, true]);
    expect(decodeTfv(list(list(["int", 1n], ["int", -2n]), ["bytes", Uint8Array.of(9)]), T("((U8,I8),Bytes)"), table)).toEqual([[1, -2], Uint8Array.of(9)]);
    expect(refusal(() => decodeTfv(list(["int", 7n]), T("(U8,Bool)"), table))?.code).toBe("value");
    expect(refusal(() => decodeTfv(["int", 7n], T("(U8,Bool)"), table))?.code).toBe("value");
    expect(refusal(() => decodeTfv(list(["int", 256n], ["bool", true]), T("(U8,Bool)"), table))?.code).toBe("value");
    expect(refusal(() => decodeTfv(list(["int", 1n], ["int", 1n]), T("(U8,Option<U8>)"), table))?.code).toBe("level-missing");
  });

  it("streams tag 64, a u32-LE component count and the components, so nested tuples cannot collide", () => {
    expect(toHex(streamBytes([1, true], T("(U8,Bool)")))).toBe("640200000010010000000000000001");
    expect(toHex(streamBytes([[1, 2], true], T("((U8,U8),Bool)")))).toBe("6402000000640200000010010000000000000010020000000000000001");
    // (A,(B,C)) and ((A,B),C) are different bytes: the count and the tag make a tuple self-delimiting
    const left = toHex(streamBytes([1, [2, 3]], T("(U8,(U8,U8))")));
    const right = toHex(streamBytes([[1, 2], 3], T("((U8,U8),U8)")));
    expect(left).not.toBe(right);
    expect(toHex(streamBytes([{ ok: [1, 2] }, Uint8Array.of(7)], T("(Result<(U8,U8),E>,Bytes)")))).toBe("640200000050640200000010010000000000000010020000000000000020010000000" + "7");
  });

  it("writes through a small sink that has to flush in the middle of a tuple", () => {
    const chunks: number[] = [];
    const sink = new StreamSink((bytes) => chunks.push(bytes.length));
    const write = makeWriter(T("(Bytes,U8)"));
    for (let i = 0; i < 40; i += 1) write(sink, [new Uint8Array(3000), 1]);
    sink.flush();
    expect(chunks.reduce((sum, n) => sum + n, 0)).toBe(40 * (5 + 5 + 3000 + 9));
  });

  it("compares structurally, component by component, order mattering", () => {
    expect(valuesEqual([1, true], [1, true])).toBe(true);
    expect(valuesEqual([1, true], [1, false])).toBe(false);
    expect(valuesEqual([1, 2], [2, 1])).toBe(false);
    expect(valuesEqual([[1, 2], Uint8Array.of(1)], [[1, 2], Uint8Array.of(1)])).toBe(true);
    expect(valuesEqual([[1, 2], Uint8Array.of(1)], [[1, 2], Uint8Array.of(2)])).toBe(false);
    expect(valuesEqual([1, 2], [1, 2, 3])).toBe(false);
    expect(valuesEqual([1, 2], { ok: 1 })).toBe(false);
    expect(valuesEqual([{ ok: 1 }], [{ ok: 1 }])).toBe(true);
  });
});

describe("the tuple descriptor (section 5.1)", () => {
  const build = (descriptor: Json, type: string, types: Json = TYPES): ReturnType<typeof buildDomain> => buildDomain(descriptor, T(type), readTypeTable(types), true);
  const bool = { d: "bool", size: 2 };
  const u8 = { d: "u8_all", size: 256 };
  it("is the Cartesian product, last component fastest, nested tuples included", () => {
    const pair = build({ d: "tuple", of: [u8, bool], size: 512 }, "(U8,Bool)");
    expect(pair.size).toBe(512);
    expect([pair.nth(0), pair.nth(1), pair.nth(2), pair.nth(511)]).toEqual([[0, false], [0, true], [1, false], [255, true]]);
    const nested = build({ d: "tuple", of: [{ d: "tuple", of: [bool, bool], size: 4 }, bool], size: 8 }, "((Bool,Bool),Bool)");
    expect(nested.nth(5)).toEqual([[true, false], true]);
    const result = build({ d: "tuple", of: [{ d: "result", ok: bool, errs: ["BAD", "WORSE"], size: 4 }, bool], size: 8 }, "(Result<Bool,E>,Bool)");
    expect(result.nth(7)).toEqual([{ err: "WORSE" }, true]);
    // every fresh value is its own array: a consumer that kept one would not see the next
    expect(pair.nth(3)).not.toBe(pair.nth(3));
  });

  it("refuses a wrong size, a wrong arity, a type that is not a tuple, and a malformed node", () => {
    const refuse = (descriptor: Json, type: string): string | undefined => refusal(() => build(descriptor, type))?.code;
    expect(refuse({ d: "tuple", of: [bool, bool], size: 5 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool, bool], size: 8 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool], size: 4 }, "(Bool,Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool], size: 4 }, "Bool")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool], size: 4 }, "Result<Bool,E>")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool], size: 4 }, "(Bool,U8)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [], size: 1 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: Array.from({ length: 33 }, () => bool), size: 2 ** 33 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool] }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", size: 4 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, bool], size: 4, extra: 1 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: bool, size: 2 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: ["bool", bool], size: 4 }, "(Bool,Bool)")).toBe("descriptor");
    expect(refuse({ d: "tuple", of: [bool, { ...bool, size: 3 }], size: 6 }, "(Bool,Bool)")).toBe("descriptor");
    // a product too large for an exact count is refused, never wrapped
    const huge = { d: "bytes_n", n: 4096, size: 4 + 3 * 4096 };
    expect(refuse({ d: "tuple", of: Array.from({ length: 8 }, () => huge), size: 1 }, "(Bytes<4096>,Bytes<4096>,Bytes<4096>,Bytes<4096>,Bytes<4096>,Bytes<4096>,Bytes<4096>,Bytes<4096>)")).toBe("descriptor");
    // a scalar descriptor for a tuple type
    expect(refuse(bool, "(Bool,Bool)")).toBe("descriptor");
    // a record's descriptor is `tuple` too, and a record is ir.data
    const records = { P: { kind: "record", fields: [["x", "U8"], ["y", "Bool"]] } };
    expect(refusal(() => build({ d: "tuple", of: [u8, bool], size: 512 }, "P", records))?.level).toBe("ir.data");
  });

  it("is at most 16 levels deep, counting nodes on a path", () => {
    const deep = (levels: number): Json => {
      let node: Json = bool;
      for (let i = 0; i < levels; i += 1) node = { d: "tuple", of: [node, bool], size: 2 ** (i + 2) };
      return node;
    };
    const type = (levels: number): string => {
      let text = "Bool";
      for (let i = 0; i < levels; i += 1) text = `(${text},Bool)`;
      return text;
    };
    expect(build(deep(15), type(15)).size).toBe(2 ** 16);
    expect(refusal(() => buildDomain(deep(16), T(type(16)), table, true))?.code).toBe("descriptor");
  });
});

describe("tuple, project and ==", () => {
  it("types a literal from its components and costs 1 plus their sum", () => {
    expect(evaluate(["tuple", [U8(1), TRUE]])).toMatchObject({ value: [1, true], type: "(U8,Bool)", cost: 3 });
    expect(evaluate(["tuple", [["add", lit("Nat", 1), lit("Nat", 2)], U8(0)]])).toMatchObject({ value: [3n, 0], type: "(Int,U8)" });
    expect(evaluate(["tuple", [["tuple", [U8(1), U8(2)]], TRUE]])).toMatchObject({ value: [[1, 2], true], type: "((U8,U8),Bool)" });
    expect(evaluate(["tuple", [["lit", { type: "Bytes", value: "0a0b" }], TRUE]]).type).toBe("(Bytes<2>,Bool)");
    expect(evaluate(["tuple", [["ok", U8(1)], U8(2)]], [], program()).type).toBe("(Result<U8,?>,U8)");
    for (const bad of [["tuple"], ["tuple", []], ["tuple", [U8(1)]], ["tuple", Array.from({ length: 9 }, () => U8(1))], ["tuple", U8(1)], ["tuple", [U8(1), U8(2)], 3], ["tuple", 5]]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("projects a component, by a 0-based JSON integer below the arity", () => {
    const pair = ["tuple", [U8(7), TRUE]];
    expect(evaluate(["project", pair, 0])).toMatchObject({ value: 7, type: "U8", cost: 4 });
    expect(evaluate(["project", pair, 1])).toMatchObject({ value: true, type: "Bool" });
    expect(evaluate(["project", ["project", ["tuple", [pair, U8(2)]], 0], 0]).value).toBe(7);
    for (const bad of [["project", pair, 2], ["project", pair, -1], ["project", pair, 1.5], ["project", pair, "0"], ["project", pair], ["project", U8(1), 0], ["project", pair, 0, 1]]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("== and != are structural over tuples of one type, and refuse tuples of different types", () => {
    const vars: [string, string, Json][] = [["a", "(U8,Bool)", ["1", true]], ["b", "(U8,Bool)", ["1", false]], ["n", "(Nat,Bool)", ["1", true]], ["i", "(Int,Bool)", ["1", true]]];
    expect(evaluate(["eq", ["var", "a"], ["var", "a"]], vars).value).toBe(true);
    expect(evaluate(["eq", ["var", "a"], ["var", "b"]], vars).value).toBe(false);
    expect(evaluate(["ne", ["var", "a"], ["var", "b"]], vars).value).toBe(true);
    expect(evaluate(["eq", ["var", "a"], ["tuple", [U8(1), TRUE]]], vars).value).toBe(true);
    // a tuple is invariant (R51): (Nat,Bool) and (Int,Bool) share no type, and a tuple with a Bytes<N> and one with a Bytes neither
    expect(outcome(() => evaluate(["eq", ["var", "n"], ["var", "i"]], vars))).toBe("ill-typed");
    expect(outcome(() => evaluate(["eq", ["var", "a"], ["var", "n"]], vars))).toBe("ill-typed");
    expect(outcome(() => evaluate(["eq", ["tuple", [U8(1), U8(2)]], ["tuple", [U8(1), TRUE]]]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["eq", ["tuple", [["lit", { type: "Bytes", value: "00" }], TRUE]], ["tuple", [["var", "p"], TRUE]]], [["p", "Bytes", "00"]]))).toBe("ill-typed");
    // an order is for integers only
    expect(outcome(() => evaluate(["lt", ["var", "a"], ["var", "a"]], vars))).toBe("ill-typed");
    // a tuple of a Result compares the Result nominally
    expect(evaluate(["eq", ["var", "r"], ["var", "s"]], [["r", "(Result<U8,E>,Bool)", [{ err: "BAD" }, true]], ["s", "(Result<U8,E>,Bool)", [{ err: "WORSE" }, true]]]).value).toBe(false);
  });

  it("a tuple flows through a model parameter and result, an if, and a subject call", () => {
    const models = { swap: { params: [["t", "(U8,Bool)"]], body: ["tuple", [["project", ["var", "t"], 1], ["project", ["var", "t"], 0]]] } };
    // the swap above is ill-typed (Bool and U8 exchanged): a model whose result differs from its use is the caller's problem, not a refusal
    expect(evaluate(["model_call", "swap", [["tuple", [U8(1), TRUE]]]], [], program({ models })).type).toBe("(Bool,U8)");
    expect(outcome(() => evaluate(["model_call", "swap", [["tuple", [TRUE, U8(1)]]]], [], program({ models })))).toBe("ill-typed");
    expect(evaluate(["if", TRUE, ["tuple", [U8(1), U8(2)]], ["tuple", [U8(3), U8(4)]]]).value).toEqual([1, 2]);
    expect(outcome(() => evaluate(["if", TRUE, ["tuple", [U8(1), U8(2)]], ["tuple", [U8(3), TRUE]]]))).toBe("ill-typed");
    // a subject that answers a tuple, counted once
    const ops = { mk: { params: [["a", "U8"]], ret: "(U8,Bool)", requires: ["ir.core", "ir.tuple"] } };
    const p = program({ ops });
    const compiled = p.compileExpr(["project", ["subject_op", "mk", [U8(5)]], 0], [], "body");
    p.state.calls = 0;
    p.state.subject = (_op, args) => [args[0], true];
    expect([compiled.run([]), p.state.calls]).toEqual([5, 1]);
  });
});

describe("match on a Result (R76; Appendix A.1)", () => {
  const E = (ir: Json, vars: [string, string, Json][] = [["r", "Result<U8,E>", { ok: "7" }]], p?: Program): Json => shown(ir, vars, p);
  const arm = (pat: Json, body: Json): Json[] => [pat, body];

  it("tries the arms in order and runs exactly one", () => {
    const ir = (r: Json): [Json, [string, string, Json][]] => [
      ["match", ["var", "r"], [arm(["err", "BAD"], lit("Nat", 1)), arm(["err", "WORSE"], lit("Nat", 2)), arm(["ok", ["wild"]], lit("Nat", 0))]],
      [["r", "Result<U8,E>", r]],
    ];
    expect(E(...ir({ err: "BAD" }))).toBe("1");
    expect(E(...ir({ err: "WORSE" }))).toBe("2");
    expect(E(...ir({ ok: "5" }))).toBe("0");
    // the first matching arm wins: err_any after a case never sees that case
    const first: Json = ["match", ["var", "r"], [arm(["err", "BAD"], lit("Nat", 1)), arm(["err_any"], lit("Nat", 2)), arm(["ok", ["wild"]], lit("Nat", 0))]];
    expect(E(first, [["r", "Result<U8,E>", { err: "BAD" }]])).toBe("1");
    expect(E(first, [["r", "Result<U8,E>", { err: "WORSE" }]])).toBe("2");
  });

  it("binds the payload, the whole value, tuple components and nested Results", () => {
    expect(E(["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["var", "x"]), arm(["err_any"], U8(0))]])).toBe("7");
    expect(E(["match", ["var", "r"], [arm(["ok", ["wild"]], TRUE), arm(["bind", "w"], ["eq", ["var", "w"], ["var", "r"]])]], [["r", "Result<U8,E>", { err: "BAD" }]])).toBe(true);
    const pair: [string, string, Json][] = [["r", "Result<((U8,U8),U8),E>", { ok: [["1", "2"], "3"] }]];
    expect(E(["match", ["var", "r"], [arm(["ok", ["tuple", [["tuple", [["bind", "a"], ["wild"]]], ["bind", "c"]]]], ["add", ["var", "a"], ["var", "c"]]), arm(["err_any"], U8(0))]], pair)).toBe("4");
    const nested: [string, string, Json][] = [["r", "Result<Result<U8,E>,E>", { ok: { err: "BAD" } }]];
    expect(E(["match", ["var", "r"], [arm(["ok", ["ok", ["wild"]]], lit("Nat", 0)), arm(["ok", ["err_any"]], lit("Nat", 1)), arm(["err_any"], lit("Nat", 2))]], nested)).toBe("1");
    // an arm's bind is visible in the arm and nowhere else; a nested match binds more
    expect(E(["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["match", ["var", "s"], [arm(["ok", ["bind", "y"]], ["add", ["var", "x"], ["var", "y"]]), arm(["err_any"], U8(0))]]), arm(["err_any"], U8(0))]],
      [["r", "Result<U8,E>", { ok: "7" }], ["s", "Result<U8,E>", { ok: "3" }]])).toBe("10");
    // two matches side by side reuse the slot of their binds without interfering
    expect(E(["tuple", [["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["var", "x"]), arm(["err_any"], U8(0))]], ["match", ["var", "s"], [arm(["ok", ["bind", "y"]], ["var", "y"]), arm(["err_any"], U8(1))]]]],
      [["r", "Result<U8,E>", { ok: "7" }], ["s", "Result<U8,E>", { err: "BAD" }]])).toEqual(["7", "1"]);
  });

  it("is exhaustive, with no arm that cannot be reached, and its binds do not shadow (corpus errors, ill-typed here)", () => {
    const bad = (arms: Json[], vars?: [string, string, Json][]): string => outcome(() => E(["match", ["var", "r"], arms], vars));
    // falls through: no ok arm, an err case missing, nothing for a payload
    expect(bad([arm(["err_any"], U8(0))])).toBe("ill-typed");
    expect(bad([arm(["ok", ["wild"]], U8(0)), arm(["err", "BAD"], U8(0))])).toBe("ill-typed");
    expect(bad([arm(["ok", ["tuple", [["wild"], ["wild"]]]], U8(0)), arm(["err_any"], U8(0))], [["r", "Result<(U8,U8),E>", { ok: ["1", "2"] }]])).toBe("ok");
    expect(bad([arm(["ok", ["ok", ["wild"]]], U8(0)), arm(["err_any"], U8(0))], [["r", "Result<Result<U8,E>,E>", { ok: { ok: "1" } }]])).toBe("ill-typed");
    expect(bad([arm(["ok", ["ok", ["wild"]]], U8(0)), arm(["ok", ["err", "BAD"]], U8(0)), arm(["err_any"], U8(0))], [["r", "Result<Result<U8,E>,E>", { ok: { ok: "1" } }]])).toBe("ill-typed");
    expect(bad([arm(["ok", ["ok", ["wild"]]], U8(0)), arm(["ok", ["err", "BAD"]], U8(0)), arm(["ok", ["err", "WORSE"]], U8(0)), arm(["err_any"], U8(0))], [["r", "Result<Result<U8,E>,E>", { ok: { ok: "1" } }]])).toBe("ok");
    // every err case listed is exhaustive without err_any
    expect(bad([arm(["ok", ["wild"]], U8(0)), arm(["err", "BAD"], U8(0)), arm(["err", "WORSE"], U8(0))])).toBe("ok");
    // an arm no value reaches
    expect(bad([arm(["wild"], U8(0)), arm(["ok", ["wild"]], U8(0))])).toBe("ill-typed");
    expect(bad([arm(["ok", ["wild"]], U8(0)), arm(["ok", ["bind", "x"]], U8(0)), arm(["err_any"], U8(0))])).toBe("ill-typed");
    expect(bad([arm(["err_any"], U8(0)), arm(["err", "BAD"], U8(0)), arm(["ok", ["wild"]], U8(0))])).toBe("ill-typed");
    expect(bad([arm(["err", "BAD"], U8(0)), arm(["err", "BAD"], U8(0)), arm(["wild"], U8(0))])).toBe("ill-typed");
    // a bind that repeats a name, or shadows a variable, `result` or an outer bind
    expect(bad([arm(["ok", ["tuple", [["bind", "a"], ["bind", "a"]]]], U8(0)), arm(["err_any"], U8(0))], [["r", "Result<(U8,U8),E>", { ok: ["1", "2"] }]])).toBe("ill-typed");
    expect(bad([arm(["ok", ["bind", "r"]], U8(0)), arm(["err_any"], U8(0))])).toBe("ill-typed");
    expect(bad([arm(["ok", ["bind", "result"]], U8(0)), arm(["err_any"], U8(0))])).toBe("ill-typed");
    expect(outcome(() => E(["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["match", ["var", "r"], [arm(["ok", ["bind", "x"]], U8(0)), arm(["err_any"], U8(0))]]), arm(["err_any"], U8(0))]]))).toBe("ill-typed");
    // an unbound name outside the arm
    expect(bad([arm(["ok", ["bind", "x"]], U8(0)), arm(["err_any"], ["var", "x"])])).toBe("ill-typed");
  });

  it("checks a hostile match in bounded time: 64 error cases in each of 8 tuple components is not 64^8 work", () => {
    const cases = Array.from({ length: 64 }, (_unused, i) => `C${i}`);
    const types = { Big: { kind: "error", cases } };
    const component = "Result<U8,Big>";
    const type = `Result<(${Array.from({ length: 8 }, () => component).join(",")}),Big>`;
    const column = (i: number, pattern: Json): Json => ["ok", ["tuple", Array.from({ length: 8 }, (_unused, k) => (k === i ? pattern : ["wild"]))]];
    const arms: Json[] = [];
    for (let i = 0; i < 8; i += 1) {
      arms.push(arm(column(i, ["ok", ["wild"]]), U8(i)));
      arms.push(arm(column(i, ["err_any"]), U8(i)));
      arms.push(arm(column(i, ["err", "C3"]), U8(i)));
    }
    arms.push(arm(["err_any"], U8(0)));
    const value = { ok: Array.from({ length: 8 }, () => ({ ok: "1" })) };
    const started = Date.now();
    // exhaustive (every column's ok and err_any are there) and every later arm is unreachable: refused, and quickly
    expect(outcome(() => evaluate(["match", ["var", "r"], arms], [["r", type, value]], program({ types })))).toBe("ill-typed");
    expect(Date.now() - started).toBeLessThan(5000);
  });

  it("refuses malformed matches and patterns as ill-typed", () => {
    const vars: [string, string, Json][] = [["r", "Result<(U8,U8),E>", { ok: ["1", "2"] }]];
    const ok = arm(["ok", ["wild"]], U8(0));
    const any = arm(["err_any"], U8(0));
    for (const bad of [
      ["match", ["var", "r"]], ["match", ["var", "r"], []], ["match", ["var", "r"], "x"], ["match", ["var", "r"], [ok, any], 1], ["match", ["var", "r"], [[["wild"]], any]],
      ["match", ["var", "r"], [arm("ok", U8(0)), any]], ["match", ["var", "r"], [arm([1], U8(0)), any]], ["match", ["var", "r"], [arm(["frobnicate"], U8(0)), ok, any]],
      ["match", ["var", "r"], [arm(["wild", 1], U8(0))]], ["match", ["var", "r"], [arm(["bind"], U8(0))]], ["match", ["var", "r"], [arm(["bind", 5], U8(0))]], ["match", ["var", "r"], [arm(["bind", "1x"], U8(0))]],
      ["match", ["var", "r"], [arm(["ok"], U8(0)), any]], ["match", ["var", "r"], [ok, arm(["err"], U8(0))]], ["match", ["var", "r"], [ok, arm(["err", 5], U8(0))]],
      ["match", ["var", "r"], [ok, arm(["err", "OTHER"], U8(0)), any]], ["match", ["var", "r"], [ok, arm(["err_any", 1], U8(0))]],
      ["match", ["var", "r"], [arm(["ok", ["tuple", [["wild"]]]], U8(0)), any]], ["match", ["var", "r"], [arm(["ok", ["tuple", "x"]], U8(0)), any]],
      ["match", ["var", "r"], [arm(["ok", ["tuple", [["wild"], ["wild"], ["wild"]]]], U8(0)), any]],
      ["match", ["var", "r"], [ok, any, ...Array.from({ length: 256 }, () => any)]],
      ["match", ["var", "r"], [arm(["tuple", [["wild"], ["wild"]]], U8(0)), ok, any]],
    ]) {
      expect(outcome(() => E(bad, vars)), JSON.stringify(bad)).toBe("ill-typed");
    }
    // a pattern nested deeper than 8 is refused too
    let deep: Json = ["wild"];
    for (let i = 0; i < 9; i += 1) deep = ["ok", deep];
    let type = "U8";
    for (let i = 0; i < 9; i += 1) type = `Result<${type},E>`;
    expect(outcome(() => E(["match", ["var", "r"], [arm(deep, U8(0)), arm(["err_any"], U8(0))]], [["r", type, { err: "BAD" }]]))).toBe("ill-typed");
  });

  it("is ir.data when the scrutinee is not a Result or a pattern is a data pattern: level-missing, naming ir.data (R76)", () => {
    const vars: [string, string, Json][] = [["t", "(U8,U8)", ["1", "2"]], ["c", "Bool", true], ["r", "Result<Bool,E>", { ok: true }]];
    for (const ir of [
      ["match", ["var", "t"], [arm(["tuple", [["bind", "a"], ["bind", "b"]]], ["var", "a"])]],
      ["match", ["var", "c"], [arm(["bind", "x"], ["var", "x"])]],
      ["match", ["var", "r"], [arm(["ok", ["bool", true]], U8(1)), arm(["ok", ["bool", false]], U8(2)), arm(["err_any"], U8(3))]],
      ["match", ["var", "r"], [arm(["some", ["wild"]], U8(1)), arm(["wild"], U8(2))]],
      ["match", ["var", "r"], [arm(["ok", ["ctor", "C", []]], U8(1)), arm(["wild"], U8(2))]],
      ["match", ["var", "r"], [arm(["none"], U8(1)), arm(["wild"], U8(2))]],
    ]) {
      const error = refusal(() => evaluate(ir, vars));
      expect([error?.code, error?.level], JSON.stringify(ir)).toEqual(["level-missing", "ir.data"]);
    }
    // a lane without ir.tuple names ir.tuple for any match
    const noTuple = program({ levels: ["ir.core", "ir.fixed"] });
    expect(refusal(() => evaluate(["match", ["var", "r"], [arm(["wild"], U8(1))]], [["r", "Result<Bool,E>", { ok: true }]], noTuple))?.level).toBe("ir.tuple");
    // a model that matches a tuple is not live; one that matches a Result is
    const models = { bad: { params: [["t", "(U8,U8)"]], body: ["match", ["var", "t"], [arm(["wild"], U8(1))]] }, good: { params: [["r", "Result<U8,E>"]], body: ["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["var", "x"]), arm(["err_any"], U8(0))]] } };
    const p = program({ models, order: ["bad", "good"] });
    expect(evaluate(["model_call", "good", [["ok", U8(4)]]], [], p).value).toBe(4);
    expect(refusal(() => evaluate(["model_call", "bad", [["tuple", [U8(1), U8(2)]]]], [], p))?.level).toBe("ir.data");
  });

  it("costs 1 plus the scrutinee plus the largest arm, and short-circuits to one arm", () => {
    const big = ["add", ["add", U8(1), U8(2)], ["add", U8(3), U8(4)]];
    const ir = ["match", ["var", "r"], [arm(["ok", ["wild"]], U8(0)), arm(["err_any"], big)]];
    expect(evaluate(ir, [["r", "Result<U8,E>", { ok: "1" }]]).cost).toBe(1 + 1 + (1 + (1 + 1 + 1) + (1 + 1 + 1)));
  });

  it("counts the subject calls of the arm that runs (the pure trace) and of the scrutinee", () => {
    const ops = { bump: { params: [["a", "U8"]], ret: "U8", requires: ["ir.core"] }, res: { params: [["r", "Result<U8,E>"]], ret: "Result<U8,E>", requires: ["ir.core"] } };
    const p = program({ ops });
    const armCalls = ["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["var", "x"]), arm(["err_any"], ["subject_op", "bump", [U8(5)]])]];
    const trace = (compiled: ReturnType<Program["compileExpr"]>, env: Value[]): number => ("fixed" in compiled.trace ? compiled.trace.fixed : compiled.trace.dynamic(env));
    const compiled = p.compileExpr(armCalls, [{ name: "r", ty: T("Result<U8,E>") }], "body");
    expect(trace(compiled, [{ ok: 7 }])).toBe(0);
    expect(trace(compiled, [{ err: "BAD" }])).toBe(1);
    const scrut = p.compileExpr(["match", ["subject_op", "res", [["var", "r"]]], [arm(["ok", ["bind", "x"]], ["var", "x"]), arm(["err_any"], U8(0))]], [{ name: "r", ty: T("Result<U8,E>") }], "body");
    expect(trace(scrut, [{ ok: 7 }])).toBe(1);
    // the same count in every arm is a constant
    const same = p.compileExpr(["match", ["var", "r"], [arm(["ok", ["bind", "x"]], ["subject_op", "bump", [["var", "x"]]]), arm(["err_any"], ["subject_op", "bump", [U8(0)]])]], [{ name: "r", ty: T("Result<U8,E>") }], "body");
    expect(same.trace).toEqual({ fixed: 1 });
    // a subject call in the scrutinee and in an arm is an impure guard (section 4.4)
    expect(outcome(() => p.compileExpr(["match", ["subject_op", "res", [["var", "r"]]], [arm(["ok", ["wild"]], ["subject_op", "bump", [U8(1)]]), arm(["err_any"], U8(0))]], [{ name: "r", ty: T("Result<U8,E>") }], "body"))).toBe("impure-guard");
  });

  it("classifies an overflow in an arm by its guard (R71): corpus when the scrutinee is pure, case when it calls the subject", () => {
    const boom = ["add", ["mul", lit("Nat", "18446744073709551615"), lit("Nat", "18446744073709551615")], lit("Int", 0)];
    const ops = { res: { params: [["r", "Result<U8,E>"]], ret: "Result<U8,E>", requires: ["ir.core"] } };
    const p = program({ ops });
    const vars: Param[] = [{ name: "r", ty: T("Result<U8,E>") }];
    const pure = p.compileExpr(["match", ["var", "r"], [arm(["ok", ["wild"]], boom), arm(["err_any"], lit("Int", 0))]], vars, "body");
    expect(outcome(() => pure.run([{ ok: 1 }]))).toBe("ir-overflow:corpus");
    expect(pure.run([{ err: "BAD" }])).toBe(0n);
    const sub = p.compileExpr(["match", ["subject_op", "res", [["var", "r"]]], [arm(["ok", ["wild"]], boom), arm(["err_any"], lit("Int", 0))]], vars, "body");
    p.state.subject = (_op, args) => args[0] as Value;
    expect(outcome(() => sub.run([{ ok: 1 }]))).toBe("ir-overflow:case");
  });
});

describe("a fault row over tuples: the common type of R94 to R98", () => {
  const op = (params: [string, string][], ret: string, requires: string[] = ["ir.core", "ir.tuple"]): OpSig => readOp("f", { params, ret, requires });
  const post = (expr: Json, o: OpSig, levels = LEVELS): ReturnType<typeof compileFault> => {
    const p = program({ levels });
    return compileFault(p, { fault: "x", op: "f", kind: "post", expr, requires: ["ir.tuple"] }, o);
  };
  const result = ["var", "result"];

  it("faultCommon: Nat and Int widen under a tuple, Bytes widths do not, an undetermined component takes the other side", () => {
    expect(showType(faultCommon(T("(Bytes,Int)"), T("(Bytes,Nat)")) as Ty)).toBe("(Bytes,Int)");
    expect(faultCommon(T("(Bytes<1>,Bool)"), T("(Bytes<2>,Bool)"))).toBeNull();
    expect(faultCommon(T("(Bytes<1>,Bool)"), T("(Bytes,Bool)"))).toBeNull();
    // at the top level the two Bytes widths do unify (R94)
    expect(showType(faultCommon(T("Bytes<1>"), T("Bytes<2>")) as Ty)).toBe("Bytes");
    expect(showType(faultCommon(T("Result<Bytes<1>,E>"), T("Result<Bytes<2>,E>")) as Ty)).toBe("Result<Bytes,E>");
    // ... and not under a tuple, at any depth
    expect(faultCommon(T("(Result<Bytes<1>,E>,Bool)"), T("(Result<Bytes<2>,E>,Bool)"))).toBeNull();
    expect(faultCommon(T("((Bytes<1>,Bool),Bool)"), T("((Bytes<2>,Bool),Bool)"))).toBeNull();
    expect(faultCommon(T("(U8,U8)"), T("(U8,U8,U8)"))).toBeNull();
    expect(faultCommon(T("(U8,U8)"), T("(U8,Bool)"))).toBeNull();
    expect(faultCommon(T("(U8,U8)"), T("U8"))).toBeNull();
    const bareOk: Ty = { k: "Result", ok: T("U8"), err: null };
    expect(showType(faultCommon({ k: "Tuple", of: [bareOk, T("Bool")] }, T("(Result<U8,E>,Bool)")) as Ty)).toBe("(Result<U8,E>,Bool)");
    expect(faultCommon({ k: "Result", ok: null, err: "E" }, T("Result<(U8,U8),E>"))).toEqual(T("Result<(U8,U8),E>"));
    expect(faultCommon(T("Result<U8,E>"), T("Result<U8,F>"))).toBeNull();
    expect(showType(faultCommon({ k: "Option", of: null }, T("Option<U8>")) as Ty)).toBe("Option<U8>");
    expect(showType(faultCommon(T("Option<Nat>"), T("Option<Int>")) as Ty)).toBe("Option<Int>");
    expect(faultCommon(T("Option<U8>"), T("Option<Bool>"))).toBeNull();
  });

  it("swaps the components of a tuple result, inside an Ok and bare", () => {
    const swap = post(["tuple", [["project", result, 1], ["project", result, 0]]], op([["a", "U8"]], "(U8,U8)"));
    expect(swap.rewriteResult([1], [3, 4])).toEqual([4, 3]);
    const inOk = post(["match", result, [[["ok", ["bind", "p"]], ["ok", ["tuple", [["project", ["var", "p"], 1], ["project", ["var", "p"], 0]]]]], [["err_any"], result]]], op([["a", "U8"]], "Result<(U8,U8),E>"));
    expect(inOk.rewriteResult([1], { ok: [3, 4] })).toEqual({ ok: [4, 3] });
    expect(inOk.rewriteResult([1], { err: "BAD" })).toEqual({ err: "BAD" });
  });

  it("accepts a Nat component faulted to an Int one (off-by-one-consumed, R96) and refuses the others", () => {
    const o = op([["a", "Bytes"]], "Result<(Bytes,Nat),E>");
    const off = ["match", result, [[["ok", ["bind", "p"]], ["ok", ["tuple", [["project", ["var", "p"], 0], ["add", ["project", ["var", "p"], 1], lit("Nat", 1)]]]]], [["err_any"], result]]];
    expect(post(off, o).rewriteResult([Uint8Array.of(1)], { ok: [Uint8Array.of(1), 4n] })).toEqual({ ok: [Uint8Array.of(1), 5n] });
    // a Bool in the Nat component has no common type
    const wrong = ["match", result, [[["ok", ["bind", "p"]], ["ok", ["tuple", [["project", ["var", "p"], 0], TRUE]]]], [["err_any"], result]]];
    expect(outcome(() => post(wrong, o))).toBe("ill-typed");
    // R67: a one-byte literal in a Bytes<2> component, and a plain Bytes literal in a plain Bytes one
    expect(outcome(() => post(["tuple", [["lit", { type: "Bytes", value: "00" }], TRUE]], op([["a", "Bytes"]], "(Bytes<2>,Bool)")))).toBe("ill-typed");
    expect(post(["tuple", [["lit", { type: "Bytes", value: "0000" }], TRUE]], op([["a", "Bytes"]], "(Bytes<2>,Bool)")).rewriteResult([Uint8Array.of(1)], [Uint8Array.of(9, 9), false])).toEqual([Uint8Array.of(0, 0), true]);
    expect(post(["tuple", [["lit", { type: "Bytes", value: "" }], TRUE]], op([["a", "Bytes"]], "(Bytes,Bool)")).rewriteResult([Uint8Array.of(1)], [Uint8Array.of(9, 9), false])).toEqual([Uint8Array.of(), true]);
    // a tuple for a scalar result
    expect(outcome(() => post(["tuple", [["var", "a"], ["var", "a"]]], op([["a", "U8"]], "U8")))).toBe("ill-typed");
  });

  it("compares every branch, arm, ok payload and tuple component with the declared type (R98)", () => {
    const o = op([["a", "U8"]], "(Result<U8,E>,Bool)");
    const comp = ["project", result, 0];
    // the typer would build the type of the `if` from one branch: the other one has to be compared with the declared component
    expect(outcome(() => post(["tuple", [["if", ["is_err", comp], comp, ["ok", TRUE]], ["project", result, 1]]], o))).toBe("ill-typed");
    expect(outcome(() => post(["tuple", [["if", ["is_err", comp], comp, ["ok", U8(1)]], ["project", result, 1]]], o))).toBe("ok");
    const res = op([["a", "U8"]], "Result<U8,E>");
    expect(outcome(() => post(["match", result, [[["ok", ["bind", "p"]], ["ok", ["var", "p"]]], [["err_any"], ["ok", TRUE]]]], res))).toBe("ill-typed");
    expect(outcome(() => post(["match", result, [[["ok", ["bind", "p"]], ["ok", ["var", "p"]]], [["err_any"], result]]], res))).toBe("ok");
    expect(outcome(() => post(["ok", TRUE], res))).toBe("ill-typed");
    expect(outcome(() => post(["if", ["is_err", result], ["ok", U8(0)], result], res))).toBe("ok");
    // each rewrite of a pre fault is compared with the parameter it replaces
    const pre = (expr: Json): string => outcome(() => compileFault(program(), { fault: "x", op: "f", kind: "pre", rewrites: [{ param: "t", expr }], requires: ["ir.tuple"] }, op([["t", "(U8,Bool)"]], "Bool")));
    expect(pre(["tuple", [["project", ["var", "t"], 0], FALSE]])).toBe("ok");
    expect(pre(["tuple", [["project", ["var", "t"], 0], U8(1)]])).toBe("ill-typed");
    expect(pre(["tuple", [FALSE, ["project", ["var", "t"], 0]]])).toBe("ill-typed");
  });

  it("is level-missing at a lane without ir.tuple and rewrites a tuple parameter at a lane that has it", () => {
    const o = op([["t", "(U8,Bool)"]], "Bool");
    const fault = { fault: "x", op: "f", kind: "pre", rewrites: [{ param: "t", expr: ["tuple", [["project", ["var", "t"], 0], FALSE]] }], requires: ["ir.tuple"] };
    expect(refusal(() => compileFault(program({ levels: ["ir.core", "ir.fixed"] }), fault, o))?.level).toBe("ir.tuple");
    const wrapper = compileFault(program(), fault, o);
    expect(wrapper.kind === "pre" && wrapper.rewriteArgs([[5, true]])).toEqual([[5, false]]);
  });
});
