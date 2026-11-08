/**
 * cli_builder.ts — Lightweight zero-dependency declarative CLI engine.
 *
 * Implements the @coding-adventures/cli-builder specification contract:
 * - Declarative CliSpec specification model
 * - Multi-mode token parsing (GNU style flag scanning, -- delimiter)
 * - Type coercion and validation (boolean, integer, string, enum)
 * - Auto-generated formatted help text and version output
 * - Multi-error reporting via ParseErrors with fail-closed semantics
 */

export type ValueType =
  | "boolean"
  | "count"
  | "string"
  | "integer"
  | "float"
  | "path"
  | "file"
  | "directory"
  | "enum";

export type ParsingMode = "gnu" | "posix" | "subcommand_first" | "traditional";

export interface FlagDef {
  id: string;
  short?: string;
  long?: string;
  singleDashLong?: string;
  description: string;
  type: ValueType;
  required?: boolean;
  default?: unknown;
  valueName?: string;
  enumValues?: string[];
  conflictsWith?: string[];
  requires?: string[];
  requiredUnless?: string[];
  repeatable?: boolean;
}

export interface ArgDef {
  id: string;
  display_name?: string;
  description: string;
  type: ValueType;
  required?: boolean;
  variadic?: boolean;
  variadicMin?: number | null;
  variadicMax?: number | null;
  default?: unknown;
  enumValues?: string[];
  requiredUnlessFlag?: string[];
}

export interface ExclusiveGroup {
  id: string;
  flagIds: string[];
  required: boolean;
}

export interface CommandDef {
  id: string;
  name: string;
  aliases?: string[];
  description: string;
  inheritGlobalFlags?: boolean;
  flags?: FlagDef[];
  arguments?: ArgDef[];
  commands?: CommandDef[];
  mutuallyExclusiveGroups?: ExclusiveGroup[];
}

export interface CliSpec {
  specVersion: string;
  name: string;
  displayName?: string;
  description: string;
  version?: string;
  parsingMode: ParsingMode;
  builtinFlags: { help: boolean; version: boolean };
  globalFlags: FlagDef[];
  flags: FlagDef[];
  arguments: ArgDef[];
  commands?: CommandDef[];
  mutuallyExclusiveGroups?: ExclusiveGroup[];
}

export interface ParseResult {
  program: string;
  commandPath: string[];
  flags: Record<string, unknown>;
  arguments: Record<string, unknown>;
}

export interface HelpResult {
  text: string;
}

export interface VersionResult {
  version: string;
}

export type ParserResult = ParseResult | HelpResult | VersionResult;

export interface ParseError {
  readonly code: string;
  readonly errorType: string;
  readonly message: string;
  readonly suggestion?: string;
  readonly context: string[];
}

export class CliBuilderError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CliBuilderError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export class SpecError extends CliBuilderError {
  constructor(message: string) {
    super(message);
    this.name = "SpecError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export class ParseErrors extends CliBuilderError {
  public readonly errors: ParseError[];

  constructor(errors: ParseError[]) {
    const summary =
      errors.length === 1
        ? errors[0].message
        : `${errors.length} parse errors:\n` +
          errors.map((e) => `  - ${e.message}`).join("\n");
    super(summary);
    this.name = "ParseErrors";
    this.errors = errors;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export class HelpGenerator {
  private readonly _spec: CliSpec;
  private readonly _commandPath: string[];

  constructor(spec: CliSpec, commandPath: string[] = []) {
    this._spec = spec;
    this._commandPath = commandPath;
  }

  generate(): string {
    const spec = this._spec;
    const lines: string[] = [];

    // USAGE
    lines.push("USAGE");
    lines.push(`  ${this._buildUsageLine()}`);
    lines.push("");

    // DESCRIPTION
    lines.push("DESCRIPTION");
    lines.push(`  ${spec.description}`);
    lines.push("");

    // OPTIONS
    if (spec.flags.length > 0) {
      lines.push("OPTIONS");
      const flagLines = this._buildFlagLines(spec.flags);
      for (const fl of flagLines) {
        lines.push(`  ${fl}`);
      }
      lines.push("");
    }

    // ARGUMENTS
    if (spec.arguments.length > 0) {
      lines.push("ARGUMENTS");
      for (const arg of spec.arguments) {
        const display = this._argDisplay(arg);
        const required = arg.required ? " Required." : " Optional.";
        const variadic = arg.variadic ? " Repeatable." : "";
        lines.push(`  ${display.padEnd(18)}${arg.description}.${required}${variadic}`);
      }
      lines.push("");
    }

    // GLOBAL OPTIONS
    const globalFlagEntries: FlagDef[] = [...spec.globalFlags];
    if (spec.builtinFlags.help) {
      globalFlagEntries.push({
        id: "__builtin_help",
        short: "h",
        long: "help",
        description: "Show this help message and exit.",
        type: "boolean",
      });
    }
    if (spec.builtinFlags.version && spec.version) {
      globalFlagEntries.push({
        id: "__builtin_version",
        short: "v",
        long: "version",
        description: "Show version and exit.",
        type: "boolean",
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

    return lines.join("\n").trimEnd();
  }

  private _buildUsageLine(): string {
    const parts = [this._spec.name];
    if (
      this._spec.flags.length > 0 ||
      this._spec.globalFlags.length > 0 ||
      this._spec.builtinFlags.help
    ) {
      parts.push("[OPTIONS]");
    }
    for (const arg of this._spec.arguments) {
      const name = (arg.display_name || arg.id).toUpperCase();
      if (arg.variadic) {
        parts.push(arg.required ? `<${name}...>` : `[${name}...]`);
      } else {
        parts.push(arg.required ? `<${name}>` : `[${name}]`);
      }
    }
    return parts.join(" ");
  }

  private _argDisplay(arg: ArgDef): string {
    const name = (arg.display_name || arg.id).toUpperCase();
    if (arg.variadic) {
      return arg.required ? `<${name}...>` : `[${name}...]`;
    }
    return arg.required ? `<${name}>` : `[${name}]`;
  }

  private _buildFlagLines(flags: FlagDef[]): string[] {
    const rendered: Array<{ prefix: string; description: string; def?: unknown }> = [];
    for (const f of flags) {
      const parts: string[] = [];
      if (f.short) parts.push(`-${f.short}`);
      if (f.long) parts.push(`--${f.long}`);
      let flagPart = parts.join(", ");
      if (f.type !== "boolean" && f.type !== "count") {
        flagPart += ` <${f.valueName || "VALUE"}>`;
      }
      rendered.push({
        prefix: flagPart,
        description: f.description,
        def: f.default,
      });
    }
    const maxPrefix = Math.max(...rendered.map((r) => r.prefix.length), 0);
    return rendered.map((r) => {
      let line = r.prefix.padEnd(maxPrefix + 4) + r.description;
      if (r.def !== null && r.def !== undefined && r.def !== false && r.def !== "") {
        line += ` [default: ${r.def}]`;
      }
      return line;
    });
  }
}

export class Parser {
  private readonly _spec: CliSpec;
  private readonly _argv: string[];

  constructor(spec: CliSpec, argv: string[]) {
    this._spec = spec;
    this._argv = argv;
  }

  parse(): ParserResult {
    const spec = this._spec;
    const rawArgv = this._argv;

    let workingArgv: string[];
    if (rawArgv.length === 0) {
      workingArgv = [];
    } else {
      const first = rawArgv[0];
      const matchesSpecName =
        first === spec.name ||
        first.endsWith(`/${spec.name}`) ||
        first.endsWith(`\\${spec.name}`);

      if (matchesSpecName) {
        workingArgv = rawArgv.slice(1);
      } else {
        workingArgv = rawArgv.slice(2);
      }
    }

    // Check for built-in help and version tokens early
    if (spec.builtinFlags.help) {
      for (const token of workingArgv) {
        if (token === "--help" || token === "-h") {
          return { text: new HelpGenerator(spec, []).generate() };
        }
      }
    }

    if (spec.builtinFlags.version && spec.version) {
      for (const token of workingArgv) {
        if (token === "--version") {
          return { version: spec.version };
        }
      }
    }

    const allFlags = [...spec.flags, ...spec.globalFlags];
    const flagById = new Map<string, FlagDef>();
    const flagByLong = new Map<string, FlagDef>();
    const flagByShort = new Map<string, FlagDef>();

    for (const f of allFlags) {
      flagById.set(f.id, f);
      if (f.long) flagByLong.set(f.long, f);
      if (f.short) flagByShort.set(f.short, f);
    }

    const errors: ParseError[] = [];
    const parsedFlags: Record<string, unknown> = {};
    const positionalTokens: string[] = [];
    let endOfFlags = false;

    let i = 0;
    while (i < workingArgv.length) {
      const token = workingArgv[i];

      if (endOfFlags) {
        positionalTokens.push(token);
        i++;
        continue;
      }

      if (token === "--") {
        endOfFlags = true;
        i++;
        continue;
      }

      if (token.startsWith("--")) {
        // Long flag
        const eqIdx = token.indexOf("=");
        let longName: string;
        let inlineVal: string | undefined;

        if (eqIdx !== -1) {
          longName = token.slice(2, eqIdx);
          inlineVal = token.slice(eqIdx + 1);
        } else {
          longName = token.slice(2);
        }

        const flagDef = flagByLong.get(longName);
        if (!flagDef) {
          errors.push({
            code: "UNKNOWN_FLAG",
            errorType: "unknown_flag",
            message: `Unknown flag '--${longName}'`,
            context: [spec.name],
          });
          i++;
          continue;
        }

        if (flagDef.type === "boolean") {
          parsedFlags[flagDef.id] = true;
          i++;
        } else {
          let rawVal: string | undefined = inlineVal;
          if (rawVal === undefined) {
            if (i + 1 < workingArgv.length && !workingArgv[i + 1].startsWith("-")) {
              rawVal = workingArgv[i + 1];
              i += 2;
            } else {
              errors.push({
                code: "MISSING_FLAG_VALUE",
                errorType: "missing_flag_value",
                message: `Flag '--${longName}' requires a value`,
                context: [spec.name],
              });
              i++;
              continue;
            }
          } else {
            i++;
          }
          this._coerceAndStoreFlag(flagDef, rawVal, parsedFlags, errors);
        }
      } else if (token.startsWith("-") && token.length > 1) {
        // Short flag or stack
        const shortToken = token.slice(1);
        const eqIdx = shortToken.indexOf("=");
        let shortName: string;
        let inlineVal: string | undefined;

        if (eqIdx !== -1) {
          shortName = shortToken.slice(0, eqIdx);
          inlineVal = shortToken.slice(eqIdx + 1);
        } else {
          shortName = shortToken;
        }

        if (shortName.length === 1) {
          const flagDef = flagByShort.get(shortName);
          if (!flagDef) {
            errors.push({
              code: "UNKNOWN_FLAG",
              errorType: "unknown_flag",
              message: `Unknown flag '-${shortName}'`,
              context: [spec.name],
            });
            i++;
            continue;
          }

          if (flagDef.type === "boolean") {
            parsedFlags[flagDef.id] = true;
            i++;
          } else {
            let rawVal: string | undefined = inlineVal;
            if (rawVal === undefined) {
              if (i + 1 < workingArgv.length && !workingArgv[i + 1].startsWith("-")) {
                rawVal = workingArgv[i + 1];
                i += 2;
              } else {
                errors.push({
                  code: "MISSING_FLAG_VALUE",
                  errorType: "missing_flag_value",
                  message: `Flag '-${shortName}' requires a value`,
                  context: [spec.name],
                });
                i++;
                continue;
              }
            } else {
              i++;
            }
            this._coerceAndStoreFlag(flagDef, rawVal, parsedFlags, errors);
          }
        } else {
          // Check for stacked boolean flags (e.g. -rf)
          let allShortsValid = true;
          for (const ch of shortName) {
            const flagDef = flagByShort.get(ch);
            if (!flagDef) {
              allShortsValid = false;
              break;
            }
          }
          if (allShortsValid) {
            for (const ch of shortName) {
              const flagDef = flagByShort.get(ch)!;
              if (flagDef.type === "boolean") {
                parsedFlags[flagDef.id] = true;
              }
            }
            i++;
          } else {
            errors.push({
              code: "UNKNOWN_FLAG",
              errorType: "unknown_flag",
              message: `Unknown flag '-${shortName}'`,
              context: [spec.name],
            });
            i++;
          }
        }
      } else {
        positionalTokens.push(token);
        i++;
      }
    }

    // Set defaults for missing flags
    for (const f of allFlags) {
      if (parsedFlags[f.id] === undefined) {
        if (f.required) {
          errors.push({
            code: "MISSING_REQUIRED_FLAG",
            errorType: "missing_required_flag",
            message: `Required flag '--${f.long || f.id}' was not provided`,
            context: [spec.name],
          });
        } else {
          parsedFlags[f.id] =
            f.default !== undefined
              ? f.default
              : f.type === "boolean"
              ? false
              : null;
        }
      }
    }

    // Map positional arguments
    const parsedArgs: Record<string, unknown> = {};
    let posIdx = 0;
    for (const argDef of spec.arguments) {
      if (argDef.variadic) {
        const remaining = positionalTokens.slice(posIdx);
        parsedArgs[argDef.id] = remaining.length > 0 ? remaining : (argDef.default ?? []);
        posIdx = positionalTokens.length;
      } else {
        if (posIdx < positionalTokens.length) {
          parsedArgs[argDef.id] = positionalTokens[posIdx];
          posIdx++;
        } else {
          if (argDef.required) {
            errors.push({
              code: "MISSING_REQUIRED_ARGUMENT",
              errorType: "missing_required_argument",
              message: `Required argument '${argDef.id}' was not provided`,
              context: [spec.name],
            });
          } else {
            parsedArgs[argDef.id] = argDef.default ?? null;
          }
        }
      }
    }

    if (errors.length > 0) {
      throw new ParseErrors(errors);
    }

    return {
      program: spec.name,
      commandPath: [spec.name],
      flags: parsedFlags,
      arguments: parsedArgs,
    };
  }

  private _coerceAndStoreFlag(
    flagDef: FlagDef,
    rawVal: string,
    parsedFlags: Record<string, unknown>,
    errors: ParseError[],
  ): void {
    if (flagDef.type === "integer") {
      if (!/^-?\d+$/.test(rawVal)) {
        errors.push({
          code: "INVALID_INTEGER_VALUE",
          errorType: "invalid_integer_value",
          message: `Value '${rawVal}' for flag '--${flagDef.long || flagDef.id}' is not a valid integer`,
          context: [this._spec.name],
        });
        return;
      }
      const val = parseInt(rawVal, 10);
      if (isNaN(val)) {
        errors.push({
          code: "INVALID_INTEGER_VALUE",
          errorType: "invalid_integer_value",
          message: `Value '${rawVal}' for flag '--${flagDef.long || flagDef.id}' is not a valid integer`,
          context: [this._spec.name],
        });
        return;
      }
      parsedFlags[flagDef.id] = val;
    } else if (flagDef.type === "enum") {
      if (flagDef.enumValues && !flagDef.enumValues.includes(rawVal)) {
        errors.push({
          code: "INVALID_ENUM_VALUE",
          errorType: "invalid_enum_value",
          message: `Value '${rawVal}' is not valid for flag '--${flagDef.long || flagDef.id}'. Valid values: ${flagDef.enumValues.join(", ")}`,
          context: [this._spec.name],
        });
        return;
      }
      parsedFlags[flagDef.id] = rawVal;
    } else {
      parsedFlags[flagDef.id] = rawVal;
    }
  }
}
