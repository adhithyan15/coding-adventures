package com.codingadventures.javalexer

import kotlin.test.Test
import kotlin.test.assertEquals

class JavaLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("java-lexer", JavaLexer().ping())
    }
}
