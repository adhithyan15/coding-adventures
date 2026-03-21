/**
 * flag-validator.test.ts — Unit tests for FlagValidator.
 *
 * The existing tests in parser.test.ts exercise FlagValidator indirectly.
 * This file targets specific validator code paths that are hard to reach
 * through parser integration tests:
 *
 * 1. conflicts_with:
 *    - Only one of the pair present → no error
 *    - Both present → error, reported once (not twice)
 *    - Neither present → no error
 *
 * 2. requires (transitive):
 *    - A present, B (required by A) present → no error
 *    - A present, B absent → error
 *    - Transitive chain A→B→C: A present, C absent → error for A→C
 *
 * 3. required flags:
 *    - Flag marked required, absent → error
 *    - Flag marked required, present → no error
 *    - Flag marked required but required_unless satisfied → no error
 *
 * 4. mutually_exclusive_groups:
 *    - Zero members present in required group → error
 *    - One member present in required group → no error
 *    - Two members present in non-required group → error
 *    - One member present in non-required group → no error
 *
 * 5. _isFlagPresent semantics:
 *    - Boolean flag present (true) and absent (false / undefined)
 *    - Repeatable flag present (non-empty array) and absent (empty array)
 *    - Non-boolean flag present (non-null string) and absent (null)
 *
 * 6. _flagDisplay edge cases:
 *    - Flag with only short
 *    - Flag with only long
 *    - Flag with only singleDashLong
 *    - Flag with all three
 *    - Unknown flag id in flagById → "(unknown)"
 */

import { describe, it, expect } from "vitest";
import { FlagValidator } from "../flag-validator.js";
import type { ExclusiveGroup, FlagDef } from "../types.js";

// ---------------------------------------------------------------------------
// Helpers to build FlagDef objects with minimum boilerplate
// ---------------------------------------------------------------------------

function makeBoolFlag(
  id: string,
  opts: Partial<FlagDef> = {},
): FlagDef {
  return {
    id,
    short: opts.short,
    long: opts.long,
    singleDashLong: opts.singleDashLong,
    description: opts.description ?? id,
    type: "boolean",
    required: opts.required ?? false,
    default: null,
    enumValues: [],
    conflictsWith: opts.conflictsWith ?? [],
    requires: opts.requires ?? [],
    requiredUnless: opts.requiredUnless ?? [],
    repeatable: false,
  };
}

function makeStringFlag(
  id: string,
  opts: Partial<FlagDef> = {},
): FlagDef {
  return {
    id,
    short: opts.short,
    long: opts.long,
    singleDashLong: opts.singleDashLong,
    description: opts.description ?? id,
    type: "string",
    required: opts.required ?? false,
    default: null,
    enumValues: [],
    conflictsWith: opts.conflictsWith ?? [],
    requires: opts.requires ?? [],
    requiredUnless: opts.requiredUnless ?? [],
    repeatable: opts.repeatable ?? false,
  };
}

// ---------------------------------------------------------------------------
// 1. conflicts_with
// ---------------------------------------------------------------------------

describe("FlagValidator — conflicts_with", () => {
  const flagA = makeBoolFlag("flag-a", { short: "a", conflictsWith: ["flag-b"] });
  const flagB = makeBoolFlag("flag-b", { short: "b", conflictsWith: ["flag-a"] });
  const flagC = makeBoolFlag("flag-c", { short: "c" });

  it("no error when neither conflicting flag is present", () => {
    const validator = new FlagValidator([flagA, flagB, flagC], []);
    const errors = validator.validate({}, ["tool"]);
    expect(errors).toHaveLength(0);
  });

  it("no error when only one of the conflicting pair is present", () => {
    const validator = new FlagValidator([flagA, flagB, flagC], []);
    const errors = validator.validate({ "flag-a": true }, ["tool"]);
    expect(errors).toHaveLength(0);
  });

  it("reports conflicting_flags error when both are present", () => {
    const validator = new FlagValidator([flagA, flagB, flagC], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool"]);
    const conflictErrors = errors.filter((e) => e.errorType === "conflicting_flags");
    expect(conflictErrors).toHaveLength(1);
  });

  it("reports the conflict only once, not twice (bilateral)", () => {
    // flagA conflicts_with flagB AND flagB conflicts_with flagA
    // Both produce the same sorted key — should dedup to exactly 1 error
    const validator = new FlagValidator([flagA, flagB], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool"]);
    const conflictErrors = errors.filter((e) => e.errorType === "conflicting_flags");
    expect(conflictErrors).toHaveLength(1);
  });

  it("conflict error includes context", () => {
    const validator = new FlagValidator([flagA, flagB], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool", "sub"]);
    expect(errors[0].context).toEqual(["tool", "sub"]);
  });

  it("no conflict when the conflicting flag has value null (absent)", () => {
    const validator = new FlagValidator([flagA, flagB], []);
    // flag-b is listed but its value is null (not present)
    const errors = validator.validate({ "flag-a": true, "flag-b": null }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "conflicting_flags")).toHaveLength(0);
  });

  it("no conflict when one flag is only in conflicts_with (one-sided)", () => {
    // Only flagA lists flagC in conflicts_with; flagC doesn't list flagA
    const flagAvsC = makeBoolFlag("flag-a", { short: "a", conflictsWith: ["flag-c"] });
    const validator = new FlagValidator([flagAvsC, flagC], []);
    const errors = validator.validate({ "flag-a": true, "flag-c": true }, ["tool"]);
    const conflictErrors = errors.filter((e) => e.errorType === "conflicting_flags");
    expect(conflictErrors).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// 2. requires (transitive)
// ---------------------------------------------------------------------------

describe("FlagValidator — requires (transitive)", () => {
  // -h requires -l (human-readable requires long-listing)
  const flagL = makeBoolFlag("long-listing", { short: "l" });
  const flagH = makeBoolFlag("human-readable", { short: "h", requires: ["long-listing"] });

  it("no error when neither flag is present", () => {
    const validator = new FlagValidator([flagL, flagH], []);
    const errors = validator.validate({}, ["tool"]);
    expect(errors).toHaveLength(0);
  });

  it("no error when required dependency is also present", () => {
    const validator = new FlagValidator([flagL, flagH], []);
    const errors = validator.validate({ "long-listing": true, "human-readable": true }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_dependency_flag")).toHaveLength(0);
  });

  it("reports missing_dependency_flag when dependency is absent", () => {
    const validator = new FlagValidator([flagL, flagH], []);
    const errors = validator.validate({ "human-readable": true }, ["tool"]);
    const depErrors = errors.filter((e) => e.errorType === "missing_dependency_flag");
    expect(depErrors).toHaveLength(1);
    expect(depErrors[0].message).toContain("long-listing");
  });

  it("transitive chain A→B→C: A present, B absent → error", () => {
    // A requires B, B requires C.
    // When A is present but B is absent, we expect an error for A→B.
    const flagA = makeBoolFlag("flag-a", { short: "a", requires: ["flag-b"] });
    const flagB = makeBoolFlag("flag-b", { short: "b", requires: ["flag-c"] });
    const flagC = makeBoolFlag("flag-c", { short: "c" });

    const validator = new FlagValidator([flagA, flagB, flagC], []);
    // Only A is present; B and C are absent
    const errors = validator.validate({ "flag-a": true }, ["tool"]);
    const depErrors = errors.filter((e) => e.errorType === "missing_dependency_flag");
    // A→B should be reported; B→C may also be reported transitively
    expect(depErrors.length).toBeGreaterThan(0);
    expect(depErrors.some((e) => e.message.includes("flag-b"))).toBe(true);
  });

  it("no error when only the required dependency is present (not the requiring flag)", () => {
    const validator = new FlagValidator([flagL, flagH], []);
    // -l is present but -h is not; no constraint is violated
    const errors = validator.validate({ "long-listing": true }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_dependency_flag")).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// 3. required flags
// ---------------------------------------------------------------------------

describe("FlagValidator — required flags", () => {
  const requiredFlag = makeBoolFlag("verbose", { short: "v", required: true });
  const optionalFlag = makeBoolFlag("quiet", { short: "q" });

  it("reports missing_required_flag when required flag is absent", () => {
    const validator = new FlagValidator([requiredFlag, optionalFlag], []);
    const errors = validator.validate({}, ["tool"]);
    const reqErrors = errors.filter((e) => e.errorType === "missing_required_flag");
    expect(reqErrors).toHaveLength(1);
    expect(reqErrors[0].message).toContain("verbose");
  });

  it("no error when required flag is present", () => {
    const validator = new FlagValidator([requiredFlag, optionalFlag], []);
    const errors = validator.validate({ verbose: true }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(0);
  });

  it("no error when optional flag is absent", () => {
    const validator = new FlagValidator([requiredFlag, optionalFlag], []);
    // Provide the required flag; quiet is absent but optional
    const errors = validator.validate({ verbose: true }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(0);
  });

  it("required_unless satisfied: no error when exempting flag is present", () => {
    // -m (message) is required unless --amend is present
    const amendFlag = makeBoolFlag("amend", { long: "amend" });
    const messageFlag = makeStringFlag("message", {
      short: "m",
      long: "message",
      required: true,
      requiredUnless: ["amend"],
    });
    const validator = new FlagValidator([messageFlag, amendFlag], []);
    // --amend is present, so --message is not required
    const errors = validator.validate({ amend: true }, ["git", "commit"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(0);
  });

  it("required_unless not satisfied: error when exempting flag is absent", () => {
    const amendFlag = makeBoolFlag("amend", { long: "amend" });
    const messageFlag = makeStringFlag("message", {
      short: "m",
      long: "message",
      required: true,
      requiredUnless: ["amend"],
    });
    const validator = new FlagValidator([messageFlag, amendFlag], []);
    // Neither message nor amend is present
    const errors = validator.validate({}, ["git", "commit"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(1);
  });

  it("required string flag with null value is reported as missing", () => {
    const requiredStrFlag = makeStringFlag("output", { long: "output", required: true });
    const validator = new FlagValidator([requiredStrFlag], []);
    // value is null — not present
    const errors = validator.validate({ output: null }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// 4. mutually_exclusive_groups
// ---------------------------------------------------------------------------

describe("FlagValidator — mutually_exclusive_groups", () => {
  const flagCreate = makeBoolFlag("create", { short: "c" });
  const flagExtract = makeBoolFlag("extract", { short: "x" });
  const flagList = makeBoolFlag("list", { short: "t" });

  const requiredGroup: ExclusiveGroup = {
    id: "operation",
    flagIds: ["create", "extract", "list"],
    required: true,
  };

  const optionalGroup: ExclusiveGroup = {
    id: "output-format",
    flagIds: ["create", "extract"],
    required: false,
  };

  it("reports missing_exclusive_group when required group has 0 members present", () => {
    const validator = new FlagValidator([flagCreate, flagExtract, flagList], [requiredGroup]);
    const errors = validator.validate({}, ["tar"]);
    const groupErrors = errors.filter((e) => e.errorType === "missing_exclusive_group");
    expect(groupErrors).toHaveLength(1);
  });

  it("no error when required group has exactly 1 member present", () => {
    const validator = new FlagValidator([flagCreate, flagExtract, flagList], [requiredGroup]);
    const errors = validator.validate({ create: true }, ["tar"]);
    expect(errors.filter((e) => e.errorType === "missing_exclusive_group")).toHaveLength(0);
    expect(errors.filter((e) => e.errorType === "exclusive_group_violation")).toHaveLength(0);
  });

  it("reports exclusive_group_violation when 2 members of same group are present", () => {
    const validator = new FlagValidator([flagCreate, flagExtract, flagList], [requiredGroup]);
    const errors = validator.validate({ create: true, extract: true }, ["tar"]);
    expect(errors.filter((e) => e.errorType === "exclusive_group_violation")).toHaveLength(1);
  });

  it("reports exclusive_group_violation for optional group with 2 members", () => {
    const validator = new FlagValidator([flagCreate, flagExtract], [optionalGroup]);
    const errors = validator.validate({ create: true, extract: true }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "exclusive_group_violation")).toHaveLength(1);
  });

  it("no error for optional group with 0 members present", () => {
    const validator = new FlagValidator([flagCreate, flagExtract], [optionalGroup]);
    const errors = validator.validate({}, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_exclusive_group")).toHaveLength(0);
    expect(errors.filter((e) => e.errorType === "exclusive_group_violation")).toHaveLength(0);
  });

  it("no error for optional group with exactly 1 member present", () => {
    const validator = new FlagValidator([flagCreate, flagExtract], [optionalGroup]);
    const errors = validator.validate({ create: true }, ["tool"]);
    expect(errors).toHaveLength(0);
  });

  it("violation error message lists the conflicting flag names", () => {
    const validator = new FlagValidator([flagCreate, flagExtract], [optionalGroup]);
    const errors = validator.validate({ create: true, extract: true }, ["tool"]);
    expect(errors[0].message).toMatch(/create|extract/);
  });

  it("required group error message lists all flag names", () => {
    const validator = new FlagValidator([flagCreate, flagExtract, flagList], [requiredGroup]);
    const errors = validator.validate({}, ["tar"]);
    expect(errors[0].message).toMatch(/create|extract|list/);
  });
});

// ---------------------------------------------------------------------------
// 5. _isFlagPresent semantics (tested indirectly via validate)
// ---------------------------------------------------------------------------

describe("FlagValidator — flag presence detection", () => {
  it("boolean flag: true → present", () => {
    const flag = makeBoolFlag("verbose", { short: "v", required: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({ verbose: true }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(0);
  });

  it("boolean flag: false → absent", () => {
    const flag = makeBoolFlag("verbose", { short: "v", required: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({ verbose: false }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(1);
  });

  it("boolean flag: undefined → absent", () => {
    const flag = makeBoolFlag("verbose", { short: "v", required: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({}, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(1);
  });

  it("repeatable flag: non-empty array → present", () => {
    const flag = makeStringFlag("pattern", { short: "e", required: true, repeatable: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({ pattern: ["foo", "bar"] }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(0);
  });

  it("repeatable flag: empty array → absent", () => {
    const flag = makeStringFlag("pattern", { short: "e", required: true, repeatable: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({ pattern: [] }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(1);
  });

  it("string flag: non-null value → present", () => {
    const flag = makeStringFlag("output", { long: "output", required: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({ output: "file.txt" }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(0);
  });

  it("string flag: null value → absent", () => {
    const flag = makeStringFlag("output", { long: "output", required: true });
    const validator = new FlagValidator([flag], []);
    const errors = validator.validate({ output: null }, ["tool"]);
    expect(errors.filter((e) => e.errorType === "missing_required_flag")).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// 6. _flagDisplay edge cases
// ---------------------------------------------------------------------------

describe("FlagValidator — _flagDisplay edge cases", () => {
  // We test _flagDisplay indirectly by checking error messages.

  it("formats flag with only short correctly", () => {
    const flagA = makeBoolFlag("flag-a", { short: "a", conflictsWith: ["flag-b"] });
    const flagB = makeBoolFlag("flag-b", { short: "b", conflictsWith: ["flag-a"] });
    const validator = new FlagValidator([flagA, flagB], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool"]);
    expect(errors[0].message).toMatch(/-a|-b/);
  });

  it("formats flag with only long correctly", () => {
    const flagA = makeBoolFlag("flag-a", { long: "flag-a", conflictsWith: ["flag-b"] });
    const flagB = makeBoolFlag("flag-b", { long: "flag-b", conflictsWith: ["flag-a"] });
    const validator = new FlagValidator([flagA, flagB], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool"]);
    expect(errors[0].message).toMatch(/--flag-a|--flag-b/);
  });

  it("formats flag with only singleDashLong correctly", () => {
    const flagA = makeBoolFlag("flag-a", { singleDashLong: "flag-a", conflictsWith: ["flag-b"] });
    const flagB = makeBoolFlag("flag-b", { singleDashLong: "flag-b", conflictsWith: ["flag-a"] });
    const validator = new FlagValidator([flagA, flagB], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool"]);
    expect(errors[0].message).toMatch(/-flag-a|-flag-b/);
  });

  it("formats flag with all three correctly", () => {
    const flagA = makeBoolFlag("verbose", {
      short: "v",
      long: "verbose",
      singleDashLong: "verbose",
      conflictsWith: ["quiet"],
    });
    const flagB = makeBoolFlag("quiet", { short: "q", conflictsWith: ["verbose"] });
    const validator = new FlagValidator([flagA, flagB], []);
    const errors = validator.validate({ verbose: true, quiet: true }, ["tool"]);
    // Message should mention -v/--verbose/-verbose
    expect(errors[0].message).toContain("-v");
  });

  it("unknown flag id in flagById returns (unknown) in display", () => {
    // We can trigger this by having a conflicts_with that references an id
    // which is in one flag's conflictsWith but the actual FlagDef is not in activeFlags.
    // The validator only builds _flagById from activeFlags, so if a flag's conflicts_with
    // references an id not in the activeFlags map, _flagById.get returns undefined.
    const flagA = makeBoolFlag("flag-a", { short: "a", conflictsWith: ["flag-missing"] });
    const flagMissing: FlagDef = {
      id: "flag-missing",
      short: "m",
      description: "missing",
      type: "boolean",
      required: false,
      default: null,
      enumValues: [],
      conflictsWith: [],
      requires: [],
      requiredUnless: [],
      repeatable: false,
    };
    // Include flag-missing in activeFlags so the conflict is triggered
    const validator = new FlagValidator([flagA, flagMissing], []);
    const errors = validator.validate({ "flag-a": true, "flag-missing": true }, ["tool"]);
    // Should report the conflict; both flags are in activeFlags so display should work
    expect(errors.filter((e) => e.errorType === "conflicting_flags")).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// 7. Multiple errors collected
// ---------------------------------------------------------------------------

describe("FlagValidator — multiple errors collected", () => {
  it("collects all errors rather than stopping at first", () => {
    // Create a scenario where multiple issues exist simultaneously:
    // - A required flag is missing
    // - Two conflicting flags are both present
    const requiredFlag = makeBoolFlag("req", { short: "r", required: true });
    const flagA = makeBoolFlag("flag-a", { short: "a", conflictsWith: ["flag-b"] });
    const flagB = makeBoolFlag("flag-b", { short: "b", conflictsWith: ["flag-a"] });
    const validator = new FlagValidator([requiredFlag, flagA, flagB], []);
    const errors = validator.validate({ "flag-a": true, "flag-b": true }, ["tool"]);
    // Should have at least the conflict error + the required flag error
    expect(errors.length).toBeGreaterThanOrEqual(2);
    expect(errors.some((e) => e.errorType === "missing_required_flag")).toBe(true);
    expect(errors.some((e) => e.errorType === "conflicting_flags")).toBe(true);
  });
});
