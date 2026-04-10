-- vhdl_lexer — Tokenizes VHDL source using the grammar-driven infrastructure
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `vhdl.tokens` grammar file to configure the tokenizer.
--
-- # What is VHDL tokenization?
--
-- VHDL (VHSIC Hardware Description Language, IEEE 1076-2008) was designed
-- by the US Department of Defense for documenting and simulating digital
-- systems. Where Verilog is terse and C-like, VHDL is verbose and Ada-like:
-- strong typing, explicit declarations, and case-insensitive identifiers.
--
-- Given the input:  entity adder is port (a : in std_logic);
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(ENTITY,     "entity",   1:1)
--   Token(NAME,       "adder",    1:8)
--   Token(IS,         "is",       1:14)
--   Token(PORT,       "port",     1:17)
--   Token(LPAREN,     "(",        1:22)
--   Token(NAME,       "a",        1:23)
--   Token(COLON,      ":",        1:25)
--   Token(IN,         "in",       1:27)
--   Token(NAME,       "std_logic",1:30)
--   Token(RPAREN,     ")",        1:39)
--   Token(SEMICOLON,  ";",        1:40)
--   Token(EOF,        "",         1:41)
--
-- VHDL is case-insensitive: ENTITY, Entity, and entity are all the same.
-- The `vhdl.tokens` grammar sets `case_sensitive: false`, so the lexer
-- lowercases all text before matching. All token values are lowercase.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `vhdl.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/vhdl_lexer/src/coding_adventures/vhdl_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/vhdl.tokens`.
--
-- Directory structure from script_dir upward:
--   vhdl_lexer/          (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   vhdl_lexer/          (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/vhdl.tokens
--
-- # Token types produced
--
-- Keyword tokens (NAME tokens with lowercased values matched against keyword list):
--   ABS, ACCESS, AFTER, ALIAS, ALL, AND, ARCHITECTURE, ARRAY, ASSERT,
--   ATTRIBUTE, BEGIN, BLOCK, BODY, BUFFER, BUS, CASE, COMPONENT,
--   CONFIGURATION, CONSTANT, DISCONNECT, DOWNTO, ELSE, ELSIF, END, ENTITY,
--   EXIT, FILE, FOR, FUNCTION, GENERATE, GENERIC, GROUP, GUARDED, IF,
--   IMPURE, IN, INOUT, IS, LABEL, LIBRARY, LINKAGE, LITERAL, LOOP, MAP,
--   MOD, NAND, NEW, NEXT, NOR, NOT, NULL, OF, ON, OPEN, OR, OTHERS, OUT,
--   PACKAGE, PORT, POSTPONED, PROCEDURE, PROCESS, PURE, RANGE, RECORD,
--   REGISTER, REJECT, REM, REPORT, RETURN, ROL, ROR, SELECT, SEVERITY,
--   SIGNAL, SHARED, SLA, SLL, SRA, SRL, SUBTYPE, THEN, TO, TRANSPORT,
--   TYPE, UNAFFECTED, UNITS, UNTIL, USE, VARIABLE, WAIT, WHEN, WHILE,
--   WITH, XNOR, XOR
--
-- Literal/regex tokens:
--   BASED_LITERAL — e.g. 16#FF#, 2#1010#
--   REAL_NUMBER   — e.g. 3.14, 1.0E-3
--   NUMBER        — plain integers like 42, 1_000
--   STRING        — double-quoted string with "" escaping
--   BIT_STRING    — prefix b/o/x/d + quoted value, e.g. X"FF", B"1010"
--   CHAR_LITERAL  — single character between tick marks: '0', '1', 'X', 'Z'
--   EXTENDED_IDENT — backslash-delimited identifier: \my odd name\
--   NAME          — regular identifier
--
-- Two-char operators: VAR_ASSIGN (:=), LESS_EQUALS (<=), GREATER_EQUALS (>=),
--                     ARROW (=>), NOT_EQUALS (/=), POWER (**), BOX (<>)
-- Single-char operators: PLUS, MINUS, STAR, SLASH, AMPERSAND,
--                        LESS_THAN, GREATER_THAN, EQUALS, TICK, PIPE
-- Delimiters: LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON, COMMA, DOT, COLON

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
    -- Security: Do not pass the dir string to io.popen (shell injection risk).
    -- Instead, use os.getenv to resolve relative paths -- no subprocess or
    -- shell invocation is involved. The previously removed pattern
    --   io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
    -- was unsafe because dir could contain shell metacharacters.
    -- Fixed: 2026-04-10 security review.
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local cwd = os.getenv("PWD") or os.getenv("CD") or ""
        if cwd ~= "" then
            dir = cwd:gsub("\\\\", "/"):gsub("%c+$", "") .. "/" .. dir
            -- Normalise .. and . segments so dirname-based traversal works
            -- correctly when the source was loaded via a relative package.path
            -- entry (e.g. "../src/?.lua" from a tests/ subdirectory).
            local is_abs = dir:sub(1, 1) == "/"
            local parts = {}
            for seg in dir:gmatch("[^/]+") do
                if seg == ".." then
                    if #parts > 0 then table.remove(parts) end
                elseif seg ~= "." then
                    table.insert(parts, seg)
                end
            end
            dir = (is_abs and "/" or "") .. table.concat(parts, "/")
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

--- Load and parse the `vhdl.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed VHDL token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/vhdl_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/vhdl_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/vhdl.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "vhdl_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("vhdl_lexer: failed to parse vhdl.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a VHDL source string.
--
-- Loads the `vhdl.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- VHDL is case-insensitive: `vhdl.tokens` sets `case_sensitive: false`,
-- so the grammar lowercases input before matching. All returned token
-- values will be in lowercase.
--
-- Whitespace and comments are consumed silently via the skip patterns in
-- `vhdl.tokens`. The caller receives only meaningful tokens: NAME (and
-- keyword subtypes), number/literal tokens, operators, delimiters, and EOF.
--
-- @param source string  The VHDL text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local vh = require("coding_adventures.vhdl_lexer")
--   local tokens = vh.tokenize("entity adder is")
--   -- tokens[1].type  → "ENTITY"
--   -- tokens[1].value → "entity"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "adder"
--   -- tokens[3].type  → "IS"
--   -- tokens[3].value → "is"
function M.tokenize(source)
    local grammar    = get_grammar()
    local normalized = source:lower()
    local gl         = lexer_pkg.GrammarLexer.new(normalized, grammar)
    local raw        = gl:tokenize()
    local tokens     = {}
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

--- Return the cached (or freshly loaded) TokenGrammar for VHDL.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed VHDL token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
