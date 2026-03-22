/**
 * spec-loader.ts — Load, validate, and normalize a CLI spec JSON file.
 *
 * === What SpecLoader does ===
 *
 * The raw JSON spec is a flexible, human-friendly format with many optional
 * fields and defaults. SpecLoader performs four functions:
 *
 * 1. **Read** — Parse the file as UTF-8 JSON.
 * 2. **Validate** — Check all required fields, cross-references, and structural
 *    invariants per §13 of the spec.
 * 3. **Normalize** — Fill in defaults, canonicalize camelCase names, convert
 *    arrays from undefined to [].
 * 4. **Build graphs** — Construct a DirectedGraph for the flag dependency (G_flag)
 *    of each scope and call `hasCycle()`. A cycle = spec error.
 *
 * The output of SpecLoader.load() is a `CliSpec` object — a well-typed,
 * ready-to-use internal representation with no optional arrays and no raw JSON.
 *
 * === Spec validation rules (§6.4.3) ===
 *
 * 1. `cli_builder_spec_version` must be "1.0".
 * 2. No duplicate flag `id`, command `id`, or argument `id` within any scope.
 * 3. Every flag has at least one of `short`, `long`, `single_dash_long`.
 * 4. All `conflicts_with` and `requires` IDs exist in the same scope or
 *    `global_flags`.
 * 5. All `mutually_exclusive_groups` reference valid flag IDs in the scope.
 * 6. `enum_values` is present and non-empty when `type` is "enum".
 * 7. At most one argument per scope has `variadic: true`.
 * 8. G_flag (requires edges) has no cycle in any scope.
 *
 * @module spec-loader
 */

import { readFileSync } from "fs";
import { Graph } from "@coding-adventures/directed-graph";
import { SpecError } from "./errors.js";
import type {
  ArgDef,
  CliSpec,
  CommandDef,
  ExclusiveGroup,
  FlagDef,
  ParsingMode,
  ValueType,
} from "./types.js";

// ---------------------------------------------------------------------------
// SpecLoader class
// ---------------------------------------------------------------------------

/**
 * Loads, validates, and normalizes a CLI Builder JSON spec file.
 *
 * Construct with the path to the spec file. Call `load()` to get the
 * internal `CliSpec` object. Throws `SpecError` on any validation failure.
 *
 * The loader caches the result after the first call — spec files are
 * read-once at startup, not on every parse.
 *
 * @example
 * ```typescript
 * const loader = new SpecLoader("./git.json");
 * const spec = loader.load(); // throws SpecError if invalid
 * ```
 */
export class SpecLoader {
  private readonly _filePath: string;
  private _cached: CliSpec | null = null;

  constructor(specFilePath: string) {
    this._filePath = specFilePath;
  }

  /**
   * Load and validate the spec file, returning the internal CliSpec.
   *
   * Results are cached after the first call. Subsequent calls return the
   * same object without re-reading the file.
   *
   * @throws SpecError if the JSON is invalid or any validation check fails.
   */
  load(): CliSpec {
    if (this._cached !== null) {
      return this._cached;
    }

    // --- Step 1: Read and parse the file ---
    let raw: Record<string, unknown>;
    try {
      const text = readFileSync(this._filePath, "utf-8");
      raw = JSON.parse(text) as Record<string, unknown>;
    } catch (e) {
      throw new SpecError(
        `Failed to read spec file '${this._filePath}': ${(e as Error).message}`,
      );
    }

    // --- Step 2: Validate and normalize ---
    this._cached = this._parseSpec(raw);
    return this._cached;
  }

  /**
   * Load a spec from an already-parsed object (used in tests).
   *
   * Skips file I/O. Useful for embedding spec JSON directly in test code.
   * Results are cached after the first call — subsequent calls return the same object.
   */
  loadFromObject(raw: Record<string, unknown>): CliSpec {
    if (this._cached !== null) {
      return this._cached;
    }
    this._cached = this._parseSpec(raw);
    return this._cached;
  }

  // ---------------------------------------------------------------------------
  // Private parsing helpers
  // ---------------------------------------------------------------------------

  /** Parse and validate the top-level spec object. */
  private _parseSpec(raw: Record<string, unknown>): CliSpec {
    // Rule 1: version check
    const specVersion = raw["cli_builder_spec_version"];
    if (specVersion !== "1.0") {
      throw new SpecError(
        `cli_builder_spec_version must be "1.0", got: ${JSON.stringify(specVersion)}`,
      );
    }

    // Required top-level fields
    const name = this._requireString(raw, "name");
    const description = this._requireString(raw, "description");

    // Optional fields with defaults
    const displayName =
      typeof raw["display_name"] === "string" ? raw["display_name"] : undefined;
    const version =
      typeof raw["version"] === "string" ? raw["version"] : undefined;

    const parsingMode = this._parseParsingMode(raw["parsing_mode"]);

    const builtinFlagsRaw = raw["builtin_flags"] as
      | Record<string, unknown>
      | undefined;
    const builtinFlags = {
      help: builtinFlagsRaw?.["help"] !== false,
      version: builtinFlagsRaw?.["version"] !== false,
    };

    // Parse global_flags first — they are available in every scope
    const globalFlags = this._parseFlagArray(
      raw["global_flags"],
      "global_flags",
      [],
    );

    // Parse root-level flags, arguments, commands, exclusive groups
    const flags = this._parseFlagArray(raw["flags"], "flags", globalFlags);
    const args = this._parseArgArray(raw["arguments"], "arguments");
    const commands = this._parseCommandArray(
      raw["commands"],
      "commands",
      globalFlags,
    );
    const mutuallyExclusiveGroups = this._parseExclusiveGroups(
      raw["mutually_exclusive_groups"],
      "mutually_exclusive_groups",
      flags,
    );

    // Rule 7: at most one variadic argument per scope
    this._checkVariadicCount(args, "root");

    // Rule 8: check G_flag for cycles in root scope
    this._checkFlagGraphCycle(flags, "root");

    const spec: CliSpec = {
      specVersion: "1.0",
      name,
      displayName,
      description,
      version,
      parsingMode,
      builtinFlags,
      globalFlags,
      flags,
      arguments: args,
      commands,
      mutuallyExclusiveGroups,
    };

    return spec;
  }

  /** Parse parsing_mode field, defaulting to "gnu". */
  private _parseParsingMode(raw: unknown): ParsingMode {
    const valid: ParsingMode[] = ["gnu", "posix", "subcommand_first", "traditional"];
    if (raw === undefined || raw === null) return "gnu";
    if (typeof raw !== "string" || !valid.includes(raw as ParsingMode)) {
      throw new SpecError(
        `parsing_mode must be one of ${valid.join(", ")}, got: ${JSON.stringify(raw)}`,
      );
    }
    return raw as ParsingMode;
  }

  /** Parse an array of flag definitions from raw JSON. */
  private _parseFlagArray(
    raw: unknown,
    fieldPath: string,
    globalFlags: FlagDef[],
  ): FlagDef[] {
    if (raw === undefined || raw === null) return [];
    if (!Array.isArray(raw)) {
      throw new SpecError(`${fieldPath} must be an array`);
    }

    const flags: FlagDef[] = [];
    const seenIds = new Set<string>();

    for (let i = 0; i < raw.length; i++) {
      const item = raw[i] as Record<string, unknown>;
      const flag = this._parseFlagDef(item, `${fieldPath}[${i}]`);

      // Rule 2: no duplicate IDs
      if (seenIds.has(flag.id)) {
        throw new SpecError(`Duplicate flag id "${flag.id}" in ${fieldPath}`);
      }
      seenIds.add(flag.id);
      flags.push(flag);
    }

    // Rule 4: validate conflicts_with and requires references
    const allFlagIds = new Set([
      ...globalFlags.map((f) => f.id),
      ...flags.map((f) => f.id),
    ]);
    for (const flag of flags) {
      for (const ref of flag.conflictsWith) {
        if (!allFlagIds.has(ref)) {
          throw new SpecError(
            `Flag "${flag.id}" conflicts_with unknown flag id "${ref}" in ${fieldPath}`,
          );
        }
      }
      for (const ref of flag.requires) {
        if (!allFlagIds.has(ref)) {
          throw new SpecError(
            `Flag "${flag.id}" requires unknown flag id "${ref}" in ${fieldPath}`,
          );
        }
      }
      for (const ref of flag.requiredUnless) {
        if (!allFlagIds.has(ref)) {
          throw new SpecError(
            `Flag "${flag.id}" required_unless unknown flag id "${ref}" in ${fieldPath}`,
          );
        }
      }
    }

    return flags;
  }

  /** Parse a single flag definition object. */
  private _parseFlagDef(
    raw: Record<string, unknown>,
    path: string,
  ): FlagDef {
    const id = this._requireString(raw, "id", path);
    const description = this._requireString(raw, "description", path);
    const type = this._parseValueType(raw["type"], path);

    // Rule 3: at least one of short, long, single_dash_long
    const short =
      typeof raw["short"] === "string" ? raw["short"] : undefined;
    const long =
      typeof raw["long"] === "string" ? raw["long"] : undefined;
    const singleDashLong =
      typeof raw["single_dash_long"] === "string"
        ? raw["single_dash_long"]
        : undefined;

    if (!short && !long && !singleDashLong) {
      throw new SpecError(
        `Flag "${id}" at ${path} must have at least one of: short, long, single_dash_long`,
      );
    }

    // Rule 6: enum_values required when type is "enum"
    const enumValues = Array.isArray(raw["enum_values"])
      ? (raw["enum_values"] as string[])
      : [];
    if (type === "enum" && enumValues.length === 0) {
      throw new SpecError(
        `Flag "${id}" at ${path} has type "enum" but enum_values is absent or empty`,
      );
    }

    // v1.1: default_when_present — only valid for enum flags.
    // When present, the value must be a member of enum_values.
    const defaultWhenPresent =
      typeof raw["default_when_present"] === "string"
        ? raw["default_when_present"]
        : undefined;

    if (defaultWhenPresent !== undefined) {
      if (type !== "enum") {
        throw new SpecError(
          `Flag "${id}" at ${path} has default_when_present but type is "${type}" (must be "enum")`,
        );
      }
      if (!enumValues.includes(defaultWhenPresent)) {
        throw new SpecError(
          `Flag "${id}" at ${path} has default_when_present "${defaultWhenPresent}" which is not in enum_values: ${enumValues.join(", ")}`,
        );
      }
    }

    return {
      id,
      short,
      long,
      singleDashLong,
      description,
      type,
      required: raw["required"] === true,
      default: raw["default"] !== undefined ? raw["default"] : null,
      valueName:
        typeof raw["value_name"] === "string" ? raw["value_name"] : undefined,
      enumValues,
      defaultWhenPresent,
      conflictsWith: Array.isArray(raw["conflicts_with"])
        ? (raw["conflicts_with"] as string[])
        : [],
      requires: Array.isArray(raw["requires"])
        ? (raw["requires"] as string[])
        : [],
      requiredUnless: Array.isArray(raw["required_unless"])
        ? (raw["required_unless"] as string[])
        : [],
      repeatable: raw["repeatable"] === true,
    };
  }

  /** Parse an array of argument definitions. */
  private _parseArgArray(raw: unknown, fieldPath: string): ArgDef[] {
    if (raw === undefined || raw === null) return [];
    if (!Array.isArray(raw)) {
      throw new SpecError(`${fieldPath} must be an array`);
    }

    const args: ArgDef[] = [];
    const seenIds = new Set<string>();

    for (let i = 0; i < raw.length; i++) {
      const item = raw[i] as Record<string, unknown>;
      const arg = this._parseArgDef(item, `${fieldPath}[${i}]`);

      // Rule 2: no duplicate IDs
      if (seenIds.has(arg.id)) {
        throw new SpecError(`Duplicate argument id "${arg.id}" in ${fieldPath}`);
      }
      seenIds.add(arg.id);
      args.push(arg);
    }

    return args;
  }

  /** Parse a single argument definition. */
  private _parseArgDef(
    raw: Record<string, unknown>,
    path: string,
  ): ArgDef {
    const id = this._requireString(raw, "id", path);
    // Accept display_name (preferred) or name (backward compatibility).
    const display_name =
      typeof raw["display_name"] === "string"
        ? raw["display_name"]
        : this._requireString(raw, "name", path);
    const description = this._requireString(raw, "description", path);
    const type = this._parseValueType(raw["type"], path);
    const required = raw["required"] !== false; // defaults to true
    const variadic = raw["variadic"] === true;

    const defaultVariadicMin = required ? 1 : 0;
    const variadicMin =
      typeof raw["variadic_min"] === "number"
        ? raw["variadic_min"]
        : defaultVariadicMin;
    const variadicMax =
      typeof raw["variadic_max"] === "number" ? raw["variadic_max"] : null;

    // Rule 6: enum_values required when type is "enum"
    const enumValues = Array.isArray(raw["enum_values"])
      ? (raw["enum_values"] as string[])
      : [];
    if (type === "enum" && enumValues.length === 0) {
      throw new SpecError(
        `Argument "${id}" at ${path} has type "enum" but enum_values is absent or empty`,
      );
    }

    return {
      id,
      display_name,
      description,
      type,
      required,
      variadic,
      variadicMin,
      variadicMax,
      default: raw["default"] !== undefined ? raw["default"] : null,
      enumValues,
      requiredUnlessFlag: Array.isArray(raw["required_unless_flag"])
        ? (raw["required_unless_flag"] as string[])
        : [],
    };
  }

  /** Parse an array of command definitions (recursive). */
  private _parseCommandArray(
    raw: unknown,
    fieldPath: string,
    globalFlags: FlagDef[],
  ): CommandDef[] {
    if (raw === undefined || raw === null) return [];
    if (!Array.isArray(raw)) {
      throw new SpecError(`${fieldPath} must be an array`);
    }

    const commands: CommandDef[] = [];
    const seenIds = new Set<string>();
    const seenNames = new Set<string>();

    for (let i = 0; i < raw.length; i++) {
      const item = raw[i] as Record<string, unknown>;
      const cmd = this._parseCommandDef(item, `${fieldPath}[${i}]`, globalFlags);

      // Rule 2: no duplicate command IDs or names among siblings
      if (seenIds.has(cmd.id)) {
        throw new SpecError(`Duplicate command id "${cmd.id}" in ${fieldPath}`);
      }
      seenIds.add(cmd.id);

      const allNames = [cmd.name, ...cmd.aliases];
      for (const n of allNames) {
        if (seenNames.has(n)) {
          throw new SpecError(
            `Duplicate command name/alias "${n}" in ${fieldPath}`,
          );
        }
        seenNames.add(n);
      }

      commands.push(cmd);
    }

    return commands;
  }

  /** Parse a single command definition (recursively). */
  private _parseCommandDef(
    raw: Record<string, unknown>,
    path: string,
    globalFlags: FlagDef[],
  ): CommandDef {
    const id = this._requireString(raw, "id", path);
    const name = this._requireString(raw, "name", path);
    const description = this._requireString(raw, "description", path);
    const aliases = Array.isArray(raw["aliases"])
      ? (raw["aliases"] as string[])
      : [];
    const inheritGlobalFlags = raw["inherit_global_flags"] !== false;

    const effectiveGlobal = inheritGlobalFlags ? globalFlags : [];

    const flags = this._parseFlagArray(raw["flags"], `${path}.flags`, effectiveGlobal);
    const args = this._parseArgArray(raw["arguments"], `${path}.arguments`);
    const nestedCommands = this._parseCommandArray(
      raw["commands"],
      `${path}.commands`,
      globalFlags,
    );
    const mutuallyExclusiveGroups = this._parseExclusiveGroups(
      raw["mutually_exclusive_groups"],
      `${path}.mutually_exclusive_groups`,
      [...effectiveGlobal, ...flags],
    );

    // Rule 7: at most one variadic per scope
    this._checkVariadicCount(args, path);

    // Rule 8: check G_flag for cycles in this scope
    this._checkFlagGraphCycle([...effectiveGlobal, ...flags], path);

    return {
      id,
      name,
      aliases,
      description,
      inheritGlobalFlags,
      flags,
      arguments: args,
      commands: nestedCommands,
      mutuallyExclusiveGroups,
    };
  }

  /** Parse mutually_exclusive_groups and validate flag references. */
  private _parseExclusiveGroups(
    raw: unknown,
    fieldPath: string,
    scopeFlags: FlagDef[],
  ): ExclusiveGroup[] {
    if (raw === undefined || raw === null) return [];
    if (!Array.isArray(raw)) {
      throw new SpecError(`${fieldPath} must be an array`);
    }

    const groups: ExclusiveGroup[] = [];
    const validIds = new Set(scopeFlags.map((f) => f.id));

    for (let i = 0; i < raw.length; i++) {
      const item = raw[i] as Record<string, unknown>;
      const id = this._requireString(item, "id", `${fieldPath}[${i}]`);

      if (!Array.isArray(item["flag_ids"])) {
        throw new SpecError(
          `${fieldPath}[${i}] missing required field "flag_ids"`,
        );
      }
      const flagIds = item["flag_ids"] as string[];

      // Rule 5: all flag_ids must reference valid flags in scope
      for (const fid of flagIds) {
        if (!validIds.has(fid)) {
          throw new SpecError(
            `mutually_exclusive_group "${id}" references unknown flag id "${fid}" in ${fieldPath}`,
          );
        }
      }

      groups.push({
        id,
        flagIds,
        required: item["required"] === true,
      });
    }

    return groups;
  }

  /**
   * Build the flag dependency graph for a scope and check for cycles.
   *
   * G_flag has one node per flag ID. A directed edge A → B means
   * "flag A requires flag B" (from flag A's `requires` array).
   *
   * A cycle like A requires B and B requires A makes the spec logically
   * impossible — there is no valid invocation that satisfies it.
   */
  private _checkFlagGraphCycle(flags: FlagDef[], scopeName: string): void {
    const graph = new Graph();

    for (const flag of flags) {
      graph.addNode(flag.id);
    }

    for (const flag of flags) {
      for (const reqId of flag.requires) {
        // Only add edges between nodes that exist in this scope
        if (graph.hasNode(reqId)) {
          graph.addEdge(flag.id, reqId);
        }
      }
    }

    if (graph.hasCycle()) {
      throw new SpecError(
        `Circular requires dependency detected in scope "${scopeName}". ` +
          `Check the "requires" fields of your flags.`,
      );
    }
  }

  /** Check that at most one argument in a scope is variadic. */
  private _checkVariadicCount(args: ArgDef[], scopeName: string): void {
    const variadicCount = args.filter((a) => a.variadic).length;
    if (variadicCount > 1) {
      throw new SpecError(
        `Scope "${scopeName}" has ${variadicCount} variadic arguments. At most one is allowed.`,
      );
    }
  }

  // ---------------------------------------------------------------------------
  // Low-level helpers
  // ---------------------------------------------------------------------------

  /** Require a string field on a raw object. Throws SpecError if missing. */
  private _requireString(
    raw: Record<string, unknown>,
    field: string,
    path = "spec",
  ): string {
    const val = raw[field];
    if (typeof val !== "string" || val.length === 0) {
      throw new SpecError(
        `${path} is missing required string field "${field}"`,
      );
    }
    return val;
  }

  /** Parse and validate a ValueType string. */
  private _parseValueType(raw: unknown, path: string): ValueType {
    const valid: ValueType[] = [
      "boolean",
      "count",
      "string",
      "integer",
      "float",
      "path",
      "file",
      "directory",
      "enum",
    ];
    if (typeof raw !== "string" || !valid.includes(raw as ValueType)) {
      throw new SpecError(
        `${path} has invalid type "${raw}". Must be one of: ${valid.join(", ")}`,
      );
    }
    return raw as ValueType;
  }
}
