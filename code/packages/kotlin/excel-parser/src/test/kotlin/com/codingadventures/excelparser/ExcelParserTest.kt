package com.codingadventures.excelparser

import kotlin.test.Test
import kotlin.test.assertEquals

class ExcelParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("excel-parser", ExcelParser().ping())
    }
}
