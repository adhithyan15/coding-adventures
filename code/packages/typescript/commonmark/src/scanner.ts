/**
 * String Scanner
 *
 * A cursor-based scanner over a string. Used by both the block parser
 * (to scan individual lines) and the inline parser (to scan inline
 * content character by character).
 *
 * === Design ===
 *
 * The scanner maintains a position `pos` into the string. All read
 * operations advance `pos`. The scanner never backtracks on its own —
 * callers must save and restore `pos` explicitly when lookahead fails.
 *
 * This is the same pattern used by hand-rolled recursive descent parsers
 * everywhere: try to match, if it fails, restore the saved position.
 *
 *   const saved = scanner.pos;
 *   if (!scanner.match("```")) {
 *     scanner.pos = saved; // backtrack
 *   }
 *
 * === Character classification ===
 *
 * CommonMark cares about several Unicode character categories:
 *   - ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
 *   - Unicode punctuation (for emphasis rules)
 *   - ASCII whitespace: space, tab, CR, LF, FF
 *   - Unicode whitespace
 *
 * These classification functions are co-located with the scanner because
 * they are used throughout the inline parser.
 *
 * @module scanner
 */

// ─── Scanner ──────────────────────────────────────────────────────────────────

export class Scanner {
  readonly source: string;
  pos: number;

  constructor(source: string, start = 0) {
    this.source = source;
    this.pos = start;
  }

  /** True if the scanner has consumed all input. */
  get done(): boolean {
    return this.pos >= this.source.length;
  }

  /** Number of characters remaining. */
  get remaining(): number {
    return this.source.length - this.pos;
  }

  /** Peek at the character at `pos` without advancing. */
  peek(offset = 0): string {
    return this.source[this.pos + offset] ?? "";
  }

  /** Peek at `n` characters starting at `pos` without advancing. */
  peekSlice(n: number): string {
    return this.source.slice(this.pos, this.pos + n);
  }

  /** Advance `pos` by one and return the consumed character. */
  advance(): string {
    return this.source[this.pos++] ?? "";
  }

  /** Advance `pos` by `n` characters. */
  skip(n: number): void {
    this.pos = Math.min(this.pos + n, this.source.length);
  }

  /**
   * If the next characters exactly match `str`, advance past them
   * and return true. Otherwise leave `pos` unchanged and return false.
   */
  match(str: string): boolean {
    if (this.source.startsWith(str, this.pos)) {
      this.pos += str.length;
      return true;
    }
    return false;
  }

  /**
   * If the next characters match the regex (anchored at current pos),
   * advance past the match and return the matched string.
   * Otherwise return null and leave `pos` unchanged.
   *
   * The regex should NOT have the global flag — we use sticky matching.
   */
  matchRegex(re: RegExp): string | null {
    const stickyRe = new RegExp(re.source, re.flags.includes("y") ? re.flags : re.flags + "y");
    stickyRe.lastIndex = this.pos;
    const m = stickyRe.exec(this.source);
    if (m === null) return null;
    this.pos += m[0].length;
    return m[0];
  }

  /**
   * Consume characters while the predicate returns true.
   * Returns the consumed string.
   */
  consumeWhile(pred: (ch: string) => boolean): string {
    const start = this.pos;
    while (!this.done && pred(this.source[this.pos]!)) {
      this.pos++;
    }
    return this.source.slice(start, this.pos);
  }

  /** Consume the rest of the line (up to but not including the newline). */
  consumeLine(): string {
    const start = this.pos;
    while (!this.done && this.source[this.pos] !== "\n") {
      this.pos++;
    }
    return this.source.slice(start, this.pos);
  }

  /** Return the rest of the input from current pos without advancing. */
  rest(): string {
    return this.source.slice(this.pos);
  }

  /** Return a slice of source from `start` to current pos. */
  sliceFrom(start: number): string {
    return this.source.slice(start, this.pos);
  }

  /** Skip ASCII spaces and tabs. Returns number of spaces skipped. */
  skipSpaces(): number {
    const start = this.pos;
    while (!this.done && (this.source[this.pos] === " " || this.source[this.pos] === "\t")) {
      this.pos++;
    }
    return this.pos - start;
  }

  /** Count leading spaces/tabs without advancing. Returns virtual column. */
  countIndent(): number {
    let indent = 0;
    let i = this.pos;
    while (i < this.source.length) {
      const ch = this.source[i]!;
      if (ch === " ") { indent++; i++; }
      else if (ch === "\t") { indent += 4 - (indent % 4); i++; } // tab = expand to next tab stop
      else break;
    }
    return indent;
  }

  /** Advance past exactly `n` spaces of indentation (expanding tabs). */
  skipIndent(n: number): void {
    let remaining = n;
    while (remaining > 0 && !this.done) {
      const ch = this.source[this.pos]!;
      if (ch === " ") { this.pos++; remaining--; }
      else if (ch === "\t") {
        // A tab expands to the next 4-space tab stop
        const tabWidth = 4 - ((this.pos) % 4);
        if (tabWidth <= remaining) {
          this.pos++;
          remaining -= tabWidth;
        } else {
          break; // partial tab — don't consume
        }
      } else {
        break;
      }
    }
  }
}

// ─── Character Classification ─────────────────────────────────────────────────

/**
 * ASCII punctuation characters as defined by CommonMark.
 * These are exactly: ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~
 */
const ASCII_PUNCTUATION = new Set(
  "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".split(""),
);

/**
 * True if `ch` is an ASCII punctuation character (CommonMark definition).
 * Used in the emphasis rules to determine flanking delimiter runs.
 */
export function isAsciiPunctuation(ch: string): boolean {
  return ASCII_PUNCTUATION.has(ch);
}

/**
 * True if `ch` is a Unicode punctuation character for CommonMark flanking.
 *
 * CommonMark defines this (per the cmark reference implementation) as any
 * ASCII punctuation character OR any character in Unicode categories:
 *   Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or Sm, Sc, Sk, So (symbols).
 *
 * The symbol categories (S*) are included because cmark treats them as
 * punctuation for delimiter flanking (e.g. £ U+00A3 Sc, € U+20AC Sc).
 */
export function isUnicodePunctuation(ch: string): boolean {
  if (ch === "") return false;
  // ASCII punctuation is a subset
  if (ASCII_PUNCTUATION.has(ch)) return true;
  // Unicode punctuation categories (P*) and symbol categories (S*)
  return /^\p{P}$/u.test(ch) || /^\p{S}$/u.test(ch);
}

/**
 * True if `ch` is ASCII whitespace: space (U+0020), tab (U+0009),
 * newline (U+000A), form feed (U+000C), carriage return (U+000D).
 */
export function isAsciiWhitespace(ch: string): boolean {
  return ch === " " || ch === "\t" || ch === "\n" || ch === "\r" || ch === "\f";
}

/**
 * True if `ch` is Unicode whitespace (any code point with Unicode
 * property White_Space=yes).
 */
export function isUnicodeWhitespace(ch: string): boolean {
  if (ch === "") return false;
  return /^\s$/u.test(ch) || ch === "\u00A0" || ch === "\u1680" ||
    (ch >= "\u2000" && ch <= "\u200A") || ch === "\u202F" ||
    ch === "\u205F" || ch === "\u3000";
}

/**
 * True if `ch` is an ASCII digit (0-9).
 */
export function isDigit(ch: string): boolean {
  return ch >= "0" && ch <= "9";
}

/**
 * Normalize a link label per CommonMark:
 *   - Strip leading and trailing whitespace
 *   - Collapse internal whitespace runs to a single space
 *   - Fold to lowercase
 *
 * Two labels are equivalent if their normalized forms are equal.
 */
export function normalizeLinkLabel(label: string): string {
  return label.trim().replace(/\s+/g, " ").toLowerCase();
}

/**
 * Normalize a URL: percent-encode spaces and certain characters that
 * should not appear unencoded in HTML href/src attributes.
 */
export function normalizeUrl(url: string): string {
  // Encode characters that need percent-encoding in HTML attributes
  // but are not already encoded
  return url.replace(
    /[^\w\-._~:/?#@!$&'()*+,;=%]/g,
    (ch) => encodeURIComponent(ch),
  );
}
