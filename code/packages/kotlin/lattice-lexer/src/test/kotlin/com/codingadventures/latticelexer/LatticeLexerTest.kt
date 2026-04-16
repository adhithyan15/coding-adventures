package com.codingadventures.latticelexer

import kotlin.test.Test
import kotlin.test.assertEquals

class LatticeLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("lattice-lexer", LatticeLexer().ping())
    }
}
