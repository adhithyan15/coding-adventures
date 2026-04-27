-- csharp_lexer — Tokenizes C# source using the grammar-driven infrastructure
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar file to configure the tokenizer.
--
-- # What is C# tokenization?
--
-- Given the input:  int x = 42;
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(INT,        "int",  1:1)
--   Token(NAME,       "x",    1:5)
--   Token(EQUALS,     "=",    1:7)
--   Token(NUMBER,     "42",   1:9)
--   Token(SEMICOLON,  ";",    1:11)
--   Token(EOF,        "",     1:12)
--
-- Whitespace and comments are silently consumed (declared as skip patterns
-- in the grammar file). The parser never sees them.
--
-- # Architecture
--
-- This module:
--   1. Locates the correct grammar file in `code/grammars/csharp/`.
--   2. Reads and parses it once per version (cached) using
--      `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Version routing
--
-- When `version` is nil or "" → loads `code/grammars/csharp/csharp12.0.tokens`
--   (defaults to C# 12.0, the latest stable release)
-- When `version` is "1.0"  → loads `code/grammars/csharp/csharp1.0.tokens`
-- When `version` is "8.0"  → loads `code/grammars/csharp/csharp8.0.tokens`
-- ... etc.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/csharp_lexer/src/coding_adventures/csharp_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/csharp/`.
--
-- Directory structure from script_dir upward:
--   csharp_lexer/        (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   csharp_lexer/        (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/csharp/...

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid C# versions
-- =========================================================================
--
-- C# is versioned by language release. The version strings we accept
-- match the grammar files under code/grammars/csharp/:
--
--   "1.0"  — C# 1.0  (2002): the original .NET 1.0 release. Classes,
--             interfaces, structs, delegates, events, enums, basic OOP.
--   "2.0"  — C# 2.0  (2005): generics, nullable types, anonymous methods,
--             iterators (yield), partial types.
--   "3.0"  — C# 3.0  (2007): LINQ, lambda expressions, extension methods,
--             anonymous types, var keyword, auto-properties.
--   "4.0"  — C# 4.0  (2010): dynamic typing, named/optional arguments,
--             generic covariance and contravariance.
--   "5.0"  — C# 5.0  (2012): async/await, caller info attributes.
--   "6.0"  — C# 6.0  (2015): string interpolation, null-conditional operators,
--             expression-bodied members, nameof, using static.
--   "7.0"  — C# 7.0  (2017): tuples, pattern matching (is), local functions,
--             out variables, ref returns, discards (_).
--   "8.0"  — C# 8.0  (2019): nullable reference types, switch expressions,
--             default interface members, ranges and indices (..  ^).
--   "9.0"  — C# 9.0  (2020): records, init-only setters, top-level statements,
--             pattern matching improvements, nint/nuint native types.
--   "10.0" — C# 10.0 (2021): record structs, global using, file-scoped
--             namespace, extended property patterns.
--   "11.0" — C# 11.0 (2022): raw string literals, list patterns, required
--             members, generic attributes, file-local types.
--   "12.0" — C# 12.0 (2023): primary constructors on classes/structs,
--             collection expressions, inline arrays, alias any type.
--   nil / "" — defaults to C# 12.0 (latest stable).

local VALID_CSHARP_VERSIONS = {
    ["1.0"]  = true, ["2.0"]  = true, ["3.0"]  = true,
    ["4.0"]  = true, ["5.0"]  = true, ["6.0"]  = true,
    ["7.0"]  = true, ["8.0"]  = true, ["9.0"]  = true,
    ["10.0"] = true, ["11.0"] = true, ["12.0"] = true,
}

local DEFAULT_VERSION = "12.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory of this source file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Normalize Windows backslashes to forward slashes.
    src = src:gsub("\\", "/")
    return src:match("(.+)/[^/]+$") or "."
end

--- Walk up `levels` directory levels from `path`.
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = result .. "/.."
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = {}

--- Resolve the path to the correct .tokens grammar file for a given version.
--
-- @param version string|nil  The C# version tag, or nil/empty for default (12.0).
-- @return string             Absolute path to the grammar .tokens file.
local function resolve_tokens_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    -- Default to C# 12.0 when no version specified.
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    -- Validate the version string before building the path.
    if not VALID_CSHARP_VERSIONS[version] then
        error(
            "csharp_lexer: unknown C# version '" .. version .. "'. " ..
            "Valid values are: 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, " ..
            "9.0, 10.0, 11.0, 12.0, or nil/\"\" for default (12.0)."
        )
    end

    return repo_root .. "/grammars/csharp/csharp" .. version .. ".tokens"
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- Caching is important for performance: the grammar files are parsed from text
-- using pattern matching and table construction. For programs that tokenize many
-- C# snippets, we don't want to re-parse the grammar on each call.
--
-- @param version string|nil  The C# version tag (see resolve_tokens_path).
-- @return TokenGrammar       The parsed C# token grammar.
local function get_grammar(version)
    local key = version or DEFAULT_VERSION
    if key == "" then key = DEFAULT_VERSION end
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local tokens_path = resolve_tokens_path(version)

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "csharp_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    -- Some grammar files may use \v and \f in whitespace patterns.
    -- Lua's regex engine does not support these escape sequences inside
    -- character classes, so we replace them with the actual control characters.
    --   \v  →  vertical tab   (0x0B)
    --   \f  →  form feed      (0x0C)
    content = content:gsub("\\v", "\x0B")
    content = content:gsub("\\f", "\x0C")

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("csharp_lexer: failed to parse grammar file: " .. (parse_err or "unknown error"))
    end

    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a C# source string.
--
-- @param source  string       The C# text to tokenize.
-- @param version string|nil   C# version: "1.0", "2.0", "3.0", "4.0", "5.0",
--                             "6.0", "7.0", "8.0", "9.0", "10.0", "11.0",
--                             "12.0", or nil/"" for default (12.0).
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (default):
--
--   local csharp_lexer = require("coding_adventures.csharp_lexer")
--   local tokens = csharp_lexer.tokenize_csharp("int x = 1;")
--
-- Example (versioned):
--
--   local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "8.0")
function M.tokenize_csharp(source, version)
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

--- Create a GrammarLexer for a C# source string without tokenizing yet.
--
-- This is useful when you need the lexer object directly — for example,
-- to configure it, to stream tokens lazily, or to pass it to another tool.
--
-- @param source  string       The C# text to lex.
-- @param version string|nil   C# version tag (see tokenize_csharp for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
function M.create_csharp_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for C#.
--
-- Useful for inspecting what tokens the grammar defines, or for passing
-- the grammar to other infrastructure components.
--
-- @param version string|nil  C# version tag (see tokenize_csharp for valid values).
-- @return TokenGrammar       The parsed C# token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
