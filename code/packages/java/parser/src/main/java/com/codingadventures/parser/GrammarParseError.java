// ============================================================================
// GrammarParseError.java — Exception for grammar-driven parse failures
// ============================================================================

package com.codingadventures.parser;

import com.codingadventures.lexer.Token;

/**
 * Thrown when grammar-driven parsing fails.
 */
public class GrammarParseError extends Exception {

    private final Token token;

    public GrammarParseError(String message, Token token) {
        super("Parse error at " + token.getLine() + ":" + token.getColumn() + ": " + message);
        this.token = token;
    }

    public Token getToken() { return token; }
}
