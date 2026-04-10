-- brainfuck.lexer — Tokenizes Brainfuck source text using the grammar infrastructure
-- ==================================================================================
--
-- This module is the tokenization layer for the Brainfuck front-end pipeline.
-- It is a thin wrapper around the grammar-driven `GrammarLexer` from the
-- `coding_adventures.lexer` package, loading the `brainfuck.tokens` grammar
-- file to configure the tokenizer.
--
-- # What is Brainfuck tokenization?
--
-- Given the input:  ++[>+<-]
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(INC,        "+",  1:1)
--   Token(INC,        "+",  1:2)
--   Token(LOOP_START, "[",  1:3)
--   Token(RIGHT,      ">",  1:4)
--   Token(INC,        "+",  1:5)
--   Token(LEFT,       "<",  1:6)
--   Token(DEC,        "-",  1:7)
--   Token(LOOP_END,   "]",  1:8)
--   Token(EOF,        "",   1:9)
--
-- Comment characters (everything that is not `><+-.,[]`) are silently consumed
-- by the `brainfuck.tokens` grammar's skip: section. The parser only ever
-- receives the eight command tokens and EOF.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `brainfuck.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/brainfuck/src/coding_adventures/brainfuck/lexer.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/brainfuck.tokens`.
--
-- Directory structure from script_dir upward:
--   brainfuck/           (1)  — this file's directory
--   coding_adventures/   (2)
--   src/                 (3)
--   brainfuck/           (4)  — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/brainfuck.tokens

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- Copied verbatim from coding_adventures.json_lexer, which uses the same
-- approach. We need to navigate from this file's location to the repo root
-- so we can open the shared grammar file.

--- Return the directory portion of a file path (without trailing slash).
-- For example:  "/a/b/c/lexer.lua"  →  "/a/b/c"
-- @param path string The full file path.
-- @return string     The directory portion.
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua embeds the source path in the chunk debug info with a leading "@".
-- We strip that prefix to get the real filesystem path.
-- @return string Absolute directory of this lexer.lua file.
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
    -- Security: io.popen is used only with fixed built-in commands ("cd"/"pwd"),
    -- never with user-controlled input. The previously removed pattern
    --   io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
    -- was unsafe because dir could contain shell metacharacters.
    -- The current approach is safe: no user input reaches the shell.
    -- Updated: 2026-04-10.
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local cwd = os.getenv("PWD") or os.getenv("CD") or ""
        if cwd == "" then
            -- Safe fallback: fixed built-in command, no user input — no injection risk.
            -- On Windows `cd` prints the current directory; on POSIX `pwd` does the same.
            local is_win = package.config:sub(1, 1) == "\\"
            local h = is_win and io.popen("cd") or io.popen("pwd")
            if h then
                local line = h:read("*l") or ""
                h:close()
                cwd = line:gsub("%c+$", "")
            end
        end
        if cwd ~= "" then
            cwd = cwd:gsub("\\", "/"):gsub("%c+$", "")
            dir = cwd .. "/" .. dir
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

--- Load and parse the `brainfuck.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Brainfuck token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- lexer.lua is 3 dirs inside the package (src/coding_adventures/brainfuck/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/brainfuck/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/brainfuck.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "brainfuck.lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("brainfuck.lexer: failed to parse brainfuck.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Brainfuck source string.
--
-- Loads the `brainfuck.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Comment characters (any character that is not `><+-.,[]`) are consumed
-- silently via the skip patterns in `brainfuck.tokens`. The caller receives
-- only meaningful tokens: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT,
-- LOOP_START, LOOP_END, EOF.
--
-- Unlike JSON, Brainfuck tokenization never raises an error on unexpected
-- characters — every non-command character is a valid comment.
--
-- @param source string  The Brainfuck text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
--
-- Example:
--
--   local bf_lexer = require("coding_adventures.brainfuck.lexer")
--   local tokens = bf_lexer.tokenize("++[>+<-]")
--   -- tokens[1].type  → "INC"
--   -- tokens[1].value → "+"
--   -- tokens[3].type  → "LOOP_START"
--   -- tokens[3].value → "["
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

--- Create a GrammarLexer for a Brainfuck source string without immediately tokenizing.
--
-- Use this when you want fine-grained control over lexing — for example,
-- to tokenize incrementally or to access the raw GrammarLexer object.
--
-- @param source string      The Brainfuck text to tokenize.
-- @return GrammarLexer      An initialized lexer, ready to call :tokenize().
--
-- Example:
--
--   local lx = bf_lexer.create_lexer("++")
--   local raw_tokens = lx:tokenize()
function M.create_lexer(source)
    local grammar = get_grammar()
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for Brainfuck.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Brainfuck token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
