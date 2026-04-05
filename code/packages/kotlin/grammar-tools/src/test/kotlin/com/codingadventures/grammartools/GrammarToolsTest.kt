package com.codingadventures.grammartools

import kotlin.test.*

class GrammarToolsTest {

    // === Token Grammar Parsing ===

    @Test fun emptyInput() {
        val g = parseTokenGrammar("")
        assertTrue(g.definitions.isEmpty())
    }

    @Test fun simpleRegex() {
        val g = parseTokenGrammar("NUMBER = /[0-9]+/\n")
        assertEquals(1, g.definitions.size)
        assertEquals("NUMBER", g.definitions[0].name)
        assertEquals("[0-9]+", g.definitions[0].pattern)
        assertTrue(g.definitions[0].isRegex)
    }

    @Test fun simpleLiteral() {
        val g = parseTokenGrammar("PLUS = \"+\"\n")
        assertEquals("+", g.definitions[0].pattern)
        assertFalse(g.definitions[0].isRegex)
    }

    @Test fun aliasDefinition() {
        val g = parseTokenGrammar("STRING_DQ = /\"[^\"]*\"/ -> STRING\n")
        assertEquals("STRING", g.definitions[0].alias)
    }

    @Test fun keywordsSection() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nkeywords:\n  if\n  else\n")
        assertEquals(listOf("if", "else"), g.keywords)
    }

    @Test fun reservedKeywords() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nreserved:\n  class\n")
        assertEquals(listOf("class"), g.reservedKeywords)
    }

    @Test fun skipSection() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\n")
        assertEquals(1, g.skipDefinitions.size)
    }

    @Test fun errorsSection() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\nerrors:\n  BAD = /[^\\n]*/\n")
        assertEquals(1, g.errorDefinitions.size)
    }

    @Test fun modeDirective() {
        val g = parseTokenGrammar("mode: indentation\nNAME = /[a-z]+/\n")
        assertEquals("indentation", g.mode)
    }

    @Test fun escapesDirective() {
        val g = parseTokenGrammar("escapes: none\nSTRING = /\"[^\"]*\"/\n")
        assertEquals("none", g.escapeMode)
    }

    @Test fun caseSensitiveDirective() {
        val g = parseTokenGrammar("case_sensitive: false\nNAME = /[a-z]+/\n")
        assertFalse(g.caseSensitive)
    }

    @Test fun magicCommentVersion() {
        val g = parseTokenGrammar("# @version 2\nNUMBER = /[0-9]+/\n")
        assertEquals(2, g.version)
    }

    @Test fun patternGroup() {
        val g = parseTokenGrammar("OPEN = \"<\"\ngroup tag:\n  ATTR = /[a-z]+/\n")
        assertTrue("tag" in g.groups)
        assertEquals(1, g.groups["tag"]!!.definitions.size)
    }

    @Test fun contextKeywords() {
        val g = parseTokenGrammar("NAME = /[a-z]+/\ncontext_keywords:\n  async\n")
        assertEquals(listOf("async"), g.contextKeywords)
    }

    @Test fun tokenNames() {
        val g = parseTokenGrammar("NUM = /[0-9]+/\nSTR_DQ = /\".*\"/ -> STR\n")
        val names = g.tokenNames()
        assertTrue("NUM" in names)
        assertTrue("STR_DQ" in names)
        assertTrue("STR" in names)
    }

    @Test fun effectiveTokenNames() {
        val g = parseTokenGrammar("NUM = /[0-9]+/\nSTR_DQ = /\".*\"/ -> STR\n")
        val names = g.effectiveTokenNames()
        assertTrue("NUM" in names)
        assertFalse("STR_DQ" in names)
        assertTrue("STR" in names)
    }

    @Test fun unclosedRegex() {
        assertFailsWith<TokenGrammarError> { parseTokenGrammar("BAD = /unclosed\n") }
    }

    @Test fun duplicateGroup() {
        assertFailsWith<TokenGrammarError> { parseTokenGrammar("group a:\n  X = \"x\"\ngroup a:\n  Y = \"y\"\n") }
    }

    @Test fun reservedGroupName() {
        assertFailsWith<TokenGrammarError> { parseTokenGrammar("group default:\n  X = \"x\"\n") }
    }

    // === Token Grammar Validation ===

    @Test fun validGrammar() {
        val g = parseTokenGrammar("NUMBER = /[0-9]+/\n")
        assertTrue(validateTokenGrammar(g).isEmpty())
    }

    @Test fun duplicateNameWarning() {
        val g = TokenGrammar(definitions = mutableListOf(
            TokenDefinition("NUM", "[0-9]+", true, 1),
            TokenDefinition("NUM", "[0-9]+", true, 2)
        ))
        assertTrue(validateTokenGrammar(g).any { "Duplicate" in it })
    }

    @Test fun invalidRegexWarning() {
        val g = TokenGrammar(definitions = mutableListOf(
            TokenDefinition("BAD", "[unclosed", true, 1)
        ))
        assertTrue(validateTokenGrammar(g).any { "Invalid regex" in it })
    }

    @Test fun nonUpperCaseWarning() {
        val g = TokenGrammar(definitions = mutableListOf(
            TokenDefinition("number", "[0-9]+", true, 1)
        ))
        assertTrue(validateTokenGrammar(g).any { "UPPER_CASE" in it })
    }

    @Test fun unknownMode() {
        val g = TokenGrammar(mode = "bad")
        assertTrue(validateTokenGrammar(g).any { "Unknown lexer mode" in it })
    }

    // === Parser Grammar Parsing ===

    @Test fun simpleRule() {
        val g = parseParserGrammar("program = NUMBER ;")
        assertEquals(1, g.rules.size)
        assertEquals("program", g.rules[0].name)
    }

    @Test fun alternation() {
        val g = parseParserGrammar("expr = NUMBER | NAME ;")
        assertIs<Alternation>(g.rules[0].body)
    }

    @Test fun sequence() {
        val g = parseParserGrammar("a = NAME EQUALS NUMBER ;")
        assertIs<Sequence>(g.rules[0].body)
    }

    @Test fun repetition() {
        val g = parseParserGrammar("list = { item } ;")
        assertIs<Repetition>(g.rules[0].body)
    }

    @Test fun oneOrMore() {
        val g = parseParserGrammar("list = { item }+ ;")
        assertIs<OneOrMoreRepetition>(g.rules[0].body)
    }

    @Test fun optional() {
        val g = parseParserGrammar("call = NAME [ args ] ;")
        assertIs<Sequence>(g.rules[0].body)
    }

    @Test fun literal() {
        val g = parseParserGrammar("op = \"+\" ;")
        assertIs<Literal>(g.rules[0].body)
        assertEquals("+", (g.rules[0].body as Literal).value)
    }

    @Test fun positiveLookahead() {
        val g = parseParserGrammar("c = &NUMBER item ;")
        assertIs<Sequence>(g.rules[0].body)
    }

    @Test fun negativeLookahead() {
        val g = parseParserGrammar("c = !NUMBER item ;")
        assertIs<Sequence>(g.rules[0].body)
    }

    @Test fun separatedRepetition() {
        val g = parseParserGrammar("args = { expr // COMMA } ;")
        assertIs<SeparatedRepetition>(g.rules[0].body)
    }

    @Test fun ruleReferences() {
        val g = parseParserGrammar("p = { stmt } ;\nstmt = expr ;")
        assertTrue("stmt" in g.ruleReferences())
        assertTrue("expr" in g.ruleReferences())
    }

    @Test fun tokenReferences() {
        val g = parseParserGrammar("a = NAME EQUALS NUMBER ;")
        assertTrue("NAME" in g.tokenReferences())
        assertTrue("EQUALS" in g.tokenReferences())
    }

    @Test fun magicVersion() {
        val g = parseParserGrammar("# @version 3\nexpr = NUMBER ;")
        assertEquals(3, g.version)
    }

    // === Parser Grammar Validation ===

    @Test fun validParserGrammar() {
        val g = parseParserGrammar("p = { s } ;\ns = NUMBER ;")
        assertTrue(validateParserGrammar(g).isEmpty())
    }

    @Test fun undefinedRuleRef() {
        val g = parseParserGrammar("p = undef ;")
        assertTrue(validateParserGrammar(g).any { "Undefined rule" in it })
    }

    @Test fun undefinedTokenRef() {
        val g = parseParserGrammar("p = MISSING ;")
        assertTrue(validateParserGrammar(g, setOf("NUMBER")).any { "Undefined token" in it })
    }

    @Test fun syntheticTokensValid() {
        val g = parseParserGrammar("p = NEWLINE EOF ;")
        assertFalse(validateParserGrammar(g, emptySet()).any { "Undefined token" in it })
    }

    @Test fun unreachableRule() {
        val g = parseParserGrammar("start = NUMBER ;\nunused = NAME ;")
        assertTrue(validateParserGrammar(g).any { "unreachable" in it })
    }

    // === Cross Validation ===

    @Test fun consistentGrammars() {
        val tg = parseTokenGrammar("NUMBER = /[0-9]+/\nNAME = /[a-z]+/\n")
        val pg = parseParserGrammar("expr = NUMBER | NAME ;")
        assertTrue(crossValidate(tg, pg).isEmpty())
    }

    @Test fun missingToken() {
        val tg = parseTokenGrammar("NUMBER = /[0-9]+/\n")
        val pg = parseParserGrammar("expr = NUMBER | MISSING ;")
        assertTrue(crossValidate(tg, pg).any { "Error:" in it && "MISSING" in it })
    }

    @Test fun unusedToken() {
        val tg = parseTokenGrammar("NUMBER = /[0-9]+/\nUNUSED = \"~\"\n")
        val pg = parseParserGrammar("expr = NUMBER ;")
        assertTrue(crossValidate(tg, pg).any { "Warning:" in it && "UNUSED" in it })
    }
}
