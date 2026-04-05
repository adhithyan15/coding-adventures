-- ecmascript_es1_lexer — Tokenizes ECMAScript 1 (1997) source code
-- ================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `ecmascript/es1.tokens` grammar file to configure the tokenizer.
--
-- # What is ECMAScript 1?
--
-- ECMAScript 1 (ECMA-262, 1st Edition, June 1997) was the very first
-- standardized version of JavaScript. Brendan Eich created the language
-- for Netscape Navigator in 1995; two years later ECMA International
-- published this specification.
--
-- ES1 defines:
--   - 23 keywords (break, case, continue, default, delete, do, else, for,
--     function, if, in, new, return, switch, this, typeof, var, void,
--     while, with, true, false, null)
--   - Basic operators: arithmetic, bitwise, logical, comparison, assignment
--   - String literals (single and double quoted)
--   - Numeric literals (decimal, float, hex with 0x prefix, scientific)
--   - The $ character is valid in identifiers
--
-- ES1 does NOT have:
--   - === or !== (strict equality — added in ES3)
--   - try/catch/finally/throw (error handling — added in ES3)
--   - Regular expression literals (formalized in ES3)
--   - Template literals, arrow functions, let/const (added in ES2015)
--
-- # How tokenization works
--
-- Given the input:  var x = 42;
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(VAR,       "var", 1:1)
--   Token(NAME,      "x",   1:5)
--   Token(EQUALS,    "=",   1:7)
--   Token(NUMBER,    "42",  1:9)
--   Token(SEMICOLON, ";",   1:11)
--   Token(EOF,       "",    1:12)
--
-- Whitespace and comments are silently consumed (declared as skip patterns
-- in `es1.tokens`). The parser never sees them.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `ecmascript/es1.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/ecmascript_es1_lexer/src/coding_adventures/ecmascript_es1_lexer/init.lua
--
-- Directory structure from script_dir upward:
--   ecmascript_es1_lexer/  (1) — module dir
--   coding_adventures/     (2)
--   src/                   (3)
--   ecmascript_es1_lexer/  (4) — the package directory
--   lua/                   (5)
--   packages/              (6)
--   code/                  → then /grammars/ecmascript/es1.tokens

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

--- Load and parse the `ecmascript/es1.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed ES1 token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/ecmascript_es1_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/ecmascript_es1_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/ecmascript/es1.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "ecmascript_es1_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    -- Lua's regex engine does not support \v (vertical tab) or \f (form feed)
    -- as escape sequences inside character classes. The ES token grammars use
    -- /[ \t\r\n\v\f]+/ for whitespace skip patterns. We replace \v and \f
    -- with their actual control characters so the pattern works correctly
    -- in Lua without accidentally matching the letters 'v' and 'f'.
    content = content:gsub("\\v", "\x0B")
    content = content:gsub("\\f", "\x0C")

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("ecmascript_es1_lexer: failed to parse es1.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an ECMAScript 1 source string.
--
-- Loads the `ecmascript/es1.tokens` grammar (cached after first call) and
-- feeds the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in `es1.tokens`.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source string  The ECMAScript 1 text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local es1_lexer = require("coding_adventures.ecmascript_es1_lexer")
--   local tokens = es1_lexer.tokenize("var x = 1;")
--   -- tokens[1].type  → "VAR"
--   -- tokens[1].value → "var"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "x"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    local raw     = gl:tokenize()
    local tokens  = {}
    for _, tok in ipairs(raw) do
        if tok.type_name ~= "NEWLINE" then
            tokens[#tokens + 1] = {
                type  = tok.type_name,
                value = tok.value,
                line  = tok.line,
                col   = tok.column,
            }
        end
    end
    return tokens
end

--- Return the cached (or freshly loaded) TokenGrammar for ECMAScript 1.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed ECMAScript 1 token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
