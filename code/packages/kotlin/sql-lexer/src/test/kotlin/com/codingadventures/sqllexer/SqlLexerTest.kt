package com.codingadventures.sqllexer

import kotlin.test.Test
import kotlin.test.assertEquals

class SqlLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("sql-lexer", SqlLexer().ping())
    }
}
