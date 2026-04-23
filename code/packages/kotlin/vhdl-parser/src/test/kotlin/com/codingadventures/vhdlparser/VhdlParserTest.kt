package com.codingadventures.vhdlparser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class VhdlParserTest {
    @Test
    fun parsesSimpleEntity() {
        val ast = VhdlParser.parseVhdl("entity top is end entity top;")

        assertEquals("design_file", ast.ruleName)
        assertTrue(ast.descendantCount() > 0)
    }

    @Test
    fun defaultVersionMatchesExplicit2008() {
        val defaultAst = VhdlParser.parseVhdl("entity top is end entity top;")
        val explicitAst = VhdlParser.parseVhdl("entity top is end entity top;", "2008")

        assertEquals(defaultAst.ruleName, explicitAst.ruleName)
    }

    @Test
    fun rejectsUnknownVersion() {
        val error = assertFailsWith<IllegalArgumentException> {
            VhdlParser.parseVhdl("entity top is end entity top;", "2099")
        }

        assertTrue(error.message!!.contains("Unknown VHDL version"))
    }
}
