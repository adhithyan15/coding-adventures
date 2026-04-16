package com.codingadventures.excellexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class ExcelLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("excel-lexer", new ExcelLexer().ping());
    }
}
