/**
 * spec-loader.test.ts — Tests for SpecLoader.
 *
 * Tests validate:
 * 1. A valid spec loads without errors and returns the correct CliSpec.
 * 2. Missing required fields throw SpecError.
 * 3. Duplicate IDs (flag, argument, command) throw SpecError.
 * 4. A circular requires dependency is detected via G_flag cycle check.
 * 5. Invalid type / missing enum_values throws SpecError.
 * 6. Multiple variadic args in same scope throws SpecError.
 * 7. Cross-reference validation (conflicts_with, requires with unknown IDs).
 */

import { describe, it, expect } from "vitest";
import { SpecLoader } from "../spec-loader.js";
import { SpecError } from "../errors.js";

// ---------------------------------------------------------------------------
// Helper: create a minimal valid echo spec
// ---------------------------------------------------------------------------
const ECHO_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "echo",
  description: "Display a line of text",
  version: "8.32",
  flags: [
    {
      id: "no-newline",
      short: "n",
      description: "Do not output the trailing newline",
      type: "boolean",
    },
    {
      id: "enable-escapes",
      short: "e",
      description: "Enable interpretation of backslash escapes",
      type: "boolean",
      conflicts_with: ["disable-escapes"],
    },
    {
      id: "disable-escapes",
      short: "E",
      description: "Disable interpretation of backslash escapes",
      type: "boolean",
      conflicts_with: ["enable-escapes"],
    },
  ],
  arguments: [
    {
      id: "string",
      name: "STRING",
      description: "Text to print",
      type: "string",
      required: false,
      variadic: true,
      variadic_min: 0,
    },
  ],
};

// ---------------------------------------------------------------------------
// Tests: valid spec
// ---------------------------------------------------------------------------

describe("SpecLoader — valid spec", () => {
  it("loads a minimal echo spec without errors", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);

    expect(spec.name).toBe("echo");
    expect(spec.description).toBe("Display a line of text");
    expect(spec.version).toBe("8.32");
    expect(spec.parsingMode).toBe("gnu");
    expect(spec.flags).toHaveLength(3);
    expect(spec.arguments).toHaveLength(1);
  });

  it("normalizes optional fields with defaults", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);

    // parsingMode defaults to "gnu"
    expect(spec.parsingMode).toBe("gnu");

    // builtinFlags defaults to both true
    expect(spec.builtinFlags.help).toBe(true);
    expect(spec.builtinFlags.version).toBe(true);

    // globalFlags defaults to []
    expect(spec.globalFlags).toHaveLength(0);

    // commands defaults to []
    expect(spec.commands).toHaveLength(0);

    // mutuallyExclusiveGroups defaults to []
    expect(spec.mutuallyExclusiveGroups).toHaveLength(0);
  });

  it("parses flag fields correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);

    const noNewline = spec.flags.find((f) => f.id === "no-newline")!;
    expect(noNewline.short).toBe("n");
    expect(noNewline.long).toBeUndefined();
    expect(noNewline.type).toBe("boolean");
    expect(noNewline.required).toBe(false);
    expect(noNewline.repeatable).toBe(false);

    const enableEscapes = spec.flags.find((f) => f.id === "enable-escapes")!;
    expect(enableEscapes.conflictsWith).toContain("disable-escapes");
  });

  it("parses argument fields correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);

    const strArg = spec.arguments[0];
    expect(strArg.id).toBe("string");
    expect(strArg.variadic).toBe(true);
    expect(strArg.variadicMin).toBe(0);
    expect(strArg.required).toBe(false);
  });

  it("loads the ls spec with flag requires", () => {
    const lsSpec = {
      cli_builder_spec_version: "1.0",
      name: "ls",
      description: "List directory contents",
      flags: [
        {
          id: "long-listing",
          short: "l",
          description: "Use long listing format",
          type: "boolean",
        },
        {
          id: "human-readable",
          short: "h",
          long: "human-readable",
          description: "Print sizes like 1K 234M 2G",
          type: "boolean",
          requires: ["long-listing"],
        },
      ],
    };

    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(lsSpec);
    expect(spec.flags).toHaveLength(2);
    const hr = spec.flags.find((f) => f.id === "human-readable")!;
    expect(hr.requires).toContain("long-listing");
  });

  it("loads the git spec with global flags and nested commands", () => {
    const gitSpec = {
      cli_builder_spec_version: "1.0",
      name: "git",
      description: "The stupid content tracker",
      version: "2.43.0",
      parsing_mode: "subcommand_first",
      global_flags: [
        {
          id: "no-pager",
          long: "no-pager",
          description: "Do not pipe output into a pager",
          type: "boolean",
        },
      ],
      commands: [
        {
          id: "cmd-remote",
          name: "remote",
          description: "Manage set of tracked repositories",
          flags: [
            {
              id: "verbose",
              short: "v",
              long: "verbose",
              description: "Be verbose",
              type: "boolean",
            },
          ],
          commands: [
            {
              id: "cmd-remote-add",
              name: "add",
              description: "Add a remote",
              arguments: [
                {
                  id: "name",
                  name: "NAME",
                  description: "Remote name",
                  type: "string",
                  required: true,
                },
                {
                  id: "url",
                  name: "URL",
                  description: "Remote URL",
                  type: "string",
                  required: true,
                },
              ],
            },
          ],
        },
      ],
    };

    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(gitSpec);
    expect(spec.parsingMode).toBe("subcommand_first");
    expect(spec.globalFlags).toHaveLength(1);
    expect(spec.commands).toHaveLength(1);

    const remote = spec.commands[0];
    expect(remote.name).toBe("remote");
    expect(remote.commands).toHaveLength(1);
    expect(remote.commands[0].name).toBe("add");
  });

  it("loads spec with mutually_exclusive_groups", () => {
    const tarSpec = {
      cli_builder_spec_version: "1.0",
      name: "tar",
      description: "An archiving utility",
      flags: [
        { id: "create", short: "c", description: "Create", type: "boolean" },
        { id: "extract", short: "x", description: "Extract", type: "boolean" },
      ],
      mutually_exclusive_groups: [
        { id: "operation", flag_ids: ["create", "extract"], required: true },
      ],
    };

    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(tarSpec);
    expect(spec.mutuallyExclusiveGroups).toHaveLength(1);
    expect(spec.mutuallyExclusiveGroups[0].required).toBe(true);
    expect(spec.mutuallyExclusiveGroups[0].flagIds).toContain("create");
  });

  it("caches the result on repeated load() calls", () => {
    const loader = new SpecLoader("(test)");
    const spec1 = loader.loadFromObject(ECHO_SPEC);
    const spec2 = loader.loadFromObject(ECHO_SPEC);
    // Second call returns the cached object (same reference)
    expect(spec1).toBe(spec2);
  });
});

// ---------------------------------------------------------------------------
// Tests: missing required fields
// ---------------------------------------------------------------------------

describe("SpecLoader — missing required fields", () => {
  it("throws SpecError when cli_builder_spec_version is missing", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({ name: "x", description: "y" }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when cli_builder_spec_version is wrong", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "2.0",
        name: "x",
        description: "y",
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when name is missing", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        description: "y",
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when description is missing", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError for invalid parsing_mode", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        parsing_mode: "quantum",
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when flag has no short/long/single_dash_long", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          {
            id: "verbose",
            description: "Be verbose",
            type: "boolean",
            // no short, long, or single_dash_long
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when enum flag has no enum_values", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          {
            id: "format",
            long: "format",
            description: "Output format",
            type: "enum",
            // no enum_values
          },
        ],
      }),
    ).toThrow(SpecError);
  });
});

// ---------------------------------------------------------------------------
// Tests: duplicate IDs
// ---------------------------------------------------------------------------

describe("SpecLoader — duplicate IDs", () => {
  it("throws SpecError on duplicate flag id", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          { id: "verbose", short: "v", description: "v", type: "boolean" },
          { id: "verbose", long: "verbose2", description: "v2", type: "boolean" },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError on duplicate argument id", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        arguments: [
          { id: "file", name: "FILE", description: "a file", type: "path", required: false },
          { id: "file", name: "FILE2", description: "another file", type: "path", required: false },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError on duplicate command id among siblings", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        commands: [
          { id: "cmd-add", name: "add", description: "add" },
          { id: "cmd-add", name: "add2", description: "add2" },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError on duplicate command name among siblings", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        commands: [
          { id: "cmd-a", name: "run", description: "run" },
          { id: "cmd-b", name: "run", description: "also run" },
        ],
      }),
    ).toThrow(SpecError);
  });
});

// ---------------------------------------------------------------------------
// Tests: circular requires
// ---------------------------------------------------------------------------

describe("SpecLoader — circular requires detection", () => {
  it("throws SpecError for A requires B requires A", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          {
            id: "verbose",
            short: "v",
            description: "verbose",
            type: "boolean",
            requires: ["quiet"],
          },
          {
            id: "quiet",
            short: "q",
            description: "quiet",
            type: "boolean",
            requires: ["verbose"],
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError for three-way cycle A→B→C→A", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          { id: "a", short: "a", description: "a", type: "boolean", requires: ["b"] },
          { id: "b", short: "b", description: "b", type: "boolean", requires: ["c"] },
          { id: "c", short: "c", description: "c", type: "boolean", requires: ["a"] },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("does NOT throw for a valid linear requires chain A→B→C", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          { id: "a", short: "a", description: "a", type: "boolean", requires: ["b"] },
          { id: "b", short: "b", description: "b", type: "boolean", requires: ["c"] },
          { id: "c", short: "c", description: "c", type: "boolean" },
        ],
      }),
    ).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Tests: cross-reference validation
// ---------------------------------------------------------------------------

describe("SpecLoader — cross-reference validation", () => {
  it("throws SpecError when conflicts_with references unknown flag", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          {
            id: "verbose",
            short: "v",
            description: "verbose",
            type: "boolean",
            conflicts_with: ["nonexistent"],
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when requires references unknown flag", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          {
            id: "human-readable",
            short: "h",
            description: "hr",
            type: "boolean",
            requires: ["long-listing"], // doesn't exist
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when exclusive group references unknown flag", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        flags: [
          { id: "create", short: "c", description: "c", type: "boolean" },
        ],
        mutually_exclusive_groups: [
          { id: "op", flag_ids: ["create", "nonexistent"] },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError for more than one variadic argument", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        cli_builder_spec_version: "1.0",
        name: "x",
        description: "y",
        arguments: [
          {
            id: "sources",
            name: "SOURCE",
            description: "sources",
            type: "path",
            required: false,
            variadic: true,
            variadic_min: 0,
          },
          {
            id: "dests",
            name: "DEST",
            description: "dests",
            type: "path",
            required: false,
            variadic: true,
            variadic_min: 0,
          },
        ],
      }),
    ).toThrow(SpecError);
  });
});
