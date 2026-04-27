package com.codingadventures.excellexer

import kotlin.test.Test
import kotlin.test.assertEquals

class ExcelLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("excel-lexer", ExcelLexer().ping())
    }
}
