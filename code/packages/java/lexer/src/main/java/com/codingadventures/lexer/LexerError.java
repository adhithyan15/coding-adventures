// ============================================================================
// LexerError.java — Exception for tokenization errors
// ============================================================================

package com.codingadventures.lexer;

/**
 * Thrown when the lexer encounters text it cannot tokenize.
 */
public class LexerError extends Exception {

    private final int line;
    private final int column;

    public LexerError(String message, int line, int column) {
        super("Lexer error at " + line + ":" + column + ": " + message);
        this.line = line;
        this.column = column;
    }

    public int getLine() { return line; }
    public int getColumn() { return column; }
}
