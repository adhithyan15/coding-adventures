/**
 * positional-resolver.ts — Assign positional tokens to argument slots.
 *
 * === The Positional Resolution Problem ===
 *
 * After Phase 2 (scanning), we have a flat list of positional tokens —
 * every token that wasn't a flag or subcommand name. The resolver's job
 * is to assign them to the named argument slots defined by the spec.
 *
 * For simple tools like `head`, this is trivial: one arg slot, one token.
 * For `cp`, it's trickier: any number of SOURCE files, then exactly one
 * DEST. The variadic SOURCE must consume everything except the last token.
 *
 * === The Algorithm (§6.4.1) ===
 *
 * Case 1 — No variadic argument:
 *   Consume tokens left-to-right, one per arg slot. Too few tokens = missing
 *   required argument errors. Too many = too_many_arguments error.
 *
 * Case 2 — One variadic argument:
 *   Partition the token list into three segments:
 *   - LEADING: tokens consumed by arg slots before the variadic
 *   - VARIADIC: tokens consumed by the variadic arg itself
 *   - TRAILING: tokens consumed from the end by arg slots after the variadic
 *
 *   Example: cp SOURCE... DEST
 *     tokens = ["a.txt", "b.txt", "c.txt", "/dest/"]
 *     leading  = [] (variadic is first)
 *     trailing = ["/dest/"]  (DEST is after variadic)
 *     variadic = ["a.txt", "b.txt", "c.txt"]
 *
 * === Last-wins algorithm ===
 *
 * The term "last-wins" refers to the trailing args consuming from the END
 * of the token list. This is what makes `cp src1 src2 dest` work correctly:
 * the DEST slot "wins" the last token, and SOURCE gets everything else.
 *
 * @module positional-resolver
 */

import { statSync } from "fs";
import type { ArgDef } from "./types.js";
import type { ParseError } from "./errors.js";

// ---------------------------------------------------------------------------
// Value coercion
// ---------------------------------------------------------------------------

/**
 * Coerce a string token to the native type specified by `type`.
 *
 * Returns the coerced value, or a ParseError if coercion fails.
 *
 * Filesystem existence checks (for `file` and `directory` types) are
 * performed here at parse time. Permission errors are treated as
 * invalid_value errors rather than crashes.
 */
export function coerceValue(
  raw: string,
  type: string,
  argId: string,
  context: string[],
  enumValues: string[] = [],
): { value: unknown } | { error: ParseError } {
  switch (type) {
    case "boolean":
      // Booleans are set by flag presence, not by positional values.
      // If we somehow end up here, treat as a raw string.
      return { value: raw };

    case "string":
      if (raw.length === 0) {
        return {
          error: {
            errorType: "invalid_value",
            message: `Value for "${argId}" must be a non-empty string`,
            context,
          },
        };
      }
      return { value: raw };

    case "integer": {
      const n = Number(raw);
      if (!Number.isInteger(n) || isNaN(n)) {
        return {
          error: {
            errorType: "invalid_value",
            message: `Invalid integer for "${argId}": '${raw}'`,
            context,
          },
        };
      }
      return { value: n };
    }

    case "float": {
      const f = Number(raw);
      if (isNaN(f)) {
        return {
          error: {
            errorType: "invalid_value",
            message: `Invalid float for "${argId}": '${raw}'`,
            context,
          },
        };
      }
      return { value: f };
    }

    case "path":
      // Syntactically valid path — we accept any non-empty string.
      // The "-" sentinel for stdin is also valid.
      if (raw.length === 0) {
        return {
          error: {
            errorType: "invalid_value",
            message: `Path value for "${argId}" must be non-empty`,
            context,
          },
        };
      }
      return { value: raw };

    case "file": {
      // Must refer to an existing, readable file at parse time.
      // We use a try/catch to handle permission errors gracefully.
      try {
        const stat = statSync(raw);
        if (!stat.isFile()) {
          return {
            error: {
              errorType: "invalid_value",
              message: `"${raw}" is not a file (for argument "${argId}")`,
              context,
            },
          };
        }
      } catch {
        return {
          error: {
            errorType: "invalid_value",
            message: `File not found or not accessible: "${raw}" (for argument "${argId}")`,
            context,
          },
        };
      }
      return { value: raw };
    }

    case "directory": {
      try {
        const stat = statSync(raw);
        if (!stat.isDirectory()) {
          return {
            error: {
              errorType: "invalid_value",
              message: `"${raw}" is not a directory (for argument "${argId}")`,
              context,
            },
          };
        }
      } catch {
        return {
          error: {
            errorType: "invalid_value",
            message: `Directory not found or not accessible: "${raw}" (for argument "${argId}")`,
            context,
          },
        };
      }
      return { value: raw };
    }

    case "enum":
      if (!enumValues.includes(raw)) {
        return {
          error: {
            errorType: "invalid_enum_value",
            message: `Invalid value '${raw}' for "${argId}". Must be one of: ${enumValues.join(", ")}`,
            context,
          },
        };
      }
      return { value: raw };

    default:
      return { value: raw };
  }
}

// ---------------------------------------------------------------------------
// PositionalResolver
// ---------------------------------------------------------------------------

/**
 * Resolves a flat list of positional tokens into named argument slots.
 *
 * Implements the §6.4.1 partitioning algorithm with "last-wins" for
 * trailing non-variadic arguments after a variadic one.
 *
 * @example
 * ```typescript
 * // cp SOURCE... DEST
 * const resolver = new PositionalResolver(cpArgDefs);
 * const result = resolver.resolve(["a.txt", "b.txt", "/tmp/"], {});
 * // { source: ["a.txt", "b.txt"], dest: "/tmp/" }
 * ```
 */
export class PositionalResolver {
  private readonly _argDefs: ArgDef[];

  constructor(argDefs: ArgDef[]) {
    this._argDefs = argDefs;
  }

  /**
   * Assign tokens to argument slots. Returns a plain object mapping
   * argument IDs to coerced values, and a list of any validation errors.
   *
   * @param tokens - The positional tokens collected during scanning.
   * @param parsedFlags - The flags already parsed (used for required_unless_flag).
   * @param context - The command path for error messages.
   */
  resolve(
    tokens: string[],
    parsedFlags: Record<string, unknown>,
    context: string[],
  ): { result: Record<string, unknown>; errors: ParseError[] } {
    const result: Record<string, unknown> = {};
    const errors: ParseError[] = [];

    // Filter argDefs to remove slots that are skipped due to required_unless_flag.
    // When required_unless_flag is satisfied (i.e., one of the named flags is present),
    // that argument becomes fully optional — and should not consume a positional token
    // unless there are enough tokens for all other slots too.
    // We use "effective argDefs" = only those that should participate in assignment.
    const effectiveDefs = this._argDefs.filter((def) => {
      if (def.requiredUnlessFlag.length === 0) return true;
      // If the flag exemption is NOT satisfied, this arg IS required → include it.
      const isExempt = def.requiredUnlessFlag.some(
        (fid) => {
          const val = parsedFlags[fid];
          if (Array.isArray(val)) return val.length > 0;
          return val !== undefined && val !== false && val !== null;
        }
      );
      // If exempt: include only if there are enough tokens for all non-exempt defs.
      // Simple heuristic: if exempt, move this to "optional" and only fill if tokens left.
      return !isExempt; // will be placed as "optional" below
    });

    // Slots that are exempt (optional due to flag presence) — append at end as optional
    const exemptDefs = this._argDefs.filter((def) => {
      if (def.requiredUnlessFlag.length === 0) return false;
      const isExempt = def.requiredUnlessFlag.some((fid) => {
        const val = parsedFlags[fid];
        if (Array.isArray(val)) return val.length > 0;
        return val !== undefined && val !== false && val !== null;
      });
      return isExempt;
    });

    // Use effective defs for the main resolution algorithm.
    // Exempt args default to null since they're already handled by their flag.
    for (const def of exemptDefs) {
      result[def.id] = def.default;
    }

    const argDefs = effectiveDefs;

    // Find the variadic argument (if any)
    const variadicIdx = argDefs.findIndex((a) => a.variadic);

    if (variadicIdx === -1) {
      // No variadic: one-to-one assignment in order
      this._resolveNonVariadic(
        tokens,
        argDefs,
        parsedFlags,
        context,
        result,
        errors,
      );
    } else {
      // Has a variadic: partition around it
      this._resolveWithVariadic(
        tokens,
        argDefs,
        variadicIdx,
        parsedFlags,
        context,
        result,
        errors,
      );
    }

    return { result, errors };
  }

  /** Handle the simple case: no variadic argument. */
  private _resolveNonVariadic(
    tokens: string[],
    argDefs: ArgDef[],
    parsedFlags: Record<string, unknown>,
    context: string[],
    result: Record<string, unknown>,
    errors: ParseError[],
  ): void {
    for (let i = 0; i < argDefs.length; i++) {
      const def = argDefs[i];
      if (i < tokens.length) {
        const coerced = coerceValue(
          tokens[i],
          def.type,
          def.id,
          context,
          def.enumValues,
        );
        if ("error" in coerced) {
          errors.push(coerced.error);
        } else {
          result[def.id] = coerced.value;
        }
      } else {
        // No token for this slot
        const isExempt = def.requiredUnlessFlag.some(
          (fid) => parsedFlags[fid] !== undefined && parsedFlags[fid] !== false,
        );
        if (def.required && !isExempt) {
          errors.push({
            errorType: "missing_required_argument",
            message: `Missing required argument: <${def.name}>`,
            context,
          });
        } else {
          // Use default value
          result[def.id] = def.default;
        }
      }
    }

    // Too many tokens?
    if (tokens.length > argDefs.length) {
      errors.push({
        errorType: "too_many_arguments",
        message: `Expected at most ${argDefs.length} argument(s), got ${tokens.length}`,
        context,
      });
    }
  }

  /** Handle the variadic case: partition tokens into leading/variadic/trailing. */
  private _resolveWithVariadic(
    tokens: string[],
    argDefs: ArgDef[],
    variadicIdx: number,
    parsedFlags: Record<string, unknown>,
    context: string[],
    result: Record<string, unknown>,
    errors: ParseError[],
  ): void {
    const leadingDefs = argDefs.slice(0, variadicIdx);
    const variadicDef = argDefs[variadicIdx];
    const trailingDefs = argDefs.slice(variadicIdx + 1);

    // Assign leading arguments
    for (let i = 0; i < leadingDefs.length; i++) {
      const def = leadingDefs[i];
      if (i < tokens.length) {
        const coerced = coerceValue(
          tokens[i],
          def.type,
          def.id,
          context,
          def.enumValues,
        );
        if ("error" in coerced) {
          errors.push(coerced.error);
        } else {
          result[def.id] = coerced.value;
        }
      } else {
        const isExempt = def.requiredUnlessFlag.some(
          (fid) => parsedFlags[fid] !== undefined && parsedFlags[fid] !== false,
        );
        if (def.required && !isExempt) {
          errors.push({
            errorType: "missing_required_argument",
            message: `Missing required argument: <${def.name}>`,
            context,
          });
        } else {
          result[def.id] = def.default;
        }
      }
    }

    // Compute how many tokens are available after leading args
    const remainingTokens = tokens.slice(leadingDefs.length);

    // Assign trailing arguments from the END (last-wins)
    const trailingStart = remainingTokens.length - trailingDefs.length;

    for (let i = 0; i < trailingDefs.length; i++) {
      const def = trailingDefs[i];
      const tokenIdx = trailingStart + i;
      if (tokenIdx >= 0 && tokenIdx < remainingTokens.length) {
        const coerced = coerceValue(
          remainingTokens[tokenIdx],
          def.type,
          def.id,
          context,
          def.enumValues,
        );
        if ("error" in coerced) {
          errors.push(coerced.error);
        } else {
          result[def.id] = coerced.value;
        }
      } else {
        const isExempt = def.requiredUnlessFlag.some(
          (fid) => parsedFlags[fid] !== undefined && parsedFlags[fid] !== false,
        );
        if (def.required && !isExempt) {
          errors.push({
            errorType: "missing_required_argument",
            message: `Missing required argument: <${def.name}>`,
            context,
          });
        } else {
          result[def.id] = def.default;
        }
      }
    }

    // Variadic gets everything between leading and trailing
    const variadicEndIdx = trailingStart > 0 ? trailingStart : remainingTokens.length;
    const variadicTokens = remainingTokens.slice(
      0,
      Math.max(0, variadicEndIdx),
    );
    const count = variadicTokens.length;

    // Validate variadic_min and variadic_max
    const isExempt = variadicDef.requiredUnlessFlag.some(
      (fid) => parsedFlags[fid] !== undefined && parsedFlags[fid] !== false,
    );
    if (count < variadicDef.variadicMin && !isExempt) {
      errors.push({
        errorType: "too_few_arguments",
        message: `Expected at least ${variadicDef.variadicMin} <${variadicDef.name}>, got ${count}`,
        context,
      });
    } else if (
      variadicDef.variadicMax !== null &&
      count > variadicDef.variadicMax
    ) {
      errors.push({
        errorType: "too_many_arguments",
        message: `Expected at most ${variadicDef.variadicMax} <${variadicDef.name}>, got ${count}`,
        context,
      });
    }

    // Coerce and assign variadic values
    const coercedVariadic: unknown[] = [];
    for (const tok of variadicTokens) {
      const coerced = coerceValue(
        tok,
        variadicDef.type,
        variadicDef.id,
        context,
        variadicDef.enumValues,
      );
      if ("error" in coerced) {
        errors.push(coerced.error);
      } else {
        coercedVariadic.push(coerced.value);
      }
    }

    // Apply default for variadic if no tokens (and no error)
    if (coercedVariadic.length === 0 && !errors.some(e => e.errorType === "too_few_arguments")) {
      result[variadicDef.id] = variadicDef.default !== null ? [variadicDef.default] : [];
    } else {
      result[variadicDef.id] = coercedVariadic;
    }
  }
}
