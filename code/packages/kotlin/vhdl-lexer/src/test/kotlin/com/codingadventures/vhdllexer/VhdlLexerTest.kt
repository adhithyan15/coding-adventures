package com.codingadventures.vhdllexer

import com.codingadventures.lexer.TokenType
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class VhdlLexerTest {
    @Test
    fun normalizesCaseInsensitiveTokens() {
        val tokens = VhdlLexer.tokenizeVhdl("ENTITY TOP IS END ENTITY TOP;")

        assertEquals(TokenType.KEYWORD, tokens[0].type)
        assertEquals("entity", tokens[0].value)
        assertEquals("NAME", tokens[1].effectiveTypeName())
        assertEquals("top", tokens[1].value)
    }

    @Test
    fun defaultVersionMatchesExplicit2008() {
        val defaultTokens = VhdlLexer.tokenizeVhdl("entity top is end entity top;")
        val explicitTokens = VhdlLexer.tokenizeVhdl("entity top is end entity top;", "2008")

        assertEquals(defaultTokens, explicitTokens)
    }

    @Test
    fun rejectsUnknownVersion() {
        val error = assertFailsWith<IllegalArgumentException> {
            VhdlLexer.tokenizeVhdl("entity top is end entity top;", "2099")
        }

        assertTrue(error.message!!.contains("Unknown VHDL version"))
    }
}
