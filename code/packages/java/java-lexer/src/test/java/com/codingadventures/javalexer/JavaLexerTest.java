package com.codingadventures.javalexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class JavaLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("java-lexer", new JavaLexer().ping());
    }
}
