-- Tests for csharp_lexer
-- ======================
--
-- Comprehensive busted test suite for the C# lexer package.
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
--   - Version-aware tokenization for all 12 C# versions
--   - create_csharp_lexer and get_grammar functions
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

local csharp_lexer = require("coding_adventures.csharp_lexer")

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

describe("csharp_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(csharp_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(csharp_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", csharp_lexer.VERSION)
    end)

    it("exposes tokenize_csharp as a function", function()
        assert.is_function(csharp_lexer.tokenize_csharp)
    end)

    it("exposes create_csharp_lexer as a function", function()
        assert.is_function(csharp_lexer.create_csharp_lexer)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(csharp_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = csharp_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = csharp_lexer.tokenize_csharp("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = csharp_lexer.tokenize_csharp("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Identifier tokens
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = csharp_lexer.tokenize_csharp("myVar")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVar", tokens[1].value)
    end)

    it("tokenizes an identifier starting with underscore", function()
        local tokens = csharp_lexer.tokenize_csharp("_private")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_private", tokens[1].value)
    end)

    it("tokenizes an identifier with digits in the middle", function()
        local tokens = csharp_lexer.tokenize_csharp("abc123")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("abc123", tokens[1].value)
    end)

    it("tokenizes an @ verbatim identifier", function()
        -- C# allows @ to escape reserved words as identifiers: @class, @int
        local tokens = csharp_lexer.tokenize_csharp("@class")
        -- The grammar may emit this as NAME or VERBATIM_IDENTIFIER; either is fine
        assert.is_not_nil(tokens[1])
        assert.are_not.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Number tokens
-- =========================================================================

describe("number tokens", function()
    it("tokenizes an integer", function()
        local tokens = csharp_lexer.tokenize_csharp("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = csharp_lexer.tokenize_csharp("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by operators", function()
        local tokens = csharp_lexer.tokenize_csharp("1 + 2")
        local t = types(tokens)
        assert.are.same({"NUMBER", "PLUS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a double-quoted string", function()
        local tokens = csharp_lexer.tokenize_csharp('"hello"')
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = csharp_lexer.tokenize_csharp('""')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Punctuation tokens
-- =========================================================================

describe("punctuation tokens", function()
    it("tokenizes ( and )", function()
        local tokens = csharp_lexer.tokenize_csharp("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes { and }", function()
        local tokens = csharp_lexer.tokenize_csharp("{}")
        local t = types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes [ and ]", function()
        local tokens = csharp_lexer.tokenize_csharp("[]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)

    it("tokenizes semicolon", function()
        local tokens = csharp_lexer.tokenize_csharp(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
        assert.are.equal(";", tokens[1].value)
    end)

    it("tokenizes comma", function()
        local tokens = csharp_lexer.tokenize_csharp(",")
        assert.are.equal("COMMA", tokens[1].type)
        assert.are.equal(",", tokens[1].value)
    end)

    it("tokenizes dot", function()
        local tokens = csharp_lexer.tokenize_csharp(".")
        assert.are.equal("DOT", tokens[1].type)
        assert.are.equal(".", tokens[1].value)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;")
        -- Should not contain any whitespace tokens
        for _, tok in ipairs(tokens) do
            assert.are_not.equal("WHITESPACE", tok.type)
        end
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = csharp_lexer.tokenize_csharp("int\n\tx\n=\n1;")
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
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;")
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
        local tokens = csharp_lexer.tokenize_csharp("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = csharp_lexer.tokenize_csharp("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for unknown version string", function()
        assert.has_error(function()
            csharp_lexer.tokenize_csharp("int x = 1;", "99.0")
        end)
    end)

    it("raises an error for invalid version string", function()
        assert.has_error(function()
            csharp_lexer.tokenize_csharp("int x = 1;", "csharp12")
        end)
    end)
end)

-- =========================================================================
-- Version-aware tokenization
-- =========================================================================
--
-- C# has 12 versions. We test that each grammar loads correctly and
-- produces a sensible token stream for the simplest valid C# expression:
-- `int x = 1;`.
--
-- The grammar files differ in which keywords they recognize (e.g., "async"
-- and "await" first appeared in 5.0, "dynamic" in 4.0, "var" in 3.0).
-- A plain integer assignment is valid in all versions.

describe("version-aware tokenization", function()

    it("tokenize with no version (defaults to 12.0)", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenize with empty string version (defaults to 12.0)", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 1.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "1.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 2.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "2.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 3.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "3.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 4.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "4.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 5.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "5.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 6.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "6.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 7.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "7.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 8.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "8.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 9.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "9.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 10.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "10.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 11.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "11.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    it("tokenizes with version 12.0", function()
        local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "12.0")
        assert.is_table(tokens)
        assert.is_true(#tokens > 0)
    end)

    -- create_csharp_lexer with version

    it("create_csharp_lexer with version 8.0 returns a usable GrammarLexer", function()
        local gl = csharp_lexer.create_csharp_lexer("int x = 1;", "8.0")
        assert.is_not_nil(gl)
        assert.is_function(gl.tokenize)
    end)

    it("create_csharp_lexer with no version returns a usable GrammarLexer", function()
        local gl = csharp_lexer.create_csharp_lexer("int x = 1;")
        assert.is_not_nil(gl)
        assert.is_function(gl.tokenize)
    end)

    -- get_grammar with version

    it("get_grammar with version 1.0 returns a grammar object", function()
        local g = csharp_lexer.get_grammar("1.0")
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)

    it("get_grammar with version 12.0 returns a grammar object", function()
        local g = csharp_lexer.get_grammar("12.0")
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)

    it("get_grammar caches results across calls (same object returned)", function()
        local g1 = csharp_lexer.get_grammar("8.0")
        local g2 = csharp_lexer.get_grammar("8.0")
        assert.are.equal(g1, g2)
    end)
end)
