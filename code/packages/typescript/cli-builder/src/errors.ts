/**
 * errors.ts — Error hierarchy for the CLI Builder library.
 *
 * === Error Design Philosophy ===
 *
 * CLI Builder distinguishes two kinds of failure:
 *
 * 1. **Spec errors** — The JSON spec itself is wrong. A duplicate flag ID,
 *    a circular `requires` dependency, a missing `enum_values` array. These
 *    are programming errors in the spec file, not user mistakes. They are
 *    fatal: the library refuses to parse anything until the spec is fixed.
 *
 * 2. **Parse errors** — The user typed the wrong thing. Unknown flag, missing
 *    required argument, conflicting flags. These are user mistakes. The library
 *    collects *all* of them and reports everything wrong at once, so the user
 *    can fix their invocation in one shot rather than playing whack-a-mole.
 *
 * Both extend `CliBuilderError`, which extends `Error`. This means callers can
 * either catch the broad base class or discriminate on the specific subclass.
 *
 * === Why collect all parse errors? ===
 *
 * Imagine running `git commit` and getting told only about the missing `-m` flag.
 * You add `-m`, run again, and now you hear about the conflicting flags. You fix
 * those, run again... Three round trips for three independent errors. Collecting
 * all errors upfront — a "multiple errors" model — gives users a complete picture
 * of what went wrong, which is the UX standard set by modern compilers and linters.
 *
 * @module errors
 */

// ---------------------------------------------------------------------------
// Base error class
// ---------------------------------------------------------------------------

/**
 * Base class for all errors thrown by CLI Builder.
 *
 * Callers can catch this to handle any CLI Builder failure generically,
 * then inspect the concrete subclass for specific handling.
 *
 * @example
 * ```typescript
 * try {
 *   parser.parse();
 * } catch (e) {
 *   if (e instanceof CliBuilderError) {
 *     console.error("CLI Builder error:", e.message);
 *   }
 * }
 * ```
 */
export class CliBuilderError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CliBuilderError";
    // Maintain proper prototype chain for instanceof checks in TypeScript.
    // This is required when extending built-in classes in ES5 transpiled code.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// ---------------------------------------------------------------------------
// SpecError — fatal, thrown at load time
// ---------------------------------------------------------------------------

/**
 * Thrown when the JSON spec file is invalid.
 *
 * Spec errors are **load-time failures**. They indicate a bug in the spec
 * file that must be fixed before the library can be used at all. Examples:
 *
 * - Missing required fields (`name`, `description`, `cli_builder_spec_version`)
 * - Duplicate flag IDs in the same scope
 * - A `conflicts_with` referencing a non-existent flag ID
 * - A circular `requires` dependency (A requires B requires A)
 * - `type: "enum"` without a non-empty `enum_values` array
 * - More than one variadic argument in the same scope
 *
 * @example
 * ```typescript
 * throw new SpecError(
 *   'Circular requires dependency: "verbose" → "quiet" → "verbose"'
 * );
 * ```
 */
export class SpecError extends CliBuilderError {
  constructor(message: string) {
    super(message);
    this.name = "SpecError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// ---------------------------------------------------------------------------
// ParseError — one entry in the error list
// ---------------------------------------------------------------------------

/**
 * One individual parse error, with machine-readable type, human message,
 * optional suggestion, and the command path where the error was detected.
 *
 * Parse errors are not thrown individually. They accumulate in a `ParseErrors`
 * instance, which is thrown at the end of validation when at least one error
 * exists.
 *
 * === Fields ===
 *
 * - `errorType` — Snake_case identifier for the error category. Useful for
 *   programmatic handling (e.g., displaying a custom help tip for
 *   `missing_required_flag`). See §8.2 of the spec for all error types.
 *
 * - `message` — Human-readable sentence explaining what went wrong, suitable
 *   for display to the end user.
 *
 * - `suggestion` — Optional corrective hint, e.g., a fuzzy match for a
 *   typo'd flag name. Only present when the library can make a useful guess.
 *
 * - `context` — The `command_path` at the point where the error was detected,
 *   e.g., `["git", "remote", "add"]`. Helps the user understand which
 *   subcommand's rules triggered the error.
 *
 * @example
 * ```typescript
 * const err: ParseError = {
 *   errorType: "unknown_flag",
 *   message: "Unknown flag '--mesage'. Did you mean '--message'?",
 *   suggestion: "--message",
 *   context: ["git", "commit"],
 * };
 * ```
 */
export interface ParseError {
  /** Machine-readable error category (snake_case). */
  readonly errorType: string;
  /** Human-readable explanation. */
  readonly message: string;
  /** Optional fuzzy-match suggestion or corrective hint. */
  readonly suggestion?: string;
  /** The command_path where this error was detected. */
  readonly context: string[];
}

// ---------------------------------------------------------------------------
// ParseErrors — the thrown exception holding all collected errors
// ---------------------------------------------------------------------------

/**
 * Thrown at the end of a parse that produced one or more errors.
 *
 * Rather than stopping at the first problem, CLI Builder collects every
 * error it encounters during scanning and validation, then throws this
 * exception containing all of them. This gives users the full picture
 * of what is wrong with their invocation.
 *
 * @example
 * ```typescript
 * try {
 *   parser.parse();
 * } catch (e) {
 *   if (e instanceof ParseErrors) {
 *     for (const err of e.errors) {
 *       console.error(`[${err.errorType}] ${err.message}`);
 *       if (err.suggestion) console.error(`  Suggestion: ${err.suggestion}`);
 *     }
 *   }
 * }
 * ```
 */
export class ParseErrors extends CliBuilderError {
  /** All errors collected during this parse attempt. */
  public readonly errors: ParseError[];

  constructor(errors: ParseError[]) {
    // Build a combined message so that the Error.message property is useful
    // even if the caller doesn't inspect the individual errors.
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
