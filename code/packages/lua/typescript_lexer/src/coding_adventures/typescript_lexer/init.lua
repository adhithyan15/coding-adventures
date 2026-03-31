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

--- Load and parse the `typescript.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed TypeScript token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/typescript_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/typescript_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/typescript.tokens"

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
        error("typescript_lexer: failed to parse typescript.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a TypeScript source string.
--
-- Loads the `typescript.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in `typescript.tokens`.
-- The caller receives only meaningful tokens.
--
-- TypeScript is a superset of JavaScript, so all JavaScript tokens are
-- recognized plus TypeScript-specific keywords: INTERFACE, TYPE, ENUM,
-- NAMESPACE, DECLARE, READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT,
-- IMPLEMENTS, EXTENDS, KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID,
-- NUMBER (keyword), STRING (keyword), BOOLEAN, OBJECT, SYMBOL, BIGINT.
--
-- @param source string  The TypeScript text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local ts_lexer = require("coding_adventures.typescript_lexer")
--   local tokens = ts_lexer.tokenize("interface Foo { x: number }")
--   -- tokens[1].type  → "INTERFACE"
--   -- tokens[1].value → "interface"
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

--- Return the cached (or freshly loaded) TokenGrammar for TypeScript.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed TypeScript token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
