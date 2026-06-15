/**
 * Regular expressions — the SIR `regex` builtin (Ruby dialect on JS `RegExp`).
 *
 * The Ruby→SIR frontend lowers a regex literal `/pat/flags` to
 * `BuiltinCall("regex", [StrLit pattern, StrLit flags])`. This module is the
 * *TypeScript/JavaScript* landing point for that builtin: it builds a native
 * `RegExp`, translating Ruby's flag and anchoring conventions so the compiled
 * object behaves the way the Ruby source intended.
 *
 * **Why a translation layer — Ruby's dialect differs from JS `RegExp`.** Both
 * descend from Perl-compatible regular expressions, so most *syntax* (`\d`,
 * `[a-z]`, `(?:...)`, `a|b`, `+ * ?`) is shared. They diverge in how inline
 * flags are spelled, in what `^`/`$` mean by default, and in extended mode:
 *
 * | Ruby flag | JS flag | Meaning |
 * |-----------|---------|---------|
 * | `i`       | `i`     | Case-insensitive matching. |
 * | `m`       | `s`     | Ruby `/m` ("multiline") makes `.` match a newline — that is JS's *dotAll* (`s`), **not** JS's `m`. The shared letter is a foot-gun. |
 * | `x`       | (none)  | "Extended" mode: ignore unescaped whitespace and `#` comments. JS has no equivalent flag, so we approximate by stripping them from the pattern text (best-effort subset — see {@link stripExtended}). |
 * | (other)   | —       | Unknown flag characters are silently dropped. |
 *
 * **The `^`/`$` nuance.** In Ruby, `^` and `$` *always* anchor to the
 * start/end of a **line** — the whole-string anchors are the separate escapes
 * `\A` and `\z`/`\Z`. In JS the default is the opposite (`^`/`$` anchor the
 * whole string unless the `m` flag is set). To behave faithfully we therefore
 * **always include the JS `m` flag**. (`\A`/`\z` are not valid JS escapes, but
 * the unconditional `m` covers the common Ruby line-anchor case.)
 *
 * **Match semantics.** Ruby's `=~` and `String#match?` perform an *unanchored
 * search* — a hit anywhere counts, not a full-string match. {@link isMatch} and
 * {@link matchData} therefore use `.test` / `.exec` on a non-global RegExp.
 */

/** The SIR universal value type at this package's boundary. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Val = any;

/**
 * Approximate Ruby's `/x` (extended / "verbose") mode by stripping the parts of
 * the pattern that extended mode ignores: unescaped whitespace and `#`-to-end-of-line
 * comments. JavaScript's `RegExp` has no extended flag, so this is a *best-effort
 * subset* — it handles the common cases (laying a pattern out across lines with
 * comments) but does not implement every nuance (e.g. whitespace inside a
 * character class, which Ruby keeps literal, is also stripped here).
 *
 * A backslash escapes the following character, so `\#` keeps a literal `#` and
 * `\ ` keeps a literal space; both are preserved verbatim.
 */
export function stripExtended(pattern: string): string {
  let out = "";
  let i = 0;
  while (i < pattern.length) {
    const ch = pattern[i];
    if (ch === "\\") {
      // An escape: keep the backslash and whatever it escapes, untouched.
      out += pattern.slice(i, i + 2);
      i += 2;
      continue;
    }
    if (ch === "#") {
      // A comment runs to the end of the line; drop it.
      while (i < pattern.length && pattern[i] !== "\n") {
        i += 1;
      }
      continue;
    }
    if (/\s/.test(ch)) {
      // Unescaped whitespace is ignored in extended mode.
      i += 1;
      continue;
    }
    out += ch;
    i += 1;
  }
  return out;
}

/**
 * Compile a Ruby-dialect regex `pattern` under Ruby-dialect `flags`.
 *
 * `flags` is the inline-flag *string* exactly as it trails a Ruby literal —
 * e.g. the `"imx"` of `/.../imx`. Each recognised character maps to a JS flag
 * per the table in the module doc comment; unknown characters are ignored. The
 * JS `m` flag is *always* included so Ruby's line-anchored `^`/`$` behave
 * correctly. For `x` there is no JS flag, so the pattern text is preprocessed
 * with {@link stripExtended} instead.
 */
export function compile(pattern: string, flags = ""): RegExp {
  // Ruby's ^/$ are always line anchors, so 'm' is unconditional. A Set keeps
  // the JS flag string de-duplicated and order-independent.
  const jsFlags = new Set<string>(["m"]);
  let source = pattern;
  for (const ch of flags) {
    if (ch === "i") {
      jsFlags.add("i");
    } else if (ch === "m") {
      jsFlags.add("s"); // Ruby /m == dotAll == JS 's'.
    } else if (ch === "x") {
      // No JS equivalent: approximate by stripping ignorable whitespace/comments.
      source = stripExtended(source);
    }
    // Any other character contributes nothing.
  }
  return new RegExp(source, [...jsFlags].join(""));
}

/**
 * Build a fresh, non-global `RegExp` from `pattern`.
 *
 * Accepting a raw string compiles it with the default Ruby flag set (just the
 * unconditional `m`). Accepting an existing `RegExp` clones it *without* the
 * `g`/`y` flags — a global/sticky regex carries a mutable `lastIndex`, so
 * reusing the caller's instance across `.test`/`.exec` calls would make results
 * depend on prior calls. The clone makes each match independent.
 */
function fresh(pattern: RegExp | string): RegExp {
  if (pattern instanceof RegExp) {
    const flags = pattern.flags.replace(/[gy]/g, "");
    return new RegExp(pattern.source, flags);
  }
  return compile(pattern);
}

/**
 * True iff `pattern` matches anywhere in `s` (Ruby `=~` / `match?`).
 *
 * This is an *unanchored search*, mirroring Ruby semantics. `pattern` may be a
 * `RegExp` or a raw pattern string; a `RegExp` is tested via a fresh non-global
 * clone so a stale `lastIndex` cannot leak across calls.
 */
export function isMatch(pattern: RegExp | string, s: string): boolean {
  return fresh(pattern).test(s);
}

/**
 * Return the matched substring (match[0]), or `null` if there is no match.
 *
 * A minimal model of Ruby's `String#match`, which returns a truthy `MatchData`
 * (whose `[0]` is the matched text) or `nil`. `pattern` may be a `RegExp` or a
 * raw pattern string.
 */
export function matchData(pattern: RegExp | string, s: string): string | null {
  const m = fresh(pattern).exec(s);
  return m === null ? null : m[0];
}
