/**
 * v1.1-features.test.ts — Tests for CLI Builder v1.1 features.
 *
 * This file covers all four backwards-compatible features added in v1.1:
 *
 * 1. **Count type** — `-vvv` = 3, `--verbose --verbose` = 2
 * 2. **Enum optional values (default_when_present)** — `--color` uses default
 * 3. **Flag presence detection (explicitFlags)** — track which flags the user set
 * 4. **int64 range validation** — reject integers outside Number.MIN/MAX_SAFE_INTEGER
 */

import { describe, it, expect } from "vitest";
import { Parser } from "../parser.js";
import { SpecLoader } from "../spec-loader.js";
import { ParseErrors, SpecError } from "../errors.js";
import { coerceValue } from "../positional-resolver.js";
import { HelpGenerator } from "../help-generator.js";
import { TokenClassifier } from "../token-classifier.js";
import type { CliSpec, ParseResult, FlagDef } from "../types.js";

// ---------------------------------------------------------------------------
// Helper to build a Parser from a raw spec object
// ---------------------------------------------------------------------------

function makeParser(rawSpec: Record<string, unknown>, argv: string[]): Parser {
  const loader = new SpecLoader("(test)");
  const spec = loader.loadFromObject(rawSpec);
  return new Parser(spec, argv);
}

function parseAs(
  rawSpec: Record<string, unknown>,
  argv: string[],
): ParseResult {
  const result = makeParser(rawSpec, argv).parse();
  if ("text" in result || ("version" in result && !("flags" in result))) {
    throw new Error("Expected ParseResult but got HelpResult or VersionResult");
  }
  return result as ParseResult;
}

// ---------------------------------------------------------------------------
// Spec fixtures for v1.1 features
// ---------------------------------------------------------------------------

/**
 * A tool with a count flag for verbosity.
 * `-v` = 1, `-vv` = 2, `-vvv` = 3, `--verbose --verbose` = 2.
 */
const COUNT_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "tool",
  description: "A tool with count flags",
  flags: [
    {
      id: "verbose",
      short: "v",
      long: "verbose",
      description: "Increase verbosity",
      type: "count",
    },
    {
      id: "quiet",
      short: "q",
      long: "quiet",
      description: "Decrease output",
      type: "boolean",
    },
  ],
  arguments: [
    {
      id: "file",
      name: "FILE",
      description: "Input file",
      type: "path",
      required: false,
      variadic: true,
      variadic_min: 0,
    },
  ],
};

/**
 * A tool with an enum flag that has default_when_present.
 * `--color` = "always", `--color=never` = "never", `--color auto` = "auto".
 */
const ENUM_DEFAULT_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "ls",
  description: "List files",
  flags: [
    {
      id: "color",
      long: "color",
      description: "Colorize output",
      type: "enum",
      enum_values: ["always", "never", "auto"],
      default: "auto",
      default_when_present: "always",
    },
    {
      id: "all",
      short: "a",
      long: "all",
      description: "Show all files",
      type: "boolean",
    },
  ],
  arguments: [
    {
      id: "path",
      name: "PATH",
      description: "Directory to list",
      type: "path",
      required: false,
      variadic: true,
      variadic_min: 0,
    },
  ],
};

/**
 * A tool with an integer flag for testing int64 range validation.
 */
const INT_RANGE_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "tool",
  description: "Tool with integer flags",
  flags: [
    {
      id: "id",
      long: "id",
      description: "Resource ID",
      type: "integer",
    },
    {
      id: "count",
      short: "n",
      long: "count",
      description: "Number of items",
      type: "integer",
    },
  ],
};

// =========================================================================
// Feature 1: Count Type
// =========================================================================

describe("v1.1 Feature 1 — Count type", () => {
  it("-v → verbose = 1", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-v"]);
    expect(result.flags["verbose"]).toBe(1);
  });

  it("-vv → verbose = 2 (stacked)", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-vv"]);
    expect(result.flags["verbose"]).toBe(2);
  });

  it("-vvv → verbose = 3 (stacked)", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-vvv"]);
    expect(result.flags["verbose"]).toBe(3);
  });

  it("--verbose --verbose → verbose = 2 (long flag repeated)", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "--verbose", "--verbose"]);
    expect(result.flags["verbose"]).toBe(2);
  });

  it("-v --verbose → verbose = 2 (mixed short and long)", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-v", "--verbose"]);
    expect(result.flags["verbose"]).toBe(2);
  });

  it("no -v → verbose = 0 (default)", () => {
    const result = parseAs(COUNT_SPEC, ["tool"]);
    expect(result.flags["verbose"]).toBe(0);
  });

  it("-vvq → verbose = 2, quiet = true (mixed with boolean in stack)", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-vvq"]);
    expect(result.flags["verbose"]).toBe(2);
    expect(result.flags["quiet"]).toBe(true);
  });

  it("-qvv → quiet = true, verbose = 2 (boolean before counts in stack)", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-qvv"]);
    expect(result.flags["quiet"]).toBe(true);
    expect(result.flags["verbose"]).toBe(2);
  });

  it("count flags don't produce duplicate_flag errors", () => {
    // Unlike non-repeatable flags, count flags can appear multiple times
    expect(() =>
      parseAs(COUNT_SPEC, ["tool", "--verbose", "--verbose", "--verbose"]),
    ).not.toThrow();
    const result = parseAs(COUNT_SPEC, [
      "tool",
      "--verbose",
      "--verbose",
      "--verbose",
    ]);
    expect(result.flags["verbose"]).toBe(3);
  });

  it("count flag with positional arguments", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-vv", "file.txt"]);
    expect(result.flags["verbose"]).toBe(2);
    expect(result.arguments["file"]).toEqual(["file.txt"]);
  });

  it("count flag does not consume a value token", () => {
    // Make sure -v doesn't consume the next token as its value
    const result = parseAs(COUNT_SPEC, ["tool", "-v", "file.txt"]);
    expect(result.flags["verbose"]).toBe(1);
    expect(result.arguments["file"]).toEqual(["file.txt"]);
  });
});

// =========================================================================
// Feature 2: Enum Optional Values (default_when_present)
// =========================================================================

describe("v1.1 Feature 2 — Enum optional values (default_when_present)", () => {
  it("--color (no value) → uses default_when_present ('always')", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color"]);
    expect(result.flags["color"]).toBe("always");
  });

  it("--color=never → explicit value via equals", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color=never"]);
    expect(result.flags["color"]).toBe("never");
  });

  it("--color=always → explicit value via equals", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color=always"]);
    expect(result.flags["color"]).toBe("always");
  });

  it("--color auto → next token is valid enum value, consumed", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color", "auto"]);
    expect(result.flags["color"]).toBe("auto");
  });

  it("--color never → next token is valid enum value, consumed", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color", "never"]);
    expect(result.flags["color"]).toBe("never");
  });

  it("--color /tmp → next token is NOT a valid enum value, uses default_when_present", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color", "/tmp"]);
    expect(result.flags["color"]).toBe("always");
    // "/tmp" should be treated as a positional argument
    expect(result.arguments["path"]).toEqual(["/tmp"]);
  });

  it("--color -a → next token starts with dash, uses default_when_present", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color", "-a"]);
    expect(result.flags["color"]).toBe("always");
    expect(result.flags["all"]).toBe(true);
  });

  it("no --color → uses spec default ('auto')", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls"]);
    expect(result.flags["color"]).toBe("auto");
  });

  it("--color=invalid → invalid_enum_value error", () => {
    expect(() =>
      parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color=invalid"]),
    ).toThrow(ParseErrors);
    try {
      parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color=invalid"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "invalid_enum_value"),
      ).toBe(true);
    }
  });

  it("--color at end of argv → uses default_when_present", () => {
    // When --color is the very last token and there's no next token
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "/tmp", "--color"]);
    expect(result.flags["color"]).toBe("always");
    expect(result.arguments["path"]).toEqual(["/tmp"]);
  });

  it("spec validation: default_when_present on non-enum type is rejected", () => {
    const badSpec = {
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Bad tool",
      flags: [
        {
          id: "level",
          long: "level",
          description: "Level",
          type: "integer",
          default_when_present: "5",
        },
      ],
    };
    expect(() => {
      const loader = new SpecLoader("(test)");
      loader.loadFromObject(badSpec);
    }).toThrow(SpecError);
  });

  it("spec validation: default_when_present value not in enum_values is rejected", () => {
    const badSpec = {
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Bad tool",
      flags: [
        {
          id: "color",
          long: "color",
          description: "Color",
          type: "enum",
          enum_values: ["always", "never", "auto"],
          default_when_present: "sometimes",
        },
      ],
    };
    expect(() => {
      const loader = new SpecLoader("(test)");
      loader.loadFromObject(badSpec);
    }).toThrow(SpecError);
  });
});

// =========================================================================
// Feature 3: Flag Presence Detection (explicitFlags)
// =========================================================================

describe("v1.1 Feature 3 — Flag presence detection (explicitFlags)", () => {
  it("no flags → explicitFlags is empty", () => {
    const result = parseAs(COUNT_SPEC, ["tool"]);
    expect(result.explicitFlags).toEqual([]);
  });

  it("one flag → explicitFlags contains that flag ID", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-v"]);
    expect(result.explicitFlags).toContain("verbose");
  });

  it("-vvv → explicitFlags has 'verbose' three times", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-vvv"]);
    expect(result.explicitFlags).toEqual(["verbose", "verbose", "verbose"]);
  });

  it("--verbose --verbose → explicitFlags has 'verbose' twice", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "--verbose", "--verbose"]);
    expect(result.explicitFlags).toEqual(["verbose", "verbose"]);
  });

  it("-v -q → explicitFlags has both flag IDs", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-v", "-q"]);
    expect(result.explicitFlags).toContain("verbose");
    expect(result.explicitFlags).toContain("quiet");
    expect(result.explicitFlags.length).toBe(2);
  });

  it("stacked -vq → explicitFlags has both in order", () => {
    const result = parseAs(COUNT_SPEC, ["tool", "-vq"]);
    expect(result.explicitFlags).toEqual(["verbose", "quiet"]);
  });

  it("enum flag with value → tracked in explicitFlags", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color=always"]);
    expect(result.explicitFlags).toContain("color");
  });

  it("enum flag with default_when_present → tracked in explicitFlags", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls", "--color"]);
    expect(result.explicitFlags).toContain("color");
  });

  it("integer flag with value → tracked in explicitFlags", () => {
    const result = parseAs(INT_RANGE_SPEC, ["tool", "--count", "5"]);
    expect(result.explicitFlags).toContain("count");
  });

  it("flags set by defaults are NOT in explicitFlags", () => {
    const result = parseAs(ENUM_DEFAULT_SPEC, ["ls"]);
    // color has a default of "auto" but was not explicitly set
    expect(result.explicitFlags).not.toContain("color");
    expect(result.flags["color"]).toBe("auto");
  });

  it("flags with short form and long form are both tracked by ID", () => {
    // Use short form
    const result1 = parseAs(COUNT_SPEC, ["tool", "-q"]);
    expect(result1.explicitFlags).toContain("quiet");

    // Use long form
    const result2 = parseAs(COUNT_SPEC, ["tool", "--quiet"]);
    expect(result2.explicitFlags).toContain("quiet");
  });
});

// =========================================================================
// Feature 4: int64 Range Validation
// =========================================================================

describe("v1.1 Feature 4 — int64 range validation", () => {
  it("normal integers parse correctly", () => {
    const result = parseAs(INT_RANGE_SPEC, ["tool", "--id", "42"]);
    expect(result.flags["id"]).toBe(42);
  });

  it("negative integers parse correctly", () => {
    const result = parseAs(INT_RANGE_SPEC, ["tool", "--id", "-1"]);
    expect(result.flags["id"]).toBe(-1);
  });

  it("zero parses correctly", () => {
    const result = parseAs(INT_RANGE_SPEC, ["tool", "--id", "0"]);
    expect(result.flags["id"]).toBe(0);
  });

  it("MAX_SAFE_INTEGER parses correctly", () => {
    const result = parseAs(INT_RANGE_SPEC, [
      "tool",
      "--id",
      String(Number.MAX_SAFE_INTEGER),
    ]);
    expect(result.flags["id"]).toBe(Number.MAX_SAFE_INTEGER);
  });

  it("MIN_SAFE_INTEGER parses correctly", () => {
    const result = parseAs(INT_RANGE_SPEC, [
      "tool",
      "--id",
      String(Number.MIN_SAFE_INTEGER),
    ]);
    expect(result.flags["id"]).toBe(Number.MIN_SAFE_INTEGER);
  });

  it("value above MAX_SAFE_INTEGER → invalid_value error", () => {
    // 2^53 = 9007199254740992, which is MAX_SAFE_INTEGER + 1
    const tooBig = "9007199254740992";
    expect(() => parseAs(INT_RANGE_SPEC, ["tool", "--id", tooBig])).toThrow(
      ParseErrors,
    );
    try {
      parseAs(INT_RANGE_SPEC, ["tool", "--id", tooBig]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "invalid_value"),
      ).toBe(true);
      expect(
        pe.errors.some((err) => err.message.includes("safe range")),
      ).toBe(true);
    }
  });

  it("value below MIN_SAFE_INTEGER → invalid_value error", () => {
    const tooSmall = "-9007199254740992";
    expect(() =>
      parseAs(INT_RANGE_SPEC, ["tool", "--id", tooSmall]),
    ).toThrow(ParseErrors);
    try {
      parseAs(INT_RANGE_SPEC, ["tool", "--id", tooSmall]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "invalid_value"),
      ).toBe(true);
    }
  });

  it("very large integer → invalid_value error", () => {
    const huge = "99999999999999999999";
    expect(() =>
      parseAs(INT_RANGE_SPEC, ["tool", "--id", huge]),
    ).toThrow(ParseErrors);
  });

  it("coerceValue directly — safe integer passes", () => {
    const result = coerceValue("100", "integer", "test", ["tool"]);
    expect(result).toEqual({ value: 100 });
  });

  it("coerceValue directly — unsafe integer fails", () => {
    const result = coerceValue(
      "9007199254740992",
      "integer",
      "test",
      ["tool"],
    );
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
    }
  });

  it("integer argument (positional) also validates range", () => {
    const spec = {
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool with integer arg",
      arguments: [
        {
          id: "count",
          name: "COUNT",
          description: "Number of items",
          type: "integer",
          required: true,
        },
      ],
    };
    expect(() =>
      parseAs(spec, ["tool", "9007199254740992"]),
    ).toThrow(ParseErrors);
  });
});

// =========================================================================
// Token classifier tests for count type
// =========================================================================

describe("v1.1 TokenClassifier — count type in stacks", () => {
  const countFlags: FlagDef[] = [
    {
      id: "verbose",
      short: "v",
      description: "Verbose",
      type: "count",
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
      description: "All",
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

  it("-vvv classifies as STACKED_FLAGS with three 'v' chars", () => {
    const classifier = new TokenClassifier(countFlags);
    const event = classifier.classify("-vvv");
    expect(event.type).toBe("STACKED_FLAGS");
    if (event.type === "STACKED_FLAGS") {
      expect(event.chars).toEqual(["v", "v", "v"]);
    }
  });

  it("-vav classifies as STACKED_FLAGS with ['v','a','v']", () => {
    const classifier = new TokenClassifier(countFlags);
    const event = classifier.classify("-vav");
    expect(event.type).toBe("STACKED_FLAGS");
    if (event.type === "STACKED_FLAGS") {
      expect(event.chars).toEqual(["v", "a", "v"]);
    }
  });

  it("-v classifies as SHORT_FLAG", () => {
    const classifier = new TokenClassifier(countFlags);
    const event = classifier.classify("-v");
    expect(event.type).toBe("SHORT_FLAG");
  });
});

// =========================================================================
// Help generator tests for v1.1 features
// =========================================================================

describe("v1.1 HelpGenerator — count and default_when_present", () => {
  it("count flag shows like boolean in help (no value placeholder)", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(COUNT_SPEC);
    const gen = new HelpGenerator(spec, []);
    const text = gen.generate();
    // Count flag should appear as "-v, --verbose" (no <VALUE>)
    expect(text).toContain("-v, --verbose");
    expect(text).not.toContain("--verbose <COUNT>");
  });

  it("enum flag with default_when_present shows [=VALUE] in help", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ENUM_DEFAULT_SPEC);
    const gen = new HelpGenerator(spec, []);
    const text = gen.generate();
    // Should show optional value syntax
    expect(text).toContain("[=");
  });
});

// =========================================================================
// Integration: all features working together
// =========================================================================

describe("v1.1 Integration — all features together", () => {
  const FULL_SPEC = {
    cli_builder_spec_version: "1.0",
    name: "tool",
    description: "A tool demonstrating all v1.1 features",
    version: "1.1.0",
    flags: [
      {
        id: "verbose",
        short: "v",
        long: "verbose",
        description: "Increase verbosity",
        type: "count",
      },
      {
        id: "color",
        long: "color",
        description: "Colorize output",
        type: "enum",
        enum_values: ["always", "never", "auto"],
        default: "auto",
        default_when_present: "always",
      },
      {
        id: "limit",
        short: "n",
        long: "limit",
        description: "Result limit",
        type: "integer",
      },
    ],
    arguments: [
      {
        id: "query",
        name: "QUERY",
        description: "Search query",
        type: "string",
        required: false,
      },
    ],
  };

  it("tool -vvv --color --limit 10 query → all features exercised", () => {
    const result = parseAs(FULL_SPEC, [
      "tool",
      "-vvv",
      "--color",
      "--limit",
      "10",
      "search term",
    ]);
    // Count: 3 verbosity
    expect(result.flags["verbose"]).toBe(3);
    // Default when present: color uses "always"
    expect(result.flags["color"]).toBe("always");
    // Integer: limit is 10
    expect(result.flags["limit"]).toBe(10);
    // Query
    expect(result.arguments["query"]).toBe("search term");
    // Explicit flags: verbose x3, color, limit
    expect(result.explicitFlags).toContain("verbose");
    expect(result.explicitFlags).toContain("color");
    expect(result.explicitFlags).toContain("limit");
    expect(
      result.explicitFlags.filter((f) => f === "verbose").length,
    ).toBe(3);
  });

  it("tool --color=never -n 5 → explicit enum value + integer", () => {
    const result = parseAs(FULL_SPEC, [
      "tool",
      "--color=never",
      "-n",
      "5",
    ]);
    expect(result.flags["color"]).toBe("never");
    expect(result.flags["limit"]).toBe(5);
    expect(result.flags["verbose"]).toBe(0); // default
    expect(result.explicitFlags).toContain("color");
    expect(result.explicitFlags).toContain("limit");
    expect(result.explicitFlags).not.toContain("verbose");
  });
});
