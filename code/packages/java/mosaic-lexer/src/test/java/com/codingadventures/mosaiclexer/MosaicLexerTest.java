package com.codingadventures.mosaiclexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class MosaicLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("mosaic-lexer", new MosaicLexer().ping());
    }
}
