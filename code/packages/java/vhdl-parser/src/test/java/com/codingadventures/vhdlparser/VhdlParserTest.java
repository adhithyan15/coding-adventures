package com.codingadventures.vhdlparser;

import com.codingadventures.parser.ASTNode;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class VhdlParserTest {
    @Test
    void parsesSimpleEntity() {
        ASTNode ast = VhdlParser.parseVhdl("entity top is end entity top;");

        assertEquals("design_file", ast.getRuleName());
        assertTrue(ast.descendantCount() > 0);
    }

    @Test
    void defaultVersionMatchesExplicit2008() {
        ASTNode defaultAst = VhdlParser.parseVhdl("entity top is end entity top;");
        ASTNode explicitAst = VhdlParser.parseVhdl("entity top is end entity top;", "2008");

        assertEquals(defaultAst.getRuleName(), explicitAst.getRuleName());
    }

    @Test
    void rejectsUnknownVersion() {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> VhdlParser.parseVhdl("entity top is end entity top;", "2099")
        );

        assertTrue(error.getMessage().contains("Unknown VHDL version"));
    }
}
