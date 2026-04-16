package com.codingadventures.jsonlexer

import kotlin.test.Test
import kotlin.test.assertEquals

class JsonLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("json-lexer", JsonLexer().ping())
    }
}
