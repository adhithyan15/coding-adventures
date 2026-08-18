-- dartmouth_basic_lexer — Tokenizes 1964 Dartmouth BASIC source text
-- ===================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `dartmouth_basic.tokens` grammar file to configure the tokenizer.
--
-- # What is Dartmouth BASIC?
--
-- Dartmouth BASIC was invented by John Kemeny and Thomas Kurtz at Dartmouth
-- College in 1964 and first run on the college's General Electric GE-225
-- mainframe. It was designed with a single radical goal: let non-science
-- students write and run programs without needing to be computer scientists.
--
-- Key design choices of the 1964 original:
--   - Line-numbered: every statement must start with a line number (10, 20, ...)
--   - Interactive: the BASIC system ran as a time-sharing service, so students
--     could type programs at a teletype and get results immediately
--   - All uppercase: the GE-225's teletypes only supported uppercase characters
--   - Simple variables: only single-letter names (A–Z) and letter+digit (A0–A9)
--   - All numbers are floating-point internally — even 42 is stored as 42.0
--   - 11 built-in mathematical functions: SIN, COS, TAN, ATN, EXP, LOG, ABS,
--     SQR, INT, RND, SGN
--   - GOTO, GOSUB/RETURN for control flow
--   - FOR/NEXT loops with optional STEP
--
-- # What is Dartmouth BASIC tokenization?
--
-- Given the multi-line program:
--
--   10 LET X = 5
--   20 PRINT X
--   30 END
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(LINE_NUM, "10",    1:1)
--   Token(KEYWORD,  "LET",   1:4)
--   Token(NAME,     "X",     1:8)
--   Token(EQ,       "=",     1:10)
--   Token(NUMBER,   "5",     1:12)
--   Token(NEWLINE,  "\n",    1:13)
--   Token(LINE_NUM, "20",    2:1)
--   Token(KEYWORD,  "PRINT", 2:4)
--   Token(NAME,     "X",     2:10)
--   Token(NEWLINE,  "\n",    2:11)
--   Token(LINE_NUM, "30",    3:1)
--   Token(KEYWORD,  "END",   3:4)
--   Token(NEWLINE,  "\n",    3:7)
--   Token(EOF,      "",      4:1)
--
-- Notable differences from ALGOL or other lexers:
--
--   1. NEWLINE tokens are KEPT (they are statement terminators, not whitespace)
--   2. LINE_NUM is a special token type that only appears at the start of each
--      source line — it is distinguished from NUMBER by position
--   3. REM (remark) tokens suppress all tokens until the next NEWLINE
--   4. The whole source is normalised to uppercase before matching (because the
--      original Dartmouth teletypes had no lowercase characters)
--
-- # Architecture
--
-- This module:
--   1. Requires the pre-compiled `_grammar` module (once, cached).
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Post-processes the raw token list:
--      a. `relabel_line_numbers` — renames the first NUMBER on each source line
--         to LINE_NUM, since line numbers and expression numbers use the same
--         regex and can only be distinguished by position.
--      b. `suppress_rem_content` — drops all tokens after a KEYWORD("REM") until
--         the next NEWLINE, implementing BASIC's comment syntax.
--   5. Returns the processed flat token list.
--
-- Note: the Lua GrammarLexer does NOT have an `add_post_tokenize` method (unlike
-- the Elixir implementation). Post-processing is applied manually after calling
-- `gl:tokenize()`.
--
-- # Grammar source
--
-- The token grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `dartmouth_basic.tokens` is pre-compiled
-- (via `grammar-tools compile-tokens`, after stripping the `(?:...)`
-- non-capturing groups the Lua GrammarLexer cannot parse -- see the note
-- below) into `_grammar.lua`, a plain Lua module that embeds the
-- TokenGrammar as native Lua data structures. That module ships as part of
-- this package, so `require()` always finds it.

local lexer_pkg = require("coding_adventures.lexer")

-- The numeric constant for keyword tokens.  The GrammarLexer returns this
-- value in tok.type when the matched NAME is in the keyword_set.  We use
-- it to distinguish keyword tokens from other NAME tokens because, in the
-- Lua GrammarLexer, keyword tok.type_name is the KEYWORD VALUE (e.g. "LET")
-- rather than the string "KEYWORD".
local TOKEN_TYPE_KEYWORD = lexer_pkg.TokenType.Keyword   -- == 3

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `token_grammar()` result cached in a module-level variable. Subsequent
-- calls to `tokenize` reuse the cached grammar.
--
-- Lua GrammarLexer compatibility note (baked into `_grammar.lua` at compile
-- time, so it no longer needs to happen here on every load):
--
-- The grammar source uses /(?:sin|cos|...|int|...)/ for BUILTIN_FN so that
-- GrammarLexers in other languages (Elixir, Rust, Perl) can prepend a start
-- anchor to the whole group: \A(?:sin|...|int|...) anchors every alternative.
-- The Lua GrammarLexer does NOT support (?:...) — it converts such groups to
-- the wildcard [%s%S]- (any-char), breaking matching entirely. However, the
-- Lua GrammarLexer DOES correctly handle top-level alternation: it splits
-- /sin|cos|...|int|.../ on | and anchors each alternative with ^, giving
-- ^sin, ^cos, ..., ^int, ... tried in order. This is correct behaviour.
-- The `_grammar.lua` module was compiled from a copy of
-- `dartmouth_basic.tokens` with `/(?:alternatives)/` rewritten to
-- `/alternatives/` (the non-capturing group delimiters stripped) before
-- calling `grammar-tools compile-tokens`, so the embedded BUILTIN_FN pattern
-- is already the bare-alternation form the Lua GrammarLexer needs.

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for Dartmouth BASIC.
-- On the first call, requires the pre-compiled `_grammar` module and
-- invokes `token_grammar()`. On subsequent calls, returns the cached
-- TokenGrammar object immediately.
-- @return TokenGrammar  The Dartmouth BASIC token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local compiled = require("coding_adventures.dartmouth_basic_lexer._grammar")
    _grammar_cache = compiled.token_grammar()
    return _grammar_cache
end

-- =========================================================================
-- Post-tokenize transformations
-- =========================================================================
--
-- The Lua GrammarLexer has no add_post_tokenize hook API. We apply both
-- transformations manually after calling gl:tokenize().

--- Re-label the first NUMBER token on each source line as LINE_NUM.
--
-- In Dartmouth BASIC, every program line must start with a line number:
--   10 LET X = 5
--   20 GOTO 10
--
-- Both the line label "10" and the goto target "10" look like numbers.
-- The grammar file defines LINE_NUM with the same regex as NUMBER. This
-- function disambiguates by position: the first NUMBER token seen after
-- a NEWLINE (or at the very start of the source) is promoted to LINE_NUM.
--
-- State machine:
--
--   at_line_start = true   (initial state — we ARE at the start of a line)
--
--   For each token:
--     if at_line_start and type == "NUMBER":
--       → emit LINE_NUM (relabelled copy)
--       → at_line_start = false
--     else if at_line_start:
--       → emit token as-is (e.g., a bare NEWLINE blank line)
--       → at_line_start = false
--     else:
--       → emit token as-is
--
--     if type == "NEWLINE":
--       → at_line_start = true  (next line begins)
--
-- @param tokens  table  The raw token list from GrammarLexer.
-- @return table         The same list with first-NUMBER-per-line relabelled.
local function relabel_line_numbers(tokens)
    local result = {}
    local at_line_start = true
    for _, tok in ipairs(tokens) do
        if at_line_start and tok.type == "NUMBER" then
            result[#result + 1] = {
                type  = "LINE_NUM",
                value = tok.value,
                line  = tok.line,
                col   = tok.col,
            }
            at_line_start = false
        else
            if at_line_start then at_line_start = false end
            result[#result + 1] = tok
        end
        if tok.type == "NEWLINE" then
            at_line_start = true
        end
    end
    return result
end

--- Suppress all tokens between a KEYWORD("REM") and the next NEWLINE.
--
-- In Dartmouth BASIC, REM introduces a remark (comment) that extends
-- to the end of the current source line:
--
--   10 REM THIS IS A COMMENT
--   20 LET X = 1
--
-- After relabeling, this tokenises (raw) as:
--   LINE_NUM("10"), KEYWORD("REM"), NAME("THIS"), NAME("IS"), ...
--
-- This function discards the remark text, leaving only:
--   LINE_NUM("10"), KEYWORD("REM"), NEWLINE
--
-- The REM token itself is kept so the parser / executor can recognise
-- that the line is a comment line (and skip it during execution).
--
-- Algorithm:
--   suppressing = false
--
--   For each token:
--     if not suppressing: emit token
--     if type == KEYWORD and value == "REM": suppressing = true
--     if type == NEWLINE: suppressing = false
--
-- Note: the NEWLINE after REM is NOT suppressed because we exited
-- suppressing mode before the emit guard would have dropped it.
-- Wait — re-reading: when NEWLINE arrives, suppressing is still true
-- when the emit check runs (suppressing = false happens AFTER the emit
-- guard). So NEWLINE while suppressing is NOT emitted. Let's trace:
--
--   Token: KEYWORD("REM")  → not suppressing → emit; set suppressing=true
--   Token: NAME("THIS")    → suppressing → skip; type≠NEWLINE
--   Token: NEWLINE         → suppressing → skip; set suppressing=false
--
-- That means the NEWLINE is also dropped. The spec says:
--   "10 REM THIS IS A COMMENT" → [LINE_NUM("10"), KEYWORD("REM"), NEWLINE, EOF]
--
-- So the NEWLINE should NOT be suppressed. We fix this by checking for
-- NEWLINE BEFORE the emit guard:
--
--   For each token:
--     if type == NEWLINE: suppressing = false
--     if not suppressing: emit token
--     if type == KEYWORD and value == "REM": suppressing = true
--
-- This way:
--   Token: KEYWORD("REM")  → not NEWLINE; not suppressing → emit; set suppressing=true
--   Token: NAME("THIS")    → not NEWLINE; suppressing → skip
--   Token: NEWLINE         → IS NEWLINE → suppressing=false; not suppressing → emit
--
-- @param tokens  table  Token list (after relabel_line_numbers).
-- @return table         Token list with REM remark content removed.
local function suppress_rem_content(tokens)
    local result = {}
    local suppressing = false
    for _, tok in ipairs(tokens) do
        -- Turn off suppression when we reach the end of the REM line,
        -- BEFORE the emit guard, so the NEWLINE itself is preserved.
        if tok.type == "NEWLINE" then
            suppressing = false
        end
        if not suppressing then
            result[#result + 1] = tok
        end
        if tok.type == "KEYWORD" and tok.value == "REM" then
            suppressing = true
        end
    end
    return result
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Dartmouth BASIC source string.
--
-- Loads the `dartmouth_basic.tokens` grammar (cached after first call) and
-- feeds the source to a `GrammarLexer`.  Applies two post-processing passes:
--
--   1. `relabel_line_numbers` — promotes the first NUMBER on each source line
--      to LINE_NUM, implementing the positional disambiguation rule.
--
--   2. `suppress_rem_content` — removes all tokens between a REM keyword and
--      the end of the same source line, implementing BASIC's comment syntax.
--
-- Returns the complete flat token list, including NEWLINE tokens (which are
-- significant in BASIC — they terminate statements) and a terminal EOF token.
--
-- Whitespace (spaces and tabs between tokens) is consumed silently via the
-- skip pattern in `dartmouth_basic.tokens`.  The caller never sees whitespace.
--
-- Because `@case_insensitive true` is set in the grammar, the entire source is
-- uppercased before matching.  `print`, `Print`, and `PRINT` all produce a
-- KEYWORD token with value `"PRINT"`.
--
-- @param source string  The Dartmouth BASIC text to tokenize.
-- @return table         Array of token tables: {type, value, line, col}.
-- @error                Raises an error on grammar loading failure.
--
-- Example:
--
--   local basic = require("coding_adventures.dartmouth_basic_lexer")
--   local tokens = basic.tokenize("10 LET X = 5\n20 PRINT X\n30 END\n")
--   -- tokens[1].type  → "LINE_NUM"
--   -- tokens[1].value → "10"
--   -- tokens[2].type  → "KEYWORD"
--   -- tokens[2].value → "LET"
--   -- tokens[3].type  → "NAME"
--   -- tokens[3].value → "X"
function M.tokenize(source)
    local grammar = get_grammar()

    -- The grammar uses `@case_insensitive true`, which means all patterns are
    -- written in lowercase (e.g. the KEYWORD regex is `restore|gosub|...let...`).
    -- The Lua GrammarLexer's case_insensitive support only adds lowercase keyword
    -- entries to the keyword_set — it does NOT lowercase the source text before
    -- pattern matching. We therefore normalise the source to lowercase ourselves
    -- before tokenising.
    --
    -- This is faithful to the Dartmouth BASIC spec: the 1964 system ran on
    -- uppercase-only teletypes, so `print`, `PRINT`, and `Print` are all the
    -- same statement. After lowercasing, all keyword values will be lowercase
    -- (e.g. "let", "print"), and all NAME values will be lowercase ("x", "a1").
    -- The NEWLINE value is normalised separately (see below).
    --
    -- We then uppercase keyword and NAME values in the normalisation loop so
    -- the caller sees the canonical uppercase form that matches the spec:
    --   KEYWORD("LET") not KEYWORD("let")
    --   NAME("X")      not NAME("x")
    local normalised_source = source:lower()

    local gl  = lexer_pkg.GrammarLexer.new(normalised_source, grammar)
    local raw = gl:tokenize()
    -- Convert the raw GrammarLexer output to a normalised token table.
    -- The GrammarLexer returns objects with fields:
    --   tok.type_name  — the token type string (e.g., "NUMBER", "KEYWORD")
    --   tok.value      — the matched source text (already lowercased)
    --   tok.line       — 1-based source line number
    --   tok.column     — 1-based column number within the line
    --
    -- Uppercasing policy:
    --   KEYWORD tokens  → uppercase value ("let" → "LET")
    --   NAME tokens     → uppercase value ("x" → "X", "a1" → "A1")
    --   BUILTIN_FN      → uppercase value ("sin" → "SIN")
    --   USER_FN         → uppercase value ("fna" → "FNA")
    --   LINE_NUM/NUMBER → value unchanged (digits are not affected by case)
    --   STRING          → value unchanged (string content is case-significant)
    --   NEWLINE         → use "\n" (normalised from the lexer's "\\n" escape)
    --   Everything else → value unchanged
    local tokens = {}
    for _, tok in ipairs(raw) do
        local type_name = tok.type_name
        local value     = tok.value

        -- The built-in lexer emits NEWLINE with value "\\n" (the two-char
        -- escape sequence). We normalise to the actual newline character "\n"
        -- so callers can reliably check tok.value == "\n".
        if type_name == "NEWLINE" then
            value = "\n"
        end

        -- Keyword normalisation.
        --
        -- The Lua GrammarLexer is unusual: when it promotes a NAME to a keyword,
        -- it sets tok.type_name to the KEYWORD VALUE in uppercase (e.g. "LET"),
        -- not to the string "KEYWORD".  The numeric tok.type == TokenType.Keyword
        -- (== 3) is the reliable discriminator.
        --
        -- We unify all keyword tokens so that:
        --   tok.type  == "KEYWORD"   (consistent type string for all keywords)
        --   tok.value == "LET"       (uppercased keyword name)
        --
        -- This matches the spec, which says every keyword token has type
        -- "KEYWORD" and a value that is the uppercase keyword name.
        if tok.type == TOKEN_TYPE_KEYWORD then
            type_name = "KEYWORD"
            -- tok.type_name is already uppercase (e.g. "LET").  tok.value is
            -- the lowercased matched text (e.g. "let").  Use tok.type_name so
            -- we do not have to call value:upper() a second time.
            value = tok.type_name
        end

        -- Uppercase the value for token types that represent identifiers
        -- (these were lowercased as part of case normalisation).
        -- Keywords were already handled above.
        if type_name == "NAME"
        or type_name == "BUILTIN_FN"
        or type_name == "USER_FN" then
            value = value:upper()
        end

        -- For NUMBER tokens with scientific notation, normalise the exponent
        -- separator to uppercase "E". The grammar regex is /[Ee]/, and since
        -- we lowercased the source text, any exponent will be lowercase "e".
        -- The spec and tests expect "1.5E3" not "1.5e3".
        -- We only touch the exponent separator — digits and "-"/"+" are unchanged.
        if type_name == "NUMBER" or type_name == "LINE_NUM" then
            value = value:gsub("[eE]", "E")
        end

        tokens[#tokens + 1] = {
            type  = type_name,
            value = value,
            line  = tok.line,
            col   = tok.column,
        }
    end
    -- Apply post-processing hooks manually (the Lua GrammarLexer has no
    -- add_post_tokenize hook API, unlike the Elixir implementation).
    tokens = relabel_line_numbers(tokens)
    tokens = suppress_rem_content(tokens)
    return tokens
end

--- Return the cached (or freshly loaded) TokenGrammar for Dartmouth BASIC.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Dartmouth BASIC token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
