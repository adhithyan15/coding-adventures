/**
 * starlark-evaluator.ts -- Starlark BUILD File Evaluator
 * =======================================================
 *
 * This module bridges the Starlark interpreter and the build tool. It
 * evaluates Starlark BUILD files (as opposed to plain shell BUILD files)
 * and extracts the declared build targets.
 *
 * ==========================================================================
 * Chapter 1: Why Starlark BUILD Files?
 * ==========================================================================
 *
 * Traditional BUILD files in this monorepo are shell scripts -- each line
 * is a command executed sequentially. This works but has limitations:
 *
 *   - **No change detection metadata**: the build tool guesses which files
 *     matter based on file extensions, not explicit declarations.
 *   - **No dependency declarations**: deps are parsed from language-specific
 *     config files (pyproject.toml, go.mod, etc.) with heuristic matching.
 *   - **No validation**: a typo in a BUILD file only surfaces at build time.
 *
 * Starlark BUILD files solve all three. They are real programs that declare
 * targets with explicit ``srcs``, ``deps``, and build metadata. The build
 * tool evaluates them using its built-in Starlark interpreter and extracts
 * the declared targets.
 *
 * ==========================================================================
 * Chapter 2: How Evaluation Works
 * ==========================================================================
 *
 * The evaluation pipeline has five steps:
 *
 *   1. **Read** the BUILD file contents from disk.
 *   2. **Create** a Starlark interpreter with:
 *      - A file resolver rooted at the repo root (for ``load()`` statements).
 *      - Configuration for the compiler and VM pipeline.
 *   3. **Execute** the BUILD file through the interpreter pipeline:
 *      ``source -> lexer -> parser -> compiler -> VM -> result``
 *   4. **Extract** the ``_targets`` list from the result's variables.
 *   5. **Convert** each target dict to a structured Target object.
 *
 * The ``_targets`` list is a convention: rule functions like ``py_library()``
 * append a dict to the global ``_targets`` list. After evaluation, we read
 * that list and convert each dict into a Target.
 *
 * ==========================================================================
 * Chapter 3: Detecting Starlark vs Shell BUILD Files
 * ==========================================================================
 *
 * We use a simple heuristic: look at the first non-comment, non-blank line.
 * If it starts with ``load(`` or matches a known rule call pattern like
 * ``py_library(``, the file is Starlark. Otherwise it is shell.
 *
 * This heuristic is fast (only examines the first significant line) and
 * accurate (shell commands never start with ``load(`` or ``py_library(``).
 *
 * Truth table:
 *
 * | First significant line         | Classification |
 * |-------------------------------|----------------|
 * | ``load("rules.star", ...)``   | Starlark       |
 * | ``py_library(name = ...)``    | Starlark       |
 * | ``def my_rule():``            | Starlark       |
 * | ``npm install --silent``      | Shell          |
 * | ``uv pip install -e .``       | Shell          |
 * | (empty file)                  | Shell          |
 *
 * ==========================================================================
 * Chapter 4: Command Generation
 * ==========================================================================
 *
 * Once we have structured targets, we convert each rule type to the shell
 * commands the executor knows how to run. This mapping is the same as in
 * the Go implementation:
 *
 *   - ``py_library``  -> ``uv pip install`` + ``pytest``
 *   - ``go_library``  -> ``go build`` + ``go test`` + ``go vet``
 *   - ``ruby_library`` -> ``bundle install`` + ``rake test``
 *   - ``ts_library``  -> ``npm install`` + ``vitest``
 *   - ``rust_library`` -> ``cargo build`` + ``cargo test``
 *   - ``elixir_library`` -> ``mix deps.get`` + ``mix test``
 *
 * @module
 */

import * as fs from "node:fs";
import { createRequire } from "node:module";
import * as os from "node:os";
import * as path from "node:path";

// The starlark-interpreter types are defined inline (not imported) so this
// module loads even when the interpreter package isn't installed. The actual
// class is required dynamically inside evaluateBuildFile().
type FileResolverFn = (label: string) => string;
type StarlarkResult = { variables: Record<string, unknown> };

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * A single build target declared in a Starlark BUILD file.
 *
 * Each call to ``py_library()``, ``go_library()``, etc. produces one Target.
 * The target holds all the metadata the build tool needs: what rule to use,
 * what sources to watch, what dependencies to resolve, and how to test.
 *
 * @property rule       - Rule type: "py_library", "go_binary", etc.
 * @property name       - Target name: "starlark-vm", "build-tool", etc.
 * @property srcs       - Declared source file patterns for change detection.
 * @property deps       - Dependencies as "language/package-name" strings.
 * @property testRunner - Test framework: "pytest", "vitest", "minitest", etc.
 * @property entryPoint - Binary entry point: "main.py", "src/index.ts", etc.
 */
/** Schema version for the _ctx build context dict. */
const CTX_SCHEMA_VERSION = 1;

/** OS name normalization: os.platform() -> runtime.GOOS equivalents. */
const OS_MAP: Record<string, string> = {
  darwin: "darwin",
  linux: "linux",
  win32: "windows",
};

export interface Target {
  rule: string;
  name: string;
  srcs: string[];
  deps: string[];
  testRunner: string;
  entryPoint: string;
  commands: Record<string, unknown>[];
}

/**
 * The result of evaluating a Starlark BUILD file.
 *
 * Contains all the targets declared in the file. A BUILD file with no
 * rule calls produces an empty targets array (this is valid -- a file
 * might only define helper functions for other BUILD files to load).
 */
export interface BuildResult {
  targets: Target[];
}

// ---------------------------------------------------------------------------
// Known Rules -- the Starlark indicators
// ---------------------------------------------------------------------------

/**
 * Known Starlark rule function prefixes.
 *
 * These are the rule functions that the build tool recognizes. When we
 * see a line starting with one of these in a BUILD file, we know it is
 * Starlark (not shell). The list covers all six supported languages,
 * each with a ``_library`` and ``_binary`` variant.
 */
const KNOWN_RULES: readonly string[] = [
  "py_library(",
  "py_binary(",
  "go_library(",
  "go_binary(",
  "ruby_library(",
  "ruby_binary(",
  "ts_library(",
  "ts_binary(",
  "rust_library(",
  "rust_binary(",
  "elixir_library(",
  "elixir_binary(",
];

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/**
 * Check whether a BUILD file contains Starlark code (vs shell commands).
 *
 * Scans the file content line by line, skipping blanks and comments.
 * The first significant line determines the classification:
 *
 *   - Starts with ``load(`` -> Starlark (importing rules)
 *   - Starts with ``def `` -> Starlark (function definition)
 *   - Starts with a known rule call -> Starlark (e.g., ``py_library(``)
 *   - Anything else -> Shell
 *
 * If the file is empty or contains only comments, it is classified as
 * shell (the executor will simply have no commands to run).
 *
 * @param content - The raw text content of the BUILD file.
 * @returns ``true`` if the content is Starlark, ``false`` if shell.
 *
 * @example
 * ```typescript
 * isStarlarkBuild('load("rules.star", "py_library")\n');  // true
 * isStarlarkBuild('npm install --silent\n');                // false
 * ```
 */
export function isStarlarkBuild(content: string): boolean {
  for (const line of content.split("\n")) {
    const trimmed = line.trim();

    // Skip blank lines and comments -- they tell us nothing about format.
    if (trimmed === "" || trimmed.startsWith("#")) {
      continue;
    }

    // Check for Starlark patterns on the first significant line.
    if (trimmed.startsWith("load(")) {
      return true;
    }
    if (trimmed.startsWith("def ")) {
      return true;
    }
    for (const rule of KNOWN_RULES) {
      if (trimmed.startsWith(rule)) {
        return true;
      }
    }

    // If we reach here, the first significant line does not match any
    // Starlark pattern. It is almost certainly a shell command.
    break;
  }

  return false;
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/**
 * Evaluate a Starlark BUILD file and extract its declared targets.
 *
 * This is the main entry point for Starlark evaluation. It:
 *
 *   1. Reads the BUILD file from disk.
 *   2. Creates a file resolver rooted at ``repoRoot`` so that
 *      ``load("code/packages/starlark/rules/python.star", "py_library")``
 *      resolves to ``<repoRoot>/code/packages/starlark/rules/python.star``.
 *   3. Creates a ``StarlarkInterpreter`` with the resolver.
 *   4. Executes the BUILD file through the interpreter pipeline.
 *   5. Extracts ``_targets`` from the result variables.
 *
 * @param buildFilePath - Absolute path to the BUILD file.
 * @param _pkgDir       - Absolute path to the package directory (for future
 *                        glob() support -- currently unused).
 * @param repoRoot      - Absolute path to the repository root directory.
 * @returns A BuildResult with the extracted targets.
 * @throws {Error} If the file cannot be read or the interpreter fails.
 *
 * @example
 * ```typescript
 * const result = evaluateBuildFile(
 *   "/repo/code/packages/python/logic-gates/BUILD",
 *   "/repo/code/packages/python/logic-gates",
 *   "/repo",
 * );
 * console.log(result.targets[0].rule); // "py_library"
 * ```
 */
export function evaluateBuildFile(
  buildFilePath: string,
  _pkgDir: string,
  repoRoot: string,
): BuildResult {
  // Step 1: Read the BUILD file.
  const content = fs.readFileSync(buildFilePath, "utf-8");

  // Step 2: Create a file resolver.
  //
  // The resolver maps load() labels to file contents. Labels are
  // filesystem paths relative to the repo root:
  //
  //   load("code/packages/starlark/rules/python.star", "py_library")
  //
  // resolves to <repoRoot>/code/packages/starlark/rules/python.star
  const fileResolver: FileResolverFn = (label: string): string => {
    const fullPath = path.join(repoRoot, label);
    try {
      return fs.readFileSync(fullPath, "utf-8");
    } catch (err) {
      throw new Error(
        `load("${label}"): ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  };

  // Step 3: Create the interpreter with the file resolver.
  //
  // We dynamically require the starlark-interpreter package here rather
  // than importing it at the top level. This keeps the evaluator module
  // loadable even when the interpreter package isn't installed (the build
  // tool falls back to shell BUILD files in that case).
  const req = createRequire(import.meta.url);
  const { StarlarkInterpreter: Interpreter } = req(
    "@coding-adventures/starlark-interpreter",
  );
  // Build the _ctx dict — build context injected into every Starlark scope.
  const platform = os.platform();
  const ctxDict = {
    version: CTX_SCHEMA_VERSION,
    os: OS_MAP[platform] ?? platform,
    arch: os.arch(),
    cpu_count: os.cpus().length,
    ci: (process.env.CI ?? "") !== "",
    repo_root: repoRoot,
  };

  const interp = new Interpreter({
    fileResolver,
    globals: { _ctx: ctxDict },
  });

  // Step 4: Execute the BUILD file.
  let result: StarlarkResult;
  try {
    result = interp.interpret(content);
  } catch (err) {
    throw new Error(
      `Evaluating BUILD file ${buildFilePath}: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  // Step 5: Extract targets from the result's variables.
  const targets = extractTargets(result.variables);

  return { targets };
}

// ---------------------------------------------------------------------------
// Target Extraction
// ---------------------------------------------------------------------------

/**
 * Extract targets from the ``_targets`` variable in the Starlark result.
 *
 * The convention is that rule functions (``py_library``, ``go_binary``, etc.)
 * append a dict to a global ``_targets`` list. Each dict has these keys:
 *
 *   - ``rule``: string -- the rule type (e.g., "py_library")
 *   - ``name``: string -- the target name
 *   - ``srcs``: string[] -- source file patterns
 *   - ``deps``: string[] -- dependency labels
 *   - ``test_runner``: string (optional) -- test framework name
 *   - ``entry_point``: string (optional) -- binary entry point
 *
 * If ``_targets`` does not exist in the variables, we return an empty
 * array. This is valid -- a BUILD file might only define helpers.
 *
 * @param variables - The variable map from the Starlark execution result.
 * @returns An array of Target objects.
 */
function extractTargets(variables: Record<string, unknown>): Target[] {
  const rawTargets = variables["_targets"];

  // No _targets variable -- the BUILD file declared no targets.
  if (rawTargets === undefined || rawTargets === null) {
    return [];
  }

  // _targets must be an array (list in Starlark).
  if (!Array.isArray(rawTargets)) {
    throw new Error(
      `_targets is not a list (got ${typeof rawTargets})`,
    );
  }

  const targets: Target[] = [];

  for (let i = 0; i < rawTargets.length; i++) {
    const raw = rawTargets[i];

    // Each element must be a dict (object in JavaScript).
    if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
      throw new Error(
        `_targets[${i}] is not a dict (got ${typeof raw})`,
      );
    }

    const dict = raw as Record<string, unknown>;

    targets.push({
      rule: getString(dict, "rule"),
      name: getString(dict, "name"),
      srcs: getStringList(dict, "srcs"),
      deps: getStringList(dict, "deps"),
      testRunner: getString(dict, "test_runner"),
      entryPoint: getString(dict, "entry_point"),
      commands: getDictList(dict, "commands"),
    });
  }

  return targets;
}

/**
 * Safely extract a string value from a dict.
 *
 * Returns ``""`` if the key does not exist or is not a string. This
 * defensive approach means a malformed target dict does not crash the
 * build tool -- it just produces empty fields that the reporter can flag.
 *
 * @param dict - The object to extract from.
 * @param key  - The key to look up.
 * @returns The string value, or ``""`` if missing/wrong type.
 */
function getString(dict: Record<string, unknown>, key: string): string {
  const v = dict[key];
  if (typeof v === "string") {
    return v;
  }
  return "";
}

/**
 * Safely extract a string array from a dict.
 *
 * Returns ``[]`` if the key does not exist or is not an array. Non-string
 * elements in the array are silently dropped. This mirrors the Go
 * implementation's ``getStringList`` helper.
 *
 * @param dict - The object to extract from.
 * @param key  - The key to look up.
 * @returns An array of strings, or ``[]`` if missing/wrong type.
 */
function getStringList(dict: Record<string, unknown>, key: string): string[] {
  const v = dict[key];
  if (!Array.isArray(v)) {
    return [];
  }
  const result: string[] = [];
  for (const item of v) {
    if (typeof item === "string") {
      result.push(item);
    }
  }
  return result;
}

// ---------------------------------------------------------------------------
// Command Generation
// ---------------------------------------------------------------------------

/**
 * Convert a Target into shell commands the executor can run.
 *
 * This is the bridge between Starlark declarations and actual build/test
 * commands. Each rule type maps to a standard set of commands that match
 * the language ecosystem's conventions:
 *
 * | Rule             | Commands                                          |
 * |------------------|---------------------------------------------------|
 * | py_library       | uv pip install + pytest                           |
 * | py_binary        | uv pip install + pytest                           |
 * | go_library/bin   | go build + go test + go vet                       |
 * | ruby_library/bin | bundle install + rake test                        |
 * | ts_library/bin   | npm install + vitest                              |
 * | rust_library/bin | cargo build + cargo test                          |
 * | elixir_lib/bin   | mix deps.get + mix test                           |
 *
 * For ``py_library``, the test runner defaults to ``pytest`` but can be
 * overridden via the ``test_runner`` field (e.g., ``"unittest"``).
 *
 * @param target - The target to generate commands for.
 * @returns An array of shell command strings.
 *
 * @example
 * ```typescript
 * const cmds = generateCommands({
 *   rule: "py_library",
 *   name: "logic-gates",
 *   srcs: ["src/foo.py"],
 *   deps: [],
 *   testRunner: "",
 *   entryPoint: "",
 * });
 * // ["uv pip install --system -e \".[dev]\"",
 * //  "python -m pytest --cov --cov-report=term-missing"]
 * ```
 */
export function generateCommands(target: Target): string[] {
  switch (target.rule) {
    // -----------------------------------------------------------------------
    // Python rules
    // -----------------------------------------------------------------------
    case "py_library": {
      const runner = target.testRunner || "pytest";
      if (runner === "pytest") {
        return [
          `uv pip install --system -e ".[dev]"`,
          "python -m pytest --cov --cov-report=term-missing",
        ];
      }
      return [
        `uv pip install --system -e ".[dev]"`,
        "python -m unittest discover tests/",
      ];
    }

    case "py_binary":
      return [
        `uv pip install --system -e ".[dev]"`,
        "python -m pytest --cov --cov-report=term-missing",
      ];

    // -----------------------------------------------------------------------
    // Go rules
    // -----------------------------------------------------------------------
    case "go_library":
    case "go_binary":
      return [
        "go build ./...",
        "go test ./... -v -cover",
        "go vet ./...",
      ];

    // -----------------------------------------------------------------------
    // Ruby rules
    // -----------------------------------------------------------------------
    case "ruby_library":
    case "ruby_binary":
      return [
        "bundle install --quiet",
        "bundle exec rake test",
      ];

    // -----------------------------------------------------------------------
    // TypeScript rules
    // -----------------------------------------------------------------------
    case "ts_library":
    case "ts_binary":
      return [
        "npm install --silent",
        "npx vitest run --coverage",
      ];

    // -----------------------------------------------------------------------
    // Rust rules
    // -----------------------------------------------------------------------
    case "rust_library":
    case "rust_binary":
      return [
        "cargo build",
        "cargo test",
      ];

    // -----------------------------------------------------------------------
    // Elixir rules
    // -----------------------------------------------------------------------
    case "elixir_library":
    case "elixir_binary":
      return [
        "mix deps.get",
        "mix test --cover",
      ];

    // -----------------------------------------------------------------------
    // Unknown rule -- produce a diagnostic command
    // -----------------------------------------------------------------------
    default:
      return [`echo 'Unknown rule: ${target.rule}'`];
  }
}

// ---------------------------------------------------------------------------
// Dict List Extraction
// ---------------------------------------------------------------------------

function getDictList(
  dict: Record<string, unknown>,
  key: string,
): Record<string, unknown>[] {
  const v = dict[key];
  if (!Array.isArray(v)) {
    return [];
  }
  const result: Record<string, unknown>[] = [];
  for (const item of v) {
    if (typeof item === "object" && item !== null && !Array.isArray(item)) {
      result.push(item as Record<string, unknown>);
    }
  }
  return result;
}

// ---------------------------------------------------------------------------
// Command Rendering
// ---------------------------------------------------------------------------

const SHELL_META = new Set(` \t"'$\`\\|&;()<>!#*?[]{}`);

function needsQuoting(arg: string): boolean {
  for (const ch of arg) {
    if (SHELL_META.has(ch)) return true;
  }
  return false;
}

function quoteArg(arg: string): string {
  if (arg === "") return '""';
  if (!needsQuoting(arg)) return arg;
  const escaped = arg.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  return `"${escaped}"`;
}

export function renderCommand(cmdDict: Record<string, unknown>): string {
  const program = cmdDict["program"];
  if (typeof program !== "string" || program === "") {
    throw new Error(`command dict missing 'program' key: ${JSON.stringify(cmdDict)}`);
  }
  const parts = [quoteArg(program)];
  const args = cmdDict["args"];
  if (Array.isArray(args)) {
    for (const arg of args) {
      parts.push(quoteArg(String(arg)));
    }
  }
  return parts.join(" ");
}

export function renderCommands(cmds: unknown[]): string[] {
  const result: string[] = [];
  for (const cmd of cmds) {
    if (cmd == null) continue;
    if (typeof cmd === "object" && !Array.isArray(cmd)) {
      result.push(renderCommand(cmd as Record<string, unknown>));
    }
  }
  return result;
}
