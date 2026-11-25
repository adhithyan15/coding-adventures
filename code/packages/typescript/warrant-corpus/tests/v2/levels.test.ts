/** Feature levels and the tag registry (VER04r section 4.1, Appendix A, A.1, A.2). */
import { describe, expect, it } from "vitest";
import { CORE_TAG_LIST, DEFINED_LEVELS, FIXED_TAG_LIST, IMPLEMENTED_LEVELS, firstMissing, levelOfTag } from "../../src/v2/levels.ts";

describe("the tag registry", () => {
  it("holds the 21 tags v1 emits plus neg, concat, take, drop and is_err at ir.core (Appendix A)", () => {
    expect(CORE_TAG_LIST).toHaveLength(26);
    for (const tag of ["lit", "var", "subject_op", "model_call", "len", "ok", "err", "eq", "ne", "lt", "gt", "le", "ge", "add", "sub", "mul", "and", "or", "not", "implies", "if"]) {
      expect(levelOfTag(tag), tag).toBe("ir.core");
    }
    for (const tag of ["neg", "concat", "take", "drop", "is_err"]) expect(levelOfTag(tag), tag).toBe("ir.core");
  });

  it("holds exactly the 35 names of Appendix A.1 at ir.fixed [D24]", () => {
    expect(FIXED_TAG_LIST).toHaveLength(35);
    expect(new Set(FIXED_TAG_LIST).size).toBe(35);
    for (const tag of FIXED_TAG_LIST) expect(levelOfTag(tag), tag).toBe("ir.fixed");
  });

  it("places the tuple, data, list and reserved sequence names, and refuses the rest as unknown", () => {
    for (const tag of ["tuple", "project", "match"]) expect(levelOfTag(tag)).toBe("ir.tuple");
    for (const tag of ["some", "none", "ctor", "field"]) expect(levelOfTag(tag)).toBe("ir.data");
    expect(levelOfTag("list")).toBe("ir.list");
    for (const tag of ["fold", "map", "all", "any", "range", "count"]) expect(levelOfTag(tag)).toBe("ir.seq");
    for (const tag of ["frobnicate", "NOT", "Lit", "", "try_nat"]) expect(levelOfTag(tag), tag).toBeUndefined();
  });

  it("implements ir.core only and knows the name of every defined level", () => {
    expect(IMPLEMENTED_LEVELS).toEqual(["ir.core"]);
    for (const level of ["ir.core", "ir.fixed", "ir.tuple", "ir.data", "ir.list", "ir.seq", "ir.inout.records"]) expect(DEFINED_LEVELS.has(level), level).toBe(true);
    expect(DEFINED_LEVELS.has("ir.fancy")).toBe(false);
  });
});

describe("naming a missing level", () => {
  const have = new Set(["ir.core"]);
  it("names the highest-content level first: ir.list implies ir.data, so a List<T> names ir.list [D63]", () => {
    expect(firstMissing(["ir.core"], have)).toBeUndefined();
    expect(firstMissing([], have)).toBeUndefined();
    expect(firstMissing(["ir.core", "ir.fixed"], have)).toBe("ir.fixed");
    expect(firstMissing(["ir.data", "ir.list"], have)).toBe("ir.list");
    expect(firstMissing(["ir.fixed", "ir.tuple", "ir.data"], have)).toBe("ir.data");
    expect(firstMissing(["ir.tuple", "ir.fixed"], have)).toBe("ir.tuple");
    expect(firstMissing(["ir.seq", "ir.list"], have)).toBe("ir.list");
    expect(firstMissing(["ir.zzz", "ir.aaa"], have)).toBe("ir.aaa");
    expect(firstMissing(["ir.fixed"], new Set(["ir.core", "ir.fixed"]))).toBeUndefined();
  });
});
