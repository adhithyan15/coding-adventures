package com.codingadventures.typescriptlexer

import kotlin.test.Test
import kotlin.test.assertEquals

class TypescriptLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("typescript-lexer", TypescriptLexer().ping())
    }
}
