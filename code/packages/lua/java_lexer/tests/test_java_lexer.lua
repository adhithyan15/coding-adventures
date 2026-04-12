-- Tests for java_lexer
-- ====================
--
-- Comprehensive busted test suite for the Java lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Identifiers (NAME tokens)
--   - Numbers: integer literals
--   - Strings: double-quoted string literals
--   - Punctuation: (, ), {, }, [, ], ;, ,, .
--   - Whitespace is consumed silently
--   - Token positions (line, col) are tracked correctly
--   - Version-aware tokenization for all Java versions
--   - create_lexer and get_grammar functions
--   - Error handling for unexpected characters and invalid versions

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

local java_lexer = require("coding_adventures.java_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
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

describe("java_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(java_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(java_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", java_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(java_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(java_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = java_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = java_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = java_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = java_lexer.tokenize("myVar")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVar", tokens[1].value)
    end)

    it("tokenizes an identifier starting with underscore", function()
        local tokens = java_lexer.tokenize("_private")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_private", tokens[1].value)
    end)

    it("tokenizes identifier with digits in the middle", function()
        local tokens = java_lexer.tokenize("abc123")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("abc123", tokens[1].value)
    end)
end)

-- =========================================================================
-- Number tokens
-- =========================================================================

describe("number tokens", function()
    it("tokenizes an integer", function()
        local tokens = java_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = java_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by operators", function()
        local tokens = java_lexer.tokenize("1+2")
        local t = types(tokens)
        assert.are.same({"NUMBER", "PLUS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a double-quoted string", function()
        local tokens = java_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = java_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Punctuation tokens
-- =========================================================================

describe("punctuation tokens", function()
    it("tokenizes ( and )", function()
        local tokens = java_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes { and }", function()
        local tokens = java_lexer.tokenize("{}")
        local t = types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes [ and ]", function()
        local tokens = java_lexer.tokenize("[]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)

    it("tokenizes semicolon", function()
        local tokens = java_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
        assert.are.equal(";", tokens[1].value)
    end)

    it("tokenizes comma", function()
        local tokens = java_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
        assert.are.equal(",", tokens[1].value)
    end)

    it("tokenizes dot", function()
        local tokens = java_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
        assert.are.equal(".", tokens[1].value)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = java_lexer.tokenize("int x = 1;")
        -- Should not contain any whitespace tokens
        for _, tok in ipairs(tokens) do
            assert.are_not.equal("WHITESPACE", tok.type)
        end
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = java_lexer.tokenize("int\n\tx\n=\n1;")
        for _, tok in ipairs(tokens) do
            assert.are_not.equal("WHITESPACE", tok.type)
        end
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("all tokens on line 1 for single-line input", function()
        local tokens = java_lexer.tokenize("int x = 1;")
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
        local tokens = java_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = java_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character", function()
        assert.has_error(function()
            java_lexer.tokenize("#")
        end)
    end)
end)

-- =========================================================================
-- Version-aware tokenization
-- =========================================================================

describe("version-aware tokenization", function()

    it("tokenize with no version (default to 21)", function()
        local tokens = java_lexer.tokenize("int x = 1;")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenize with empty string version (default to 21)", function()
        local tokens = java_lexer.tokenize("int x = 1;", "")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 1.0", function()
        local tokens = java_lexer.tokenize("int x = 1;", "1.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 1.1", function()
        local tokens = java_lexer.tokenize("int x = 1;", "1.1")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 1.4", function()
        local tokens = java_lexer.tokenize("int x = 1;", "1.4")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 5", function()
        local tokens = java_lexer.tokenize("int x = 1;", "5")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 7", function()
        local tokens = java_lexer.tokenize("int x = 1;", "7")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 8", function()
        local tokens = java_lexer.tokenize("int x = 1;", "8")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 10", function()
        local tokens = java_lexer.tokenize("int x = 1;", "10")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 14", function()
        local tokens = java_lexer.tokenize("int x = 1;", "14")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 17", function()
        local tokens = java_lexer.tokenize("int x = 1;", "17")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 21", function()
        local tokens = java_lexer.tokenize("int x = 1;", "21")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    -- create_lexer with version

    it("create_lexer with version 8 returns a usable GrammarLexer", function()
        local gl = java_lexer.create_lexer("int x = 1;", "8")
        assert.is_not_nil(gl)
        assert.is_function(gl.tokenize)
    end)

    it("create_lexer with no version returns a usable GrammarLexer", function()
        local gl = java_lexer.create_lexer("int x = 1;")
        assert.is_not_nil(gl)
        assert.is_function(gl.tokenize)
    end)

    -- get_grammar with version

    it("get_grammar with version 1.0 returns a grammar object", function()
        local g = java_lexer.get_grammar("1.0")
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)

    it("get_grammar caches results across calls (same object returned)", function()
        local g1 = java_lexer.get_grammar("8")
        local g2 = java_lexer.get_grammar("8")
        assert.are.equal(g1, g2)
    end)

    -- Error on unknown version

    it("raises an error for unknown version string", function()
        assert.has_error(function()
            java_lexer.tokenize("int x = 1;", "99")
        end)
    end)

    it("raises an error for invalid version string", function()
        assert.has_error(function()
            java_lexer.tokenize("int x = 1;", "java21")
        end)
    end)
end)
