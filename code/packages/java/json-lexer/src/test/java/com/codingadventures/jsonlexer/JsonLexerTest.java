package com.codingadventures.jsonlexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class JsonLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("json-lexer", new JsonLexer().ping());
    }
}
