-- javascript_lexer — Tokenizes JavaScript source using the grammar-driven infrastructure
-- ======================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar file to configure the tokenizer.
--
-- # What is JavaScript tokenization?
--
-- Given the input:  const x = 42;
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(CONST,      "const", 1:1)
--   Token(NAME,       "x",     1:7)
--   Token(EQUALS,     "=",     1:9)
--   Token(NUMBER,     "42",    1:11)
--   Token(SEMICOLON,  ";",     1:13)
--   Token(EOF,        "",      1:14)
--
-- Whitespace and comments are silently consumed (declared as skip patterns
-- in the grammar file). The parser never sees them.
--
-- # Architecture
--
-- This module:
--   1. Locates the correct grammar file in `code/grammars/`.
--   2. Reads and parses it once per version (cached) using
--      `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Version routing
--
-- When `version` is nil or "" → loads `code/grammars/javascript.tokens`
-- When `version` is "es1"     → loads `code/grammars/ecmascript/es1.tokens`
-- When `version` is "es2015"  → loads `code/grammars/ecmascript/es2015.tokens`
-- ... etc.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/javascript_lexer/src/coding_adventures/javascript_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/`.
--
-- Directory structure from script_dir upward:
--   javascript_lexer/    (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   javascript_lexer/    (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/...
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   NUMBER  — integer literals (e.g. 42, 0xFF)
--   STRING  — double-quoted string literals
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN,
--   CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF,
--   TRUE, FALSE, NULL, UNDEFINED
--
-- Operators and delimiters:
--   STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS,
--   LESS_EQUALS, GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS, STAR,
--   SLASH, LESS_THAN, GREATER_THAN, BANG,
--   LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET,
--   COMMA, COLON, SEMICOLON, DOT

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid ECMAScript / JavaScript versions
-- =========================================================================
--
-- JavaScript is standardized as ECMAScript. The version strings we accept
-- match the grammar files under code/grammars/ecmascript/:
--
--   "es1"    — ECMAScript 1  (1997): original standardization.
--   "es3"    — ECMAScript 3  (1999): try/catch, regex literals.
--   "es5"    — ECMAScript 5  (2009): strict mode, JSON, Array extras.
--   "es2015" — ECMAScript 6  (2015): let/const, arrow functions, classes.
--   "es2016" — ECMAScript 7  (2016): exponentiation operator.
--   "es2017" — ECMAScript 8  (2017): async/await.
--   "es2018" — ECMAScript 9  (2018): rest/spread properties.
--   "es2019" — ECMAScript 10 (2019): flat, flatMap.
--   "es2020" — ECMAScript 11 (2020): nullish coalescing, optional chaining.
--   "es2021" — ECMAScript 12 (2021): logical assignment, numeric separators.
--   "es2022" — ECMAScript 13 (2022): class fields, top-level await.
--   "es2023" — ECMAScript 14 (2023): array findLast, change array by copy.
--   "es2024" — ECMAScript 15 (2024): Object.groupBy, Promise.withResolvers.
--   "es2025" — ECMAScript 16 (2025): import attributes, RegExp.escape.
--   nil / "" — Generic JavaScript (uses the unified javascript.tokens grammar).

local VALID_JS_VERSIONS = {
    ["es1"]    = true, ["es3"]    = true, ["es5"]    = true,
    ["es2015"] = true, ["es2016"] = true, ["es2017"] = true,
    ["es2018"] = true, ["es2019"] = true, ["es2020"] = true,
    ["es2021"] = true, ["es2022"] = true, ["es2023"] = true,
    ["es2024"] = true, ["es2025"] = true,
}

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
-- Grammars are cached per version string. The cache key is the version
-- string (or "" for generic). This avoids repeated file I/O while still
-- allowing different versions to load different grammar files.

local _grammar_cache = {}

--- Resolve the path to the correct .tokens grammar file for a given version.
--
-- Version routing logic:
--   - nil or ""  →  code/grammars/javascript.tokens       (generic)
--   - "es1"      →  code/grammars/ecmascript/es1.tokens
--   - "es2015"   →  code/grammars/ecmascript/es2015.tokens
--   - ...etc.
--
-- If an unrecognized version string is passed, we raise an error immediately.
--
-- @param version string|nil  The ECMAScript version tag, or nil/empty for generic.
-- @return string             Absolute path to the grammar .tokens file.
local function resolve_tokens_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    -- Generic (no version specified) — use the unified grammar.
    if not version or version == "" then
        return repo_root .. "/grammars/javascript.tokens"
    end

    -- Validate the version string before building the path.
    if not VALID_JS_VERSIONS[version] then
        error(
            "javascript_lexer: unknown ECMAScript version '" .. version .. "'. " ..
            "Valid values are: es1, es3, es5, es2015..es2025, or nil/\"\" for generic."
        )
    end

    return repo_root .. "/grammars/ecmascript/" .. version .. ".tokens"
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- On the first call for a given version, opens the file, parses it with
-- `grammar_tools.parse_token_grammar`, and stores the result in _grammar_cache.
-- On subsequent calls for the same version, returns the cached object immediately.
--
-- @param version string|nil  The ECMAScript version tag (see resolve_tokens_path).
-- @return TokenGrammar       The parsed JavaScript token grammar.
local function get_grammar(version)
    local key = version or ""
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local tokens_path = resolve_tokens_path(version)

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "javascript_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    -- Some ECMAScript grammar files use \v and \f in whitespace patterns.
    -- Lua's regex engine does not support these escape sequences inside
    -- character classes, so we replace them with the actual control characters.
    content = content:gsub("\\v", "\x0B")
    content = content:gsub("\\f", "\x0C")

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("javascript_lexer: failed to parse grammar file: " .. (parse_err or "unknown error"))
    end

    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a JavaScript source string.
--
-- Loads the grammar for the requested ECMAScript version (cached after first
-- call) and feeds the source to a `GrammarLexer`. Returns the complete flat
-- token list, including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in the grammar file.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source  string       The JavaScript text to tokenize.
-- @param version string|nil   ECMAScript version: "es1", "es3", "es5",
--                             "es2015".."es2025", or nil/"" for generic.
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (generic):
--
--   local js_lexer = require("coding_adventures.javascript_lexer")
--   local tokens = js_lexer.tokenize("const x = 1;")
--   -- tokens[1].type  → "CONST"
--   -- tokens[1].value → "const"
--
-- Example (versioned):
--
--   local tokens = js_lexer.tokenize("var x = 1;", "es1")
--   -- Uses code/grammars/ecmascript/es1.tokens
function M.tokenize(source, version)
    local grammar = get_grammar(version)
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

--- Create a GrammarLexer for a JavaScript source string without tokenizing yet.
--
-- Returns the initialized `GrammarLexer` instance; call `:tokenize()` to run it.
-- Useful when you need to configure the lexer before consuming tokens, or to
-- measure performance without counting grammar-load time.
--
-- @param source  string       The JavaScript text to lex.
-- @param version string|nil   ECMAScript version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
--
-- Example:
--
--   local gl = js_lexer.create_lexer("var x = 1;", "es5")
--   local raw = gl:tokenize()
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for JavaScript.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @param version string|nil  ECMAScript version tag (see tokenize for valid values).
-- @return TokenGrammar       The parsed JavaScript token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
