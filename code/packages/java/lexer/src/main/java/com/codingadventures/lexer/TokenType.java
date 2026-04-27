// ============================================================================
// TokenType.java — Standard token type enumeration
// ============================================================================
//
// These are the built-in token types used by the hand-written lexer.
// Grammar-driven lexers use the typeName field on Token instead, since
// token types are defined by the .tokens file, not a fixed enum.
// ============================================================================

package com.codingadventures.lexer;

/**
 * Standard token types for hand-written lexers.
 *
 * <p>Grammar-driven lexers use string type names from the .tokens file instead
 * of this enum, but this enum is still needed for the generic Token class.
 */
public enum TokenType {
    NAME,
    NUMBER,
    STRING,
    KEYWORD,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    EQUALS,
    EQUALS_EQUALS,
    LPAREN,
    RPAREN,
    COMMA,
    COLON,
    SEMICOLON,
    LBRACE,
    RBRACE,
    LBRACKET,
    RBRACKET,
    DOT,
    BANG,
    NEWLINE,
    EOF,
    /** Grammar-driven token — the actual type is in Token.typeName. */
    GRAMMAR
}
