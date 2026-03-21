/**
 * help-generator-extra.test.ts — Additional coverage for HelpGenerator.
 *
 * The existing help-generator.test.ts covers root help, subcommand help,
 * flag formatting, and argument formatting. This file targets branches not
 * yet covered:
 *
 * 1. Spec with no flags at all → no OPTIONS section
 * 2. Spec with builtin_flags.version = false → no --version in GLOBAL OPTIONS
 * 3. Spec with builtin_flags.help = false → no --help in GLOBAL OPTIONS
 * 4. Spec with no subcommands → no COMMANDS section in help
 * 5. Spec with no arguments at root → no ARGUMENTS section
 * 6. Single-dash-long flag in OPTIONS → _flagSignature with singleDashLong
 * 7. Flag with only singleDashLong (no short, no long) → signature is "-name"
 * 8. Flag with both long and singleDashLong
 * 9. _resolveCommandNode with invalid path (unknown segment) → null
 * 10. _buildFlagLines with single flag → no alignment padding issues
 * 11. Flag with long, type is non-boolean but no valueName → uses type.toUpperCase()
 * 12. Flag with singleDashLong, non-boolean, no valueName → uses type.toUpperCase()
 * 13. Spec with no global_flags and no builtins → no GLOBAL OPTIONS section
 * 14. USAGE line has no [OPTIONS] when no flags exist
 * 15. USAGE line has no [COMMAND] when no subcommands
 * 16. Required argument shows "Required." in ARGUMENTS section
 * 17. Optional argument shows "Optional." in ARGUMENTS section
 * 18. Variadic argument shows "Repeatable." in ARGUMENTS section
 */

import { describe, it, expect } from "vitest";
import { HelpGenerator } from "../help-generator.js";
import { SpecLoader } from "../spec-loader.js";
import type { CliSpec } from "../types.js";

function loadSpec(raw: Record<string, unknown>): CliSpec {
  const loader = new SpecLoader("(test)");
  return loader.loadFromObject(raw);
}

// ---------------------------------------------------------------------------
// Spec with no flags → no OPTIONS section
// ---------------------------------------------------------------------------

describe("HelpGenerator — no flags", () => {
  it("no OPTIONS section when spec has no flags", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "minimal",
      description: "Minimal tool",
      builtin_flags: { help: false, version: false },
      arguments: [
        { id: "file", name: "FILE", description: "A file", type: "path", required: true },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("OPTIONS");
    expect(help).toContain("ARGUMENTS");
    expect(help).toContain("FILE");
  });

  it("no GLOBAL OPTIONS when builtin_flags disabled and no global_flags", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "minimal",
      description: "Minimal tool",
      builtin_flags: { help: false, version: false },
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("GLOBAL OPTIONS");
  });

  it("no [OPTIONS] in USAGE line when no flags exist and builtins disabled", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "minimal",
      description: "Minimal",
      builtin_flags: { help: false, version: false },
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    const usageLine = help.split("\n").find((l) => l.startsWith("  minimal"))!;
    expect(usageLine).not.toContain("[OPTIONS]");
  });
});

// ---------------------------------------------------------------------------
// builtin_flags.version = false → no --version in GLOBAL OPTIONS
// ---------------------------------------------------------------------------

describe("HelpGenerator — builtin_flags.version = false", () => {
  it("--version not in GLOBAL OPTIONS when builtin version is disabled", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      version: "1.0",
      builtin_flags: { help: true, version: false },
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("--version");
    // --help should still be there
    expect(help).toContain("--help");
  });

  it("--version not in GLOBAL OPTIONS when spec has no version string", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      // no version field
      builtin_flags: { help: true, version: true },
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    // builtinFlags.version is true but spec.version is undefined
    // → --version is not added
    expect(help).not.toContain("--version");
  });
});

// ---------------------------------------------------------------------------
// builtin_flags.help = false → no --help in GLOBAL OPTIONS
// ---------------------------------------------------------------------------

describe("HelpGenerator — builtin_flags.help = false", () => {
  it("--help not in GLOBAL OPTIONS when builtin help is disabled", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      builtin_flags: { help: false, version: false },
      global_flags: [
        { id: "verbose", long: "verbose", description: "Verbose", type: "boolean" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("--help");
  });
});

// ---------------------------------------------------------------------------
// No subcommands → no COMMANDS section
// ---------------------------------------------------------------------------

describe("HelpGenerator — no subcommands", () => {
  it("no COMMANDS section when spec has no subcommands", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "echo",
      description: "Echo tool",
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("COMMANDS");
  });

  it("no [COMMAND] in USAGE line when no subcommands", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "echo",
      description: "Echo",
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("[COMMAND]");
  });
});

// ---------------------------------------------------------------------------
// No arguments → no ARGUMENTS section
// ---------------------------------------------------------------------------

describe("HelpGenerator — no arguments", () => {
  it("no ARGUMENTS section when spec has no positional arguments", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool with no args",
      flags: [
        { id: "verbose", short: "v", description: "Verbose", type: "boolean" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).not.toContain("ARGUMENTS");
  });
});

// ---------------------------------------------------------------------------
// singleDashLong flag in OPTIONS section
// ---------------------------------------------------------------------------

describe("HelpGenerator — singleDashLong flag signature", () => {
  it("flag with only singleDashLong shows '-name' in OPTIONS", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "java",
      description: "Java",
      flags: [
        { id: "verbose", single_dash_long: "verbose", description: "Verbose", type: "boolean" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("-verbose");
  });

  it("singleDashLong non-boolean flag shows '-name <TYPE>' in OPTIONS", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "java",
      description: "Java",
      flags: [
        { id: "classpath", single_dash_long: "classpath", description: "Classpath", type: "string" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("-classpath");
    expect(help).toContain("<STRING>");
  });

  it("singleDashLong non-boolean with value_name shows custom value name", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "java",
      description: "Java",
      flags: [
        {
          id: "classpath",
          single_dash_long: "classpath",
          description: "Classpath",
          type: "string",
          value_name: "classpath",
        },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("-classpath <classpath>");
  });

  it("flag with both long and singleDashLong shows both in signature", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        {
          id: "verbose",
          long: "verbose",
          single_dash_long: "verbose",
          description: "Verbose",
          type: "boolean",
        },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    // Both --verbose and -verbose should appear
    expect(help).toContain("--verbose");
    expect(help).toContain("-verbose");
  });
});

// ---------------------------------------------------------------------------
// Flag signature type names
// ---------------------------------------------------------------------------

describe("HelpGenerator — flag signature value names", () => {
  it("non-boolean flag without value_name uses type.toUpperCase() as placeholder", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "count", long: "count", description: "Count", type: "integer" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<INTEGER>");
  });

  it("non-boolean flag with value_name uses value_name as placeholder", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "count", long: "count", description: "Count", type: "integer", value_name: "NUM" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<NUM>");
    expect(help).not.toContain("<INTEGER>");
  });

  it("non-boolean string flag without value_name shows <STRING>", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "output", long: "output", description: "Output file", type: "string" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<STRING>");
  });

  it("float flag shows <FLOAT>", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "ratio", long: "ratio", description: "Ratio", type: "float" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<FLOAT>");
  });

  it("path flag shows <PATH>", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "dir", long: "dir", description: "Directory", type: "path" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<PATH>");
  });
});

// ---------------------------------------------------------------------------
// ARGUMENTS section: Required/Optional/Repeatable labels
// ---------------------------------------------------------------------------

describe("HelpGenerator — ARGUMENTS section formatting", () => {
  const specWithArgs = {
    cli_builder_spec_version: "1.0",
    name: "cp",
    description: "Copy",
    arguments: [
      { id: "source", name: "SOURCE", description: "Source file", type: "path", required: true, variadic: true, variadic_min: 1 },
      { id: "dest", name: "DEST", description: "Destination", type: "path", required: true },
      { id: "extra", name: "EXTRA", description: "Extra optional", type: "path", required: false, variadic: true, variadic_min: 0 },
    ],
  };

  it("required argument shows 'Required.' in ARGUMENTS", () => {
    const spec = loadSpec(specWithArgs);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("Required.");
  });

  it("optional argument shows 'Optional.' in ARGUMENTS", () => {
    const spec = loadSpec(specWithArgs);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("Optional.");
  });

  it("variadic argument shows 'Repeatable.' in ARGUMENTS", () => {
    const spec = loadSpec(specWithArgs);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("Repeatable.");
  });

  it("required variadic arg shows <SOURCE...> in ARGUMENTS", () => {
    const spec = loadSpec(specWithArgs);
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("<SOURCE...>");
  });
});

// ---------------------------------------------------------------------------
// _resolveCommandNode with invalid/unknown path
// ---------------------------------------------------------------------------

describe("HelpGenerator — _resolveCommandNode with unknown path", () => {
  it("generates help at root level when command path segment doesn't match", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "git",
      description: "Git",
      commands: [
        { id: "cmd-commit", name: "commit", description: "Commit" },
      ],
    });
    // "nonexistent" doesn't match any command → falls back to null (root level)
    const gen = new HelpGenerator(spec, ["nonexistent"]);
    const help = gen.generate();
    // Should not crash; generates root-level help
    expect(help).toContain("USAGE");
    expect(help).toContain("git");
  });
});

// ---------------------------------------------------------------------------
// Default value in flag description
// ---------------------------------------------------------------------------

describe("HelpGenerator — default value in flag description", () => {
  it("flag with default value shows [default: X] in description", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        {
          id: "lines",
          short: "n",
          long: "lines",
          description: "Number of lines",
          type: "integer",
          default: 10,
        },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    expect(help).toContain("[default: 10]");
  });

  it("flag with required: true and default does NOT show [default: X]", () => {
    // Per the source: only shows default if !flag.required
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        {
          id: "output",
          long: "output",
          description: "Output file",
          type: "string",
          required: true,
          default: "stdout",
        },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    // required: true, so no default shown
    expect(help).not.toContain("[default:");
  });
});

// ---------------------------------------------------------------------------
// _buildFlagLines alignment with single flag
// ---------------------------------------------------------------------------

describe("HelpGenerator — flag alignment", () => {
  it("single flag has at least 2 spaces padding before description", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      flags: [
        { id: "verbose", short: "v", description: "Be verbose", type: "boolean" },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    // The flag line should have "-v" followed by at least 2 spaces then the description
    expect(help).toMatch(/-v\s{2,}Be verbose/);
  });
});

// ---------------------------------------------------------------------------
// USAGE line includes argument placeholders
// ---------------------------------------------------------------------------

describe("HelpGenerator — USAGE line argument placeholders", () => {
  it("required argument shows <NAME> in USAGE line", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      arguments: [
        { id: "file", name: "FILE", description: "A file", type: "path", required: true },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    const usageLine = help.split("\n").find((l) => l.trim().startsWith("tool"))!;
    expect(usageLine).toContain("<FILE>");
  });

  it("optional argument shows [NAME] in USAGE line", () => {
    const spec = loadSpec({
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool",
      arguments: [
        { id: "file", name: "FILE", description: "A file", type: "path", required: false },
      ],
    });
    const gen = new HelpGenerator(spec, []);
    const help = gen.generate();
    const usageLine = help.split("\n").find((l) => l.trim().startsWith("tool"))!;
    expect(usageLine).toContain("[FILE]");
  });
});
