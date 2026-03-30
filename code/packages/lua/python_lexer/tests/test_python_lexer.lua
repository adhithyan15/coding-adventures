-- Tests for python_lexer
-- ======================
--
-- Comprehensive busted test suite for the Python lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Keywords: def, class, if, elif, else, for, while, return,
--               import, from, as, True, False, None
--   - Identifiers (NAME tokens for non-keywords)
--   - Numbers: integer literals
--   - Strings: double-quoted literals
--   - Operators: ==, =, +, -, *, /
--   - Punctuation: (, ), ,, :
--   - Whitespace is consumed silently
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

local py_lexer = require("coding_adventures.python_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by py_lexer.tokenize.
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
-- @param tokens  table  The token list returned by py_lexer.tokenize.
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

--- Find the first token with the given type.
-- @param tokens  table   Token list.
-- @param typ     string  Token type to search for.
-- @return table|nil      The first matching token, or nil.
local function first_of(tokens, typ)
    for _, tok in ipairs(tokens) do
        if tok.type == typ then return tok end
    end
    return nil
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("python_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(py_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(py_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", py_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(py_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(py_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = py_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = py_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = py_lexer.tokenize("   \t  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords
-- =========================================================================

describe("keyword tokens", function()
    -- Python control flow keywords
    it("tokenizes if", function()
        local tokens = py_lexer.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
        assert.are.equal("if", tokens[1].value)
    end)

    it("tokenizes elif", function()
        local tokens = py_lexer.tokenize("elif")
        assert.are.equal("ELIF", tokens[1].type)
        assert.are.equal("elif", tokens[1].value)
    end)

    it("tokenizes else", function()
        local tokens = py_lexer.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
        assert.are.equal("else", tokens[1].value)
    end)

    it("tokenizes while", function()
        local tokens = py_lexer.tokenize("while")
        assert.are.equal("WHILE", tokens[1].type)
        assert.are.equal("while", tokens[1].value)
    end)

    it("tokenizes for", function()
        local tokens = py_lexer.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
        assert.are.equal("for", tokens[1].value)
    end)

    -- Python function/class keywords
    it("tokenizes def", function()
        local tokens = py_lexer.tokenize("def")
        assert.are.equal("DEF", tokens[1].type)
        assert.are.equal("def", tokens[1].value)
    end)

    it("tokenizes return", function()
        local tokens = py_lexer.tokenize("return")
        assert.are.equal("RETURN", tokens[1].type)
        assert.are.equal("return", tokens[1].value)
    end)

    it("tokenizes class", function()
        local tokens = py_lexer.tokenize("class")
        assert.are.equal("CLASS", tokens[1].type)
        assert.are.equal("class", tokens[1].value)
    end)

    -- Import-related keywords
    it("tokenizes import", function()
        local tokens = py_lexer.tokenize("import")
        assert.are.equal("IMPORT", tokens[1].type)
        assert.are.equal("import", tokens[1].value)
    end)

    it("tokenizes from", function()
        local tokens = py_lexer.tokenize("from")
        assert.are.equal("FROM", tokens[1].type)
        assert.are.equal("from", tokens[1].value)
    end)

    it("tokenizes as", function()
        local tokens = py_lexer.tokenize("as")
        assert.are.equal("AS", tokens[1].type)
        assert.are.equal("as", tokens[1].value)
    end)

    -- Python singleton literals (keywords in Python)
    it("tokenizes True", function()
        local tokens = py_lexer.tokenize("True")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("True", tokens[1].value)
    end)

    it("tokenizes False", function()
        local tokens = py_lexer.tokenize("False")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("False", tokens[1].value)
    end)

    it("tokenizes None", function()
        local tokens = py_lexer.tokenize("None")
        assert.are.equal("NONE", tokens[1].type)
        assert.are.equal("None", tokens[1].value)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = py_lexer.tokenize("my_var")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("my_var", tokens[1].value)
    end)

    it("tokenizes an identifier starting with underscore", function()
        local tokens = py_lexer.tokenize("_private")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_private", tokens[1].value)
    end)

    it("tokenizes an identifier with digits in the middle", function()
        local tokens = py_lexer.tokenize("abc123")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("abc123", tokens[1].value)
    end)

    it("tokenizes a dunder identifier", function()
        local tokens = py_lexer.tokenize("__init__")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("__init__", tokens[1].value)
    end)
end)

-- =========================================================================
-- Number tokens
-- =========================================================================

describe("number tokens", function()
    it("tokenizes an integer", function()
        local tokens = py_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = py_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by operators", function()
        local tokens = py_lexer.tokenize("1+2")
        local t = types(tokens)
        assert.are.same({"NUMBER", "PLUS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a double-quoted string", function()
        local tokens = py_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = py_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('', tokens[1].value)
    end)

    it("tokenizes a string with escape sequence", function()
        local tokens = py_lexer.tokenize('"a\\nb"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Operator tokens
-- =========================================================================

describe("operator tokens", function()
    -- Python uses == for equality (must match before =)
    it("tokenizes == (equals equals)", function()
        local tokens = py_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
        assert.are.equal("==", tokens[1].value)
    end)

    it("tokenizes = (assignment)", function()
        local tokens = py_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
        assert.are.equal("=", tokens[1].value)
    end)

    it("tokenizes + (plus)", function()
        local tokens = py_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes - (minus)", function()
        local tokens = py_lexer.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
        assert.are.equal("-", tokens[1].value)
    end)

    it("tokenizes * (star)", function()
        local tokens = py_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
        assert.are.equal("*", tokens[1].value)
    end)

    it("tokenizes / (slash)", function()
        local tokens = py_lexer.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
        assert.are.equal("/", tokens[1].value)
    end)
end)

-- =========================================================================
-- Punctuation tokens
-- =========================================================================

describe("punctuation tokens", function()
    it("tokenizes ( and )", function()
        local tokens = py_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes comma", function()
        local tokens = py_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
        assert.are.equal(",", tokens[1].value)
    end)

    it("tokenizes colon", function()
        local tokens = py_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
        assert.are.equal(":", tokens[1].value)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes a simple assignment: x = 1", function()
        local tokens = py_lexer.tokenize("x = 1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
        assert.are.equal("x", tokens[1].value)
    end)

    it("tokenizes an equality check: x == 1", function()
        local tokens = py_lexer.tokenize("x == 1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS_EQUALS", "NUMBER"}, t)
    end)

    it("tokenizes a function definition header: def foo(x):", function()
        local tokens = py_lexer.tokenize("def foo(x):")
        local t = types(tokens)
        assert.are.same({"DEF", "NAME", "LPAREN", "NAME", "RPAREN", "COLON"}, t)
        assert.are.equal("foo", tokens[2].value)
    end)

    it("tokenizes a function call: foo(a, b)", function()
        local tokens = py_lexer.tokenize("foo(a, b)")
        local t = types(tokens)
        assert.are.same({"NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN"}, t)
    end)

    it("tokenizes an if/else header", function()
        local src = "if x == 1:"
        local tokens = py_lexer.tokenize(src)
        local first_if = first_of(tokens, "IF")
        assert.is_not_nil(first_if)
        local first_eq = first_of(tokens, "EQUALS_EQUALS")
        assert.is_not_nil(first_eq)
    end)

    it("tokenizes import statement: from os import path", function()
        local tokens = py_lexer.tokenize("from os import path")
        local t = types(tokens)
        assert.are.same({"FROM", "NAME", "IMPORT", "NAME"}, t)
    end)

    it("tokenizes import as: import os as operating_system", function()
        local tokens = py_lexer.tokenize("import os as operating_system")
        local t = types(tokens)
        assert.are.same({"IMPORT", "NAME", "AS", "NAME"}, t)
    end)

    it("tokenizes class definition header: class Foo:", function()
        local tokens = py_lexer.tokenize("class Foo:")
        local t = types(tokens)
        assert.are.same({"CLASS", "NAME", "COLON"}, t)
    end)

    it("tokenizes return statement: return True", function()
        local tokens = py_lexer.tokenize("return True")
        local t = types(tokens)
        assert.are.same({"RETURN", "TRUE"}, t)
    end)

    it("tokenizes None literal", function()
        local tokens = py_lexer.tokenize("x = None")
        local first_none = first_of(tokens, "NONE")
        assert.is_not_nil(first_none)
        assert.are.equal("None", first_none.value)
    end)

    it("tokenizes arithmetic: a + b * c", function()
        local tokens = py_lexer.tokenize("a + b * c")
        local t = types(tokens)
        assert.are.same({"NAME", "PLUS", "NAME", "STAR", "NAME"}, t)
    end)

    it("tokenizes for loop header: for i in x:", function()
        local tokens = py_lexer.tokenize("for i in x:")
        -- for, i, in (NAME since not a keyword in python.tokens), x, :
        local first_for = first_of(tokens, "FOR")
        assert.is_not_nil(first_for)
    end)

    it("tokenizes while loop header: while True:", function()
        local tokens = py_lexer.tokenize("while True:")
        local t = types(tokens)
        assert.are.same({"WHILE", "TRUE", "COLON"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = py_lexer.tokenize("x = 1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)

    it("strips tabs between tokens", function()
        local tokens = py_lexer.tokenize("x\t=\t1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: x = 42", function()
        -- x _ = _ 4 2
        -- 1 2 3 4 5 6
        local tokens = py_lexer.tokenize("x = 42")
        assert.are.equal(1, tokens[1].col)  -- x
        assert.are.equal(3, tokens[2].col)  -- =
        assert.are.equal(5, tokens[3].col)  -- 42
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = py_lexer.tokenize("x = 1")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = py_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = py_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character @", function()
        assert.has_error(function()
            py_lexer.tokenize("@")
        end)
    end)

    it("raises an error on unexpected character $", function()
        assert.has_error(function()
            py_lexer.tokenize("$x")
        end)
    end)
end)
