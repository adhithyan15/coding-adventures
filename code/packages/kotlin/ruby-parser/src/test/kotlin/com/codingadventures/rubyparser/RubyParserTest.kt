package com.codingadventures.rubyparser

import kotlin.test.Test
import kotlin.test.assertEquals

class RubyParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("ruby-parser", RubyParser().ping())
    }
}
