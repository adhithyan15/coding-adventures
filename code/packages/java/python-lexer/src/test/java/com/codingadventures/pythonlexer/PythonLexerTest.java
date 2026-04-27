package com.codingadventures.pythonlexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class PythonLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("python-lexer", new PythonLexer().ping());
    }
}
