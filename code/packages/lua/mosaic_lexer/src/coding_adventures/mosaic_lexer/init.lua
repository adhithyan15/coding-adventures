-- mosaic_lexer — Hand-written recursive lexer for the Mosaic language
-- =====================================================================
--
-- # What is Mosaic?
--
-- Mosaic is a Component Description Language (CDL) for declaring UI component
-- structure with named, typed slots. A .mosaic file declares one component
-- with slot types and a visual tree. It compiles to platform-specific code
-- (React TSX, Web Components, SwiftUI, etc.).
--
-- Example Mosaic source:
--
--   component ProfileCard {
--     slot name: text;
--     slot avatar: image;
--     slot active: bool = false;
--
--     Column {
--       Image { source: @avatar; }
--       Text { content: @name; }
--       when @active {
--         Text { content: "Online"; color: #22c55e; }
--       }
--     }
--   }
--
-- # Lexer Output
--
-- The lexer converts source text into a flat stream of typed tokens, each
-- represented as a Lua table:
--
--   { type="COMPONENT", value="component", line=1, col=1 }
--   { type="NAME",      value="ProfileCard", line=1, col=11 }
--   { type="LBRACE",    value="{",  line=1, col=23 }
--   ...
--   { type="EOF",       value="",   line=N, col=M }
--
-- # Token Types
--
-- Keywords (returned as their own type, uppercase):
--   COMPONENT  SLOT  WHEN  EACH  AS  FROM  IMPORT
--
-- Type keywords (returned as KEYWORD type):
--   text  number  bool  image  color  node  list
--   true  false
--
-- Delimiters:
--   LBRACE    {       RBRACE    }
--   LANGLE    <       RANGLE    >
--   COLON     :       SEMICOLON ;
--   AT        @       COMMA     ,
--   DOT       .       EQUALS    =
--
-- Literals:
--   HEX_COLOR     #rgb / #rrggbb / #rrggbbaa
--   DIMENSION     16dp / 1.5sp / 100%
--   NUMBER        42 / -3.14
--   STRING        "hello" (with escape sequences)
--   NAME          identifier (allows hyphens for CSS-style names)
--
-- Skipped silently:
--   Whitespace, // line comments, /* */ block comments
--
-- # Architecture
--
-- The lexer is hand-written — it does NOT use the grammar_tools infrastructure.
-- This is deliberate: Mosaic is a simple, regular language that does not
-- benefit from the overhead of a DFA-based grammar engine. A hand-written
-- lexer is easier to understand and debug, and it runs faster.
--
-- The main entry point is `MosaicLexer.tokenize(source)`:
--   Returns: tokens, nil      on success
--   Returns: nil, error_msg   on lexical error

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- Keyword Tables
-- ============================================================================
--
-- We classify identifiers into three tiers:
--
--   1. Control keywords — get their own distinct token type:
--      component → COMPONENT
--      slot      → SLOT
--      when      → WHEN
--      each      → EACH
--      as        → AS
--      from      → FROM
--      import    → IMPORT
--
--   2. Type/value keywords — returned as KEYWORD type.
--      The downstream parser uses the VALUE to distinguish them:
--      text, number, bool, image, color, node, list, true, false
--
--   3. Plain identifiers — returned as NAME.

-- Maps keyword text → token type for control keywords
local CONTROL_KEYWORDS = {
    component = "COMPONENT",
    slot      = "SLOT",
    when      = "WHEN",
    each      = "EACH",
    as        = "AS",
    from      = "FROM",
    import    = "IMPORT",
}

-- Set of type/value keywords that return token type "KEYWORD"
local TYPE_KEYWORDS = {
    text   = true,
    number = true,
    bool   = true,
    image  = true,
    color  = true,
    node   = true,
    list   = true,
    ["true"]  = true,
    ["false"] = true,
}

-- ============================================================================
-- Lexer State
-- ============================================================================
--
-- The lexer cursor tracks position within the source string:
--   pos  — 1-based byte index into source (Lua string index)
--   line — current line number (increments on newlines)
--   col  — current column (resets to 1 on newlines)

--- Create a new lexer state table.
-- @param source string  The Mosaic source text to tokenize.
-- @return table         A mutable state table used by all lex_ helpers.
local function new_state(source)
    return {
        source = source,
        len    = #source,
        pos    = 1,
        line   = 1,
        col    = 1,
    }
end

--- Peek at the character at `offset` positions ahead (default 0 = current).
-- Returns "" at end of input.
-- @param s      table  Lexer state.
-- @param offset number How many positions ahead to peek (default 0).
-- @return string       Single character, or "" at EOF.
local function peek(s, offset)
    local i = s.pos + (offset or 0)
    if i > s.len then return "" end
    return s.source:sub(i, i)
end

--- Advance the cursor by one character and return it.
-- Updates line/col tracking: newlines bump line and reset col to 1.
-- @param s table  Lexer state.
-- @return string  The character that was consumed.
local function advance(s)
    local ch = s.source:sub(s.pos, s.pos)
    if ch == "\n" then
        s.line = s.line + 1
        s.col  = 1
    else
        s.col = s.col + 1
    end
    s.pos = s.pos + 1
    return ch
end

--- Check if the cursor is at or past the end of input.
-- @param s table  Lexer state.
-- @return boolean True if no more input.
local function at_eof(s)
    return s.pos > s.len
end

-- ============================================================================
-- Skip Helpers
-- ============================================================================

--- Skip a // line comment (consume everything up to but not including \n).
-- Precondition: current and next chars are both '/'.
-- @param s table  Lexer state.
local function skip_line_comment(s)
    -- Consume the '//'
    advance(s); advance(s)
    -- Consume to end of line
    while not at_eof(s) and peek(s) ~= "\n" do
        advance(s)
    end
end

--- Skip a /* block comment. Raises error on unterminated comment.
-- Precondition: current char is '/' and next char is '*'.
-- @param s table  Lexer state.
local function skip_block_comment(s)
    local start_line = s.line
    advance(s); advance(s)  -- consume '/*'
    while not at_eof(s) do
        if peek(s) == "*" and peek(s, 1) == "/" then
            advance(s); advance(s)  -- consume '*/'
            return
        end
        advance(s)
    end
    error(("mosaic_lexer: unterminated block comment starting at line %d"):format(start_line))
end

--- Skip whitespace and comments. Returns after consuming all whitespace/comments.
-- @param s table  Lexer state.
local function skip_whitespace(s)
    while not at_eof(s) do
        local ch = peek(s)
        if ch == " " or ch == "\t" or ch == "\r" or ch == "\n" then
            advance(s)
        elseif ch == "/" and peek(s, 1) == "/" then
            skip_line_comment(s)
        elseif ch == "/" and peek(s, 1) == "*" then
            skip_block_comment(s)
        else
            break
        end
    end
end

-- ============================================================================
-- Token Constructors
-- ============================================================================

--- Build a token table.
-- @param typ   string  Token type (e.g. "NAME", "NUMBER").
-- @param value string  Token text value.
-- @param line  number  Source line number.
-- @param col   number  Source column number.
-- @return table        The token.
local function make_token(typ, value, line, col)
    return { type = typ, value = value, line = line, col = col }
end

-- ============================================================================
-- Specific Token Lexers
-- ============================================================================

--- Lex a string literal: "...", supporting escape sequences.
-- Precondition: current char is '"'.
-- Returns the token with the UNQUOTED content (escapes preserved as-is).
-- @param s table  Lexer state.
-- @return table   STRING token.
local function lex_string(s)
    local start_line = s.line
    local start_col  = s.col
    advance(s)  -- consume opening '"'
    local buf = {}
    while not at_eof(s) do
        local ch = peek(s)
        if ch == '"' then
            advance(s)  -- consume closing '"'
            return make_token("STRING", table.concat(buf), start_line, start_col)
        elseif ch == "\\" then
            -- Preserve escape sequences as-is (interpreter's job to decode)
            local esc = advance(s)  -- consume '\'
            local next_ch = peek(s)
            if at_eof(s) then
                error(("mosaic_lexer: unterminated string escape at line %d:%d"):format(start_line, start_col))
            end
            buf[#buf + 1] = esc
            buf[#buf + 1] = advance(s)  -- consume escaped char
        elseif ch == "\n" then
            error(("mosaic_lexer: unterminated string literal at line %d:%d"):format(start_line, start_col))
        else
            buf[#buf + 1] = advance(s)
        end
    end
    error(("mosaic_lexer: unterminated string literal at line %d:%d"):format(start_line, start_col))
end

--- Lex a hex color: #[0-9a-fA-F]{3,8}
-- Precondition: current char is '#'.
-- @param s table  Lexer state.
-- @return table   HEX_COLOR token.
local function lex_hex_color(s)
    local start_line = s.line
    local start_col  = s.col
    local buf = { advance(s) }  -- consume '#'
    while not at_eof(s) do
        local ch = peek(s)
        if ch:match("[0-9a-fA-F]") then
            buf[#buf + 1] = advance(s)
        else
            break
        end
    end
    return make_token("HEX_COLOR", table.concat(buf), start_line, start_col)
end

--- Lex a number or dimension: [-]?[0-9]*\.?[0-9]+([unit])?
--
-- A DIMENSION is a number immediately followed by a unit suffix (letters or %).
-- A plain NUMBER has no suffix.
--
-- Examples:
--   16dp     → DIMENSION "16dp"
--   1.5sp    → DIMENSION "1.5sp"
--   100%     → DIMENSION "100%"
--   42       → NUMBER    "42"
--   -3.14    → NUMBER    "-3.14"
--
-- Precondition: current char is '-' or a digit.
-- @param s table  Lexer state.
-- @return table   NUMBER or DIMENSION token.
local function lex_number(s)
    local start_line = s.line
    local start_col  = s.col
    local buf = {}

    -- Optional leading minus
    if peek(s) == "-" then
        buf[#buf + 1] = advance(s)
    end

    -- Integer part
    while not at_eof(s) and peek(s):match("[0-9]") do
        buf[#buf + 1] = advance(s)
    end

    -- Optional decimal part
    if peek(s) == "." and peek(s, 1):match("[0-9]") then
        buf[#buf + 1] = advance(s)  -- '.'
        while not at_eof(s) and peek(s):match("[0-9]") do
            buf[#buf + 1] = advance(s)
        end
    end

    -- Check for unit suffix (letters or %)
    local unit_buf = {}
    while not at_eof(s) and (peek(s):match("[a-zA-Z]") or peek(s) == "%") do
        unit_buf[#unit_buf + 1] = advance(s)
    end

    local num_str = table.concat(buf)
    if #unit_buf > 0 then
        return make_token("DIMENSION", num_str .. table.concat(unit_buf), start_line, start_col)
    else
        return make_token("NUMBER", num_str, start_line, start_col)
    end
end

--- Lex an identifier or keyword.
--
-- Identifiers in Mosaic allow hyphens (for CSS-style property names like
-- corner-radius, a11y-label). A hyphen is allowed mid-identifier but not
-- at the start or end.
--
-- After reading the identifier text, we classify it:
--   1. If it's a control keyword → return its specific token type
--   2. If it's a type keyword    → return KEYWORD
--   3. Otherwise                 → return NAME
--
-- Precondition: current char matches [a-zA-Z_].
-- @param s table  Lexer state.
-- @return table   COMPONENT/SLOT/WHEN/EACH/AS/FROM/IMPORT/KEYWORD/NAME token.
local function lex_name(s)
    local start_line = s.line
    local start_col  = s.col
    local buf = {}

    -- First char: must be letter or underscore
    buf[#buf + 1] = advance(s)

    -- Subsequent chars: letter, digit, underscore, or hyphen
    -- (hyphen is only valid if not followed by end of identifier)
    while not at_eof(s) do
        local ch = peek(s)
        if ch:match("[a-zA-Z0-9_]") then
            buf[#buf + 1] = advance(s)
        elseif ch == "-" then
            -- Allow hyphen only if next char is alphanumeric (avoids trailing -)
            local next_ch = peek(s, 1)
            if next_ch and next_ch:match("[a-zA-Z0-9_]") then
                buf[#buf + 1] = advance(s)
            else
                break
            end
        else
            break
        end
    end

    local text = table.concat(buf)

    -- Classify the identifier
    local ctrl = CONTROL_KEYWORDS[text]
    if ctrl then
        return make_token(ctrl, text, start_line, start_col)
    elseif TYPE_KEYWORDS[text] then
        return make_token("KEYWORD", text, start_line, start_col)
    else
        return make_token("NAME", text, start_line, start_col)
    end
end

-- ============================================================================
-- Main Tokenizer
-- ============================================================================

--- Tokenize a Mosaic source string into a flat list of tokens.
--
-- Returns two values:
--   tokens, nil     — success; tokens is an array of token tables
--   nil, errmsg     — failure; errmsg is a human-readable error string
--
-- Each token is a table: { type, value, line, col }
-- The last token is always { type="EOF", value="", ... }
--
-- Example:
--
--   local lexer = require("coding_adventures.mosaic_lexer")
--   local tokens, err = lexer.tokenize('component Foo { Text { } }')
--   -- tokens[1].type  → "COMPONENT"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "Foo"
--
-- @param source string  The Mosaic source text.
-- @return table|nil     Array of tokens, or nil on error.
-- @return nil|string    nil on success, error message on failure.
function M.tokenize(source)
    local ok, result = pcall(function()
        local s = new_state(source)
        local tokens = {}

        while true do
            skip_whitespace(s)

            if at_eof(s) then
                tokens[#tokens + 1] = make_token("EOF", "", s.line, s.col)
                break
            end

            local ch = peek(s)
            local line, col = s.line, s.col

            -- ----------------------------------------------------------------
            -- Single-character structural tokens
            -- ----------------------------------------------------------------
            if ch == "{" then
                advance(s)
                tokens[#tokens + 1] = make_token("LBRACE", "{", line, col)
            elseif ch == "}" then
                advance(s)
                tokens[#tokens + 1] = make_token("RBRACE", "}", line, col)
            elseif ch == "<" then
                advance(s)
                tokens[#tokens + 1] = make_token("LANGLE", "<", line, col)
            elseif ch == ">" then
                advance(s)
                tokens[#tokens + 1] = make_token("RANGLE", ">", line, col)
            elseif ch == ":" then
                advance(s)
                tokens[#tokens + 1] = make_token("COLON", ":", line, col)
            elseif ch == ";" then
                advance(s)
                tokens[#tokens + 1] = make_token("SEMICOLON", ";", line, col)
            elseif ch == "@" then
                advance(s)
                tokens[#tokens + 1] = make_token("AT", "@", line, col)
            elseif ch == "," then
                advance(s)
                tokens[#tokens + 1] = make_token("COMMA", ",", line, col)
            elseif ch == "." then
                advance(s)
                tokens[#tokens + 1] = make_token("DOT", ".", line, col)
            elseif ch == "=" then
                advance(s)
                tokens[#tokens + 1] = make_token("EQUALS", "=", line, col)

            -- ----------------------------------------------------------------
            -- String literals
            -- ----------------------------------------------------------------
            elseif ch == '"' then
                tokens[#tokens + 1] = lex_string(s)

            -- ----------------------------------------------------------------
            -- Hex colors: #rrggbb
            -- ----------------------------------------------------------------
            elseif ch == "#" then
                tokens[#tokens + 1] = lex_hex_color(s)

            -- ----------------------------------------------------------------
            -- Numbers and dimensions: 42 / -3.14 / 16dp / 100%
            -- Note: a bare '-' that is NOT followed by a digit is an error.
            -- ----------------------------------------------------------------
            elseif ch:match("[0-9]") then
                tokens[#tokens + 1] = lex_number(s)
            elseif ch == "-" and peek(s, 1):match("[0-9]") then
                tokens[#tokens + 1] = lex_number(s)

            -- ----------------------------------------------------------------
            -- Identifiers and keywords
            -- ----------------------------------------------------------------
            elseif ch:match("[a-zA-Z_]") then
                tokens[#tokens + 1] = lex_name(s)

            -- ----------------------------------------------------------------
            -- Unknown character → lexical error
            -- ----------------------------------------------------------------
            else
                error(
                    ("mosaic_lexer: unexpected character %q at line %d:%d"):format(
                        ch, line, col
                    )
                )
            end
        end

        return tokens
    end)

    if ok then
        return result, nil
    else
        return nil, result
    end
end

return M
