/**
 * Follow-ups of the three-way differential: a `types` that is not an object is refused in every context (`null`
 * included), and R58 (VER04r section 2.2, 16.16): a record, variant or constructor named for a reserved built-in is
 * refused with the context's code; a field name stays free.
 */
import { describe, expect, it } from "vitest";
import { readCorpusPart } from "../../src/v2/chunks.ts";
import { KitError } from "../../src/v2/errors.ts";
import { runCase } from "../../src/v2/kit.ts";
import { readTypeTable } from "../../src/v2/types.ts";
import { kitCase } from "./support.ts";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any
const LEVELS = new Set(["ir.core"]);
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

const NOT_TABLES: Record<string, Json> = {
  null: null,
  "an empty list": [],
  "a string": "x",
  "a number": 0,
  false: false,
  "a list holding a row": [{ kind: "error", cases: ["A"] }],
};

describe("a `types` that is not an object is refused in every context", () => {
  for (const [name, bad] of Object.entries(NOT_TABLES)) {
    it(`refuses ${name}`, () => {
      expect(runCase("v2-value", { ...clone(kitCase("v2-value/bool-true")), types: bad })).toEqual({ outcome: "reject", error: "type" });
      expect(runCase("v2-ir", { ...clone(kitCase("v2-ir/lit-bool-true")), types: bad })).toEqual({ error: "ill-typed" });
      expect(runCase("v2-domain", { ...clone(kitCase("v2-domain/bool-index-0")), types: bad })).toEqual({ error: "descriptor" });
      const chunk = clone(kitCase("v2-chunk/ragged-short-last-chunk"));
      chunk.corpus.types = bad;
      expect(code(() => readCorpusPart(chunk.corpus, LEVELS))).toBe("corpus");
      expect(runCase("v2-chunk", chunk)).toEqual({ error: "corpus" });
      for (const c of ["type", "ill-typed", "descriptor", "corpus"] as const) expect(code(() => readTypeTable(bad, c))).toBe(c);
    });
  }

  it("still treats an absent `types` as {} in v2-value", () => {
    const c = clone(kitCase("v2-value/bool-true"));
    delete c.types;
    expect(runCase("v2-value", c).outcome).toBe("accept");
  });
});

const RESERVED = [
  "Some", "None", "Ok", "Err", "len", "at", "concat", "fixed", "div",
  "Bool", "Nat", "U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Bytes", "List", "Option", "Result", "Int",
];

describe("R58: reserved built-ins are not declarable names", () => {
  for (const name of RESERVED) {
    const asRecord = { [name]: { kind: "record", fields: [["x", "U8"]] } };
    const asVariant = { [name]: { kind: "variant", ctors: [["A", []], ["B", []]] } };
    const asError = { [name]: { kind: "error", cases: ["A"] } };
    const asCtor = { V: { kind: "variant", ctors: [["A", []], [name, []]] } };
    it(`refuses ${name} as a type row and as a constructor`, () => {
      for (const table of [asRecord, asVariant, asError, asCtor]) {
        for (const c of ["type", "ill-typed", "descriptor", "corpus"] as const) expect(code(() => readTypeTable(clone(table), c))).toBe(c);
        expect(runCase("v2-value", { ...clone(kitCase("v2-value/bool-true")), types: clone(table) })).toEqual({ outcome: "reject", error: "type" });
        expect(runCase("v2-ir", { ...clone(kitCase("v2-ir/lit-bool-true")), types: clone(table) })).toEqual({ error: "ill-typed" });
        expect(runCase("v2-domain", { ...clone(kitCase("v2-domain/bool-index-0")), types: clone(table) })).toEqual({ error: "descriptor" });
        const chunk = clone(kitCase("v2-chunk/ragged-short-last-chunk"));
        chunk.corpus.types = clone(table);
        expect(runCase("v2-chunk", chunk)).toEqual({ error: "corpus" });
        expect(code(() => readCorpusPart(chunk.corpus, LEVELS))).toBe("corpus");
      }
    });
  }

  it("leaves field names free and accepts non-reserved names", () => {
    const table = {
      R: { kind: "record", fields: [["Some", "U8"], ["len", "Nat"], ["Ok", "Bool"]] },
      V: { kind: "variant", ctors: [["Alpha", []], ["Beta", ["R"]]] },
    };
    expect(readTypeTable(table).data.size).toBe(2);
  });
});
