package com.codingadventures.parser;

import com.codingadventures.grammartools.*;
import com.codingadventures.lexer.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Nested;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class ParserTest {

    // =========================================================================
    // ASTNode Tests
    // =========================================================================

    @Nested
    class ASTNodeTests {

        @Test void emptyNode() {
            ASTNode node = new ASTNode("test");
            assertEquals("test", node.getRuleName());
            assertTrue(node.getChildren().isEmpty());
            assertFalse(node.isLeaf());
            assertNull(node.getToken());
        }

        @Test void leafNode() {
            Token t = new Token(TokenType.NUMBER, "42", 1, 1);
            ASTNode node = new ASTNode("number", List.of(t));
            assertTrue(node.isLeaf());
            assertEquals(t, node.getToken());
        }

        @Test void branchNode() {
            Token t1 = new Token(TokenType.NUMBER, "1", 1, 1);
            Token t2 = new Token(TokenType.NUMBER, "2", 1, 5);
            ASTNode child1 = new ASTNode("num", List.of(t1));
            ASTNode child2 = new ASTNode("num", List.of(t2));
            ASTNode parent = new ASTNode("add", List.of(child1, child2));
            assertFalse(parent.isLeaf());
            assertEquals(2, parent.getChildren().size());
        }

        @Test void descendantCount() {
            Token t = new Token(TokenType.NUMBER, "1", 1, 1);
            ASTNode leaf = new ASTNode("num", List.of(t));
            ASTNode parent = new ASTNode("expr", List.of(leaf));
            assertEquals(2, parent.descendantCount()); // leaf + token
        }

        @Test void positionTracking() {
            ASTNode node = new ASTNode("test", List.of(), 1, 5, 3, 10);
            assertEquals(1, node.getStartLine());
            assertEquals(5, node.getStartColumn());
            assertEquals(3, node.getEndLine());
            assertEquals(10, node.getEndColumn());
        }
    }

    // =========================================================================
    // Grammar Parser Tests
    // =========================================================================

    @Nested
    class GrammarParserTests {

        private List<Token> makeTokens(String... typesAndValues) {
            List<Token> tokens = new java.util.ArrayList<>();
            for (int i = 0; i < typesAndValues.length; i += 2) {
                tokens.add(new Token(TokenType.GRAMMAR, typesAndValues[i + 1],
                        1, tokens.size() + 1, typesAndValues[i]));
            }
            tokens.add(new Token(TokenType.EOF, "", 1, tokens.size() + 1, "EOF"));
            return tokens;
        }

        @Test void singleTokenRule() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("program = NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            List<Token> tokens = makeTokens("NUMBER", "42");
            ASTNode ast = parser.parse(tokens);
            assertEquals("program", ast.getRuleName());
        }

        @Test void sequenceRule() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("assign = NAME EQUALS NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            List<Token> tokens = makeTokens("NAME", "x", "EQUALS", "=", "NUMBER", "42");
            ASTNode ast = parser.parse(tokens);
            assertEquals("assign", ast.getRuleName());
        }

        @Test void alternationRule() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("value = NUMBER | NAME ;");
            GrammarParser parser = new GrammarParser(g);

            // First alternative
            ASTNode ast1 = parser.parse(makeTokens("NUMBER", "42"));
            assertEquals("value", ast1.getRuleName());

            // Second alternative
            ASTNode ast2 = parser.parse(makeTokens("NAME", "x"));
            assertEquals("value", ast2.getRuleName());
        }

        @Test void repetitionRule() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("list = { NUMBER } ;");
            GrammarParser parser = new GrammarParser(g);

            // Zero items
            ASTNode empty = parser.parse(makeTokens());
            assertEquals("list", empty.getRuleName());

            // Multiple items
            ASTNode multi = parser.parse(makeTokens("NUMBER", "1", "NUMBER", "2", "NUMBER", "3"));
            assertEquals("list", multi.getRuleName());
        }

        @Test void optionalRule() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("maybe = [ NUMBER ] ;");
            GrammarParser parser = new GrammarParser(g);

            // Present
            ASTNode present = parser.parse(makeTokens("NUMBER", "42"));
            assertEquals("maybe", present.getRuleName());

            // Absent
            ASTNode absent = parser.parse(makeTokens());
            assertEquals("maybe", absent.getRuleName());
        }

        @Test void nestedRules() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse(
                    "program = { statement } ;\nstatement = NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            ASTNode ast = parser.parse(makeTokens("NUMBER", "1", "NUMBER", "2"));
            assertEquals("program", ast.getRuleName());
        }

        @Test void parseFailure() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("program = NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            assertThrows(GrammarParseError.class, () -> parser.parse(makeTokens("NAME", "x")));
        }

        @Test void emptyGrammarThrows() {
            ParserGrammar g = new ParserGrammar();
            GrammarParser parser = new GrammarParser(g);
            assertThrows(GrammarParseError.class, () -> parser.parse(makeTokens()));
        }

        @Test void positiveLookahead() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("check = &NUMBER NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            ASTNode ast = parser.parse(makeTokens("NUMBER", "42"));
            assertEquals("check", ast.getRuleName());
        }

        @Test void negativeLookahead() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("check = !NAME NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            ASTNode ast = parser.parse(makeTokens("NUMBER", "42"));
            assertEquals("check", ast.getRuleName());
        }

        @Test void negativeLookaheadFails() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("check = !NUMBER NUMBER ;");
            GrammarParser parser = new GrammarParser(g);
            // Negative lookahead sees NUMBER, so it fails, and the whole parse fails
            assertThrows(GrammarParseError.class, () -> parser.parse(makeTokens("NUMBER", "42")));
        }

        @Test void separatedRepetition() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("args = { NUMBER // COMMA } ;");
            GrammarParser parser = new GrammarParser(g);
            ASTNode ast = parser.parse(makeTokens("NUMBER", "1", "COMMA", ",", "NUMBER", "2"));
            assertEquals("args", ast.getRuleName());
        }

        @Test void literalMatch() throws Exception {
            ParserGrammar g = ParserGrammarParser.parse("op = \"+\" ;");
            GrammarParser parser = new GrammarParser(g);
            ASTNode ast = parser.parse(makeTokens("PLUS", "+"));
            assertEquals("op", ast.getRuleName());
        }
    }
}
