/**
 * The three leniencies the dual-lane review found in the CV2-S7a runner (VER04r section 2.2 rules 3 and 4, R44, R72,
 * section 5.5): inputs this lane accepted and the spec says every consumer refuses.
 */
import { describe, expect, it } from "vitest";
import { loadSuite, readCorpusPart } from "../../src/v2/chunks.ts";
import { chunkFileName } from "../../src/v2/corpus.ts";
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

/** The malformed `types` tables of R2: each is a refusal of section 2.2 rules 3 and 4. */
const BAD_TABLES: Record<string, Json> = {
  "a record field name outside the grammar": { R: { kind: "record", fields: [["9a", "U8"]] } },
  "a constructor of 9 fields": { V: { kind: "variant", ctors: [["A", []], ["B", ["U8", "U8", "U8", "U8", "U8", "U8", "U8", "U8", "U8"]]] } },
  "a constructor name outside the grammar": { V: { kind: "variant", ctors: [["A", []], ["9", []]] } },
  "a field that is not a pair": { R: { kind: "record", fields: ["x"] } },
  "a field of three entries": { R: { kind: "record", fields: [["x", "U8", "U8"]] } },
  "a field whose type is not text": { R: { kind: "record", fields: [["x", 8]] } },
  "a constructor that is not a pair": { V: { kind: "variant", ctors: [["A", []], "B"] } },
  "a constructor whose fields are not a list": { V: { kind: "variant", ctors: [["A", []], ["B", "U8"]] } },
  "a constructor field that is not text": { V: { kind: "variant", ctors: [["A", []], ["B", [1]]] } },
};
const GOOD_TABLE = {
  R: { kind: "record", fields: [["x", "U8"], ["_y9", "Nat"]] },
  V: { kind: "variant", ctors: [["A", []], ["B", ["U8", "U8", "U8", "U8", "U8", "U8", "U8", "U8"]]] },
};

describe("R2: readTypeTable enforces section 2.2 rules 3 and 4", () => {
  it("accepts a well-formed table, 8 fields in a constructor included", () => {
    expect(readTypeTable(GOOD_TABLE).data.size).toBe(2);
  });

  for (const [name, table] of Object.entries(BAD_TABLES)) {
    it(`refuses ${name} with the caller's code`, () => {
      for (const c of ["type", "ill-typed", "descriptor", "corpus"] as const) {
        expect(code(() => readTypeTable(clone(table), c))).toBe(c);
      }
    });
  }

  it("refuses in v2-value with `type`", () => {
    for (const table of Object.values(BAD_TABLES)) {
      const c = { ...clone(kitCase("v2-value/bool-true")), types: clone(table) };
      expect(runCase("v2-value", c)).toEqual({ outcome: "reject", error: "type" });
    }
    expect(runCase("v2-value", { ...clone(kitCase("v2-value/bool-true")), types: GOOD_TABLE }).outcome).toBe("accept");
  });

  it("refuses in v2-ir with `ill-typed`", () => {
    for (const table of Object.values(BAD_TABLES)) {
      const c = { ...clone(kitCase("v2-ir/lit-bool-true")), types: clone(table) };
      expect(runCase("v2-ir", c)).toEqual({ error: "ill-typed" });
    }
  });

  it("refuses in v2-domain with `descriptor`", () => {
    for (const table of Object.values(BAD_TABLES)) {
      const c = { ...clone(kitCase("v2-domain/bool-index-0")), types: clone(table) };
      expect(runCase("v2-domain", c)).toEqual({ error: "descriptor" });
    }
  });

  it("refuses in v2-chunk and in a corpus part with `corpus`", () => {
    const base = kitCase("v2-chunk/ragged-short-last-chunk");
    for (const table of Object.values(BAD_TABLES)) {
      const c = clone(base);
      c.corpus.types = clone(table);
      expect(code(() => readCorpusPart(c.corpus, LEVELS))).toBe("corpus");
      expect(runCase("v2-chunk", c)).toEqual({ error: "corpus" });
    }
  });
});

describe("R1: runDomain reads the case's `types` table (R44, R72)", () => {
  const result = (errs: string[], types?: Json): Json => ({
    id: "v2-domain/x",
    descriptor: { d: "result", ok: { d: "bool", size: 2 }, errs, size: 2 + errs.length },
    type: "Result<Bool,E>",
    index: 0,
    requires: ["ir.core"],
    ...(types === undefined ? {} : { types }),
  });
  const declared = { E: { kind: "error", cases: ["A", "B"] } };

  it("accepts an errs list equal to the declared cases", () => {
    expect(runCase("v2-domain", result(["A", "B"], declared))).toEqual({ size: 4, values_hex: ["5000"] });
  });

  it("refuses a list that differs, in content, length or order", () => {
    expect(runCase("v2-domain", result(["A", "C"], declared))).toEqual({ error: "descriptor" });
    expect(runCase("v2-domain", result(["A"], declared))).toEqual({ error: "descriptor" });
    expect(runCase("v2-domain", result(["A", "B", "C"], declared))).toEqual({ error: "descriptor" });
    expect(runCase("v2-domain", result(["B", "A"], declared))).toEqual({ error: "descriptor" });
  });

  it("refuses a list for an error type the table does not declare", () => {
    expect(runCase("v2-domain", result(["A", "B"], {}))).toEqual({ error: "descriptor" });
  });

  it("refuses an invalid table and a table that is not an object", () => {
    expect(runCase("v2-domain", result(["A"], { E: { kind: "error", cases: [] } }))).toEqual({ error: "descriptor" });
    expect(runCase("v2-domain", result(["A"], []))).toEqual({ error: "descriptor" });
    expect(runCase("v2-domain", result(["A"], null))).toEqual({ error: "descriptor" });
  });

  it("with no table checks the list by itself, as before", () => {
    expect(runCase("v2-domain", result(["A", "C"]))).toEqual({ size: 4, values_hex: ["5000"] });
  });
});

describe("R5: the device-name check on a claim is in loadSuite (section 5.5)", () => {
  const base = kitCase("v2-chunk/ragged-short-last-chunk");
  const load = (claim: string): string => {
    const c = clone(base);
    c.suite.claim = claim;
    return code(() => loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite));
  };

  it("refuses a reserved device segment, case-insensitively, anywhere in the claim", () => {
    for (const claim of ["nul", "NUL", "con/post", "x/com1", "lpt9/y", "aux/prn", "Com5"]) expect(load(claim)).toBe("corpus");
  });

  it("accepts names that merely contain a device name", () => {
    for (const claim of ["console", "nul_x", "com10", "a/b"]) expect(load(claim)).toBe("none");
  });

  it("is also refused by chunkFileName, with the same code", () => {
    expect(code(() => chunkFileName("con/post"))).toBe("corpus");
    expect(chunkFileName("a/post")).toBe("a.post.txt");
  });
});
