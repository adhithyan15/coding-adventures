package com.codingadventures.csharpparser

import kotlin.test.Test
import kotlin.test.assertEquals

class CsharpParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("csharp-parser", CsharpParser().ping())
    }
}
