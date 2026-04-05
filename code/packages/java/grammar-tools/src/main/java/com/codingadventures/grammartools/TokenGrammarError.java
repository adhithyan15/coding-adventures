// ============================================================================
// TokenGrammarError.java — Exception for .tokens file parse errors
// ============================================================================

package com.codingadventures.grammartools;

/**
 * Thrown when a .tokens file cannot be parsed.
 */
public class TokenGrammarError extends Exception {

    private final String msg;
    private final int lineNumber;

    public TokenGrammarError(String message, int lineNumber) {
        super("Line " + lineNumber + ": " + message);
        this.msg = message;
        this.lineNumber = lineNumber;
    }

    /** Human-readable description of the problem. */
    public String getMsg() { return msg; }

    /** 1-based line number where the error occurred. */
    public int getLineNumber() { return lineNumber; }
}
