-- css_lexer — Tokenizes CSS source using the grammar-driven infrastructure
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `css.tokens` grammar file to configure the tokenizer.
--
-- # What is CSS tokenization?
--
-- CSS tokenization is substantially harder than most languages because it
-- uses *compound tokens* — single lexical units made from multiple character
-- classes that would be separate tokens in other languages.
--
-- Consider: `10px`
--   In Python:   NUMBER("10")  NAME("px")  — two tokens
--   In CSS:      DIMENSION("10px")         — one token
--
-- Other compound token challenges:
--
--   50%      → PERCENTAGE   (number + percent sign, not NUMBER + literal)
--   @media   → AT_KEYWORD   (@ is not an operator here)
--   #333     → HASH         (could be ID selector or hex color — both HASH)
--   rgba(    → FUNCTION     (identifier + lparen are a single token in CSS)
--   url(./x) → URL_TOKEN    (the whole url() is one token if unquoted)
--
-- # Token ordering is critical
--
-- The `css.tokens` grammar uses first-match-wins priority ordering. The most
-- important ordering constraints are:
--
--   1. DIMENSION before PERCENTAGE before NUMBER
--      "10px" must not become NUMBER("10") + IDENT("px")
--      "50%" must not become NUMBER("50") + PERCENT_LITERAL
--
--   2. URL_TOKEN before FUNCTION
--      "url(./image.png)" must not become FUNCTION("url(") + ...
--
--   3. FUNCTION before IDENT
--      "rgba(" must not become IDENT("rgba") + LPAREN("(")
--
--   4. COLON_COLON before COLON
--      "::before" must not become COLON + COLON + ...
--
--   5. CUSTOM_PROPERTY before IDENT
--      "--main-color" must not become MINUS + MINUS + ...
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `css.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/css_lexer/src/coding_adventures/css_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/css.tokens`.
--
-- Directory structure from script_dir upward:
--   css_lexer/           (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   css_lexer/           (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/css.tokens
--
-- # escapes: none
--
-- The `css.tokens` grammar declares `escapes: none`. CSS uses a different
-- escape format (\26 for hex values, not JSON-style \uXXXX). The lexer
-- preserves escape sequences as raw text; CSS escape decoding is a semantic
-- concern handled post-parse.
--
-- # Token types produced
--
-- Compound tokens (the unique CSS challenge):
--   DIMENSION   — number + unit: 10px, 1.5em, 100vh, 360deg
--   PERCENTAGE  — number + percent: 50%, 100%, 0.5%
--   AT_KEYWORD  — @identifier: @media, @import, @keyframes, @charset
--   HASH        — #identifier: #333, #header, #ff0000, #nav
--   FUNCTION    — identifier(: rgba(, calc(, linear-gradient(, var(
--   URL_TOKEN   — url(unquoted-path): url(./img.png), url(data:...)
--
-- Simple tokens:
--   NUMBER      — bare number: 42, 3.14, -0.5, 1e10
--   STRING      — quoted string: "hello", 'world'
--   IDENT       — identifier: color, sans-serif, block, -webkit-box
--   CUSTOM_PROPERTY — CSS variable: --main-color, --bg-color
--   UNICODE_RANGE   — U+XXXX: U+0025-00FF, U+4??
--   CDO, CDC    — legacy HTML comment delimiters: <!--, -->
--
-- Operators and delimiters:
--   COLON_COLON, TILDE_EQUALS, PIPE_EQUALS, CARET_EQUALS,
--   DOLLAR_EQUALS, STAR_EQUALS,
--   LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET,
--   SEMICOLON, COLON, COMMA, DOT, PLUS, GREATER, TILDE,
--   STAR, PIPE, BANG, SLASH, EQUALS, AMPERSAND, MINUS
--
-- Error tokens (graceful degradation):
--   BAD_STRING  — unclosed string: "hello
--   BAD_URL     — unclosed url(): url(./path

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
    if dir:sub(2, 2) ~= ":" then
        local f = io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
        local resolved = f and f:read("*l")
        if f then f:close() end
        if resolved and resolved ~= "" then
            return resolved
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
--
-- CSS grammars are larger than most (many token types with complex ordering
-- constraints), so caching is especially important for performance.

local _grammar_cache = nil

--- Load and parse the `css.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed CSS token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/css_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/css_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/css.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "css_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("css_lexer: failed to parse css.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a CSS source string.
--
-- Loads the `css.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- The tokenizer handles all CSS3 token types including compound tokens
-- (DIMENSION, PERCENTAGE), special tokens (AT_KEYWORD, HASH, FUNCTION,
-- URL_TOKEN), and error tokens (BAD_STRING, BAD_URL) for graceful
-- degradation of malformed CSS input.
--
-- # Token ordering guarantee
--
-- The `css.tokens` grammar specifies DIMENSION before PERCENTAGE before
-- NUMBER, ensuring that "10px" always produces a single DIMENSION token
-- rather than NUMBER + IDENT. Similarly, URL_TOKEN comes before FUNCTION
-- so that url(./path) is a single token when unquoted.
--
-- @param source string  The CSS text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local css_lexer = require("coding_adventures.css_lexer")
--   local tokens = css_lexer.tokenize("h1 { color: red; }")
--   -- tokens[1].type  → "IDENT"
--   -- tokens[1].value → "h1"
--   -- tokens[2].type  → "LBRACE"
--   -- tokens[3].type  → "IDENT"   (color)
--   -- tokens[4].type  → "COLON"
--   -- tokens[5].type  → "IDENT"   (red)
--   -- tokens[6].type  → "SEMICOLON"
--   -- tokens[7].type  → "RBRACE"
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

--- Return the cached (or freshly loaded) TokenGrammar for CSS.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed CSS token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
