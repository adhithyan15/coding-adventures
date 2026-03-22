/**
 * types.ts — Core type definitions for the CLI Builder library.
 *
 * === Type Architecture ===
 *
 * This module defines two categories of types:
 *
 * 1. **Spec types** — The internal representation of a parsed and validated
 *    JSON spec. These are richer than the raw JSON: defaults are filled in,
 *    optional arrays are never undefined, and cross-references are resolved.
 *
 * 2. **Result types** — What the parser returns to the caller. A successful
 *    parse produces one of three result shapes depending on what was requested:
 *    `ParseResult` (normal invocation), `HelpResult` (--help), or
 *    `VersionResult` (--version).
 *
 * === Why separate spec types from the JSON schema? ===
 *
 * The JSON spec allows many optional fields with complex defaults. Passing raw
 * `Record<string, unknown>` through the parsing logic would require constant
 * null checks and type assertions. Instead, SpecLoader validates the JSON
 * once and converts it into well-typed internal objects. The rest of the
 * library operates on these clean types.
 *
 * @module types
 */

// ---------------------------------------------------------------------------
// Spec types — internal representation of a validated spec
// ---------------------------------------------------------------------------

/**
 * Valid value types for flags and arguments.
 *
 * These correspond to §3 of the spec. Each type determines:
 * - How the raw string value is coerced (e.g., "42" → 42 for integer)
 * - What validation is applied (e.g., path existence checks for `file`)
 * - What the resulting TypeScript type is in ParseResult.arguments
 */
export type ValueType =
  | "boolean"
  | "string"
  | "integer"
  | "float"
  | "path"
  | "file"
  | "directory"
  | "enum";

/**
 * Valid parsing modes (§2.1).
 *
 * - `gnu`: flags may appear anywhere, `--` ends flag scanning (default)
 * - `posix`: first non-flag token ends flag scanning
 * - `subcommand_first`: first non-flag is always a subcommand, never positional
 * - `traditional`: first token without `-` may be stacked flags (tar-style)
 */
export type ParsingMode = "gnu" | "posix" | "subcommand_first" | "traditional";

/**
 * Internal representation of a flag definition (§2.2).
 *
 * All optional fields from the JSON spec are normalized here:
 * - `required` defaults to `false`
 * - `repeatable` defaults to `false`
 * - Arrays default to `[]`
 * - `default` is `null` if absent
 */
export interface FlagDef {
  /** Unique identifier within the scope. Used as the key in ParseResult.flags. */
  id: string;
  /** Single-character short flag (without `-`). */
  short?: string;
  /** Long flag name (without `--`). */
  long?: string;
  /** Single-dash multi-character name (without `-`), e.g., "classpath". */
  singleDashLong?: string;
  /** Human-readable description for help output. */
  description: string;
  /** The value type — determines coercion and validation. */
  type: ValueType;
  /** Whether this flag must be present. */
  required: boolean;
  /** Default value when absent and not required. */
  default: unknown;
  /** Display name for the value in help text (e.g., "FILE", "PATH"). */
  valueName?: string;
  /** Valid values when type is "enum". */
  enumValues: string[];
  /** IDs of flags that cannot be used alongside this one. */
  conflictsWith: string[];
  /** IDs of flags that must also be present when this flag is used. */
  requires: string[];
  /** This flag is required unless at least one of these IDs is present. */
  requiredUnless: string[];
  /** If true, flag may appear multiple times; result is an array. */
  repeatable: boolean;
}

/**
 * Internal representation of a positional argument definition (§2.3).
 */
export interface ArgDef {
  /** Unique identifier within the scope. Used as the key in ParseResult.arguments. */
  id: string;
  /** Display name in help text (e.g., "FILE", "DEST"). Accepts display_name (preferred) or name (backward compat). */
  display_name: string;
  /** Human-readable description. */
  description: string;
  /** The value type. */
  type: ValueType;
  /** Whether at least one value must be provided. */
  required: boolean;
  /** Whether multiple values may be provided. */
  variadic: boolean;
  /** Minimum count when variadic. */
  variadicMin: number;
  /** Maximum count when variadic (null = unlimited). */
  variadicMax: number | null;
  /** Default value when absent. */
  default: unknown;
  /** Valid values when type is "enum". */
  enumValues: string[];
  /** This argument is optional if any of these flag IDs is present. */
  requiredUnlessFlag: string[];
}

/**
 * A mutually exclusive group of flags (§2.5).
 *
 * At most one (or exactly one, if required) flag in `flagIds` may be present.
 */
export interface ExclusiveGroup {
  /** Unique identifier. */
  id: string;
  /** IDs of the flags in this group. */
  flagIds: string[];
  /** If true, exactly one flag must be present. If false, at most one. */
  required: boolean;
}

/**
 * Internal representation of a command/subcommand (§2.4).
 *
 * Commands are recursive: a command may contain nested commands.
 */
export interface CommandDef {
  /** Unique ID among siblings. */
  id: string;
  /** The token the user types. */
  name: string;
  /** Alternative tokens (aliases) for this command. */
  aliases: string[];
  /** Human-readable description. */
  description: string;
  /** Whether root-level global_flags apply in this context. */
  inheritGlobalFlags: boolean;
  /** Flags specific to this command. */
  flags: FlagDef[];
  /** Positional arguments for this command. */
  arguments: ArgDef[];
  /** Nested subcommands. */
  commands: CommandDef[];
  /** Mutually exclusive groups in this scope. */
  mutuallyExclusiveGroups: ExclusiveGroup[];
}

/**
 * The top-level internal spec representation (§2.1).
 *
 * Produced by SpecLoader after validating the raw JSON.
 */
export interface CliSpec {
  /** The format version string. Must be "1.0". */
  specVersion: string;
  /** Program name as invoked (e.g., "ls", "git"). */
  name: string;
  /** Human-readable name for help output. */
  displayName?: string;
  /** One-line description shown in help. */
  description: string;
  /** Version string. When present, --version is auto-enabled. */
  version?: string;
  /** Parsing mode (gnu, posix, subcommand_first, traditional). */
  parsingMode: ParsingMode;
  /** Controls auto-injection of --help and --version. */
  builtinFlags: { help: boolean; version: boolean };
  /** Flags valid at every nesting level. */
  globalFlags: FlagDef[];
  /** Flags valid only at root level. */
  flags: FlagDef[];
  /** Positional arguments at root level. */
  arguments: ArgDef[];
  /** Subcommands. */
  commands: CommandDef[];
  /** Mutually exclusive groups at root scope. */
  mutuallyExclusiveGroups: ExclusiveGroup[];
}

// ---------------------------------------------------------------------------
// Result types — what the parser returns
// ---------------------------------------------------------------------------

/**
 * Returned when parsing succeeds normally (§7).
 *
 * All flags in scope appear in `flags` — absent optional booleans are `false`,
 * absent optional non-booleans are `null` (or the flag's `default` value).
 *
 * @example
 * ```typescript
 * // git remote add origin https://example.com
 * const result: ParseResult = {
 *   program: "git",
 *   commandPath: ["git", "remote", "add"],
 *   flags: { verbose: false, "no-pager": false },
 *   arguments: { name: "origin", url: "https://example.com" },
 * };
 * ```
 */
export interface ParseResult {
  /** argv[0] — the program name as invoked. */
  program: string;
  /** Full path from root to the resolved command: ["git", "remote", "add"]. */
  commandPath: string[];
  /** Map from flag ID to coerced value. All in-scope flags are present. */
  flags: Record<string, unknown>;
  /** Map from argument ID to coerced value. Variadic args produce arrays. */
  arguments: Record<string, unknown>;
}

/**
 * Returned when `--help` or `-h` is encountered (§7).
 *
 * The caller should print `text` and exit 0.
 */
export interface HelpResult {
  /** The rendered help text for the deepest resolved command. */
  text: string;
  /** The command path at which help was requested. */
  commandPath: string[];
}

/**
 * Returned when `--version` is encountered (§7).
 *
 * The caller should print `version` and exit 0.
 */
export interface VersionResult {
  /** The version string from the spec's `version` field. */
  version: string;
}

/**
 * Union of all possible successful parse outcomes.
 *
 * The caller discriminates on the shape:
 * - Has `commandPath` and `flags`? → ParseResult
 * - Has `text`?                    → HelpResult
 * - Has `version` only?            → VersionResult
 *
 * @example
 * ```typescript
 * const result = parser.parse();
 * if ("text" in result) {
 *   process.stdout.write(result.text + "\n");
 *   process.exit(0);
 * } else if ("version" in result && !("flags" in result)) {
 *   process.stdout.write(result.version + "\n");
 *   process.exit(0);
 * } else {
 *   // result is ParseResult
 *   doWork(result);
 * }
 * ```
 */
export type ParserResult = ParseResult | HelpResult | VersionResult;
