package com.codingadventures.javascriptparser

import kotlin.test.Test
import kotlin.test.assertEquals

class JavascriptParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("javascript-parser", JavascriptParser().ping())
    }
}
