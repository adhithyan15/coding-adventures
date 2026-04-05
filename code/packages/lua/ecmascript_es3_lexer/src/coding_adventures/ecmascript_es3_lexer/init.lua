-- ecmascript_es3_lexer — Tokenizes ECMAScript 3 (1999) source code
-- ================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `ecmascript/es3.tokens` grammar file to configure the tokenizer.
--
-- # What is ECMAScript 3?
--
-- ECMAScript 3 (ECMA-262, 3rd Edition, December 1999) made JavaScript a
-- real, complete language. It landed two years after ES1 and added features
-- that developers today consider fundamental.
--
-- What ES3 adds over ES1:
--   - === and !== (strict equality — no type coercion)
--   - try/catch/finally/throw (structured error handling)
--   - Regular expression literals (/pattern/flags)
--   - `instanceof` operator
--   - 28 keywords total (5 new: catch, finally, instanceof, throw, try)
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `ecmascript/es3.tokens` grammar file.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- Directory structure from script_dir upward:
--   ecmascript_es3_lexer/  (1) — module dir
--   coding_adventures/     (2)
--   src/                   (3)
--   ecmascript_es3_lexer/  (4) — the package directory
--   lua/                   (5)
--   packages/              (6)
--   code/                  → then /grammars/ecmascript/es3.tokens

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
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

local _grammar_cache = nil

local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/ecmascript/es3.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "ecmascript_es3_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    -- Lua's regex engine does not support \v (vertical tab) or \f (form feed)
    -- as escape sequences. Replace with actual control characters.
    content = content:gsub("\\v", "\x0B")
    content = content:gsub("\\f", "\x0C")

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("ecmascript_es3_lexer: failed to parse es3.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an ECMAScript 3 source string.
--
-- @param source string  The ECMAScript 3 text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
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

--- Return the cached (or freshly loaded) TokenGrammar for ECMAScript 3.
function M.get_grammar()
    return get_grammar()
end

return M
