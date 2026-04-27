package com.codingadventures.algollexer

import com.codingadventures.lexer.TokenType
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class AlgolLexerTest {
    @Test
    fun tokenizesAssignmentStatement() {
        val tokens = AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end")

        assertEquals(TokenType.KEYWORD, tokens[0].type)
        assertEquals("begin", tokens[0].value)
        assertEquals(TokenType.KEYWORD, tokens[1].type)
        assertEquals("integer", tokens[1].value)
        assertEquals("NAME", tokens[2].effectiveTypeName())
        assertEquals("x", tokens[2].value)
        assertEquals("NAME", tokens[4].effectiveTypeName())
        assertEquals("x", tokens[4].value)
        assertEquals("ASSIGN", tokens[5].effectiveTypeName())
        assertEquals(":=", tokens[5].value)
        assertEquals("INTEGER_LIT", tokens[6].effectiveTypeName())
        assertEquals("42", tokens[6].value)
    }

    @Test
    fun defaultVersionMatchesExplicitAlgol60() {
        val defaultTokens = AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end")
        val explicitTokens = AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end", "algol60")

        assertEquals(defaultTokens, explicitTokens)
    }

    @Test
    fun rejectsUnknownVersion() {
        val error = assertFailsWith<IllegalArgumentException> {
            AlgolLexer.tokenizeAlgol("begin integer x; x := 42 end", "algol68")
        }

        assertTrue(error.message!!.contains("Unknown ALGOL version"))
    }
}
