package com.codingadventures.algolparser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class AlgolParserTest {
    @Test
    fun parsesMinimalProgram() {
        val ast = AlgolParser.parseAlgol("begin integer x; x := 42 end")

        assertEquals("program", ast.ruleName)
        assertTrue(ast.descendantCount() > 0)
    }

    @Test
    fun defaultVersionMatchesExplicitAlgol60() {
        val defaultAst = AlgolParser.parseAlgol("begin integer x; x := 42 end")
        val explicitAst = AlgolParser.parseAlgol("begin integer x; x := 42 end", "algol60")

        assertEquals(defaultAst.ruleName, explicitAst.ruleName)
    }

    @Test
    fun rejectsUnknownVersion() {
        val error = assertFailsWith<IllegalArgumentException> {
            AlgolParser.parseAlgol("begin integer x; x := 42 end", "algol68")
        }

        assertTrue(error.message!!.contains("Unknown ALGOL version"))
    }
}
