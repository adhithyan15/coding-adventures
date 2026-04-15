package com.codingadventures.lexer;

import com.codingadventures.grammartools.TokenGrammar;
import com.codingadventures.grammartools.TokenGrammarParser;
import com.codingadventures.grammartools.TokenGrammarError;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Nested;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class LexerTest {

    // =========================================================================
    // Token Tests
    // =========================================================================

    @Nested
    class TokenTests {

        @Test void construction() {
            Token t = new Token(TokenType.NUMBER, "42", 1, 1);
            assertEquals(TokenType.NUMBER, t.getType());
            assertEquals("42", t.getValue());
            assertEquals(1, t.getLine());
            assertEquals(1, t.getColumn());
        }

        @Test void effectiveTypeNameWithoutTypeName() {
            Token t = new Token(TokenType.NUMBER, "42", 1, 1);
            assertEquals("NUMBER", t.effectiveTypeName());
        }

        @Test void effectiveTypeNameWithTypeName() {
            Token t = new Token(TokenType.GRAMMAR, "42", 1, 1, "INT");
            assertEquals("INT", t.effectiveTypeName());
        }

        @Test void flagPrecededByNewline() {
            Token t = new Token(TokenType.GRAMMAR, "x", 2, 1, "NAME", Token.FLAG_PRECEDED_BY_NEWLINE);
            assertTrue(t.hasFlag(Token.FLAG_PRECEDED_BY_NEWLINE));
            assertFalse(t.hasFlag(Token.FLAG_CONTEXT_KEYWORD));
        }

        @Test void flagContextKeyword() {
            Token t = new Token(TokenType.GRAMMAR, "async", 1, 1, "NAME", Token.FLAG_CONTEXT_KEYWORD);
            assertTrue(t.hasFlag(Token.FLAG_CONTEXT_KEYWORD));
        }

        @Test void toStringFormat() {
            Token t = new Token(TokenType.NUMBER, "42", 1, 5);
            assertEquals("Token(NUMBER, \"42\", 1:5)", t.toString());
        }

        @Test void equality() {
            Token a = new Token(TokenType.NUMBER, "42", 1, 1);
            Token b = new Token(TokenType.NUMBER, "42", 1, 1);
            assertEquals(a, b);
        }

        @Test void inequality() {
            Token a = new Token(TokenType.NUMBER, "42", 1, 1);
            Token b = new Token(TokenType.NUMBER, "43", 1, 1);
            assertNotEquals(a, b);
        }
    }

    // =========================================================================
    // Grammar Lexer Tests
    // =========================================================================

    @Nested
    class GrammarLexerTests {

        private TokenGrammar parseGrammar(String source) {
            try {
                return TokenGrammarParser.parse(source);
            } catch (TokenGrammarError e) {
                throw new RuntimeException(e);
            }
        }

        @Test void simpleTokenization() throws Exception {
            TokenGrammar g = parseGrammar(
                    "NUMBER = /[0-9]+/\nPLUS = \"+\"\nskip:\n  WS = /[ \\t]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("42 + 7");

            assertEquals(4, tokens.size()); // NUMBER, PLUS, NUMBER, EOF
            assertEquals("NUMBER", tokens.get(0).getTypeName());
            assertEquals("42", tokens.get(0).getValue());
            assertEquals("PLUS", tokens.get(1).getTypeName());
            assertEquals("NUMBER", tokens.get(2).getTypeName());
            assertEquals("7", tokens.get(2).getValue());
            assertEquals("EOF", tokens.get(3).getTypeName());
        }

        @Test void lineAndColumnTracking() throws Exception {
            TokenGrammar g = parseGrammar(
                    "NAME = /[a-z]+/\nNL = /\\n/\nskip:\n  WS = /[ \\t]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("abc\ndef");

            assertEquals("abc", tokens.get(0).getValue());
            assertEquals(1, tokens.get(0).getLine());
            assertEquals(1, tokens.get(0).getColumn());

            // NL token
            assertEquals(1, tokens.get(1).getLine());
            assertEquals(4, tokens.get(1).getColumn());

            assertEquals("def", tokens.get(2).getValue());
            assertEquals(2, tokens.get(2).getLine());
            assertEquals(1, tokens.get(2).getColumn());
        }

        @Test void keywordPromotion() throws Exception {
            TokenGrammar g = parseGrammar(
                    "NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\nkeywords:\n  if\n  else\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("if x else y");

            assertEquals("KEYWORD", tokens.get(0).getTypeName());
            assertEquals("if", tokens.get(0).getValue());
            assertEquals("NAME", tokens.get(1).getTypeName());
            assertEquals("KEYWORD", tokens.get(2).getTypeName());
            assertEquals("NAME", tokens.get(3).getTypeName());
        }

        @Test void aliasedToken() throws Exception {
            TokenGrammar g = parseGrammar(
                    "STRING_DQ = /\"[^\"]*\"/ -> STRING\nskip:\n  WS = /[ \\t]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("\"hello\"");

            assertEquals("STRING", tokens.get(0).getTypeName());
            assertEquals("\"hello\"", tokens.get(0).getValue());
        }

        @Test void unexpectedCharacterError() {
            TokenGrammar g = parseGrammar("NUMBER = /[0-9]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            assertThrows(LexerError.class, () -> lexer.tokenize("abc"));
        }

        @Test void reservedKeywordError() {
            TokenGrammar g = parseGrammar(
                    "NAME = /[a-z]+/\nreserved:\n  class\n");
            GrammarLexer lexer = new GrammarLexer(g);
            assertThrows(LexerError.class, () -> lexer.tokenize("class"));
        }

        @Test void emptyInput() throws Exception {
            TokenGrammar g = parseGrammar("NUMBER = /[0-9]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("");
            assertEquals(1, tokens.size());
            assertEquals("EOF", tokens.get(0).getTypeName());
        }

        @Test void multiplePatterns() throws Exception {
            TokenGrammar g = parseGrammar(
                    "NUMBER = /[0-9]+/\nNAME = /[a-zA-Z_][a-zA-Z0-9_]*/\nPLUS = \"+\"\nMINUS = \"-\"\n"
                    + "skip:\n  WS = /[ \\t]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("x + 42 - y");
            assertEquals(6, tokens.size()); // NAME PLUS NUMBER MINUS NAME EOF
        }

        @Test void firstMatchWins() throws Exception {
            // When two patterns could match at the same position, the first
            // definition in the file wins. Here FLOAT is listed before INT,
            // so "3.14" matches FLOAT rather than INT + error.
            TokenGrammar g = parseGrammar(
                    "FLOAT = /[0-9]+\\.[0-9]+/\nINT = /[0-9]+/\nskip:\n  WS = /[ \\t]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("3.14 42");
            assertEquals("FLOAT", tokens.get(0).getTypeName());
            assertEquals("3.14", tokens.get(0).getValue());
            assertEquals("INT", tokens.get(1).getTypeName());
            assertEquals("42", tokens.get(1).getValue());
        }

        @Test void contextKeywordFlag() throws Exception {
            TokenGrammar g = parseGrammar(
                    "NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\ncontext_keywords:\n  async\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("async foo");
            assertTrue(tokens.get(0).hasFlag(Token.FLAG_CONTEXT_KEYWORD));
            assertFalse(tokens.get(1).hasFlag(Token.FLAG_CONTEXT_KEYWORD));
        }

        @Test void errorRecoveryPatterns() throws Exception {
            TokenGrammar g = parseGrammar(
                    "STRING = /\"[^\"]*\"/\nskip:\n  WS = /[ \\t]+/\nerrors:\n  BAD_STRING = /\"[^\"\\n]*/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("\"unclosed");
            assertEquals("BAD_STRING", tokens.get(0).getTypeName());
        }

        @Test void precededByNewlineFlag() throws Exception {
            TokenGrammar g = parseGrammar(
                    "NAME = /[a-z]+/\nskip:\n  WS = /[ \\t\\n]+/\n");
            GrammarLexer lexer = new GrammarLexer(g);
            List<Token> tokens = lexer.tokenize("a\nb");
            assertFalse(tokens.get(0).hasFlag(Token.FLAG_PRECEDED_BY_NEWLINE));
            assertTrue(tokens.get(1).hasFlag(Token.FLAG_PRECEDED_BY_NEWLINE));
        }
    }
}
