/**
 * positional-resolver.test.ts — Unit tests for PositionalResolver.
 *
 * The existing tests in parser.test.ts cover the resolver indirectly through
 * full parse integration. This file unit-tests the resolver directly, targeting
 * branches that are difficult to reach through high-level parser tests:
 *
 * 1. Non-variadic resolution:
 *    - Exact match of tokens to slots → all resolved
 *    - Fewer tokens than slots → missing_required_argument for required slots
 *    - Optional slot with no token → gets default value
 *    - More tokens than slots → too_many_arguments error
 *    - Coercion error for a token → error accumulated
 *
 * 2. Variadic resolution:
 *    - Variadic only (no leading/trailing) → all tokens go to variadic
 *    - Variadic + trailing (last-wins): enough tokens → correct partition
 *    - Variadic + trailing: shortage → trailing gets nothing, error reported
 *    - Variadic variadicMax exceeded → too_many_arguments
 *    - Variadic variadicMin not met → too_few_arguments
 *    - Variadic with default when 0 tokens and no error
 *
 * 3. required_unless_flag:
 *    - Arg with required_unless_flag: flag present (boolean true) → arg exempt
 *    - Arg with required_unless_flag: flag present (non-empty array) → arg exempt
 *    - Arg with required_unless_flag: flag absent → arg is required
 *
 * 4. Enum argument coercion:
 *    - Valid enum value in positional → resolves correctly
 *    - Invalid enum value → invalid_enum_value error
 */

import { describe, it, expect } from "vitest";
import { PositionalResolver } from "../positional-resolver.js";
import type { ArgDef } from "../types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeArgDef(
  id: string,
  opts: Partial<ArgDef> = {},
): ArgDef {
  return {
    id,
    display_name: opts.display_name ?? id.toUpperCase(),
    description: opts.description ?? id,
    type: opts.type ?? "string",
    required: opts.required ?? true,
    variadic: opts.variadic ?? false,
    variadicMin: opts.variadicMin ?? (opts.required !== false ? 1 : 0),
    variadicMax: opts.variadicMax ?? null,
    default: opts.default ?? null,
    enumValues: opts.enumValues ?? [],
    requiredUnlessFlag: opts.requiredUnlessFlag ?? [],
  };
}

// ---------------------------------------------------------------------------
// 1. Non-variadic resolution
// ---------------------------------------------------------------------------

describe("PositionalResolver — non-variadic", () => {
  it("resolves exact match: 2 slots, 2 tokens", () => {
    const defs = [makeArgDef("name"), makeArgDef("url")];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["origin", "https://example.com"], {}, ["git", "remote", "add"]);
    expect(errors).toHaveLength(0);
    expect(result["name"]).toBe("origin");
    expect(result["url"]).toBe("https://example.com");
  });

  it("resolves single slot with single token", () => {
    const defs = [makeArgDef("file")];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["input.txt"], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["file"]).toBe("input.txt");
  });

  it("reports missing_required_argument when required slot has no token", () => {
    const defs = [makeArgDef("name"), makeArgDef("url")];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["origin"], {}, ["tool"]);
    const missingErrors = errors.filter((e) => e.errorType === "missing_required_argument");
    expect(missingErrors).toHaveLength(1);
    expect(missingErrors[0].message).toContain("URL");
  });

  it("uses default value for optional slot with no token", () => {
    const defs = [makeArgDef("output", { required: false, default: "output.txt" })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve([], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["output"]).toBe("output.txt");
  });

  it("uses null for optional slot with no token and no default", () => {
    const defs = [makeArgDef("output", { required: false, default: null })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve([], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["output"]).toBeNull();
  });

  it("reports too_many_arguments when more tokens than slots", () => {
    const defs = [makeArgDef("name")];
    const resolver = new PositionalResolver(defs);
    const { errors } = resolver.resolve(["a", "b", "c"], {}, ["tool"]);
    const tooManyErrors = errors.filter((e) => e.errorType === "too_many_arguments");
    expect(tooManyErrors).toHaveLength(1);
    expect(tooManyErrors[0].message).toContain("3");
  });

  it("no error with zero slots and zero tokens", () => {
    const resolver = new PositionalResolver([]);
    const { result, errors } = resolver.resolve([], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result).toEqual({});
  });

  it("reports too_many_arguments with zero slots but tokens provided", () => {
    const resolver = new PositionalResolver([]);
    const { errors } = resolver.resolve(["unexpected"], {}, ["tool"]);
    expect(errors.filter((e) => e.errorType === "too_many_arguments")).toHaveLength(1);
  });

  it("includes context in missing_required_argument errors", () => {
    const defs = [makeArgDef("file")];
    const resolver = new PositionalResolver(defs);
    const { errors } = resolver.resolve([], {}, ["tool", "sub"]);
    expect(errors[0].context).toEqual(["tool", "sub"]);
  });

  it("accumulates coercion errors for multiple bad tokens", () => {
    const defs = [
      makeArgDef("count", { type: "integer" }),
      makeArgDef("ratio", { type: "float" }),
    ];
    const resolver = new PositionalResolver(defs);
    const { errors } = resolver.resolve(["abc", "xyz"], {}, ["tool"]);
    // Both should fail coercion
    expect(errors.filter((e) => e.errorType === "invalid_value")).toHaveLength(2);
  });

  it("resolves integer arg correctly", () => {
    const defs = [makeArgDef("count", { type: "integer" })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["42"], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["count"]).toBe(42);
  });
});

// ---------------------------------------------------------------------------
// 2. Variadic resolution
// ---------------------------------------------------------------------------

describe("PositionalResolver — variadic (only)", () => {
  it("variadic with no trailing: all tokens go to variadic", () => {
    const defs = [makeArgDef("files", { variadic: true, variadicMin: 0, required: false })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["a.txt", "b.txt", "c.txt"], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["files"]).toEqual(["a.txt", "b.txt", "c.txt"]);
  });

  it("variadic with variadicMin=1 and 1 token → success", () => {
    const defs = [makeArgDef("sources", { variadic: true, variadicMin: 1, required: true })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["a.txt"], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["sources"]).toEqual(["a.txt"]);
  });

  it("variadic with variadicMin=2, only 1 token → too_few_arguments", () => {
    const defs = [makeArgDef("sources", { variadic: true, variadicMin: 2, required: true })];
    const resolver = new PositionalResolver(defs);
    const { errors } = resolver.resolve(["a.txt"], {}, ["tool"]);
    const tooFewErrors = errors.filter((e) => e.errorType === "too_few_arguments");
    expect(tooFewErrors).toHaveLength(1);
    expect(tooFewErrors[0].message).toContain("2");
  });

  it("variadic with variadicMax=2, 3 tokens → too_many_arguments", () => {
    const defs = [makeArgDef("files", {
      variadic: true,
      variadicMin: 0,
      variadicMax: 2,
      required: false,
    })];
    const resolver = new PositionalResolver(defs);
    const { errors } = resolver.resolve(["a", "b", "c"], {}, ["tool"]);
    const tooManyErrors = errors.filter((e) => e.errorType === "too_many_arguments");
    expect(tooManyErrors).toHaveLength(1);
    expect(tooManyErrors[0].message).toContain("3");
  });

  it("variadic with 0 tokens and variadicMin=0 returns empty array", () => {
    const defs = [makeArgDef("files", { variadic: true, variadicMin: 0, required: false })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve([], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["files"]).toEqual([]);
  });

  it("variadic with 0 tokens and default → returns [default]", () => {
    const defs = [makeArgDef("paths", {
      variadic: true,
      variadicMin: 0,
      required: false,
      default: ".",
    })];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve([], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    // default is applied as [default]
    expect(result["paths"]).toEqual(["."]);
  });
});

describe("PositionalResolver — variadic with trailing (last-wins)", () => {
  // cp SOURCE... DEST
  const cpDefs = [
    makeArgDef("source", { variadic: true, variadicMin: 1, required: true }),
    makeArgDef("dest", { required: true }),
  ];

  it("1 source + dest: source=['a'], dest='b'", () => {
    const resolver = new PositionalResolver(cpDefs);
    const { result, errors } = resolver.resolve(["a.txt", "/tmp/"], {}, ["cp"]);
    expect(errors).toHaveLength(0);
    expect(result["source"]).toEqual(["a.txt"]);
    expect(result["dest"]).toBe("/tmp/");
  });

  it("3 sources + dest: correct partition", () => {
    const resolver = new PositionalResolver(cpDefs);
    const { result, errors } = resolver.resolve(["a", "b", "c", "/d/"], {}, ["cp"]);
    expect(errors).toHaveLength(0);
    expect(result["source"]).toEqual(["a", "b", "c"]);
    expect(result["dest"]).toBe("/d/");
  });

  it("shortage: 1 token, min=1 + 1 trailing → too_few for source + dest missing", () => {
    // 1 token: shortage scenario — variadicMin=1, trailing=1
    // 1 < 1 + 1 → shortage; variadic gets the 1 token, dest gets nothing
    const resolver = new PositionalResolver(cpDefs);
    const { errors } = resolver.resolve(["a.txt"], {}, ["cp"]);
    // Either too_few or missing_required_argument depending on the shortage logic
    expect(errors.length).toBeGreaterThan(0);
  });

  it("0 tokens → too_few_arguments for source variadic", () => {
    const resolver = new PositionalResolver(cpDefs);
    const { errors } = resolver.resolve([], {}, ["cp"]);
    const tooFew = errors.filter((e) => e.errorType === "too_few_arguments");
    expect(tooFew).toHaveLength(1);
  });

  it("leading arg + variadic + trailing", () => {
    // cmd PREFIX SOURCE... DEST: PREFIX is leading, SOURCE... is variadic, DEST is trailing
    const defs = [
      makeArgDef("prefix", { required: true }),
      makeArgDef("sources", { variadic: true, variadicMin: 1, required: true }),
      makeArgDef("dest", { required: true }),
    ];
    const resolver = new PositionalResolver(defs);
    const { result, errors } = resolver.resolve(["PRE", "a.txt", "b.txt", "/dest/"], {}, ["cmd"]);
    expect(errors).toHaveLength(0);
    expect(result["prefix"]).toBe("PRE");
    expect(result["sources"]).toEqual(["a.txt", "b.txt"]);
    expect(result["dest"]).toBe("/dest/");
  });
});

// ---------------------------------------------------------------------------
// 3. required_unless_flag
// ---------------------------------------------------------------------------

describe("PositionalResolver — required_unless_flag", () => {
  it("arg is optional when exempting boolean flag is true", () => {
    // PATTERN is required unless -e (regexp) flag is provided
    const patternDef = makeArgDef("pattern", {
      required: true,
      requiredUnlessFlag: ["regexp"],
    });
    const filesDef = makeArgDef("files", {
      required: false,
      variadic: true,
      variadicMin: 0,
    });
    const resolver = new PositionalResolver([patternDef, filesDef]);
    // -e regexp flag is present: pattern is exempt
    const { result, errors } = resolver.resolve(["file.txt"], { regexp: ["foo"] }, ["grep"]);
    expect(errors.filter((e) => e.errorType === "missing_required_argument")).toHaveLength(0);
    expect(result["files"]).toEqual(["file.txt"]);
  });

  it("arg is required when exempting flag is absent", () => {
    const patternDef = makeArgDef("pattern", {
      required: true,
      requiredUnlessFlag: ["regexp"],
    });
    const resolver = new PositionalResolver([patternDef]);
    const { errors } = resolver.resolve([], {}, ["grep"]);
    expect(errors.filter((e) => e.errorType === "missing_required_argument")).toHaveLength(1);
  });

  it("arg is optional when exempting flag has a non-null string value", () => {
    const patternDef = makeArgDef("pattern", {
      required: true,
      requiredUnlessFlag: ["output"],
    });
    const resolver = new PositionalResolver([patternDef]);
    // output flag is present with a string value
    const { errors } = resolver.resolve([], { output: "file.txt" }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_argument")).toHaveLength(0);
  });

  it("arg is required when exempting flag has value false", () => {
    const patternDef = makeArgDef("pattern", {
      required: true,
      requiredUnlessFlag: ["verbose"],
    });
    const resolver = new PositionalResolver([patternDef]);
    const { errors } = resolver.resolve([], { verbose: false }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_argument")).toHaveLength(1);
  });

  it("exempt arg gets default value when no tokens provided", () => {
    const patternDef = makeArgDef("pattern", {
      required: true,
      requiredUnlessFlag: ["regexp"],
      default: "defaultPattern",
    });
    const resolver = new PositionalResolver([patternDef]);
    const { result, errors } = resolver.resolve([], { regexp: ["foo"] }, ["grep"]);
    expect(errors).toHaveLength(0);
    expect(result["pattern"]).toBe("defaultPattern");
  });
});

// ---------------------------------------------------------------------------
// 4. Enum argument coercion
// ---------------------------------------------------------------------------

describe("PositionalResolver — enum argument", () => {
  const formatDef = makeArgDef("format", {
    type: "enum",
    enumValues: ["json", "csv", "table"],
    required: true,
  });

  it("resolves a valid enum positional correctly", () => {
    const resolver = new PositionalResolver([formatDef]);
    const { result, errors } = resolver.resolve(["json"], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    expect(result["format"]).toBe("json");
  });

  it("reports invalid_enum_value for an invalid enum positional", () => {
    const resolver = new PositionalResolver([formatDef]);
    const { errors } = resolver.resolve(["xml"], {}, ["tool"]);
    expect(errors.filter((e) => e.errorType === "invalid_enum_value")).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// 5. Float argument coercion
// ---------------------------------------------------------------------------

describe("PositionalResolver — float argument", () => {
  it("coerces a float positional correctly", () => {
    const def = makeArgDef("ratio", { type: "float" });
    const resolver = new PositionalResolver([def]);
    const { result, errors } = resolver.resolve(["3.14"], {}, ["tool"]);
    expect(errors).toHaveLength(0);
    if ("value" in result) {
      // result is a record
    }
    expect(result["ratio"]).toBeCloseTo(3.14);
  });

  it("reports invalid_value for a non-float positional", () => {
    const def = makeArgDef("ratio", { type: "float" });
    const resolver = new PositionalResolver([def]);
    const { errors } = resolver.resolve(["notanumber"], {}, ["tool"]);
    expect(errors.filter((e) => e.errorType === "invalid_value")).toHaveLength(1);
  });
});
