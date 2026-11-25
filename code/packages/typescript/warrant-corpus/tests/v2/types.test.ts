/** Types: the section 2.1 grammar, the section 2.2 table, flows and unify (R51), static widths (R70). */
import { describe, expect, it } from "vitest";
import { KitError } from "../../src/v2/errors.ts";
import {
  EMPTY_TABLE, type Ty, checkNames, eraseNamedWidths, flows, isExact, isFixed, isInteger, isName, isUndetermined,
  parseType, readTypeTable, sameType, showType, substituteWidths, typeLevels, unify, widthOf,
} from "../../src/v2/types.ts";

const code = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.code;
    throw error;
  }
  return "none";
};
const levels = (text: string): string[] => {
  const out = new Set<string>();
  typeLevels(parseType(text), out);
  return [...out].sort();
};

describe("parseType", () => {
  it("reads every scalar and the composite forms", () => {
    for (const text of ["Bool", "U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Nat", "Bytes", "Bytes<0>", "Bytes<4096>", "(U8,Bool)", "List<U8>", "Option<Nat>", "Result<U8,E>", "Rec"]) {
      expect(showType(parseType(text))).toBe(text);
    }
    expect(showType(parseType("(U8,(Bool,Nat),Bytes<3>)"))).toBe("(U8,(Bool,Nat),Bytes<3>)");
  });

  it("refuses what section 2.1 refuses", () => {
    for (const text of [
      "", "Int", "U8 ", " U8", "Bytes<07>", "Bytes<4097>", "Bytes<00>", "Bytes<N>", "Bytes<", "Bytes<5", "Bytes<12345>",
      "(U8)", "(U8,", "(U8,Bool", "(A,B,C,D,E,F,G,H,I)", "List<U8", "List<>", "Option", "Result<U8>", "Result<U8,>", "Result<U8,E", "Result<U8,1>",
      "Result<U8, E>", "U8,U8", "<U8>", "Bool<U8>",
    ]) {
      expect(code(() => parseType(text)), text).toBe("type");
    }
  });

  it("allows Int and a named width only when asked", () => {
    expect(showType(parseType("Int", { allowInt: true }))).toBe("Int");
    expect(showType(parseType("Bytes<N>", { allowNamedWidth: true }))).toBe("Bytes<N>");
    expect(code(() => parseType("Int"))).toBe("type");
    expect(code(() => parseType("Bytes<N>"))).toBe("type");
    expect(code(() => parseType("Int", { code: "ill-typed" }))).toBe("ill-typed");
  });

  it("counts the constructors that enclose a name: 16 are accepted, 17 refused (R40)", () => {
    const wrap = (n: number): string => `${"Option<".repeat(n)}Bool${">".repeat(n)}`;
    expect(code(() => parseType(wrap(16)))).toBe("none");
    expect(code(() => parseType(wrap(17)))).toBe("type");
    expect(code(() => parseType(wrap(17), { maxDepth: 64 }))).toBe("none");
    expect(code(() => parseType(wrap(65), { maxDepth: 64 }))).toBe("type");
    expect(code(() => parseType(`${"(".repeat(1000)}U8`))).toBe("type");
  });
});

describe("the types table (section 2.2)", () => {
  it("reads error, record and variant rows", () => {
    const table = readTypeTable({
      E: { kind: "error", cases: ["A", "B"] },
      R: { kind: "record", fields: [["x", "U8"]] },
      V: { kind: "variant", ctors: [["A", []], ["B", ["Nat"]]] },
    });
    expect(table.errors.get("E")).toEqual(["A", "B"]);
    expect(table.data.get("R")).toBe("record");
    expect(table.data.get("V")).toBe("variant");
  });

  it("refuses every violation of the rules", () => {
    const bad: unknown[] = [
      [], null, 7,
      { "1x": { kind: "error", cases: ["A"] } },
      { Bool: { kind: "error", cases: ["A"] } },
      { E: 7 }, { E: [] },
      { E: { kind: "enum" } },
      { E: { kind: "error", cases: [] } },
      { E: { kind: "error", cases: Array.from({ length: 65 }, (_u, i) => `C${i}`) } },
      { E: { kind: "error", cases: ["A", "A"] } },
      { E: { kind: "error", cases: ["A-B"] } },
      { E: { kind: "error", cases: [1] } },
      { E: { kind: "error" } },
      { R: { kind: "record", fields: [] } },
      { R: { kind: "record" } },
      { R: { kind: "record", fields: Array.from({ length: 33 }, () => ["x", "U8"]) } },
      { V: { kind: "variant", ctors: [["A", []]] } },
      { V: { kind: "variant" } },
    ];
    for (const raw of bad) expect(code(() => readTypeTable(raw)), JSON.stringify(raw)).toBe("corpus");
    expect(code(() => readTypeTable([], "type"))).toBe("type");
  });

  it("checks that names are declared", () => {
    const table = readTypeTable({ E: { kind: "error", cases: ["A"] }, R: { kind: "record", fields: [["x", "U8"]] } });
    const check = (text: string): string => code(() => checkNames(parseType(text), table));
    expect(check("Result<U8,E>")).toBe("none");
    expect(check("Result<U8,X>")).toBe("type");
    expect(check("Result<U8,R>")).toBe("type");
    expect(check("R")).toBe("none");
    expect(check("E")).toBe("type");
    expect(check("u8")).toBe("type");
    expect(check("U128")).toBe("type");
    expect(check("List<Option<R>>")).toBe("none");
    expect(check("(R,Nope)")).toBe("type");
    expect(check("List<Nope>")).toBe("type");
    expect(check("Option<Nope>")).toBe("type");
    expect(check("Result<Nope,E>")).toBe("type");
    expect(isName("A_1")).toBe(true);
    expect(isName("1A")).toBe(false);
  });
});

describe("levels of a type (R69 b)", () => {
  it("names the level each type needs", () => {
    expect(levels("Bool")).toEqual(["ir.core"]);
    expect(levels("Result<Bytes,E>")).toEqual(["ir.core"]);
    expect(levels("(U8,U8)")).toEqual(["ir.core", "ir.tuple"]);
    expect(levels("Option<U8>")).toEqual(["ir.core", "ir.data"]);
    expect(levels("Rec")).toEqual(["ir.data"]);
    expect(levels("List<U8>")).toEqual(["ir.core", "ir.list"]);
    expect(levels("Result<(U8,U8),E>")).toEqual(["ir.core", "ir.tuple"]);
    expect(levels("Result<Nat,E>")).toEqual(["ir.core"]);
  });
});

describe("structure helpers", () => {
  const t = (text: string): Ty => parseType(text, { allowInt: true, allowNamedWidth: true });
  it("compares structurally, widths and payloads included", () => {
    expect(sameType(t("U8"), t("U8"))).toBe(true);
    expect(sameType(t("U8"), t("U16"))).toBe(false);
    expect(sameType(t("U8"), t("I8"))).toBe(false);
    expect(sameType(t("Bytes<2>"), t("Bytes<2>"))).toBe(true);
    expect(sameType(t("Bytes<2>"), t("Bytes"))).toBe(false);
    expect(sameType(t("(U8,Bool)"), t("(U8,Bool)"))).toBe(true);
    expect(sameType(t("(U8,Bool)"), t("(U8,Nat)"))).toBe(false);
    expect(sameType(t("(U8,Bool)"), t("(U8,Bool,Bool)"))).toBe(false);
    expect(sameType(t("List<U8>"), t("List<U8>"))).toBe(true);
    expect(sameType(t("Option<U8>"), t("Option<U8>"))).toBe(true);
    expect(sameType({ k: "Option", of: null }, { k: "Option", of: null })).toBe(true);
    expect(sameType({ k: "Option", of: null }, t("Option<U8>"))).toBe(false);
    expect(sameType(t("Result<U8,E>"), t("Result<U8,E>"))).toBe(true);
    expect(sameType(t("Result<U8,E>"), t("Result<U8,F>"))).toBe(false);
    expect(sameType({ k: "Result", ok: null, err: "E" }, { k: "Result", ok: null, err: "E" })).toBe(true);
    expect(sameType({ k: "Result", ok: null, err: "E" }, t("Result<U8,E>"))).toBe(false);
    expect(sameType(t("Rec"), t("Rec"))).toBe(true);
    expect(sameType(t("Rec"), t("Sec"))).toBe(false);
    expect(sameType(t("Bool"), t("Nat"))).toBe(false);
  });

  it("detects undetermined components and classifies integer types", () => {
    expect(isUndetermined({ k: "Result", ok: t("U8"), err: null })).toBe(true);
    expect(isUndetermined({ k: "Result", ok: null, err: "E" })).toBe(true);
    expect(isUndetermined(t("Result<U8,E>"))).toBe(false);
    expect(isUndetermined({ k: "Option", of: null })).toBe(true);
    expect(isUndetermined(t("Option<U8>"))).toBe(false);
    expect(isUndetermined(t("(U8,Option<U8>)"))).toBe(false);
    expect(isUndetermined({ k: "Tuple", of: [t("U8"), { k: "Option", of: null }] })).toBe(true);
    expect(isUndetermined({ k: "List", of: { k: "Option", of: null } })).toBe(true);
    expect(isUndetermined(t("Bool"))).toBe(false);
    expect(isFixed(t("U8"))).toBe(true);
    expect(isFixed(t("Nat"))).toBe(false);
    expect(isExact(t("Int"))).toBe(true);
    expect(isInteger(t("Bool"))).toBe(false);
    expect(widthOf(t("I16"))).toBe(16);
    expect(widthOf(t("Nat"))).toBeUndefined();
  });

  it("prints widths unless asked for the plain form", () => {
    expect(showType(t("Result<Bytes<2>,E>"))).toBe("Result<Bytes<2>,E>");
    expect(showType(t("Result<Bytes<2>,E>"), true)).toBe("Result<Bytes,E>");
    expect(showType({ k: "Option", of: null })).toBe("Option<?>");
    expect(showType({ k: "Result", ok: null, err: null })).toBe("Result<?,?>");
    expect(showType(t("List<(Bytes<1>,U8)>"), true)).toBe("List<(Bytes,U8)>");
  });
});

describe("flows and unify (R51)", () => {
  const t = (text: string): Ty => parseType(text, { allowInt: true });
  it("flows: itself, Nat to Int, Bytes<N> to Bytes, lifted through Option and Result", () => {
    expect(flows(t("U8"), t("U8"))).toBe(true);
    expect(flows(t("Nat"), t("Int"))).toBe(true);
    expect(flows(t("Int"), t("Nat"))).toBe(false);
    expect(flows(t("Bytes<2>"), t("Bytes"))).toBe(true);
    expect(flows(t("Bytes"), t("Bytes<2>"))).toBe(false);
    expect(flows(t("Bytes<2>"), t("Bytes<3>"))).toBe(false);
    expect(flows(t("Option<Nat>"), t("Option<Int>"))).toBe(true);
    expect(flows(t("Option<Option<Nat>>"), t("Option<Option<Int>>"))).toBe(true);
    expect(flows({ k: "Option", of: null }, t("Option<U8>"))).toBe(true);
    expect(flows(t("Result<Nat,E>"), t("Result<Int,E>"))).toBe(true);
    expect(flows(t("Result<Nat,E>"), t("Result<Int,F>"))).toBe(false);
    expect(flows({ k: "Result", ok: t("Bytes<1>"), err: null }, t("Result<Bytes,E>"))).toBe(true);
    expect(flows({ k: "Result", ok: null, err: "E" }, t("Result<U8,E>"))).toBe(true);
    expect(flows({ k: "Result", ok: null, err: "E" }, t("Result<U8,F>"))).toBe(false);
    expect(flows({ k: "Result", ok: null, err: null, cands: ["E", "F"] }, t("Result<U8,F>"))).toBe(true);
    expect(flows({ k: "Result", ok: null, err: null, cands: ["E", "F"] }, t("Result<U8,G>"))).toBe(false);
    expect(flows(t("(Nat,Nat)"), t("(Int,Int)"))).toBe(false);
    expect(flows(t("List<Nat>"), t("List<Int>"))).toBe(false);
    expect(flows(t("Option<(Nat,Nat)>"), t("Option<(Int,Int)>"))).toBe(false);
    expect(flows(t("Bool"), t("U8"))).toBe(false);
  });

  it("unify: the common type, or none", () => {
    expect(unify(t("U8"), t("U8"))).toEqual(t("U8"));
    expect(unify(t("Nat"), t("Int"))).toEqual(t("Int"));
    expect(unify(t("Int"), t("Nat"))).toEqual(t("Int"));
    expect(unify(t("U8"), t("Nat"))).toBeNull();
    expect(unify(t("Bool"), t("U8"))).toBeNull();
    expect(showType(unify(t("Bytes<2>"), t("Bytes<3>")) as Ty)).toBe("Bytes");
    expect(showType(unify(t("Bytes<2>"), t("Bytes")) as Ty)).toBe("Bytes");
    expect(unify({ k: "Bytes", n: 2 }, { k: "Bytes", n: "N" })).toBeNull();
    expect(unify({ k: "Bytes", n: "N" }, { k: "Bytes", n: 2 })).toBeNull();
    expect(showType(unify({ k: "Bytes", n: "N" }, { k: "Bytes", n: null }) as Ty)).toBe("Bytes");
    expect(showType(unify(t("Option<Nat>"), t("Option<Int>")) as Ty)).toBe("Option<Int>");
    expect(showType(unify({ k: "Option", of: null }, t("Option<U8>")) as Ty)).toBe("Option<U8>");
    expect(showType(unify(t("Option<U8>"), { k: "Option", of: null }) as Ty)).toBe("Option<U8>");
    expect(unify(t("Option<U8>"), t("Option<Bool>"))).toBeNull();
    expect(showType(unify({ k: "Result", ok: t("U8"), err: null }, { k: "Result", ok: null, err: "E" }) as Ty)).toBe("Result<U8,E>");
    expect(showType(unify({ k: "Result", ok: null, err: "E" }, { k: "Result", ok: t("U8"), err: null }) as Ty)).toBe("Result<U8,E>");
    expect(unify(t("Result<U8,E>"), t("Result<U8,F>"))).toBeNull();
    expect(unify(t("Result<U8,E>"), t("Result<Bool,E>"))).toBeNull();
    expect(showType(unify(t("Result<Nat,E>"), t("Result<Int,E>")) as Ty)).toBe("Result<Int,E>");
    expect(unify({ k: "Result", ok: null, err: "E" }, { k: "Result", ok: null, err: "E" })).toEqual({ k: "Result", ok: null, err: "E" });
    expect(unify({ k: "Result", ok: null, err: "G" }, { k: "Result", ok: null, err: null, cands: ["E"] })).toBeNull();
    expect(unify({ k: "Result", ok: t("U8"), err: null }, { k: "Result", ok: t("U8"), err: null })).toEqual({ k: "Result", ok: t("U8"), err: null });
    // two bare errors whose candidate sets meet
    expect(unify({ k: "Result", ok: null, err: null, cands: ["E", "F"] }, { k: "Result", ok: null, err: null, cands: ["F", "G"] })).toMatchObject({ err: "F" });
    expect(unify({ k: "Result", ok: null, err: null, cands: ["E", "F"] }, { k: "Result", ok: null, err: null, cands: ["E", "F", "G"] })).toMatchObject({ err: null, cands: ["E", "F"] });
    expect(unify({ k: "Result", ok: null, err: null, cands: ["E"] }, { k: "Result", ok: null, err: null, cands: ["F"] })).toBeNull();
    expect(unify({ k: "Result", ok: null, err: null, cands: ["E", "F"] }, { k: "Result", ok: null, err: null })).toMatchObject({ cands: ["E", "F"] });
    expect(unify({ k: "Result", ok: null, err: null }, { k: "Result", ok: null, err: null, cands: ["E", "F"] })).toMatchObject({ cands: ["E", "F"] });
    expect(unify({ k: "Result", ok: null, err: null }, { k: "Result", ok: null, err: null, cands: ["E"] })).toMatchObject({ err: "E" });
    expect(unify({ k: "Result", ok: null, err: null, cands: ["E"] }, { k: "Result", ok: null, err: null })).toMatchObject({ err: "E" });
    expect(unify({ k: "Result", ok: null, err: null }, { k: "Result", ok: null, err: null })).toMatchObject({ err: null });
    expect(unify({ k: "Result", ok: null, err: null, cands: ["E", "F"] }, { k: "Result", ok: null, err: "F" })).toMatchObject({ err: "F" });
    expect(unify(t("(U8,U8)"), t("(U8,U8)"))).toEqual(t("(U8,U8)"));
    expect(unify(t("(U8,U8)"), t("(U8,U16)"))).toBeNull();
  });

  it("substitutes static widths and erases named ones (R70)", () => {
    const sig = parseType("Result<(Bytes<N>,List<Bytes<M>>),E>", { allowNamedWidth: true });
    expect(showType(substituteWidths(sig, ["N", "M"], [2, 3]))).toBe("Result<(Bytes<2>,List<Bytes<3>>),E>");
    expect(showType(substituteWidths(sig, ["Z"], [9]))).toBe("Result<(Bytes<N>,List<Bytes<M>>),E>");
    expect(showType(eraseNamedWidths(sig))).toBe("Result<(Bytes,List<Bytes>),E>");
    const option = parseType("Option<Bytes<N>>", { allowNamedWidth: true });
    expect(showType(substituteWidths(option, ["N"], [1]))).toBe("Option<Bytes<1>>");
    expect(showType(eraseNamedWidths(option))).toBe("Option<Bytes>");
    expect(showType(substituteWidths({ k: "Option", of: null }, ["N"], [1]))).toBe("Option<?>");
    expect(showType(eraseNamedWidths({ k: "Option", of: null }))).toBe("Option<?>");
    expect(showType(substituteWidths({ k: "Result", ok: null, err: "E" }, ["N"], [1]))).toBe("Result<?,E>");
    expect(showType(eraseNamedWidths({ k: "Result", ok: null, err: "E" }))).toBe("Result<?,E>");
    expect(showType(eraseNamedWidths(parseType("Bytes<3>")))).toBe("Bytes<3>");
    expect(showType(substituteWidths(parseType("U8"), ["N"], [1]))).toBe("U8");
    expect(EMPTY_TABLE.errors.size).toBe(0);
  });
});
