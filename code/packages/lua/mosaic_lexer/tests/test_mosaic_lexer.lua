-- Tests for mosaic_lexer
-- =====================
--
-- Comprehensive busted test suite for the hand-written Mosaic lexer.
--
-- Coverage:
--   - Module loads and exposes the public API
--   - VERSION string is present
--   - Empty input → only EOF
--   - Whitespace and comments are skipped silently
--   - All control keywords: component, slot, when, each, as, from, import
--   - All type keywords: text, number, bool, image, color, node, list, true, false
--   - Structural tokens: { } < > : ; @ , . =
--   - String literals (with escape sequences)
--   - HEX_COLOR: #rgb, #rrggbb, #rrggbbaa
--   - DIMENSION: 16dp, 1.5sp, 100%
--   - NUMBER: integers, negative, floats
--   - NAME: plain identifiers, hyphenated CSS-style names
--   - Position tracking (line and col)
--   - Error handling: unexpected characters, unterminated strings

-- Path setup so busted can find the module source
package.path = (
    "../src/?.lua;"          ..
    "../src/?/init.lua;"     ..
    package.path
)

local lexer = require("coding_adventures.mosaic_lexer")

-- ============================================================================
-- Helpers
-- ============================================================================

--- Extract the type sequence (excluding EOF) from a token list.
local function types(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then out[#out + 1] = tok.type end
    end
    return out
end

--- Extract the value sequence (excluding EOF) from a token list.
local function values(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then out[#out + 1] = tok.value end
    end
    return out
end

--- Tokenize and assert no error.
local function lex(src)
    local toks, err = lexer.tokenize(src)
    assert(toks, "unexpected lex error: " .. tostring(err))
    return toks
end

-- ============================================================================
-- Module surface
-- ============================================================================

describe("mosaic_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(lexer)
    end)

    it("exposes VERSION", function()
        assert.is_string(lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(lexer.tokenize)
    end)

    it("returns two values: tokens and nil on success", function()
        local toks, err = lexer.tokenize("")
        assert.is_table(toks)
        assert.is_nil(err)
    end)
end)

-- ============================================================================
-- Empty and whitespace inputs
-- ============================================================================

describe("empty and whitespace inputs", function()
    it("empty string → only EOF", function()
        local toks = lex("")
        assert.equals(1, #toks)
        assert.equals("EOF", toks[1].type)
        assert.equals("",    toks[1].value)
    end)

    it("whitespace-only → only EOF", function()
        local toks = lex("   \t\r\n  ")
        assert.equals(1, #toks)
        assert.equals("EOF", toks[1].type)
    end)

    it("line comment is skipped", function()
        local toks = lex("// this is a comment\n{}")
        assert.are.same({"LBRACE", "RBRACE"}, types(toks))
    end)

    it("block comment is skipped", function()
        local toks = lex("/* skip me */ {}")
        assert.are.same({"LBRACE", "RBRACE"}, types(toks))
    end)

    it("multi-line block comment is skipped", function()
        local toks = lex("/*\nline1\nline2\n*/ component")
        assert.are.same({"COMPONENT"}, types(toks))
    end)
end)

-- ============================================================================
-- Control keywords
-- ============================================================================

describe("control keywords", function()
    it("component → COMPONENT", function()
        local toks = lex("component")
        assert.equals("COMPONENT", toks[1].type)
        assert.equals("component", toks[1].value)
    end)

    it("slot → SLOT", function()
        local toks = lex("slot")
        assert.equals("SLOT", toks[1].type)
    end)

    it("when → WHEN", function()
        local toks = lex("when")
        assert.equals("WHEN", toks[1].type)
    end)

    it("each → EACH", function()
        local toks = lex("each")
        assert.equals("EACH", toks[1].type)
    end)

    it("as → AS", function()
        local toks = lex("as")
        assert.equals("AS", toks[1].type)
    end)

    it("from → FROM", function()
        local toks = lex("from")
        assert.equals("FROM", toks[1].type)
    end)

    it("import → IMPORT", function()
        local toks = lex("import")
        assert.equals("IMPORT", toks[1].type)
    end)
end)

-- ============================================================================
-- Type keywords
-- ============================================================================

describe("type keywords", function()
    local type_kws = {"text", "number", "bool", "image", "color", "node", "list", "true", "false"}
    for _, kw in ipairs(type_kws) do
        it(kw .. " → KEYWORD", function()
            local toks = lex(kw)
            assert.equals("KEYWORD", toks[1].type)
            assert.equals(kw, toks[1].value)
        end)
    end
end)

-- ============================================================================
-- Structural tokens
-- ============================================================================

describe("structural tokens", function()
    it("{ → LBRACE", function()
        assert.equals("LBRACE", lex("{")[1].type)
    end)

    it("} → RBRACE", function()
        assert.equals("RBRACE", lex("}")[1].type)
    end)

    it("< → LANGLE", function()
        assert.equals("LANGLE", lex("<")[1].type)
    end)

    it("> → RANGLE", function()
        assert.equals("RANGLE", lex(">")[1].type)
    end)

    it(": → COLON", function()
        assert.equals("COLON", lex(":")[1].type)
    end)

    it("; → SEMICOLON", function()
        assert.equals("SEMICOLON", lex(";")[1].type)
    end)

    it("@ → AT", function()
        assert.equals("AT", lex("@")[1].type)
    end)

    it(", → COMMA", function()
        assert.equals("COMMA", lex(",")[1].type)
    end)

    it(". → DOT", function()
        assert.equals("DOT", lex(".")[1].type)
    end)

    it("= → EQUALS", function()
        assert.equals("EQUALS", lex("=")[1].type)
    end)

    it("all structural in sequence", function()
        local toks = lex("{}:<>;@,.")
        assert.are.same(
            {"LBRACE","RBRACE","COLON","LANGLE","RANGLE","SEMICOLON","AT","COMMA","DOT"},
            types(toks)
        )
    end)
end)

-- ============================================================================
-- String literals
-- ============================================================================

describe("string literals", function()
    it("simple string", function()
        local toks = lex('"hello"')
        assert.equals("STRING", toks[1].type)
        assert.equals("hello",  toks[1].value)
    end)

    it("empty string", function()
        local toks = lex('""')
        assert.equals("STRING", toks[1].type)
        assert.equals("",       toks[1].value)
    end)

    it("string with escape sequences preserved as-is", function()
        local toks = lex('"a\\nb"')
        assert.equals("STRING", toks[1].type)
        assert.equals("a\\nb",  toks[1].value)
    end)

    it("string with embedded escaped quote", function()
        local toks = lex('"say \\"hi\\""')
        assert.equals("STRING", toks[1].type)
    end)

    it("string with unicode escape", function()
        local toks = lex('"\\u0041"')
        assert.equals("STRING",   toks[1].type)
        assert.equals("\\u0041", toks[1].value)
    end)
end)

-- ============================================================================
-- HEX_COLOR
-- ============================================================================

describe("HEX_COLOR tokens", function()
    it("#rrggbb → HEX_COLOR", function()
        local toks = lex("#2563eb")
        assert.equals("HEX_COLOR", toks[1].type)
        assert.equals("#2563eb",   toks[1].value)
    end)

    it("#rgb → HEX_COLOR", function()
        local toks = lex("#fff")
        assert.equals("HEX_COLOR", toks[1].type)
        assert.equals("#fff",      toks[1].value)
    end)

    it("#rrggbbaa → HEX_COLOR", function()
        local toks = lex("#ff000080")
        assert.equals("HEX_COLOR", toks[1].type)
        assert.equals("#ff000080", toks[1].value)
    end)

    it("color inside property: background: #2563eb;", function()
        local toks = lex("background: #2563eb;")
        assert.are.same({"NAME","COLON","HEX_COLOR","SEMICOLON"}, types(toks))
    end)
end)

-- ============================================================================
-- DIMENSION and NUMBER
-- ============================================================================

describe("DIMENSION tokens", function()
    it("16dp → DIMENSION", function()
        local toks = lex("16dp")
        assert.equals("DIMENSION", toks[1].type)
        assert.equals("16dp",      toks[1].value)
    end)

    it("1.5sp → DIMENSION", function()
        local toks = lex("1.5sp")
        assert.equals("DIMENSION", toks[1].type)
        assert.equals("1.5sp",     toks[1].value)
    end)

    it("100% → DIMENSION", function()
        local toks = lex("100%")
        assert.equals("DIMENSION", toks[1].type)
        assert.equals("100%",      toks[1].value)
    end)

    it("24px → DIMENSION", function()
        local toks = lex("24px")
        assert.equals("DIMENSION", toks[1].type)
        assert.equals("24px",      toks[1].value)
    end)
end)

describe("NUMBER tokens", function()
    it("42 → NUMBER", function()
        local toks = lex("42")
        assert.equals("NUMBER", toks[1].type)
        assert.equals("42",     toks[1].value)
    end)

    it("-3.14 → NUMBER", function()
        local toks = lex("-3.14")
        assert.equals("NUMBER", toks[1].type)
        assert.equals("-3.14",  toks[1].value)
    end)

    it("0 → NUMBER", function()
        local toks = lex("0")
        assert.equals("NUMBER", toks[1].type)
        assert.equals("0",      toks[1].value)
    end)
end)

-- ============================================================================
-- NAME tokens
-- ============================================================================

describe("NAME tokens", function()
    it("plain identifier", function()
        local toks = lex("Column")
        assert.equals("NAME",   toks[1].type)
        assert.equals("Column", toks[1].value)
    end)

    it("underscore identifier", function()
        local toks = lex("_my_var")
        assert.equals("NAME",    toks[1].type)
        assert.equals("_my_var", toks[1].value)
    end)

    it("CSS-style hyphenated name", function()
        local toks = lex("corner-radius")
        assert.equals("NAME",          toks[1].type)
        assert.equals("corner-radius", toks[1].value)
    end)

    it("a11y-label hyphenated name", function()
        local toks = lex("a11y-label")
        assert.equals("NAME",       toks[1].type)
        assert.equals("a11y-label", toks[1].value)
    end)
end)

-- ============================================================================
-- Position tracking
-- ============================================================================

describe("position tracking", function()
    it("tracks column on single line", function()
        -- component Foo
        -- ^col=1    ^col=11
        local toks = lex("component Foo")
        assert.equals(1,  toks[1].col)
        assert.equals(11, toks[2].col)
    end)

    it("all tokens on single line have line=1", function()
        local toks = lex("component Foo { }")
        for _, tok in ipairs(toks) do
            assert.equals(1, tok.line)
        end
    end)

    it("line number increments after newline", function()
        local toks = lex("component\nFoo")
        assert.equals(1, toks[1].line)
        assert.equals(2, toks[2].line)
    end)

    it("col resets to 1 after newline", function()
        local toks = lex("component\nFoo")
        assert.equals(1, toks[2].col)
    end)
end)

-- ============================================================================
-- Full component round-trip
-- ============================================================================

describe("full component tokenization", function()
    it("tokenizes a complete component declaration", function()
        local src = [[
component ProfileCard {
  slot name: text;
  slot count: number = 0;
  Column {
    Text { content: @name; }
  }
}
]]
        local toks = lex(src)
        -- Verify overall token count is reasonable
        assert.truthy(#toks > 15)

        -- First token must be COMPONENT
        assert.equals("COMPONENT", toks[1].type)
        assert.equals("component", toks[1].value)

        -- Second token must be NAME = "ProfileCard"
        assert.equals("NAME",        toks[2].type)
        assert.equals("ProfileCard", toks[2].value)

        -- Last token must be EOF
        assert.equals("EOF", toks[#toks].type)
    end)

    it("tokenizes when block correctly", function()
        local src = 'when @active { Text { content: "Online"; } }'
        local toks = lex(src)
        local t = types(toks)
        assert.equals("WHEN", t[1])
        assert.equals("AT",   t[2])
        assert.equals("NAME", t[3])
        assert.equals("LBRACE", t[4])
    end)

    it("tokenizes each block correctly", function()
        local src = 'each @items as item { }'
        local toks = lex(src)
        assert.are.same(
            {"EACH","AT","NAME","AS","NAME","LBRACE","RBRACE"},
            types(toks)
        )
    end)

    it("tokenizes list<text> slot type", function()
        local src = "list<text>"
        local toks = lex(src)
        assert.are.same({"KEYWORD","LANGLE","KEYWORD","RANGLE"}, types(toks))
        assert.equals("list", toks[1].value)
        assert.equals("text", toks[3].value)
    end)

    it("tokenizes property with dimension value", function()
        local src = "padding: 16dp;"
        local toks = lex(src)
        assert.are.same({"NAME","COLON","DIMENSION","SEMICOLON"}, types(toks))
        assert.equals("16dp", toks[3].value)
    end)
end)

-- ============================================================================
-- Error handling
-- ============================================================================

describe("error handling", function()
    it("returns nil, errmsg on unexpected character", function()
        local toks, err = lexer.tokenize("^")
        assert.is_nil(toks)
        assert.is_string(err)
        assert.truthy(err:find("unexpected character"))
    end)

    it("returns nil, errmsg on unterminated string", function()
        local toks, err = lexer.tokenize('"hello')
        assert.is_nil(toks)
        assert.is_string(err)
    end)

    it("returns nil, errmsg on unterminated block comment", function()
        local toks, err = lexer.tokenize("/* not closed")
        assert.is_nil(toks)
        assert.is_string(err)
    end)
end)
