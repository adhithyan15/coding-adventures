/**
 * spec-loader-extra.test.ts — Additional coverage tests for SpecLoader.
 *
 * The existing spec-loader.test.ts covers the main happy paths and common
 * error paths. This file targets branches not yet covered:
 *
 * 1. Various valid parsing modes (posix, traditional, subcommand_first)
 * 2. builtin_flags field set to false explicitly
 * 3. display_name field
 * 4. required_unless references in flag validation
 * 5. Argument with required=false defaults (required defaults to true in args)
 * 6. flags is not an array → SpecError
 * 7. arguments is not an array → SpecError
 * 8. commands is not an array → SpecError
 * 9. mutually_exclusive_groups is not an array → SpecError
 * 10. exclusive group missing flag_ids → SpecError
 * 11. inherit_global_flags: false → command does not inherit global flags
 * 12. duplicate alias name among siblings → SpecError
 * 13. required_unless references unknown flag id → SpecError
 * 14. enum arg with enum_values → loads correctly
 * 15. variadic_max field parsed correctly
 * 16. Command with no arguments/flags/commands loads correctly
 * 17. Flag with value_name field
 * 18. Flag with repeatable: true
 * 19. Flag with default value
 * 20. Argument with default value
 */

import { describe, it, expect } from "vitest";
import { SpecLoader } from "../spec-loader.js";
import { SpecError } from "../errors.js";

const BASE = {
  cli_builder_spec_version: "1.0",
  name: "tool",
  description: "A test tool",
};

// ---------------------------------------------------------------------------
// Parsing modes
// ---------------------------------------------------------------------------

describe("SpecLoader — parsing modes", () => {
  it("accepts 'posix' as a valid parsing_mode", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({ ...BASE, parsing_mode: "posix" });
    expect(spec.parsingMode).toBe("posix");
  });

  it("accepts 'traditional' as a valid parsing_mode", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({ ...BASE, parsing_mode: "traditional" });
    expect(spec.parsingMode).toBe("traditional");
  });

  it("accepts 'subcommand_first' as a valid parsing_mode", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({ ...BASE, parsing_mode: "subcommand_first" });
    expect(spec.parsingMode).toBe("subcommand_first");
  });

  it("accepts 'gnu' as a valid parsing_mode", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({ ...BASE, parsing_mode: "gnu" });
    expect(spec.parsingMode).toBe("gnu");
  });

  it("defaults to 'gnu' when parsing_mode is null", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({ ...BASE, parsing_mode: null });
    expect(spec.parsingMode).toBe("gnu");
  });
});

// ---------------------------------------------------------------------------
// builtin_flags field
// ---------------------------------------------------------------------------

describe("SpecLoader — builtin_flags", () => {
  it("builtin_flags.help can be set to false", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      builtin_flags: { help: false, version: true },
    });
    expect(spec.builtinFlags.help).toBe(false);
    expect(spec.builtinFlags.version).toBe(true);
  });

  it("builtin_flags.version can be set to false", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      builtin_flags: { help: true, version: false },
    });
    expect(spec.builtinFlags.help).toBe(true);
    expect(spec.builtinFlags.version).toBe(false);
  });

  it("builtin_flags defaults to both true when field is absent", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(BASE);
    expect(spec.builtinFlags.help).toBe(true);
    expect(spec.builtinFlags.version).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// display_name
// ---------------------------------------------------------------------------

describe("SpecLoader — display_name", () => {
  it("parses display_name when present", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({ ...BASE, display_name: "My Tool" });
    expect(spec.displayName).toBe("My Tool");
  });

  it("displayName is undefined when field is absent", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(BASE);
    expect(spec.displayName).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// required_unless validation
// ---------------------------------------------------------------------------

describe("SpecLoader — required_unless validation", () => {
  it("throws SpecError when required_unless references unknown flag id", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [
          {
            id: "message",
            short: "m",
            description: "Commit message",
            type: "string",
            required: true,
            required_unless: ["nonexistent-flag"],
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("does not throw when required_unless references a valid flag id", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [
          {
            id: "message",
            short: "m",
            description: "message",
            type: "string",
            required: true,
            required_unless: ["amend"],
          },
          {
            id: "amend",
            long: "amend",
            description: "amend",
            type: "boolean",
          },
        ],
      }),
    ).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Non-array fields → SpecError
// ---------------------------------------------------------------------------

describe("SpecLoader — non-array fields", () => {
  it("throws SpecError when flags is not an array", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({ ...BASE, flags: "not-an-array" }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when arguments is not an array", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({ ...BASE, arguments: "not-an-array" }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when commands is not an array", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({ ...BASE, commands: "not-an-array" }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when mutually_exclusive_groups is not an array", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({ ...BASE, mutually_exclusive_groups: "not-an-array" }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when global_flags is not an array", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({ ...BASE, global_flags: "not-an-array" }),
    ).toThrow(SpecError);
  });
});

// ---------------------------------------------------------------------------
// Exclusive group validation
// ---------------------------------------------------------------------------

describe("SpecLoader — exclusive group validation", () => {
  it("throws SpecError when exclusive group is missing flag_ids", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [{ id: "create", short: "c", description: "c", type: "boolean" }],
        mutually_exclusive_groups: [
          { id: "op" }, // missing flag_ids
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when exclusive group flag_ids references unknown flag", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [{ id: "create", short: "c", description: "c", type: "boolean" }],
        mutually_exclusive_groups: [
          { id: "op", flag_ids: ["create", "nonexistent"] },
        ],
      }),
    ).toThrow(SpecError);
  });
});

// ---------------------------------------------------------------------------
// inherit_global_flags: false
// ---------------------------------------------------------------------------

describe("SpecLoader — inherit_global_flags: false", () => {
  it("command with inherit_global_flags: false does not include global flags", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      global_flags: [
        { id: "verbose", long: "verbose", description: "Verbose", type: "boolean" },
      ],
      commands: [
        {
          id: "cmd-isolated",
          name: "isolated",
          description: "Isolated command",
          inherit_global_flags: false,
          flags: [
            { id: "local-flag", short: "l", description: "Local", type: "boolean" },
          ],
        },
      ],
    });
    const isolatedCmd = spec.commands[0];
    expect(isolatedCmd.inheritGlobalFlags).toBe(false);
    // The command's own flags should only include its local flags
    expect(isolatedCmd.flags).toHaveLength(1);
    expect(isolatedCmd.flags[0].id).toBe("local-flag");
  });

  it("command with inherit_global_flags: true (default) does inherit", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      global_flags: [
        { id: "verbose", long: "verbose", description: "Verbose", type: "boolean" },
      ],
      commands: [
        {
          id: "cmd-normal",
          name: "normal",
          description: "Normal command",
          // inherit_global_flags defaults to true
        },
      ],
    });
    const normalCmd = spec.commands[0];
    expect(normalCmd.inheritGlobalFlags).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Duplicate alias name among siblings
// ---------------------------------------------------------------------------

describe("SpecLoader — duplicate alias/name among siblings", () => {
  it("throws SpecError when a command alias duplicates another command name", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        commands: [
          { id: "cmd-add", name: "add", description: "add" },
          {
            id: "cmd-create",
            name: "create",
            aliases: ["add"], // 'add' is already taken
            description: "create",
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError when two commands have the same alias", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        commands: [
          { id: "cmd-a", name: "a", aliases: ["common"], description: "a" },
          { id: "cmd-b", name: "b", aliases: ["common"], description: "b" },
        ],
      }),
    ).toThrow(SpecError);
  });
});

// ---------------------------------------------------------------------------
// Flag and argument field details
// ---------------------------------------------------------------------------

describe("SpecLoader — flag and argument field details", () => {
  it("parses flag with value_name correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      flags: [
        {
          id: "output",
          short: "o",
          long: "output",
          description: "Output file",
          type: "string",
          value_name: "FILE",
        },
      ],
    });
    const flag = spec.flags[0];
    expect(flag.valueName).toBe("FILE");
  });

  it("parses flag with repeatable: true correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      flags: [
        {
          id: "pattern",
          short: "e",
          description: "Pattern",
          type: "string",
          repeatable: true,
        },
      ],
    });
    const flag = spec.flags[0];
    expect(flag.repeatable).toBe(true);
  });

  it("parses flag default value correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      flags: [
        {
          id: "lines",
          short: "n",
          long: "lines",
          description: "Lines",
          type: "integer",
          default: 10,
        },
      ],
    });
    const flag = spec.flags[0];
    expect(flag.default).toBe(10);
  });

  it("parses argument default value correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      arguments: [
        {
          id: "dir",
          name: "DIR",
          description: "Directory",
          type: "path",
          required: false,
          default: ".",
        },
      ],
    });
    const arg = spec.arguments[0];
    expect(arg.default).toBe(".");
  });

  it("argument required defaults to true when not specified", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      arguments: [
        {
          id: "file",
          name: "FILE",
          description: "File",
          type: "path",
          // required not specified → defaults to true
        },
      ],
    });
    expect(spec.arguments[0].required).toBe(true);
  });

  it("argument required: false sets required to false", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      arguments: [
        {
          id: "file",
          name: "FILE",
          description: "File",
          type: "path",
          required: false,
        },
      ],
    });
    expect(spec.arguments[0].required).toBe(false);
  });

  it("parses argument variadicMax correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      arguments: [
        {
          id: "files",
          name: "FILE",
          description: "Files",
          type: "path",
          required: false,
          variadic: true,
          variadic_min: 0,
          variadic_max: 5,
        },
      ],
    });
    expect(spec.arguments[0].variadicMax).toBe(5);
  });

  it("parses enum argument correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      arguments: [
        {
          id: "format",
          name: "FORMAT",
          description: "Output format",
          type: "enum",
          enum_values: ["json", "csv"],
          required: true,
        },
      ],
    });
    const arg = spec.arguments[0];
    expect(arg.type).toBe("enum");
    expect(arg.enumValues).toEqual(["json", "csv"]);
  });

  it("throws SpecError when enum arg has no enum_values", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        arguments: [
          {
            id: "format",
            name: "FORMAT",
            description: "Output format",
            type: "enum",
            // no enum_values
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("parses required_unless_flag on argument correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      flags: [
        { id: "pattern-flag", short: "e", description: "Pattern", type: "string" },
      ],
      arguments: [
        {
          id: "pattern-arg",
          name: "PATTERN",
          description: "Pattern",
          type: "string",
          required_unless_flag: ["pattern-flag"],
        },
      ],
    });
    expect(spec.arguments[0].requiredUnlessFlag).toEqual(["pattern-flag"]);
  });

  it("parses all valid flag types without error", () => {
    const types = ["boolean", "string", "integer", "float", "path", "file", "directory"];
    for (const type of types) {
      const loader = new SpecLoader("(test)");
      expect(() =>
        loader.loadFromObject({
          ...BASE,
          flags: [
            {
              id: "myflag",
              long: "myflag",
              description: "A flag",
              type,
            },
          ],
        }),
      ).not.toThrow();
    }
  });

  it("throws SpecError for an invalid flag type", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [
          {
            id: "myflag",
            long: "myflag",
            description: "A flag",
            type: "invalid-type",
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError for flag with missing id field", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [
          {
            short: "v",
            description: "verbose",
            type: "boolean",
            // id missing
          },
        ],
      }),
    ).toThrow(SpecError);
  });

  it("throws SpecError for flag with missing description field", () => {
    const loader = new SpecLoader("(test)");
    expect(() =>
      loader.loadFromObject({
        ...BASE,
        flags: [
          {
            id: "verbose",
            short: "v",
            type: "boolean",
            // description missing
          },
        ],
      }),
    ).toThrow(SpecError);
  });
});

// ---------------------------------------------------------------------------
// Command aliases are parsed
// ---------------------------------------------------------------------------

describe("SpecLoader — command aliases", () => {
  it("parses command aliases correctly", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      commands: [
        {
          id: "cmd-remove",
          name: "remove",
          aliases: ["rm", "del"],
          description: "Remove something",
        },
      ],
    });
    const cmd = spec.commands[0];
    expect(cmd.aliases).toEqual(["rm", "del"]);
  });

  it("command with no aliases has empty aliases array", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject({
      ...BASE,
      commands: [
        { id: "cmd-run", name: "run", description: "Run" },
      ],
    });
    expect(spec.commands[0].aliases).toEqual([]);
  });
});
