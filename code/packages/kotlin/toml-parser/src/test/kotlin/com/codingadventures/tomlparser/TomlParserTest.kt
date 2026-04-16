package com.codingadventures.tomlparser

import kotlin.test.Test
import kotlin.test.assertEquals

class TomlParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("toml-parser", TomlParser().ping())
    }
}
