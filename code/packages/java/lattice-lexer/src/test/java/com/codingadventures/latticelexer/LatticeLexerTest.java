package com.codingadventures.latticelexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class LatticeLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("lattice-lexer", new LatticeLexer().ping());
    }
}
