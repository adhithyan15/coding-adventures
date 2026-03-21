/**
 * parser.ts — The main CLI argument parser.
 *
 * === How the Parser Works ===
 *
 * The parser implements the three-phase algorithm from §6 of the spec:
 *
 * **Phase 1 — Routing (Directed Graph)**
 *
 *   Walk argv looking for subcommand tokens. Use G_cmd (built from the spec's
 *   `commands` array) to navigate: when a token matches a command name or alias
 *   at the current node, follow that edge. Stop when no match is found.
 *
 *   After Phase 1, `commandPath` is the full path like ["git", "remote", "add"],
 *   and `current_node` is the leaf CommandDef whose flags/args apply.
 *
 * **Phase 2 — Scanning (Modal State Machine)**
 *
 *   Re-walk the same argv (skipping command tokens already consumed in Phase 1).
 *   The TokenClassifier classifies each token. The parse mode switches between
 *   SCANNING, FLAG_VALUE, and END_OF_FLAGS:
 *
 *   - SCANNING: normal mode — classify and handle each token
 *   - FLAG_VALUE: the previous token was a non-boolean flag; this token is its value
 *   - END_OF_FLAGS: saw "--"; all remaining tokens are positional
 *
 *   Handle --help and --version immediately (return early).
 *
 * **Phase 3 — Validation**
 *
 *   6.4.1: Positional argument resolution (via PositionalResolver)
 *   6.4.2: Flag constraint validation (via FlagValidator)
 *
 *   All errors are collected and thrown together as `ParseErrors`.
 *
 * === How the Modal State Machine Is Used ===
 *
 * We use ModalStateMachine from @coding-adventures/state-machine to track the
 * parse mode. The three parse modes are:
 *   - "SCANNING": looking for flags and positionals
 *   - "FLAG_VALUE": next token is a value for a pending flag
 *   - "END_OF_FLAGS": all remaining tokens are positional (after seeing "--")
 *
 * Mode transitions:
 *   - SCANNING + non-boolean flag → FLAG_VALUE
 *   - FLAG_VALUE + (any token) → SCANNING
 *   - SCANNING + "--" → END_OF_FLAGS
 *   - SCANNING + POSIX positional → END_OF_FLAGS
 *
 * @module parser
 */

import { ModalStateMachine, DFA, transitionKey } from "@coding-adventures/state-machine";
import { SpecLoader } from "./spec-loader.js";
import { TokenClassifier } from "./token-classifier.js";
import { PositionalResolver, coerceValue } from "./positional-resolver.js";
import { FlagValidator } from "./flag-validator.js";
import { HelpGenerator } from "./help-generator.js";
import { ParseErrors } from "./errors.js";
import type { ParseError } from "./errors.js";
import type {
  CliSpec,
  CommandDef,
  FlagDef,
  ParserResult,
  ParseResult,
  HelpResult,
  VersionResult,
} from "./types.js";

// ---------------------------------------------------------------------------
// Fuzzy matching helper (Levenshtein distance)
// ---------------------------------------------------------------------------

/**
 * Compute the Levenshtein edit distance between two strings.
 *
 * Used for fuzzy suggestions in unknown_command and unknown_flag errors (§8.3).
 * If the closest match has distance ≤ 2, it is offered as a suggestion.
 *
 * The standard dynamic programming algorithm runs in O(m*n) where m and n
 * are the lengths of the two strings. For short CLI token lengths (< 50 chars),
 * this is essentially O(1) in practice.
 */
function levenshtein(a: string, b: string): number {
  const m = a.length;
  const n = b.length;
  // Create a (m+1) x (n+1) matrix. We use a flat array for cache efficiency.
  const dp: number[] = Array.from({ length: (m + 1) * (n + 1) }, () => 0);
  const idx = (i: number, j: number) => i * (n + 1) + j;

  for (let i = 0; i <= m; i++) dp[idx(i, 0)] = i;
  for (let j = 0; j <= n; j++) dp[idx(0, j)] = j;

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1] === b[j - 1]) {
        dp[idx(i, j)] = dp[idx(i - 1, j - 1)];
      } else {
        dp[idx(i, j)] =
          1 + Math.min(dp[idx(i - 1, j)], dp[idx(i, j - 1)], dp[idx(i - 1, j - 1)]);
      }
    }
  }
  return dp[idx(m, n)];
}

/**
 * Return the best fuzzy suggestion from `candidates` for `target`,
 * or undefined if no match has distance ≤ 2.
 */
function bestMatch(target: string, candidates: string[]): string | undefined {
  let best: string | undefined;
  let bestDist = 3; // threshold: only suggest if distance ≤ 2

  for (const cand of candidates) {
    const dist = levenshtein(target, cand);
    if (dist < bestDist) {
      bestDist = dist;
      best = cand;
    }
  }

  return best;
}

// ---------------------------------------------------------------------------
// Parse mode state machine construction
// ---------------------------------------------------------------------------

/**
 * Build the modal state machine for parse mode tracking.
 *
 * States: SCANNING, FLAG_VALUE, END_OF_FLAGS
 * Transitions:
 *   SCANNING    + "to_flag_value"  → FLAG_VALUE
 *   SCANNING    + "to_end_of_flags"→ END_OF_FLAGS
 *   FLAG_VALUE  + "to_scanning"    → SCANNING
 *   END_OF_FLAGS: no transitions (terminal)
 *
 * Each mode is a trivial single-state DFA that accepts everything.
 */
function buildParseModeMachine(): ModalStateMachine {
  const modes = ["SCANNING", "FLAG_VALUE", "END_OF_FLAGS"] as const;

  const makeSingleStateDfa = (stateName: string) =>
    new DFA(
      new Set([stateName]),
      new Set(["tick"]),
      new Map([[transitionKey(stateName, "tick"), stateName]]),
      stateName,
      new Set([stateName]),
    );

  const modeMap = new Map(modes.map((m) => [m, makeSingleStateDfa(m)]));

  return new ModalStateMachine(
    modeMap,
    new Map([
      [transitionKey("SCANNING", "to_flag_value"), "FLAG_VALUE"],
      [transitionKey("SCANNING", "to_end_of_flags"), "END_OF_FLAGS"],
      [transitionKey("FLAG_VALUE", "to_scanning"), "SCANNING"],
      // END_OF_FLAGS is terminal — no transitions out
    ]),
    "SCANNING",
  );
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/**
 * The main CLI argument parser.
 *
 * Loads the spec (via SpecLoader), then parses an argv array through the
 * three-phase algorithm. Returns a `ParserResult`, or throws `ParseErrors`
 * if the invocation is invalid, or `SpecError` if the spec itself is bad.
 *
 * @example
 * ```typescript
 * // Using a spec file path:
 * const parser = new Parser("./git-spec.json", process.argv);
 * const result = parser.parse();
 *
 * // Using an embedded spec object (for testing):
 * const parser = new Parser(spec, ["git", "commit", "-m", "fix bug"]);
 * ```
 */
export class Parser {
  private readonly _specLoader: SpecLoader | null;
  private readonly _inlineSpec: CliSpec | null;
  private readonly _argv: string[];

  /**
   * Create a parser from a spec file path.
   * @param specFilePath - Path to the JSON spec file.
   * @param argv - The full argv array (process.argv), including argv[0] (program path).
   */
  constructor(specFilePath: string, argv: string[]);
  /**
   * Create a parser from a pre-loaded CliSpec (useful in tests).
   * @param spec - The validated internal spec object.
   * @param argv - The full argv array.
   */
  constructor(spec: CliSpec, argv: string[]);
  constructor(specOrPath: string | CliSpec, argv: string[]) {
    if (typeof specOrPath === "string") {
      this._specLoader = new SpecLoader(specOrPath);
      this._inlineSpec = null;
    } else {
      this._specLoader = null;
      this._inlineSpec = specOrPath;
    }
    this._argv = argv;
  }

  /**
   * Parse the argv and return a `ParserResult`.
   *
   * Returns:
   * - `ParseResult` if parsing succeeds normally
   * - `HelpResult` if --help or -h is encountered
   * - `VersionResult` if --version is encountered
   *
   * Throws:
   * - `SpecError` if the spec file is invalid
   * - `ParseErrors` if one or more parse errors occurred
   */
  parse(): ParserResult {
    const spec = this._inlineSpec ?? this._specLoader!.load();

    // §6.1 Preprocessing: strip the program invocation prefix from argv.
    //
    // The parser accepts argv in two typical formats:
    //
    // Format A — Direct / test invocation:
    //   ["programname", "arg1", "arg2", ...]
    //   argv[0] is (or ends with) the spec name → strip 1
    //
    // Format B — Node.js process.argv:
    //   ["/usr/bin/node", "/path/to/script.js", "arg1", ...]
    //   argv[0] does NOT match the spec name → strip 2
    //
    // We detect format A by checking if argv[0] ends with the spec name.
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
        // Format A: strip 1 (argv[0] is the program name)
        workingArgv = rawArgv.slice(1);
      } else {
        // Format B: strip 2 (argv[0]=node, argv[1]=script)
        workingArgv = rawArgv.slice(2);
      }
    }

    const program = spec.name;

    // --- Phase 1: Routing ---
    const { commandPath, commandNode, remainingArgv, commandTokens } = this._phase1Route(
      spec,
      workingArgv,
    );

    // Build full command path including program name
    const fullCommandPath = [program, ...commandPath];

    // Build the active flag set for the resolved scope
    const activeFlags = this._buildActiveFlagSet(spec, commandPath, commandNode);

    // --- Phase 2: Scanning ---
    const { parsedFlags, positionalTokens, earlyReturn } = this._phase2Scan(
      spec,
      commandPath,
      commandNode,
      activeFlags,
      remainingArgv,
      workingArgv,
      fullCommandPath,
      commandTokens,
    );

    if (earlyReturn !== null) {
      return earlyReturn;
    }

    // --- Phase 3: Validation ---
    const errors: ParseError[] = [];

    // 6.4.1: Positional argument resolution
    const argDefs = commandNode ? commandNode.arguments : spec.arguments;
    const resolver = new PositionalResolver(argDefs);
    const { result: parsedArgs, errors: argErrors } = resolver.resolve(
      positionalTokens,
      parsedFlags,
      fullCommandPath,
    );
    errors.push(...argErrors);

    // 6.4.2: Flag constraint validation
    const exclusiveGroups = commandNode
      ? commandNode.mutuallyExclusiveGroups
      : spec.mutuallyExclusiveGroups;
    const validator = new FlagValidator(activeFlags, exclusiveGroups);
    const flagErrors = validator.validate(parsedFlags, fullCommandPath);
    errors.push(...flagErrors);

    if (errors.length > 0) {
      throw new ParseErrors(errors);
    }

    // Fill in defaults for any flags not present in parsedFlags
    this._applyFlagDefaults(activeFlags, parsedFlags);

    const parseResult: ParseResult = {
      program,
      commandPath: fullCommandPath,
      flags: parsedFlags,
      arguments: parsedArgs,
    };

    return parseResult;
  }

  // ---------------------------------------------------------------------------
  // Phase 1: Command Routing
  // ---------------------------------------------------------------------------

  /**
   * Walk argv consuming subcommand tokens and building commandPath.
   *
   * Returns:
   * - commandPath: the sequence of subcommand names matched
   * - commandNode: the leaf CommandDef (or null if at root)
   * - remainingArgv: the original argv with command tokens noted (we don't
   *   actually remove them — Phase 2 re-walks and skips them)
   */
  private _phase1Route(
    spec: CliSpec,
    argv: string[],
  ): {
    commandPath: string[];
    commandNode: CommandDef | null;
    remainingArgv: string[];
    /** The actual tokens from argv that were matched as commands (may be aliases). */
    commandTokens: string[];
  } {
    const commandPath: string[] = [];
    const commandTokens: string[] = [];
    let commandNode: CommandDef | null = null;
    let commands = spec.commands;

    // For traditional mode: check argv[0] for tar-style stacked flags
    // (handled in Phase 2 scanning, not routing)

    let i = 0;
    while (i < argv.length) {
      const token = argv[i];

      // Stop routing on end-of-flags
      if (token === "--") break;

      // Skip flag tokens during routing — they belong to Phase 2
      if (token.startsWith("-")) {
        // We need to know if this flag takes a value to skip correctly.
        // Build a rough lookup from active flags at this point.
        const allFlags = this._buildActiveFlagSet(spec, commandPath, commandNode);
        const skipCount = this._skipFlagToken(token, argv, i, allFlags);
        i += skipCount;
        continue;
      }

      // Is this token a known subcommand at the current level?
      const found = commands.find(
        (c) => c.name === token || c.aliases.includes(token),
      );

      if (found) {
        commandPath.push(found.name); // always use canonical name
        commandTokens.push(token);   // keep the original token (alias or name)
        commandNode = found;
        commands = found.commands;
        i++;
      } else {
        // Not a subcommand — stop routing (first positional or unknown token)
        break;
      }
    }

    return { commandPath, commandNode, remainingArgv: argv, commandTokens };
  }

  /**
   * Skip one flag token and its value (if non-boolean) during Phase 1.
   * Returns the number of tokens to advance (1 or 2).
   */
  private _skipFlagToken(
    token: string,
    argv: string[],
    i: number,
    activeFlags: FlagDef[],
  ): number {
    // "--" sentinel: advance 1
    if (token === "--") return 1;

    // "--name=value": entire thing is one token
    if (token.startsWith("--") && token.includes("=")) return 1;

    // Long flag without value: check if it's boolean
    if (token.startsWith("--")) {
      const name = token.slice(2);
      const flag = activeFlags.find((f) => f.long === name);
      if (flag && flag.type !== "boolean" && i + 1 < argv.length) return 2;
      return 1;
    }

    // Single dash (not "--"): might be short flag or single_dash_long
    if (token.startsWith("-") && token.length > 1) {
      const rest = token.slice(1);

      // Check single_dash_long
      const sdlFlag = activeFlags.find((f) => f.singleDashLong === rest);
      if (sdlFlag) {
        if (sdlFlag.type !== "boolean" && i + 1 < argv.length) return 2;
        return 1;
      }

      // Check if there's an "=" embedded (like "-n=5") — unlikely but handled
      if (rest.includes("=")) return 1;

      // Short flag: only advance 2 if single char and non-boolean
      const firstChar = rest[0];
      const shortFlag = activeFlags.find((f) => f.short === firstChar);
      if (shortFlag && shortFlag.type !== "boolean" && rest.length === 1 && i + 1 < argv.length) {
        return 2;
      }

      return 1;
    }

    return 1;
  }

  // ---------------------------------------------------------------------------
  // Phase 2: Scanning
  // ---------------------------------------------------------------------------

  /**
   * Re-walk argv, classifying and processing flags and positional tokens.
   *
   * Returns parsedFlags, positionalTokens, and an earlyReturn if --help or
   * --version was encountered.
   */
  private _phase2Scan(
    spec: CliSpec,
    commandPath: string[],
    commandNode: CommandDef | null,
    activeFlags: FlagDef[],
    _remainingArgv: string[],
    argv: string[],
    fullCommandPath: string[],
    commandTokens: string[],
  ): {
    parsedFlags: Record<string, unknown>;
    positionalTokens: string[];
    earlyReturn: HelpResult | VersionResult | null;
  } {
    const classifier = new TokenClassifier(activeFlags);
    const parsedFlags: Record<string, unknown> = {};
    const positionalTokens: string[] = [];
    const errors: ParseError[] = [];

    // Initialize the modal state machine for parse mode tracking
    const modeMachine = buildParseModeMachine();
    let parseMode: "SCANNING" | "FLAG_VALUE" | "END_OF_FLAGS" = "SCANNING";

    // Pending flag for FLAG_VALUE mode
    let pendingFlag: FlagDef | null = null;

    // For traditional mode: check if argv[0] should be treated as stacked flags
    let traditionalHandled = false;

    // The actual command tokens typed by the user (may be aliases).
    // We skip the first N non-flag tokens that match these tokens in order.
    // Using a cursor approach: skip commandTokens[0], then [1], etc.
    let commandPathCursor = 0;

    for (let i = 0; i < argv.length; i++) {
      const token = argv[i];

      // Handle traditional mode for first token
      if (
        spec.parsingMode === "traditional" &&
        !traditionalHandled &&
        parseMode === "SCANNING"
      ) {
        traditionalHandled = true;
        // If argv[0] doesn't start with "-" and isn't a known command name,
        // treat it as stacked flags without a leading dash.
        if (!token.startsWith("-")) {
          const isKnownCommand = spec.commands.some(
            (c) => c.name === token || c.aliases.includes(token),
          );
          if (!isKnownCommand) {
            // Try to classify as stacked flags
            const fakeToken = `-${token}`;
            const classified = classifier.classify(fakeToken);
            if (classified.type === "STACKED_FLAGS") {
              const stackResult = this._handleStackedFlags(
                classified.chars,
                classifier,
                parsedFlags,
                errors,
                fullCommandPath,
              );
              if (stackResult.pendingFlag !== null) {
                pendingFlag = stackResult.pendingFlag;
                modeMachine.switchMode("to_flag_value");
                parseMode = "FLAG_VALUE";
              }
              continue;
            }
            // Fall through to positional handling
          }
        }
      }

      // Skip command-path tokens (consumed in Phase 1).
      // We compare against the actual tokens typed (commandTokens), which
      // may be aliases. e.g., "rm" is the token even though commandPath has "remove".
      if (
        parseMode === "SCANNING" &&
        !token.startsWith("-") &&
        commandPathCursor < commandTokens.length &&
        token === commandTokens[commandPathCursor]
      ) {
        commandPathCursor++;
        continue;
      }

      switch (parseMode) {
        case "FLAG_VALUE": {
          // The entire token is the value for pendingFlag
          if (pendingFlag !== null) {
            const coerced = coerceValue(
              token,
              pendingFlag.type,
              pendingFlag.id,
              fullCommandPath,
              pendingFlag.enumValues,
            );
            if ("error" in coerced) {
              errors.push(coerced.error);
            } else {
              this._assignFlagValue(pendingFlag, coerced.value, parsedFlags, errors, fullCommandPath);
            }
            pendingFlag = null;
          }
          // Switch back to SCANNING
          modeMachine.switchMode("to_scanning");
          parseMode = "SCANNING";
          break;
        }

        case "END_OF_FLAGS": {
          // All tokens after "--" are positional
          positionalTokens.push(token);
          break;
        }

        case "SCANNING": {
          const classified = classifier.classify(token);

          switch (classified.type) {
            case "END_OF_FLAGS": {
              modeMachine.switchMode("to_end_of_flags");
              parseMode = "END_OF_FLAGS";
              break;
            }

            case "LONG_FLAG": {
              // Check for --help and --version early
              if (classified.name === "help" && spec.builtinFlags.help) {
                const helpGen = new HelpGenerator(spec, commandPath);
                const helpResult: HelpResult = {
                  text: helpGen.generate(),
                  commandPath: fullCommandPath,
                };
                return { parsedFlags, positionalTokens, earlyReturn: helpResult };
              }
              if (classified.name === "version" && spec.builtinFlags.version && spec.version) {
                const versionResult: VersionResult = { version: spec.version };
                return { parsedFlags, positionalTokens, earlyReturn: versionResult };
              }

              const flag = classifier.lookupByLong(classified.name);
              if (!flag) {
                const candidates = activeFlags
                  .filter((f) => f.long !== undefined)
                  .map((f) => `--${f.long!}`);
                const suggestion = bestMatch(`--${classified.name}`, candidates);
                errors.push({
                  errorType: "unknown_flag",
                  message: `Unknown flag '--${classified.name}'${suggestion ? `. Did you mean '${suggestion}'?` : ""}`,
                  suggestion,
                  context: fullCommandPath,
                });
              } else if (flag.type === "boolean") {
                this._assignFlagValue(flag, true, parsedFlags, errors, fullCommandPath);
              } else {
                pendingFlag = flag;
                modeMachine.switchMode("to_flag_value");
                parseMode = "FLAG_VALUE";
              }
              break;
            }

            case "LONG_FLAG_WITH_VALUE": {
              // Check for --help and --version
              if (classified.name === "help" && spec.builtinFlags.help) {
                const helpGen = new HelpGenerator(spec, commandPath);
                return {
                  parsedFlags,
                  positionalTokens,
                  earlyReturn: { text: helpGen.generate(), commandPath: fullCommandPath },
                };
              }

              const flag = classifier.lookupByLong(classified.name);
              if (!flag) {
                errors.push({
                  errorType: "unknown_flag",
                  message: `Unknown flag '--${classified.name}'`,
                  context: fullCommandPath,
                });
              } else {
                const coerced = coerceValue(
                  classified.value,
                  flag.type,
                  flag.id,
                  fullCommandPath,
                  flag.enumValues,
                );
                if ("error" in coerced) {
                  errors.push(coerced.error);
                } else {
                  this._assignFlagValue(flag, coerced.value, parsedFlags, errors, fullCommandPath);
                }
              }
              break;
            }

            case "SINGLE_DASH_LONG": {
              const flag = classifier.lookupBySingleDashLong(classified.name);
              if (!flag) {
                errors.push({
                  errorType: "unknown_flag",
                  message: `Unknown flag '-${classified.name}'`,
                  context: fullCommandPath,
                });
              } else if (flag.type === "boolean") {
                this._assignFlagValue(flag, true, parsedFlags, errors, fullCommandPath);
              } else {
                pendingFlag = flag;
                modeMachine.switchMode("to_flag_value");
                parseMode = "FLAG_VALUE";
              }
              break;
            }

            case "SHORT_FLAG": {
              // Check for -h (help)
              if (classified.char === "h" && spec.builtinFlags.help) {
                const helpGen = new HelpGenerator(spec, commandPath);
                return {
                  parsedFlags,
                  positionalTokens,
                  earlyReturn: { text: helpGen.generate(), commandPath: fullCommandPath },
                };
              }

              const flag = classifier.lookupByShort(classified.char);
              if (!flag) {
                errors.push({
                  errorType: "unknown_flag",
                  message: `Unknown flag '-${classified.char}'`,
                  context: fullCommandPath,
                });
              } else if (flag.type === "boolean") {
                this._assignFlagValue(flag, true, parsedFlags, errors, fullCommandPath);
              } else {
                pendingFlag = flag;
                modeMachine.switchMode("to_flag_value");
                parseMode = "FLAG_VALUE";
              }
              break;
            }

            case "SHORT_FLAG_WITH_VALUE": {
              const flag = classifier.lookupByShort(classified.char);
              if (!flag) {
                errors.push({
                  errorType: "unknown_flag",
                  message: `Unknown flag '-${classified.char}'`,
                  context: fullCommandPath,
                });
              } else {
                const coerced = coerceValue(
                  classified.value,
                  flag.type,
                  flag.id,
                  fullCommandPath,
                  flag.enumValues,
                );
                if ("error" in coerced) {
                  errors.push(coerced.error);
                } else {
                  this._assignFlagValue(flag, coerced.value, parsedFlags, errors, fullCommandPath);
                }
              }
              break;
            }

            case "STACKED_FLAGS": {
              const stackResult = this._handleStackedFlags(
                classified.chars,
                classifier,
                parsedFlags,
                errors,
                fullCommandPath,
              );
              // If the last char in the stack was a non-boolean flag, its
              // value is the next token → switch to FLAG_VALUE mode.
              if (stackResult.pendingFlag !== null) {
                pendingFlag = stackResult.pendingFlag;
                modeMachine.switchMode("to_flag_value");
                parseMode = "FLAG_VALUE";
              }
              break;
            }

            case "POSITIONAL": {
              if (spec.parsingMode === "posix") {
                // In POSIX mode, first positional ends flag scanning
                modeMachine.switchMode("to_end_of_flags");
                parseMode = "END_OF_FLAGS";
                positionalTokens.push(classified.value);
              } else {
                positionalTokens.push(classified.value);
              }
              break;
            }

            case "UNKNOWN_FLAG": {
              const candidates = activeFlags.flatMap((f) => {
                const parts: string[] = [];
                if (f.short) parts.push(`-${f.short}`);
                if (f.long) parts.push(`--${f.long}`);
                if (f.singleDashLong) parts.push(`-${f.singleDashLong}`);
                return parts;
              });
              const suggestion = bestMatch(classified.raw, candidates);
              errors.push({
                errorType: "unknown_flag",
                message: `Unknown flag '${classified.raw}'${suggestion ? `. Did you mean '${suggestion}'?` : ""}`,
                suggestion,
                context: fullCommandPath,
              });
              break;
            }
          }
          break;
        }
      }
    }

    // If we ended in FLAG_VALUE mode, the value was never provided
    if (parseMode === "FLAG_VALUE" && pendingFlag !== null) {
      errors.push({
        errorType: "invalid_value",
        message: `Flag '${this._flagDisplay(pendingFlag)}' requires a value`,
        context: fullCommandPath,
      });
    }

    // If there are scan errors, throw them now
    if (errors.length > 0) {
      throw new ParseErrors(errors);
    }

    return { parsedFlags, positionalTokens, earlyReturn: null };
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /**
   * Handle a STACKED_FLAGS token — a sequence of short flag chars like "lah".
   *
   * Each char is looked up in the short flag map. If a char is a non-boolean
   * flag and it's the last char, its value is the next token (caller handles
   * that via the FLAG_VALUE mode switch — but since we handle stacks inline,
   * we just record the flag without value here and the next token will be
   * consumed via normal FLAG_VALUE processing).
   *
   * Actually, per the spec, stacked flags with a non-boolean last char:
   * the value is the next token. We need to handle this by setting
   * pendingFlag after processing the stack. But since this method doesn't
   * have access to pendingFlag, we handle it differently: we process all
   * boolean chars, and for the final non-boolean char, we return it as
   * "pending". In practice, we just set parsedFlags[id] = null and let
   * the caller handle the pending state.
   *
   * For simplicity in this implementation, non-boolean chars in a stack
   * need their value from the next token — we handle by checking the
   * flag type after the loop.
   */
  private _handleStackedFlags(
    chars: string[],
    classifier: TokenClassifier,
    parsedFlags: Record<string, unknown>,
    errors: ParseError[],
    context: string[],
  ): { pendingFlag: FlagDef | null } {
    for (const c of chars) {
      const flag = classifier.lookupByShort(c);
      if (!flag) {
        errors.push({
          errorType: "invalid_stack",
          message: `Unknown flag '-${c}' in stack`,
          context,
        });
        continue;
      }
      if (flag.type === "boolean") {
        this._assignFlagValue(flag, true, parsedFlags, errors, context);
      } else {
        // Non-boolean in a stack: mark as pending (value comes next)
        // Return the pending flag so the caller can switch to FLAG_VALUE mode
        return { pendingFlag: flag };
      }
    }
    return { pendingFlag: null };
  }

  /**
   * Assign a value to a flag in parsedFlags, handling repeatable flags.
   *
   * - Non-repeatable + already present → duplicate_flag error
   * - Repeatable → append to array
   * - First occurrence → set directly
   */
  private _assignFlagValue(
    flag: FlagDef,
    value: unknown,
    parsedFlags: Record<string, unknown>,
    errors: ParseError[],
    context: string[],
  ): void {
    if (flag.repeatable) {
      if (!Array.isArray(parsedFlags[flag.id])) {
        parsedFlags[flag.id] = [];
      }
      (parsedFlags[flag.id] as unknown[]).push(value);
    } else {
      if (parsedFlags[flag.id] !== undefined && parsedFlags[flag.id] !== false && parsedFlags[flag.id] !== null) {
        errors.push({
          errorType: "duplicate_flag",
          message: `${this._flagDisplay(flag)} specified more than once`,
          context,
        });
      }
      parsedFlags[flag.id] = value;
    }
  }

  /**
   * Fill in default values for flags not set during scanning.
   *
   * After validation passes, every flag in scope should have a value.
   * Absent booleans get false; absent non-booleans get null (or their default).
   */
  private _applyFlagDefaults(
    activeFlags: FlagDef[],
    parsedFlags: Record<string, unknown>,
  ): void {
    for (const flag of activeFlags) {
      if (parsedFlags[flag.id] === undefined) {
        if (flag.repeatable) {
          parsedFlags[flag.id] = [];
        } else if (flag.type === "boolean") {
          parsedFlags[flag.id] = false;
        } else {
          parsedFlags[flag.id] = flag.default;
        }
      }
    }
  }

  /**
   * Build the active flag set for the resolved command scope.
   *
   * Per §6.3:
   *   active_flags = global_flags (if inherit_global_flags)
   *               + flags of every node in command_path
   *               + builtin flags (help, version)
   *
   * Note: `spec.flags` (root-level flags, §2.1) are only active when parsing
   * at root scope (commandPath is empty). Once inside a subcommand, only that
   * subcommand's own `flags` and `global_flags` apply.
   */
  private _buildActiveFlagSet(
    spec: CliSpec,
    commandPath: string[],
    commandNode: CommandDef | null,
  ): FlagDef[] {
    const flags: FlagDef[] = [];
    const seenIds = new Set<string>();

    const addFlags = (newFlags: FlagDef[]) => {
      for (const f of newFlags) {
        if (!seenIds.has(f.id)) {
          seenIds.add(f.id);
          flags.push(f);
        }
      }
    };

    // Global flags are valid at every level (unless overridden by inherit_global_flags)
    if (!commandNode || commandNode.inheritGlobalFlags) {
      addFlags(spec.globalFlags);
    }

    // Root-level flags: only active when at root scope (no subcommand routing)
    if (commandPath.length === 0) {
      addFlags(spec.flags);
    }

    // Flags from each command in the path
    if (commandPath.length > 0) {
      let commands = spec.commands;
      for (const segment of commandPath) {
        const cmd = commands.find(
          (c) => c.name === segment || c.aliases.includes(segment),
        );
        if (cmd) {
          addFlags(cmd.flags);
          commands = cmd.commands;
        }
      }
    }

    // Add builtin help flag
    if (spec.builtinFlags.help) {
      addFlags([
        {
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
        },
      ]);
    }

    // Add builtin version flag if version is present in spec
    if (spec.builtinFlags.version && spec.version) {
      addFlags([
        {
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
        },
      ]);
    }

    return flags;
  }

  /** Format a flag for display in error messages. */
  private _flagDisplay(flag: FlagDef): string {
    const parts: string[] = [];
    if (flag.short) parts.push(`-${flag.short}`);
    if (flag.long) parts.push(`--${flag.long}`);
    if (flag.singleDashLong) parts.push(`-${flag.singleDashLong}`);
    return parts.join("/") || `--${flag.id}`;
  }
}
