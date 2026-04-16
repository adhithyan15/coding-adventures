package com.codingadventures.typescriptparser

import kotlin.test.Test
import kotlin.test.assertEquals

class TypescriptParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("typescript-parser", TypescriptParser().ping())
    }
}
