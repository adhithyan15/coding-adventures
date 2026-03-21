/**
 * help-generator.ts — Auto-generate help text from the spec.
 *
 * === Help Text Format (§9) ===
 *
 * CLI Builder generates help text from the spec at runtime. The format
 * follows a standard Unix convention:
 *
 * ```
 * USAGE
 *   <name> [OPTIONS] [COMMAND] [ARGS...]
 *
 * DESCRIPTION
 *   <description>
 *
 * COMMANDS
 *   subcommand    Description of the subcommand.
 *
 * OPTIONS
 *   -s, --long <VALUE>    Description. [default: val]
 *   -b, --boolean         Boolean flag description.
 *
 * GLOBAL OPTIONS
 *   -h, --help     Show this help message and exit.
 *   --version      Show version and exit.
 * ```
 *
 * === For Subcommands ===
 *
 * When the user runs `program subcommand --help`, the help text is
 * scoped to that subcommand and includes its ARGUMENTS section:
 *
 * ```
 * USAGE
 *   <program> <subcommand> [OPTIONS] <ARG> [ARG...]
 *
 * DESCRIPTION
 *   <subcommand description>
 *
 * OPTIONS
 *   ...
 *
 * ARGUMENTS
 *   <ARG>      Description. Required.
 *   [ARG...]   Description. Optional, repeatable.
 * ```
 *
 * === Formatting Rules ===
 *
 * - Required positional arguments: `<NAME>`
 * - Optional positional arguments: `[NAME]`
 * - Variadic required: `<NAME...>`
 * - Variadic optional: `[NAME...]`
 * - Non-boolean flags: `-s, --long <VALUE>`
 * - Boolean flags: `-s, --long`
 * - single_dash_long flags: `-classpath <VALUE>`
 * - Default values appended as `[default: X]`
 * - Column alignment via padding (longest flag first)
 *
 * @module help-generator
 */

import type { ArgDef, CliSpec, CommandDef, FlagDef } from "./types.js";

// ---------------------------------------------------------------------------
// HelpGenerator
// ---------------------------------------------------------------------------

/**
 * Generates formatted help text for a program or subcommand.
 *
 * @example
 * ```typescript
 * // Root help: git --help
 * const gen = new HelpGenerator(spec, []);
 * const help = gen.generate();
 *
 * // Subcommand help: git remote add --help
 * const gen2 = new HelpGenerator(spec, ["remote", "add"]);
 * const help2 = gen2.generate();
 * ```
 */
export class HelpGenerator {
  private readonly _spec: CliSpec;
  private readonly _commandPath: string[];

  constructor(spec: CliSpec, commandPath: string[]) {
    this._spec = spec;
    this._commandPath = commandPath;
  }

  /**
   * Generate and return the help text string.
   */
  generate(): string {
    const spec = this._spec;

    // Resolve the current command node based on commandPath
    // commandPath is the subcommand names (not including the program name)
    const cmdNode = this._resolveCommandNode(this._commandPath);

    const lines: string[] = [];

    // --- USAGE ---
    lines.push("USAGE");
    lines.push(`  ${this._buildUsageLine(cmdNode)}`);
    lines.push("");

    // --- DESCRIPTION ---
    const description = cmdNode ? cmdNode.description : spec.description;
    lines.push("DESCRIPTION");
    lines.push(`  ${description}`);
    lines.push("");

    // --- COMMANDS (only if this node has subcommands) ---
    const commands = cmdNode ? cmdNode.commands : spec.commands;
    if (commands.length > 0) {
      lines.push("COMMANDS");
      const maxNameLen = Math.max(...commands.map((c) => c.name.length));
      for (const cmd of commands) {
        const namePad = cmd.name.padEnd(maxNameLen + 2);
        lines.push(`  ${namePad}${cmd.description}`);
      }
      lines.push("");
    }

    // --- OPTIONS ---
    // Collect local flags (not global) for this scope
    const localFlags = cmdNode ? cmdNode.flags : spec.flags;
    if (localFlags.length > 0) {
      lines.push("OPTIONS");
      const flagLines = this._buildFlagLines(localFlags);
      for (const fl of flagLines) {
        lines.push(`  ${fl}`);
      }
      lines.push("");
    }

    // --- ARGUMENTS (only for subcommands or when root has args) ---
    const argDefs = cmdNode ? cmdNode.arguments : spec.arguments;
    if (argDefs.length > 0) {
      lines.push("ARGUMENTS");
      for (const arg of argDefs) {
        const display = this._argDisplay(arg);
        const required = arg.required ? " Required." : " Optional.";
        const variadic = arg.variadic ? " Repeatable." : "";
        lines.push(`  ${display.padEnd(18)}${arg.description}.${required}${variadic}`);
      }
      lines.push("");
    }

    // --- GLOBAL OPTIONS ---
    // Include global flags and built-in flags
    const globalFlagEntries: FlagDef[] = [...spec.globalFlags];

    // Add builtin --help if enabled
    if (spec.builtinFlags.help) {
      globalFlagEntries.push({
        id: "__builtin_help",
        short: "h",
        long: "help",
        description: "Show this help message and exit.",
        type: "boolean",
        required: false,
        default: null,
        enumValues: [],
        conflictsWith: [],
        requires: [],
        requiredUnless: [],
        repeatable: false,
      });
    }

    // Add builtin --version if enabled and version is present
    if (spec.builtinFlags.version && spec.version) {
      globalFlagEntries.push({
        id: "__builtin_version",
        long: "version",
        description: "Show version and exit.",
        type: "boolean",
        required: false,
        default: null,
        enumValues: [],
        conflictsWith: [],
        requires: [],
        requiredUnless: [],
        repeatable: false,
      });
    }

    if (globalFlagEntries.length > 0) {
      lines.push("GLOBAL OPTIONS");
      const flagLines = this._buildFlagLines(globalFlagEntries);
      for (const fl of flagLines) {
        lines.push(`  ${fl}`);
      }
      lines.push("");
    }

    // Remove trailing blank line
    while (lines.length > 0 && lines[lines.length - 1] === "") {
      lines.pop();
    }

    return lines.join("\n");
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  /**
   * Walk the command tree to find the CommandDef for the given subcommand path.
   * Returns null if the path is empty (root level).
   */
  private _resolveCommandNode(commandPath: string[]): CommandDef | null {
    if (commandPath.length === 0) return null;

    let commands = this._spec.commands;
    let node: CommandDef | null = null;

    for (const segment of commandPath) {
      const found = commands.find(
        (c) => c.name === segment || c.aliases.includes(segment),
      );
      if (!found) break;
      node = found;
      commands = found.commands;
    }

    return node;
  }

  /**
   * Build the USAGE line for the given command node.
   *
   * Examples:
   *   "git [OPTIONS] [COMMAND]"
   *   "git remote add [OPTIONS] <NAME> <URL>"
   */
  private _buildUsageLine(cmdNode: CommandDef | null): string {
    const programName = this._spec.name;
    const parts: string[] = [programName];

    // Add subcommand path
    for (const segment of this._commandPath) {
      parts.push(segment);
    }

    // [OPTIONS] if there are any flags
    const flags = cmdNode ? cmdNode.flags : this._spec.flags;
    const globalFlags = this._spec.globalFlags;
    const hasFlags = flags.length > 0 || globalFlags.length > 0 || this._spec.builtinFlags.help;
    if (hasFlags) {
      parts.push("[OPTIONS]");
    }

    // [COMMAND] if there are subcommands
    const commands = cmdNode ? cmdNode.commands : this._spec.commands;
    if (commands.length > 0) {
      parts.push("[COMMAND]");
    }

    // Argument placeholders
    const args = cmdNode ? cmdNode.arguments : this._spec.arguments;
    for (const arg of args) {
      parts.push(this._argDisplay(arg));
    }

    return parts.join(" ");
  }

  /**
   * Format an argument for display in USAGE or ARGUMENTS sections.
   *
   * - Required non-variadic: `<NAME>`
   * - Optional non-variadic: `[NAME]`
   * - Required variadic: `<NAME...>`
   * - Optional variadic: `[NAME...]`
   */
  private _argDisplay(arg: ArgDef): string {
    const namePart = arg.variadic ? `${arg.name}...` : arg.name;
    if (arg.required) {
      return `<${namePart}>`;
    }
    return `[${namePart}]`;
  }

  /**
   * Build formatted flag display lines, aligned by the longest flag string.
   *
   * Returns an array of strings like:
   *   "-l, --long-listing         Use long listing format."
   *   "-h, --human-readable <SIZE>  Print sizes like 1K 234M."
   */
  private _buildFlagLines(flags: FlagDef[]): string[] {
    const entries: Array<{ left: string; right: string }> = flags.map((f) => ({
      left: this._flagSignature(f),
      right: this._flagDescription(f),
    }));

    // Align descriptions to the longest signature
    const maxLen = Math.max(...entries.map((e) => e.left.length), 0);

    return entries.map((e) => {
      const padding = " ".repeat(Math.max(2, maxLen - e.left.length + 4));
      return `${e.left}${padding}${e.right}`;
    });
  }

  /**
   * Return the flag signature string, e.g.:
   *   "-l, --long-listing"
   *   "-n, --lines <NUM>"
   *   "-classpath <classpath>"
   *   "--version"
   */
  private _flagSignature(flag: FlagDef): string {
    const parts: string[] = [];

    if (flag.short) parts.push(`-${flag.short}`);
    if (flag.long) {
      if (flag.type === "boolean") {
        parts.push(`--${flag.long}`);
      } else {
        const valName = flag.valueName ?? flag.type.toUpperCase();
        parts.push(`--${flag.long} <${valName}>`);
      }
    }
    if (flag.singleDashLong) {
      if (flag.type === "boolean") {
        parts.push(`-${flag.singleDashLong}`);
      } else {
        const valName = flag.valueName ?? flag.type.toUpperCase();
        parts.push(`-${flag.singleDashLong} <${valName}>`);
      }
    }

    return parts.join(", ");
  }

  /**
   * Return the flag description with default value appended if applicable.
   */
  private _flagDescription(flag: FlagDef): string {
    let desc = flag.description;
    if (flag.default !== null && flag.default !== undefined && !flag.required) {
      desc += ` [default: ${String(flag.default)}]`;
    }
    return desc;
  }
}
