/**
 * token-classifier.ts — Token classification DFA for argv tokens.
 *
 * === What is Token Classification? ===
 *
 * Before the parser can process argv, it needs to know what kind of token
 * each string is. Is "-l" a short flag? A stacked sequence like "-lah"?
 * Or is "-classpath" a single-dash-long flag? The classifier answers these
 * questions by walking each token character-by-character.
 *
 * This module implements §5 of the spec: the Token Classification DFA and
 * the longest-match-first disambiguation rules.
 *
 * === Token Types ===
 *
 * The classifier emits one of these typed events for each token:
 *
 * ```
 * "--"              → END_OF_FLAGS
 * "--name"          → LONG_FLAG("name")
 * "--name=value"    → LONG_FLAG_WITH_VALUE("name", "value")
 * "-classpath"      → SINGLE_DASH_LONG("classpath")   (longest-match)
 * "-x"              → SHORT_FLAG("x")
 * "-xVAL"           → SHORT_FLAG_WITH_VALUE("x", "VAL")  (non-boolean, inline val)
 * "-lah"            → STACKED_FLAGS(["l","a","h"])    (all boolean)
 * "hello"           → POSITIONAL("hello")
 * "-"               → POSITIONAL("-")                  (stdin sentinel)
 * "-xyz"            → unknown → UNKNOWN_FLAG("-xyz")
 * ```
 *
 * === Longest-match-first (§5.2) ===
 *
 * When a token starts with a single `-` followed by multiple chars, the
 * rules are applied in priority order:
 *
 * 1. Does the substring after `-` exactly match a `single_dash_long` flag? → SINGLE_DASH_LONG
 * 2. Does `token[1]` match a declared short flag? → SHORT_FLAG or SHORT_FLAG_WITH_VALUE
 * 3. Is the entire token a sequence of valid boolean short flags? → STACKED_FLAGS
 * 4. No match → UNKNOWN_FLAG
 *
 * This ensures `-classpath` is never mistakenly decomposed as stacked short flags.
 *
 * @module token-classifier
 */

import type { FlagDef } from "./types.js";

// ---------------------------------------------------------------------------
// Token event types
// ---------------------------------------------------------------------------

/** Token was exactly "--" — all subsequent tokens are positional. */
export interface EndOfFlagsToken {
  type: "END_OF_FLAGS";
}

/** Token was "--name" (no equals sign). */
export interface LongFlagToken {
  type: "LONG_FLAG";
  name: string;
}

/** Token was "--name=value". */
export interface LongFlagWithValueToken {
  type: "LONG_FLAG_WITH_VALUE";
  name: string;
  value: string;
}

/** Token matched a `single_dash_long` flag. */
export interface SingleDashLongToken {
  type: "SINGLE_DASH_LONG";
  name: string;
}

/** Token was "-x" where x is a declared short flag. */
export interface ShortFlagToken {
  type: "SHORT_FLAG";
  char: string;
}

/**
 * Token was "-xVALUE" where x is a non-boolean short flag.
 * The value is the remainder of the token after the flag character.
 */
export interface ShortFlagWithValueToken {
  type: "SHORT_FLAG_WITH_VALUE";
  char: string;
  value: string;
}

/** Token was "-lah" — all characters are valid boolean short flags. */
export interface StackedFlagsToken {
  type: "STACKED_FLAGS";
  chars: string[];
}

/** Token was a positional value (doesn't start with -, or is exactly "-"). */
export interface PositionalToken {
  type: "POSITIONAL";
  value: string;
}

/** Token started with - but matched no known flag. */
export interface UnknownFlagToken {
  type: "UNKNOWN_FLAG";
  raw: string;
}

/**
 * Union of all token event types.
 *
 * The parser pattern-matches on the `type` discriminant to drive the
 * modal state machine.
 */
export type TokenEvent =
  | EndOfFlagsToken
  | LongFlagToken
  | LongFlagWithValueToken
  | SingleDashLongToken
  | ShortFlagToken
  | ShortFlagWithValueToken
  | StackedFlagsToken
  | PositionalToken
  | UnknownFlagToken;

// ---------------------------------------------------------------------------
// TokenClassifier
// ---------------------------------------------------------------------------

/**
 * Classifies a single argv token into a typed event.
 *
 * Constructed with the active flags for the current command scope. Call
 * `classify(token)` for each token in the re-walked argv.
 *
 * The classifier is stateless: each call to `classify()` is independent.
 * The state machine (in parser.ts) tracks parse mode across tokens.
 *
 * @example
 * ```typescript
 * const classifier = new TokenClassifier(activeFlags);
 * classifier.classify("--output"); // { type: "LONG_FLAG", name: "output" }
 * classifier.classify("-lah");     // { type: "STACKED_FLAGS", chars: ["l","a","h"] }
 * classifier.classify("foo.txt");  // { type: "POSITIONAL", value: "foo.txt" }
 * ```
 */
export class TokenClassifier {
  // --- Lookup maps for O(1) flag lookups ---
  //
  // We precompute three maps from the active flags so classify() is fast.
  // Each map key is the flag identifier (the chars after the dash prefix).

  /** Map from short char to FlagDef. */
  private readonly _shortMap: Map<string, FlagDef>;
  /** Map from long name to FlagDef. */
  private readonly _longMap: Map<string, FlagDef>;
  /** Map from single_dash_long name to FlagDef. */
  private readonly _sdlMap: Map<string, FlagDef>;

  constructor(activeFlags: FlagDef[]) {
    this._shortMap = new Map();
    this._longMap = new Map();
    this._sdlMap = new Map();

    for (const flag of activeFlags) {
      if (flag.short !== undefined && !this._shortMap.has(flag.short)) {
        this._shortMap.set(flag.short, flag);
      }
      if (flag.long !== undefined && !this._longMap.has(flag.long)) {
        this._longMap.set(flag.long, flag);
      }
      if (flag.singleDashLong !== undefined && !this._sdlMap.has(flag.singleDashLong)) {
        this._sdlMap.set(flag.singleDashLong, flag);
      }
    }
  }

  /**
   * Classify a single argv token and return the appropriate TokenEvent.
   *
   * Algorithm (see §5 of the spec):
   *
   * 1. Exactly "--"? → END_OF_FLAGS
   * 2. Starts with "--"?
   *    a. Contains "="? → LONG_FLAG_WITH_VALUE
   *    b. Otherwise → LONG_FLAG
   * 3. Exactly "-"? → POSITIONAL("-")
   * 4. Starts with "-" (and length > 1)?
   *    Apply longest-match-first (§5.2):
   *    a. Single_dash_long exact match → SINGLE_DASH_LONG
   *    b. First char is a known short flag:
   *       - Boolean, rest is empty → SHORT_FLAG
   *       - Boolean, rest is non-empty → classify rest recursively as possible stack
   *       - Non-boolean, rest is non-empty → SHORT_FLAG_WITH_VALUE
   *       - Non-boolean, rest is empty → SHORT_FLAG (value is next token)
   *    c. All chars are boolean short flags → STACKED_FLAGS
   *    d. None of the above → UNKNOWN_FLAG
   * 5. Anything else → POSITIONAL
   */
  classify(token: string): TokenEvent {
    // Case 1: End-of-flags sentinel
    if (token === "--") {
      return { type: "END_OF_FLAGS" };
    }

    // Case 2: Long flags (start with "--")
    if (token.startsWith("--")) {
      const rest = token.slice(2);
      const eqIdx = rest.indexOf("=");
      if (eqIdx !== -1) {
        // "--name=value"
        return {
          type: "LONG_FLAG_WITH_VALUE",
          name: rest.slice(0, eqIdx),
          value: rest.slice(eqIdx + 1),
        };
      }
      // "--name"
      return { type: "LONG_FLAG", name: rest };
    }

    // Case 3: Lone "-" = stdin/stdout positional
    if (token === "-") {
      return { type: "POSITIONAL", value: "-" };
    }

    // Case 4: Single-dash tokens
    if (token.startsWith("-") && token.length > 1) {
      return this._classifySingleDash(token);
    }

    // Case 5: Everything else is positional
    return { type: "POSITIONAL", value: token };
  }

  /**
   * Classify a token that starts with a single dash (not "--").
   *
   * Implements the longest-match-first disambiguation from §5.2.
   */
  private _classifySingleDash(token: string): TokenEvent {
    const rest = token.slice(1); // everything after the "-"

    // Rule 1: exact single_dash_long match (longest-match-first)
    //
    // "-classpath" vs "-c" (short flag 'c'): Rule 1 fires first.
    // We check every possible single_dash_long prefix, sorted by length
    // descending so the longest match wins. For an exact-match system,
    // we check the full rest string.
    if (this._sdlMap.has(rest)) {
      return { type: "SINGLE_DASH_LONG", name: rest };
    }

    // Rule 2: first character is a known short flag
    const firstChar = rest[0];
    const firstFlag = this._shortMap.get(firstChar);

    if (firstFlag !== undefined) {
      const remainder = rest.slice(1);

      // v1.1: Count flags behave like booleans for token classification
      // purposes — they consume no value token.
      if (firstFlag.type === "boolean" || firstFlag.type === "count") {
        if (remainder.length === 0) {
          // "-x" where x is a boolean flag
          return { type: "SHORT_FLAG", char: firstChar };
        }
        // "-xyz" where x is boolean: attempt to classify the rest as a stack
        // Starting from remainder, check if all chars are boolean short flags.
        const stackResult = this._tryStack(firstChar, remainder);
        if (stackResult !== null) {
          return stackResult;
        }
        // Partial stack failure: return just SHORT_FLAG for the first char
        // and let the unknown flag be discovered on the next token pass.
        // Actually per spec Rule 3: walk each char; if unknown, emit UNKNOWN_FLAG.
        return this._classifyStack(rest);
      } else {
        // Non-boolean flag: remainder (if any) is the inline value
        if (remainder.length === 0) {
          // "-x" — value will be the next token
          return { type: "SHORT_FLAG", char: firstChar };
        }
        // "-xVALUE"
        return { type: "SHORT_FLAG_WITH_VALUE", char: firstChar, value: remainder };
      }
    }

    // Rule 3: attempt full stack classification
    return this._classifyStack(rest);
  }

  /**
   * Attempt to classify a string of characters as stacked boolean short flags.
   *
   * Returns STACKED_FLAGS if all characters are valid boolean short flags,
   * or UNKNOWN_FLAG if any character is not a valid boolean short flag
   * (except the last, which may be non-boolean with remaining chars as value).
   *
   * Per §5.2 Rule 3:
   * - Walk each character.
   * - If boolean: record it and continue.
   * - If non-boolean (last in string): also valid (value from next token).
   * - If non-boolean (not last): the remainder is its inline value → also valid.
   * - If not a known flag at all: emit UNKNOWN_FLAG.
   */
  private _classifyStack(chars: string): TokenEvent {
    const flagChars: string[] = [];

    for (let i = 0; i < chars.length; i++) {
      const c = chars[i];
      const flag = this._shortMap.get(c);

      if (flag === undefined) {
        // Unknown character in stack
        return {
          type: "UNKNOWN_FLAG",
          raw: `-${chars}`,
        };
      }

      if (flag.type !== "boolean" && flag.type !== "count") {
        // Non-boolean/non-count flag in the middle or at the end
        const remainder = chars.slice(i + 1);
        if (remainder.length > 0) {
          // The remainder is the inline value for this non-boolean flag.
          // We can still emit a valid STACKED_FLAGS with the booleans,
          // plus a SHORT_FLAG_WITH_VALUE for the non-boolean. However,
          // the spec says the last char can be non-boolean with remaining
          // chars as value. We handle this by including the boolean chars
          // and returning SHORT_FLAG_WITH_VALUE for the non-boolean.
          //
          // Since we can only return one token event per classify() call,
          // and this case (non-boolean not last) only works if we're at
          // the end of the boolean chain, we return what we have:
          if (flagChars.length > 0) {
            // Return STACKED_FLAGS for the boolean prefix; the caller will
            // need to handle the remaining "-cVALUE" part. But since we
            // process one token at a time, we handle the common case:
            // if this is the last char, treat remainder as inline value.
            // This is actually a SHORT_FLAG_WITH_VALUE for c, and the
            // booleans before it form the stack.
            // Most CLI tools handle this as: emit all booleans, then the
            // non-boolean with its value.
            // We'll return STACKED_FLAGS including the non-boolean char
            // conceptually, but the parsing logic will look up each char.
            flagChars.push(c);
            // For non-boolean with trailing value, we still return STACKED_FLAGS
            // since the parser will look up each char and handle values.
            return { type: "STACKED_FLAGS", chars: flagChars };
          }
          return { type: "SHORT_FLAG_WITH_VALUE", char: c, value: remainder };
        } else {
          // Non-boolean at the very end: value is next token
          flagChars.push(c);
          return { type: "STACKED_FLAGS", chars: flagChars };
        }
      }

      flagChars.push(c);
    }

    return { type: "STACKED_FLAGS", chars: flagChars };
  }

  /**
   * Try to classify a run of chars as a stack, starting after firstChar.
   * Returns STACKED_FLAGS([firstChar, ...rest]) if successful, null if not.
   */
  private _tryStack(
    firstChar: string,
    rest: string,
  ): StackedFlagsToken | null {
    const flagChars: string[] = [firstChar];

    for (let i = 0; i < rest.length; i++) {
      const c = rest[i];
      const flag = this._shortMap.get(c);
      if (flag === undefined) {
        return null; // unknown char → not a valid stack
      }
      flagChars.push(c);
      if (flag.type !== "boolean" && flag.type !== "count") {
        // Non-boolean/non-count at the end of a stack: valid (value is next token)
        // Return what we have so far including this char.
        break;
      }
    }

    return { type: "STACKED_FLAGS", chars: flagChars };
  }

  /**
   * Look up a FlagDef by its long flag name.
   * Returns undefined if no such flag is active.
   */
  lookupByLong(name: string): FlagDef | undefined {
    return this._longMap.get(name);
  }

  /**
   * Look up a FlagDef by its short flag character.
   */
  lookupByShort(char: string): FlagDef | undefined {
    return this._shortMap.get(char);
  }

  /**
   * Look up a FlagDef by its single_dash_long name.
   */
  lookupBySingleDashLong(name: string): FlagDef | undefined {
    return this._sdlMap.get(name);
  }
}
