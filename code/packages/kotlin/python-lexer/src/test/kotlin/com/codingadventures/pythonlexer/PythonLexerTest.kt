package com.codingadventures.pythonlexer

import kotlin.test.Test
import kotlin.test.assertEquals

class PythonLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("python-lexer", PythonLexer().ping())
    }
}
