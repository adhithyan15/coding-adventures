package com.codingadventures.tomllexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class TomlLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("toml-lexer", new TomlLexer().ping());
    }
}
