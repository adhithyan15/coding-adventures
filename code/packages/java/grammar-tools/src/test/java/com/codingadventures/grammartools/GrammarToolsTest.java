package com.codingadventures.grammartools;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Nested;

import java.util.List;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.*;

class GrammarToolsTest {

    // =========================================================================
    // Token Grammar Parsing
    // =========================================================================

    @Nested
    class TokenGrammarParserTests {

        @Test void emptyInput() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("");
            assertTrue(g.getDefinitions().isEmpty());
            assertTrue(g.getKeywords().isEmpty());
        }

        @Test void commentsAndBlanks() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("# comment\n\n# another comment\n");
            assertTrue(g.getDefinitions().isEmpty());
        }

        @Test void simpleRegexDefinition() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("NUMBER = /[0-9]+/\n");
            assertEquals(1, g.getDefinitions().size());
            TokenDefinition d = g.getDefinitions().get(0);
            assertEquals("NUMBER", d.getName());
            assertEquals("[0-9]+", d.getPattern());
            assertTrue(d.isRegex());
            assertEquals(1, d.getLineNumber());
            assertNull(d.getAlias());
        }

        @Test void simpleLiteralDefinition() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("PLUS = \"+\"\n");
            assertEquals(1, g.getDefinitions().size());
            TokenDefinition d = g.getDefinitions().get(0);
            assertEquals("PLUS", d.getName());
            assertEquals("+", d.getPattern());
            assertFalse(d.isRegex());
        }

        @Test void aliasDefinition() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("STRING_DQ = /\"[^\"]*\"/ -> STRING\n");
            TokenDefinition d = g.getDefinitions().get(0);
            assertEquals("STRING_DQ", d.getName());
            assertEquals("STRING", d.getAlias());
        }

        @Test void keywordsSection() throws TokenGrammarError {
            String src = "NUMBER = /[0-9]+/\nkeywords:\n  if\n  else\n  while\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertEquals(List.of("if", "else", "while"), g.getKeywords());
        }

        @Test void reservedKeywords() throws TokenGrammarError {
            String src = "NUMBER = /[0-9]+/\nreserved:\n  class\n  import\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertEquals(List.of("class", "import"), g.getReservedKeywords());
        }

        @Test void skipSection() throws TokenGrammarError {
            String src = "NUMBER = /[0-9]+/\nskip:\n  WS = /[ \\t]+/\n  COMMENT = /\\/\\/.*/\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertEquals(2, g.getSkipDefinitions().size());
            assertEquals("WS", g.getSkipDefinitions().get(0).getName());
        }

        @Test void errorsSection() throws TokenGrammarError {
            String src = "NUMBER = /[0-9]+/\nerrors:\n  BAD_STRING = /\"[^\"\\n]*/\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertEquals(1, g.getErrorDefinitions().size());
            assertEquals("BAD_STRING", g.getErrorDefinitions().get(0).getName());
        }

        @Test void modeDirective() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("mode: indentation\nNUMBER = /[0-9]+/\n");
            assertEquals("indentation", g.getMode());
        }

        @Test void escapesDirective() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("escapes: none\nSTRING = /\"[^\"]*\"/\n");
            assertEquals("none", g.getEscapeMode());
        }

        @Test void caseSensitiveDirective() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("case_sensitive: false\nNUMBER = /[0-9]+/\n");
            assertFalse(g.isCaseSensitive());
        }

        @Test void magicCommentVersion() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("# @version 2\nNUMBER = /[0-9]+/\n");
            assertEquals(2, g.getVersion());
        }

        @Test void magicCommentCaseInsensitive() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("# @case_insensitive true\nNUMBER = /[0-9]+/\n");
            assertTrue(g.isCaseInsensitive());
        }

        @Test void patternGroup() throws TokenGrammarError {
            String src = "OPEN = \"<\"\ngroup tag:\n  ATTR_NAME = /[a-zA-Z]+/\n  EQUALS = \"=\"\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertTrue(g.getGroups().containsKey("tag"));
            assertEquals(2, g.getGroups().get("tag").getDefinitions().size());
        }

        @Test void contextKeywords() throws TokenGrammarError {
            String src = "NAME = /[a-z]+/\ncontext_keywords:\n  async\n  await\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertEquals(List.of("async", "await"), g.getContextKeywords());
        }

        @Test void tokenNames() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("NUMBER = /[0-9]+/\nSTRING_DQ = /\".*\"/ -> STRING\n");
            Set<String> names = g.tokenNames();
            assertTrue(names.contains("NUMBER"));
            assertTrue(names.contains("STRING_DQ"));
            assertTrue(names.contains("STRING"));
        }

        @Test void effectiveTokenNames() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("NUMBER = /[0-9]+/\nSTRING_DQ = /\".*\"/ -> STRING\n");
            Set<String> names = g.effectiveTokenNames();
            assertTrue(names.contains("NUMBER"));
            assertFalse(names.contains("STRING_DQ")); // replaced by alias
            assertTrue(names.contains("STRING"));
        }

        @Test void missingModeValue() {
            assertThrows(TokenGrammarError.class, () -> TokenGrammarParser.parse("mode:\n"));
        }

        @Test void unclosedRegex() {
            assertThrows(TokenGrammarError.class, () -> TokenGrammarParser.parse("BAD = /unclosed\n"));
        }

        @Test void emptyPattern() {
            assertThrows(TokenGrammarError.class, () -> TokenGrammarParser.parse("BAD = //\n"));
        }

        @Test void duplicateGroupName() {
            String src = "group tag:\n  A = \"a\"\ngroup tag:\n  B = \"b\"\n";
            assertThrows(TokenGrammarError.class, () -> TokenGrammarParser.parse(src));
        }

        @Test void reservedGroupName() {
            assertThrows(TokenGrammarError.class, () -> TokenGrammarParser.parse("group default:\n  A = \"a\"\n"));
        }

        @Test void multipleDefinitions() throws TokenGrammarError {
            String src = "NUMBER = /[0-9]+/\nPLUS = \"+\"\nMINUS = \"-\"\n";
            TokenGrammar g = TokenGrammarParser.parse(src);
            assertEquals(3, g.getDefinitions().size());
        }

        @Test void regexWithBrackets() throws TokenGrammarError {
            // Regex with character class containing /
            TokenGrammar g = TokenGrammarParser.parse("REGEX = /[a/b]+/\n");
            assertEquals("[a/b]+", g.getDefinitions().get(0).getPattern());
        }

        @Test void findClosingSlashSkipsEscaped() {
            assertEquals(7, TokenGrammarParser.findClosingSlash("/ab\\/cd/"));
        }

        @Test void findClosingSlashSkipsBrackets() {
            assertEquals(6, TokenGrammarParser.findClosingSlash("/[a/b]/"));
        }
    }

    // =========================================================================
    // Token Grammar Validation
    // =========================================================================

    @Nested
    class TokenGrammarValidatorTests {

        @Test void validGrammar() throws TokenGrammarError {
            TokenGrammar g = TokenGrammarParser.parse("NUMBER = /[0-9]+/\nPLUS = \"+\"\n");
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.isEmpty());
        }

        @Test void duplicateNames() throws TokenGrammarError {
            TokenGrammar g = new TokenGrammar();
            g.getDefinitions().add(new TokenDefinition("NUM", "[0-9]+", true, 1));
            g.getDefinitions().add(new TokenDefinition("NUM", "[0-9]+", true, 2));
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Duplicate")));
        }

        @Test void invalidRegex() throws TokenGrammarError {
            TokenGrammar g = new TokenGrammar();
            g.getDefinitions().add(new TokenDefinition("BAD", "[unclosed", true, 1));
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Invalid regex")));
        }

        @Test void nonUpperCaseName() throws TokenGrammarError {
            TokenGrammar g = new TokenGrammar();
            g.getDefinitions().add(new TokenDefinition("number", "[0-9]+", true, 1));
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.stream().anyMatch(s -> s.contains("UPPER_CASE")));
        }

        @Test void unknownMode() {
            TokenGrammar g = new TokenGrammar();
            g.setMode("unknown_mode");
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Unknown lexer mode")));
        }

        @Test void unknownEscapeMode() {
            TokenGrammar g = new TokenGrammar();
            g.setEscapeMode("bad");
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Unknown escape mode")));
        }

        @Test void emptyGroup() throws TokenGrammarError {
            TokenGrammar g = new TokenGrammar();
            g.getGroups().put("empty", new PatternGroup("empty", List.of()));
            List<String> issues = TokenGrammarValidator.validate(g);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Empty pattern group")));
        }
    }

    // =========================================================================
    // Parser Grammar Parsing
    // =========================================================================

    @Nested
    class ParserGrammarParserTests {

        @Test void simpleRule() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("program = statement ;");
            assertEquals(1, g.getRules().size());
            assertEquals("program", g.getRules().get(0).name());
        }

        @Test void alternation() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("expr = NUMBER | NAME ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Alternation.class, body);
            assertEquals(2, ((GrammarElement.Alternation) body).choices().size());
        }

        @Test void sequence() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("assign = NAME EQUALS NUMBER SEMI ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Sequence.class, body);
        }

        @Test void repetition() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("list = { item } ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Repetition.class, body);
        }

        @Test void oneOrMore() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("list = { item }+ ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.OneOrMoreRepetition.class, body);
        }

        @Test void optional() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("call = NAME LPAREN [ args ] RPAREN ;");
            // body is a sequence with optional inside
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Sequence.class, body);
        }

        @Test void group() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("expr = term (PLUS | MINUS) term ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Sequence.class, body);
        }

        @Test void literal() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("op = \"+\" ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Literal.class, body);
            assertEquals("+", ((GrammarElement.Literal) body).value());
        }

        @Test void positiveLookahead() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("check = &NUMBER item ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Sequence.class, body);
        }

        @Test void negativeLookahead() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("check = !NUMBER item ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.Sequence.class, body);
        }

        @Test void separatedRepetition() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("args = { expr // COMMA } ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.SeparatedRepetition.class, body);
            GrammarElement.SeparatedRepetition sep = (GrammarElement.SeparatedRepetition) body;
            assertFalse(sep.atLeastOne());
        }

        @Test void separatedRepetitionAtLeastOne() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("args = { expr // COMMA }+ ;");
            GrammarElement body = g.getRules().get(0).body();
            assertInstanceOf(GrammarElement.SeparatedRepetition.class, body);
            assertTrue(((GrammarElement.SeparatedRepetition) body).atLeastOne());
        }

        @Test void multipleRules() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("a = NUMBER ;\nb = NAME ;");
            assertEquals(2, g.getRules().size());
        }

        @Test void ruleReferences() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("program = { statement } ;\nstatement = expr SEMI ;");
            assertTrue(g.ruleReferences().contains("statement"));
            assertTrue(g.ruleReferences().contains("expr"));
        }

        @Test void tokenReferences() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("assign = NAME EQUALS NUMBER ;");
            Set<String> refs = g.tokenReferences();
            assertTrue(refs.contains("NAME"));
            assertTrue(refs.contains("EQUALS"));
            assertTrue(refs.contains("NUMBER"));
        }

        @Test void magicVersion() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("# @version 3\nexpr = NUMBER ;");
            assertEquals(3, g.getVersion());
        }

        @Test void unexpectedToken() {
            assertThrows(ParserGrammarError.class, () -> ParserGrammarParser.parse("123"));
        }
    }

    // =========================================================================
    // Parser Grammar Validation
    // =========================================================================

    @Nested
    class ParserGrammarValidatorTests {

        @Test void validGrammar() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("program = { statement } ;\nstatement = NUMBER ;");
            List<String> issues = ParserGrammarValidator.validate(g, null);
            assertTrue(issues.isEmpty());
        }

        @Test void duplicateRuleName() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("a = NUMBER ;\na = NAME ;");
            List<String> issues = ParserGrammarValidator.validate(g, null);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Duplicate rule name")));
        }

        @Test void undefinedRuleRef() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("program = undefined_rule ;");
            List<String> issues = ParserGrammarValidator.validate(g, null);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Undefined rule reference")));
        }

        @Test void undefinedTokenRef() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("program = NONEXISTENT ;");
            Set<String> tokenNames = Set.of("NUMBER", "NAME");
            List<String> issues = ParserGrammarValidator.validate(g, tokenNames);
            assertTrue(issues.stream().anyMatch(s -> s.contains("Undefined token reference")));
        }

        @Test void syntheticTokensAlwaysValid() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("program = NEWLINE EOF ;");
            List<String> issues = ParserGrammarValidator.validate(g, Set.of());
            assertFalse(issues.stream().anyMatch(s -> s.contains("Undefined token")));
        }

        @Test void unreachableRule() throws ParserGrammarError {
            ParserGrammar g = ParserGrammarParser.parse("start = NUMBER ;\nunused = NAME ;");
            List<String> issues = ParserGrammarValidator.validate(g, null);
            assertTrue(issues.stream().anyMatch(s -> s.contains("unreachable")));
        }
    }

    // =========================================================================
    // Cross Validation
    // =========================================================================

    @Nested
    class CrossValidatorTests {

        @Test void consistentGrammars() throws Exception {
            TokenGrammar tg = TokenGrammarParser.parse("NUMBER = /[0-9]+/\nNAME = /[a-z]+/\n");
            ParserGrammar pg = ParserGrammarParser.parse("expr = NUMBER | NAME ;");
            List<String> issues = CrossValidator.crossValidate(tg, pg);
            assertTrue(issues.isEmpty());
        }

        @Test void missingToken() throws Exception {
            TokenGrammar tg = TokenGrammarParser.parse("NUMBER = /[0-9]+/\n");
            ParserGrammar pg = ParserGrammarParser.parse("expr = NUMBER | NONEXISTENT ;");
            List<String> issues = CrossValidator.crossValidate(tg, pg);
            assertTrue(issues.stream().anyMatch(s -> s.startsWith("Error:") && s.contains("NONEXISTENT")));
        }

        @Test void unusedToken() throws Exception {
            TokenGrammar tg = TokenGrammarParser.parse("NUMBER = /[0-9]+/\nUNUSED = \"~\"\n");
            ParserGrammar pg = ParserGrammarParser.parse("expr = NUMBER ;");
            List<String> issues = CrossValidator.crossValidate(tg, pg);
            assertTrue(issues.stream().anyMatch(s -> s.startsWith("Warning:") && s.contains("UNUSED")));
        }

        @Test void syntheticTokensValid() throws Exception {
            TokenGrammar tg = TokenGrammarParser.parse("NUMBER = /[0-9]+/\n");
            ParserGrammar pg = ParserGrammarParser.parse("program = NUMBER NEWLINE EOF ;");
            List<String> issues = CrossValidator.crossValidate(tg, pg);
            assertFalse(issues.stream().anyMatch(s -> s.contains("NEWLINE") || s.contains("EOF")));
        }

        @Test void aliasedTokenUsed() throws Exception {
            TokenGrammar tg = TokenGrammarParser.parse("STRING_DQ = /\"[^\"]*\"/ -> STRING\n");
            ParserGrammar pg = ParserGrammarParser.parse("value = STRING ;");
            List<String> issues = CrossValidator.crossValidate(tg, pg);
            assertFalse(issues.stream().anyMatch(s -> s.startsWith("Warning:")));
        }

        @Test void indentDedenValidInIndentMode() throws Exception {
            TokenGrammar tg = TokenGrammarParser.parse("mode: indentation\nNUMBER = /[0-9]+/\n");
            ParserGrammar pg = ParserGrammarParser.parse("block = INDENT { NUMBER } DEDENT ;");
            List<String> issues = CrossValidator.crossValidate(tg, pg);
            assertFalse(issues.stream().anyMatch(s -> s.contains("INDENT") || s.contains("DEDENT")));
        }
    }
}
