/**
 * The IR typer and evaluator at ir.core (VER04r sections 3, 4; R69 to R77).
 *
 * The kit pins the outcomes of 248 + 129 cases; these tests pin the *rules*
 * around them: the three tiers of [D64], the overflow class of R71, the pure
 * trace of section 4.4, and every refusal a hostile corpus could provoke.
 */
import { describe, expect, it } from "vitest";
import { KitError } from "../../src/v2/errors.ts";
import { type Param, Program, type ProgramInput, readOp } from "../../src/v2/ir.ts";
import { type Ty, parseType, showType } from "../../src/v2/types.ts";
import { type Value } from "../../src/v2/values.ts";

type Json = unknown;
const T = (text: string): Ty => parseType(text, { allowInt: true });
const U8 = (v: number): Json => ["lit", { type: "U8", value: String(v) }];
const NAT = (v: string | number): Json => ["lit", { type: "Nat", value: String(v) }];
const BYTES = (hex: string): Json => ["lit", { type: "Bytes", value: hex }];
const ERRS = { E: { kind: "error", cases: ["BAD", "WORSE"] }, F: { kind: "error", cases: ["WORSE", "OTHER"] } };
const BIG = "18446744073709551615";

function program(over: Partial<{ types: Json; ops: Json; models: Json; order: Json; levels: string[]; max: number; code: "corpus" | "ill-typed" }> = {}): Program {
  const input: ProgramInput = {
    types: over.types ?? ERRS,
    ops: over.ops ?? {},
    models: over.models ?? {},
    modelOrder: over.order ?? Object.keys((over.models ?? {}) as object),
    levels: new Set(over.levels ?? ["ir.core"]),
    ...(over.max === undefined ? {} : { maxIrCost: over.max }),
    ...(over.code === undefined ? {} : { structuralCode: over.code }),
  };
  return new Program(input);
}

const outcome = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.overflowClass === undefined ? error.code : `${error.code}:${error.overflowClass}`;
    throw error;
  }
  return "ok";
};

/** Compile and run `ir` over `vars` = [name, type text, value] with `p` (default: an empty program). */
function evaluate(ir: Json, vars: [string, string, Value][] = [], p: Program = program(), role: "body" | "admit" | "fault" | "pure" = "pure"): { value: Value; type: string; calls: number } {
  const params: Param[] = vars.map(([name, ty]) => ({ name, ty: T(ty) }));
  const compiled = p.compileExpr(ir, params, role);
  p.state.calls = 0;
  const value = compiled.run(vars.map(([, , v]) => v));
  return { value, type: showType(compiled.ty), calls: p.state.calls };
}

describe("literals, variables and operators", () => {
  it("types each literal form and refuses malformed ones (R73)", () => {
    expect(evaluate(["lit", true]).value).toBe(true);
    expect(evaluate(BYTES("0a0b")).value).toEqual(Uint8Array.of(10, 11));
    expect(evaluate(["lit", { type: "Int", value: "-3" }])).toMatchObject({ value: -3n, type: "Int" });
    expect(evaluate(["lit", { type: "I16", value: "-300" }])).toMatchObject({ value: -300, type: "I16" });
    for (const bad of [
      ["lit"], ["lit", 1], ["lit", "x"], ["lit", []], ["lit", { type: "Bool", value: true }], ["lit", { type: "Bytes<2>", value: "0a0b" }],
      ["lit", { type: "Bytes", value: "0A" }], ["lit", { type: "Bytes", value: 5 }], ["lit", { type: "Bytes", value: "00".repeat(4097) }],
      ["lit", { type: "U8", value: "256" }], ["lit", { type: "U8", value: "-1" }], ["lit", { type: "I8", value: "128" }], ["lit", { type: "Nat", value: "-1" }],
      ["lit", { type: "Nat", value: "18446744073709551616" }], ["lit", { type: "Nat", value: 1 }], ["lit", { type: "Nope", value: "1" }],
      ["lit", { type: "Result<U8,E>", value: "1" }], ["lit", { type: "U8" }], ["lit", { value: "1" }], ["lit", { type: 5, value: "1" }],
      ["lit", { type: "Option<U8>", value: "1" }],
    ]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toMatch(/ill-typed|level-missing/);
    }
  });

  it("refuses every shape error with ill-typed, never a crash", () => {
    for (const bad of [
      7, "x", [], [1], null, {}, ["var"], ["var", 5], ["var", "nope"], ["len"], ["len", 1, 2], ["not"], ["ok"], ["is_err"], ["err"], ["err", 5],
      ["add", 1], ["eq", 1], ["if", 1, 2], ["concat", 1], ["take", 1], ["model_call"], ["model_call", "m"], ["subject_op"], ["subject_op", "x"],
      ["and", 1], ["neg"], ["lt", 1],
    ]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toMatch(/ill-typed|model-order/);
    }
  });

  it("evaluates the operators of the core level", () => {
    const vars: [string, string, Value][] = [["a", "U8", 200], ["b", "U8", 100], ["n", "Nat", 5n], ["s", "I8", -128], ["bs", "Bytes", Uint8Array.of(1, 2, 3)]];
    expect(evaluate(["add", ["var", "a"], ["var", "b"]], vars).value).toBe(44);
    expect(evaluate(["sub", ["var", "b"], ["var", "a"]], vars).value).toBe(156);
    expect(evaluate(["mul", ["var", "a"], ["var", "b"]], vars).value).toBe(32);
    expect(evaluate(["neg", ["var", "s"]], vars).value).toBe(-128);
    expect(evaluate(["neg", ["var", "n"]], vars)).toMatchObject({ value: -5n, type: "Int" });
    expect(evaluate(["len", ["var", "bs"]], vars)).toMatchObject({ value: 3n, type: "Nat" });
    expect(evaluate(["concat", ["var", "bs"], BYTES("ff")], vars).value).toEqual(Uint8Array.of(1, 2, 3, 255));
    expect(evaluate(["take", ["var", "bs"], ["var", "n"]], vars).value).toEqual(Uint8Array.of(1, 2, 3));
    expect(evaluate(["drop", ["var", "bs"], NAT(1)], vars).value).toEqual(Uint8Array.of(2, 3));
    expect(evaluate(["implies", ["lit", true], ["lit", false]]).value).toBe(false);
    expect(evaluate(["ne", ["var", "a"], ["var", "b"]], vars).value).toBe(true);
    expect(evaluate(["ge", ["var", "n"], ["lit", { type: "Int", value: "5" }]], vars).value).toBe(true);
    expect(evaluate(["lt", ["var", "a"], ["var", "b"]], vars).value).toBe(false);
    expect(evaluate(["gt", ["var", "a"], ["var", "b"]], vars).value).toBe(true);
    expect(evaluate(["le", ["var", "a"], ["var", "b"]], vars).value).toBe(false);
    expect(evaluate(["ne", BYTES("00"), BYTES("00")]).value).toBe(false);
    expect(evaluate(["ne", ["ok", ["lit", true]], ["err", "BAD"]]).value).toBe(true);
  });

  it("wraps every fixed-width type per node and refuses mixed or non-integer arithmetic", () => {
    const lit = (type: string, value: string): Json => ["lit", { type, value }];
    for (const [type, max, min] of [["U8", "255", "0"], ["U16", "65535", "0"], ["U32", "4294967295", "0"], ["U64", BIG, "0"], ["I8", "127", "-128"], ["I16", "32767", "-32768"], ["I32", "2147483647", "-2147483648"], ["I64", "9223372036854775807", "-9223372036854775808"]] as const) {
      const wrapped = evaluate(["add", lit(type, max), lit(type, "1")]).value;
      expect(String(wrapped), `${type} max+1`).toBe(min);
      expect(String(evaluate(["sub", lit(type, min), lit(type, "1")]).value), `${type} min-1`).toBe(max);
      expect(String(evaluate(["mul", lit(type, max), lit(type, max)]).value), `${type} max*max`).toBe(type.startsWith("U") ? "1" : "1");
      expect(String(evaluate(["neg", lit(type, min)]).value), `${type} -min`).toBe(type.startsWith("U") ? (min === "0" ? "0" : "") : min);
    }
    for (const bad of [
      ["add", U8(1), NAT(1)], ["add", ["lit", true], ["lit", true]], ["add", U8(1), ["lit", { type: "U16", value: "1" }]], ["neg", ["lit", true]],
      ["lt", ["lit", true], ["lit", false]], ["lt", U8(1), NAT(1)], ["lt", BYTES("00"), BYTES("00")], ["eq", U8(1), NAT(1)], ["eq", ["lit", true], U8(1)],
      ["not", U8(1)], ["and", ["lit", true], U8(1)], ["if", U8(1), U8(1), U8(2)], ["if", ["lit", true], U8(1), ["lit", true]], ["len", U8(1)], ["concat", U8(1), BYTES("00")],
      ["take", BYTES("00"), U8(1)], ["drop", BYTES("00"), ["lit", true]], ["take", U8(1), NAT(1)], ["is_err", U8(1)],
    ]) {
      expect(outcome(() => evaluate(bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("refuses the window of the exact integers and a doubled value, with the class of R71", () => {
    const big = (n: string): Json => ["lit", { type: "Int", value: n }];
    expect(outcome(() => evaluate(["mul", NAT(BIG), NAT(BIG)]))).toBe("ir-overflow:corpus");
    expect(outcome(() => evaluate(["neg", ["mul", NAT(BIG), NAT(BIG)]]))).toBe("ir-overflow:corpus");
    expect(outcome(() => evaluate(["add", ["mul", NAT(BIG), NAT(BIG)], big("0")]))).toBe("ir-overflow:corpus");
    // the largest value is reachable by arithmetic, and the same node is a case failure under a subject call
    const p = program({ ops: { idn: { params: [["a", "Nat"]], ret: "Nat", requires: ["ir.core"] } }, models: { keep: { params: [["a", "Nat"]], body: ["var", "a"] } } });
    p.state.subject = (_op, args) => args[0] as Value;
    const below = ["mul", ["subject_op", "idn", [NAT(BIG)]], NAT(BIG)];
    expect(outcome(() => evaluate(below, [], p, "body"))).toBe("ir-overflow:case");
    expect(outcome(() => evaluate(["neg", ["mul", ["subject_op", "idn", [NAT(BIG)]], NAT(BIG)]], [], p, "body"))).toBe("ir-overflow:case");
    const sub = ["sub", ["neg", ["subject_op", "idn", [NAT(BIG)]]], ["mul", NAT(BIG), NAT(BIG)]];
    expect(outcome(() => evaluate(sub, [], p, "body"))).toBe("ir-overflow:corpus");
  });

  it("refuses a concat past 65,536 bytes before it allocates", () => {
    const chunk = new Uint8Array(40_000);
    const vars: [string, string, Value][] = [["a", "Bytes", chunk]];
    expect(outcome(() => evaluate(["concat", ["var", "a"], ["var", "a"]], vars))).toBe("ir-overflow:corpus");
    expect(outcome(() => evaluate(["concat", ["var", "a"], BYTES("00")], vars))).toBe("ok");
  });
});

describe("results, errors and widths", () => {
  it("unifies bare ok and err with a Result and keeps nominal error cases", () => {
    const r: [string, string, Value][] = [["r", "Result<U8,E>", { ok: 4 }]];
    expect(evaluate(["if", ["lit", true], ["err", "BAD"], ["var", "r"]], r)).toMatchObject({ type: "Result<U8,E>", value: { err: "BAD" } });
    expect(evaluate(["if", ["lit", true], ["ok", U8(1)], ["var", "r"]], r)).toMatchObject({ type: "Result<U8,E>", value: { ok: 1 } });
    expect(evaluate(["eq", ["var", "r"], ["ok", U8(4)]], r).value).toBe(true);
    expect(evaluate(["eq", ["err", "BAD"], ["err", "BAD"]]).value).toBe(true);
    // WORSE is declared by both E and F: a bare err of it is ambiguous until context fixes the type
    expect(evaluate(["eq", ["err", "WORSE"], ["var", "r"]], r).value).toBe(false);
    expect(evaluate(["eq", ["err", "WORSE"], ["err", "WORSE"]]).value).toBe(true);
    expect(outcome(() => evaluate(["eq", ["err", "BAD"], ["err", "OTHER"]]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["err", "NOPE"]))).toBe("ill-typed");
    expect(outcome(() => evaluate(["eq", ["err", "OTHER"], ["var", "r"]], r))).toBe("ill-typed");
  });

  it("types a subject_op with static widths (R70)", () => {
    const ops = { eq_fixed: { params: [["a", "Bytes<N>"], ["b", "Bytes<N>"]], ret: "Bool", width_params: ["N"], requires: ["ir.core"] }, plain: { params: [["a", "U8"]], ret: "U8", requires: ["ir.core"] } };
    const p = program({ ops });
    p.state.subject = () => true;
    const vars: [string, string, Value][] = [["a", "Bytes<2>", Uint8Array.of(1, 2)], ["b", "Bytes<2>", Uint8Array.of(1, 2)], ["c", "Bytes", Uint8Array.of(1, 2)]];
    expect(evaluate(["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [2]], vars, p, "body")).toMatchObject({ value: true, calls: 1 });
    for (const bad of [
      ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [3]], ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]]],
      ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [2, 2]], ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [4097]],
      ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [-1]], ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [1.5]],
      ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], "2"], ["subject_op", "eq_fixed", [["var", "a"], ["var", "c"]], [2]],
      ["subject_op", "plain", [U8(1)], [1]], ["subject_op", "plain", [U8(1)], []], ["subject_op", "plain", [U8(1), U8(2)]], ["subject_op", "plain", [BYTES("00")]],
      ["subject_op", "ghost", [U8(1)]], ["subject_op", "plain", 5], ["subject_op", 5, []], ["subject_op", "plain", [U8(1)], [1], 7],
    ]) {
      expect(outcome(() => evaluate(bad, vars, p, "body")), JSON.stringify(bad)).toBe("ill-typed");
    }
  });

  it("reads op rows", () => {
    expect(readOp("x", { params: [["a", "Bytes<N>"]], ret: "Bytes<N>", width_params: ["N"], requires: ["ir.core"] })).toMatchObject({ widthParams: ["N"], requires: ["ir.core"] });
    expect(readOp("x", { params: [], ret: "Bool" }).requires).toEqual([]);
    for (const bad of [
      null, [], { params: [], ret: 1 }, { ret: "Bool" }, { params: [[1, "U8"]], ret: "Bool" }, { params: [["a"]], ret: "Bool" }, { params: [["a", 7]], ret: "Bool" },
      { params: [], ret: "Bool", width_params: "N" }, { params: [], ret: "Bool", width_params: [1] }, { params: [], ret: "Bool", width_params: ["N", "N"] },
      { params: [["a", "Bytes<N>"]], ret: "Bool" }, { params: [], ret: "Bytes<N>" }, { params: [], ret: "Nope<" }, { params: [["a", "(Bytes<N>,U8)"]], ret: "Bool", width_params: ["M"] },
      { params: [["a", "List<Bytes<N>>"]], ret: "Bool", width_params: ["M"] }, { params: [["a", "Option<Bytes<N>>"]], ret: "Bool", width_params: ["M"] }, { params: [["a", "Result<Bytes<N>,E>"]], ret: "Bool", width_params: ["M"] },
    ]) {
      expect(outcome(() => readOp("x", bad)), JSON.stringify(bad)).toBe("ill-typed");
    }
  });
});

describe("guards and the pure trace (section 4.4)", () => {
  const ops = { f: { params: [["a", "U8"]], ret: "U8", requires: ["ir.core"] } };
  const p = program({ ops, models: { id: { params: [["a", "U8"]], body: ["var", "a"] } } });
  p.state.subject = (_op, args) => args[0] as Value;
  const call = (arg: Json): Json => ["subject_op", "f", [arg]];
  const vars: [string, string, Value][] = [["x", "U8", 5], ["flag", "Bool", true]];
  const trace = (ir: Json): { fixed: number } | { dynamic: (env: Value[]) => number } => p.compileExpr(ir, vars.map(([name, ty]) => ({ name, ty: T(ty) })), "body").trace;
  const count = (ir: Json, env: Value[]): number => {
    const t = trace(ir);
    return "fixed" in t ? t.fixed : t.dynamic(env);
  };

  it("counts subject calls under pure guards without calling the subject", () => {
    const eq = (a: Json): Json => ["eq", a, U8(5)];
    expect(trace(["lit", true])).toEqual({ fixed: 0 });
    expect(trace(eq(call(["var", "x"])))).toEqual({ fixed: 1 });
    expect(trace(eq(call(call(["var", "x"]))))).toEqual({ fixed: 2 });
    expect(trace(eq(["model_call", "id", [call(["var", "x"])]]))).toEqual({ fixed: 1 });
    // an `if` whose branches call equally often has a fixed count; otherwise it decides on the (pure) guard
    expect(trace(["if", ["var", "flag"], eq(call(["var", "x"])), eq(call(["var", "x"]))])).toEqual({ fixed: 1 });
    const branchy: Json = ["if", ["var", "flag"], eq(call(["var", "x"])), ["lit", true]];
    expect(count(branchy, [5, true])).toBe(1);
    expect(count(branchy, [5, false])).toBe(0);
    // and / or / implies: the right operand runs only when the left lets it
    for (const [tag, runsOn] of [["and", true], ["or", false], ["implies", true]] as const) {
      const guarded: Json = [tag, ["var", "flag"], eq(call(["var", "x"]))];
      expect(count(guarded, [5, runsOn])).toBe(1);
      expect(count(guarded, [5, !runsOn])).toBe(0);
    }
    // a subject call on the left guards nothing: it is counted, the pure right is not
    expect(trace(["and", eq(call(["var", "x"])), ["var", "flag"]])).toEqual({ fixed: 1 });
    // a call in the condition and none in the branches
    expect(trace(["if", eq(call(["var", "x"])), ["lit", true], ["lit", false]])).toEqual({ fixed: 1 });
    // a dynamic subtree summed with a fixed one
    expect(count(["and", ["var", "flag"], ["and", ["var", "flag"], ["eq", ["add", call(["var", "x"]), call(["var", "x"])], U8(0)]]], [5, true])).toBe(2);
  });

  it("refuses a subject call under a guard that contains one (impure-guard)", () => {
    const hit = ["eq", call(["var", "x"]), U8(5)];
    for (const bad of [
      ["if", hit, hit, ["lit", true]], ["if", hit, ["lit", true], hit], ["and", hit, hit], ["or", hit, hit], ["implies", hit, hit],
      ["if", ["var", "flag"], ["if", hit, hit, hit], ["lit", true]],
    ]) {
      expect(outcome(() => trace(bad)), JSON.stringify(bad)).toBe("impure-guard");
    }
    // an impure guard that guards nothing that calls is fine
    expect(outcome(() => trace(["and", hit, ["var", "flag"]]))).toBe("ok");
    expect(outcome(() => trace(["if", hit, ["lit", true], ["lit", false]]))).toBe("ok");
  });

  it("classifies an overflow by R71: under a subject-guarded branch it is a case failure", () => {
    const overflow: Json = ["eq", ["mul", NAT(BIG), NAT(BIG)], ["lit", { type: "Int", value: "0" }]];
    const guard: Json = ["eq", call(["var", "x"]), U8(5)];
    expect(outcome(() => evaluate(["if", guard, overflow, ["lit", false]], vars, p, "body"))).toBe("ir-overflow:case");
    expect(outcome(() => evaluate(["and", guard, overflow], vars, p, "body"))).toBe("ir-overflow:case");
    expect(outcome(() => evaluate(["or", ["not", guard], overflow], vars, p, "body"))).toBe("ir-overflow:case");
    expect(outcome(() => evaluate(["implies", guard, overflow], vars, p, "body"))).toBe("ir-overflow:case");
    expect(outcome(() => evaluate(["if", ["var", "flag"], overflow, ["lit", false]], vars, p, "body"))).toBe("ir-overflow:corpus");
    expect(outcome(() => evaluate(["and", ["var", "flag"], overflow], vars, p, "body"))).toBe("ir-overflow:corpus");
    // pure siblings of the call are the generator's: corpus
    expect(outcome(() => evaluate(["eq", call(["var", "x"]), ["add", U8(1), ["lit", { type: "U8", value: "4" }]]], vars, p, "body"))).toBe("ok");
    // a model entered with a subject argument, at any depth, is a case failure; with a pure argument it is the generator's
    const q = program({
      ops, models: {
        boom: { params: [["a", "U8"]], body: overflow },
        outer: { params: [["a", "U8"]], body: ["model_call", "boom", [["var", "a"]]] },
      },
    });
    q.state.subject = (_op, args) => args[0] as Value;
    expect(outcome(() => evaluate(["model_call", "outer", [call(["var", "x"])]], vars, q, "body"))).toBe("ir-overflow:case");
    expect(outcome(() => evaluate(["model_call", "outer", [["var", "x"]]], vars, q, "body"))).toBe("ir-overflow:corpus");
    expect(outcome(() => evaluate(["if", guard, ["model_call", "outer", [["var", "x"]]], ["lit", false]], vars, q, "body"))).toBe("ir-overflow:case");
  });

  it("runs a stand-in model as the subject: its overflow is the subject's answer, a case failure", () => {
    const q = program({ models: { boom: { params: [["a", "U8"]], body: ["eq", ["mul", NAT(BIG), NAT(BIG)], ["lit", { type: "Int", value: "0" }]] }, ok: { params: [["a", "U8"]], body: ["var", "a"] } } });
    expect(q.callModel("ok", [7])).toBe(7);
    expect(outcome(() => q.callModel("boom", [7]))).toBe("ir-overflow:case");
    expect(() => q.callModel("nope", [7])).toThrow(/not live/);
    const r = program({ models: { fx: { params: [["a", "U8"]], body: ["bit_not", ["var", "a"]] } } });
    expect(() => r.callModel("fx", [1])).toThrow(/not live/);
    expect(() => new Program({ types: {}, ops: {}, models: {}, modelOrder: [], levels: new Set(["ir.core"]) }).state.subject("x", [])).toThrow(/no subject/);
  });
});

describe("levels by reach: the three tiers of [D64] and R69", () => {
  const fx = { params: [["a", "U8"]], body: ["bit_not", ["var", "a"]] };
  const call = (name: string): Json => ["model_call", name, [U8(1)]];

  it("(1) compiles and validates an implemented model whether or not anything reaches it", () => {
    const bad = (body: Json): Json => ({ m: { params: [["a", "U8"]], body } });
    expect(outcome(() => program({ models: bad(["add", ["var", "a"], ["lit", true]]) }))).toBe("ill-typed");
    expect(outcome(() => program({ models: bad(["subject_op", "x", []]) }))).toBe("subject-forbidden");
    expect(outcome(() => program({ models: bad(["var", "nope"]) }))).toBe("ill-typed");
    expect(outcome(() => program({ models: bad(["lit", true]), max: 1999 }))).toBe("corpus");
    expect(outcome(() => program({ models: bad(["lit", true]), max: 1999, code: "ill-typed" }))).toBe("ir-cost");
    expect(outcome(() => program({ models: bad(["lit", true]), max: 65_537 }))).toBe("corpus");
    expect(outcome(() => program({ models: bad(["lit", true]), max: 1.5 }))).toBe("corpus");
    expect(outcome(() => program({ models: bad(["lit", true]), max: 65_536 }))).toBe("ok");
    expect(outcome(() => program({ models: { m: { params: [["a", "Nope"]], body: ["lit", true] } } }))).toBe("ill-typed");
    expect(outcome(() => program({ ops: { f: { params: [["a", "Nope"]], ret: "U8", requires: ["ir.core"] } } }))).toBe("ill-typed");
  });

  it("(2) does not compile a model that needs a missing level, but reads its tags", () => {
    expect(outcome(() => program({ models: { fx } }))).toBe("ok");
    // typing and cost are not checked there ...
    expect(outcome(() => program({ models: { fx: { params: [["a", "U8"]], body: ["bit_not", ["add", ["lit", true], ["lit", true]]] } } }))).toBe("ok");
    // ... but the checks that read tag names alone apply
    expect(outcome(() => program({ models: { fx: { params: [["a", "U8"]], body: ["bit_not", ["frobnicate", 1]] } } }))).toBe("unknown-tag");
    expect(outcome(() => program({ models: { fx: { params: [["a", "U8"]], body: ["bit_not", ["subject_op", "f", []]] } } }))).toBe("subject-forbidden");
    expect(outcome(() => program({ models: { fx: { params: [["a", "U8"]], body: ["bit_not", ["model_call", "ghost", []]] } } }))).toBe("model-order");
    let deep: Json = ["var", "a"];
    for (let i = 0; i < 70; i += 1) deep = ["bit_not", deep];
    expect(outcome(() => program({ models: { fx: { params: [["a", "U8"]], body: deep } } }))).toBe("depth");
    // a model that calls a model of a missing level needs one too (transitively), and is not compiled either
    expect(outcome(() => program({ models: { fx, wrap: { params: [["a", "U8"]], body: ["add", call("fx"), ["lit", true]] } } }))).toBe("ok");
    // a parameter of a type of a missing level makes the model not live
    expect(outcome(() => program({ models: { pair: { params: [["a", "(U8,U8)"]], body: ["lit", true] } } }))).toBe("ok");
    // an op of a missing level is not compiled; an undeclared name is still a typing error, of any level
    expect(outcome(() => program({ ops: { f: { params: [["a", "(U8,U8)"]], ret: "U8", requires: ["ir.core"] } } }))).toBe("ok");
    expect(outcome(() => program({ ops: { f: { params: [["a", "(U8,Nope)"]], ret: "U8", requires: ["ir.core"] } } }))).toBe("ill-typed");
  });

  it("(3) scopes only level-missing by reach: a call, an op, a type and a literal each reach", () => {
    const p = program({
      models: { fx, wrap: { params: [["a", "U8"]], body: call("fx") }, pair: { params: [["a", "(U8,U8)"]], body: ["lit", true] } },
      ops: { g: { params: [["a", "(U8,U8)"]], ret: "U8", requires: ["ir.fixed"] }, h: { params: [["a", "U8"]], ret: "Option<U8>", requires: ["ir.data"] } },
    });
    const run = (ir: Json, vars: [string, string, Value][] = []): string => outcome(() => evaluate(ir, vars, p, "body"));
    expect(run(["lit", true])).toBe("ok");
    expect(run(call("fx"))).toBe("level-missing");
    expect(run(call("wrap"))).toBe("level-missing");
    expect(run(["model_call", "pair", [["lit", true]]])).toBe("level-missing");
    expect(run(["subject_op", "g", [["lit", true]]])).toBe("level-missing");
    expect(run(["subject_op", "h", [U8(1)]])).toBe("level-missing");
    expect(run(["lit", true], [["t", "(U8,U8)", [1, 2] as never]])).toBe("level-missing");
    expect(run(["lit", { type: "(U8,U8)", value: "x" }])).toBe("level-missing");
    expect(run(["bit_and", U8(1), U8(2)])).toBe("level-missing");
    expect(run(["match", ["lit", true], []])).toBe("level-missing");
    expect(run(["list", []])).toBe("level-missing");
    expect(run(["fold", []])).toBe("level-missing");
    expect(run(["some", U8(1)])).toBe("level-missing");
    expect(run(["tuple", [U8(1), U8(2)]])).toBe("level-missing");
    expect(run(["lit", true], [["l", "List<U8>", [] as never]])).toBe("level-missing");
    expect(run(["frobnicate"])).toBe("unknown-tag");
    expect(run(["lit", true], [["r", "Result<U8,Nope>", { ok: 1 }]])).toBe("ill-typed");
    // a level-missing tree with a subject call under a role that forbids one: the earlier code wins
    expect(outcome(() => evaluate(["bit_and", ["subject_op", "f", []], U8(1)], [], p, "fault"))).toBe("level-missing");
    expect(outcome(() => evaluate(["subject_op", "f", []], [], p, "fault"))).toBe("subject-forbidden");
    expect(outcome(() => evaluate(["bit_and", ["frobnicate"], U8(1)], [], p, "fault"))).toBe("unknown-tag");
  });

  it("names a refusal of List<T> as the level ir.list, never ir.data [D63]", () => {
    const p = program({ ops: { h: { params: [["a", "List<Option<U8>>"]], ret: "U8", requires: ["ir.list"] } } });
    try {
      evaluate(["subject_op", "h", [["lit", true]]], [], p, "body");
      expect.unreachable();
    } catch (error) {
      expect((error as KitError).level).toBe("ir.list");
    }
  });

  it("refuses a tree deeper than 64 nodes and a model chain of 65, called or not (R77)", () => {
    const nest = (n: number): Json => {
      let ir: Json = ["lit", true];
      for (let i = 1; i < n; i += 1) ir = ["not", ir];
      return ir;
    };
    expect(outcome(() => evaluate(nest(64)))).toBe("ok");
    expect(outcome(() => evaluate(nest(65)))).toBe("depth");
    const chain = (n: number): Json => Object.fromEntries(Array.from({ length: n }, (_u, i) => [`m${i + 1}`, { params: [["a", "Bool"]], body: i === 0 ? ["var", "a"] : ["model_call", `m${i}`, [["var", "a"]]] }]));
    const order = (n: number): string[] => Array.from({ length: n }, (_u, i) => `m${i + 1}`);
    expect(outcome(() => program({ models: chain(64), order: order(64) }))).toBe("ok");
    expect(outcome(() => program({ models: chain(65), order: order(65) }))).toBe("depth");
  });

  it("validates model_order and the calls it licenses", () => {
    const m = { a: { params: [], body: ["lit", true] }, b: { params: [], body: ["model_call", "a", []] } };
    expect(outcome(() => program({ models: m, order: ["a", "b"] }))).toBe("ok");
    expect(outcome(() => program({ models: m, order: ["b", "a"] }))).toBe("model-order");
    expect(outcome(() => program({ models: m, order: ["a"] }))).toBe("model-order");
    expect(outcome(() => program({ models: m, order: ["a", "a", "b"] }))).toBe("model-order");
    expect(outcome(() => program({ models: m, order: ["a", "ghost", "b"] }))).toBe("model-order");
    expect(outcome(() => program({ models: m, order: [1] }))).toBe("model-order");
    expect(outcome(() => program({ models: m, order: "ab" }))).toBe("corpus");
    expect(outcome(() => program({ models: [], order: [] }))).toBe("corpus");
    expect(outcome(() => program({ models: { a: 5 }, order: ["a"] }))).toBe("corpus");
    expect(outcome(() => program({ models: { a: { params: 5, body: ["lit", true] } }, order: ["a"] }))).toBe("corpus");
    expect(outcome(() => program({ models: { a: { params: [["x"]], body: ["lit", true] } }, order: ["a"] }))).toBe("ill-typed");
    expect(outcome(() => program({ ops: [] }))).toBe("corpus");
    expect(outcome(() => program({ types: [] }))).toBe("corpus");
    expect(outcome(() => program({ models: { a: { params: [], body: ["lit", 5] } }, order: ["a"] }))).toBe("ill-typed");
    expect(outcome(() => program({ models: { a: { params: [], body: 5 } }, order: ["a"] }))).toBe("ill-typed");
    expect(outcome(() => program({ models: { a: { params: [], body: ["model_call", 5, []] } }, order: ["a"] }))).toBe("ill-typed");
    expect(outcome(() => program({ models: { a: { params: [], body: ["model_call", "a", 5] } }, order: ["a"] }))).toBe("model-order");
    // a model that is not listed can call nothing
    expect(outcome(() => program({ models: { u: { params: [], body: ["model_call", "a", []] }, a: m.a }, order: ["a"] }))).toBe("model-order");
    expect(program({ models: m, order: ["a", "b"] }).hasModel("b")).toBe(true);
    expect(program({ models: m, order: ["a", "b"] }).hasModel("z")).toBe(false);
  });

  it("types a model_call against the model's parameters and flows Nat to Int, Bytes<N> to Bytes", () => {
    const p = program({ models: { wide: { params: [["a", "Nat"]], body: ["var", "a"] }, ok2: { params: [["a", "Bytes"]], body: ["len", ["var", "a"]] } } });
    expect(evaluate(["model_call", "wide", [NAT(3)]], [], p).value).toBe(3n);
    // [D75]: a model row is a signature, so an `Int` parameter is accepted (it was `ill-typed` before section 16.22).
    expect(outcome(() => program({ models: { wide: { params: [["a", "Int"]], body: ["var", "a"] } } }))).toBe("ok");
    expect(evaluate(["model_call", "ok2", [BYTES("0102")]], [], p).value).toBe(2n);
    expect(outcome(() => evaluate(["model_call", "wide", [U8(3)]], [], p))).toBe("ill-typed");
    expect(outcome(() => evaluate(["model_call", "wide", []], [], p))).toBe("ill-typed");
    expect(outcome(() => evaluate(["model_call", "ghost", []], [], p))).toBe("model-order");
  });
});

describe("cost (section 4.2 rule 6)", () => {
  it("charges the worst path and never credits a short circuit", () => {
    const p = program();
    const cost = (ir: Json): number => p.compileExpr(ir, [], "pure").cost;
    expect(cost(["lit", true])).toBe(1);
    expect(cost(["not", ["lit", true]])).toBe(2);
    expect(cost(["if", ["lit", true], ["not", ["lit", true]], ["lit", false]])).toBe(1 + 1 + 2);
    expect(cost(["and", ["lit", true], ["not", ["lit", true]]])).toBe(1 + 1 + 2);
    expect(cost(U8(1))).toBe(1);
    const q = program({ models: { m: { params: [["a", "Bool"]], body: ["not", ["var", "a"]] } } });
    expect(q.compileExpr(["model_call", "m", [["lit", true]]], [], "pure").cost).toBe(1 + 1 + 2);
    expect(outcome(() => program({ models: { m: { params: [], body: ["lit", true] } }, max: 2000 }).compileExpr(nestSum(2001), [], "pure"))).toBe("ir-cost");
    expect(outcome(() => program({ max: 4096 }).compileExpr(nestSum(2001), [], "pure"))).toBe("ok");
  });
});

/** A balanced sum of `n` literals: width 1, depth about log2(n), cost exactly n + (n - 1) additions. */
function nestSum(count: number): Json {
  let level: Json[] = Array.from({ length: Math.ceil(count / 2) }, () => ["lit", true] as Json);
  // build a Bool tree with `count` cost using and-chains of bounded depth
  let ir: Json = ["lit", true];
  const nodes: Json[] = [];
  for (let i = 0; i < count; i += 1) nodes.push(["lit", true]);
  level = nodes;
  while (level.length > 1) {
    const next: Json[] = [];
    for (let i = 0; i < level.length; i += 2) next.push(i + 1 < level.length ? ["and", level[i], level[i + 1]] : level[i]);
    level = next;
  }
  ir = level[0];
  return ir;
}

describe("a shown type for the kit", () => {
  it("keeps a bytes width: a bare literal is Bytes<len> (R73) and the width survives an ok", () => {
    expect(evaluate(BYTES("0a")).type).toBe("Bytes<1>");
    expect(evaluate(BYTES("0a0b0c")).type).toBe("Bytes<3>");
    expect(evaluate(["ok", BYTES("0a")]).type).toBe("Result<Bytes<1>,?>");
    expect(evaluate(["take", BYTES("0a0b"), NAT(1)]).type).toBe("Bytes");
  });
});
