/**
 * VER04r section 16.21 R108 and section 16.22 ([D74], [D75], R110, R111, R112, R115, R117, R118): the `types` table is
 * read whole and first, `v2-fault` has the code `ill-typed`, and a model parameter may be `Int`.
 */
import { cpSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { loadV2Corpus } from "../../src/v2/corpus.ts";
import { KitError } from "../../src/v2/errors.ts";
import { Program } from "../../src/v2/ir.ts";
import { runCase } from "../../src/v2/kit.ts";
import { type Ty, checkNames, parseType, readTypeTable } from "../../src/v2/types.ts";
import { decodeJson } from "../../src/v2/values.ts";
import { CORPORA, kitCase, tempDir } from "./support.ts";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any
const clone = <X>(value: X): X => JSON.parse(JSON.stringify(value)) as X;
const code = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.code;
    throw error;
  }
  return "none";
};
const CONTEXTS = ["type", "ill-typed", "descriptor", "corpus"] as const;
const E = { kind: "error", cases: ["BAD", "WORSE"] };

/** The table defects of R108, R117 and R118, one table each (R110 (d)'s list). */
const DEFECTS: Record<string, Json> = {
  "a record with a field of its own type": { R: { kind: "record", fields: [["a", "R"]] } },
  "a record holding itself under List (R118)": { R: { kind: "record", fields: [["a", "List<R>"]] } },
  "a record holding itself under Option": { R: { kind: "record", fields: [["a", "Option<R>"]] } },
  "a record holding itself in a tuple": { R: { kind: "record", fields: [["a", "(U8,R)"]] } },
  "a record holding itself in a Result payload": { R: { kind: "record", fields: [["a", "Result<R,E>"]] }, E },
  "a variant holding itself": { V: { kind: "variant", ctors: [["A", []], ["B", ["Option<V>"]]] } },
  "two records holding each other under Option": {
    P: { kind: "record", fields: [["q", "Option<Q>"]] },
    Q: { kind: "record", fields: [["p", "Option<P>"]] },
  },
  "a record leading into a ring it is not part of": {
    A: { kind: "record", fields: [["b", "B"]] },
    B: { kind: "record", fields: [["c", "C"]] },
    C: { kind: "record", fields: [["b", "B"]] },
  },
  "a repeated field name": { R: { kind: "record", fields: [["a", "U8"], ["a", "Bool"]] } },
  "a repeated constructor name": { V: { kind: "variant", ctors: [["A", []], ["A", ["U8"]]] } },
  "a field of an undeclared type": { R: { kind: "record", fields: [["a", "Ghost"]] } },
  "a field of an undeclared type under Option": { R: { kind: "record", fields: [["a", "Option<Ghost>"]] } },
  "a constructor component of an undeclared type": { V: { kind: "variant", ctors: [["A", []], ["B", ["U8", "Ghost"]]] } },
  "a Result whose error type is undeclared": { R: { kind: "record", fields: [["a", "Result<U8,Ghost>"]] } },
  "a field of an error type (R117)": { E, R: { kind: "record", fields: [["a", "E"]] } },
  "a field of an error type under Option (R117)": { E, R: { kind: "record", fields: [["a", "Option<E>"]] } },
  "a field of an error type under List (R117)": { E, R: { kind: "record", fields: [["a", "List<E>"]] } },
  "a field of an error type in a tuple (R117)": { E, R: { kind: "record", fields: [["a", "(U8,E)"]] } },
  "a constructor component of an error type (R117)": { E, V: { kind: "variant", ctors: [["A", []], ["B", ["E"]]] } },
  "a Result whose error slot names a record": { R: { kind: "record", fields: [["a", "Result<U8,R>"]] } },
  "a field of type Int (R105)": { R: { kind: "record", fields: [["a", "Int"]] } },
  "a field of type Option<Int>": { R: { kind: "record", fields: [["a", "Option<Int>"]] } },
  "a constructor component of type Int": { V: { kind: "variant", ctors: [["A", []], ["B", ["Int"]]] } },
  "a field whose type text is malformed": { R: { kind: "record", fields: [["a", "Option<U8"]] } },
  "a field type deeper than 16 constructors": { R: { kind: "record", fields: [["a", `${"List<".repeat(17)}U8${">".repeat(17)}`]] } },
};

const ACCEPTED: Record<string, Json> = {
  "a field of a declared record, a variant and an error under Result": {
    E,
    P: { kind: "record", fields: [["x", "U8"], ["r", "Result<Option<(U8,List<Q>)>,E>"]] },
    Q: { kind: "variant", ctors: [["Leaf", []], ["Node", ["U8", "Bytes<4>"]]] },
  },
  "a diamond (a name reached twice is not recursion)": {
    A: { kind: "record", fields: [["b", "B"], ["c", "C"]] },
    B: { kind: "record", fields: [["d", "D"]] },
    C: { kind: "record", fields: [["d", "D"]] },
    D: { kind: "record", fields: [["x", "U8"]] },
  },
  "a field naming a type declared later in the table": {
    A: { kind: "record", fields: [["z", "Z"]] },
    Z: { kind: "record", fields: [["x", "U8"]] },
  },
  "a field that is 16 constructors deep": { R: { kind: "record", fields: [["a", `${"List<".repeat(16)}U8${">".repeat(16)}`]] } },
};

describe("R108, R117, R118: a bad table is refused whole with the code of its context", () => {
  for (const [name, table] of Object.entries(DEFECTS)) {
    it(`${name}`, () => {
      for (const context of CONTEXTS) expect(code(() => readTypeTable(clone(table), context)), context).toBe(context);
    });
  }

  for (const [name, table] of Object.entries(ACCEPTED)) {
    it(`accepts ${name}`, () => {
      expect(() => readTypeTable(clone(table), "type")).not.toThrow();
    });
  }

  it("walks a long chain without recursing: 200,000 records in a line are accepted, and closing the ring is refused", () => {
    const length = 200_000;
    const chain: Record<string, Json> = {};
    for (let i = 0; i < length; i += 1) chain[`R${i}`] = { kind: "record", fields: [["next", i + 1 < length ? `R${i + 1}` : "U8"]] };
    expect(readTypeTable(chain, "corpus").data.size).toBe(length);
    const ring = clone(chain);
    ring[`R${length - 1}`] = { kind: "record", fields: [["next", "Option<R0>"]] };
    expect(code(() => readTypeTable(ring, "corpus"))).toBe("corpus");
    // The chain written in the opposite order of declaration must not matter either.
    const backwards: Record<string, Json> = {};
    for (let i = length - 1; i >= 0; i -= 1) backwards[`R${i}`] = chain[`R${i}`];
    expect(readTypeTable(backwards, "corpus").data.size).toBe(length);
  });

  it("refuses an undeclared name at the end of a long chain", () => {
    const chain: Record<string, Json> = {};
    for (let i = 0; i < 5_000; i += 1) chain[`R${i}`] = { kind: "record", fields: [["next", i + 1 < 5_000 ? `R${i + 1}` : "Ghost"]] };
    expect(code(() => readTypeTable(chain, "ill-typed"))).toBe("ill-typed");
  });

  it("keeps the type readers' refusal of an error type as a value type (R117 (c))", () => {
    const table = readTypeTable({ E });
    for (const text of ["E", "Option<E>", "List<E>", "(U8,E)", "Result<E,E>"]) {
      expect(code(() => checkNames(parseType(text), table, "ill-typed")), text).toBe("ill-typed");
    }
    expect(() => checkNames(parseType("Result<U8,E>"), table, "ill-typed")).not.toThrow();
    expect(code(() => decodeJson("0", parseType("E") as Ty, table))).not.toBe("none");
  });
});

describe("the kit contexts of R108", () => {
  const bad = DEFECTS["a record with a field of its own type"];

  it("v2-value: `type`, before any value is read", () => {
    for (const table of Object.values(DEFECTS)) {
      const c = { ...clone(kitCase("v2-value/bool-true")), types: clone(table) };
      expect(runCase("v2-value", c)).toEqual({ outcome: "reject", error: "type" });
    }
  });

  it("v2-domain: `descriptor`", () => {
    for (const table of Object.values(DEFECTS)) {
      const c = { ...clone(kitCase("v2-domain/bool-index-0")), types: clone(table) };
      expect(runCase("v2-domain", c)).toEqual({ error: "descriptor" });
    }
  });

  it("v2-ir: `ill-typed`, ahead of an unknown tag, a model-order error and a level that is missing", () => {
    for (const table of Object.values(DEFECTS)) {
      const c = { ...clone(kitCase("v2-ir/lit-bool-true")), types: clone(table), ir: ["bogus"] };
      expect(runCase("v2-ir", c), JSON.stringify(table)).toEqual({ error: "ill-typed" });
    }
    const order = { ...clone(kitCase("v2-ir/lit-bool-true")), types: clone(bad), ir: ["lit", true], model_order: ["ghost"] };
    expect(runCase("v2-ir", order)).toEqual({ error: "ill-typed" });
  });

  it("v2-ir (R115 (b)): the bound on max_ir_cost comes first, then the table, then everything else", () => {
    const base = { ...clone(kitCase("v2-ir/lit-bool-true")), types: clone(bad), ir: ["bogus"] };
    expect(runCase("v2-ir", { ...base, max_ir_cost: 1999 })).toEqual({ error: "ir-cost" });
    expect(runCase("v2-ir", { ...base, max_ir_cost: 65537 })).toEqual({ error: "ir-cost" });
    expect(runCase("v2-ir", { ...base, max_ir_cost: 2000 })).toEqual({ error: "ill-typed" });
    expect(runCase("v2-ir", base)).toEqual({ error: "ill-typed" });
    // With a good table the tree is read: `unknown-tag` is back.
    expect(runCase("v2-ir", { ...base, types: {} })).toEqual({ error: "unknown-tag" });
  });

  it("v2-chunk: a bad table of the corpus is `corpus`", () => {
    const c = clone(kitCase("v2-chunk/big-first-chunk"));
    expect(runCase("v2-chunk", c).error).toBeUndefined();
    for (const [name, table] of Object.entries(DEFECTS)) {
      expect(runCase("v2-chunk", { ...clone(c), corpus: { ...clone(c.corpus), types: clone(table) } }), name).toEqual({ error: "corpus" });
    }
  });
});

describe("[D74], R110: v2-fault has `ill-typed`, in the order table, level-missing, subject-forbidden, row", () => {
  const base = (): Json => clone(kitCase("v2-fault/invert-true-result"));
  const withCall = (): Json => {
    const c = base();
    c.fault.expr = ["subject_op", "cmp2", [["var", "a"], ["var", "b"]]];
    return c;
  };

  it("a bad table of any defect is `ill-typed`, on a row that is otherwise accepted (and not `corpus`)", () => {
    expect(runCase("v2-fault", base())).toEqual({ result: false });
    for (const [name, table] of Object.entries(DEFECTS)) {
      expect(runCase("v2-fault", { ...base(), types: clone(table) }), name).toEqual({ error: "ill-typed" });
    }
  });

  it("(1) the table comes before level-missing, before subject-forbidden and before the row's typing", () => {
    const table = clone(DEFECTS["a repeated field name"]);
    expect(runCase("v2-fault", { ...base(), levels: [], types: table })).toEqual({ error: "ill-typed" });
    expect(runCase("v2-fault", { ...withCall(), types: table })).toEqual({ error: "ill-typed" });
    const untyped = base();
    untyped.fault.expr = ["lit", { type: "U8", value: "1" }];
    expect(runCase("v2-fault", { ...untyped, types: table })).toEqual({ error: "ill-typed" });
  });

  it("(2) level-missing comes before subject-forbidden", () => {
    expect(runCase("v2-fault", { ...withCall(), levels: [] })).toEqual({ error: "level-missing" });
    expect(runCase("v2-fault", withCall())).toEqual({ error: "subject-forbidden" });
  });

  it("(3) subject-forbidden comes before the typing of the row", () => {
    const c = base();
    c.fault.expr = ["add", ["subject_op", "cmp2", [["var", "a"], ["var", "b"]]], ["lit", true]];
    expect(runCase("v2-fault", c)).toEqual({ error: "subject-forbidden" });
  });

  it("(4) a fault expression that is ill-typed as a tree, or has no common type with the result, is `ill-typed`", () => {
    const tree = base();
    tree.fault.expr = ["add", ["lit", true], ["lit", true]];
    expect(runCase("v2-fault", tree)).toEqual({ error: "ill-typed" });
    const unbound = base();
    unbound.fault.expr = ["var", "nowhere"];
    expect(runCase("v2-fault", unbound)).toEqual({ error: "ill-typed" });
    const common = base();
    common.fault.expr = ["lit", { type: "U8", value: "1" }];
    expect(runCase("v2-fault", common)).toEqual({ error: "ill-typed" });
  });

  it("a table that is fine and unreached never causes level-missing or any other refusal (R69)", () => {
    const c = { ...base(), types: clone(ACCEPTED["a field of a declared record, a variant and an error under Result"]) };
    expect(runCase("v2-fault", c)).toEqual({ result: false });
  });
});

describe("[D75], R111, R112: a model parameter may be Int", () => {
  const input = (models: Json, over: Partial<ConstructorParameters<typeof Program>[0]> = {}) => ({
    types: { E },
    ops: {},
    models,
    modelOrder: Object.keys(models),
    levels: new Set(["ir.core", "ir.tuple", "ir.data", "ir.list"]),
    ...over,
  });
  const lit = (type: string, value: string): Json => ["lit", { type, value }];
  const run = (program: Program, ir: Json): { value: unknown; ty: string } => {
    const compiled = program.compileExpr(ir, [], "pure");
    program.state.calls = 0;
    return { value: compiled.run([]), ty: compiled.ty.k };
  };

  it("the R111 (d) vector: m(a: Int) compares its parameter with an Int literal, called with an Int literal: true, a Bool", () => {
    const program = new Program(input({ m: { params: [["a", "Int"]], body: ["eq", ["var", "a"], lit("Int", "-3")] } }));
    expect(run(program, ["model_call", "m", [lit("Int", "-3")]])).toEqual({ value: true, ty: "Bool" });
    expect(run(program, ["model_call", "m", [lit("Int", "3")]])).toEqual({ value: false, ty: "Bool" });
    // A Nat flows to an Int parameter, as it does everywhere else.
    expect(run(program, ["model_call", "m", [lit("Nat", "3")]]).value).toBe(false);
  });

  it("accepts Int under Option, Result, a tuple and List (the R111 (c) reading)", () => {
    const models = {
      o: { params: [["a", "Option<Int>"]], body: ["lit", true] },
      r: { params: [["a", "Result<Int,E>"]], body: ["lit", true] },
      t: { params: [["a", "(Int,Bool)"]], body: ["lit", true] },
      l: { params: [["a", "List<Int>"]], body: ["lit", true] },
      n: { params: [["a", "List<Option<(U8,Result<Int,E>)>>"], ["b", "Int"]], body: ["lit", true] },
    };
    expect(() => new Program(input(models))).not.toThrow();
  });

  it("an Int parameter needs no level of its own, and a level it lacks is still reported", () => {
    const program = new Program(input({ m: { params: [["a", "Int"]], body: ["var", "a"] } }, { levels: new Set(["ir.core"]) }));
    expect(run(program, ["model_call", "m", [lit("Int", "7")]]).value).toBe(7n);
    // An Int under a tuple is a tuple parameter: its level is `ir.tuple`, which this runner lacks, so the model is not live.
    const tuple = new Program(input({ m: { params: [["a", "(Int,Bool)"]], body: ["lit", true] } }, { levels: new Set(["ir.core"]) }));
    expect(code(() => run(tuple, ["model_call", "m", [["lit", true]]]))).toBe("level-missing");
  });

  it("R112: a model whose result is Int is accepted and its results are unchanged", () => {
    const program = new Program(input({ d: { params: [["a", "Int"], ["b", "Nat"]], body: ["sub", ["var", "a"], ["var", "b"]] } }));
    expect(run(program, ["model_call", "d", [lit("Int", "-3"), lit("Nat", "4")]])).toEqual({ value: -7n, ty: "Int" });
    expect(run(program, ["model_call", "d", [lit("Nat", "10"), lit("Nat", "4")]]).value).toBe(6n);
  });

  it("the other places still refuse Int", () => {
    const refused = (over: Partial<ConstructorParameters<typeof Program>[0]>): string => code(() => new Program(input({}, over)));
    expect(refused({ ops: { f: { params: [["a", "Int"]], ret: "U8", requires: ["ir.core"] } } })).toBe("ill-typed");
    expect(refused({ ops: { f: { params: [["a", "U8"]], ret: "Option<Int>", requires: ["ir.core"] } } })).toBe("ill-typed");
    expect(refused({ types: { R: { kind: "record", fields: [["a", "Int"]] } } })).toBe("corpus");
    expect(refused({ types: { R: { kind: "record", fields: [["a", "Int"]] } }, structuralCode: "ill-typed" })).toBe("ill-typed");
    // A binder (an env entry) of type Int is refused by the kit's IR runner, a v2-value type of Int by the value runner.
    const env = { ...clone(kitCase("v2-ir/lit-bool-true")), env: [["x", { type: "Int", value: "1" }]] };
    expect(runCase("v2-ir", env)).toEqual({ error: "ill-typed" });
    const value = { ...clone(kitCase("v2-value/bool-true")), type: "Int" };
    expect(runCase("v2-value", value)).toEqual({ outcome: "reject", error: "type" });
    // And a parameter of an op is refused in a v2-fault case's op row.
    const fault = { ...clone(kitCase("v2-fault/invert-true-result")), op: { params: [["a", "Int"]], requires: ["ir.core"], ret: "Bool" }, args: ["1"] };
    expect(runCase("v2-fault", fault)).toEqual({ error: "ill-typed" });
  });

  it("a v2-ir case with an Int model parameter runs through the kit", () => {
    const c = {
      ...clone(kitCase("v2-ir/lit-bool-true")),
      ir: ["model_call", "m", [lit("Int", "-3")]],
      models: { m: { params: [["a", "Int"]], body: ["eq", ["var", "a"], lit("Int", "-3")] } },
      model_order: ["m"],
    };
    expect(runCase("v2-ir", c)).toEqual({ calls: 0, type: "Bool", value: true });
  });

  it("a v2-chunk corpus reads an Int model parameter (a corpus reader's `models`)", () => {
    const c = clone(kitCase("v2-chunk/big-first-chunk"));
    c.corpus.models.wide = { params: [["a", "Int"]], body: ["lit", true] };
    c.corpus.model_order = [...c.corpus.model_order, "wide"];
    expect(runCase("v2-chunk", c).error).toBeUndefined();
  });
});

describe("the op row of a v2-fault case is name-checked like v2-ir's (R117 (b), (c); [D64])", () => {
  const withOp = (params: string[], ret: string, types: Json = { E }): Json => {
    const c = clone(kitCase("v2-fault/invert-true-result"));
    c.types = types;
    c.op = { params: params.map((ty, i) => [`p${i}`, ty]), requires: ["ir.core"], ret };
    c.fault.op = "cmp2";
    c.args = params.map(() => "0");
    c.fault.expr = ["lit", true];
    return c;
  };

  it("an error type, or an undeclared name, in a parameter or the result is `ill-typed`, not `level-missing`", () => {
    for (const bad of ["E", "Option<E>", "Ghost", "List<Ghost>", "(U8,Ghost)"]) {
      expect(runCase("v2-fault", withOp([bad], "Bool")), `param ${bad}`).toEqual({ error: "ill-typed" });
    }
    for (const bad of ["E", "Result<E,E>", "Ghost", "Option<E>"]) {
      expect(runCase("v2-fault", withOp(["U8"], bad)), `result ${bad}`).toEqual({ error: "ill-typed" });
    }
  });

  it("a declared name that needs a missing level is still `level-missing`, and a declared Result is accepted", () => {
    const record = { R: { kind: "record", fields: [["x", "U8"]] }, E };
    expect(runCase("v2-fault", withOp(["R"], "Bool", record))).toEqual({ error: "level-missing" });
    const result = withOp(["Result<U8,E>"], "Bool", record);
    result.args = [{ ok: "0" }];
    expect(runCase("v2-fault", result)).toEqual({ result: true });
  });

  it("the table is still read first", () => {
    const c = withOp(["Ghost"], "Bool", clone(DEFECTS["a repeated field name"]));
    expect(runCase("v2-fault", c)).toEqual({ error: "ill-typed" });
  });
});

describe("R117 through Program: a bare error type is no model or op parameter", () => {
  const make = (over: Json): Program =>
    new Program({ types: { E }, ops: {}, models: {}, modelOrder: [], levels: new Set(["ir.core", "ir.tuple", "ir.data", "ir.list"]), ...over });

  it("a model parameter of a bare error type is `ill-typed`, bare or wrapped", () => {
    for (const ty of ["E", "Option<E>", "List<E>", "(U8,E)"]) {
      const models = { m: { params: [["a", ty]], body: ["lit", true] } };
      expect(code(() => make({ models, modelOrder: ["m"] })), ty).toBe("ill-typed");
    }
    const ok = { m: { params: [["a", "Result<U8,E>"]], body: ["lit", true] } };
    expect(() => make({ models: ok, modelOrder: ["m"] })).not.toThrow();
  });

  it("an op parameter or result of a bare error type is `ill-typed`", () => {
    for (const row of [{ params: [["a", "E"]], ret: "Bool" }, { params: [["a", "U8"]], ret: "E" }, { params: [["a", "Option<E>"]], ret: "Bool" }]) {
      expect(code(() => make({ ops: { f: { ...row, requires: ["ir.core"] } } }))).toBe("ill-typed");
    }
  });
});

describe("the corpus reader (corpus.ts) applies the table check and [D75]", () => {
  const copy = (): { dir: string; document: Json } => {
    const dir = tempDir("warrant-errata-");
    cpSync((CORPORA.find((c) => c.name === "ct-compare") as { dir: string }).dir, dir, { recursive: true });
    return { dir, document: JSON.parse(readFileSync(join(dir, "corpus.json"), "utf8")) };
  };
  const save = (dir: string, document: Json): void => writeFileSync(join(dir, "corpus.json"), JSON.stringify(document));

  it("the unchanged corpus loads", () => {
    expect(() => loadV2Corpus(copy().dir)).not.toThrow();
  });

  it("a bad table is `corpus`, for every defect", () => {
    for (const [name, table] of Object.entries(DEFECTS)) {
      const { dir, document } = copy();
      document.types = clone(table);
      save(dir, document);
      let refused = "none";
      try {
        loadV2Corpus(dir);
      } catch (error) {
        if (error instanceof KitError) refused = error.code;
        else throw error;
      }
      expect(refused, name).toBe("corpus");
    }
  });

  it("an uncalled model with an Int parameter (also under Option) is read", () => {
    const { dir, document } = copy();
    document.models.wide_int = { params: [["a", "Int"]], body: ["lit", true] };
    document.models.wide_opt = { params: [["a", "Option<Int>"]], body: ["lit", true] };
    document.model_order = [...document.model_order, "wide_int", "wide_opt"];
    save(dir, document);
    expect(() => loadV2Corpus(dir)).not.toThrow();
  });
});

describe("more accepted tables and models", () => {
  it("a diamond reached through wrappers is not recursion", () => {
    const table = {
      A: { kind: "record", fields: [["l", "List<B>"], ["r", "Option<(U8,C)>"]] },
      B: { kind: "record", fields: [["d", "Option<D>"]] },
      C: { kind: "variant", ctors: [["X", ["List<D>"]], ["Y", []]] },
      D: { kind: "record", fields: [["x", "U8"]] },
    };
    for (const context of CONTEXTS) expect(() => readTypeTable(clone(table), context)).not.toThrow();
  });

  it("the same constructor name in two different variants is fine", () => {
    const table = {
      V: { kind: "variant", ctors: [["Same", []], ["Other", ["U8"]]] },
      W: { kind: "variant", ctors: [["Same", ["Bool"]], ["Else", []]] },
    };
    expect(() => readTypeTable(table, "corpus")).not.toThrow();
  });

  it("an uncalled model with a (Int,Bool) parameter leaves a core-only case accepted", () => {
    const c = {
      ...clone(kitCase("v2-ir/lit-bool-true")),
      models: { m: { params: [["a", "(Int,Bool)"]], body: ["lit", true] } },
      model_order: ["m"],
    };
    expect(runCase("v2-ir", c)).toEqual({ calls: 0, type: "Bool", value: true });
  });
});
