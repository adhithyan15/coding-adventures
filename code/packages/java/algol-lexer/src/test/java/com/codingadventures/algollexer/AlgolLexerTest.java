package com.codingadventures.algollexer;

import com.codingadventures.lexer.Token;
import com.codingadventures.lexer.TokenType;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class AlgolLexerTest {
    @Test
    void tokenizesAssignmentStatement() {
        List<Token> tokens = AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end");

        assertEquals(TokenType.KEYWORD, tokens.get(0).getType());
        assertEquals("begin", tokens.get(0).getValue());
        assertEquals(TokenType.KEYWORD, tokens.get(1).getType());
        assertEquals("integer", tokens.get(1).getValue());
        assertEquals("NAME", tokens.get(2).effectiveTypeName());
        assertEquals("x", tokens.get(2).getValue());
        assertEquals("NAME", tokens.get(4).effectiveTypeName());
        assertEquals("x", tokens.get(4).getValue());
        assertEquals("ASSIGN", tokens.get(5).effectiveTypeName());
        assertEquals(":=", tokens.get(5).getValue());
        assertEquals("INTEGER_LIT", tokens.get(6).effectiveTypeName());
        assertEquals("42", tokens.get(6).getValue());
    }

    @Test
    void defaultVersionMatchesExplicitAlgol60() {
        List<Token> defaultTokens = AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end");
        List<Token> explicitTokens = AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end", "algol60");

        assertEquals(defaultTokens, explicitTokens);
    }

    @Test
    void rejectsUnknownVersion() {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end", "algol68")
        );

        assertTrue(error.getMessage().contains("Unknown ALGOL version"));
    }
}
