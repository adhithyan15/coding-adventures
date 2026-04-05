-- Tests for ecmascript_es1_lexer
-- ==============================
--
-- Comprehensive busted test suite for the ECMAScript 1 (1997) lexer package.
--
-- ES1 is the first standardized version of JavaScript. It has 23 keywords,
-- basic operators (no === or !==), string/number literals, and no regex
-- literals. This test suite validates the lexer against all ES1 features.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Keywords: var, function, return, if, else, for, while, do, break,
--               case, continue, default, delete, in, new, switch, this,
--               typeof, void, with, true, false, null
--   - Identifiers (NAME tokens for non-keywords)
--   - Numbers: integer, hex, float, scientific notation
--   - Strings: single-quoted and double-quoted
--   - Operators: +, -, *, /, %, ==, !=, <, >, <=, >=, =, +=, -=, *=, /=,
--               %=, &&, ||, !, ++, --, <<, >>, >>>, &, |, ^, ~, ?
--   - ES1 does NOT have === or !== (those are ES3)
--   - Delimiters: (, ), {, }, [, ], ;, ,, :, .
--   - Whitespace and comments are consumed silently
--   - Token positions (line, col) are tracked correctly
--   - Unexpected character raises an error

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    package.path
)

local es1_lexer = require("coding_adventures.ecmascript_es1_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by es1_lexer.tokenize.
-- @return table         Ordered list of type strings (no EOF entry).
local function types(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Collect token values from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by es1_lexer.tokenize.
-- @return table         Ordered list of value strings (no EOF entry).
local function values(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.value
        end
    end
    return out
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("ecmascript_es1_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(es1_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(es1_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", es1_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(es1_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(es1_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = es1_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = es1_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = es1_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("line comment is consumed silently", function()
        local tokens = es1_lexer.tokenize("// this is a comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("block comment is consumed silently", function()
        local tokens = es1_lexer.tokenize("/* block */")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords
-- =========================================================================

describe("ES1 keyword tokens", function()
    -- Variable declaration
    it("tokenizes var", function()
        local tokens = es1_lexer.tokenize("var")
        assert.are.equal("VAR", tokens[1].type)
        assert.are.equal("var", tokens[1].value)
    end)

    -- Control flow
    it("tokenizes if", function()
        local tokens = es1_lexer.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
    end)

    it("tokenizes else", function()
        local tokens = es1_lexer.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
    end)

    it("tokenizes for", function()
        local tokens = es1_lexer.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
    end)

    it("tokenizes while", function()
        local tokens = es1_lexer.tokenize("while")
        assert.are.equal("WHILE", tokens[1].type)
    end)

    it("tokenizes do", function()
        local tokens = es1_lexer.tokenize("do")
        assert.are.equal("DO", tokens[1].type)
    end)

    it("tokenizes break", function()
        local tokens = es1_lexer.tokenize("break")
        assert.are.equal("BREAK", tokens[1].type)
    end)

    it("tokenizes continue", function()
        local tokens = es1_lexer.tokenize("continue")
        assert.are.equal("CONTINUE", tokens[1].type)
    end)

    it("tokenizes switch", function()
        local tokens = es1_lexer.tokenize("switch")
        assert.are.equal("SWITCH", tokens[1].type)
    end)

    it("tokenizes case", function()
        local tokens = es1_lexer.tokenize("case")
        assert.are.equal("CASE", tokens[1].type)
    end)

    it("tokenizes default", function()
        local tokens = es1_lexer.tokenize("default")
        assert.are.equal("DEFAULT", tokens[1].type)
    end)

    it("tokenizes return", function()
        local tokens = es1_lexer.tokenize("return")
        assert.are.equal("RETURN", tokens[1].type)
    end)

    -- Function
    it("tokenizes function", function()
        local tokens = es1_lexer.tokenize("function")
        assert.are.equal("FUNCTION", tokens[1].type)
    end)

    -- Operators as keywords
    it("tokenizes new", function()
        local tokens = es1_lexer.tokenize("new")
        assert.are.equal("NEW", tokens[1].type)
    end)

    it("tokenizes delete", function()
        local tokens = es1_lexer.tokenize("delete")
        assert.are.equal("DELETE", tokens[1].type)
    end)

    it("tokenizes typeof", function()
        local tokens = es1_lexer.tokenize("typeof")
        assert.are.equal("TYPEOF", tokens[1].type)
    end)

    it("tokenizes void", function()
        local tokens = es1_lexer.tokenize("void")
        assert.are.equal("VOID", tokens[1].type)
    end)

    it("tokenizes in", function()
        local tokens = es1_lexer.tokenize("in")
        assert.are.equal("IN", tokens[1].type)
    end)

    it("tokenizes this", function()
        local tokens = es1_lexer.tokenize("this")
        assert.are.equal("THIS", tokens[1].type)
    end)

    it("tokenizes with", function()
        local tokens = es1_lexer.tokenize("with")
        assert.are.equal("WITH", tokens[1].type)
    end)

    -- Literals as keywords
    it("tokenizes true", function()
        local tokens = es1_lexer.tokenize("true")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)
    end)

    it("tokenizes false", function()
        local tokens = es1_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)

    it("tokenizes null", function()
        local tokens = es1_lexer.tokenize("null")
        assert.are.equal("NULL", tokens[1].type)
        assert.are.equal("null", tokens[1].value)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifiers", function()
    it("simple identifier", function()
        local tokens = es1_lexer.tokenize("myVar")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVar", tokens[1].value)
    end)

    it("identifier with dollar sign prefix", function()
        local tokens = es1_lexer.tokenize("$el")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("$el", tokens[1].value)
    end)

    it("identifier with underscore prefix", function()
        local tokens = es1_lexer.tokenize("_priv")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_priv", tokens[1].value)
    end)

    it("single-letter identifier", function()
        local tokens = es1_lexer.tokenize("x")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("x", tokens[1].value)
    end)
end)

-- =========================================================================
-- Numbers
-- =========================================================================

describe("number tokens", function()
    it("integer", function()
        local tokens = es1_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("zero", function()
        local tokens = es1_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("hex number", function()
        local tokens = es1_lexer.tokenize("0xFF")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0xFF", tokens[1].value)
    end)

    it("float number", function()
        local tokens = es1_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("leading dot float", function()
        local tokens = es1_lexer.tokenize(".5")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal(".5", tokens[1].value)
    end)

    it("scientific notation", function()
        local tokens = es1_lexer.tokenize("1e10")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("1e10", tokens[1].value)
    end)
end)

-- =========================================================================
-- Strings
-- =========================================================================

describe("string tokens", function()
    it("double-quoted string", function()
        local tokens = es1_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        -- GrammarLexer strips surrounding quotes from string values
        assert.are.equal("hello", tokens[1].value)
    end)

    it("single-quoted string", function()
        local tokens = es1_lexer.tokenize("'world'")
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal("world", tokens[1].value)
    end)

    it("empty double-quoted string", function()
        local tokens = es1_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal("", tokens[1].value)
    end)

    it("string with escape sequence", function()
        local tokens = es1_lexer.tokenize('"a\\nb"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Operators — multi-character
-- =========================================================================

describe("multi-character operators", function()
    -- ES1 has == and != but NOT === and !==
    it("equals equals ==", function()
        local tokens = es1_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
        assert.are.equal("==", tokens[1].value)
    end)

    it("not equals !=", function()
        local tokens = es1_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
        assert.are.equal("!=", tokens[1].value)
    end)

    it("less than or equal <=", function()
        local tokens = es1_lexer.tokenize("<=")
        assert.are.equal("LESS_EQUALS", tokens[1].type)
    end)

    it("greater than or equal >=", function()
        local tokens = es1_lexer.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", tokens[1].type)
    end)

    it("logical AND &&", function()
        local tokens = es1_lexer.tokenize("&&")
        assert.are.equal("AND_AND", tokens[1].type)
    end)

    it("logical OR ||", function()
        local tokens = es1_lexer.tokenize("||")
        assert.are.equal("OR_OR", tokens[1].type)
    end)

    it("increment ++", function()
        local tokens = es1_lexer.tokenize("++")
        assert.are.equal("PLUS_PLUS", tokens[1].type)
    end)

    it("decrement --", function()
        local tokens = es1_lexer.tokenize("--")
        assert.are.equal("MINUS_MINUS", tokens[1].type)
    end)

    it("left shift <<", function()
        local tokens = es1_lexer.tokenize("<<")
        assert.are.equal("LEFT_SHIFT", tokens[1].type)
    end)

    it("right shift >>", function()
        local tokens = es1_lexer.tokenize(">>")
        assert.are.equal("RIGHT_SHIFT", tokens[1].type)
    end)

    it("unsigned right shift >>>", function()
        local tokens = es1_lexer.tokenize(">>>")
        assert.are.equal("UNSIGNED_RIGHT_SHIFT", tokens[1].type)
    end)

    it("plus equals +=", function()
        local tokens = es1_lexer.tokenize("+=")
        assert.are.equal("PLUS_EQUALS", tokens[1].type)
    end)

    it("minus equals -=", function()
        local tokens = es1_lexer.tokenize("-=")
        assert.are.equal("MINUS_EQUALS", tokens[1].type)
    end)

    it("star equals *=", function()
        local tokens = es1_lexer.tokenize("*=")
        assert.are.equal("STAR_EQUALS", tokens[1].type)
    end)

    it("slash equals /=", function()
        local tokens = es1_lexer.tokenize("/=")
        assert.are.equal("SLASH_EQUALS", tokens[1].type)
    end)

    it("percent equals %=", function()
        local tokens = es1_lexer.tokenize("%=")
        assert.are.equal("PERCENT_EQUALS", tokens[1].type)
    end)
end)

-- =========================================================================
-- Operators — single-character
-- =========================================================================

describe("single-character operators", function()
    it("assignment =", function()
        local tokens = es1_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
    end)

    it("plus +", function()
        local tokens = es1_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
    end)

    it("minus -", function()
        local tokens = es1_lexer.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
    end)

    it("star *", function()
        local tokens = es1_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
    end)

    it("slash /", function()
        local tokens = es1_lexer.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
    end)

    it("percent %", function()
        local tokens = es1_lexer.tokenize("%")
        assert.are.equal("PERCENT", tokens[1].type)
    end)

    it("less than <", function()
        local tokens = es1_lexer.tokenize("<")
        assert.are.equal("LESS_THAN", tokens[1].type)
    end)

    it("greater than >", function()
        local tokens = es1_lexer.tokenize(">")
        assert.are.equal("GREATER_THAN", tokens[1].type)
    end)

    it("bang !", function()
        local tokens = es1_lexer.tokenize("!")
        assert.are.equal("BANG", tokens[1].type)
    end)

    it("ampersand &", function()
        local tokens = es1_lexer.tokenize("&")
        assert.are.equal("AMPERSAND", tokens[1].type)
    end)

    it("pipe |", function()
        local tokens = es1_lexer.tokenize("|")
        assert.are.equal("PIPE", tokens[1].type)
    end)

    it("caret ^", function()
        local tokens = es1_lexer.tokenize("^")
        assert.are.equal("CARET", tokens[1].type)
    end)

    it("tilde ~", function()
        local tokens = es1_lexer.tokenize("~")
        assert.are.equal("TILDE", tokens[1].type)
    end)

    it("question ?", function()
        local tokens = es1_lexer.tokenize("?")
        assert.are.equal("QUESTION", tokens[1].type)
    end)
end)

-- =========================================================================
-- Delimiters
-- =========================================================================

describe("delimiter tokens", function()
    it("parentheses", function()
        assert.are.same({"LPAREN", "RPAREN"}, types(es1_lexer.tokenize("()")))
    end)

    it("braces", function()
        assert.are.same({"LBRACE", "RBRACE"}, types(es1_lexer.tokenize("{}")))
    end)

    it("brackets", function()
        assert.are.same({"LBRACKET", "RBRACKET"}, types(es1_lexer.tokenize("[]")))
    end)

    it("semicolon", function()
        local tokens = es1_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)

    it("comma", function()
        local tokens = es1_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
    end)

    it("colon", function()
        local tokens = es1_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
    end)

    it("dot", function()
        local tokens = es1_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("variable declaration: var x = 1;", function()
        assert.are.same(
            {"VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"},
            types(es1_lexer.tokenize("var x = 1;"))
        )
    end)

    it("function declaration", function()
        assert.are.same(
            {"FUNCTION", "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN",
             "LBRACE", "RETURN", "NAME", "PLUS", "NAME", "SEMICOLON", "RBRACE"},
            types(es1_lexer.tokenize("function add(a, b) { return a + b; }"))
        )
    end)

    it("typeof expression", function()
        assert.are.same(
            {"TYPEOF", "NAME"},
            types(es1_lexer.tokenize("typeof x"))
        )
    end)

    it("new expression", function()
        assert.are.same(
            {"NEW", "NAME", "LPAREN", "RPAREN"},
            types(es1_lexer.tokenize("new Foo()"))
        )
    end)

    it("method call: obj.method(arg)", function()
        assert.are.same(
            {"NAME", "DOT", "NAME", "LPAREN", "NAME", "RPAREN"},
            types(es1_lexer.tokenize("obj.method(arg)"))
        )
    end)

    it("if/else with true/false", function()
        local tokens = es1_lexer.tokenize("if (x) { return true; } else { return false; }")
        local t = types(tokens)
        assert.is_true(#t > 0)
        assert.are.equal("IF", t[1])
    end)

    it("while loop", function()
        assert.are.same(
            {"WHILE", "LPAREN", "NAME", "RPAREN", "LBRACE", "RBRACE"},
            types(es1_lexer.tokenize("while (x) {}"))
        )
    end)

    it("for loop", function()
        assert.are.same(
            {"FOR", "LPAREN", "VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON",
             "NAME", "LESS_THAN", "NUMBER", "SEMICOLON", "NAME", "PLUS_PLUS",
             "RPAREN", "LBRACE", "RBRACE"},
            types(es1_lexer.tokenize("for (var i = 0; i < 10; i++) {}"))
        )
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("spaces between tokens are consumed silently", function()
        assert.are.same(
            {"VAR", "NAME", "EQUALS", "NUMBER"},
            types(es1_lexer.tokenize("var x = 1"))
        )
    end)

    it("tabs and newlines consumed silently", function()
        assert.are.same(
            {"VAR", "NAME", "EQUALS", "NUMBER"},
            types(es1_lexer.tokenize("var\n\tx\n=\n1"))
        )
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("column tracking: var x = 1;", function()
        -- v a r _ x _ = _ 1 ;
        -- 1 . . 4 5 6 7 8 9 10
        local tokens = es1_lexer.tokenize("var x = 1;")
        assert.are.equal(1, tokens[1].col)   -- var
        assert.are.equal(5, tokens[2].col)   -- x
        assert.are.equal(7, tokens[3].col)   -- =
        assert.are.equal(9, tokens[4].col)   -- 1
        assert.are.equal(10, tokens[5].col)  -- ;
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = es1_lexer.tokenize("var x = 1;")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("EOF is always last", function()
        local tokens = es1_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("unexpected character # raises an error", function()
        assert.has_error(function()
            es1_lexer.tokenize("#")
        end)
    end)

    it("backtick raises an error (template literals not in ES1)", function()
        assert.has_error(function()
            es1_lexer.tokenize("`hello`")
        end)
    end)
end)
