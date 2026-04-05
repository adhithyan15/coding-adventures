-- ecmascript_es5_lexer — Tokenizes ECMAScript 5 (2009) source code
-- ================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `ecmascript/es5.tokens` grammar file to configure the tokenizer.
--
-- # What is ECMAScript 5?
--
-- ECMAScript 5 (ECMA-262, 5th Edition, December 2009) landed a full decade
-- after ES3. The syntactic changes are modest — the real innovations were
-- strict mode semantics, native JSON support, and property descriptors.
--
-- What ES5 adds over ES3:
--   - `debugger` keyword (moved from future-reserved to keyword)
--   - Getter/setter syntax in object literals
--   - String line continuation
--   - Trailing commas in object literals
--
-- ES5 does NOT have: let/const, class, arrow functions, template literals,
-- modules, or destructuring (all added in ES2015).
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `ecmascript/es5.tokens` grammar file.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` for each call.
--   4. Returns the flat token list.

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
    local tokens_path = repo_root .. "/grammars/ecmascript/es5.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "ecmascript_es5_lexer: cannot open grammar file: " .. tokens_path ..
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
        error("ecmascript_es5_lexer: failed to parse es5.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an ECMAScript 5 source string.
--
-- @param source string  The ECMAScript 5 text to tokenize.
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

--- Return the cached (or freshly loaded) TokenGrammar for ECMAScript 5.
function M.get_grammar()
    return get_grammar()
end

return M
