/**
 * token-classifier.test.ts — Tests for TokenClassifier.
 *
 * Tests cover all token types and edge cases:
 * 1. END_OF_FLAGS for "--"
 * 2. LONG_FLAG and LONG_FLAG_WITH_VALUE
 * 3. SHORT_FLAG and SHORT_FLAG_WITH_VALUE
 * 4. STACKED_FLAGS (all boolean, mixed with value at end)
 * 5. SINGLE_DASH_LONG (longest-match-first: -classpath vs -c)
 * 6. POSITIONAL (plain values, "-" for stdin)
 * 7. UNKNOWN_FLAG
 */

import { describe, it, expect } from "vitest";
import { TokenClassifier } from "../token-classifier.js";
import type { FlagDef } from "../types.js";

// ---------------------------------------------------------------------------
// Test fixtures: flag sets for various tool configurations
// ---------------------------------------------------------------------------

/** ls-style flags: -l -a -h -r -t -R -1 with --long for some */
const LS_FLAGS: FlagDef[] = [
  {
    id: "long-listing",
    short: "l",
    description: "Use long listing format",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "all",
    short: "a",
    long: "all",
    description: "Do not ignore entries starting with .",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "human-readable",
    short: "h",
    long: "human-readable",
    description: "Print sizes",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "reverse",
    short: "r",
    long: "reverse",
    description: "Reverse order",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "sort-time",
    short: "t",
    description: "Sort by time",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
];

/** Java-style single_dash_long flags */
const JAVA_FLAGS: FlagDef[] = [
  {
    id: "classpath",
    singleDashLong: "classpath",
    description: "Classpath",
    type: "string",
    required: false,
    default: null,
    valueName: "classpath",
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "classpath-short",
    singleDashLong: "cp",
    description: "Alias for -classpath",
    type: "string",
    required: false,
    default: null,
    valueName: "classpath",
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "verbose",
    singleDashLong: "verbose",
    description: "Verbose",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
];

/** head-style flags with integer values */
const HEAD_FLAGS: FlagDef[] = [
  {
    id: "lines",
    short: "n",
    long: "lines",
    description: "Number of lines",
    type: "integer",
    required: false,
    default: 10,
    valueName: "NUM",
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
  {
    id: "quiet",
    short: "q",
    long: "quiet",
    description: "Quiet",
    type: "boolean",
    required: false,
    default: null,
    enumValues: [],
    conflictsWith: [],
    requires: [],
    requiredUnless: [],
    repeatable: false,
  },
];

// ---------------------------------------------------------------------------
// Tests: basic token types
// ---------------------------------------------------------------------------

describe("TokenClassifier — basic token types", () => {
  it("classifies '--' as END_OF_FLAGS", () => {
    const c = new TokenClassifier([]);
    expect(c.classify("--")).toEqual({ type: "END_OF_FLAGS" });
  });

  it("classifies '--name' as LONG_FLAG", () => {
    const c = new TokenClassifier(LS_FLAGS);
    expect(c.classify("--reverse")).toEqual({ type: "LONG_FLAG", name: "reverse" });
  });

  it("classifies '--name=value' as LONG_FLAG_WITH_VALUE", () => {
    const c = new TokenClassifier(HEAD_FLAGS);
    expect(c.classify("--lines=20")).toEqual({
      type: "LONG_FLAG_WITH_VALUE",
      name: "lines",
      value: "20",
    });
  });

  it("classifies '--name=value with equals in value' correctly", () => {
    const c = new TokenClassifier([]);
    expect(c.classify("--env=KEY=VALUE")).toEqual({
      type: "LONG_FLAG_WITH_VALUE",
      name: "env",
      value: "KEY=VALUE",
    });
  });

  it("classifies '-' as POSITIONAL (stdin sentinel)", () => {
    const c = new TokenClassifier(LS_FLAGS);
    expect(c.classify("-")).toEqual({ type: "POSITIONAL", value: "-" });
  });

  it("classifies a plain word as POSITIONAL", () => {
    const c = new TokenClassifier(LS_FLAGS);
    expect(c.classify("hello")).toEqual({ type: "POSITIONAL", value: "hello" });
  });

  it("classifies a path as POSITIONAL", () => {
    const c = new TokenClassifier(LS_FLAGS);
    expect(c.classify("/tmp/foo.txt")).toEqual({
      type: "POSITIONAL",
      value: "/tmp/foo.txt",
    });
  });
});

// ---------------------------------------------------------------------------
// Tests: short flags
// ---------------------------------------------------------------------------

describe("TokenClassifier — short flags", () => {
  it("classifies '-l' as SHORT_FLAG", () => {
    const c = new TokenClassifier(LS_FLAGS);
    expect(c.classify("-l")).toEqual({ type: "SHORT_FLAG", char: "l" });
  });

  it("classifies '-n' as SHORT_FLAG (non-boolean, value follows)", () => {
    const c = new TokenClassifier(HEAD_FLAGS);
    expect(c.classify("-n")).toEqual({ type: "SHORT_FLAG", char: "n" });
  });

  it("classifies '-n20' as SHORT_FLAG_WITH_VALUE", () => {
    const c = new TokenClassifier(HEAD_FLAGS);
    expect(c.classify("-n20")).toEqual({
      type: "SHORT_FLAG_WITH_VALUE",
      char: "n",
      value: "20",
    });
  });

  it("classifies '-n5' as SHORT_FLAG_WITH_VALUE", () => {
    const c = new TokenClassifier(HEAD_FLAGS);
    expect(c.classify("-n5")).toEqual({
      type: "SHORT_FLAG_WITH_VALUE",
      char: "n",
      value: "5",
    });
  });
});

// ---------------------------------------------------------------------------
// Tests: stacked flags
// ---------------------------------------------------------------------------

describe("TokenClassifier — stacked flags", () => {
  it("classifies '-lah' as STACKED_FLAGS", () => {
    const c = new TokenClassifier(LS_FLAGS);
    const result = c.classify("-lah");
    expect(result.type).toBe("STACKED_FLAGS");
    if (result.type === "STACKED_FLAGS") {
      expect(result.chars).toContain("l");
      expect(result.chars).toContain("a");
      expect(result.chars).toContain("h");
    }
  });

  it("classifies '-la' as STACKED_FLAGS", () => {
    const c = new TokenClassifier(LS_FLAGS);
    const result = c.classify("-la");
    expect(result.type).toBe("STACKED_FLAGS");
  });

  it("classifies '-lrt' as STACKED_FLAGS", () => {
    const c = new TokenClassifier(LS_FLAGS);
    const result = c.classify("-lrt");
    expect(result.type).toBe("STACKED_FLAGS");
    if (result.type === "STACKED_FLAGS") {
      expect(result.chars).toEqual(expect.arrayContaining(["l", "r", "t"]));
    }
  });

  it("classifies '-qn' where q is boolean and n is non-boolean", () => {
    // q = boolean, n = non-boolean (integer, takes a value)
    const c = new TokenClassifier(HEAD_FLAGS);
    const result = c.classify("-qn");
    // q is boolean, then n is non-boolean (value from next token)
    expect(result.type).toBe("STACKED_FLAGS");
    if (result.type === "STACKED_FLAGS") {
      expect(result.chars).toContain("q");
      expect(result.chars).toContain("n");
    }
  });
});

// ---------------------------------------------------------------------------
// Tests: single_dash_long (longest-match-first)
// ---------------------------------------------------------------------------

describe("TokenClassifier — single_dash_long / longest-match-first", () => {
  it("classifies '-classpath' as SINGLE_DASH_LONG, not stacked chars", () => {
    const c = new TokenClassifier(JAVA_FLAGS);
    expect(c.classify("-classpath")).toEqual({
      type: "SINGLE_DASH_LONG",
      name: "classpath",
    });
  });

  it("classifies '-cp' as SINGLE_DASH_LONG", () => {
    const c = new TokenClassifier(JAVA_FLAGS);
    expect(c.classify("-cp")).toEqual({
      type: "SINGLE_DASH_LONG",
      name: "cp",
    });
  });

  it("classifies '-verbose' as SINGLE_DASH_LONG (boolean)", () => {
    const c = new TokenClassifier(JAVA_FLAGS);
    expect(c.classify("-verbose")).toEqual({
      type: "SINGLE_DASH_LONG",
      name: "verbose",
    });
  });

  it("prioritizes single_dash_long over short flag stacking", () => {
    // If we had both a short 'c' flag AND 'classpath' single_dash_long,
    // '-classpath' must match the SDL rule first.
    const mixedFlags: FlagDef[] = [
      ...JAVA_FLAGS,
      {
        id: "create",
        short: "c",
        description: "create",
        type: "boolean",
        required: false,
        default: null,
        enumValues: [],
        conflictsWith: [],
        requires: [],
        requiredUnless: [],
        repeatable: false,
      },
    ];
    const c = new TokenClassifier(mixedFlags);
    // -classpath should still match SDL rule, not be treated as -c + "lasspath"
    expect(c.classify("-classpath")).toEqual({
      type: "SINGLE_DASH_LONG",
      name: "classpath",
    });
  });
});

// ---------------------------------------------------------------------------
// Tests: unknown flags
// ---------------------------------------------------------------------------

describe("TokenClassifier — unknown flags", () => {
  it("classifies '-z' as UNKNOWN_FLAG when z is not in scope", () => {
    const c = new TokenClassifier(LS_FLAGS);
    // 'z' is not in ls flags
    const result = c.classify("-z");
    expect(result.type).toBe("UNKNOWN_FLAG");
  });

  it("classifies '-xyz' with unknown x as UNKNOWN_FLAG", () => {
    const c = new TokenClassifier(LS_FLAGS);
    // 'x' is not in ls flags
    const result = c.classify("-xyz");
    expect(result.type).toBe("UNKNOWN_FLAG");
  });

  it("produces LONG_FLAG for unknown long flags (validation happens later)", () => {
    // The classifier doesn't validate long flags, it just classifies structure
    const c = new TokenClassifier(LS_FLAGS);
    // Even if "--nonexistent" isn't a declared flag, classifier returns LONG_FLAG
    // The parser/validator will catch the unknown flag error
    expect(c.classify("--nonexistent")).toEqual({
      type: "LONG_FLAG",
      name: "nonexistent",
    });
  });
});

// ---------------------------------------------------------------------------
// Tests: lookup methods
// ---------------------------------------------------------------------------

describe("TokenClassifier — lookup methods", () => {
  it("lookupByLong returns the correct flag", () => {
    const c = new TokenClassifier(LS_FLAGS);
    const flag = c.lookupByLong("reverse");
    expect(flag?.id).toBe("reverse");
  });

  it("lookupByLong returns undefined for unknown", () => {
    const c = new TokenClassifier(LS_FLAGS);
    expect(c.lookupByLong("nonexistent")).toBeUndefined();
  });

  it("lookupByShort returns the correct flag", () => {
    const c = new TokenClassifier(LS_FLAGS);
    const flag = c.lookupByShort("l");
    expect(flag?.id).toBe("long-listing");
  });

  it("lookupBySingleDashLong returns the correct flag", () => {
    const c = new TokenClassifier(JAVA_FLAGS);
    const flag = c.lookupBySingleDashLong("classpath");
    expect(flag?.id).toBe("classpath");
  });

  it("lookupBySingleDashLong returns undefined for unknown", () => {
    const c = new TokenClassifier(JAVA_FLAGS);
    expect(c.lookupBySingleDashLong("nonexistent")).toBeUndefined();
  });
});
