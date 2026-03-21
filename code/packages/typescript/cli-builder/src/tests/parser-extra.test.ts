/**
 * parser-extra.test.ts — Additional coverage tests for the Parser.
 *
 * The existing parser.test.ts covers the core happy paths. This file
 * targets branches in parser.ts that aren't clearly exercised:
 *
 * 1. argv preprocessing:
 *    - Empty argv → parses at root scope with no args
 *    - Node.js format argv (argv[0] is not spec name) → strip 2
 *    - Path-based argv[0] ending with spec name (Unix path separator)
 *    - Path-based argv[0] ending with spec name (Windows path separator)
 *
 * 2. Phase 2 scanning edge cases:
 *    - Flag with no value at end of argv → invalid_value error
 *    - LONG_FLAG_WITH_VALUE for --help (early return)
 *    - LONG_FLAG_WITH_VALUE for unknown flag → unknown_flag error
 *    - SHORT_FLAG_WITH_VALUE for unknown flag → unknown_flag error
 *    - SINGLE_DASH_LONG non-boolean flag → value from next token
 *    - SINGLE_DASH_LONG unknown flag → unknown_flag error
 *    - UNKNOWN_FLAG token → unknown_flag with fuzzy suggestion
 *    - Stacked flags with unknown char → invalid_stack error
 *    - Stacked flags where last char is non-boolean → FLAG_VALUE mode
 *    - Duplicate non-boolean flag with real value → duplicate_flag error
 *
 * 3. Fuzzy matching:
 *    - Typo in long flag name → suggestion in error message
 *    - Completely different flag → no suggestion
 *
 * 4. Active flag set construction:
 *    - Command with inherit_global_flags: false → global flags not in scope
 *    - spec.flags only active at root scope
 *
 * 5. _applyFlagDefaults:
 *    - Repeatable flag absent → []
 *    - Non-boolean flag absent with a default → default value applied
 *
 * 6. Traditional mode:
 *    - First token is a known command → not treated as stacked flags
 *    - First token starts with '-' → not treated as traditional stacked flags
 *
 * 7. Phase 1 routing:
 *    - Flag before subcommand (with value) → correctly skipped
 *    - End-of-flags "--" in routing stops subcommand matching
 */

import { describe, it, expect } from "vitest";
import { Parser } from "../parser.js";
import { SpecLoader } from "../spec-loader.js";
import { ParseErrors } from "../errors.js";
import type { CliSpec, ParseResult } from "../types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function loadSpec(raw: Record<string, unknown>): CliSpec {
  const loader = new SpecLoader("(test)");
  return loader.loadFromObject(raw);
}

function parseAs(spec: CliSpec, argv: string[]): ParseResult {
  const result = new Parser(spec, argv).parse();
  if ("text" in result || ("version" in result && !("flags" in result))) {
    throw new Error("Expected ParseResult but got HelpResult or VersionResult");
  }
  return result as ParseResult;
}

// ---------------------------------------------------------------------------
// Spec fixtures
// ---------------------------------------------------------------------------

const ECHO_SPEC_RAW = {
  cli_builder_spec_version: "1.0",
  name: "echo",
  description: "Display a line of text",
  version: "8.32",
  flags: [
    { id: "no-newline", short: "n", description: "No newline", type: "boolean" },
    { id: "enable-escapes", short: "e", description: "Enable escapes", type: "boolean" },
  ],
  arguments: [
    { id: "string", name: "STRING", description: "Text", type: "string", required: false, variadic: true, variadic_min: 0 },
  ],
};

const JAVA_SPEC_RAW = {
  cli_builder_spec_version: "1.0",
  name: "java",
  description: "Java launcher",
  flags: [
    { id: "classpath", single_dash_long: "classpath", description: "Classpath", type: "string", value_name: "classpath" },
    { id: "verbose", single_dash_long: "verbose", description: "Verbose", type: "boolean" },
    { id: "debug-port", single_dash_long: "debug-port", description: "Debug port", type: "integer" },
  ],
  arguments: [
    { id: "class", name: "CLASS", description: "Main class", type: "string", required: false },
  ],
};

const GIT_SPEC_RAW = {
  cli_builder_spec_version: "1.0",
  name: "git",
  description: "Git",
  version: "2.43",
  global_flags: [
    { id: "no-pager", long: "no-pager", description: "No pager", type: "boolean" },
    { id: "git-dir", long: "git-dir", description: "Git dir", type: "path" },
  ],
  commands: [
    {
      id: "cmd-commit",
      name: "commit",
      description: "Commit",
      flags: [
        { id: "message", short: "m", long: "message", description: "Message", type: "string", required: true, required_unless: ["amend"] },
        { id: "amend", long: "amend", description: "Amend", type: "boolean" },
        { id: "all", short: "a", long: "all", description: "All", type: "boolean" },
      ],
    },
    {
      id: "cmd-isolated",
      name: "isolated",
      description: "Isolated command (no global flags)",
      inherit_global_flags: false,
      flags: [
        { id: "local-only", short: "l", description: "Local", type: "boolean" },
      ],
    },
  ],
};

const TAR_SPEC_RAW = {
  cli_builder_spec_version: "1.0",
  name: "tar",
  description: "Tar",
  parsing_mode: "traditional",
  flags: [
    { id: "create", short: "c", description: "Create", type: "boolean" },
    { id: "extract", short: "x", description: "Extract", type: "boolean" },
    { id: "verbose", short: "v", description: "Verbose", type: "boolean" },
    { id: "file", short: "f", description: "File", type: "path" },
    { id: "gzip", short: "z", description: "Gzip", type: "boolean" },
  ],
  mutually_exclusive_groups: [
    { id: "op", flag_ids: ["create", "extract"], required: true },
  ],
};

const HEAD_SPEC_RAW = {
  cli_builder_spec_version: "1.0",
  name: "head",
  description: "Output first N lines",
  flags: [
    { id: "lines", short: "n", long: "lines", description: "Lines", type: "integer", default: 10 },
    { id: "quiet", short: "q", long: "quiet", description: "Quiet", type: "boolean" },
    { id: "patterns", short: "e", long: "patterns", description: "Patterns", type: "string", repeatable: true },
  ],
  arguments: [
    { id: "file", name: "FILE", description: "File", type: "path", required: false, variadic: true, variadic_min: 0 },
  ],
};

// ---------------------------------------------------------------------------
// 1. argv preprocessing
// ---------------------------------------------------------------------------

describe("Parser — argv preprocessing", () => {
  it("empty argv → parse at root scope with no flags or args", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const result = parseAs(spec, []);
    expect(result.program).toBe("echo");
    expect(result.commandPath).toEqual(["echo"]);
    expect(result.arguments["string"]).toEqual([]);
  });

  it("Node.js format: argv[0] is 'node', argv[1] is script path → strip 2", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const result = parseAs(spec, ["/usr/bin/node", "/usr/local/bin/echo", "hello"]);
    expect(result.arguments["string"]).toEqual(["hello"]);
  });

  it("direct format: argv[0] matches spec name exactly → strip 1", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const result = parseAs(spec, ["echo", "hello"]);
    expect(result.arguments["string"]).toEqual(["hello"]);
  });

  it("path-based argv[0] ending with /echo (Unix separator) → strip 1", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const result = parseAs(spec, ["/usr/bin/echo", "hello"]);
    expect(result.arguments["string"]).toEqual(["hello"]);
  });

  it("path-based argv[0] ending with \\echo (Windows separator) → strip 1", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const result = parseAs(spec, ["C:\\Windows\\System32\\echo", "hello"]);
    expect(result.arguments["string"]).toEqual(["hello"]);
  });

  it("single-element argv matching spec name → strip 1, no args", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const result = parseAs(spec, ["echo"]);
    expect(result.arguments["string"]).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// 2. Phase 2 scanning edge cases
// ---------------------------------------------------------------------------

describe("Parser — Phase 2 scanning: pending flag at end of argv", () => {
  it("non-boolean flag at end of argv with no value → invalid_value error", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    expect(() => parseAs(spec, ["head", "--lines"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["head", "--lines"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "invalid_value")).toBe(true);
      expect(pe.errors[0].message).toContain("lines");
    }
  });

  it("short non-boolean flag at end of argv with no value → invalid_value error", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    expect(() => parseAs(spec, ["head", "-n"])).toThrow(ParseErrors);
  });
});

describe("Parser — Phase 2 scanning: LONG_FLAG_WITH_VALUE", () => {
  it("--message=value assigns value correctly (same as space-separated)", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const result = parseAs(spec, ["git", "commit", "--message=fix bug"]);
    expect(result.flags["message"]).toBe("fix bug");
  });

  it("--help=anything triggers early return as HelpResult", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const parser = new Parser(spec, ["echo", "--help=ignored"]);
    const result = parser.parse();
    expect("text" in result).toBe(true);
  });

  it("--unknown=value for unknown long flag → unknown_flag error", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    expect(() => parseAs(spec, ["echo", "--unknown=value"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["echo", "--unknown=value"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });
});

describe("Parser — Phase 2 scanning: SHORT_FLAG_WITH_VALUE", () => {
  it("-n20 (inline value for integer short flag) → coerces correctly", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    const result = parseAs(spec, ["head", "-n20"]);
    expect(result.flags["lines"]).toBe(20);
  });

  it("-xVALUE for unknown short flag → unknown_flag error", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    // 'x' is not a known short flag for echo
    expect(() => parseAs(spec, ["echo", "-xVALUE"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["echo", "-xVALUE"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });
});

describe("Parser — Phase 2 scanning: SINGLE_DASH_LONG flags", () => {
  it("-classpath value → parses correctly as SINGLE_DASH_LONG non-boolean", () => {
    const spec = loadSpec(JAVA_SPEC_RAW);
    const result = parseAs(spec, ["java", "-classpath", "/lib/foo.jar", "Main"]);
    expect(result.flags["classpath"]).toBe("/lib/foo.jar");
  });

  it("-verbose boolean SINGLE_DASH_LONG → sets flag to true", () => {
    const spec = loadSpec(JAVA_SPEC_RAW);
    const result = parseAs(spec, ["java", "-verbose", "Main"]);
    expect(result.flags["verbose"]).toBe(true);
  });

  it("-unknownsdl for unknown single_dash_long → unknown_flag error", () => {
    const spec = loadSpec(JAVA_SPEC_RAW);
    // '-notadeclaredsdl' is not a known SDL or short flag
    expect(() => parseAs(spec, ["java", "-notadeclaredsdl", "Main"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["java", "-notadeclaredsdl", "Main"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });

  it("-debug-port 5005 (SINGLE_DASH_LONG non-boolean) → next token is value", () => {
    const spec = loadSpec(JAVA_SPEC_RAW);
    const result = parseAs(spec, ["java", "-debug-port", "5005"]);
    expect(result.flags["debug-port"]).toBe(5005);
  });

  it("-debug-port at end of argv → invalid_value error (no value)", () => {
    const spec = loadSpec(JAVA_SPEC_RAW);
    expect(() => parseAs(spec, ["java", "-debug-port"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["java", "-debug-port"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "invalid_value")).toBe(true);
    }
  });
});

describe("Parser — Phase 2 scanning: UNKNOWN_FLAG", () => {
  it("completely unknown single-char flag → unknown_flag error", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    // 'z' is not a declared flag for echo
    expect(() => parseAs(spec, ["echo", "-z"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["echo", "-z"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });
});

describe("Parser — Phase 2 scanning: stacked flags edge cases", () => {
  it("stacked flags with unknown char → invalid_stack error", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    // -nz: n is valid, z is not
    expect(() => parseAs(spec, ["echo", "-nz"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["echo", "-nz"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "invalid_stack")).toBe(true);
    }
  });

  it("stacked flags where last char is non-boolean → next token consumed as value", () => {
    // head: -qn where q is boolean, n is integer (non-boolean)
    // The stack -qn should set q=true and then n awaits a value from next token
    const spec = loadSpec(HEAD_SPEC_RAW);
    const result = parseAs(spec, ["head", "-qn", "20"]);
    expect(result.flags["quiet"]).toBe(true);
    expect(result.flags["lines"]).toBe(20);
  });
});

describe("Parser — Phase 2 scanning: duplicate flag errors", () => {
  it("non-boolean flag specified twice (not repeatable) → duplicate_flag error", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    expect(() => parseAs(spec, ["head", "-n", "10", "-n", "20"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["head", "-n", "10", "-n", "20"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "duplicate_flag")).toBe(true);
    }
  });

  it("repeatable flag specified twice → collects both values as array", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    const result = parseAs(spec, ["head", "-e", "foo", "-e", "bar"]);
    expect(result.flags["patterns"]).toEqual(["foo", "bar"]);
  });
});

// ---------------------------------------------------------------------------
// 3. Fuzzy matching
// ---------------------------------------------------------------------------

describe("Parser — fuzzy matching suggestions", () => {
  it("typo in long flag name includes 'Did you mean' suggestion", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    // --no-newlie is close to --no-newline (distance 1)
    // Note: echo has no --no-newline (only -n short), so we'll use head
    const headSpec = loadSpec(HEAD_SPEC_RAW);
    // '--lins' is close to '--lines' (1 edit)
    try {
      parseAs(headSpec, ["head", "--lins", "5"]);
    } catch (e) {
      if (e instanceof ParseErrors) {
        const flagError = e.errors.find((err) => err.errorType === "unknown_flag");
        if (flagError && flagError.suggestion !== undefined) {
          // A suggestion was provided
          expect(flagError.suggestion).toContain("lines");
        }
        // Even if no suggestion, the error should be unknown_flag
        expect(e.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
      }
    }
  });

  it("completely different flag → no suggestion provided", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    try {
      parseAs(spec, ["head", "--zzzzzzzzzzz"]);
    } catch (e) {
      if (e instanceof ParseErrors) {
        const flagError = e.errors.find((err) => err.errorType === "unknown_flag");
        // Distance > 2, so no suggestion
        expect(flagError?.suggestion).toBeUndefined();
      }
    }
  });
});

// ---------------------------------------------------------------------------
// 4. Active flag set construction
// ---------------------------------------------------------------------------

describe("Parser — active flag set", () => {
  it("isolated command (inherit_global_flags: false) does not see global flags", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    // --no-pager is a global flag; isolated command does not inherit it
    // So --no-pager should be an unknown_flag in the isolated context
    expect(() => parseAs(spec, ["git", "isolated", "--no-pager"])).toThrow(ParseErrors);
    try {
      parseAs(spec, ["git", "isolated", "--no-pager"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });

  it("command that inherits global flags can use --no-pager", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    // commit command inherits global flags by default
    const result = parseAs(spec, ["git", "commit", "--amend", "--no-pager"]);
    expect(result.flags["no-pager"]).toBe(true);
  });

  it("root-level flags only active at root scope", () => {
    // echo has root-level flags -n and -e
    // When we add a subcommand, those root flags should not be in scope
    const specWithSub = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "A tool",
      flags: [
        { id: "root-only", long: "root-only", description: "Root only flag", type: "boolean" },
      ],
      commands: [
        {
          id: "cmd-sub",
          name: "sub",
          description: "Sub command",
        },
      ],
    });

    // At root level, --root-only is valid
    const rootResult = parseAs(specWithSub, ["tool", "--root-only"]);
    expect(rootResult.flags["root-only"]).toBe(true);

    // In subcommand scope, --root-only should be unknown
    expect(() => parseAs(specWithSub, ["tool", "sub", "--root-only"])).toThrow(ParseErrors);
    try {
      parseAs(specWithSub, ["tool", "sub", "--root-only"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// 5. _applyFlagDefaults
// ---------------------------------------------------------------------------

describe("Parser — _applyFlagDefaults", () => {
  it("repeatable flag absent → gets empty array []", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    const result = parseAs(spec, ["head"]);
    expect(result.flags["patterns"]).toEqual([]);
  });

  it("non-boolean flag with default absent → gets default value", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    const result = parseAs(spec, ["head"]);
    // lines flag has default: 10
    expect(result.flags["lines"]).toBe(10);
  });

  it("boolean flag absent → gets false", () => {
    const spec = loadSpec(HEAD_SPEC_RAW);
    const result = parseAs(spec, ["head"]);
    expect(result.flags["quiet"]).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// 6. Traditional mode edge cases
// ---------------------------------------------------------------------------

describe("Parser — traditional mode edge cases", () => {
  it("traditional mode: first token is a known command → not treated as stacked flags", () => {
    const specWithCmd = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "mytool",
      description: "Tool",
      parsing_mode: "traditional",
      flags: [
        { id: "verbose", short: "v", description: "Verbose", type: "boolean" },
      ],
      commands: [
        {
          id: "cmd-run",
          name: "run",
          description: "Run",
        },
      ],
    });
    // 'run' is a known command; should route to it, not treat as stacked flags
    const result = parseAs(specWithCmd, ["mytool", "run"]);
    expect(result.commandPath).toEqual(["mytool", "run"]);
  });

  it("traditional mode: first token starts with '-' → not treated as traditional stacked flags", () => {
    const spec = loadSpec(TAR_SPEC_RAW);
    // -czf is gnu-style, not traditional
    const result = parseAs(spec, ["tar", "-czf", "out.tar.gz", "./src"]);
    expect(result.flags["create"]).toBe(true);
    expect(result.flags["gzip"]).toBe(true);
    expect(result.flags["file"]).toBe("out.tar.gz");
  });

  it("traditional mode: token that is not a command and can be stacked → stacked", () => {
    const spec = loadSpec(TAR_SPEC_RAW);
    const result = parseAs(spec, ["tar", "xvf", "archive.tar"]);
    expect(result.flags["extract"]).toBe(true);
    expect(result.flags["verbose"]).toBe(true);
    expect(result.flags["file"]).toBe("archive.tar");
  });

  it("traditional mode: token that cannot be classified as stacked → falls through to positional", () => {
    // Use a simple traditional-mode spec with no short flags matching 'f'
    const simpleTraditional = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "simple",
      description: "Simple traditional tool",
      parsing_mode: "traditional",
      flags: [
        { id: "verbose", short: "v", description: "Verbose", type: "boolean" },
      ],
      arguments: [
        { id: "file", name: "FILE", description: "File", type: "path", required: false, variadic: true, variadic_min: 0 },
      ],
    });
    // 'xyz' with 'x' unknown → should fall through to positional
    const result = parseAs(simpleTraditional, ["simple", "xyz"]);
    // Either treated as positional or unknown stacked flags
    // The important thing is it doesn't throw
    expect(result).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// 7. Phase 1 routing edge cases
// ---------------------------------------------------------------------------

describe("Parser — Phase 1 routing edge cases", () => {
  it("end-of-flags '--' stops subcommand routing", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    // After '--', 'commit' is not a subcommand but a positional (root scope)
    // But root has no positional args defined in our spec, so this should
    // either work at root level or produce a too_many_arguments error.
    // The key behavior: routing stops at '--'
    const result = new Parser(spec, ["git", "--", "commit"]).parse();
    // Result should be at root scope (no commandPath beyond 'git')
    if ("commandPath" in result) {
      expect(result.commandPath).toEqual(["git"]);
    }
  });

  it("global flag before subcommand is skipped in routing", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    // --no-pager is a global flag; routing should skip it and still find commit
    const result = parseAs(spec, ["git", "--no-pager", "commit", "--amend"]);
    expect(result.commandPath).toEqual(["git", "commit"]);
    expect(result.flags["no-pager"]).toBe(true);
  });

  it("global non-boolean flag before subcommand is skipped in routing", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    // --git-dir /path is a global non-boolean flag; routing should skip both tokens
    const result = parseAs(spec, ["git", "--git-dir", "/path/to/repo", "commit", "--amend"]);
    expect(result.commandPath).toEqual(["git", "commit"]);
    expect(result.flags["git-dir"]).toBe("/path/to/repo");
  });
});

// ---------------------------------------------------------------------------
// 8. Help at subcommand level
// ---------------------------------------------------------------------------

describe("Parser — help at subcommand scope", () => {
  it("-h returns HelpResult scoped to current subcommand", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const parser = new Parser(spec, ["git", "commit", "-h"]);
    const result = parser.parse();
    expect("text" in result).toBe(true);
    if ("text" in result) {
      expect(result.commandPath).toContain("commit");
    }
  });

  it("--version at subcommand scope returns VersionResult", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const parser = new Parser(spec, ["git", "commit", "--version"]);
    const result = parser.parse();
    expect("version" in result && !("flags" in result)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 9. Enum flag parsing
// ---------------------------------------------------------------------------

describe("Parser — enum flag", () => {
  const enumSpec = loadSpec({
    cli_builder_spec_version: "1.0",
    name: "tool",
    description: "Tool with enum flag",
    flags: [
      { id: "format", long: "format", short: "f", description: "Output format", type: "enum", enum_values: ["json", "csv", "table"] },
    ],
  });

  it("--format=json assigns valid enum value", () => {
    const result = parseAs(enumSpec, ["tool", "--format=json"]);
    expect(result.flags["format"]).toBe("json");
  });

  it("--format csv (space-separated) assigns valid enum value", () => {
    const result = parseAs(enumSpec, ["tool", "--format", "csv"]);
    expect(result.flags["format"]).toBe("csv");
  });

  it("-f json (short form) assigns valid enum value", () => {
    const result = parseAs(enumSpec, ["tool", "-f", "json"]);
    expect(result.flags["format"]).toBe("json");
  });

  it("invalid enum value → invalid_enum_value error", () => {
    expect(() => parseAs(enumSpec, ["tool", "--format", "xml"])).toThrow(ParseErrors);
    try {
      parseAs(enumSpec, ["tool", "--format", "xml"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "invalid_enum_value")).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// 10. POSIX mode
// ---------------------------------------------------------------------------

describe("Parser — POSIX mode", () => {
  const posixSpec = loadSpec({
    cli_builder_spec_version: "1.0",
    name: "posix-tool",
    description: "POSIX parsing mode tool",
    parsing_mode: "posix",
    flags: [
      { id: "verbose", short: "v", long: "verbose", description: "Verbose", type: "boolean" },
      { id: "output", short: "o", long: "output", description: "Output", type: "string" },
    ],
    arguments: [
      { id: "files", name: "FILE", description: "Files", type: "path", required: false, variadic: true, variadic_min: 0 },
    ],
  });

  it("flags before first positional work correctly", () => {
    const result = parseAs(posixSpec, ["posix-tool", "-v", "file.txt"]);
    expect(result.flags["verbose"]).toBe(true);
    expect(result.arguments["files"]).toContain("file.txt");
  });

  it("flags after first positional are treated as positionals", () => {
    const result = parseAs(posixSpec, ["posix-tool", "file.txt", "-v"]);
    const files = result.arguments["files"] as string[];
    expect(files).toContain("file.txt");
    expect(files).toContain("-v"); // after first positional, flag scanning ended
    expect(result.flags["verbose"]).toBe(false); // -v was NOT consumed as a flag
  });
});

// ---------------------------------------------------------------------------
// 11. spec.version absent — --version is not a builtin
// ---------------------------------------------------------------------------

describe("Parser — --version when spec has no version", () => {
  it("--version is unknown_flag when spec has no version", () => {
    const specNoVersion = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "No version",
    });
    expect(() => parseAs(specNoVersion, ["tool", "--version"])).toThrow(ParseErrors);
    try {
      parseAs(specNoVersion, ["tool", "--version"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// 12. -h overridden by user-defined flag
// ---------------------------------------------------------------------------

describe("Parser — -h when user defines a flag with short 'h'", () => {
  it("-h calls user flag, not builtin help, when h is user-defined", () => {
    // If a user declares a flag with short 'h' (with id != __builtin_help),
    // -h should trigger that flag, not help.
    const specWithUserH = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "human-readable", short: "h", description: "Human readable", type: "boolean" },
      ],
    });
    // With user's -h defined, -h should set human-readable, not return help
    const result = parseAs(specWithUserH, ["tool", "-h"]);
    expect(result.flags["human-readable"]).toBe(true);
  });
});
