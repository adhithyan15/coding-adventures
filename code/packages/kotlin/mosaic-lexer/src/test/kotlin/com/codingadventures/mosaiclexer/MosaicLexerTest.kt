package com.codingadventures.mosaiclexer

import kotlin.test.Test
import kotlin.test.assertEquals

class MosaicLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("mosaic-lexer", MosaicLexer().ping())
    }
}
