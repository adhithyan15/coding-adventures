package com.codingadventures.csharplexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class CsharpLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("csharp-lexer", new CsharpLexer().ping());
    }
}
