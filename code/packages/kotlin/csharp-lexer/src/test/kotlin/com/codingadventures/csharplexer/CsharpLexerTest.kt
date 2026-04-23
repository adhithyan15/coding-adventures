package com.codingadventures.csharplexer

import kotlin.test.Test
import kotlin.test.assertEquals

class CsharpLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("csharp-lexer", CsharpLexer().ping())
    }
}
