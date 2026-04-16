package com.codingadventures.pythonparser

import kotlin.test.Test
import kotlin.test.assertEquals

class PythonParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("python-parser", PythonParser().ping())
    }
}
