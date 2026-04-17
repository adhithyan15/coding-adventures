-- Tests for grammar-tools — comprehensive busted test suite.
--
-- This test file covers all three modules:
--   1. Token grammar parsing and validation
--   2. Parser grammar parsing and validation
--   3. Cross-validation between token and parser grammars
--
-- We target 95%+ code coverage by testing:
--   - Happy paths (valid inputs producing correct output)
--   - Error paths (invalid inputs producing correct error messages)
--   - Edge cases (empty inputs, boundary conditions)
--   - All section types (keywords, reserved, skip, errors, groups)
--   - All grammar element types (reference, literal, sequence, alternation,
--     repetition, optional, group)

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local grammar_tools = require("coding_adventures.grammar_tools")

-- ============================================================================
-- Utility: check that an issue list contains a string
-- ============================================================================

local function has_issue(issues, pattern)
    for _, issue in ipairs(issues) do
        if issue:find(pattern, 1, true) then
            return true
        end
    end
    return false
end

-- ============================================================================
-- Module basics
-- ============================================================================

describe("grammar-tools", function()
    it("has a version", function()
        assert.are.equal("0.1.0", grammar_tools.VERSION)
    end)

    it("exports expected types and functions", function()
        assert.is_not_nil(grammar_tools.TokenDefinition)
        assert.is_not_nil(grammar_tools.PatternGroup)
        assert.is_not_nil(grammar_tools.TokenGrammar)
        assert.is_not_nil(grammar_tools.GrammarRule)
        assert.is_not_nil(grammar_tools.ParserGrammar)
        assert.is_function(grammar_tools.parse_token_grammar)
        assert.is_function(grammar_tools.parse_parser_grammar)
        assert.is_function(grammar_tools.validate_token_grammar)
        assert.is_function(grammar_tools.validate_parser_grammar)
        assert.is_function(grammar_tools.cross_validate)
    end)
end)

-- ============================================================================
-- TokenDefinition
-- ============================================================================

describe("TokenDefinition", function()
    it("creates a definition with all fields", function()
        local defn = grammar_tools.TokenDefinition.new({
            name = "NUMBER",
            pattern = "[0-9]+",
            is_regex = true,
            line_number = 5,
            alias = "NUM",
        })
        assert.are.equal("NUMBER", defn.name)
        assert.are.equal("[0-9]+", defn.pattern)
        assert.is_true(defn.is_regex)
        assert.are.equal(5, defn.line_number)
        assert.are.equal("NUM", defn.alias)
    end)

    it("defaults fields when not provided", function()
        local defn = grammar_tools.TokenDefinition.new({})
        assert.are.equal("", defn.name)
        assert.are.equal("", defn.pattern)
        assert.is_false(defn.is_regex)
        assert.are.equal(0, defn.line_number)
        assert.are.equal("", defn.alias)
    end)
end)

-- ============================================================================
-- PatternGroup
-- ============================================================================

describe("PatternGroup", function()
    it("creates a group with name and definitions", function()
        local group = grammar_tools.PatternGroup.new("tag", {})
        assert.are.equal("tag", group.name)
        assert.are.same({}, group.definitions)
    end)

    it("defaults definitions to empty table", function()
        local group = grammar_tools.PatternGroup.new("cdata")
        assert.are.same({}, group.definitions)
    end)
end)

-- ============================================================================
-- TokenGrammar construction
-- ============================================================================

describe("TokenGrammar", function()
    it("creates an empty grammar", function()
        local g = grammar_tools.TokenGrammar.new()
        assert.are.same({}, g.definitions)
        assert.are.same({}, g.keywords)
        assert.are.equal("", g.mode)
        assert.are.equal("", g.escape_mode)
        assert.are.same({}, g.skip_definitions)
        assert.are.same({}, g.error_definitions)
        assert.are.same({}, g.reserved_keywords)
        assert.are.same({}, g.groups)
    end)
end)

-- ============================================================================
-- parse_token_grammar — happy paths
-- ============================================================================

describe("parse_token_grammar", function()
    it("parses basic token definitions", function()
        local source = [[
NAME = /[a-zA-Z_]+/
EQUALS = "="
keywords:
  if
  else
]]
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, #grammar.definitions)
        assert.are.equal("NAME", grammar.definitions[1].name)
        assert.are.equal("[a-zA-Z_]+", grammar.definitions[1].pattern)
        assert.is_true(grammar.definitions[1].is_regex)
        assert.are.equal("EQUALS", grammar.definitions[2].name)
        assert.are.equal("=", grammar.definitions[2].pattern)
        assert.is_false(grammar.definitions[2].is_regex)
        assert.are.equal(2, #grammar.keywords)
        assert.are.equal("if", grammar.keywords[1])
        assert.are.equal("else", grammar.keywords[2])
    end)

    it("parses mode directive", function()
        local grammar, err = grammar_tools.parse_token_grammar("mode: indentation\nNAME = /[a-z]+/")
        assert.is_nil(err)
        assert.are.equal("indentation", grammar.mode)
    end)

    it("parses layout keywords section", function()
        local source = "mode: layout\nNAME = /[a-z]+/\nlayout_keywords:\n  let\n  where\n  do\n  of"
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal("layout", grammar.mode)
        assert.are.same({ "let", "where", "do", "of" }, grammar.layout_keywords)
    end)

    it("parses escapes directive", function()
        local grammar, err = grammar_tools.parse_token_grammar("escapes: none\nNAME = /[a-z]+/")
        assert.is_nil(err)
        assert.are.equal("none", grammar.escape_mode)
    end)

    it("parses skip section", function()
        local source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/"
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, #grammar.skip_definitions)
        assert.are.equal("WHITESPACE", grammar.skip_definitions[1].name)
        assert.are.equal("COMMENT", grammar.skip_definitions[2].name)
    end)

    it("parses reserved section", function()
        local source = "NAME = /[a-z]+/\nreserved:\n  class\n  import"
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, #grammar.reserved_keywords)
        assert.are.equal("class", grammar.reserved_keywords[1])
        assert.are.equal("import", grammar.reserved_keywords[2])
    end)

    it("parses errors section with regex", function()
        local source = 'NAME = /[a-z]+/\nerrors:\n  BAD_STRING = /"[^"\\n]*/\n  BAD_CHAR = /./'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, #grammar.error_definitions)
        assert.are.equal("BAD_STRING", grammar.error_definitions[1].name)
        assert.is_true(grammar.error_definitions[1].is_regex)
        assert.are.equal("BAD_CHAR", grammar.error_definitions[2].name)
    end)

    it("parses errors section with literal", function()
        local source = 'NAME = /[a-z]+/\nerrors:\n  BAD_EQ = "=="'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(1, #grammar.error_definitions)
        assert.are.equal("==", grammar.error_definitions[1].pattern)
    end)

    it("parses regex alias (-> ALIAS)", function()
        local source = 'STRING_DQ = /"[^"]*"/ -> STRING'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal("STRING", grammar.definitions[1].alias)
    end)

    it("parses literal alias (-> ALIAS)", function()
        local source = 'PLUS_SIGN = "+" -> PLUS'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal("PLUS", grammar.definitions[1].alias)
    end)

    it("handles comments and blank lines", function()
        local source = "# comment\nNAME = /[a-z]+/\n# another comment\n\n"
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(1, #grammar.definitions)
    end)

    it("exits section on non-indented line", function()
        local source = "keywords:\n  if\nNAME = /[a-z]+/"
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(1, #grammar.keywords)
        assert.are.equal("if", grammar.keywords[1])
        assert.are.equal(1, #grammar.definitions)
        assert.are.equal("NAME", grammar.definitions[1].name)
    end)

    it("handles alternate section headers (space before colon)", function()
        local g1, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\nkeywords :\n  for")
        assert.are.equal(1, #g1.keywords)
        assert.are.equal("for", g1.keywords[1])

        local g2, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\nreserved :\n  yield")
        assert.are.equal(1, #g2.reserved_keywords)
        assert.are.equal("yield", g2.reserved_keywords[1])

        local g3, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\nskip :\n  WS = /[ ]+/")
        assert.are.equal(1, #g3.skip_definitions)

        local g4, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\nerrors :\n  BAD = /./")
        assert.are.equal(1, #g4.error_definitions)
    end)

    it("parses a starlark-like token file", function()
        local source = [[
mode: indentation

NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
EQUALS = "="
PLUS = "+"
COLON = ":"
LPAREN = "("
RPAREN = ")"
COMMA = ","

keywords:
  def
  return
  if
  else
  for
  in
  pass

reserved:
  class
  import

skip:
  WHITESPACE = /[ \t]+/
  COMMENT = /#[^\n]*/
]]
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal("indentation", grammar.mode)
        assert.are.equal(2, #grammar.reserved_keywords)
        assert.are.equal(2, #grammar.skip_definitions)
        assert.are.equal(8, #grammar.definitions)
    end)

    -- Groups
    it("parses a basic group", function()
        local source = 'TEXT = /[^<]+/\nTAG_OPEN = "<"\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n  TAG_CLOSE = ">"\n'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, #grammar.definitions)
        assert.is_not_nil(grammar.groups["tag"])
        assert.are.equal("tag", grammar.groups["tag"].name)
        assert.are.equal(2, #grammar.groups["tag"].definitions)
        assert.are.equal("TAG_NAME", grammar.groups["tag"].definitions[1].name)
        assert.are.equal("TAG_CLOSE", grammar.groups["tag"].definitions[2].name)
    end)

    it("parses multiple groups", function()
        local source = 'TEXT = /[^<]+/\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n\ngroup cdata:\n  CDATA_TEXT = /[^]]+/\n  CDATA_END = "]]>"\n'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.is_not_nil(grammar.groups["tag"])
        assert.is_not_nil(grammar.groups["cdata"])
        assert.are.equal(1, #grammar.groups["tag"].definitions)
        assert.are.equal(2, #grammar.groups["cdata"].definitions)
    end)

    it("parses group with alias", function()
        local source = 'TEXT = /[^<]+/\n\ngroup tag:\n  ATTR_VALUE_DQ = /"[^"]*"/ -> ATTR_VALUE\n'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        local group = grammar.groups["tag"]
        assert.are.equal("ATTR_VALUE_DQ", group.definitions[1].name)
        assert.are.equal("ATTR_VALUE", group.definitions[1].alias)
    end)

    it("parses group with literal and regex patterns", function()
        local source = 'TEXT = /[^<]+/\n\ngroup tag:\n  EQUALS = "="\n  TAG_NAME = /[a-zA-Z]+/\n'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        local group = grammar.groups["tag"]
        assert.is_false(group.definitions[1].is_regex)
        assert.are.equal("=", group.definitions[1].pattern)
        assert.is_true(group.definitions[2].is_regex)
    end)

    it("has empty groups map when no groups defined", function()
        local source = 'NUMBER = /[0-9]+/\nPLUS = "+"\n'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.is_table(grammar.groups)
        -- Count entries
        local count = 0
        for _ in pairs(grammar.groups) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("handles groups and skip sections together", function()
        local source = 'skip:\n  WS = /[ \\t]+/\n\nTEXT = /[^<]+/\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n'
        local grammar, err = grammar_tools.parse_token_grammar(source)
        assert.is_nil(err)
        assert.are.equal(1, #grammar.skip_definitions)
        assert.are.equal(1, #grammar.definitions)
        assert.is_not_nil(grammar.groups["tag"])
    end)
end)

-- ============================================================================
-- parse_token_grammar — error paths
-- ============================================================================

describe("parse_token_grammar errors", function()
    it("rejects missing mode value", function()
        local _, err = grammar_tools.parse_token_grammar("mode:")
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing value after 'mode:'"))
    end)

    it("rejects missing escapes value", function()
        local _, err = grammar_tools.parse_token_grammar("escapes:")
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing value after 'escapes:'"))
    end)

    it("rejects unclosed regex", function()
        local _, err = grammar_tools.parse_token_grammar("FOO = /unclosed")
        assert.is_not_nil(err)
        assert.truthy(err:find("Unclosed regex"))
    end)

    it("rejects unclosed literal", function()
        local _, err = grammar_tools.parse_token_grammar('FOO = "unclosed')
        assert.is_not_nil(err)
        assert.truthy(err:find("Unclosed literal"))
    end)

    it("rejects empty regex", function()
        local _, err = grammar_tools.parse_token_grammar("FOO = //")
        assert.is_not_nil(err)
        assert.truthy(err:find("Empty regex"))
    end)

    it("rejects empty literal", function()
        local _, err = grammar_tools.parse_token_grammar('FOO = ""')
        assert.is_not_nil(err)
        assert.truthy(err:find("Empty literal"))
    end)

    it("rejects missing alias after ->", function()
        local _, err = grammar_tools.parse_token_grammar("FOO = /x/ ->")
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing alias"))
    end)

    it("rejects missing alias after -> on literal", function()
        local _, err = grammar_tools.parse_token_grammar('FOO = "+" ->')
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing alias"))
    end)

    it("rejects unexpected text after regex", function()
        local _, err = grammar_tools.parse_token_grammar("FOO = /abc/ extra")
        assert.is_not_nil(err)
        assert.truthy(err:find("Unexpected text"))
    end)

    it("rejects unexpected text after literal", function()
        local _, err = grammar_tools.parse_token_grammar('FOO = "+" extra')
        assert.is_not_nil(err)
        assert.truthy(err:find("Unexpected text"))
    end)

    it("rejects bad pattern delimiter", function()
        local _, err = grammar_tools.parse_token_grammar("FOO = xyz")
        assert.is_not_nil(err)
        assert.truthy(err:find("Pattern must be"))
    end)

    it("rejects missing token name", function()
        local _, err = grammar_tools.parse_token_grammar(" = /abc/")
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing token name"))
    end)

    it("rejects missing pattern after =", function()
        local _, err = grammar_tools.parse_token_grammar("FOO = ")
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing pattern"))
    end)

    it("rejects line without =", function()
        local _, err = grammar_tools.parse_token_grammar("FOO /abc/")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected token definition"))
    end)

    it("rejects skip without =", function()
        local _, err = grammar_tools.parse_token_grammar("skip:\n  BAD_PATTERN")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected skip pattern"))
    end)

    it("rejects incomplete skip", function()
        local _, err = grammar_tools.parse_token_grammar("skip:\n  BAD =")
        assert.is_not_nil(err)
        assert.truthy(err:find("Incomplete skip"))
    end)

    it("rejects error without =", function()
        local _, err = grammar_tools.parse_token_grammar("NAME = /x/\nerrors:\n  BAD_PATTERN")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected error pattern"))
    end)

    it("rejects incomplete error", function()
        local _, err = grammar_tools.parse_token_grammar("NAME = /x/\nerrors:\n  BAD =")
        assert.is_not_nil(err)
        assert.truthy(err:find("Incomplete error pattern"))
    end)

    -- Group error paths
    it("rejects missing group name", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup :\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Missing group name"))
    end)

    it("rejects uppercase group name", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup Tag:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Invalid group name"))
    end)

    it("rejects group name starting with digit", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup 1tag:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Invalid group name"))
    end)

    it("rejects reserved group name 'default'", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup default:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Reserved group name"))
    end)

    it("rejects reserved group name 'skip'", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup skip:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Reserved group name"))
    end)

    it("rejects reserved group name 'keywords'", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup keywords:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Reserved group name"))
    end)

    it("rejects reserved group name 'reserved'", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup reserved:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Reserved group name"))
    end)

    it("rejects reserved group name 'errors'", function()
        local _, err = grammar_tools.parse_token_grammar("TEXT = /abc/\ngroup errors:\n  FOO = /x/\n")
        assert.is_not_nil(err)
        assert.truthy(err:find("Reserved group name"))
    end)

    it("rejects duplicate group name", function()
        local source = "TEXT = /abc/\ngroup tag:\n  FOO = /x/\ngroup tag:\n  BAR = /y/\n"
        local _, err = grammar_tools.parse_token_grammar(source)
        assert.is_not_nil(err)
        assert.truthy(err:find("Duplicate group name"))
    end)

    it("rejects bad definition in group", function()
        local source = "TEXT = /abc/\ngroup tag:\n  not a definition\n"
        local _, err = grammar_tools.parse_token_grammar(source)
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected token definition"))
    end)

    it("rejects incomplete definition in group", function()
        local source = "TEXT = /abc/\ngroup tag:\n  FOO = \n"
        local _, err = grammar_tools.parse_token_grammar(source)
        assert.is_not_nil(err)
        assert.truthy(err:find("Incomplete definition"))
    end)
end)

-- ============================================================================
-- TokenGrammar: token_names and effective_token_names
-- ============================================================================

describe("TokenGrammar:token_names", function()
    it("includes both original name and alias", function()
        local source = 'STRING_DQ = /"[^"]*"/ -> STRING'
        local grammar, _ = grammar_tools.parse_token_grammar(source)
        local names = grammar:token_names()
        assert.is_true(names["STRING_DQ"])
        assert.is_true(names["STRING"])
    end)

    it("includes names from groups", function()
        local source = 'TEXT = /[^<]+/\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n  ATTR_DQ = /"[^"]*"/ -> ATTR_VALUE\n'
        local grammar, _ = grammar_tools.parse_token_grammar(source)
        local names = grammar:token_names()
        assert.is_true(names["TEXT"])
        assert.is_true(names["TAG_NAME"])
        assert.is_true(names["ATTR_DQ"])
        assert.is_true(names["ATTR_VALUE"])
    end)

    it("does not include error definition names", function()
        local source = 'NAME = /[a-z]+/\nerrors:\n  BAD_STRING = /"[^"\\n]*/\n'
        local grammar, _ = grammar_tools.parse_token_grammar(source)
        local names = grammar:token_names()
        assert.is_nil(names["BAD_STRING"])
    end)
end)

describe("TokenGrammar:effective_token_names", function()
    it("returns alias instead of name when alias exists", function()
        local source = 'TEXT = /[^<]+/\n\ngroup tag:\n  ATTR_DQ = /"[^"]*"/ -> ATTR_VALUE\n'
        local grammar, _ = grammar_tools.parse_token_grammar(source)
        local names = grammar:effective_token_names()
        assert.is_true(names["TEXT"])
        assert.is_true(names["ATTR_VALUE"])
        assert.is_nil(names["ATTR_DQ"])
    end)
end)

-- ============================================================================
-- validate_token_grammar
-- ============================================================================

describe("validate_token_grammar", function()
    it("reports no issues for a clean grammar", function()
        local source = "NAME = /[a-z]+/\nNUMBER = /[0-9]+/"
        local grammar, _ = grammar_tools.parse_token_grammar(source)
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.are.equal(0, #issues)
    end)

    it("reports duplicate token names", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.definitions = {
            grammar_tools.TokenDefinition.new({ name = "FOO", pattern = "x", is_regex = true, line_number = 1 }),
            grammar_tools.TokenDefinition.new({ name = "FOO", pattern = "y", is_regex = true, line_number = 2 }),
        }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "Duplicate"))
    end)

    it("reports empty pattern", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.definitions = {
            grammar_tools.TokenDefinition.new({ name = "FOO", pattern = "", is_regex = true, line_number = 1 }),
        }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "Empty pattern"))
    end)

    it("reports non-UPPER_CASE token names", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.definitions = {
            grammar_tools.TokenDefinition.new({ name = "foo", pattern = "x", is_regex = true, line_number = 1 }),
        }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "should be UPPER_CASE"))
    end)

    it("reports non-UPPER_CASE alias", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.definitions = {
            grammar_tools.TokenDefinition.new({ name = "FOO", pattern = "x", is_regex = true, line_number = 1, alias = "bar" }),
        }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "Alias 'bar'"))
    end)

    it("reports unknown lexer mode", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.mode = "unknown"
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "Unknown lexer mode"))
    end)

    it("accepts valid mode 'indentation'", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.mode = "indentation"
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_false(has_issue(issues, "Unknown lexer mode"))
    end)

    it("accepts valid mode 'layout'", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.mode = "layout"
        grammar.layout_keywords = { "let" }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_false(has_issue(issues, "Unknown lexer mode"))
    end)

    it("requires layout keywords in layout mode", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.mode = "layout"
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "layout_keywords"))
    end)

    it("reports unknown escape mode", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.escape_mode = "backslash"
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "Unknown escape mode"))
    end)

    it("accepts valid escape mode 'none'", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.escape_mode = "none"
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_false(has_issue(issues, "Unknown escape mode"))
    end)

    it("reports empty pattern group", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.groups["empty"] = grammar_tools.PatternGroup.new("empty")
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "Empty pattern group"))
    end)

    it("validates definitions within groups", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.groups["tag"] = grammar_tools.PatternGroup.new("tag", {
            grammar_tools.TokenDefinition.new({ name = "foo", pattern = "x", is_regex = true, line_number = 5 }),
        })
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "should be UPPER_CASE"))
    end)

    it("validates skip definitions", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.skip_definitions = {
            grammar_tools.TokenDefinition.new({ name = "ws", pattern = "x", is_regex = true, line_number = 1 }),
        }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "should be UPPER_CASE"))
    end)

    it("validates error definitions", function()
        local grammar = grammar_tools.TokenGrammar.new()
        grammar.error_definitions = {
            grammar_tools.TokenDefinition.new({ name = "bad", pattern = "x", is_regex = true, line_number = 1 }),
        }
        local issues = grammar_tools.validate_token_grammar(grammar)
        assert.is_true(has_issue(issues, "should be UPPER_CASE"))
    end)
end)

-- ============================================================================
-- parse_parser_grammar — happy paths
-- ============================================================================

describe("parse_parser_grammar", function()
    it("parses a basic grammar with sequence and repetition", function()
        local source = "expression = term { ( PLUS | MINUS ) term } ;\nterm = NUMBER ;\n"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, #grammar.rules)
        assert.are.equal("expression", grammar.rules[1].name)
        assert.are.equal("term", grammar.rules[2].name)
    end)

    it("parses alternation", function()
        local source = "expr = NUMBER | NAME ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal("alternation", grammar.rules[1].body.type)
        assert.are.equal(2, #grammar.rules[1].body.choices)
    end)

    it("parses optional", function()
        local source = "expr = NUMBER [ PLUS NUMBER ] ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal("sequence", grammar.rules[1].body.type)
        assert.are.equal("optional", grammar.rules[1].body.elements[2].type)
    end)

    it("parses repetition", function()
        local source = "list = { NUMBER } ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal("repetition", grammar.rules[1].body.type)
    end)

    it("parses group (parentheses)", function()
        local source = "expr = ( NUMBER ) ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal("group", grammar.rules[1].body.type)
    end)

    it("parses literal strings", function()
        local source = 'expr = NUMBER "+" NUMBER ;'
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal("sequence", grammar.rules[1].body.type)
        assert.are.equal("literal", grammar.rules[1].body.elements[2].type)
        assert.are.equal("+", grammar.rules[1].body.elements[2].value)
    end)

    it("distinguishes rule references from token references", function()
        local source = "program = expr ; expr = NUMBER ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        local ref = grammar.rules[1].body
        assert.are.equal("rule_reference", ref.type)
        assert.is_false(ref.is_token)
        assert.are.equal("expr", ref.name)

        local tok_ref = grammar.rules[2].body
        assert.are.equal("rule_reference", tok_ref.type)
        assert.is_true(tok_ref.is_token)
        assert.are.equal("NUMBER", tok_ref.name)
    end)

    it("parses multiple rules", function()
        local source = "a = NUMBER ; b = NAME ; c = STRING ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal(3, #grammar.rules)
    end)

    it("handles comments", function()
        local source = "# comment\nexpr = NUMBER ; # inline comment"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal(1, #grammar.rules)
    end)

    it("parses nested alternation inside group", function()
        local source = "expr = ( NUMBER | NAME | STRING ) ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal("group", grammar.rules[1].body.type)
        assert.are.equal("alternation", grammar.rules[1].body.element.type)
        assert.are.equal(3, #grammar.rules[1].body.element.choices)
    end)

    it("tracks line numbers", function()
        local source = "\nexpr = NUMBER ;\nterm = NAME ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal(2, grammar.rules[1].line_number)
        assert.are.equal(3, grammar.rules[2].line_number)
    end)

    it("parses complex expression grammar", function()
        local source = [[
program = { statement } ;
statement = assignment | expr_stmt ;
assignment = NAME EQUALS expression NEWLINE ;
expr_stmt = expression NEWLINE ;
expression = term { ( PLUS | MINUS ) term } ;
term = factor { ( STAR | SLASH ) factor } ;
factor = NUMBER | NAME | "(" expression ")" ;
]]
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        assert.are.equal(7, #grammar.rules)
    end)

    it("returns single element (not sequence) for single-element body", function()
        local source = "expr = NUMBER ;"
        local grammar, err = grammar_tools.parse_parser_grammar(source)
        assert.is_nil(err)
        -- Should be a RuleReference directly, not wrapped in a Sequence
        assert.are.equal("rule_reference", grammar.rules[1].body.type)
    end)
end)

-- ============================================================================
-- parse_parser_grammar — error paths
-- ============================================================================

describe("parse_parser_grammar errors", function()
    it("rejects unterminated string literal", function()
        local _, err = grammar_tools.parse_parser_grammar('expr = "unclosed ;')
        assert.is_not_nil(err)
        assert.truthy(err:find("Unterminated string"))
    end)

    it("rejects unexpected character", function()
        local _, err = grammar_tools.parse_parser_grammar("expr = @ ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Unexpected character"))
    end)

    it("rejects missing semicolon", function()
        local _, err = grammar_tools.parse_parser_grammar("expr = NUMBER")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected SEMI"))
    end)

    it("rejects missing equals", function()
        local _, err = grammar_tools.parse_parser_grammar("expr NUMBER ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected EQUALS"))
    end)

    it("rejects empty sequence before pipe", function()
        local _, err = grammar_tools.parse_parser_grammar("expr = | NUMBER ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected at least one element"))
    end)

    it("rejects unclosed brace", function()
        local _, err = grammar_tools.parse_parser_grammar("expr = { NUMBER ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected RBRACE"))
    end)

    it("rejects unclosed bracket", function()
        local _, err = grammar_tools.parse_parser_grammar("expr = [ NUMBER ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected RBRACKET"))
    end)

    it("rejects unclosed paren", function()
        local _, err = grammar_tools.parse_parser_grammar("expr = ( NUMBER ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected RPAREN"))
    end)

    it("rejects unexpected token at start of rule", function()
        local _, err = grammar_tools.parse_parser_grammar("; = NUMBER ;")
        assert.is_not_nil(err)
        assert.truthy(err:find("Expected IDENT"))
    end)
end)

-- ============================================================================
-- ParserGrammar: rule_names, rule_references, token_references
-- ============================================================================

describe("ParserGrammar:rule_names", function()
    it("returns all defined rule names", function()
        local source = "program = statement ;\nstatement = NAME ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local names = grammar:rule_names()
        assert.is_true(names["program"])
        assert.is_true(names["statement"])
    end)
end)

describe("ParserGrammar:rule_references", function()
    it("returns lowercase rule references from bodies", function()
        local source = "program = statement ;\nstatement = NAME ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:rule_references()
        assert.is_true(refs["statement"])
        assert.is_nil(refs["program"]) -- defined, not referenced in bodies
    end)

    it("collects references from nested elements", function()
        local source = "program = { statement } ;\nstatement = expr ;\nexpr = NUMBER ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:rule_references()
        assert.is_true(refs["statement"])
        assert.is_true(refs["expr"])
    end)
end)

describe("ParserGrammar:token_references", function()
    it("returns UPPERCASE token references from bodies", function()
        local source = "expr = NUMBER PLUS NAME ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:token_references()
        assert.is_true(refs["NUMBER"])
        assert.is_true(refs["PLUS"])
        assert.is_true(refs["NAME"])
    end)

    it("collects token references from alternation", function()
        local source = "expr = NUMBER | STRING ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:token_references()
        assert.is_true(refs["NUMBER"])
        assert.is_true(refs["STRING"])
    end)

    it("collects token references from optional", function()
        local source = "expr = [ NUMBER ] ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:token_references()
        assert.is_true(refs["NUMBER"])
    end)

    it("collects token references from group", function()
        local source = "expr = ( NUMBER ) ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:token_references()
        assert.is_true(refs["NUMBER"])
    end)

    it("collects token references from repetition", function()
        local source = "expr = { NUMBER } ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local refs = grammar:token_references()
        assert.is_true(refs["NUMBER"])
    end)
end)

-- ============================================================================
-- validate_parser_grammar
-- ============================================================================

describe("validate_parser_grammar", function()
    it("reports no issues for a clean grammar", function()
        local source = "program = { statement } ;\nstatement = NAME NUMBER ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local token_names = { NAME = true, NUMBER = true }
        local issues = grammar_tools.validate_parser_grammar(grammar, token_names)
        assert.are.equal(0, #issues)
    end)

    it("reports duplicate rule names", function()
        local source = "expr = NUMBER ;\nexpr = NAME ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.is_true(has_issue(issues, "Duplicate rule name"))
    end)

    it("reports non-lowercase rule names", function()
        local source = "Program = NUMBER ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.is_true(has_issue(issues, "should be lowercase"))
    end)

    it("reports undefined rule references", function()
        local source = "program = statement ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.is_true(has_issue(issues, "Undefined rule reference"))
        assert.is_true(has_issue(issues, "statement"))
    end)

    it("reports undefined token references", function()
        local source = "expr = SEMICOLON ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local token_names = { NUMBER = true, NAME = true }
        local issues = grammar_tools.validate_parser_grammar(grammar, token_names)
        assert.is_true(has_issue(issues, "Undefined token reference"))
        assert.is_true(has_issue(issues, "SEMICOLON"))
    end)

    it("does not report undefined tokens when token_names is nil", function()
        local source = "expr = SEMICOLON ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.is_false(has_issue(issues, "Undefined token reference"))
    end)

    it("allows synthetic tokens (NEWLINE, INDENT, DEDENT, EOF)", function()
        local source = "stmt = NAME NEWLINE ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local token_names = { NAME = true }
        local issues = grammar_tools.validate_parser_grammar(grammar, token_names)
        assert.is_false(has_issue(issues, "NEWLINE"))
    end)

    it("reports unreachable rules (not start rule)", function()
        local source = "program = NUMBER ;\northan = NAME ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.is_true(has_issue(issues, "unreachable"))
        assert.is_true(has_issue(issues, "orthan"))
    end)

    it("does not flag start rule as unreachable", function()
        local source = "program = NUMBER ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.is_false(has_issue(issues, "unreachable"))
    end)

    it("handles empty grammar", function()
        local source = ""
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local issues = grammar_tools.validate_parser_grammar(grammar, nil)
        assert.are.equal(0, #issues)
    end)
end)

-- ============================================================================
-- cross_validate
-- ============================================================================

describe("cross_validate", function()
    it("reports no issues for a clean pair", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\nNUMBER = /[0-9]+/\n")
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("expr = NAME NUMBER ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        assert.are.equal(0, #issues)
    end)

    it("reports missing token reference", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\n")
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("expr = NAME SEMICOLON ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        assert.is_true(has_issue(issues, "Error:"))
        assert.is_true(has_issue(issues, "SEMICOLON"))
    end)

    it("reports unused token as warning", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\nUNUSED = /[0-9]+/\n")
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("expr = NAME ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        assert.is_true(has_issue(issues, "Warning:"))
        assert.is_true(has_issue(issues, "UNUSED"))
    end)

    it("treats synthetic tokens (NEWLINE, EOF) as valid", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar("NAME = /[a-z]+/\n")
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("stmt = NAME NEWLINE ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        assert.is_false(has_issue(issues, "NEWLINE"))
    end)

    it("adds INDENT/DEDENT when mode is indentation", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar("mode: indentation\nNAME = /[a-z]+/\n")
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("block = INDENT NAME DEDENT ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        assert.is_false(has_issue(issues, "INDENT"))
        assert.is_false(has_issue(issues, "DEDENT"))
    end)

    it("considers alias as used", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar('STRING_DQ = /"[^"]*"/ -> STRING\n')
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("expr = STRING ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        -- STRING_DQ should not be reported as unused because its alias STRING is used
        assert.is_false(has_issue(issues, "Warning:"))
    end)

    it("reports token without alias as unused when alias is used", function()
        local token_grammar, _ = grammar_tools.parse_token_grammar('STRING_DQ = /"[^"]*"/ -> STRING\nUNUSED = /./\n')
        local parser_grammar, _ = grammar_tools.parse_parser_grammar("expr = STRING ;\n")
        local issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
        assert.is_true(has_issue(issues, "UNUSED"))
    end)
end)

-- ============================================================================
-- Element constructors
-- ============================================================================

describe("element constructors", function()
    it("creates rule_reference", function()
        local elem = grammar_tools.make_rule_reference("expr", false)
        assert.are.equal("rule_reference", elem.type)
        assert.are.equal("expr", elem.name)
        assert.is_false(elem.is_token)
    end)

    it("creates literal", function()
        local elem = grammar_tools.make_literal("+")
        assert.are.equal("literal", elem.type)
        assert.are.equal("+", elem.value)
    end)

    it("creates sequence", function()
        local elem = grammar_tools.make_sequence({})
        assert.are.equal("sequence", elem.type)
    end)

    it("creates alternation", function()
        local elem = grammar_tools.make_alternation({})
        assert.are.equal("alternation", elem.type)
    end)

    it("creates repetition", function()
        local inner = grammar_tools.make_literal("x")
        local elem = grammar_tools.make_repetition(inner)
        assert.are.equal("repetition", elem.type)
        assert.are.equal("literal", elem.element.type)
    end)

    it("creates optional", function()
        local inner = grammar_tools.make_literal("x")
        local elem = grammar_tools.make_optional(inner)
        assert.are.equal("optional", elem.type)
    end)

    it("creates group", function()
        local inner = grammar_tools.make_literal("x")
        local elem = grammar_tools.make_group(inner)
        assert.are.equal("group", elem.type)
    end)
end)

-- ============================================================================
-- GrammarRule
-- ============================================================================

describe("GrammarRule", function()
    it("creates a rule with name, body, and line number", function()
        local body = grammar_tools.make_literal("x")
        local rule = grammar_tools.GrammarRule.new("expr", body, 42)
        assert.are.equal("expr", rule.name)
        assert.are.equal("literal", rule.body.type)
        assert.are.equal(42, rule.line_number)
    end)
end)

-- ============================================================================
-- ParserGrammar construction
-- ============================================================================

describe("ParserGrammar", function()
    it("creates an empty grammar", function()
        local g = grammar_tools.ParserGrammar.new()
        assert.are.same({}, g.rules)
    end)

    it("creates a grammar with rules", function()
        local body = grammar_tools.make_literal("x")
        local rule = grammar_tools.GrammarRule.new("expr", body, 1)
        local g = grammar_tools.ParserGrammar.new({ rule })
        assert.are.equal(1, #g.rules)
    end)
end)

-- ============================================================================
-- Edge cases: collect_rule_refs and collect_token_refs with nil
-- ============================================================================

describe("reference collection edge cases", function()
    it("handles literal elements (no references)", function()
        local source = 'expr = "+" ;'
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local rule_refs = grammar:rule_references()
        local token_refs = grammar:token_references()
        -- Literal has no references
        local rule_count = 0
        for _ in pairs(rule_refs) do rule_count = rule_count + 1 end
        local token_count = 0
        for _ in pairs(token_refs) do token_count = token_count + 1 end
        assert.are.equal(0, rule_count)
        assert.are.equal(0, token_count)
    end)

    it("handles deeply nested structures", function()
        local source = "expr = { [ ( NUMBER | name ) ] } ;\nname = NAME ;\n"
        local grammar, _ = grammar_tools.parse_parser_grammar(source)
        local rule_refs = grammar:rule_references()
        local token_refs = grammar:token_references()
        assert.is_true(rule_refs["name"])
        assert.is_true(token_refs["NUMBER"])
        assert.is_true(token_refs["NAME"])
    end)
end)
