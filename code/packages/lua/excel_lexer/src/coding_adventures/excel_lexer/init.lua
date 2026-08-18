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
--   1. Requires the pre-compiled `_grammar` module (once, cached).
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.
--
-- # Grammar source
--
-- The token grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `excel.tokens` is pre-compiled (via
-- `grammar-tools compile-tokens`) into `_grammar.lua`, a plain Lua module
-- that embeds the TokenGrammar as native Lua data structures. That module
-- ships as part of this package, so `require()` always finds it.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `token_grammar()` result cached in a module-level variable. Subsequent
-- calls to `tokenize` reuse the cached grammar.

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for Excel formulas.
-- On the first call, requires the pre-compiled `_grammar` module and
-- invokes `token_grammar()`. On subsequent calls, returns the cached
-- TokenGrammar object immediately.
-- @return TokenGrammar  The Excel token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local compiled = require("coding_adventures.excel_lexer._grammar")
    _grammar_cache = compiled.token_grammar()
    return _grammar_cache
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
