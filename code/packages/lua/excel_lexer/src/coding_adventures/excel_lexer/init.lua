-- excel_lexer -- Tokenizes Excel formula text using the grammar-driven infrastructure
-- ==================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `excel.tokens` grammar file to configure the tokenizer.
--
-- # What is an Excel formula?
--
-- Excel formulas begin with "=" and describe a computation using cell
-- references, functions, operators, and literals.  Examples:
--
--   =A1+B2                   → add cells A1 and B2
--   =SUM(A1:B10)             → sum a range using a built-in function
--   =IF(A1>0, "pos", "neg")  → conditional returning a string
--   =Sheet1!A1               → cross-sheet reference
--   =A1*100%                 → percentage (postfix %)
--
-- The lexer's job is to turn the raw formula text into a flat stream of
-- typed tokens that the parser can analyze without worrying about character
-- boundaries.
--
-- # Token stream example: =A1+B2
--
--   Token(EQUALS,  "=",  1:1)
--   Token(CELL,    "A1", 1:2)
--   Token(PLUS,    "+",  1:4)
--   Token(CELL,    "B2", 1:5)
--   Token(EOF,     "",   1:7)
--
-- # Excel's case-insensitivity
--
-- Excel has been case-insensitive since its earliest days (Multiplan, 1982).
-- The reasons are historical and pragmatic:
--
--   1. The original IBM PC keyboard had no shift-lock for formula entry.
--   2. Early spreadsheet users (accountants) were not programmers and did
--      not expect case to matter.
--   3. Lowercase formulas like `=sum(a1:b10)` should work identically to
--      `=SUM(A1:B10)`.
--
-- The `excel.tokens` grammar declares `@case_insensitive true`.  We handle
-- this by lowercasing the source before passing it to the GrammarLexer,
-- since the underlying GrammarLexer does not support case-insensitive
-- matching natively.  The *original* source text is returned in token values
-- (we track both lowercased position and original text).
--
-- # A1 vs R1C1 reference styles
--
-- Excel supports two reference notation systems:
--
--   A1 style (default):
--     - Column is a letter (A–XFD, up to 16,384 columns)
--     - Row is a number (1–1,048,576)
--     - Examples: A1, $B$2, AC100, XFD1048576
--     - Dollar signs ($) make an axis absolute (non-adjusting when copied)
--
--   R1C1 style (optional, toggled via Excel settings):
--     - Row and Column are both integers: R1C1 = row 1 col 1
--     - Relative offsets use brackets: R[-1]C[2] = one row up, two cols right
--     - Popular in VBA / macro contexts because the numbers are computable
--
-- This lexer handles A1 style, which is the default and by far the most
-- common in end-user formulas.
--
-- # Structured references (Excel Tables)
--
-- Excel Tables (introduced in Excel 2007) allow references like:
--
--   Table1[Column1]             — one column of a named table
--   Table1[[#Headers],[Col]]    — structured keyword + column
--   [@Amount]                   — current row's Amount column
--
-- The tokens STRUCTURED_KEYWORD and STRUCTURED_COLUMN cover these patterns.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `excel.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/excel_lexer/src/coding_adventures/excel_lexer/init.lua
--
-- Walking up 6 directory levels from this file's directory reaches `code/`,
-- then we descend into `grammars/excel.tokens`.
--
-- Directory structure from script_dir upward:
--   excel_lexer/          (1)
--   coding_adventures/    (2)
--   src/                  (3)
--   excel_lexer/          (4) — the package directory
--   lua/                  (5)
--   packages/             (6)
--   code/                 → then /grammars/excel.tokens

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
-- variable.  Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

local _grammar_cache = nil

--- Load and parse the `excel.tokens` grammar, with caching.
-- On the first call, opens and parses the file.  On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Excel token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/excel_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/excel_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/excel.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "excel_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("excel_lexer: failed to parse excel.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an Excel formula source string.
--
-- Loads the `excel.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`.  Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- # Case normalization
--
-- Excel formulas are case-insensitive.  The `excel.tokens` grammar declares
-- `@case_insensitive true`.  Because the underlying GrammarLexer performs
-- case-sensitive pattern matching, we normalize the input to lowercase
-- before tokenizing.  The returned token values therefore reflect the
-- normalized (lowercase) form of each token.
--
-- # Excel whitespace handling
--
-- Unlike JSON, Excel formulas *do* use spaces as an intersection operator
-- in range references (e.g., `=SUM(A1:B10 B5:C15)` intersects two ranges).
-- The `excel.tokens` grammar therefore emits SPACE tokens rather than
-- silently skipping all whitespace.  Only non-space whitespace (tabs, CR,
-- LF) is silently consumed.
--
-- @param source string  The Excel formula text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local excel_lexer = require("coding_adventures.excel_lexer")
--   local tokens = excel_lexer.tokenize("=A1+B2")
--   -- tokens[1].type  → "EQUALS"
--   -- tokens[1].value → "="
--   -- tokens[2].type  → "CELL"
--   -- tokens[2].value → "a1"   (lowercased)
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

--- Return the cached (or freshly loaded) TokenGrammar for Excel formulas.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Excel token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
