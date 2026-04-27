package com.codingadventures.rubylexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class RubyLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("ruby-lexer", new RubyLexer().ping());
    }
}
