-- algol_lexer -- Tokenizes ALGOL 60 source text using the grammar-driven infrastructure
-- =====================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `algol.tokens` grammar file to configure the tokenizer.
--
-- # What is ALGOL 60?
--
-- ALGOL 60 (ALGOrithmic Language, 1960) was the first programming language to be
-- formally specified using BNF (Backus-Naur Form). It introduced block structure,
-- lexical scoping, recursion, and the call stack — concepts every modern language
-- inherits. It is the ancestor of Pascal, C, Ada, and Simula (the first OOP language).
--
-- # What is ALGOL 60 tokenization?
--
-- Given the input:  begin integer x; x := 42 end
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(BEGIN,       "begin",   1:1)
--   Token(INTEGER,     "integer", 1:7)
--   Token(IDENT,       "x",       1:15)
--   Token(SEMICOLON,   ";",       1:16)
--   Token(IDENT,       "x",       1:18)
--   Token(ASSIGN,      ":=",      1:20)
--   Token(INTEGER_LIT, "42",      1:23)
--   Token(END,         "end",     1:26)
--   Token(EOF,         "",        1:29)
--
-- Whitespace is silently consumed (the `algol.tokens` grammar declares it
-- as a skip pattern). Comments (`comment ... ;`) are also silently consumed.
--
-- Keywords are case-insensitive: `BEGIN`, `Begin`, and `begin` all produce
-- a BEGIN token. The value field preserves the original case.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `algol.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/algol_lexer/src/coding_adventures/algol_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/algol.tokens`.
--
-- Directory structure from script_dir upward:
--   algol_lexer/         (1)  ← inner module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   algol_lexer/         (4)  ← the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/algol.tokens

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory of this source file.
-- Lua embeds the source path in chunk debug info with a leading "@".
-- Returns the directory portion of that path (may be relative when the
-- test runner uses a relative package.path like "../src/?.lua").
-- @return string Directory of this init.lua file (absolute or relative).
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
-- Appends `/../` segments so the OS resolves the result when it is passed
-- to io.open(). Works for both absolute paths (C:/foo, /foo) and relative
-- paths (../src/foo) without needing to know the working directory.
-- The old regex-based dirname approach broke on relative paths starting
-- with `..` because the pattern "(.+)/[^/]+" does not match strings like
-- ".." that have no slash — causing repo_root to collapse to ".".
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string        Path with `levels` parent-dir jumps appended.
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
--
-- The grammar is read from disk exactly once and cached in a module-level
-- variable.  Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

local _grammar_cache = {}

local function normalize_version(version)
    if version == nil or version == "" then
        return "algol60"
    end
    if version ~= "algol60" then
        error("algol_lexer: unknown ALGOL version '" .. tostring(version) .. "' (valid: algol60)")
    end
    return version
end

--- Load and parse the `algol.tokens` grammar, with caching.
-- On the first call, opens and parses the file.  On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed ALGOL 60 token grammar.
local function get_grammar(version)
    version = normalize_version(version)
    if _grammar_cache[version] then
        return _grammar_cache[version]
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/algol_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/algol_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/algol/" .. version .. ".tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "algol_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("algol_lexer: failed to parse " .. version .. ".tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache[version] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an ALGOL 60 source string.
--
-- Loads the `algol.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`.  Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace and comments (`comment ... ;`) are consumed silently via
-- the skip patterns in `algol.tokens`.  The caller receives only
-- meaningful tokens.
--
-- Keywords are case-insensitive.  The `value` field of a keyword token
-- preserves the original source text.  The `type` field is normalized
-- to the keyword name in uppercase (e.g., "BEGIN", "END", "IF").
--
-- @param source string  The ALGOL 60 text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local algol_lexer = require("coding_adventures.algol_lexer")
--   local tokens = algol_lexer.tokenize("begin integer x; x := 42 end")
--   -- tokens[1].type  → "BEGIN"
--   -- tokens[1].value → "begin"
--   -- tokens[3].type  → "NAME"
--   -- tokens[3].value → "x"
function M.tokenize(source, version)
    local grammar = get_grammar(version)
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

--- Return the cached (or freshly loaded) TokenGrammar for ALGOL 60.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed ALGOL 60 token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
