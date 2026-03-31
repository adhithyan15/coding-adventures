-- Tests for json_lexer
-- ====================
--
-- Comprehensive busted test suite for the JSON lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - All structural tokens: { } [ ] : ,
--   - Value tokens: STRING, NUMBER, TRUE, FALSE, NULL
--   - String with escape sequences: \" \\ \/ \b \f \n \r \t \uXXXX
--   - Numbers: integer, negative, float, scientific notation
--   - Whitespace is consumed silently (not in output)
--   - Nested JSON object
--   - Nested JSON array
--   - Real-world mixed JSON value
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

local json_lexer = require("coding_adventures.json_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by json_lexer.tokenize.
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
-- @param tokens  table  The token list returned by json_lexer.tokenize.
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

describe("json_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(json_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(json_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", json_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(json_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(json_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = json_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = json_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = json_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Structural tokens
-- =========================================================================

describe("structural tokens", function()
    it("tokenizes empty object {}", function()
        local tokens = json_lexer.tokenize("{}")
        local t = types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes empty array []", function()
        local tokens = json_lexer.tokenize("[]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)

    it("tokenizes colon", function()
        local tokens = json_lexer.tokenize('"key": 1')
        local t = types(tokens)
        assert.truthy(t[2] == "COLON")
    end)

    it("tokenizes comma", function()
        local tokens = json_lexer.tokenize('1, 2')
        local t = types(tokens)
        assert.truthy(t[2] == "COMMA")
    end)

    it("correct values for all structural tokens", function()
        local tokens = json_lexer.tokenize("{}[]:,")
        local v = values(tokens)
        assert.are.same({"{", "}", "[", "]", ":", ","}, v)
    end)
end)

-- =========================================================================
-- Value tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a simple string", function()
        local tokens = json_lexer.tokenize('"hello"')
        local t = types(tokens)
        assert.are.same({"STRING"}, t)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes an empty string", function()
        local tokens = json_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('', tokens[1].value)
    end)

    it("preserves escape sequences in string value", function()
        -- The lexer returns the raw source text (with escapes intact).
        -- Escape processing (converting \n to newline, etc.) is the parser's job.
        local tokens = json_lexer.tokenize('"a\\nb"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('a\\nb', tokens[1].value)
    end)

    it("tokenizes string with \\\" escape", function()
        local tokens = json_lexer.tokenize('"say \\"hi\\""')
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("tokenizes string with unicode escape \\uXXXX", function()
        local tokens = json_lexer.tokenize('"\\u0041"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('\\u0041', tokens[1].value)
    end)

    it("tokenizes string with all simple escapes", function()
        local tokens = json_lexer.tokenize('"\\\\\\/\\b\\f\\n\\r\\t"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

describe("number tokens", function()
    it("tokenizes a positive integer", function()
        local tokens = json_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = json_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes a negative integer", function()
        local tokens = json_lexer.tokenize("-7")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("-7", tokens[1].value)
    end)

    it("tokenizes a float", function()
        local tokens = json_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes a negative float", function()
        local tokens = json_lexer.tokenize("-0.5")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("-0.5", tokens[1].value)
    end)

    it("tokenizes scientific notation with e+", function()
        local tokens = json_lexer.tokenize("1e10")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("1e10", tokens[1].value)
    end)

    it("tokenizes scientific notation with E-", function()
        local tokens = json_lexer.tokenize("2.5E-3")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("2.5E-3", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by commas", function()
        local tokens = json_lexer.tokenize("1,2,3")
        local t = types(tokens)
        assert.are.same({"NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER"}, t)
    end)
end)

describe("literal tokens", function()
    it("tokenizes true", function()
        local tokens = json_lexer.tokenize("true")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)
    end)

    it("tokenizes false", function()
        local tokens = json_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)

    it("tokenizes null", function()
        local tokens = json_lexer.tokenize("null")
        assert.are.equal("NULL", tokens[1].type)
        assert.are.equal("null", tokens[1].value)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = json_lexer.tokenize('{ "k" : 1 }')
        local t = types(tokens)
        assert.are.same({"LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"}, t)
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = json_lexer.tokenize("[\n\t1,\n\t2\n]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "NUMBER", "COMMA", "NUMBER", "RBRACKET"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input", function()
        -- Input: {"k":1}
        -- col:   1234567
        local tokens = json_lexer.tokenize('{"k":1}')
        assert.are.equal(1, tokens[1].col)  -- {
        assert.are.equal(2, tokens[2].col)  -- "k"
        assert.are.equal(5, tokens[3].col)  -- :
        assert.are.equal(6, tokens[4].col)  -- 1
        assert.are.equal(7, tokens[5].col)  -- }
    end)

    it("all tokens start on line 1 for a single-line input", function()
        local tokens = json_lexer.tokenize('{"a":1}')
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- Composite structures
-- =========================================================================

describe("composite JSON structures", function()
    it("tokenizes a simple key-value object", function()
        local tokens = json_lexer.tokenize('{"key": 42}')
        local t = types(tokens)
        assert.are.same(
            {"LBRACE", "STRING", "COLON", "NUMBER", "RBRACE"},
            t
        )
    end)

    it("tokenizes an array of mixed values", function()
        local tokens = json_lexer.tokenize('[1, "two", true, null]')
        local t = types(tokens)
        assert.are.same(
            {"LBRACKET", "NUMBER", "COMMA", "STRING", "COMMA",
             "TRUE", "COMMA", "NULL", "RBRACKET"},
            t
        )
    end)

    it("tokenizes a nested object", function()
        local src = '{"a": {"b": 2}}'
        local tokens = json_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same(
            {"LBRACE", "STRING", "COLON",
             "LBRACE", "STRING", "COLON", "NUMBER", "RBRACE",
             "RBRACE"},
            t
        )
    end)

    it("tokenizes a nested array", function()
        local tokens = json_lexer.tokenize("[[1,2],[3]]")
        local t = types(tokens)
        assert.are.same(
            {"LBRACKET",
             "LBRACKET", "NUMBER", "COMMA", "NUMBER", "RBRACKET",
             "COMMA",
             "LBRACKET", "NUMBER", "RBRACKET",
             "RBRACKET"},
            t
        )
    end)

    it("tokenizes a real-world JSON blob", function()
        local src = [[
{
  "name": "Alice",
  "age": 30,
  "active": true,
  "score": -1.5e2,
  "tags": ["lua", "json"],
  "meta": null
}]]
        local tokens = json_lexer.tokenize(src)
        -- Must have at least 20+ tokens; spot-check a few
        assert.truthy(#tokens > 20)

        local first_string = first_of(tokens, "STRING")
        assert.is_not_nil(first_string)
        assert.are.equal('name', first_string.value)

        local first_number = first_of(tokens, "NUMBER")
        assert.is_not_nil(first_number)

        local first_true = first_of(tokens, "TRUE")
        assert.is_not_nil(first_true)

        local first_null = first_of(tokens, "NULL")
        assert.is_not_nil(first_null)

        -- Last real token before EOF is RBRACE
        local last = tokens[#tokens - 1]
        assert.are.equal("RBRACE", last.type)
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = json_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = json_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character", function()
        assert.has_error(function()
            json_lexer.tokenize("@")
        end)
    end)

    it("raises an error on bare identifier (not true/false/null)", function()
        assert.has_error(function()
            json_lexer.tokenize("undefined")
        end)
    end)
end)
