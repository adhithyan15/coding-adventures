package com.codingadventures.algolparser;

import com.codingadventures.parser.ASTNode;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class AlgolParserTest {
    @Test
    void parsesMinimalProgram() {
        ASTNode ast = AlgolParser.parseAlgol("begin integer x; x := 42 end");

        assertEquals("program", ast.getRuleName());
        assertTrue(ast.descendantCount() > 0);
    }

    @Test
    void defaultVersionMatchesExplicitAlgol60() {
        ASTNode defaultAst = AlgolParser.parseAlgol("begin integer x; x := 42 end");
        ASTNode explicitAst = AlgolParser.parseAlgol("begin integer x; x := 42 end", "algol60");

        assertEquals(defaultAst.getRuleName(), explicitAst.getRuleName());
    }

    @Test
    void rejectsUnknownVersion() {
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> AlgolParser.parseAlgol("begin integer x; x := 42 end", "algol68")
        );

        assertTrue(error.getMessage().contains("Unknown ALGOL version"));
    }
}
