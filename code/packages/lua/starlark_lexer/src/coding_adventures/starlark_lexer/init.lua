-- starlark_lexer — Tokenizes Starlark source using the grammar-driven infrastructure
-- ====================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `starlark.tokens` grammar file to configure the tokenizer.
--
-- # What is Starlark?
--
-- Starlark is a deterministic subset of Python designed for use as a
-- configuration language (famously used in Bazel BUILD files). It is
-- syntactically similar to Python but with significant differences:
--   - No while loops (no general iteration)
--   - No classes or class definitions
--   - No try/except/raise
--   - No global/nonlocal
--   - Significant indentation (like Python)
--   - Certain Python keywords are reserved but disallowed
--
-- # What is Starlark tokenization?
--
-- Given the input:  def foo(x):
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(DEF,    "def",  1:1)
--   Token(NAME,   "foo",  1:5)
--   Token(LPAREN, "(",    1:8)
--   Token(NAME,   "x",    1:9)
--   Token(RPAREN, ")",    1:10)
--   Token(COLON,  ":",    1:11)
--   Token(EOF,    "",     1:12)
--
-- Whitespace between tokens is silently consumed (declared as skip patterns
-- in `starlark.tokens`). The parser never sees ordinary whitespace.
--
-- # Indentation mode
--
-- `starlark.tokens` declares `mode: indentation`, which activates the
-- Python-style INDENT/DEDENT/NEWLINE token emission in the GrammarLexer.
-- This means:
--   - NEWLINE is emitted at each logical line boundary
--   - INDENT is emitted when indentation level increases
--   - DEDENT is emitted (possibly multiple times) when it decreases
--   - INDENT/DEDENT/NEWLINE are suppressed inside (), [], {}
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `starlark.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/starlark_lexer/src/coding_adventures/starlark_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach `code/`, then descend into
-- `grammars/starlark.tokens`.
--
-- Directory structure from script_dir upward:
--   starlark_lexer/    (1) — module dir
--   coding_adventures/ (2)
--   src/               (3)
--   starlark_lexer/    (4) — the package directory
--   lua/               (5)
--   packages/          (6)
--   code/              → then /grammars/starlark.tokens
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   INT     — integer literals (hex, octal, decimal) via -> INT aliases
--   FLOAT   — floating-point literals
--   STRING  — all string variants (single, double, triple, raw, bytes) via -> STRING
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   AND, BREAK, CONTINUE, DEF, ELIF, ELSE, FOR, IF, IN, LAMBDA,
--   LOAD, NOT, OR, PASS, RETURN, TRUE, FALSE, NONE
--
-- Three-character augmented assignment operators:
--   DOUBLE_STAR_EQUALS, LEFT_SHIFT_EQUALS, RIGHT_SHIFT_EQUALS, FLOOR_DIV_EQUALS
--
-- Two-character operators:
--   DOUBLE_STAR, FLOOR_DIV, LEFT_SHIFT, RIGHT_SHIFT,
--   EQUALS_EQUALS, NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS,
--   PLUS_EQUALS, MINUS_EQUALS, STAR_EQUALS, SLASH_EQUALS,
--   PERCENT_EQUALS, AMP_EQUALS, PIPE_EQUALS, CARET_EQUALS
--
-- Single-character operators:
--   PLUS, MINUS, STAR, SLASH, PERCENT, EQUALS,
--   LESS_THAN, GREATER_THAN, AMP, PIPE, CARET, TILDE
--
-- Delimiters:
--   LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE,
--   COMMA, COLON, SEMICOLON, DOT
--
-- Indentation tokens (emitted by mode: indentation):
--   INDENT, DEDENT, NEWLINE
--
-- Reserved words cause a lexer error if used as identifiers.

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
    return dirname(src)
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

--- Load and parse the `starlark.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Starlark token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/starlark_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/starlark_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/starlark.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "starlark_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("starlark_lexer: failed to parse starlark.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Starlark source string.
--
-- Loads the `starlark.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Because `starlark.tokens` uses `mode: indentation`, the GrammarLexer
-- automatically emits INDENT, DEDENT, and NEWLINE tokens at logical line
-- boundaries. INDENT/DEDENT/NEWLINE are suppressed inside (), [], {}.
--
-- Whitespace and comments are consumed silently via the skip patterns in
-- `starlark.tokens`.
--
-- @param source string  The Starlark text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters or reserved
--                       keywords used as identifiers.
--
-- Example:
--
--   local starlark_lexer = require("coding_adventures.starlark_lexer")
--   local tokens = starlark_lexer.tokenize("def foo(x):")
--   -- tokens[1].type  → "DEF"
--   -- tokens[1].value → "def"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "foo"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    return gl:tokenize()
end

--- Return the cached (or freshly loaded) TokenGrammar for Starlark.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Starlark token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
