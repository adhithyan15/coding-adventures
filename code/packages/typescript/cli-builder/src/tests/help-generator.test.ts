/**
 * help-generator.test.ts — Tests for HelpGenerator.
 *
 * Tests verify:
 * 1. Root help for a simple program (echo) includes all sections.
 * 2. Root help for git shows COMMANDS section.
 * 3. Subcommand help scopes to the subcommand.
 * 4. Nested subcommand help (git remote add).
 * 5. Formatting: flag signatures, argument displays, defaults.
 */

import { describe, it, expect } from "vitest";
import { HelpGenerator } from "../help-generator.js";
import { SpecLoader } from "../spec-loader.js";
import type { CliSpec } from "../types.js";

// ---------------------------------------------------------------------------
// Spec fixtures
// ---------------------------------------------------------------------------

const ECHO_SPEC_RAW = {
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

const GIT_SPEC_RAW = {
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
      id: "cmd-commit",
      name: "commit",
      description: "Record changes to the repository",
      flags: [
        {
          id: "message",
          short: "m",
          long: "message",
          description: "Use the given message",
          type: "string",
          value_name: "MSG",
          required: true,
        },
        {
          id: "verbose",
          short: "v",
          long: "verbose",
          description: "Show diff",
          type: "boolean",
        },
      ],
    },
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
          description: "Add a remote named NAME",
          arguments: [
            {
              id: "name",
              name: "NAME",
              description: "Name for the remote",
              type: "string",
              required: true,
            },
            {
              id: "url",
              name: "URL",
              description: "URL of the remote",
              type: "string",
              required: true,
            },
          ],
        },
      ],
    },
  ],
};

function loadSpec(raw: Record<string, unknown>): CliSpec {
  const loader = new SpecLoader("(test)");
  return loader.loadFromObject(raw);
}

// ---------------------------------------------------------------------------
// Tests: root help
// ---------------------------------------------------------------------------

describe("HelpGenerator — root help", () => {
  it("includes USAGE section", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();

    expect(help).toContain("USAGE");
    expect(help).toContain("echo");
  });

  it("includes DESCRIPTION section", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();

    expect(help).toContain("DESCRIPTION");
    expect(help).toContain("Display a line of text");
  });

  it("includes OPTIONS section with flag descriptions", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();

    expect(help).toContain("OPTIONS");
    expect(help).toContain("-n");
    expect(help).toContain("Do not output the trailing newline");
    expect(help).toContain("-e");
    expect(help).toContain("Enable interpretation");
  });

  it("includes ARGUMENTS section for programs with positional args", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();

    expect(help).toContain("ARGUMENTS");
    expect(help).toContain("STRING");
  });

  it("includes GLOBAL OPTIONS with --help and --version", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();

    expect(help).toContain("GLOBAL OPTIONS");
    expect(help).toContain("--help");
    expect(help).toContain("--version");
  });

  it("includes COMMANDS section for programs with subcommands", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();

    expect(help).toContain("COMMANDS");
    expect(help).toContain("commit");
    expect(help).toContain("remote");
    expect(help).toContain("Record changes to the repository");
  });

  it("uses [OPTIONS] placeholder in USAGE when flags exist", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("[OPTIONS]");
  });

  it("uses [COMMAND] placeholder in USAGE when subcommands exist", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("[COMMAND]");
  });
});

// ---------------------------------------------------------------------------
// Tests: flag formatting
// ---------------------------------------------------------------------------

describe("HelpGenerator — flag formatting", () => {
  it("formats boolean flags as '-s, --long'", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    // -n flag has no long form; should appear as just "-n"
    expect(help).toMatch(/-n/);
  });

  it("formats non-boolean flags with value name", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["commit"]);
    const help = gen.generate();
    // -m/--message should show value name MSG
    expect(help).toMatch(/--message/);
    expect(help).toMatch(/MSG/);
  });

  it("includes --long for flags that have both short and long", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["commit"]);
    const help = gen.generate();
    expect(help).toContain("--verbose");
  });

  it("includes default value when flag has a default", () => {
    const specWithDefault = {
      cli_builder_spec_version: "1.0",
      name: "head",
      description: "Output the first part of files",
      flags: [
        {
          id: "lines",
          short: "n",
          long: "lines",
          description: "Print the first NUM lines",
          type: "integer",
          value_name: "NUM",
          default: 10,
        },
      ],
    };
    const spec = loadSpec(specWithDefault);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("default: 10");
  });
});

// ---------------------------------------------------------------------------
// Tests: argument formatting
// ---------------------------------------------------------------------------

describe("HelpGenerator — argument formatting", () => {
  it("shows required args as <NAME>", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["remote", "add"]);
    const help = gen.generate();
    expect(help).toContain("<NAME>");
    expect(help).toContain("<URL>");
  });

  it("shows optional variadic args as [NAME...]", () => {
    const spec = loadSpec(ECHO_SPEC_RAW);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    // STRING is optional and variadic → [STRING...]
    expect(help).toContain("[STRING...]");
  });

  it("shows required variadic args as <NAME...>", () => {
    const specWithRequiredVariadic = {
      cli_builder_spec_version: "1.0",
      name: "cp",
      description: "Copy files",
      arguments: [
        {
          id: "source",
          name: "SOURCE",
          description: "Source files",
          type: "path",
          required: true,
          variadic: true,
          variadic_min: 1,
        },
        {
          id: "dest",
          name: "DEST",
          description: "Destination",
          type: "path",
          required: true,
        },
      ],
    };
    const spec = loadSpec(specWithRequiredVariadic);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<SOURCE...>");
    expect(help).toContain("<DEST>");
  });
});

// ---------------------------------------------------------------------------
// Tests: subcommand help
// ---------------------------------------------------------------------------

describe("HelpGenerator — subcommand help", () => {
  it("scopes help to the subcommand's description and flags", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["commit"]);
    const help = gen.generate();

    expect(help).toContain("Record changes to the repository");
    expect(help).toContain("-m");
    expect(help).toContain("--message");
  });

  it("scopes help to the USAGE line for the subcommand", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["commit"]);
    const help = gen.generate();
    expect(help).toContain("git commit");
  });

  it("shows nested subcommand help for git remote add", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["remote", "add"]);
    const help = gen.generate();

    expect(help).toContain("Add a remote named NAME");
    expect(help).toContain("NAME");
    expect(help).toContain("URL");
  });

  it("shows COMMANDS section for intermediate subcommand (git remote)", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["remote"]);
    const help = gen.generate();

    expect(help).toContain("COMMANDS");
    expect(help).toContain("add");
    expect(help).toContain("Add a remote named NAME");
  });

  it("includes global flags in GLOBAL OPTIONS section", () => {
    const spec = loadSpec(GIT_SPEC_RAW);
    const gen = new HelpGenerator(spec, ["commit"]);
    const help = gen.generate();

    expect(help).toContain("GLOBAL OPTIONS");
    expect(help).toContain("--no-pager");
  });
});
