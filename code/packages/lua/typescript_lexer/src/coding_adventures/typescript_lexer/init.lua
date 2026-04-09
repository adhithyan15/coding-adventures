-- typescript_lexer — Tokenizes TypeScript source using the grammar-driven infrastructure
-- =======================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `typescript.tokens` grammar file to configure the tokenizer.
--
-- # What is TypeScript tokenization?
--
-- TypeScript is a strict superset of JavaScript. Every valid JavaScript
-- program is also valid TypeScript. TypeScript adds:
--   - Type annotations: `let x: number = 1`
--   - Interfaces: `interface Foo { bar: string }`
--   - Generics: `Array<number>`
--   - Access modifiers: `public`, `private`, `protected`
--   - `enum`, `type`, `namespace`, `declare`, `readonly`
--   - Abstract classes, `implements`, `extends`
--   - Type utilities: `keyof`, `infer`, `never`, `unknown`
--   - Primitive type keywords: `any`, `void`, `number`, `string`,
--     `boolean`, `object`, `symbol`, `bigint`
--
-- Given the input:  interface Foo { bar: number; }
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(INTERFACE, "interface", 1:1)
--   Token(NAME,      "Foo",       1:11)
--   Token(LBRACE,    "{",         1:15)
--   Token(NAME,      "bar",       1:17)
--   Token(COLON,     ":",         1:20)
--   Token(NUMBER_KW, "number",    1:22)   -- keyword, not a number literal
--   Token(SEMICOLON, ";",         1:28)
--   Token(RBRACE,    "}",         1:30)
--   Token(EOF,       "",          1:31)
--
-- Whitespace is silently consumed (declared as skip patterns in
-- `typescript.tokens`). The parser never sees whitespace tokens.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `typescript.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/typescript_lexer/src/coding_adventures/typescript_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/typescript.tokens`.
--
-- Directory structure from script_dir upward:
--   typescript_lexer/    (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   typescript_lexer/    (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/typescript.tokens

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid TypeScript versions
-- =========================================================================
--
-- TypeScript has had several major releases. The canonical version strings
-- we accept are:
--
--   "ts1.0"  — TypeScript 1.0 (April 2014): initial public release.
--   "ts2.0"  — TypeScript 2.0 (September 2016): non-nullable types.
--   "ts3.0"  — TypeScript 3.0 (July 2018): project references, tuples.
--   "ts4.0"  — TypeScript 4.0 (August 2020): variadic tuple types.
--   "ts5.0"  — TypeScript 5.0 (March 2023): decorators (Stage 3).
--   "ts5.8"  — TypeScript 5.8 (February 2025): granular control-flow.
--   nil / "" — Generic TypeScript (uses the latest stable grammar).
--
-- Each version maps to a grammar file under:
--   code/grammars/typescript/<version>.tokens

local VALID_TS_VERSIONS = {
    ["ts1.0"] = true,
    ["ts2.0"] = true,
    ["ts3.0"] = true,
    ["ts4.0"] = true,
    ["ts5.0"] = true,
    ["ts5.8"] = true,
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
-- The grammar is read from disk exactly once and cached in a module-level
-- variable. Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

-- Cache keyed by version string (or "" for generic).
-- Loading and parsing grammar files from disk is expensive; we do it once
-- per version per process and reuse the result for every subsequent call.
local _grammar_cache = {}

--- Resolve the path to the correct .tokens grammar file for a given version.
--
-- Version routing logic:
--   - nil or ""  →  code/grammars/typescript.tokens   (generic)
--   - "ts1.0"    →  code/grammars/typescript/ts1.0.tokens
--   - "ts2.0"    →  code/grammars/typescript/ts2.0.tokens
--   - ...etc.
--
-- If an unrecognized version string is passed we raise an error immediately
-- rather than silently falling back, which would hide bugs in callers.
--
-- @param version string|nil  The TypeScript version tag, or nil/empty for generic.
-- @return string             Absolute path to the grammar .tokens file.
local function resolve_tokens_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    -- Generic (no version specified) — use the unified grammar.
    if not version or version == "" then
        return repo_root .. "/grammars/typescript.tokens"
    end

    -- Validate the version string before building the path.
    if not VALID_TS_VERSIONS[version] then
        error(
            "typescript_lexer: unknown TypeScript version '" .. version .. "'. " ..
            "Valid values are: ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8, or nil/\"\" for generic."
        )
    end

    return repo_root .. "/grammars/typescript/" .. version .. ".tokens"
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- On the first call for a given version, opens the file, parses it with
-- `grammar_tools.parse_token_grammar`, and stores the result in _grammar_cache.
-- On subsequent calls for the same version, returns the cached object immediately.
--
-- @param version string|nil  The TypeScript version tag (see resolve_tokens_path).
-- @return TokenGrammar       The parsed TypeScript token grammar.
local function get_grammar(version)
    local key = version or ""
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local tokens_path = resolve_tokens_path(version)

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "typescript_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("typescript_lexer: failed to parse grammar file: " .. (parse_err or "unknown error"))
    end

    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a TypeScript source string.
--
-- Loads the grammar for the requested TypeScript version (cached after first
-- call) and feeds the source to a `GrammarLexer`. Returns the complete flat
-- token list, including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in the grammar file.
-- The caller receives only meaningful tokens.
--
-- TypeScript is a superset of JavaScript, so all JavaScript tokens are
-- recognized plus TypeScript-specific keywords: INTERFACE, TYPE, ENUM,
-- NAMESPACE, DECLARE, READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT,
-- IMPLEMENTS, EXTENDS, KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID,
-- NUMBER (keyword), STRING (keyword), BOOLEAN, OBJECT, SYMBOL, BIGINT.
--
-- @param source  string       The TypeScript text to tokenize.
-- @param version string|nil   TypeScript version: "ts1.0", "ts2.0", "ts3.0",
--                             "ts4.0", "ts5.0", "ts5.8", or nil/"" for generic.
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (generic):
--
--   local ts_lexer = require("coding_adventures.typescript_lexer")
--   local tokens = ts_lexer.tokenize("interface Foo { x: number }")
--   -- tokens[1].type  → "INTERFACE"
--   -- tokens[1].value → "interface"
--
-- Example (versioned):
--
--   local tokens = ts_lexer.tokenize("let x: number = 1;", "ts5.0")
--   -- Uses code/grammars/typescript/ts5.0.tokens
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

--- Create a GrammarLexer for a TypeScript source string without tokenizing yet.
--
-- Returns the initialized `GrammarLexer` instance; call `:tokenize()` to run it.
-- Useful when you need to configure the lexer before consuming tokens, or to
-- measure performance without counting grammar-load time.
--
-- @param source  string       The TypeScript text to lex.
-- @param version string|nil   TypeScript version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
--
-- Example:
--
--   local gl = ts_lexer.create_lexer("let x = 1;", "ts5.8")
--   local raw = gl:tokenize()
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for TypeScript.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @param version string|nil  TypeScript version tag (see tokenize for valid values).
-- @return TokenGrammar       The parsed TypeScript token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
