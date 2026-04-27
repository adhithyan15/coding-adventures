-- java_lexer — Tokenizes Java source using the grammar-driven infrastructure
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar file to configure the tokenizer.
--
-- # What is Java tokenization?
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
--   1. Locates the correct grammar file in `code/grammars/java/`.
--   2. Reads and parses it once per version (cached) using
--      `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Version routing
--
-- When `version` is nil or "" → loads `code/grammars/java/java21.tokens`
--   (defaults to Java 21, the latest LTS)
-- When `version` is "1.0"    → loads `code/grammars/java/java1.0.tokens`
-- When `version` is "8"      → loads `code/grammars/java/java8.tokens`
-- ... etc.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/java_lexer/src/coding_adventures/java_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/java/`.
--
-- Directory structure from script_dir upward:
--   java_lexer/          (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   java_lexer/          (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/java/...

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid Java versions
-- =========================================================================
--
-- Java is versioned by release number. The version strings we accept
-- match the grammar files under code/grammars/java/:
--
--   "1.0" — Java 1.0  (1996): the original release.
--   "1.1" — Java 1.1  (1997): inner classes.
--   "1.4" — Java 1.4  (2002): assert keyword.
--   "5"   — Java 5    (2004): generics, annotations, enums.
--   "7"   — Java 7    (2011): try-with-resources, diamond operator.
--   "8"   — Java 8    (2014): lambdas, default methods, streams.
--   "10"  — Java 10   (2018): local variable type inference (var).
--   "14"  — Java 14   (2020): switch expressions, records (preview).
--   "17"  — Java 17   (2021): sealed classes, pattern matching.
--   "21"  — Java 21   (2023): virtual threads, record patterns.
--   nil / "" — defaults to Java 21 (latest LTS).

local VALID_JAVA_VERSIONS = {
    ["1.0"] = true, ["1.1"] = true, ["1.4"] = true,
    ["5"]   = true, ["7"]   = true, ["8"]   = true,
    ["10"]  = true, ["14"]  = true, ["17"]  = true,
    ["21"]  = true,
}

local DEFAULT_VERSION = "21"

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
-- @param version string|nil  The Java version tag, or nil/empty for default (21).
-- @return string             Absolute path to the grammar .tokens file.
local function resolve_tokens_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    -- Default to Java 21 when no version specified.
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    -- Validate the version string before building the path.
    if not VALID_JAVA_VERSIONS[version] then
        error(
            "java_lexer: unknown Java version '" .. version .. "'. " ..
            "Valid values are: 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21, or nil/\"\" for default (21)."
        )
    end

    return repo_root .. "/grammars/java/java" .. version .. ".tokens"
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- @param version string|nil  The Java version tag (see resolve_tokens_path).
-- @return TokenGrammar       The parsed Java token grammar.
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
            "java_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    -- Some Java grammar files may use \v and \f in whitespace patterns.
    -- Lua's regex engine does not support these escape sequences inside
    -- character classes, so we replace them with the actual control characters.
    content = content:gsub("\\v", "\x0B")
    content = content:gsub("\\f", "\x0C")

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("java_lexer: failed to parse grammar file: " .. (parse_err or "unknown error"))
    end

    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Java source string.
--
-- @param source  string       The Java text to tokenize.
-- @param version string|nil   Java version: "1.0", "1.1", "1.4", "5", "7",
--                             "8", "10", "14", "17", "21", or nil/"" for
--                             default (21).
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (default):
--
--   local java_lexer = require("coding_adventures.java_lexer")
--   local tokens = java_lexer.tokenize("int x = 1;")
--
-- Example (versioned):
--
--   local tokens = java_lexer.tokenize("int x = 1;", "1.0")
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

--- Create a GrammarLexer for a Java source string without tokenizing yet.
--
-- @param source  string       The Java text to lex.
-- @param version string|nil   Java version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for Java.
--
-- @param version string|nil  Java version tag (see tokenize for valid values).
-- @return TokenGrammar       The parsed Java token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
