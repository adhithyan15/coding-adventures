package com.codingadventures.lexer

import com.codingadventures.grammartools.parseTokenGrammar
import kotlin.test.*

class LexerTest {

    @Test fun tokenConstruction() {
        val t = Token(TokenType.NUMBER, "42", 1, 1)
        assertEquals(TokenType.NUMBER, t.type)
        assertEquals("42", t.value)
    }

    @Test fun effectiveTypeName() {
        assertEquals("NUMBER", Token(TokenType.NUMBER, "42", 1, 1).effectiveTypeName())
        assertEquals("INT", Token(TokenType.GRAMMAR, "42", 1, 1, "INT").effectiveTypeName())
    }

    @Test fun flags() {
        val t = Token(TokenType.GRAMMAR, "x", 1, 1, "NAME", FLAG_PRECEDED_BY_NEWLINE)
        assertTrue(t.hasFlag(FLAG_PRECEDED_BY_NEWLINE))
        assertFalse(t.hasFlag(FLAG_CONTEXT_KEYWORD))
    }

    @Test fun simpleTokenization() {
        val g = parseTokenGrammar("NUMBER = /[0-9]+/\nPLUS = \"+\"\nskip:\n  WS = /[ \\t]+/\n")
        val tokens = GrammarLexer(g).tokenize("42 + 7")
        assertEquals(4, tokens.size)
        assertEquals("NUMBER", tokens[0].typeName)
        assertEquals("42", tokens[0].value)
        assertEquals("PLUS", tokens[1].typeName)
        assertEquals("NUMBER", tokens[2].typeName)
        assertEquals("EOF", tokens[3].typeName)
    }

    @Test fun lineColumnTracking() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nNL = /\\n/\nskip:\n  WS = /[ \\t]+/\n")
        val tokens = GrammarLexer(g).tokenize("abc\ndef")
        assertEquals(1, tokens[0].line)
        assertEquals(1, tokens[0].column)
        assertEquals(2, tokens[2].line)
        assertEquals(1, tokens[2].column)
    }

    @Test fun keywordPromotion() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\nkeywords:\n  if\n  else\n")
        val tokens = GrammarLexer(g).tokenize("if x else y")
        assertEquals("KEYWORD", tokens[0].typeName)
        assertEquals("NAME", tokens[1].typeName)
        assertEquals("KEYWORD", tokens[2].typeName)
    }

    @Test fun aliasedToken() {
        val g = parseTokenGrammar("STRING_DQ = /\"[^\"]*\"/ -> STRING\n")
        val tokens = GrammarLexer(g).tokenize("\"hello\"")
        assertEquals("STRING", tokens[0].typeName)
    }

    @Test fun unexpectedCharacter() {
        val g = parseTokenGrammar("NUMBER = /[0-9]+/\n")
        assertFailsWith<LexerError> { GrammarLexer(g).tokenize("abc") }
    }

    @Test fun reservedKeyword() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nreserved:\n  class\n")
        assertFailsWith<LexerError> { GrammarLexer(g).tokenize("class") }
    }

    @Test fun emptyInput() {
        val g = parseTokenGrammar("NUMBER = /[0-9]+/\n")
        val tokens = GrammarLexer(g).tokenize("")
        assertEquals(1, tokens.size)
        assertEquals("EOF", tokens[0].typeName)
    }

    @Test fun contextKeywordFlag() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\ncontext_keywords:\n  async\n")
        val tokens = GrammarLexer(g).tokenize("async foo")
        assertTrue(tokens[0].hasFlag(FLAG_CONTEXT_KEYWORD))
        assertFalse(tokens[1].hasFlag(FLAG_CONTEXT_KEYWORD))
    }

    @Test fun errorRecovery() {
        val g = parseTokenGrammar("STRING = /\"[^\"]*\"/\nskip:\n  WS = /[ \\t]+/\nerrors:\n  BAD_STRING = /\"[^\"\\n]*/\n")
        val tokens = GrammarLexer(g).tokenize("\"unclosed")
        assertEquals("BAD_STRING", tokens[0].typeName)
    }

    @Test fun precededByNewlineFlag() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nskip:\n  WS = /[ \\t\\n]+/\n")
        val tokens = GrammarLexer(g).tokenize("a\nb")
        assertFalse(tokens[0].hasFlag(FLAG_PRECEDED_BY_NEWLINE))
        assertTrue(tokens[1].hasFlag(FLAG_PRECEDED_BY_NEWLINE))
    }
}
