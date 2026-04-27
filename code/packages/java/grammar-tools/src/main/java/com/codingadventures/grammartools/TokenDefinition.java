// ============================================================================
// TokenDefinition.java — A single token rule from a .tokens file
// ============================================================================
//
// Each line in a .tokens file defines one token: a name paired with a pattern.
// Patterns can be regular expressions (delimited by /slashes/) or literal
// strings (delimited by "quotes"). An optional -> ALIAS suffix lets multiple
// patterns emit the same token type.
//
// Examples from a .tokens file:
//
//   NUMBER  = /[0-9]+/              — regex pattern, no alias
//   PLUS    = "+"                   — literal pattern, no alias
//   STRING_DQ = /"[^"]*"/  -> STRING  — regex pattern with alias
//
// The lexer tries patterns in definition order (first match wins), so a
// TokenDefinition's position in the list determines its priority.
//
// Layer: TE (text/language layer — foundational infrastructure)
// ============================================================================

package com.codingadventures.grammartools;

import java.util.Objects;

/**
 * An immutable record of a single token rule parsed from a .tokens file.
 *
 * <p>Fields:
 * <ul>
 *   <li>{@code name} — the token name, e.g. "NUMBER" or "PLUS"</li>
 *   <li>{@code pattern} — the pattern string (without delimiters)</li>
 *   <li>{@code isRegex} — true if /regex/, false if "literal"</li>
 *   <li>{@code lineNumber} — 1-based line where this definition appeared</li>
 *   <li>{@code alias} — optional type alias (null if none)</li>
 * </ul>
 */
public final class TokenDefinition {

    private final String name;
    private final String pattern;
    private final boolean isRegex;
    private final int lineNumber;
    private final String alias;

    public TokenDefinition(String name, String pattern, boolean isRegex, int lineNumber, String alias) {
        this.name = Objects.requireNonNull(name);
        this.pattern = Objects.requireNonNull(pattern);
        this.isRegex = isRegex;
        this.lineNumber = lineNumber;
        this.alias = alias; // null means no alias
    }

    public TokenDefinition(String name, String pattern, boolean isRegex, int lineNumber) {
        this(name, pattern, isRegex, lineNumber, null);
    }

    public String getName() { return name; }
    public String getPattern() { return pattern; }
    public boolean isRegex() { return isRegex; }
    public int getLineNumber() { return lineNumber; }
    public String getAlias() { return alias; }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof TokenDefinition that)) return false;
        return isRegex == that.isRegex
                && lineNumber == that.lineNumber
                && name.equals(that.name)
                && pattern.equals(that.pattern)
                && Objects.equals(alias, that.alias);
    }

    @Override
    public int hashCode() {
        return Objects.hash(name, pattern, isRegex, lineNumber, alias);
    }

    @Override
    public String toString() {
        String aliasStr = alias != null ? " -> " + alias : "";
        String patternStr = isRegex ? "/" + pattern + "/" : "\"" + pattern + "\"";
        return "TokenDefinition(" + name + " = " + patternStr + aliasStr + ", line " + lineNumber + ")";
    }
}
