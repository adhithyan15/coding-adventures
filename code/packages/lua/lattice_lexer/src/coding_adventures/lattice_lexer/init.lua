-- lattice_lexer — Tokenizes Lattice source using the grammar-driven infrastructure
-- ==================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `lattice.tokens` grammar file to configure the tokenizer.
--
-- # What is Lattice?
--
-- Lattice is a CSS superset language that adds:
--   - Variables:        $color, $font-size
--   - Mixins:           @mixin, @include
--   - Control flow:     @if, @else, @for, @each
--   - Functions:        @function, @return
--   - Modules:          @use
--   - Nesting:          .parent { .child { ... } }
--   - Placeholder selectors: %button-base (for @extend)
--   - Single-line comments: // to end of line
--   - Comparison operators: ==, !=, >=, <= (for @if conditions)
--   - Variable flags:   !default, !global
--
-- Every valid CSS file is valid Lattice. Lattice adds tokens on top of
-- the CSS token set without modifying any existing CSS token behaviour.
--
-- # What is Lattice tokenization?
--
-- Given the input:  $color: #ff0000;
--
-- The lexer produces:
--
--   Token(VARIABLE,   "$color",  1:1)
--   Token(COLON,      ":",       1:7)
--   Token(HASH,       "#ff0000", 1:9)
--   Token(SEMICOLON,  ";",       1:16)
--   Token(EOF,        "",        1:17)
--
-- # Escape handling
--
-- `lattice.tokens` declares `escapes: none`, which tells the GrammarLexer
-- to strip the surrounding quotes from STRING tokens but leave the string
-- content as raw text (no escape-sequence processing). CSS escape sequences
-- use a different format from JSON (\26 vs \n) and are a semantic concern
-- to be handled post-parse, not at the lexer level.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `lattice.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/lattice_lexer/src/coding_adventures/lattice_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach `code/`, then descend into
-- `grammars/lattice.tokens`.
--
-- Directory structure from script_dir upward:
--   lattice_lexer/     (1) — module dir
--   coding_adventures/ (2)
--   src/               (3)
--   lattice_lexer/     (4) — the package directory
--   lua/               (5)
--   packages/          (6)
--   code/              → then /grammars/lattice.tokens
--
-- # Token types produced
--
-- Lattice-specific tokens (new):
--   VARIABLE        — $color, $font-size
--   PLACEHOLDER     — %button-base, %flex-center
--   EQUALS_EQUALS   — ==
--   NOT_EQUALS      — !=
--   GREATER_EQUALS  — >=
--   LESS_EQUALS     — <=
--   BANG_DEFAULT    — !default
--   BANG_GLOBAL     — !global
--
-- Shared with CSS:
--   STRING          — "hello", 'world' (via -> STRING alias)
--   DIMENSION       — 10px, 1.5em, -2rem
--   PERCENTAGE      — 50%, 100%
--   NUMBER          — 3.14, -1, 0
--   HASH            — #ff0000, #abc
--   AT_KEYWORD      — @media, @mixin, @include, @if
--   URL_TOKEN       — url(https://example.com)
--   FUNCTION        — rgb(, calc(, var(
--   CDO             — <!--
--   CDC             — -->
--   UNICODE_RANGE   — U+0041, U+0041-005A
--   CUSTOM_PROPERTY — --primary-color
--   IDENT           — red, serif, auto
--   COLON_COLON     — ::
--   TILDE_EQUALS    — ~=
--   PIPE_EQUALS     — |=
--   CARET_EQUALS    — ^=
--   DOLLAR_EQUALS   — $=
--   STAR_EQUALS     — *=
--   LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET
--   SEMICOLON, COLON, COMMA, DOT
--   PLUS, GREATER, LESS, TILDE, STAR, PIPE
--   BANG, SLASH, EQUALS, AMPERSAND, MINUS

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (without trailing slash).
-- For example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string The full file path.
-- @return string     The directory portion.
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua embeds the source path in the chunk debug info with a leading "@".
-- We strip that prefix to get the real filesystem path.
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
    -- Extract the directory portion of the source path (may be relative
    -- and may contain .. when busted uses ../src in package.path).
    local dir = src:match("(.+)/[^/]+$") or "."
    -- Resolve to an absolute normalised path. Using 'cd dir && pwd' correctly
    -- resolves any .. components -- unlike string-based dirname traversal.
    -- Skip on Windows drive paths (C:\...) and fall back to the raw string.
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
            -- Normalise .. and . segments so dirname-based traversal works
            -- correctly when the source was loaded via a relative package.path
            -- entry (e.g. "../src/?.lua" from a tests/ subdirectory).
            local is_abs = dir:sub(1, 1) == "/"
            local parts = {}
            for seg in dir:gmatch("[^/]+") do
                if seg == ".." then
                    if #parts > 0 then table.remove(parts) end
                elseif seg ~= "." then
                    table.insert(parts, seg)
                end
            end
            dir = (is_abs and "/" or "") .. table.concat(parts, "/")
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- Each call to this function strips one path component.
-- For example: up("/a/b/c", 2) → "/a"
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string        Resulting directory.
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
--
-- The grammar is read from disk exactly once and cached in a module-level
-- variable. Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

local _grammar_cache = nil

--- Load and parse the `lattice.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Lattice token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/lattice_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/lattice_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/lattice.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "lattice_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("lattice_lexer: failed to parse lattice.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Lattice source string.
--
-- Loads the `lattice.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Because `lattice.tokens` declares `escapes: none`, STRING token values
-- retain their surrounding quote characters and any escape sequences as
-- raw text. CSS escape decoding (e.g. \26 → &) is a semantic concern
-- handled after parsing, not at the lexer level.
--
-- Whitespace and comments (// line comments and /* block comments */) are
-- consumed silently via the skip patterns in `lattice.tokens`.
--
-- @param source string  The Lattice text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local lattice_lexer = require("coding_adventures.lattice_lexer")
--   local tokens = lattice_lexer.tokenize("$color: #ff0000;")
--   -- tokens[1].type  → "VARIABLE"
--   -- tokens[1].value → "$color"
--   -- tokens[2].type  → "COLON"
--   -- tokens[3].type  → "HASH"
--   -- tokens[3].value → "#ff0000"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    local raw     = gl:tokenize()
    local tokens  = {}
    for _, tok in ipairs(raw) do
        tokens[#tokens + 1] = {
            type  = tok.type_name,
            value = tok.value,
            line  = tok.line,
            col   = tok.column,
        }
    end
    return tokens
end

--- Return the cached (or freshly loaded) TokenGrammar for Lattice.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Lattice token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
