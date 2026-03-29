-- Tests for lattice_lexer
-- =======================
--
-- Comprehensive busted test suite for the Lattice lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Lattice variables: $color, $font-size
--   - Placeholder selectors: %button-base
--   - Numbers: DIMENSION (10px, 1.5em), PERCENTAGE (50%), NUMBER (3.14)
--   - Hash tokens: #ff0000, #abc
--   - At-keywords: @media, @mixin, @if
--   - URL tokens: url(https://example.com)
--   - Function tokens: rgb(, calc(
--   - Identifiers: IDENT (red, serif, auto)
--   - Custom properties: --primary-color
--   - Multi-character operators: ::, ~=, |=, ^=, $=, *=, ==, !=, >=, <=
--   - Single-character delimiters: {, }, (, ), [, ], ;, :, ,, ., +, >, <, etc.
--   - Lattice-specific operators: !default, !global, !
--   - String literals (escapes: none — raw content preserved)
--   - Whitespace and comments are consumed silently
--   - Token positions (line, col) are tracked correctly

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

local lattice_lexer = require("coding_adventures.lattice_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by lattice_lexer.tokenize.
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
-- @param tokens  table  The token list returned by lattice_lexer.tokenize.
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

describe("lattice_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(lattice_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(lattice_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", lattice_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(lattice_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(lattice_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = lattice_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = lattice_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = lattice_lexer.tokenize("   \t\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Lattice variable tokens
-- =========================================================================

describe("VARIABLE tokens", function()
    -- Lattice variables start with $ followed by an identifier.
    -- The $ character never appears in valid CSS, making this unambiguous.
    -- VARIABLE must come before DOLLAR_EQUALS ($=) in priority order.

    it("tokenizes $color", function()
        local tokens = lattice_lexer.tokenize("$color")
        assert.are.equal("VARIABLE", tokens[1].type)
        assert.are.equal("$color", tokens[1].value)
    end)

    it("tokenizes $font-size (hyphenated name)", function()
        local tokens = lattice_lexer.tokenize("$font-size")
        assert.are.equal("VARIABLE", tokens[1].type)
        assert.are.equal("$font-size", tokens[1].value)
    end)

    it("tokenizes $my_var (underscore in name)", function()
        local tokens = lattice_lexer.tokenize("$my_var")
        assert.are.equal("VARIABLE", tokens[1].type)
        assert.are.equal("$my_var", tokens[1].value)
    end)

    it("does NOT tokenize $= as VARIABLE (DOLLAR_EQUALS comes after)", function()
        -- $= is an attribute selector operator, not a variable
        local tokens = lattice_lexer.tokenize("$=")
        assert.are.equal("DOLLAR_EQUALS", tokens[1].type)
    end)
end)

-- =========================================================================
-- Placeholder selector tokens
-- =========================================================================

describe("PLACEHOLDER tokens", function()
    -- Placeholder selectors start with % followed by an identifier.
    -- Must come before PERCENTAGE to avoid %name being tokenized as PERCENTAGE.

    it("tokenizes %button-base", function()
        local tokens = lattice_lexer.tokenize("%button-base")
        assert.are.equal("PLACEHOLDER", tokens[1].type)
        assert.are.equal("%button-base", tokens[1].value)
    end)

    it("tokenizes %flex-center", function()
        local tokens = lattice_lexer.tokenize("%flex-center")
        assert.are.equal("PLACEHOLDER", tokens[1].type)
        assert.are.equal("%flex-center", tokens[1].value)
    end)
end)

-- =========================================================================
-- Numeric tokens
-- =========================================================================

describe("numeric tokens", function()
    -- ORDER IS CRITICAL: DIMENSION > PERCENTAGE > NUMBER
    -- DIMENSION must come first so "10px" is one token, not INT + IDENT.

    it("tokenizes a dimension: 10px", function()
        local tokens = lattice_lexer.tokenize("10px")
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("10px", tokens[1].value)
    end)

    it("tokenizes a dimension with decimal: 1.5em", function()
        local tokens = lattice_lexer.tokenize("1.5em")
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("1.5em", tokens[1].value)
    end)

    it("tokenizes a negative dimension: -2rem", function()
        local tokens = lattice_lexer.tokenize("-2rem")
        assert.are.equal("DIMENSION", tokens[1].type)
        assert.are.equal("-2rem", tokens[1].value)
    end)

    it("tokenizes a percentage: 50%", function()
        local tokens = lattice_lexer.tokenize("50%")
        assert.are.equal("PERCENTAGE", tokens[1].type)
        assert.are.equal("50%", tokens[1].value)
    end)

    it("tokenizes 100%", function()
        local tokens = lattice_lexer.tokenize("100%")
        assert.are.equal("PERCENTAGE", tokens[1].type)
        assert.are.equal("100%", tokens[1].value)
    end)

    it("tokenizes a bare number: 0", function()
        local tokens = lattice_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes a float: 3.14", function()
        local tokens = lattice_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes a negative number: -1", function()
        local tokens = lattice_lexer.tokenize("-1")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("-1", tokens[1].value)
    end)
end)

-- =========================================================================
-- Hash tokens
-- =========================================================================

describe("HASH tokens", function()
    -- Hash tokens represent CSS colour values and ID selectors.
    -- They start with # followed by alphanumeric characters.

    it("tokenizes a 6-digit colour: #ff0000", function()
        local tokens = lattice_lexer.tokenize("#ff0000")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#ff0000", tokens[1].value)
    end)

    it("tokenizes a 3-digit colour: #abc", function()
        local tokens = lattice_lexer.tokenize("#abc")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#abc", tokens[1].value)
    end)

    it("tokenizes an ID selector: #my-button", function()
        local tokens = lattice_lexer.tokenize("#my-button")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#my-button", tokens[1].value)
    end)
end)

-- =========================================================================
-- At-keyword tokens
-- =========================================================================

describe("AT_KEYWORD tokens", function()
    -- CSS at-rules and Lattice at-rules share the AT_KEYWORD token type.
    -- The grammar distinguishes them by matching on the token text.

    it("tokenizes @media", function()
        local tokens = lattice_lexer.tokenize("@media")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@media", tokens[1].value)
    end)

    it("tokenizes @mixin (Lattice)", function()
        local tokens = lattice_lexer.tokenize("@mixin")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@mixin", tokens[1].value)
    end)

    it("tokenizes @include (Lattice)", function()
        local tokens = lattice_lexer.tokenize("@include")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@include", tokens[1].value)
    end)

    it("tokenizes @if (Lattice)", function()
        local tokens = lattice_lexer.tokenize("@if")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@if", tokens[1].value)
    end)

    it("tokenizes @use (Lattice)", function()
        local tokens = lattice_lexer.tokenize("@use")
        assert.are.equal("AT_KEYWORD", tokens[1].type)
        assert.are.equal("@use", tokens[1].value)
    end)
end)

-- =========================================================================
-- URL tokens
-- =========================================================================

describe("URL_TOKEN tokens", function()
    it("tokenizes url(https://example.com)", function()
        local tokens = lattice_lexer.tokenize("url(https://example.com)")
        assert.are.equal("URL_TOKEN", tokens[1].type)
    end)

    it("tokenizes url(/path/to/image.png)", function()
        local tokens = lattice_lexer.tokenize("url(/path/to/image.png)")
        assert.are.equal("URL_TOKEN", tokens[1].type)
    end)
end)

-- =========================================================================
-- Function tokens
-- =========================================================================

describe("FUNCTION tokens", function()
    -- FUNCTION matches a CSS function name followed immediately by (.
    -- Note: the opening paren is part of the token value.

    it("tokenizes rgb(", function()
        local tokens = lattice_lexer.tokenize("rgb(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("rgb(", tokens[1].value)
    end)

    it("tokenizes calc(", function()
        local tokens = lattice_lexer.tokenize("calc(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("calc(", tokens[1].value)
    end)

    it("tokenizes var(", function()
        local tokens = lattice_lexer.tokenize("var(")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("var(", tokens[1].value)
    end)
end)

-- =========================================================================
-- Identifier tokens
-- =========================================================================

describe("IDENT tokens", function()
    -- IDENT matches CSS identifiers — property names, values like 'red',
    -- element selectors, etc.

    it("tokenizes 'red'", function()
        local tokens = lattice_lexer.tokenize("red")
        assert.are.equal("IDENT", tokens[1].type)
        assert.are.equal("red", tokens[1].value)
    end)

    it("tokenizes 'serif'", function()
        local tokens = lattice_lexer.tokenize("serif")
        assert.are.equal("IDENT", tokens[1].type)
        assert.are.equal("serif", tokens[1].value)
    end)

    it("tokenizes 'auto'", function()
        local tokens = lattice_lexer.tokenize("auto")
        assert.are.equal("IDENT", tokens[1].type)
        assert.are.equal("auto", tokens[1].value)
    end)

    it("tokenizes a hyphenated identifier: border-radius", function()
        local tokens = lattice_lexer.tokenize("border-radius")
        assert.are.equal("IDENT", tokens[1].type)
        assert.are.equal("border-radius", tokens[1].value)
    end)
end)

-- =========================================================================
-- Custom property tokens
-- =========================================================================

describe("CUSTOM_PROPERTY tokens", function()
    -- CSS custom properties (CSS variables) start with --.
    -- Must be matched before IDENT so --primary doesn't parse as MINUS MINUS IDENT.

    it("tokenizes --primary-color", function()
        local tokens = lattice_lexer.tokenize("--primary-color")
        assert.are.equal("CUSTOM_PROPERTY", tokens[1].type)
        assert.are.equal("--primary-color", tokens[1].value)
    end)

    it("tokenizes --spacing-md", function()
        local tokens = lattice_lexer.tokenize("--spacing-md")
        assert.are.equal("CUSTOM_PROPERTY", tokens[1].type)
        assert.are.equal("--spacing-md", tokens[1].value)
    end)
end)

-- =========================================================================
-- Multi-character operator tokens
-- =========================================================================

describe("multi-character operator tokens", function()
    -- CSS attribute selector operators
    it("tokenizes ::", function()
        local tokens = lattice_lexer.tokenize("::")
        assert.are.equal("COLON_COLON", tokens[1].type)
        assert.are.equal("::", tokens[1].value)
    end)

    it("tokenizes ~=", function()
        local tokens = lattice_lexer.tokenize("~=")
        assert.are.equal("TILDE_EQUALS", tokens[1].type)
        assert.are.equal("~=", tokens[1].value)
    end)

    it("tokenizes |=", function()
        local tokens = lattice_lexer.tokenize("|=")
        assert.are.equal("PIPE_EQUALS", tokens[1].type)
        assert.are.equal("|=", tokens[1].value)
    end)

    it("tokenizes ^=", function()
        local tokens = lattice_lexer.tokenize("^=")
        assert.are.equal("CARET_EQUALS", tokens[1].type)
        assert.are.equal("^=", tokens[1].value)
    end)

    it("tokenizes $=", function()
        local tokens = lattice_lexer.tokenize("$=")
        assert.are.equal("DOLLAR_EQUALS", tokens[1].type)
        assert.are.equal("$=", tokens[1].value)
    end)

    it("tokenizes *=", function()
        local tokens = lattice_lexer.tokenize("*=")
        assert.are.equal("STAR_EQUALS", tokens[1].type)
        assert.are.equal("*=", tokens[1].value)
    end)

    -- Lattice comparison operators
    it("tokenizes == (EQUALS_EQUALS)", function()
        local tokens = lattice_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
        assert.are.equal("==", tokens[1].value)
    end)

    it("tokenizes != (NOT_EQUALS)", function()
        local tokens = lattice_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
        assert.are.equal("!=", tokens[1].value)
    end)

    it("tokenizes >= (GREATER_EQUALS)", function()
        local tokens = lattice_lexer.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", tokens[1].type)
        assert.are.equal(">=", tokens[1].value)
    end)

    it("tokenizes <= (LESS_EQUALS)", function()
        local tokens = lattice_lexer.tokenize("<=")
        assert.are.equal("LESS_EQUALS", tokens[1].type)
        assert.are.equal("<=", tokens[1].value)
    end)
end)

-- =========================================================================
-- Lattice bang tokens
-- =========================================================================

describe("Lattice bang tokens", function()
    -- !default and !global must come before BANG so they are matched
    -- as whole tokens rather than BANG + IDENT.

    it("tokenizes !default", function()
        local tokens = lattice_lexer.tokenize("!default")
        assert.are.equal("BANG_DEFAULT", tokens[1].type)
        assert.are.equal("!default", tokens[1].value)
    end)

    it("tokenizes !global", function()
        local tokens = lattice_lexer.tokenize("!global")
        assert.are.equal("BANG_GLOBAL", tokens[1].type)
        assert.are.equal("!global", tokens[1].value)
    end)

    it("tokenizes bare ! as BANG", function()
        local tokens = lattice_lexer.tokenize("!")
        assert.are.equal("BANG", tokens[1].type)
        assert.are.equal("!", tokens[1].value)
    end)
end)

-- =========================================================================
-- Delimiter tokens
-- =========================================================================

describe("delimiter tokens", function()
    it("tokenizes { and }", function()
        local tokens = lattice_lexer.tokenize("{}")
        local t = types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes ( and )", function()
        local tokens = lattice_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes [ and ]", function()
        local tokens = lattice_lexer.tokenize("[]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)

    it("tokenizes ;", function()
        local tokens = lattice_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)

    it("tokenizes :", function()
        local tokens = lattice_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
    end)

    it("tokenizes ,", function()
        local tokens = lattice_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
    end)

    it("tokenizes .", function()
        local tokens = lattice_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
    end)

    it("tokenizes &", function()
        local tokens = lattice_lexer.tokenize("&")
        assert.are.equal("AMPERSAND", tokens[1].type)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    -- lattice.tokens declares escapes: none, so STRING values include
    -- the surrounding quotes and any backslash sequences as raw text.
    -- This matches the CSS escape format (\26 vs JSON \n).

    it("tokenizes a double-quoted string", function()
        local tokens = lattice_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('"hello"', tokens[1].value)
    end)

    it("tokenizes a single-quoted string", function()
        local tokens = lattice_lexer.tokenize("'world'")
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal("'world'", tokens[1].value)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = lattice_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('""', tokens[1].value)
    end)
end)

-- =========================================================================
-- Comment handling
-- =========================================================================

describe("comment handling", function()
    it("silently consumes a // line comment", function()
        -- "color // this is a comment" → just IDENT
        local tokens = lattice_lexer.tokenize("color // this is a comment")
        local t = types(tokens)
        assert.are.same({"IDENT"}, t)
    end)

    it("silently consumes a /* block comment */", function()
        local tokens = lattice_lexer.tokenize("color /* block comment */ red")
        local t = types(tokens)
        assert.are.same({"IDENT", "IDENT"}, t)
    end)

    it("does not emit comment text as a token value", function()
        local tokens = lattice_lexer.tokenize("x /* secret */ y")
        for _, tok in ipairs(tokens) do
            assert.is_false(tok.value:find("secret") ~= nil,
                "comment content should not appear in any token value")
        end
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes a variable declaration: $color: #ff0000;", function()
        local tokens = lattice_lexer.tokenize("$color: #ff0000;")
        local t = types(tokens)
        assert.are.same({"VARIABLE", "COLON", "HASH", "SEMICOLON"}, t)
        assert.are.equal("$color", tokens[1].value)
        assert.are.equal("#ff0000", tokens[3].value)
    end)

    it("tokenizes a CSS property: color: red;", function()
        local tokens = lattice_lexer.tokenize("color: red;")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "IDENT", "SEMICOLON"}, t)
    end)

    it("tokenizes a dimension value: margin: 10px 20px;", function()
        local tokens = lattice_lexer.tokenize("margin: 10px 20px;")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "DIMENSION", "DIMENSION", "SEMICOLON"}, t)
    end)

    it("tokenizes a Lattice @if condition: @if $x == 1", function()
        local tokens = lattice_lexer.tokenize("@if $x == 1")
        local t = types(tokens)
        assert.are.same({"AT_KEYWORD", "VARIABLE", "EQUALS_EQUALS", "NUMBER"}, t)
    end)

    it("tokenizes a !default variable: $size: 10px !default;", function()
        local tokens = lattice_lexer.tokenize("$size: 10px !default;")
        local t = types(tokens)
        assert.are.same({"VARIABLE", "COLON", "DIMENSION", "BANG_DEFAULT", "SEMICOLON"}, t)
    end)

    it("tokenizes a @extend with placeholder: @extend %button-base;", function()
        local tokens = lattice_lexer.tokenize("@extend %button-base;")
        local t = types(tokens)
        assert.are.same({"AT_KEYWORD", "PLACEHOLDER", "SEMICOLON"}, t)
    end)

    it("tokenizes a CSS selector block: .foo { color: red; }", function()
        local tokens = lattice_lexer.tokenize(".foo { color: red; }")
        local t = types(tokens)
        assert.are.same({"DOT", "IDENT", "LBRACE", "IDENT", "COLON", "IDENT", "SEMICOLON", "RBRACE"}, t)
    end)

    it("tokenizes an attribute selector: a[href$='pdf']", function()
        local tokens = lattice_lexer.tokenize("a[href$='pdf']")
        local t = types(tokens)
        assert.are.same({"IDENT", "LBRACKET", "IDENT", "DOLLAR_EQUALS", "STRING", "RBRACKET"}, t)
    end)

    it("tokenizes a pseudo-element: ::before", function()
        local tokens = lattice_lexer.tokenize("::before")
        local t = types(tokens)
        assert.are.same({"COLON_COLON", "IDENT"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = lattice_lexer.tokenize("color : red")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "IDENT"}, t)
    end)

    it("strips newlines between tokens", function()
        local tokens = lattice_lexer.tokenize("color\n:\nred")
        local t = types(tokens)
        assert.are.same({"IDENT", "COLON", "IDENT"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: $x: 1px;", function()
        -- $ x  :  _  1  p  x  ;
        -- 1 2  3  4  5  6  7  8   (1-based column)
        -- $x is cols 1-2, : is col 3, 1px is col 5
        local tokens = lattice_lexer.tokenize("$x: 1px;")
        assert.are.equal(1, tokens[1].col)  -- $x
        assert.are.equal(3, tokens[2].col)  -- :
        assert.are.equal(5, tokens[3].col)  -- 1px
    end)

    it("reports line 1 for all tokens on a single-line input", function()
        local tokens = lattice_lexer.tokenize("color: red;")
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
        local tokens = lattice_lexer.tokenize("color")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = lattice_lexer.tokenize("color")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on an unexpected character", function()
        -- The backtick ` is not a valid Lattice/CSS character
        assert.has_error(function()
            lattice_lexer.tokenize("`")
        end)
    end)
end)
