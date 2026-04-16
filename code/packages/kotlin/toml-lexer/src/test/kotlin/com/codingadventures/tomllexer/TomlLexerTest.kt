package com.codingadventures.tomllexer

import kotlin.test.Test
import kotlin.test.assertEquals

class TomlLexerTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("toml-lexer", TomlLexer().ping())
    }
}
