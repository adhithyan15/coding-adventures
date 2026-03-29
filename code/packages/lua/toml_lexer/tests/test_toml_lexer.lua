-- Tests for toml_lexer
-- ====================
--
-- Comprehensive busted test suite for the TOML lexer package.
--
-- TOML (Tom's Obvious, Minimal Language) is a configuration file format.
-- This suite exercises all token types produced by the `toml.tokens` grammar.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Simple key-value: BARE_KEY EQUALS BASIC_STRING
--   - Table headers: [section] and [[array-of-tables]]
--   - All string types: basic, literal, multi-line basic, multi-line literal
--   - Integer literals: decimal, hex, octal, binary
--   - Float literals: decimal, scientific, inf, nan
--   - Boolean literals: true, false
--   - Date/time literals: offset datetime, local datetime, local date, local time
--   - Inline tables { key = "val" }
--   - Arrays [1, 2, 3]
--   - Whitespace (spaces, tabs) consumed silently
--   - Comments consumed silently
--   - Token positions (line, col) tracked correctly
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

local toml_lexer = require("coding_adventures.toml_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF and
-- any NEWLINE tokens unless explicitly included).
-- @param tokens  table  The token list returned by toml_lexer.tokenize.
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

--- Collect token types ignoring both EOF and NEWLINE tokens.
-- Useful for testing value token sequences without caring about line endings.
-- @param tokens  table  Token list.
-- @return table         Ordered list of type strings (no EOF, no NEWLINE).
local function types_no_nl(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" and tok.type ~= "NEWLINE" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Collect token values from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by toml_lexer.tokenize.
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

--- Count tokens of a given type.
-- @param tokens table   Token list.
-- @param typ    string  Token type to count.
-- @return number        Number of tokens with that type.
local function count_of(tokens, typ)
    local n = 0
    for _, tok in ipairs(tokens) do
        if tok.type == typ then n = n + 1 end
    end
    return n
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("toml_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(toml_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(toml_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", toml_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(toml_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(toml_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = toml_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = toml_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input (spaces and tabs) produces only EOF", function()
        -- TOML skips spaces and tabs, but NOT newlines.
        local tokens = toml_lexer.tokenize("   \t   ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("comment-only line produces only EOF (comment is skipped)", function()
        -- TOML comments run from # to end of line; the newline itself
        -- would be emitted but in this test there's no trailing newline.
        local tokens = toml_lexer.tokenize("# this is a comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Key-value pairs
-- =========================================================================

describe("key-value pairs", function()
    it("tokenizes: key = \"value\"", function()
        local tokens = toml_lexer.tokenize('key = "value"')
        local t = types_no_nl(tokens)
        assert.are.same({"BARE_KEY", "EQUALS", "BASIC_STRING"}, t)
    end)

    it("bare key value is correct", function()
        local tokens = toml_lexer.tokenize('key = "value"')
        assert.are.equal("key",     tokens[1].value)
        assert.are.equal("=",       tokens[2].value)
        assert.are.equal('"value"', tokens[3].value)
    end)

    it("tokenizes dotted key: a.b = 1", function()
        local tokens = toml_lexer.tokenize("a.b = 1")
        local t = types_no_nl(tokens)
        -- a DOT b EQUALS INTEGER
        assert.are.same({"BARE_KEY", "DOT", "BARE_KEY", "EQUALS", "INTEGER"}, t)
    end)

    it("tokenizes multiple key-value pairs across lines", function()
        local src = 'name = "Alice"\nage = 30\n'
        local tokens = toml_lexer.tokenize(src)
        local t = types_no_nl(tokens)
        assert.are.same(
            {"BARE_KEY", "EQUALS", "BASIC_STRING",
             "BARE_KEY", "EQUALS", "INTEGER"},
            t
        )
    end)
end)

-- =========================================================================
-- Table headers
-- =========================================================================

describe("table headers", function()
    it("tokenizes [section]", function()
        local tokens = toml_lexer.tokenize("[section]")
        local t = types_no_nl(tokens)
        assert.are.same({"LBRACKET", "BARE_KEY", "RBRACKET"}, t)
    end)

    it("tokenizes [[array-of-tables]]", function()
        -- [[array]] is two consecutive LBRACKET tokens followed by two RBRACKET.
        local tokens = toml_lexer.tokenize("[[products]]")
        local t = types_no_nl(tokens)
        assert.are.same(
            {"LBRACKET", "LBRACKET", "BARE_KEY", "RBRACKET", "RBRACKET"},
            t
        )
    end)

    it("tokenizes section header with dotted key: [a.b]", function()
        local tokens = toml_lexer.tokenize("[a.b]")
        local t = types_no_nl(tokens)
        assert.are.same(
            {"LBRACKET", "BARE_KEY", "DOT", "BARE_KEY", "RBRACKET"},
            t
        )
    end)
end)

-- =========================================================================
-- String literals
-- =========================================================================

describe("basic strings (double-quoted)", function()
    it("tokenizes a simple basic string", function()
        local tokens = toml_lexer.tokenize('"hello"')
        assert.are.equal("BASIC_STRING", tokens[1].type)
        assert.are.equal('"hello"', tokens[1].value)
    end)

    it("tokenizes an empty basic string", function()
        local tokens = toml_lexer.tokenize('""')
        assert.are.equal("BASIC_STRING", tokens[1].type)
        assert.are.equal('""', tokens[1].value)
    end)

    it("preserves escape sequences in basic string value", function()
        -- The lexer returns raw source text; escape processing is for the parser.
        local tokens = toml_lexer.tokenize('"a\\nb"')
        assert.are.equal("BASIC_STRING", tokens[1].type)
        assert.are.equal('"a\\nb"', tokens[1].value)
    end)

    it("tokenizes basic string with unicode escape \\uXXXX", function()
        local tokens = toml_lexer.tokenize('"\\u0041"')
        assert.are.equal("BASIC_STRING", tokens[1].type)
    end)
end)

describe("literal strings (single-quoted)", function()
    it("tokenizes a simple literal string", function()
        local tokens = toml_lexer.tokenize("'hello'")
        assert.are.equal("LITERAL_STRING", tokens[1].type)
        assert.are.equal("'hello'", tokens[1].value)
    end)

    it("tokenizes a literal string with backslash (no escape)", function()
        -- Literal strings treat backslash as a literal character.
        local tokens = toml_lexer.tokenize("'C:\\\\path'")
        assert.are.equal("LITERAL_STRING", tokens[1].type)
    end)
end)

describe("multi-line basic strings (triple-double-quoted)", function()
    it("tokenizes a multi-line basic string", function()
        local src = '"""hello\nworld"""'
        local tokens = toml_lexer.tokenize(src)
        assert.are.equal("ML_BASIC_STRING", tokens[1].type)
        assert.are.equal(src, tokens[1].value)
    end)
end)

describe("multi-line literal strings (triple-single-quoted)", function()
    it("tokenizes a multi-line literal string", function()
        local src = "'''hello\nworld'''"
        local tokens = toml_lexer.tokenize(src)
        assert.are.equal("ML_LITERAL_STRING", tokens[1].type)
        assert.are.equal(src, tokens[1].value)
    end)
end)

-- =========================================================================
-- Integer literals
-- =========================================================================

describe("integer literals", function()
    it("tokenizes a decimal integer", function()
        local tokens = toml_lexer.tokenize("42")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = toml_lexer.tokenize("0")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes a positive integer with sign", function()
        local tokens = toml_lexer.tokenize("+99")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("+99", tokens[1].value)
    end)

    it("tokenizes a negative integer", function()
        local tokens = toml_lexer.tokenize("-17")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("-17", tokens[1].value)
    end)

    it("tokenizes a hex integer (aliased to INTEGER)", function()
        local tokens = toml_lexer.tokenize("0xFF")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("0xFF", tokens[1].value)
    end)

    it("tokenizes an octal integer (aliased to INTEGER)", function()
        local tokens = toml_lexer.tokenize("0o755")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("0o755", tokens[1].value)
    end)

    it("tokenizes a binary integer (aliased to INTEGER)", function()
        local tokens = toml_lexer.tokenize("0b1010")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("0b1010", tokens[1].value)
    end)

    it("tokenizes underscore-separated integer", function()
        local tokens = toml_lexer.tokenize("1_000_000")
        assert.are.equal("INTEGER", tokens[1].type)
        assert.are.equal("1_000_000", tokens[1].value)
    end)
end)

-- =========================================================================
-- Float literals
-- =========================================================================

describe("float literals", function()
    it("tokenizes a decimal float", function()
        local tokens = toml_lexer.tokenize("3.14")
        assert.are.equal("FLOAT", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes a negative float", function()
        local tokens = toml_lexer.tokenize("-0.5")
        assert.are.equal("FLOAT", tokens[1].type)
        assert.are.equal("-0.5", tokens[1].value)
    end)

    it("tokenizes scientific notation float", function()
        local tokens = toml_lexer.tokenize("5e22")
        assert.are.equal("FLOAT", tokens[1].type)
        assert.are.equal("5e22", tokens[1].value)
    end)

    it("tokenizes scientific notation with explicit + exponent", function()
        local tokens = toml_lexer.tokenize("1e+99")
        assert.are.equal("FLOAT", tokens[1].type)
    end)

    it("tokenizes positive infinity", function()
        local tokens = toml_lexer.tokenize("inf")
        assert.are.equal("FLOAT", tokens[1].type)
        assert.are.equal("inf", tokens[1].value)
    end)

    it("tokenizes negative infinity", function()
        local tokens = toml_lexer.tokenize("-inf")
        assert.are.equal("FLOAT", tokens[1].type)
        assert.are.equal("-inf", tokens[1].value)
    end)

    it("tokenizes not-a-number", function()
        local tokens = toml_lexer.tokenize("nan")
        assert.are.equal("FLOAT", tokens[1].type)
        assert.are.equal("nan", tokens[1].value)
    end)
end)

-- =========================================================================
-- Boolean literals
-- =========================================================================

describe("boolean literals", function()
    it("tokenizes true", function()
        local tokens = toml_lexer.tokenize("true")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)
    end)

    it("tokenizes false", function()
        local tokens = toml_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)
end)

-- =========================================================================
-- Date/time literals
-- =========================================================================

describe("date and time literals", function()
    it("tokenizes offset datetime with Z", function()
        local tokens = toml_lexer.tokenize("1979-05-27T07:32:00Z")
        assert.are.equal("OFFSET_DATETIME", tokens[1].type)
        assert.are.equal("1979-05-27T07:32:00Z", tokens[1].value)
    end)

    it("tokenizes offset datetime with timezone offset", function()
        local tokens = toml_lexer.tokenize("1979-05-27T00:32:00+09:00")
        assert.are.equal("OFFSET_DATETIME", tokens[1].type)
    end)

    it("tokenizes local datetime", function()
        local tokens = toml_lexer.tokenize("1979-05-27T07:32:00")
        assert.are.equal("LOCAL_DATETIME", tokens[1].type)
        assert.are.equal("1979-05-27T07:32:00", tokens[1].value)
    end)

    it("tokenizes local date", function()
        local tokens = toml_lexer.tokenize("1979-05-27")
        assert.are.equal("LOCAL_DATE", tokens[1].type)
        assert.are.equal("1979-05-27", tokens[1].value)
    end)

    it("tokenizes local time", function()
        local tokens = toml_lexer.tokenize("07:32:00")
        assert.are.equal("LOCAL_TIME", tokens[1].type)
        assert.are.equal("07:32:00", tokens[1].value)
    end)

    it("tokenizes local time with fractional seconds", function()
        local tokens = toml_lexer.tokenize("07:32:00.999")
        assert.are.equal("LOCAL_TIME", tokens[1].type)
        assert.are.equal("07:32:00.999", tokens[1].value)
    end)
end)

-- =========================================================================
-- Inline tables
-- =========================================================================

describe("inline tables", function()
    it("tokenizes { key = \"val\" }", function()
        local tokens = toml_lexer.tokenize('{ key = "val" }')
        local t = types_no_nl(tokens)
        assert.are.same(
            {"LBRACE", "BARE_KEY", "EQUALS", "BASIC_STRING", "RBRACE"},
            t
        )
    end)

    it("tokenizes multi-key inline table", function()
        local tokens = toml_lexer.tokenize('{ x = 1, y = 2 }')
        local t = types_no_nl(tokens)
        assert.are.same(
            {"LBRACE",
             "BARE_KEY", "EQUALS", "INTEGER",
             "COMMA",
             "BARE_KEY", "EQUALS", "INTEGER",
             "RBRACE"},
            t
        )
    end)
end)

-- =========================================================================
-- Arrays
-- =========================================================================

describe("arrays", function()
    it("tokenizes [1, 2, 3]", function()
        local tokens = toml_lexer.tokenize("[1, 2, 3]")
        local t = types_no_nl(tokens)
        assert.are.same(
            {"LBRACKET",
             "INTEGER", "COMMA",
             "INTEGER", "COMMA",
             "INTEGER",
             "RBRACKET"},
            t
        )
    end)

    it("tokenizes array of strings", function()
        local tokens = toml_lexer.tokenize('["a", "b", "c"]')
        local t = types_no_nl(tokens)
        assert.are.same(
            {"LBRACKET",
             "BASIC_STRING", "COMMA",
             "BASIC_STRING", "COMMA",
             "BASIC_STRING",
             "RBRACKET"},
            t
        )
    end)

    it("tokenizes empty array []", function()
        local tokens = toml_lexer.tokenize("[]")
        local t = types_no_nl(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)
end)

-- =========================================================================
-- Whitespace and comment handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces and tabs between tokens on the same line", function()
        local tokens = toml_lexer.tokenize('key  =  "val"')
        local t = types_no_nl(tokens)
        assert.are.same({"BARE_KEY", "EQUALS", "BASIC_STRING"}, t)
    end)

    it("comment after value is consumed silently", function()
        -- The '#' comment goes to end of line; the value before it is intact.
        local tokens = toml_lexer.tokenize('key = 42 # this is the answer')
        local t = types_no_nl(tokens)
        assert.are.same({"BARE_KEY", "EQUALS", "INTEGER"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input", function()
        -- Input: key=1
        -- col:   12345
        local tokens = toml_lexer.tokenize("key=1")
        assert.are.equal(1, tokens[1].col)  -- key
        assert.are.equal(4, tokens[2].col)  -- =
        assert.are.equal(5, tokens[3].col)  -- 1
    end)

    it("all tokens start on line 1 for a single-line input", function()
        local tokens = toml_lexer.tokenize('key = "val"')
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- Composite TOML structures
-- =========================================================================

describe("composite TOML structures", function()
    it("tokenizes a full TOML document", function()
        local src = [[
[server]
host = "localhost"
port = 8080
debug = true
]]
        local tokens = toml_lexer.tokenize(src)
        -- We should find key token types present
        assert.truthy(#tokens > 10)

        local first_bracket = first_of(tokens, "LBRACKET")
        assert.is_not_nil(first_bracket)

        local first_key = first_of(tokens, "BARE_KEY")
        assert.is_not_nil(first_key)
        assert.are.equal("server", first_key.value)

        assert.truthy(count_of(tokens, "BARE_KEY") >= 4)
        assert.truthy(count_of(tokens, "EQUALS") >= 3)

        local t_tok = first_of(tokens, "TRUE")
        assert.is_not_nil(t_tok)
    end)

    it("tokenizes integer, float, boolean in sequence", function()
        local tokens = toml_lexer.tokenize("42 3.14 true false")
        local t = types_no_nl(tokens)
        assert.are.same({"INTEGER", "FLOAT", "TRUE", "FALSE"}, t)
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = toml_lexer.tokenize("key = 1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = toml_lexer.tokenize("key = 1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character", function()
        -- The backtick is not a valid TOML character
        assert.has_error(function()
            toml_lexer.tokenize("`invalid`")
        end)
    end)
end)
