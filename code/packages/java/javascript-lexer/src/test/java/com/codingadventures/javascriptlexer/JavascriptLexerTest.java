package com.codingadventures.javascriptlexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class JavascriptLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("javascript-lexer", new JavascriptLexer().ping());
    }
}
