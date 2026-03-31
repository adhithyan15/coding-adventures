-- Tests for css_lexer
-- ====================
--
-- Comprehensive busted test suite for the CSS lexer package.
--
-- # What we're testing
--
-- CSS tokenization is notably harder than most languages because of:
--
--   1. Compound tokens — "10px" is ONE token (DIMENSION), not two.
--      These tests verify the priority ordering in css.tokens.
--
--   2. Dual-purpose tokens — "#333" is a HASH whether it's a hex color
--      (in a declaration) or an ID selector (in a selector). Both tokenize
--      identically as HASH. Context disambiguation happens in the parser.
--
--   3. Function tokens — "rgba(" is a single FUNCTION token. The opening
--      paren is absorbed into the token value.
--
--   4. At-keywords — "@media" is a single AT_KEYWORD token.
--
--   5. URL tokens — "url(./path)" is a single URL_TOKEN when the path
--      is unquoted.
--
--   6. Error tokens — BAD_STRING for unclosed strings provides graceful
--      degradation rather than hard lexer errors.
--
-- # Test coverage
--   - Module loads and exposes the public API
--   - Empty/whitespace-only input produces only EOF
--   - Selectors: type (h1), class (.class), ID (#id), attribute ([attr])
--   - Declaration properties and values: color: red;
--   - At-rules: @media, @import, @charset, @keyframes
--   - DIMENSION compound tokens: 10px, 1.5em, 100vh, 360deg
--   - PERCENTAGE compound tokens: 50%, 0.5%, 100%
--   - NUMBER bare numbers: 42, 3.14, -0.5
--   - HASH tokens: #333, #ff0000, #header
--   - FUNCTION tokens: rgba(, calc(, linear-gradient(
--   - URL_TOKEN: url(./img.png), url(data:image/png;base64,...)
--   - String literals: "hello", 'world'
--   - Custom properties: --main-color, --bg
--   - COLON_COLON for pseudo-elements: ::before, ::after
--   - Multi-character attribute operators: ~=, |=, ^=, $=, *=
--   - Whitespace is consumed silently (skip patterns)
--   - CSS comments are consumed silently: /* ... */
--   - Multi-line comments span multiple lines
--   - Token position tracking (line, col)
--   - Priority ordering: DIMENSION wins over NUMBER + IDENT

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

local css_lexer = require("coding_adventures.css_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by css_lexer.tokenize.
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
-- @param tokens  table  The token list returned by css_lexer.tokenize.
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

describe("css_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(css_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(css_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", css_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(css_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(css_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = css_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = css_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        -- CSS whitespace is a skip pattern, so spaces/tabs/newlines
        -- between tokens are silently consumed.
        local tokens = css_lexer.tokenize("   \t\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("CSS comment only produces only EOF", function()
        -- /* ... */ is a skip pattern — entire comment is invisible
        local tokens = css_lexer.tokenize("/* just a comment */")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Selector tokens
-- =========================================================================

describe("selector tokens", function()
    -- Type selector: an element name like h1, div, p
    it("tokenizes a type selector: h1", function()
        local tokens = css_lexer.tokenize("h1")
        assert.are.equal("IDENT", tokens[1].type)
        assert.are.equal("h1", tokens[1].value)
    end)

    -- Class selector: the dot is a DOT token, the name is IDENT
    it("tokenizes a class selector: .class", function()
        local tokens = css_lexer.tokenize(".class")
        local t = types(tokens)
        assert.are.same({"DOT", "IDENT"}, t)
        assert.are.equal("class", tokens[2].value)
    end)

    -- ID selector: #header — the whole thing is a HASH token
    -- Note: #header and #333 are both HASH; the parser disambiguates.
    it("tokenizes an ID selector: #header", function()
        local tokens = css_lexer.tokenize("#header")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#header", tokens[1].value)
    end)

    -- Attribute selector: brackets and IDENT inside
    it("tokenizes attribute selector brackets: [attr]", function()
        local tokens = css_lexer.tokenize("[attr]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "IDENT", "RBRACKET"}, t)
        assert.are.equal("attr", tokens[2].value)
    end)

    -- Attribute selector with value: [type="text"]
    it("tokenizes attribute selector with value: [type=\"text\"]", function()
        local tokens = css_lexer.tokenize('[type="text"]')
        local t = types(tokens)
        assert.are.same({"LBRACKET", "IDENT", "EQUALS", "STRING", "RBRACKET"}, t)
    end)

    -- Universal selector
    it("tokenizes universal selector: *", function()
        local tokens = css_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
    end)
end)

-- =========================================================================
-- Declaration tokens
-- =========================================================================

describe("declaration tokens", function()
    -- Basic property: value declaration
    it("tokenizes: color: red;", function()
        local tokens = css_lexer.tokenize("color: red;")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "IDENT", "SEMICOLON"}, t)
        assert.are.equal("color", tokens[1].value)
        assert.are.equal("red", tokens[3].value)
    end)

    -- A full rule with braces
    it("tokenizes: h1 { color: red; }", function()
        local tokens = css_lexer.tokenize("h1 { color: red; }")
        local t = types(tokens)
        assert.are.same({"IDENT", "LBRACE", "IDENT", "COLON", "IDENT", "SEMICOLON", "RBRACE"}, t)
    end)
end)

-- =========================================================================
-- At-keywords
-- =========================================================================

describe("at-keyword tokens", function()
    -- @media is a single AT_KEYWORD token
    it("tokenizes @media as AT_KEYWORD", function()
        local tokens = css_lexer.tokenize("@media")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@media", tokens[1].value)
    end)

    it("tokenizes @import as AT_KEYWORD", function()
        local tokens = css_lexer.tokenize("@import")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@import", tokens[1].value)
    end)

    it("tokenizes @charset as AT_KEYWORD", function()
        local tokens = css_lexer.tokenize("@charset")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@charset", tokens[1].value)
    end)

    it("tokenizes @keyframes as AT_KEYWORD", function()
        local tokens = css_lexer.tokenize("@keyframes")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@keyframes", tokens[1].value)
    end)

    it("tokenizes @font-face as AT_KEYWORD (hyphen in name)", function()
        local tokens = css_lexer.tokenize("@font-face")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@font-face", tokens[1].value)
    end)

    -- A complete at-rule
    it("tokenizes @media rule structure: @media screen { }", function()
        local tokens = css_lexer.tokenize("@media screen { }")
        local t = types(tokens)
        assert.are.same({"AT_KEYWORD", "IDENT", "LBRACE", "RBRACE"}, t)
        assert.are.equal("@media", tokens[1].value)
        assert.are.equal("screen", tokens[2].value)
    end)
end)

-- =========================================================================
-- DIMENSION compound tokens
-- =========================================================================
--
-- This section tests the most critical ordering constraint in CSS tokenization.
-- DIMENSION must be defined BEFORE NUMBER in css.tokens, otherwise "10px"
-- would be two tokens (NUMBER + IDENT) instead of one (DIMENSION).
--
-- Think of it like a greedy match: the lexer tries longer patterns first.

describe("DIMENSION compound tokens (number + unit)", function()
    -- px — the most common CSS unit
    it("tokenizes 10px as a single DIMENSION token", function()
        local tokens = css_lexer.tokenize("10px")
        assert.are.equal(1, #types(tokens))
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("10px", tokens[1].value)
    end)

    -- em — relative to font size
    it("tokenizes 1.5em as DIMENSION", function()
        local tokens = css_lexer.tokenize("1.5em")
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("1.5em", tokens[1].value)
    end)

    -- rem — relative to root font size
    it("tokenizes 2rem as DIMENSION", function()
        local tokens = css_lexer.tokenize("2rem")
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("2rem", tokens[1].value)
    end)

    -- viewport units
    it("tokenizes 100vh as DIMENSION", function()
        local tokens = css_lexer.tokenize("100vh")
        assert.are.equal("DIMENSION", tokens[1].type)
    end)

    it("tokenizes 50vw as DIMENSION", function()
        local tokens = css_lexer.tokenize("50vw")
        assert.are.equal("DIMENSION", tokens[1].type)
    end)

    -- Angle units for transforms and gradients
    it("tokenizes 360deg as DIMENSION", function()
        local tokens = css_lexer.tokenize("360deg")
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("360deg", tokens[1].value)
    end)

    -- Time units for animations
    it("tokenizes 0.3s as DIMENSION", function()
        local tokens = css_lexer.tokenize("0.3s")
        assert.are.equal("DIMENSION", tokens[1].type)
    end)

    it("tokenizes 300ms as DIMENSION", function()
        local tokens = css_lexer.tokenize("300ms")
        assert.are.equal("DIMENSION", tokens[1].type)
    end)
end)

-- =========================================================================
-- PERCENTAGE compound tokens
-- =========================================================================
--
-- PERCENTAGE must come after DIMENSION but before NUMBER.
-- "50%" must not become NUMBER("50") + some literal.

describe("PERCENTAGE compound tokens (number + %)", function()
    it("tokenizes 50% as a single PERCENTAGE token", function()
        local tokens = css_lexer.tokenize("50%")
        assert.are.equal(1, #types(tokens))
        assert.are.equal("PERCENTAGE", tokens[1].type)
        assert.are.equal("50%", tokens[1].value)
    end)

    it("tokenizes 100% as PERCENTAGE", function()
        local tokens = css_lexer.tokenize("100%")
        assert.are.equal("PERCENTAGE", tokens[1].type)
        assert.are.equal("100%", tokens[1].value)
    end)

    it("tokenizes 0.5% as PERCENTAGE", function()
        local tokens = css_lexer.tokenize("0.5%")
        assert.are.equal("PERCENTAGE", tokens[1].type)
    end)

    it("tokenizes 0% as PERCENTAGE", function()
        local tokens = css_lexer.tokenize("0%")
        assert.are.equal("PERCENTAGE", tokens[1].type)
    end)
end)

-- =========================================================================
-- NUMBER tokens (bare numbers)
-- =========================================================================
--
-- Only after checking for DIMENSION and PERCENTAGE will the lexer match
-- a bare number. This ensures ordering correctness.

describe("NUMBER tokens (bare numbers)", function()
    it("tokenizes integer 42 as NUMBER", function()
        local tokens = css_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero (0) as NUMBER", function()
        local tokens = css_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("tokenizes decimal 3.14 as NUMBER", function()
        local tokens = css_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes negative -0.5 as NUMBER", function()
        local tokens = css_lexer.tokenize("-0.5")
        assert.are.equal("NUMBER", tokens[1].type)
    end)
end)

-- =========================================================================
-- HASH tokens
-- =========================================================================
--
-- HASH tokens serve dual duty in CSS:
--   - #fff, #333, #ff0000   → hex color values in declarations
--   - #header, #nav         → ID selectors
--
-- Both produce HASH tokens. The grammar (parser) is responsible for
-- deciding which context applies.

describe("HASH tokens", function()
    it("tokenizes #333 as HASH (hex color)", function()
        local tokens = css_lexer.tokenize("#333")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#333", tokens[1].value)
    end)

    it("tokenizes #ff0000 as HASH (hex color)", function()
        local tokens = css_lexer.tokenize("#ff0000")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#ff0000", tokens[1].value)
    end)

    it("tokenizes #header as HASH (ID selector)", function()
        local tokens = css_lexer.tokenize("#header")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#header", tokens[1].value)
    end)

    it("tokenizes #main-content as HASH (hyphenated ID)", function()
        local tokens = css_lexer.tokenize("#main-content")
        assert.are.equal("HASH", tokens[1].type)
    end)
end)

-- =========================================================================
-- FUNCTION tokens
-- =========================================================================
--
-- In CSS, a function token is an identifier immediately followed by '('.
-- The opening paren is part of the token — the FUNCTION token value
-- includes the paren: "rgba(" not "rgba".
--
-- This is because css.tokens defines FUNCTION as:
--   /-?[a-zA-Z_][a-zA-Z0-9_-]*\(/
--
-- FUNCTION must come before IDENT in the token grammar, otherwise
-- "rgba(" would match as IDENT("rgba") + LPAREN("(").

describe("FUNCTION tokens (identifier + opening paren)", function()
    it("tokenizes rgba( as FUNCTION", function()
        local tokens = css_lexer.tokenize("rgba(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("rgba(", tokens[1].value)
    end)

    it("tokenizes calc( as FUNCTION", function()
        local tokens = css_lexer.tokenize("calc(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("calc(", tokens[1].value)
    end)

    it("tokenizes linear-gradient( as FUNCTION (hyphenated)", function()
        local tokens = css_lexer.tokenize("linear-gradient(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("linear-gradient(", tokens[1].value)
    end)

    it("tokenizes var( as FUNCTION", function()
        local tokens = css_lexer.tokenize("var(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("var(", tokens[1].value)
    end)

    -- A complete function call tokenizes as: FUNCTION, args..., RPAREN
    it("tokenizes rgb(255, 0, 0) as FUNCTION + args + RPAREN", function()
        local tokens = css_lexer.tokenize("rgb(255, 0, 0)")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("rgb(", tokens[1].value)
        -- Last meaningful token before EOF should be RPAREN
        local last_real = tokens[#tokens - 1]
        assert.are.equal("RPAREN", last_real.type)
    end)
end)

-- =========================================================================
-- URL_TOKEN
-- =========================================================================
--
-- url() with an unquoted path is a single URL_TOKEN. It must come before
-- FUNCTION in the token grammar, otherwise "url(./image.png)" would be
-- tokenized as FUNCTION("url(") + IDENT(".") + ... which is wrong.
--
-- Note: url("./image.png") with a quoted path is tokenized differently:
-- FUNCTION("url(") + STRING + RPAREN.

describe("URL_TOKEN (unquoted url)", function()
    it("tokenizes url(./image.png) as URL_TOKEN", function()
        local tokens = css_lexer.tokenize("url(./image.png)")
        assert.are.equal("URL_TOKEN", tokens[1].type)
        assert.are.equal("url(./image.png)", tokens[1].value)
    end)

    it("tokenizes url(data:image/png) as URL_TOKEN", function()
        local tokens = css_lexer.tokenize("url(data:image/png)")
        assert.are.equal("URL_TOKEN", tokens[1].type)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    -- CSS supports both double-quoted and single-quoted strings.
    -- Both variants emit STRING (via the -> alias in css.tokens).

    it("tokenizes a double-quoted string", function()
        local tokens = css_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes a single-quoted string", function()
        local tokens = css_lexer.tokenize("'world'")
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal("world", tokens[1].value)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = css_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('', tokens[1].value)
    end)

    it("tokenizes a string with escape sequence", function()
        -- CSS escapes: \22 for ", \26 for &. escapes: none means the
        -- raw escape sequence is preserved as-is.
        local tokens = css_lexer.tokenize('"a\\26b"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Custom properties (CSS variables)
-- =========================================================================
--
-- CSS custom properties start with -- and are called "CSS variables":
--   --main-color: #333;
--   color: var(--main-color);
--
-- CUSTOM_PROPERTY must come before IDENT in the grammar because IDENT
-- can start with a hyphen (for vendor prefixes like -webkit-transform),
-- and the -- prefix would match incorrectly.

describe("custom property tokens (CSS variables)", function()
    it("tokenizes --main-color as CUSTOM_PROPERTY", function()
        local tokens = css_lexer.tokenize("--main-color")
        assert.are.equal("CUSTOM_PROPERTY", tokens[1].type)
        assert.are.equal("--main-color", tokens[1].value)
    end)

    it("tokenizes --bg as CUSTOM_PROPERTY", function()
        local tokens = css_lexer.tokenize("--bg")
        assert.are.equal("CUSTOM_PROPERTY", tokens[1].type)
    end)
end)

-- =========================================================================
-- Pseudo-element tokens (::before, ::after)
-- =========================================================================
--
-- CSS pseudo-elements use :: (double colon). The lexer must emit
-- COLON_COLON, not two COLON tokens. This is critical because the
-- parser expects a single COLON_COLON token type.

describe("pseudo-element tokens (double colon)", function()
    it("tokenizes :: as COLON_COLON (not two COLONs)", function()
        local tokens = css_lexer.tokenize("::")
        assert.are.equal(1, #types(tokens))
        assert.are.equal("COLON_COLON", tokens[1].type)
        assert.are.equal("::", tokens[1].value)
    end)

    it("tokenizes ::before as COLON_COLON + IDENT", function()
        local tokens = css_lexer.tokenize("::before")
        local t = types(tokens)
        assert.are.same({"COLON_COLON", "IDENT"}, t)
        assert.are.equal("before", tokens[2].value)
    end)

    it("tokenizes ::after as COLON_COLON + IDENT", function()
        local tokens = css_lexer.tokenize("::after")
        local t = types(tokens)
        assert.are.same({"COLON_COLON", "IDENT"}, t)
    end)
end)

-- =========================================================================
-- Multi-character attribute operators
-- =========================================================================
--
-- Attribute selectors support several operators: ~=, |=, ^=, $=, *=
-- Each must come before its single-character prefix (=, ~, |, ^, $, *)
-- so they are matched as a unit.

describe("multi-character attribute operators", function()
    it("tokenizes ~= as TILDE_EQUALS", function()
        local tokens = css_lexer.tokenize("~=")
        assert.are.equal("TILDE_EQUALS", tokens[1].type)
    end)

    it("tokenizes |= as PIPE_EQUALS", function()
        local tokens = css_lexer.tokenize("|=")
        assert.are.equal("PIPE_EQUALS", tokens[1].type)
    end)

    it("tokenizes ^= as CARET_EQUALS", function()
        local tokens = css_lexer.tokenize("^=")
        assert.are.equal("CARET_EQUALS", tokens[1].type)
    end)

    it("tokenizes $= as DOLLAR_EQUALS", function()
        local tokens = css_lexer.tokenize("$=")
        assert.are.equal("DOLLAR_EQUALS", tokens[1].type)
    end)

    it("tokenizes *= as STAR_EQUALS", function()
        local tokens = css_lexer.tokenize("*=")
        assert.are.equal("STAR_EQUALS", tokens[1].type)
    end)
end)

-- =========================================================================
-- Single-character delimiters
-- =========================================================================

describe("single-character delimiter tokens", function()
    it("tokenizes { as LBRACE", function()
        local tokens = css_lexer.tokenize("{")
        assert.are.equal("LBRACE", tokens[1].type)
    end)

    it("tokenizes } as RBRACE", function()
        local tokens = css_lexer.tokenize("}")
        assert.are.equal("RBRACE", tokens[1].type)
    end)

    it("tokenizes ( as LPAREN", function()
        local tokens = css_lexer.tokenize("(")
        assert.are.equal("LPAREN", tokens[1].type)
    end)

    it("tokenizes ) as RPAREN", function()
        local tokens = css_lexer.tokenize(")")
        assert.are.equal("RPAREN", tokens[1].type)
    end)

    it("tokenizes [ as LBRACKET", function()
        local tokens = css_lexer.tokenize("[")
        assert.are.equal("LBRACKET", tokens[1].type)
    end)

    it("tokenizes ] as RBRACKET", function()
        local tokens = css_lexer.tokenize("]")
        assert.are.equal("RBRACKET", tokens[1].type)
    end)

    it("tokenizes ; as SEMICOLON", function()
        local tokens = css_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)

    it("tokenizes : as COLON", function()
        local tokens = css_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
    end)

    it("tokenizes , as COMMA", function()
        local tokens = css_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
    end)

    it("tokenizes > as GREATER", function()
        local tokens = css_lexer.tokenize(">")
        assert.are.equal("GREATER", tokens[1].type)
    end)

    it("tokenizes + as PLUS", function()
        local tokens = css_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
    end)

    it("tokenizes ~ as TILDE", function()
        local tokens = css_lexer.tokenize("~")
        assert.are.equal("TILDE", tokens[1].type)
    end)

    it("tokenizes . as DOT", function()
        local tokens = css_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
    end)

    it("tokenizes ! as BANG", function()
        local tokens = css_lexer.tokenize("!")
        assert.are.equal("BANG", tokens[1].type)
    end)
end)

-- =========================================================================
-- Composite CSS snippets
-- =========================================================================

describe("composite CSS snippets", function()
    -- A typical media query
    it("tokenizes @media query structure", function()
        local tokens = css_lexer.tokenize("@media screen and (min-width: 768px)")
        local t = types(tokens)
        -- @media  screen  and  (  min-width  :  768px  )
        assert.are.equal("AT_KEYWORD", t[1])
        assert.are.equal("IDENT", t[2])   -- screen
        assert.are.equal("IDENT", t[3])   -- and
        assert.are.equal("LPAREN", t[4])
        assert.are.equal("IDENT", t[5])   -- min-width (hyphenated ident)
        assert.are.equal("COLON", t[6])
        assert.are.equal("DIMENSION", t[7]) -- 768px is ONE token
        assert.are.equal("RPAREN", t[8])
    end)

    -- Declaration block with font-size
    it("tokenizes font-size declaration", function()
        local tokens = css_lexer.tokenize("font-size: 16px;")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "DIMENSION", "SEMICOLON"}, t)
        assert.are.equal("font-size", tokens[1].value)
        assert.are.equal("16px", tokens[3].value)
    end)

    -- rgba() color function
    it("tokenizes rgba(255, 0, 0, 0.5) color", function()
        local tokens = css_lexer.tokenize("rgba(255, 0, 0, 0.5)")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("rgba(", tokens[1].value)
        -- last real token is RPAREN
        local last_real = tokens[#tokens - 1]
        assert.are.equal("RPAREN", last_real.type)
    end)

    -- calc() with mixed units
    it("tokenizes calc(100% - 20px) mixed expression", function()
        local tokens = css_lexer.tokenize("calc(100% - 20px)")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("calc(", tokens[1].value)
        assert.are.equal("PERCENTAGE", tokens[2].type) -- 100%
        assert.are.equal("MINUS", tokens[3].type)
        assert.are.equal("DIMENSION", tokens[4].type)  -- 20px
        assert.are.equal("RPAREN", tokens[5].type)
    end)

    -- !important declaration modifier
    it("tokenizes !important", function()
        local tokens = css_lexer.tokenize("!important")
        local t = types(tokens)
        assert.are.same({"BANG", "IDENT"}, t)
        assert.are.equal("important", tokens[2].value)
    end)

    -- CSS variable usage: var(--name)
    it("tokenizes var(--main-color)", function()
        local tokens = css_lexer.tokenize("var(--main-color)")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("CUSTOM_PROPERTY", tokens[2].type)
        assert.are.equal("--main-color", tokens[2].value)
    end)

    -- CSS nesting selector: & .child
    it("tokenizes CSS nesting selector: & .child", function()
        local tokens = css_lexer.tokenize("& .child")
        local t = types(tokens)
        assert.are.same({"AMPERSAND", "DOT", "IDENT"}, t)
    end)

    -- Child combinator: div > p
    it("tokenizes child combinator: div > p", function()
        local tokens = css_lexer.tokenize("div > p")
        local t = types(tokens)
        assert.are.same({"IDENT", "GREATER", "IDENT"}, t)
    end)

    -- Adjacent sibling combinator: h1 + p
    it("tokenizes adjacent sibling: h1 + p", function()
        local tokens = css_lexer.tokenize("h1 + p")
        local t = types(tokens)
        assert.are.same({"IDENT", "PLUS", "IDENT"}, t)
    end)
end)

-- =========================================================================
-- Whitespace and comment handling
-- =========================================================================

describe("whitespace and comment handling", function()
    it("strips spaces between tokens", function()
        local tokens = css_lexer.tokenize("h1 { }")
        local t = types(tokens)
        assert.are.same({"IDENT", "LBRACE", "RBRACE"}, t)
    end)

    it("strips tabs between tokens", function()
        local tokens = css_lexer.tokenize("h1\t{\t}")
        local t = types(tokens)
        assert.are.same({"IDENT", "LBRACE", "RBRACE"}, t)
    end)

    it("strips newlines between tokens", function()
        local tokens = css_lexer.tokenize("h1\n{\n}")
        local t = types(tokens)
        assert.are.same({"IDENT", "LBRACE", "RBRACE"}, t)
    end)

    -- Multi-line CSS comments are a skip pattern
    it("strips /* single-line comment */", function()
        local tokens = css_lexer.tokenize("/* comment */ color")
        local t = types(tokens)
        assert.are.same({"IDENT"}, t)
        assert.are.equal("color", tokens[1].value)
    end)

    it("strips /* multi-line comment */", function()
        local tokens = css_lexer.tokenize("/* line1\nline2\nline3 */ color")
        local t = types(tokens)
        assert.are.same({"IDENT"}, t)
    end)

    it("strips multiple comments between tokens", function()
        local tokens = css_lexer.tokenize("/* a */ color /* b */: /* c */ red")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "IDENT"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: h1 { }", function()
        local tokens = css_lexer.tokenize("h1 { }")
        -- h1 at col 1, { at col 4, } at col 6
        assert.are.equal(1, tokens[1].col)   -- h1
        assert.are.equal(4, tokens[2].col)   -- {
        assert.are.equal(6, tokens[3].col)   -- }
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = css_lexer.tokenize("color: red;")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)

    it("tracks line numbers across newlines", function()
        local tokens = css_lexer.tokenize("h1\n{\ncolor: red;\n}")
        -- h1=line1, {=line2, color=line3, ...
        assert.are.equal(1, tokens[1].line)  -- h1
        assert.are.equal(2, tokens[2].line)  -- {
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = css_lexer.tokenize("h1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = css_lexer.tokenize("h1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Priority ordering verification
-- =========================================================================
--
-- These tests specifically verify the compound-token ordering constraints
-- that make CSS tokenization unique. They are the most important correctness
-- checks in this suite.

describe("token priority ordering (CSS-specific)", function()
    -- DIMENSION before NUMBER: "10px" must be one token
    it("DIMENSION wins over NUMBER + IDENT for 10px", function()
        local tokens = css_lexer.tokenize("10px")
        assert.are.equal(1, #types(tokens), "10px must be ONE token, not two")
        assert.are.equal("DIMENSION", tokens[1].type)
    end)

    -- PERCENTAGE before NUMBER: "50%" must be one token
    it("PERCENTAGE wins over NUMBER for 50%", function()
        local tokens = css_lexer.tokenize("50%")
        assert.are.equal(1, #types(tokens), "50% must be ONE token")
        assert.are.equal("PERCENTAGE", tokens[1].type)
    end)

    -- FUNCTION before IDENT: "rgba(" must be one token
    it("FUNCTION wins over IDENT for rgba(", function()
        local tokens = css_lexer.tokenize("rgba(")
        assert.are.equal(1, #types(tokens), "rgba( must be ONE token")
        assert.are.equal("FUNCTION", tokens[1].type)
    end)

    -- COLON_COLON before COLON: "::" must be one token
    it("COLON_COLON wins over two COLONs for ::", function()
        local tokens = css_lexer.tokenize("::")
        assert.are.equal(1, #types(tokens), ":: must be ONE token")
        assert.are.equal("COLON_COLON", tokens[1].type)
    end)

    -- CUSTOM_PROPERTY before IDENT: "--var" must be CUSTOM_PROPERTY
    it("CUSTOM_PROPERTY wins over IDENT for --var-name", function()
        local tokens = css_lexer.tokenize("--var-name")
        assert.are.equal(1, #types(tokens), "--var-name must be ONE token")
        assert.are.equal("CUSTOM_PROPERTY", tokens[1].type)
    end)
end)
