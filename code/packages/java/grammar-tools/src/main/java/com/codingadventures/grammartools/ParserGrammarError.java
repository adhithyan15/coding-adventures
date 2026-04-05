// ============================================================================
// ParserGrammarError.java — Exception for .grammar file parse errors
// ============================================================================

package com.codingadventures.grammartools;

/**
 * Thrown when a .grammar file cannot be parsed.
 */
public class ParserGrammarError extends Exception {

    private final int lineNumber;

    public ParserGrammarError(String message, int lineNumber) {
        super("Line " + lineNumber + ": " + message);
        this.lineNumber = lineNumber;
    }

    public int getLineNumber() { return lineNumber; }
}
