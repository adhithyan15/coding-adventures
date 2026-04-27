package com.codingadventures.starlarklexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class StarlarkLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("starlark-lexer", new StarlarkLexer().ping());
    }
}
