-- verilog_lexer — Tokenizes Verilog source using the grammar-driven infrastructure
-- ==============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `verilog.tokens` grammar file to configure the tokenizer.
--
-- # What is Verilog tokenization?
--
-- Verilog (IEEE 1364-2005) is a Hardware Description Language (HDL). Unlike
-- software languages that describe sequential computations, Verilog describes
-- *physical structures* — gates, wires, flip-flops — that exist simultaneously
-- and operate in parallel.
--
-- Given the input:  module adder(input a, input b, output y);
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(KEYWORD, "module",  1:1)
--   Token(NAME,    "adder",   1:8)
--   Token(LPAREN,  "(",       1:13)
--   Token(KEYWORD, "input",   1:14)
--   Token(NAME,    "a",       1:20)
--   Token(COMMA,   ",",       1:21)
--   Token(KEYWORD, "input",   1:23)
--   Token(NAME,    "b",       1:29)
--   Token(COMMA,   ",",       1:30)
--   Token(KEYWORD, "output",  1:32)
--   Token(NAME,    "y",       1:39)
--   Token(RPAREN,  ")",       1:40)
--   Token(SEMICOLON, ";",     1:41)
--   Token(EOF,     "",        1:42)
--
-- Whitespace and comments are silently consumed (declared as skip patterns
-- in `verilog.tokens`). The parser never sees them.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `verilog.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/verilog_lexer/src/coding_adventures/verilog_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/verilog.tokens`.
--
-- Directory structure from script_dir upward:
--   verilog_lexer/       (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   verilog_lexer/       (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/verilog.tokens
--
-- # Token types produced
--
-- Keyword tokens (NAME tokens promoted to their uppercase keyword type):
--   MODULE, ENDMODULE, INPUT, OUTPUT, INOUT, WIRE, REG, INTEGER, REAL,
--   SIGNED, UNSIGNED, TRI, SUPPLY0, SUPPLY1, ALWAYS, INITIAL, BEGIN, END,
--   IF, ELSE, CASE, CASEX, CASEZ, ENDCASE, DEFAULT, FOR, ASSIGN, DEFPARAM,
--   PARAMETER, LOCALPARAM, GENERATE, ENDGENERATE, GENVAR, POSEDGE, NEGEDGE,
--   OR, FUNCTION, ENDFUNCTION, TASK, ENDTASK, AND, NAND, NOR, NOT, BUF,
--   XOR, XNOR
--
-- Literal/regex tokens:
--   SIZED_NUMBER, REAL_NUMBER, NUMBER, STRING
--   SYSTEM_ID, DIRECTIVE, ESCAPED_IDENT, NAME
--
-- Three-char operators: ARITH_LEFT_SHIFT, ARITH_RIGHT_SHIFT, CASE_EQ, CASE_NEQ
-- Two-char operators:   LOGIC_AND, LOGIC_OR, LEFT_SHIFT, RIGHT_SHIFT,
--                       EQUALS_EQUALS, NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS,
--                       POWER, TRIGGER
-- Single-char operators: PLUS, MINUS, STAR, SLASH, PERCENT, AMP, PIPE, CARET,
--                        TILDE, BANG, LESS_THAN, GREATER_THAN, EQUALS,
--                        QUESTION, COLON
-- Delimiters: LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE,
--             SEMICOLON, COMMA, DOT, HASH, AT

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
    if dir:sub(2, 2) ~= ":" then
        local f = io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
        local resolved = f and f:read("*l")
        if f then f:close() end
        if resolved and resolved ~= "" then
            return resolved
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

--- Load and parse the `verilog.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Verilog token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/verilog_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/verilog_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/verilog.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "verilog_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("verilog_lexer: failed to parse verilog.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Verilog source string.
--
-- Loads the `verilog.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace and comments are consumed silently via the skip patterns in
-- `verilog.tokens`. The caller receives only meaningful tokens: NAME (and
-- keyword subtypes), number tokens, STRING, operators, delimiters, and EOF.
--
-- @param source string  The Verilog text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local vl = require("coding_adventures.verilog_lexer")
--   local tokens = vl.tokenize("module adder(input a, input b, output y);")
--   -- tokens[1].type  → "MODULE"
--   -- tokens[1].value → "module"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "adder"
function M.tokenize(source)
    local grammar = get_grammar()
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

--- Return the cached (or freshly loaded) TokenGrammar for Verilog.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Verilog token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
