// ============================================================================
// Token.java — A single token produced by a lexer
// ============================================================================
//
// A token is the atomic unit of source code. Just as written English can be
// decomposed into words, punctuation, and spaces, source code can be
// decomposed into tokens: identifiers, numbers, operators, keywords, etc.
//
// Every token carries four pieces of information:
//
//   1. Type — what kind of token (NUMBER, PLUS, NAME, etc.)
//   2. Value — the actual text from the source code
//   3. Line — which line the token appeared on (1-based)
//   4. Column — which column the token starts at (1-based)
//
// The type/value distinction is important: the token "42" has type NUMBER
// and value "42". The token "+" has type PLUS and value "+". The parser
// cares about types (it matches on NUMBER, not "42"); the code generator
// cares about values (it needs the actual number 42).
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.lexer;

import java.util.Objects;

/**
 * An immutable token from source code.
 *
 * <p>Grammar-driven tokens use {@code typeName} to carry the grammar-defined
 * token name (e.g. "INT", "FLOAT_LITERAL"). Hand-written lexer tokens use
 * the {@code type} enum and leave {@code typeName} null.
 */
public final class Token {

    /** Bitmask flag: a line break appeared before this token. */
    public static final int FLAG_PRECEDED_BY_NEWLINE = 1;

    /** Bitmask flag: this is a context-sensitive keyword. */
    public static final int FLAG_CONTEXT_KEYWORD = 2;

    private final TokenType type;
    private final String value;
    private final int line;
    private final int column;
    private final String typeName;
    private final int flags;

    public Token(TokenType type, String value, int line, int column, String typeName, int flags) {
        this.type = type;
        this.value = Objects.requireNonNull(value);
        this.line = line;
        this.column = column;
        this.typeName = typeName;
        this.flags = flags;
    }

    public Token(TokenType type, String value, int line, int column) {
        this(type, value, line, column, null, 0);
    }

    public Token(TokenType type, String value, int line, int column, String typeName) {
        this(type, value, line, column, typeName, 0);
    }

    public TokenType getType() { return type; }
    public String getValue() { return value; }
    public int getLine() { return line; }
    public int getColumn() { return column; }
    public String getTypeName() { return typeName; }
    public int getFlags() { return flags; }

    /** Returns the effective type name: typeName if set, else the TokenType name. */
    public String effectiveTypeName() {
        return typeName != null ? typeName : type.name();
    }

    /** Check if a specific flag is set. */
    public boolean hasFlag(int flag) {
        return (flags & flag) != 0;
    }

    @Override
    public String toString() {
        return "Token(" + effectiveTypeName() + ", \"" + value + "\", " + line + ":" + column + ")";
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof Token that)) return false;
        return type == that.type && line == that.line && column == that.column
                && flags == that.flags && value.equals(that.value) && Objects.equals(typeName, that.typeName);
    }

    @Override
    public int hashCode() {
        return Objects.hash(type, value, line, column, typeName, flags);
    }
}
