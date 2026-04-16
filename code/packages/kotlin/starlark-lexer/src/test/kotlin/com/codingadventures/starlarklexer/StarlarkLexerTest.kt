package com.codingadventures.starlarklexer

import kotlin.test.Test
import kotlin.test.assertEquals

class StarlarkLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("starlark-lexer", StarlarkLexer().ping())
    }
}
