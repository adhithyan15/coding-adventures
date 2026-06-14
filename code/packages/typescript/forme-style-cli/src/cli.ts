/**
 * cli.ts — `forme-style` CLI (FM03 §5).
 *
 * One exported function: `run(argv, io): Promise<number>`.  Returns
 * the process exit code rather than calling `process.exit` directly
 * so tests can exercise the CLI without spawning subprocesses.
 *
 * ## CLI surface
 *
 *   forme-style <doc.json> --target css|latex|terminal
 *               [--theme NAME]
 *               [--themes themes.json]
 *               [--active CTX,CTX,...]
 *               [--used ID,ID,...]
 *               [--scope STR]
 *               [--out FILE]
 *               [--help]
 *
 * Use `-` as `<doc.json>` (or omit `--out`) to read from stdin /
 * write to stdout.
 *
 * ## Exit codes
 *
 *   0 — success
 *   1 — validator failure (errors printed to stderr)
 *   2 — file I/O or argument-parse error
 *   3 — unknown target (also caught by arg parser, but defensive)
 *
 * Output goes to `io.stdout`; diagnostic and error messages go to
 * `io.stderr`.  Both injectable for testability.
 *
 * @module cli
 */

import {
  compile, isCompileError,
  type CompileTarget, type CompileOptions, type CompileResult,
} from "@coding-adventures/forme-style-orchestrator";
import {
  createThemeRegistry,
  type ThemeRegistry,
} from "@coding-adventures/forme-style-theme";
import type { StyleRuleId, Theme } from "@coding-adventures/forme-style-ir";

// ─── IO injection point ──────────────────────────────────────────────────

/**
 * Side-effect surface the CLI uses.  In production wired to
 * `process.stdout` / `process.stderr` / `node:fs` / `process.stdin`;
 * in tests wired to in-memory buffers.
 */
export interface CliIO {
  readonly stdout: { write(s: string): void };
  readonly stderr: { write(s: string): void };
  readFile(path: string): Promise<string>;
  writeFile(path: string, contents: string): Promise<void>;
  readStdin(): Promise<string>;
}

// ─── Exit code constants ─────────────────────────────────────────────────

export const EXIT_OK              = 0;
export const EXIT_VALIDATOR_FAIL  = 1;
export const EXIT_IO_OR_ARG_ERROR = 2;
export const EXIT_UNKNOWN_TARGET  = 3;

// ─── Public entry point ──────────────────────────────────────────────────

/**
 * Run the CLI.  `argv` is the *user* arguments (no node / script
 * name); pass `process.argv.slice(2)` from the shim.
 */
export async function run(argv: readonly string[], io: CliIO): Promise<number> {
  // ─── 1. Parse arguments ──────────────────────────────────────────────
  let parsed: ParsedArgs;
  try {
    parsed = parseArgs(argv);
  } catch (e) {
    io.stderr.write(`forme-style: ${(e as Error).message}\n`);
    io.stderr.write(`Run \`forme-style --help\` for usage.\n`);
    return EXIT_IO_OR_ARG_ERROR;
  }

  if (parsed.help) {
    io.stdout.write(HELP_TEXT);
    return EXIT_OK;
  }

  // ─── 2. Read input document ──────────────────────────────────────────
  let docJson: string;
  try {
    docJson = parsed.input === "-"
      ? await io.readStdin()
      : await io.readFile(parsed.input);
  } catch (e) {
    io.stderr.write(`forme-style: failed to read input ${JSON.stringify(parsed.input)}: ${(e as Error).message}\n`);
    return EXIT_IO_OR_ARG_ERROR;
  }

  let doc: unknown;
  try {
    doc = JSON.parse(docJson);
  } catch (e) {
    io.stderr.write(`forme-style: input is not valid JSON: ${(e as Error).message}\n`);
    return EXIT_IO_OR_ARG_ERROR;
  }

  // ─── 3. Load themes (if --themes supplied) ───────────────────────────
  let themeRegistry: ThemeRegistry | undefined;
  if (parsed.themesPath !== undefined) {
    try {
      const txt = await io.readFile(parsed.themesPath);
      const themesDoc: unknown = JSON.parse(txt);
      themeRegistry = loadThemes(themesDoc);
    } catch (e) {
      io.stderr.write(`forme-style: failed to load --themes ${JSON.stringify(parsed.themesPath)}: ${(e as Error).message}\n`);
      return EXIT_IO_OR_ARG_ERROR;
    }
  }

  // ─── 4. Build compile options ────────────────────────────────────────
  const options: CompileOptions = {
    activeContexts: parsed.activeContexts,
    ...(parsed.usedRuleIds !== undefined ? { usedRuleIds: parsed.usedRuleIds } : {}),
    ...(parsed.scope        !== undefined ? { scope: parsed.scope }              : {}),
    ...(parsed.themeName    !== undefined ? { theme: parsed.themeName }          : {}),
    ...(themeRegistry       !== undefined ? { themeRegistry }                    : {}),
  };

  // ─── 5. Dispatch ─────────────────────────────────────────────────────
  let result: CompileResult;
  try {
    result = compile(doc, parsed.target, options);
  } catch (e) {
    // The orchestrator throws TypeError for unknown target (caught
    // upstream by parseArgs) and for theme-name-without-registry.
    // The arg parser also catches both — anything reaching here is a
    // genuine programmer bug, but we still want a clean exit rather
    // than a stack trace dumped to stderr.
    io.stderr.write(`forme-style: ${(e as Error).message}\n`);
    return EXIT_IO_OR_ARG_ERROR;
  }

  // ─── 6. Report warnings (always) ─────────────────────────────────────
  for (const w of result.warnings) {
    io.stderr.write(`warning [${w.code}]: ${w.message}\n`);
  }

  // ─── 7. Validator failure branch ─────────────────────────────────────
  if (isCompileError(result)) {
    io.stderr.write(`forme-style: validator rejected the input (${result.errors.length} errors):\n`);
    for (const e of result.errors) {
      const at = e.path.length > 0 ? ` at ${e.path}` : "";
      io.stderr.write(`  [${e.code}]${at}: ${e.message}\n`);
    }
    return EXIT_VALIDATOR_FAIL;
  }

  // ─── 8. Write output ─────────────────────────────────────────────────
  try {
    if (parsed.outPath !== undefined) {
      await io.writeFile(parsed.outPath, result.output);
    } else {
      io.stdout.write(result.output);
      // For terminal output we end with newline so the shell prompt
      // doesn't share a line with the last bytes.
      if (parsed.target === "terminal" || !result.output.endsWith("\n")) {
        io.stdout.write("\n");
      }
    }
  } catch (e) {
    io.stderr.write(`forme-style: failed to write output: ${(e as Error).message}\n`);
    return EXIT_IO_OR_ARG_ERROR;
  }

  return EXIT_OK;
}

// ─── Help text ───────────────────────────────────────────────────────────

const HELP_TEXT = `forme-style — translate a Forme Style IR document to CSS / LaTeX / terminal ANSI.

Usage:
  forme-style <doc.json> --target <css|latex|terminal> [options]
  forme-style - --target <css|latex|terminal> [options]     # read from stdin
  forme-style --help

Options:
  --target <css|latex|terminal>   Required.  Backend translator to dispatch to.
  --theme <name>                  Apply theme by name (requires --themes).
  --themes <themes.json>          Load a theme registry from a JSON file shaped
                                  { "themes": [ { name, ... }, ... ] }.
  --active <ctx,ctx,...>          Comma-separated list of active contexts
                                  (default: empty).
  --used <id,id,...>              Per-page slice: emit only the listed rule ids.
  --scope <string>                Caller-trusted scope prefix passed to the
                                  translator (caller must escape).
  --out <file>                    Write output to file (default: stdout).
  --help                          Show this help and exit.

Exit codes:
  0   success
  1   validator failure
  2   file I/O or argument-parse error
  3   unknown target (should be caught by arg parser)
`;

// ─── Arg parser ──────────────────────────────────────────────────────────

interface ParsedArgs {
  readonly help: boolean;
  readonly input: string;                          // doc path or "-"
  readonly target: CompileTarget;
  readonly themeName?: string;
  readonly themesPath?: string;
  readonly activeContexts: readonly string[];
  readonly usedRuleIds?: readonly StyleRuleId[];
  readonly scope?: string;
  readonly outPath?: string;
}

const ALLOWED_TARGETS: ReadonlySet<CompileTarget> = new Set(["css", "latex", "terminal"]);

function parseArgs(argv: readonly string[]): ParsedArgs {
  if (argv.length === 0 || argv.includes("--help") || argv.includes("-h")) {
    return blankParsed({ help: true });
  }

  let input: string | undefined;
  let target: string | undefined;
  let themeName: string | undefined;
  let themesPath: string | undefined;
  let activeContexts: readonly string[] = [];
  let usedRuleIds: readonly StyleRuleId[] | undefined;
  let scope: string | undefined;
  let outPath: string | undefined;

  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]!;
    switch (a) {
      case "--target":  target        = takeValue(argv, i++, a); break;
      case "--theme":   themeName     = takeValue(argv, i++, a); break;
      case "--themes":  themesPath    = takeValue(argv, i++, a); break;
      case "--active":  activeContexts = takeValue(argv, i++, a).split(",").map((s) => s.trim()).filter((s) => s.length > 0); break;
      case "--used":    usedRuleIds   = takeValue(argv, i++, a).split(",").map((s) => s.trim()).filter((s) => s.length > 0) as unknown as readonly StyleRuleId[]; break;
      case "--scope":   scope         = takeValue(argv, i++, a); break;
      case "--out":     outPath       = takeValue(argv, i++, a); break;
      default:
        if (a.startsWith("--")) throw new Error(`unknown flag ${JSON.stringify(a)}`);
        if (input !== undefined) throw new Error(`too many positional arguments (already have ${JSON.stringify(input)}; got ${JSON.stringify(a)})`);
        input = a;
        break;
    }
  }

  if (input === undefined) throw new Error("missing input document path (use \"-\" for stdin)");
  if (target === undefined) throw new Error("missing required --target");
  if (!ALLOWED_TARGETS.has(target as CompileTarget)) {
    throw new Error(`unknown --target ${JSON.stringify(target)}; expected one of: ${[...ALLOWED_TARGETS].join(", ")}`);
  }
  if (themeName !== undefined && themesPath === undefined) {
    throw new Error(`--theme ${JSON.stringify(themeName)} requires --themes to load the registry`);
  }

  return {
    help: false,
    input,
    target: target as CompileTarget,
    ...(themeName     !== undefined ? { themeName }     : {}),
    ...(themesPath    !== undefined ? { themesPath }    : {}),
    activeContexts,
    ...(usedRuleIds   !== undefined ? { usedRuleIds }   : {}),
    ...(scope         !== undefined ? { scope }         : {}),
    ...(outPath       !== undefined ? { outPath }       : {}),
  };
}

function takeValue(argv: readonly string[], i: number, flag: string): string {
  const v = argv[i + 1];
  if (v === undefined || v.startsWith("--")) {
    throw new Error(`flag ${JSON.stringify(flag)} requires a value`);
  }
  return v;
}

function blankParsed(overrides: Partial<ParsedArgs>): ParsedArgs {
  return {
    help: false,
    input: "",
    target: "css",
    activeContexts: [],
    ...overrides,
  };
}

// ─── Theme loader ────────────────────────────────────────────────────────

/**
 * Construct a `ThemeRegistry` from an arbitrary `{ themes: [...] }`
 * payload.  Defensive: any malformed theme entry is skipped with a
 * warning rather than throwing.
 *
 * Throws only when the top-level shape is wrong (`themes` not an
 * array, payload not an object).  Caller treats throw as
 * EXIT_IO_OR_ARG_ERROR.
 */
function loadThemes(payload: unknown): ThemeRegistry {
  if (typeof payload !== "object" || payload === null) {
    throw new Error(`themes file must be a JSON object with a "themes" array`);
  }
  const themes = (payload as { themes?: unknown }).themes;
  if (!Array.isArray(themes)) {
    throw new Error(`themes file must have a "themes" array at the top level`);
  }
  const reg = createThemeRegistry();
  for (const t of themes) {
    if (typeof t !== "object" || t === null) continue;
    const name = (t as { name?: unknown }).name;
    if (typeof name !== "string" || name.length === 0) continue;
    // The registry's `register` does its own defensive checks
    // (forbidden names, empty names) — we just hand it the value.
    try {
      reg.register(t as Theme);
    } catch {
      // skip malformed
    }
  }
  return reg;
}
