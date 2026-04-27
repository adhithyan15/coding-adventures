package com.codingadventures.parser

import com.codingadventures.grammartools.*
import com.codingadventures.lexer.Token
import com.codingadventures.lexer.TokenType
import kotlin.test.*

class ParserTest {

    private fun makeTokens(vararg typesAndValues: String): List<Token> {
        val tokens = mutableListOf<Token>()
        var i = 0
        while (i < typesAndValues.size) {
            tokens.add(Token(TokenType.GRAMMAR, typesAndValues[i + 1], 1, tokens.size + 1, typesAndValues[i]))
            i += 2
        }
        tokens.add(Token(TokenType.EOF, "", 1, tokens.size + 1, "EOF"))
        return tokens
    }

    @Test fun emptyNode() {
        val node = ASTNode("test", emptyList())
        assertFalse(node.isLeaf)
        assertNull(node.token)
    }

    @Test fun leafNode() {
        val t = Token(TokenType.NUMBER, "42", 1, 1)
        val node = ASTNode("num", listOf(t))
        assertTrue(node.isLeaf)
        assertEquals(t, node.token)
    }

    @Test fun singleTokenRule() {
        val g = parseParserGrammar("program = NUMBER ;")
        val ast = GrammarParser(g).parse(makeTokens("NUMBER", "42"))
        assertEquals("program", ast.ruleName)
    }

    @Test fun sequenceRule() {
        val g = parseParserGrammar("assign = NAME EQUALS NUMBER ;")
        val ast = GrammarParser(g).parse(makeTokens("NAME", "x", "EQUALS", "=", "NUMBER", "42"))
        assertEquals("assign", ast.ruleName)
    }

    @Test fun alternationRule() {
        val g = parseParserGrammar("value = NUMBER | NAME ;")
        assertEquals("value", GrammarParser(g).parse(makeTokens("NUMBER", "42")).ruleName)
        assertEquals("value", GrammarParser(g).parse(makeTokens("NAME", "x")).ruleName)
    }

    @Test fun repetitionRule() {
        val g = parseParserGrammar("list = { NUMBER } ;")
        assertEquals("list", GrammarParser(g).parse(makeTokens()).ruleName)
        assertEquals("list", GrammarParser(g).parse(makeTokens("NUMBER", "1", "NUMBER", "2")).ruleName)
    }

    @Test fun optionalRule() {
        val g = parseParserGrammar("maybe = [ NUMBER ] ;")
        assertEquals("maybe", GrammarParser(g).parse(makeTokens("NUMBER", "42")).ruleName)
        assertEquals("maybe", GrammarParser(g).parse(makeTokens()).ruleName)
    }

    @Test fun nestedRules() {
        val g = parseParserGrammar("program = { statement } ;\nstatement = NUMBER ;")
        assertEquals("program", GrammarParser(g).parse(makeTokens("NUMBER", "1", "NUMBER", "2")).ruleName)
    }

    @Test fun parseFailure() {
        val g = parseParserGrammar("program = NUMBER ;")
        assertFailsWith<GrammarParseError> { GrammarParser(g).parse(makeTokens("NAME", "x")) }
    }

    @Test fun emptyGrammar() {
        val g = ParserGrammar()
        assertFailsWith<GrammarParseError> { GrammarParser(g).parse(makeTokens()) }
    }

    @Test fun positiveLookahead() {
        val g = parseParserGrammar("check = &NUMBER NUMBER ;")
        assertEquals("check", GrammarParser(g).parse(makeTokens("NUMBER", "42")).ruleName)
    }

    @Test fun negativeLookahead() {
        val g = parseParserGrammar("check = !NAME NUMBER ;")
        assertEquals("check", GrammarParser(g).parse(makeTokens("NUMBER", "42")).ruleName)
    }

    @Test fun negativeLookaheadFails() {
        val g = parseParserGrammar("check = !NUMBER NUMBER ;")
        assertFailsWith<GrammarParseError> { GrammarParser(g).parse(makeTokens("NUMBER", "42")) }
    }

    @Test fun separatedRepetition() {
        val g = parseParserGrammar("args = { NUMBER // COMMA } ;")
        assertEquals("args", GrammarParser(g).parse(makeTokens("NUMBER", "1", "COMMA", ",", "NUMBER", "2")).ruleName)
    }

    @Test fun literalMatch() {
        val g = parseParserGrammar("op = \"+\" ;")
        assertEquals("op", GrammarParser(g).parse(makeTokens("PLUS", "+")).ruleName)
    }

    @Test fun descendantCount() {
        val t = Token(TokenType.NUMBER, "1", 1, 1)
        val leaf = ASTNode("num", listOf(t))
        val parent = ASTNode("expr", listOf(leaf))
        assertEquals(2, parent.descendantCount())
    }
}
