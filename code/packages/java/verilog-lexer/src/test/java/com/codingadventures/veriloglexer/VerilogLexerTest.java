package com.codingadventures.veriloglexer;

import com.codingadventures.lexer.Token;
import com.codingadventures.lexer.TokenType;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class VerilogLexerTest {
    @Test
    void tokenizesModuleDeclaration() {
        List<Token> tokens = VerilogLexer.tokenizeVerilog("module top; endmodule");

        assertEquals(TokenType.KEYWORD, tokens.get(0).getType());
        assertEquals("module", tokens.get(0).getValue());
        assertEquals("NAME", tokens.get(1).effectiveTypeName());
        assertEquals("top", tokens.get(1).getValue());
    }

    @Test
    void defaultVersionMatchesExplicit2005() {
        List<Token> defaultTokens = VerilogLexer.tokenizeVerilog("module top; endmodule");
        List<Token> explicitTokens = VerilogLexer.tokenizeVerilog("module top; endmodule", "2005");

        assertEquals(defaultTokens, explicitTokens);
    }

    @Test
    void rejectsUnknownVersion() {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> VerilogLexer.tokenizeVerilog("module top; endmodule", "2099")
        );

        assertTrue(error.getMessage().contains("Unknown Verilog version"));
    }
}
