package com.codingadventures.verilogparser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class VerilogParserTest {
    @Test
    fun parsesSimpleModule() {
        val ast = VerilogParser.parseVerilog("module top; endmodule")

        assertEquals("source_text", ast.ruleName)
        assertTrue(ast.descendantCount() > 0)
    }

    @Test
    fun defaultVersionMatchesExplicit2005() {
        val defaultAst = VerilogParser.parseVerilog("module top; endmodule")
        val explicitAst = VerilogParser.parseVerilog("module top; endmodule", "2005")

        assertEquals(defaultAst.ruleName, explicitAst.ruleName)
    }

    @Test
    fun rejectsUnknownVersion() {
        val error = assertFailsWith<IllegalArgumentException> {
            VerilogParser.parseVerilog("module top; endmodule", "2099")
        }

        assertTrue(error.message!!.contains("Unknown Verilog version"))
    }
}
