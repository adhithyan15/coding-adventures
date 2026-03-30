-- css_parser — Builds an AST from CSS3 stylesheets using the grammar-driven engine
-- ==================================================================================
--
-- This package is part of the coding-adventures monorepo. It sits in the
-- language-tooling layer above the css_lexer, parser, and grammar_tools packages.
-- Since css.grammar exists in the repository, this module uses the grammar-driven
-- GrammarParser, following the same pattern as json_parser and toml_parser.
--
-- # What does this parser produce?
--
-- The lexer turns CSS text into a flat token stream:
--
--   IDENT("h1") LBRACE("{") IDENT("color") COLON(":") IDENT("red")
--   SEMICOLON(";") RBRACE("}") EOF
--
-- The parser turns that stream into a tree:
--
--   stylesheet
--   └── rule
--       └── qualified_rule
--           ├── selector_list
--           │   └── complex_selector
--           │       └── compound_selector
--           │           └── simple_selector → IDENT "h1"
--           └── block
--               ├── LBRACE "{"
--               ├── block_contents
--               │   └── block_item
--               │       └── declaration_or_nested
--               │           └── declaration
--               │               ├── property → IDENT "color"
--               │               ├── COLON ":"
--               │               ├── value_list
--               │               │   └── value → IDENT "red"
--               │               └── SEMICOLON ";"
--               └── RBRACE "}"
--
-- # Grammar
--
-- The CSS grammar is defined in `code/grammars/css.grammar`. The entry
-- point is `stylesheet`.
--
--   stylesheet = { rule } ;
--   rule = at_rule | qualified_rule ;
--   at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
--   qualified_rule = selector_list block ;
--   selector_list = complex_selector { COMMA complex_selector } ;
--   complex_selector = compound_selector { [ combinator ] compound_selector } ;
--   block = LBRACE block_contents RBRACE ;
--   block_contents = { block_item } ;
--   block_item = at_rule | declaration_or_nested ;
--   declaration_or_nested = declaration | qualified_rule ;
--   declaration = property COLON value_list [ priority ] SEMICOLON ;
--   value_list = value { value } ;
--   value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH | ... ;
--
-- # Architecture
--
-- 1. **Tokenize** — call `css_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
--
-- # CSS parsing challenges
--
-- CSS is a context-sensitive language in several ways:
--
-- 1. **Declaration vs. nested rule disambiguation** — Both start with IDENT.
--    A declaration: `color: red;`
--    A nested rule:  `div { color: red; }`
--    The css.grammar handles this by trying declaration first (it fails fast
--    if no COLON follows the property name).
--
-- 2. **Compound tokens** — The lexer handles this (10px → DIMENSION). By the
--    time tokens reach the parser, compound units are already unified.
--
-- 3. **Flexible value lists** — CSS values are extraordinarily diverse.
--    `margin: 10px 20px 10px 20px` — four values separated by spaces.
--    `font: 16px/1.5 sans-serif` — slash separator.
--    The grammar accepts any sequence of value-like tokens.
--
-- 4. **At-rules** — @media, @import, @keyframes, etc. all follow the same
--    syntactic structure but have different semantics. The grammar handles
--    them uniformly; semantic analysis happens post-parse.
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/css_parser/src/coding_adventures/css_parser/init.lua
--
-- Walking 6 levels up reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   css_parser/          (1)
--   coding_adventures/   (2)
--   src/                 (3)
--   css_parser/          (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/css.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local css_lexer     = require("coding_adventures.css_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (no trailing slash).
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- When busted runs tests with a relative path containing ".." the
-- dirname-only approach produces a path that collapses to "." after
-- up() steps, so the grammar file cannot be found.  We resolve to an
-- absolute path via "cd <dir> && pwd" to give up() an absolute anchor.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Normalize Windows backslashes to forward slashes for cross-platform
    -- path handling (on Linux/macOS this is a no-op).
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local is_win = package.config:sub(1, 1) == "\\"
        local f
        if is_win then
            f = io.popen('cd /d "' .. dir:gsub("/", "\\") .. '" 2>nul && cd')
        else
            f = io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
        end
        local resolved = f and f:read("*l")
        if f then f:close() end
        if resolved and resolved ~= "" then
            return (resolved:gsub("\\", "/"):gsub("%c+$", ""))
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string
-- @param levels number
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = dirname(result)
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = nil

--- Load and parse `css.grammar`, with caching.
--
-- The CSS grammar is substantially larger than JSON or TOML:
-- ~30 rules covering selectors, combinators, pseudo-classes, pseudo-elements,
-- attribute selectors, declarations, value lists, and at-rules.
-- Caching is important so we parse this only once per process.
--
-- @return ParserGrammar
-- @error  Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/css.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "css_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "css_parser: failed to parse css.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a CSS source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `css_lexer.tokenize`.
--   2. Loads the CSS parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "stylesheet"` (the first rule in
-- the CSS grammar).
--
-- CSS is not newline-sensitive — the GrammarParser auto-detects this from
-- the grammar (no rules reference NEWLINE), so NEWLINE tokens are skipped.
-- In practice, the css_lexer already handles newlines as whitespace skip
-- patterns, so NEWLINE tokens never appear in the token stream.
--
-- @param source string  The CSS text to parse.
-- @return ASTNode       Root of the AST (rule_name == "stylesheet").
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local css_parser = require("coding_adventures.css_parser")
--   local ast = css_parser.parse("h1 { color: red; }")
--   -- ast.rule_name  → "stylesheet"
function M.parse(source)
    local tokens = css_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("css_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a CSS source string without immediately parsing.
--
-- Useful for incremental parsing, partial parsing, or testing individual
-- grammar rules in isolation.
--
-- @param source string   The CSS text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = css_parser.create_parser("h1 { color: red; }")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = css_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for CSS.
--
-- @return ParserGrammar
function M.get_grammar()
    return get_grammar()
end

return M
