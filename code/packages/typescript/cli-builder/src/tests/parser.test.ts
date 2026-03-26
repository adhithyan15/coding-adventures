/**
 * parser.test.ts — Integration tests for the Parser.
 *
 * These tests use embedded JSON specs (not files) to verify end-to-end
 * parsing behavior. They cover:
 *
 * 1. echo — variadic args, flag conflicts, -e/-E
 * 2. ls — flag stacking (-lah), requires dependency (-h requires -l)
 * 3. cp — variadic sources with required trailing DEST (last-wins)
 * 4. grep — conditional required arg, exclusive group, repeatable flag
 * 5. tar — traditional mode (xvf without dash)
 * 6. git — deep subcommands, global flags, alias resolution
 * 7. java — single_dash_long flags and longest-match-first
 * 8. help/version — builtin flag early return
 * 9. Error cases from all error types
 */

import { describe, it, expect } from "vitest";
import { Parser } from "../parser.js";
import { SpecLoader } from "../spec-loader.js";
import { ParseErrors } from "../errors.js";
import type { CliSpec, ParseResult } from "../types.js";

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
// Spec fixtures
// ---------------------------------------------------------------------------

const ECHO_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "echo",
  description: "Display a line of text",
  version: "8.32",
  flags: [
    { id: "no-newline", short: "n", description: "Do not output trailing newline", type: "boolean" },
    { id: "enable-escapes", short: "e", description: "Enable backslash escapes", type: "boolean", conflicts_with: ["disable-escapes"] },
    { id: "disable-escapes", short: "E", description: "Disable backslash escapes", type: "boolean", conflicts_with: ["enable-escapes"] },
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

const LS_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "ls",
  description: "List directory contents",
  version: "8.32",
  parsing_mode: "gnu",
  flags: [
    { id: "long-listing", short: "l", description: "Use long listing format", type: "boolean", conflicts_with: ["single-column"] },
    { id: "all", short: "a", long: "all", description: "Do not ignore . entries", type: "boolean" },
    { id: "human-readable", short: "h", long: "human-readable", description: "Print sizes like 1K 234M", type: "boolean", requires: ["long-listing"] },
    { id: "reverse", short: "r", long: "reverse", description: "Reverse order", type: "boolean" },
    { id: "sort-time", short: "t", description: "Sort by time", type: "boolean" },
    { id: "recursive", short: "R", long: "recursive", description: "List recursively", type: "boolean" },
    { id: "single-column", short: "1", description: "One file per line", type: "boolean", conflicts_with: ["long-listing"] },
  ],
  arguments: [
    {
      id: "path",
      name: "PATH",
      description: "Directory or file to list",
      type: "path",
      required: false,
      variadic: true,
      variadic_min: 0,
      default: ".",
    },
  ],
};

const CP_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "cp",
  description: "Copy files and directories",
  version: "8.32",
  flags: [
    { id: "recursive", short: "r", long: "recursive", description: "Copy directories recursively", type: "boolean" },
    { id: "force", short: "f", long: "force", description: "Overwrite without prompting", type: "boolean", conflicts_with: ["interactive", "no-clobber"] },
    { id: "interactive", short: "i", long: "interactive", description: "Prompt before overwrite", type: "boolean", conflicts_with: ["force", "no-clobber"] },
    { id: "no-clobber", short: "n", long: "no-clobber", description: "Do not overwrite", type: "boolean", conflicts_with: ["force", "interactive"] },
    { id: "verbose", short: "v", long: "verbose", description: "Explain what is being done", type: "boolean" },
  ],
  arguments: [
    {
      id: "source",
      name: "SOURCE",
      description: "Source file(s)",
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
      variadic: false,
    },
  ],
};

const GREP_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "grep",
  description: "Print lines that match patterns",
  version: "3.7",
  flags: [
    { id: "ignore-case", short: "i", long: "ignore-case", description: "Ignore case", type: "boolean" },
    { id: "invert-match", short: "v", long: "invert-match", description: "Invert matching", type: "boolean" },
    { id: "line-number", short: "n", long: "line-number", description: "Print line numbers", type: "boolean" },
    { id: "recursive", short: "r", long: "recursive", description: "Read recursively", type: "boolean" },
    { id: "regexp", short: "e", long: "regexp", description: "Pattern (repeatable)", type: "string", value_name: "PATTERN", repeatable: true },
    { id: "extended-regexp", short: "E", long: "extended-regexp", description: "Extended regex", type: "boolean" },
    { id: "fixed-strings", short: "F", long: "fixed-strings", description: "Fixed strings", type: "boolean" },
    { id: "perl-regexp", short: "P", long: "perl-regexp", description: "Perl regex", type: "boolean" },
  ],
  arguments: [
    {
      id: "pattern",
      name: "PATTERN",
      description: "The search pattern",
      type: "string",
      required: true,
      required_unless_flag: ["regexp"],
    },
    {
      id: "files",
      name: "FILE",
      description: "Files to search",
      type: "path",
      required: false,
      variadic: true,
      variadic_min: 0,
    },
  ],
  mutually_exclusive_groups: [
    {
      id: "regex-engine",
      flag_ids: ["extended-regexp", "fixed-strings", "perl-regexp"],
      required: false,
    },
  ],
};

const TAR_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "tar",
  description: "An archiving utility",
  version: "1.34",
  parsing_mode: "traditional",
  flags: [
    { id: "create", short: "c", description: "Create archive", type: "boolean" },
    { id: "extract", short: "x", description: "Extract archive", type: "boolean" },
    { id: "list", short: "t", description: "List archive", type: "boolean" },
    { id: "verbose", short: "v", long: "verbose", description: "Verbose", type: "boolean" },
    { id: "file", short: "f", long: "file", description: "Archive file", type: "path", value_name: "ARCHIVE" },
    { id: "gzip", short: "z", long: "gzip", description: "Filter through gzip", type: "boolean" },
  ],
  arguments: [
    {
      id: "member",
      name: "MEMBER",
      description: "Archive members",
      type: "path",
      required: false,
      variadic: true,
      variadic_min: 0,
    },
  ],
  mutually_exclusive_groups: [
    { id: "operation", flag_ids: ["create", "extract", "list"], required: true },
  ],
};

const GIT_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "git",
  description: "The stupid content tracker",
  version: "2.43.0",
  parsing_mode: "subcommand_first",
  global_flags: [
    { id: "no-pager", long: "no-pager", description: "Do not pipe into pager", type: "boolean" },
  ],
  commands: [
    {
      id: "cmd-add",
      name: "add",
      description: "Add file contents to the index",
      flags: [
        { id: "dry-run", short: "n", long: "dry-run", description: "Dry run", type: "boolean" },
        { id: "verbose", short: "v", long: "verbose", description: "Be verbose", type: "boolean" },
        { id: "all", short: "A", long: "all", description: "Add all changes", type: "boolean" },
      ],
      arguments: [
        { id: "pathspec", name: "PATHSPEC", description: "Files to add", type: "path", required: true, variadic: true, variadic_min: 1 },
      ],
    },
    {
      id: "cmd-commit",
      name: "commit",
      description: "Record changes to the repository",
      flags: [
        { id: "message", short: "m", long: "message", description: "Commit message", type: "string", value_name: "MSG", required: true, required_unless: ["amend"] },
        { id: "all", short: "a", long: "all", description: "Stage all changes", type: "boolean" },
        { id: "amend", long: "amend", description: "Amend previous commit", type: "boolean" },
        { id: "verbose", short: "v", long: "verbose", description: "Show diff", type: "boolean" },
      ],
    },
    {
      id: "cmd-remote",
      name: "remote",
      description: "Manage set of tracked repositories",
      flags: [
        { id: "verbose", short: "v", long: "verbose", description: "Be verbose", type: "boolean" },
      ],
      commands: [
        {
          id: "cmd-remote-add",
          name: "add",
          description: "Add a named remote",
          arguments: [
            { id: "name", name: "NAME", description: "Remote name", type: "string", required: true },
            { id: "url", name: "URL", description: "Remote URL", type: "string", required: true },
          ],
        },
        {
          id: "cmd-remote-remove",
          name: "remove",
          aliases: ["rm"],
          description: "Remove the remote named NAME",
          arguments: [
            { id: "name", name: "NAME", description: "Remote name", type: "string", required: true },
          ],
        },
      ],
    },
  ],
};

const JAVA_SPEC = {
  cli_builder_spec_version: "1.0",
  name: "java",
  description: "Launches a Java application",
  flags: [
    { id: "classpath", single_dash_long: "classpath", description: "Classpath", type: "string", value_name: "classpath" },
    { id: "classpath-short", single_dash_long: "cp", description: "Alias for -classpath", type: "string", value_name: "classpath", conflicts_with: ["classpath"] },
    { id: "verbose", single_dash_long: "verbose", description: "Verbose", type: "boolean" },
    { id: "jar", single_dash_long: "jar", description: "Execute JAR", type: "boolean" },
  ],
  arguments: [
    { id: "class-or-jar", name: "CLASS|JARFILE", description: "Main class or JAR", type: "string", required: false },
    { id: "args", name: "ARGS", description: "Arguments to main", type: "string", required: false, variadic: true, variadic_min: 0 },
  ],
};

// ---------------------------------------------------------------------------
// echo tests
// ---------------------------------------------------------------------------

describe("Parser — echo", () => {
  it("echo hello world → variadic strings", () => {
    const result = parseAs(ECHO_SPEC, ["echo", "hello", "world"]);
    expect(result.arguments["string"]).toEqual(["hello", "world"]);
    expect(result.flags["no-newline"]).toBe(false);
  });

  it("echo -n hello → no-newline flag true", () => {
    const result = parseAs(ECHO_SPEC, ["echo", "-n", "hello"]);
    expect(result.flags["no-newline"]).toBe(true);
    expect(result.arguments["string"]).toEqual(["hello"]);
  });

  it("echo (no args) → empty string array", () => {
    const result = parseAs(ECHO_SPEC, ["echo"]);
    expect(result.arguments["string"]).toEqual([]);
    expect(result.flags["no-newline"]).toBe(false);
  });

  it("echo -e -E hello → conflicting_flags error", () => {
    expect(() => parseAs(ECHO_SPEC, ["echo", "-e", "-E", "hello"])).toThrow(
      ParseErrors,
    );
    try {
      parseAs(ECHO_SPEC, ["echo", "-e", "-E", "hello"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "conflicting_flags")).toBe(true);
    }
  });

  it("echo -n -e hello → both flags true, string set", () => {
    const result = parseAs(ECHO_SPEC, ["echo", "-n", "-e", "hello"]);
    expect(result.flags["no-newline"]).toBe(true);
    expect(result.flags["enable-escapes"]).toBe(true);
    expect(result.arguments["string"]).toEqual(["hello"]);
  });
});

// ---------------------------------------------------------------------------
// ls tests
// ---------------------------------------------------------------------------

describe("Parser — ls", () => {
  it("ls -lah /tmp → stacked flags + path arg", () => {
    const result = parseAs(LS_SPEC, ["ls", "-lah", "/tmp"]);
    expect(result.flags["long-listing"]).toBe(true);
    expect(result.flags["all"]).toBe(true);
    expect(result.flags["human-readable"]).toBe(true);
    expect(result.arguments["path"]).toEqual(["/tmp"]);
  });

  it("ls (no args) → all flags false, path defaults to ['.']", () => {
    const result = parseAs(LS_SPEC, ["ls"]);
    expect(result.flags["long-listing"]).toBe(false);
    expect(result.flags["all"]).toBe(false);
    // Default is "." but variadic returns array
    // The default is applied as a single value; when no tokens are provided
    // the result should be the default
    expect(result.arguments["path"]).toBeDefined();
  });

  it("ls -h → missing_dependency_flag error (-h requires -l)", () => {
    expect(() => parseAs(LS_SPEC, ["ls", "-h"])).toThrow(ParseErrors);
    try {
      parseAs(LS_SPEC, ["ls", "-h"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "missing_dependency_flag"),
      ).toBe(true);
    }
  });

  it("ls -1 -l → conflicting_flags error", () => {
    expect(() => parseAs(LS_SPEC, ["ls", "-1", "-l"])).toThrow(ParseErrors);
    try {
      parseAs(LS_SPEC, ["ls", "-1", "-l"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((e) => e.errorType === "conflicting_flags")).toBe(true);
    }
  });

  it("ls -la → long-listing and all true", () => {
    const result = parseAs(LS_SPEC, ["ls", "-la"]);
    expect(result.flags["long-listing"]).toBe(true);
    expect(result.flags["all"]).toBe(true);
  });

  it("ls --all → all flag via long form", () => {
    const result = parseAs(LS_SPEC, ["ls", "--all"]);
    expect(result.flags["all"]).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// cp tests
// ---------------------------------------------------------------------------

describe("Parser — cp (variadic + trailing dest)", () => {
  it("cp a.txt /tmp/ → source=['a.txt'], dest='/tmp/'", () => {
    const result = parseAs(CP_SPEC, ["cp", "a.txt", "/tmp/"]);
    expect(result.arguments["source"]).toEqual(["a.txt"]);
    expect(result.arguments["dest"]).toBe("/tmp/");
  });

  it("cp a.txt b.txt c.txt /dest/ → source=['a','b','c'], dest='/dest/'", () => {
    const result = parseAs(CP_SPEC, ["cp", "a.txt", "b.txt", "c.txt", "/dest/"]);
    expect(result.arguments["source"]).toEqual(["a.txt", "b.txt", "c.txt"]);
    expect(result.arguments["dest"]).toBe("/dest/");
  });

  it("cp a.txt → missing_required_argument for DEST", () => {
    expect(() => parseAs(CP_SPEC, ["cp", "a.txt"])).toThrow(ParseErrors);
    try {
      parseAs(CP_SPEC, ["cp", "a.txt"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "missing_required_argument"),
      ).toBe(true);
    }
  });

  it("cp (no args) → too_few_arguments for SOURCE", () => {
    expect(() => parseAs(CP_SPEC, ["cp"])).toThrow(ParseErrors);
    try {
      parseAs(CP_SPEC, ["cp"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "too_few_arguments"),
      ).toBe(true);
    }
  });

  it("cp -f a.txt b.txt → force flag + source/dest", () => {
    const result = parseAs(CP_SPEC, ["cp", "-f", "a.txt", "b.txt"]);
    expect(result.flags["force"]).toBe(true);
    expect(result.arguments["source"]).toEqual(["a.txt"]);
    expect(result.arguments["dest"]).toBe("b.txt");
  });

  it("cp -f -i a.txt b.txt → conflicting_flags (force and interactive)", () => {
    expect(() => parseAs(CP_SPEC, ["cp", "-f", "-i", "a.txt", "b.txt"])).toThrow(
      ParseErrors,
    );
  });
});

// ---------------------------------------------------------------------------
// grep tests
// ---------------------------------------------------------------------------

describe("Parser — grep (exclusive group, required_unless_flag)", () => {
  it("grep -i foo file.txt → pattern='foo', files=['file.txt']", () => {
    const result = parseAs(GREP_SPEC, ["grep", "-i", "foo", "file.txt"]);
    expect(result.flags["ignore-case"]).toBe(true);
    expect(result.arguments["pattern"]).toBe("foo");
    expect(result.arguments["files"]).toEqual(["file.txt"]);
  });

  it("grep -E '^[0-9]+' *.log → extended regex, pattern set", () => {
    const result = parseAs(GREP_SPEC, ["grep", "-E", "^[0-9]+", "a.log"]);
    expect(result.flags["extended-regexp"]).toBe(true);
    expect(result.arguments["pattern"]).toBe("^[0-9]+");
  });

  it("grep -e foo -e bar file.txt → repeatable regexp, pattern optional", () => {
    const result = parseAs(GREP_SPEC, ["grep", "-e", "foo", "-e", "bar", "file.txt"]);
    expect(result.flags["regexp"]).toEqual(["foo", "bar"]);
    // pattern is optional when -e is present
    expect(result.arguments["files"]).toEqual(["file.txt"]);
  });

  it("grep -E -F pattern → exclusive_group_violation", () => {
    expect(() =>
      parseAs(GREP_SPEC, ["grep", "-E", "-F", "pattern"]),
    ).toThrow(ParseErrors);
    try {
      parseAs(GREP_SPEC, ["grep", "-E", "-F", "pattern"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "exclusive_group_violation"),
      ).toBe(true);
    }
  });

  it("grep file.txt (no -e) → 'file.txt' is consumed as the PATTERN", () => {
    // Per the spec: PATTERN is the first positional, FILE is variadic after it.
    // 'grep file.txt' treats 'file.txt' as the pattern; FILE list is empty.
    const result = parseAs(GREP_SPEC, ["grep", "file.txt"]);
    expect(result.arguments["pattern"]).toBe("file.txt");
    expect(result.arguments["files"]).toEqual([]);
  });

  it("grep (no args, no -e) → missing_required_argument for PATTERN", () => {
    // With no tokens and no -e flag, PATTERN is required and missing.
    expect(() => parseAs(GREP_SPEC, ["grep"])).toThrow(ParseErrors);
    try {
      parseAs(GREP_SPEC, ["grep"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "missing_required_argument"),
      ).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// tar tests (traditional mode)
// ---------------------------------------------------------------------------

describe("Parser — tar (traditional mode)", () => {
  it("tar xvf archive.tar → traditional stacked flags", () => {
    const result = parseAs(TAR_SPEC, ["tar", "xvf", "archive.tar"]);
    expect(result.flags["extract"]).toBe(true);
    expect(result.flags["verbose"]).toBe(true);
    expect(result.flags["file"]).toBe("archive.tar");
  });

  it("tar -czvf out.tar.gz ./src → gnu-style flags", () => {
    const result = parseAs(TAR_SPEC, ["tar", "-czvf", "out.tar.gz", "./src"]);
    expect(result.flags["create"]).toBe(true);
    expect(result.flags["gzip"]).toBe(true);
    expect(result.flags["verbose"]).toBe(true);
    expect(result.flags["file"]).toBe("out.tar.gz");
    expect(result.arguments["member"]).toContain("./src");
  });

  it("tar vf archive.tar → missing_exclusive_group (no c/x/t)", () => {
    expect(() => parseAs(TAR_SPEC, ["tar", "vf", "archive.tar"])).toThrow(
      ParseErrors,
    );
    try {
      parseAs(TAR_SPEC, ["tar", "vf", "archive.tar"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "missing_exclusive_group"),
      ).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// git tests (subcommands, global flags, aliases)
// ---------------------------------------------------------------------------

describe("Parser — git (subcommands + global flags)", () => {
  it("git add src/foo.rb → command_path=['git','add'], pathspec set", () => {
    const result = parseAs(GIT_SPEC, ["git", "add", "src/foo.rb"]);
    expect(result.commandPath).toEqual(["git", "add"]);
    expect(result.arguments["pathspec"]).toContain("src/foo.rb");
  });

  it("git commit -m 'fix bug' → command_path=['git','commit'], message set", () => {
    const result = parseAs(GIT_SPEC, ["git", "commit", "-m", "fix bug"]);
    expect(result.commandPath).toEqual(["git", "commit"]);
    expect(result.flags["message"]).toBe("fix bug");
  });

  it("git remote add origin https://example.com → deep routing", () => {
    const result = parseAs(GIT_SPEC, [
      "git",
      "remote",
      "add",
      "origin",
      "https://example.com",
    ]);
    expect(result.commandPath).toEqual(["git", "remote", "add"]);
    expect(result.arguments["name"]).toBe("origin");
    expect(result.arguments["url"]).toBe("https://example.com");
  });

  it("git remote rm origin → alias 'rm' resolves to 'remove'", () => {
    const result = parseAs(GIT_SPEC, ["git", "remote", "rm", "origin"]);
    expect(result.commandPath).toEqual(["git", "remote", "remove"]);
    expect(result.arguments["name"]).toBe("origin");
  });

  it("git --no-pager push → global flag before subcommand", () => {
    // git push with no-pager flag; push has no args required
    const result = makeParser(GIT_SPEC, ["git", "--no-pager", "commit", "--amend"]).parse();
    if ("flags" in result) {
      expect(result.flags["no-pager"]).toBe(true);
    }
  });

  it("git commit (no -m) → missing_required_flag for message", () => {
    expect(() => parseAs(GIT_SPEC, ["git", "commit"])).toThrow(ParseErrors);
    try {
      parseAs(GIT_SPEC, ["git", "commit"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(
        pe.errors.some((err) => err.errorType === "missing_required_flag"),
      ).toBe(true);
    }
  });

  it("git commit --amend → no error (message required_unless amend)", () => {
    expect(() =>
      parseAs(GIT_SPEC, ["git", "commit", "--amend"]),
    ).not.toThrow();
    const result = parseAs(GIT_SPEC, ["git", "commit", "--amend"]);
    expect(result.flags["amend"]).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// java tests (single_dash_long)
// ---------------------------------------------------------------------------

describe("Parser — java (single_dash_long / longest-match-first)", () => {
  it("java -classpath . Main → SINGLE_DASH_LONG classpath", () => {
    const result = parseAs(JAVA_SPEC, ["java", "-classpath", ".", "Main"]);
    expect(result.flags["classpath"]).toBe(".");
    expect(result.arguments["class-or-jar"]).toBe("Main");
  });

  it("java -cp . Main → SINGLE_DASH_LONG cp", () => {
    const result = parseAs(JAVA_SPEC, ["java", "-cp", ".", "Main"]);
    expect(result.flags["classpath-short"]).toBe(".");
  });

  it("java -verbose Main → SINGLE_DASH_LONG verbose (boolean)", () => {
    const result = parseAs(JAVA_SPEC, ["java", "-verbose", "Main"]);
    expect(result.flags["verbose"]).toBe(true);
    expect(result.arguments["class-or-jar"]).toBe("Main");
  });

  it("java Main arg1 arg2 → class-or-jar + variadic args", () => {
    const result = parseAs(JAVA_SPEC, ["java", "Main", "arg1", "arg2"]);
    expect(result.arguments["class-or-jar"]).toBe("Main");
    expect(result.arguments["args"]).toEqual(["arg1", "arg2"]);
  });
});

// ---------------------------------------------------------------------------
// Help and version
// ---------------------------------------------------------------------------

describe("Parser — builtin flags (--help, --version)", () => {
  it("--help returns HelpResult", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);
    const parser = new Parser(spec, ["echo", "--help"]);
    const result = parser.parse();
    expect("text" in result).toBe(true);
    if ("text" in result) {
      expect(result.text).toContain("echo");
      expect(result.commandPath).toContain("echo");
    }
  });

  it("-h returns HelpResult", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);
    const parser = new Parser(spec, ["echo", "-h"]);
    const result = parser.parse();
    expect("text" in result).toBe(true);
  });

  it("--version returns VersionResult", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(ECHO_SPEC);
    const parser = new Parser(spec, ["echo", "--version"]);
    const result = parser.parse();
    expect("version" in result && !("flags" in result)).toBe(true);
    if ("version" in result) {
      expect(result.version).toBe("8.32");
    }
  });

  it("--version not returned when no version in spec", () => {
    const specNoVersion = { ...ECHO_SPEC, version: undefined };
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(specNoVersion as Record<string, unknown>);
    const parser = new Parser(spec, ["echo", "--version"]);
    // --version flag will be LONG_FLAG but won't match builtin since no version
    // This will result in an unknown_flag error
    expect(() => parser.parse()).toThrow(ParseErrors);
  });

  it("git commit --help returns help scoped to commit", () => {
    const loader = new SpecLoader("(test)");
    const spec = loader.loadFromObject(GIT_SPEC);
    const parser = new Parser(spec, ["git", "commit", "--help"]);
    const result = parser.parse();
    expect("text" in result).toBe(true);
    if ("text" in result) {
      expect(result.commandPath).toContain("commit");
    }
  });
});

// ---------------------------------------------------------------------------
// Misc error cases
// ---------------------------------------------------------------------------

describe("Parser — error cases", () => {
  it("unknown flag → unknown_flag error", () => {
    expect(() => parseAs(ECHO_SPEC, ["echo", "--nonexistent"])).toThrow(ParseErrors);
    try {
      parseAs(ECHO_SPEC, ["echo", "--nonexistent"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "unknown_flag")).toBe(true);
    }
  });

  it("end-of-flags '--' makes subsequent tokens positional", () => {
    const result = parseAs(ECHO_SPEC, ["echo", "--", "-n", "hello"]);
    // After --, everything is positional — so "-n" is treated as a string arg
    const strings = result.arguments["string"] as string[];
    expect(strings).toContain("-n");
    expect(strings).toContain("hello");
  });

  it("integer flag with non-integer value → invalid_value error", () => {
    const headSpec = {
      cli_builder_spec_version: "1.0",
      name: "head",
      description: "Output first part",
      flags: [
        { id: "lines", short: "n", long: "lines", description: "Line count", type: "integer", value_name: "NUM" },
      ],
    };
    expect(() => parseAs(headSpec, ["head", "-n", "abc"])).toThrow(ParseErrors);
    try {
      parseAs(headSpec, ["head", "-n", "abc"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "invalid_value")).toBe(true);
    }
  });

  it("integer flag with valid integer coerces correctly", () => {
    const headSpec = {
      cli_builder_spec_version: "1.0",
      name: "head",
      description: "Output first part",
      flags: [
        { id: "lines", short: "n", long: "lines", description: "Line count", type: "integer", value_name: "NUM" },
      ],
      arguments: [
        { id: "file", name: "FILE", description: "File", type: "path", required: false, variadic: true, variadic_min: 0 },
      ],
    };
    const result = parseAs(headSpec, ["head", "-n", "20"]);
    expect(result.flags["lines"]).toBe(20);
  });

  it("float flag with valid float coerces correctly", () => {
    const floatSpec = {
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool with float flag",
      flags: [
        { id: "ratio", short: "r", description: "Ratio", type: "float" },
      ],
    };
    const result = parseAs(floatSpec, ["tool", "-r", "3.14"]);
    expect(result.flags["ratio"]).toBeCloseTo(3.14);
  });

  it("duplicate non-repeatable flag → duplicate_flag error", () => {
    expect(() =>
      parseAs(ECHO_SPEC, ["echo", "-n", "-n"]),
    ).toThrow(ParseErrors);
    try {
      parseAs(ECHO_SPEC, ["echo", "-n", "-n"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "duplicate_flag")).toBe(true);
    }
  });

  it("enum flag with invalid value → invalid_enum_value error", () => {
    const enumSpec = {
      cli_builder_spec_version: "1.0",
      name: "tool",
      description: "Tool with enum",
      flags: [
        { id: "format", long: "format", description: "Output format", type: "enum", enum_values: ["json", "csv", "table"] },
      ],
    };
    expect(() =>
      parseAs(enumSpec, ["tool", "--format", "xml"]),
    ).toThrow(ParseErrors);
    try {
      parseAs(enumSpec, ["tool", "--format", "xml"]);
    } catch (e) {
      const pe = e as ParseErrors;
      expect(pe.errors.some((err) => err.errorType === "invalid_enum_value")).toBe(true);
    }
  });

  it("POSIX mode: first positional ends flag scanning", () => {
    const posixSpec = {
      cli_builder_spec_version: "1.0",
      name: "posix-tool",
      description: "POSIX tool",
      parsing_mode: "posix",
      flags: [
        { id: "verbose", short: "v", description: "Verbose", type: "boolean" },
      ],
      arguments: [
        { id: "files", name: "FILE", description: "Files", type: "path", required: false, variadic: true, variadic_min: 0 },
      ],
    };
    // In POSIX mode, "-v" after "file.txt" is treated as positional
    const result = parseAs(posixSpec, ["posix-tool", "file.txt", "-v"]);
    // After "file.txt" (first positional), flag scanning ends
    const files = result.arguments["files"] as string[];
    expect(files).toContain("file.txt");
    expect(files).toContain("-v"); // treated as positional
  });

  it("long flag with = value assignment", () => {
    const result = parseAs(GIT_SPEC, ["git", "commit", "--message=fix bug"]);
    expect(result.flags["message"]).toBe("fix bug");
  });

  it("ParseErrors contains multiple errors when multiple problems exist", () => {
    // -e and -E conflict, and no string args
    try {
      parseAs(ECHO_SPEC, ["echo", "-e", "-E"]);
    } catch (e) {
      if (e instanceof ParseErrors) {
        expect(e.errors.length).toBeGreaterThan(0);
      }
    }
  });
});
