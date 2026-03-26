/**
 * Tests for env -- run a program in a modified environment.
 *
 * We test the exported business logic functions: buildEnvironment,
 * parseArgs, and formatEnvironment.
 */

import { describe, it, expect } from "vitest";
import {
  buildEnvironment,
  parseArgs,
  formatEnvironment,
  EnvOptions,
} from "../src/env.js";

// ---------------------------------------------------------------------------
// Helper: default env options (no flags set).
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<EnvOptions> = {}): EnvOptions {
  return {
    ignoreEnvironment: false,
    unsetVars: [],
    nullSeparator: false,
    chdir: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// buildEnvironment: basic environment construction.
// ---------------------------------------------------------------------------

describe("buildEnvironment", () => {
  it("should copy the base environment when not ignoring", () => {
    const base = { HOME: "/home/alice", PATH: "/usr/bin" };
    const env = buildEnvironment(base, defaultOpts(), []);

    expect(env.HOME).toBe("/home/alice");
    expect(env.PATH).toBe("/usr/bin");
  });

  it("should start with empty environment when ignoreEnvironment is true", () => {
    const base = { HOME: "/home/alice", PATH: "/usr/bin" };
    const env = buildEnvironment(
      base,
      defaultOpts({ ignoreEnvironment: true }),
      []
    );

    expect(Object.keys(env)).toHaveLength(0);
  });

  it("should add assignments to the environment", () => {
    const env = buildEnvironment(
      {},
      defaultOpts(),
      ["FOO=bar", "BAZ=qux"]
    );

    expect(env.FOO).toBe("bar");
    expect(env.BAZ).toBe("qux");
  });

  it("should handle values containing equals signs", () => {
    const env = buildEnvironment(
      {},
      defaultOpts(),
      ["EQUATION=a=b=c"]
    );

    expect(env.EQUATION).toBe("a=b=c");
  });

  it("should handle empty values", () => {
    const env = buildEnvironment(
      {},
      defaultOpts(),
      ["EMPTY="]
    );

    expect(env.EMPTY).toBe("");
  });

  it("should remove variables specified by unsetVars", () => {
    const base = { HOME: "/home/alice", PATH: "/usr/bin", SHELL: "/bin/zsh" };
    const env = buildEnvironment(
      base,
      defaultOpts({ unsetVars: ["PATH", "SHELL"] }),
      []
    );

    expect(env.HOME).toBe("/home/alice");
    expect(env.PATH).toBeUndefined();
    expect(env.SHELL).toBeUndefined();
  });

  it("should apply removals before additions", () => {
    const base = { FOO: "old" };
    const env = buildEnvironment(
      base,
      defaultOpts({ unsetVars: ["FOO"] }),
      ["FOO=new"]
    );

    expect(env.FOO).toBe("new");
  });

  it("should override existing variables with assignments", () => {
    const base = { FOO: "old" };
    const env = buildEnvironment(base, defaultOpts(), ["FOO=new"]);

    expect(env.FOO).toBe("new");
  });

  it("should filter out undefined values from base", () => {
    const base: Record<string, string | undefined> = {
      DEFINED: "yes",
      UNDEF: undefined,
    };
    const env = buildEnvironment(base, defaultOpts(), []);

    expect(env.DEFINED).toBe("yes");
    expect("UNDEF" in env).toBe(false);
  });

  it("should handle ignoreEnvironment with assignments", () => {
    const base = { HOME: "/home/alice" };
    const env = buildEnvironment(
      base,
      defaultOpts({ ignoreEnvironment: true }),
      ["NEWVAR=value"]
    );

    expect(env.HOME).toBeUndefined();
    expect(env.NEWVAR).toBe("value");
  });

  it("should remove a variable that doesn't exist without error", () => {
    const base = { HOME: "/home/alice" };
    const env = buildEnvironment(
      base,
      defaultOpts({ unsetVars: ["NONEXISTENT"] }),
      []
    );

    expect(env.HOME).toBe("/home/alice");
  });
});

// ---------------------------------------------------------------------------
// parseArgs: splitting positional args into assignments and command.
// ---------------------------------------------------------------------------

describe("parseArgs", () => {
  it("should separate assignments from command", () => {
    const result = parseArgs(["FOO=bar", "echo", "hello"]);

    expect(result.assignments).toEqual(["FOO=bar"]);
    expect(result.command).toEqual(["echo", "hello"]);
  });

  it("should handle multiple assignments", () => {
    const result = parseArgs(["FOO=bar", "BAZ=qux", "mycommand"]);

    expect(result.assignments).toEqual(["FOO=bar", "BAZ=qux"]);
    expect(result.command).toEqual(["mycommand"]);
  });

  it("should handle only assignments (no command)", () => {
    const result = parseArgs(["FOO=bar", "BAZ=qux"]);

    expect(result.assignments).toEqual(["FOO=bar", "BAZ=qux"]);
    expect(result.command).toEqual([]);
  });

  it("should handle only command (no assignments)", () => {
    const result = parseArgs(["echo", "hello", "world"]);

    expect(result.assignments).toEqual([]);
    expect(result.command).toEqual(["echo", "hello", "world"]);
  });

  it("should handle empty input", () => {
    const result = parseArgs([]);

    expect(result.assignments).toEqual([]);
    expect(result.command).toEqual([]);
  });

  it("should treat arguments with = but invalid name as command start", () => {
    // "123=foo" starts with a digit, not a valid variable name.
    const result = parseArgs(["123=foo", "echo"]);

    expect(result.assignments).toEqual([]);
    expect(result.command).toEqual(["123=foo", "echo"]);
  });

  it("should handle underscores in variable names", () => {
    const result = parseArgs(["MY_VAR=value", "_PRIVATE=secret", "cmd"]);

    expect(result.assignments).toEqual(["MY_VAR=value", "_PRIVATE=secret"]);
    expect(result.command).toEqual(["cmd"]);
  });

  it("should handle assignments with values containing spaces", () => {
    const result = parseArgs(["MSG=hello world", "cmd"]);
    // "MSG=hello world" is a valid assignment (value is "hello world").
    // But "cmd" is not reached since "world" doesn't have =.
    // Actually, "MSG=hello world" is one string with a space in the value.
    // The split happens at shell level; parseArgs gets pre-split args.
    // So "MSG=hello world" as a single string IS a valid assignment.

    expect(result.assignments).toEqual(["MSG=hello world"]);
    expect(result.command).toEqual(["cmd"]);
  });
});

// ---------------------------------------------------------------------------
// formatEnvironment: formatting environment for output.
// ---------------------------------------------------------------------------

describe("formatEnvironment", () => {
  it("should format with newline separator by default", () => {
    const env = { FOO: "bar", BAZ: "qux" };
    const result = formatEnvironment(env, false);

    expect(result).toContain("FOO=bar\n");
    expect(result).toContain("BAZ=qux\n");
  });

  it("should format with NUL separator when requested", () => {
    const env = { FOO: "bar", BAZ: "qux" };
    const result = formatEnvironment(env, true);

    expect(result).toContain("FOO=bar\0");
    expect(result).toContain("BAZ=qux\0");
  });

  it("should return empty string for empty environment", () => {
    const result = formatEnvironment({}, false);
    expect(result).toBe("");
  });

  it("should handle values with special characters", () => {
    const env = { PATH: "/usr/bin:/bin:/usr/local/bin" };
    const result = formatEnvironment(env, false);

    expect(result).toBe("PATH=/usr/bin:/bin:/usr/local/bin\n");
  });

  it("should handle empty values", () => {
    const env = { EMPTY: "" };
    const result = formatEnvironment(env, false);

    expect(result).toBe("EMPTY=\n");
  });
});
