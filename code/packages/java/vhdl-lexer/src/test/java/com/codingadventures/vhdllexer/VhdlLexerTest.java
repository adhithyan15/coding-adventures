package com.codingadventures.vhdllexer;

import com.codingadventures.lexer.Token;
import com.codingadventures.lexer.TokenType;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class VhdlLexerTest {
    @Test
    void normalizesCaseInsensitiveTokens() {
        List<Token> tokens = VhdlLexer.tokenizeVhdl("ENTITY TOP IS END ENTITY TOP;");

        assertEquals(TokenType.KEYWORD, tokens.get(0).getType());
        assertEquals("entity", tokens.get(0).getValue());
        assertEquals("NAME", tokens.get(1).effectiveTypeName());
        assertEquals("top", tokens.get(1).getValue());
    }

    @Test
    void defaultVersionMatchesExplicit2008() {
        List<Token> defaultTokens = VhdlLexer.tokenizeVhdl("entity top is end entity top;");
        List<Token> explicitTokens = VhdlLexer.tokenizeVhdl("entity top is end entity top;", "2008");

        assertEquals(defaultTokens, explicitTokens);
    }

    @Test
    void rejectsUnknownVersion() {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> VhdlLexer.tokenizeVhdl("entity top is end entity top;", "2099")
        );

        assertTrue(error.getMessage().contains("Unknown VHDL version"));
    }
}
