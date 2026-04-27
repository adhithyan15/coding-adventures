package com.codingadventures.veriloglexer

import com.codingadventures.lexer.TokenType
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class VerilogLexerTest {
    @Test
    fun tokenizesModuleDeclaration() {
        val tokens = VerilogLexer.tokenizeVerilog("module top; endmodule")

        assertEquals(TokenType.KEYWORD, tokens[0].type)
        assertEquals("module", tokens[0].value)
        assertEquals("NAME", tokens[1].effectiveTypeName())
        assertEquals("top", tokens[1].value)
    }

    @Test
    fun defaultVersionMatchesExplicit2005() {
        val defaultTokens = VerilogLexer.tokenizeVerilog("module top; endmodule")
        val explicitTokens = VerilogLexer.tokenizeVerilog("module top; endmodule", "2005")

        assertEquals(defaultTokens, explicitTokens)
    }

    @Test
    fun rejectsUnknownVersion() {
        val error = assertFailsWith<IllegalArgumentException> {
            VerilogLexer.tokenizeVerilog("module top; endmodule", "2099")
        }

        assertTrue(error.message!!.contains("Unknown Verilog version"))
    }
}
