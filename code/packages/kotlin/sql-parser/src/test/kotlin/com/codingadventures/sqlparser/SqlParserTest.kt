package com.codingadventures.sqlparser

import kotlin.test.Test
import kotlin.test.assertEquals

class SqlParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("sql-parser", SqlParser().ping())
    }
}
