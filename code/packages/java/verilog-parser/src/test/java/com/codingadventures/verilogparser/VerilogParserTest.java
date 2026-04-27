package com.codingadventures.verilogparser;

import com.codingadventures.parser.ASTNode;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class VerilogParserTest {
    @Test
    void parsesSimpleModule() {
        ASTNode ast = VerilogParser.parseVerilog("module top; endmodule");

        assertEquals("source_text", ast.getRuleName());
        assertTrue(ast.descendantCount() > 0);
    }

    @Test
    void defaultVersionMatchesExplicit2005() {
        ASTNode defaultAst = VerilogParser.parseVerilog("module top; endmodule");
        ASTNode explicitAst = VerilogParser.parseVerilog("module top; endmodule", "2005");

        assertEquals(defaultAst.getRuleName(), explicitAst.getRuleName());
    }

    @Test
    void rejectsUnknownVersion() {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> VerilogParser.parseVerilog("module top; endmodule", "2099")
        );

        assertTrue(error.getMessage().contains("Unknown Verilog version"));
    }
}
