package com.codingadventures.javascriptlexer

import kotlin.test.Test
import kotlin.test.assertEquals

class JavascriptLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("javascript-lexer", JavascriptLexer().ping())
    }
}
