package com.codingadventures.typescriptlexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class TypescriptLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("typescript-lexer", new TypescriptLexer().ping());
    }
}
