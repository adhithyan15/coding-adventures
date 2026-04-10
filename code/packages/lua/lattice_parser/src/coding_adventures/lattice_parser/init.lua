-- lattice_parser — Builds an AST from Lattice source text using the grammar engine
-- ==================================================================================
--
-- This package is part of the coding-adventures monorepo, the language-tooling
-- layer above lattice_lexer, parser, and grammar_tools.
--
-- # What is Lattice?
--
-- Lattice is a CSS superset language — every valid CSS file is valid Lattice.
-- Lattice adds the following features on top of CSS3:
--
--   Variables:
--     $primary: #4a90d9;
--     color: $primary;
--
--   Mixins (reusable declaration blocks):
--     @mixin button($bg, $fg: white) {
--       background: $bg;
--       color: $fg;
--     }
--     .btn { @include button(red); }
--
--   Control flow:
--     @if $debug { color: red; }
--     @for $i from 1 through 12 { .col-#{$i} { width: ...; } }
--     @each $color in red, green, blue { .text { color: $color; } }
--     @while $i <= 12 { .col { width: $i * 8%; } $i: $i + 1; }
--
--   Functions:
--     @function spacing($n) { @return $n * 8px; }
--     .card { padding: spacing(2); }
--
--   Modules:
--     @use "colors";
--     @use "utils/mixins" as m;
--
--   Nesting:
--     .parent { .child { color: blue; } }
--     .nav { &:hover { color: red; } }
--
--   Placeholder selectors (for @extend):
--     %flex-center { display: flex; align-items: center; }
--     .hero { @extend %flex-center; }
--
-- # What does this parser produce?
--
-- The lexer turns Lattice text into a flat token stream. For example:
--
--   $primary: #333;
--
-- produces tokens:
--
--   VARIABLE("$primary") COLON HASH("#333") SEMICOLON EOF
--
-- This parser turns the token stream into an AST:
--
--   stylesheet
--   └── rule
--       └── lattice_rule
--           └── variable_declaration
--               ├── VARIABLE "$primary"
--               ├── COLON ":"
--               ├── value_list
--               │   └── value
--               │       └── HASH "#333"
--               └── SEMICOLON ";"
--
-- # Grammar
--
-- The Lattice grammar is defined in `code/grammars/lattice.grammar`.
-- The entry point is `stylesheet`.  Grammar highlights:
--
--   stylesheet = { rule } ;
--   rule = lattice_rule | at_rule | qualified_rule ;
--   lattice_rule = variable_declaration | mixin_definition | function_definition
--                | use_directive | lattice_control ;
--   variable_declaration = VARIABLE COLON value_list [ BANG_DEFAULT | BANG_GLOBAL ] SEMICOLON ;
--   mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block
--                    | "@mixin" IDENT block ;
--   include_directive = "@include" FUNCTION [ include_args ] RPAREN ( SEMICOLON | block )
--                     | "@include" IDENT ( SEMICOLON | block ) ;
--   if_directive = "@if" lattice_expression block { "@else" "if" lattice_expression block }
--                  [ "@else" block ] ;
--   qualified_rule = selector_list block ;
--   declaration = property COLON value_list [ priority ] SEMICOLON ;
--
-- # Key design note: NEWLINEs are NOT significant in Lattice/CSS
--
-- Unlike TOML, CSS statement boundaries are marked by SEMICOLON, LBRACE, and
-- RBRACE — not newlines.  The GrammarParser will skip whitespace/newline tokens
-- automatically since the grammar does NOT reference NEWLINE tokens.
--
-- # Architecture
--
-- 1. **Tokenize** — call `lattice_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/lattice_parser/src/coding_adventures/lattice_parser/init.lua
--
-- Walking 6 levels up reaches `code/`, the monorepo root:
--   lattice_parser/        (1)
--   coding_adventures/     (2)
--   src/                   (3)
--   lattice_parser/        (4) — the package directory
--   lua/                   (5)
--   packages/              (6)
--   code/                  → then /grammars/lattice.grammar

local grammar_tools  = require("coding_adventures.grammar_tools")
local lattice_lexer  = require("coding_adventures.lattice_lexer")
local parser_pkg     = require("coding_adventures.parser")

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
-- Uses `debug.getinfo` to retrieve the path Lua recorded at load time.
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
    -- Security: Do not pass the dir string to io.popen (shell injection risk).
    -- Instead, use os.getenv to resolve relative paths -- no subprocess or
    -- shell invocation is involved. The previously removed pattern
    --   io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
    -- was unsafe because dir could contain shell metacharacters.
    -- Fixed: 2026-04-10 security review.
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local cwd = os.getenv("PWD") or os.getenv("CD") or ""
        if cwd ~= "" then
            dir = cwd:gsub("\\\\", "/"):gsub("%c+$", "") .. "/" .. dir
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to ascend.
-- @return string        The ancestor directory.
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

--- Load and parse `lattice.grammar`, with caching.
--
-- On the first call, walks up 6 directory levels from this file to reach
-- `code/`, then opens `grammars/lattice.grammar`.  The parsed grammar is
-- cached so subsequent calls are instant.
--
-- @return ParserGrammar
-- @error  Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/lattice.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "lattice_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "lattice_parser: failed to parse lattice.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Lattice source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `lattice_lexer.tokenize`.
--   2. Loads the Lattice parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "stylesheet"` (the first rule in the
-- Lattice grammar).
--
-- Lattice, like CSS, is NOT newline-sensitive.  Statement boundaries are
-- marked by SEMICOLON, LBRACE, and RBRACE.  The GrammarParser automatically
-- skips WHITESPACE/NEWLINE tokens since the grammar does not reference them.
--
-- @param source string  The Lattice text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local lattice_parser = require("coding_adventures.lattice_parser")
--   local ast = lattice_parser.parse("h1 { color: red; }")
--   -- ast.rule_name  → "stylesheet"
--
--   local ast2 = lattice_parser.parse("$primary: #4a90d9;")
--   -- ast2.rule_name → "stylesheet"
function M.parse(source)
    local tokens = lattice_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("lattice_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Lattice source string without immediately parsing.
--
-- Use this when you need fine-grained control over the parsing step — for
-- example, to inspect the token stream before calling `:parse()`, or to
-- drive parsing incrementally.
--
-- @param source string   The Lattice text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = lattice_parser.create_parser("$x: 1;")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = lattice_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Lattice.
--
-- Useful for inspecting the grammar rules, first-rule name, or number of
-- rules without performing a full parse.
--
-- @return ParserGrammar
function M.get_grammar()
    return get_grammar()
end

return M
