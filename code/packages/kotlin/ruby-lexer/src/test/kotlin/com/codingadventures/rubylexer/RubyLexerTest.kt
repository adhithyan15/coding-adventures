package com.codingadventures.rubylexer

import kotlin.test.Test
import kotlin.test.assertEquals

class RubyLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("ruby-lexer", RubyLexer().ping())
    }
}
