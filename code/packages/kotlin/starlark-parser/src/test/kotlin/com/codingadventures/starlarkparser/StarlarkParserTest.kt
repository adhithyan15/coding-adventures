package com.codingadventures.starlarkparser

import kotlin.test.Test
import kotlin.test.assertEquals

class StarlarkParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("starlark-parser", StarlarkParser().ping())
    }
}
