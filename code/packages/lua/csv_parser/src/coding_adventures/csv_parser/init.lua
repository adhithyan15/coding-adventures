-- csv_parser — RFC 4180 state-machine CSV parser
--
-- CSV (Comma-Separated Values) is one of the most ubiquitous data interchange
-- formats ever invented: spreadsheets, databases, data pipelines, and log files
-- all use it. Despite its apparent simplicity, CSV has many edge cases that trip
-- up naive implementations:
--
--   * Fields containing the delimiter: "hello, world" must be quoted → `"hello, world"`
--   * Fields containing quote characters: a literal " must be doubled → `"say ""hi"""`
--   * Fields containing newlines: legal in RFC 4180 quoted fields
--   * Mixed line endings: CRLF (\r\n) is the RFC standard, but UNIX (\n) and
--     old Mac (\r) line endings are common in practice
--   * Empty fields: a,,c means the second field is the empty string ""
--
-- This parser implements the RFC 4180 grammar precisely using a four-state
-- finite automaton, the same design used by many production CSV libraries:
--
--   FIELD_START        — beginning of a new field (or start of row)
--   IN_UNQUOTED_FIELD  — inside a field that did NOT start with a quote
--   IN_QUOTED_FIELD    — inside a field enclosed in double-quotes
--   IN_QUOTED_MAYBE_END — just saw a closing quote; might be end or escaped quote
--
-- State transition diagram (delimiter = ',', quote = '"'):
--
--   FIELD_START ──── '"' ──────────────────────────► IN_QUOTED_FIELD
--                 ── ',' ──────────────────────────► FIELD_START  (emit empty field)
--                 ── '\n'/'\r' ────────────────────► FIELD_START  (emit row)
--                 ── other ────────────────────────► IN_UNQUOTED_FIELD
--
--   IN_UNQUOTED_FIELD ── ',' ───────────────────────► FIELD_START  (emit field)
--                      ── '\n'/'\r' ─────────────────► FIELD_START  (emit row)
--                      ── other ─────────────────────► IN_UNQUOTED_FIELD (accumulate)
--
--   IN_QUOTED_FIELD ── '"' ─────────────────────────► IN_QUOTED_MAYBE_END
--                    ── other (including ',' '\n') ──► IN_QUOTED_FIELD (accumulate)
--
--   IN_QUOTED_MAYBE_END ── '"' ─────────────────────► IN_QUOTED_FIELD (emit '"', continue)
--                        ── ',' ─────────────────────► FIELD_START  (emit field)
--                        ── '\n'/'\r' ───────────────► FIELD_START  (emit row)
--                        ── EOF ─────────────────────► (emit field, done)
--
-- Usage:
--
--   local csv = require("coding_adventures.csv_parser")
--
--   local rows = csv.parse("a,b,c\n1,2,3")
--   -- rows[1] == {"a", "b", "c"}
--   -- rows[2] == {"1", "2", "3"}
--
--   local rows = csv.parse("a;b;c", {delimiter = ";"})
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- State constants
-- These names make the automaton code self-documenting.
-- ---------------------------------------------------------------------------

local FIELD_START        = 1  -- waiting for the first character of a new field
local IN_UNQUOTED_FIELD  = 2  -- consuming chars of an unquoted field
local IN_QUOTED_FIELD    = 3  -- consuming chars inside "…"
local IN_QUOTED_MAYBE_END = 4 -- saw a '"' inside a quoted field — escape or end?

-- ---------------------------------------------------------------------------
-- parse(text, opts) → table of rows
--
-- Parses `text` as RFC 4180 CSV and returns a list of rows. Each row is a
-- list of field strings (1-indexed, as Lua convention demands).
--
-- Options (optional table):
--   delimiter  — single-character field separator (default: ",")
--
-- All three line-ending conventions are handled:
--   \r\n  (Windows / RFC 4180 canonical)
--   \n    (Unix)
--   \r    (old Mac)
--
-- Empty input returns an empty table {}.
-- A trailing newline does NOT produce an extra empty row.
-- ---------------------------------------------------------------------------
function M.parse(text, opts)
    opts = opts or {}
    local delimiter = opts.delimiter or ","

    -- Validate delimiter: must be exactly one character
    if type(delimiter) ~= "string" or #delimiter ~= 1 then
        error("csv_parser: delimiter must be a single character, got: " .. tostring(delimiter))
    end

    -- We'll accumulate characters into `field` and push completed fields into
    -- `current_row`. Completed rows go into `rows`.
    local rows       = {}   -- list of completed rows
    local current_row = {}  -- fields in the row being built
    local field      = {}   -- characters of the field being built (as a char table)
    local state      = FIELD_START
    local n          = #text

    -- Helper: flush the current field into current_row
    local function push_field()
        current_row[#current_row + 1] = table.concat(field)
        field = {}
    end

    -- Helper: flush current_row into rows and start a new row
    local function push_row()
        push_field()
        rows[#rows + 1] = current_row
        current_row = {}
    end

    local i = 1
    while i <= n do
        local ch = text:sub(i, i)

        if state == FIELD_START then
            -- ----------------------------------------------------------------
            -- FIELD_START: We are at the beginning of a new field.
            -- ----------------------------------------------------------------
            if ch == '"' then
                -- Quoted field begins: transition into quoted mode
                state = IN_QUOTED_FIELD
            elseif ch == delimiter then
                -- Empty field followed by delimiter: emit "" and stay in FIELD_START
                push_field()
            elseif ch == "\r" then
                -- CR: end of row. Peek ahead to consume optional LF (CRLF).
                push_row()
                if i + 1 <= n and text:sub(i + 1, i + 1) == "\n" then
                    i = i + 1  -- skip the LF of CRLF
                end
            elseif ch == "\n" then
                -- LF: end of row
                push_row()
            else
                -- Regular character: start an unquoted field
                field[#field + 1] = ch
                state = IN_UNQUOTED_FIELD
            end

        elseif state == IN_UNQUOTED_FIELD then
            -- ----------------------------------------------------------------
            -- IN_UNQUOTED_FIELD: Accumulating a plain (non-quoted) field.
            -- ----------------------------------------------------------------
            if ch == delimiter then
                -- Delimiter ends this field
                push_field()
                state = FIELD_START
            elseif ch == "\r" then
                -- CR ends the row
                push_row()
                state = FIELD_START
                if i + 1 <= n and text:sub(i + 1, i + 1) == "\n" then
                    i = i + 1
                end
            elseif ch == "\n" then
                -- LF ends the row
                push_row()
                state = FIELD_START
            else
                -- Ordinary character: accumulate into the field buffer
                field[#field + 1] = ch
            end

        elseif state == IN_QUOTED_FIELD then
            -- ----------------------------------------------------------------
            -- IN_QUOTED_FIELD: Inside "…". Almost everything is literal here.
            -- The only special character is '"', which either ends the field
            -- or (if doubled) represents a literal quote.
            -- ----------------------------------------------------------------
            if ch == '"' then
                -- Could be end-of-field or escaped quote (""). Defer decision.
                state = IN_QUOTED_MAYBE_END
            else
                -- Everything else (including commas, newlines) is literal content
                field[#field + 1] = ch
            end

        elseif state == IN_QUOTED_MAYBE_END then
            -- ----------------------------------------------------------------
            -- IN_QUOTED_MAYBE_END: We just saw a '"' inside a quoted field.
            -- If the next character is also '"', this is an escaped quote.
            -- Otherwise, the field has ended.
            -- ----------------------------------------------------------------
            if ch == '"' then
                -- Escaped quote: emit a literal '"' and continue in quoted mode
                field[#field + 1] = '"'
                state = IN_QUOTED_FIELD
            elseif ch == delimiter then
                -- Field ended normally, delimiter follows
                push_field()
                state = FIELD_START
            elseif ch == "\r" then
                -- Field ended, CR ends the row
                push_row()
                state = FIELD_START
                if i + 1 <= n and text:sub(i + 1, i + 1) == "\n" then
                    i = i + 1
                end
            elseif ch == "\n" then
                -- Field ended, LF ends the row
                push_row()
                state = FIELD_START
            else
                -- RFC 4180 says characters after a closing quote (before
                -- delimiter/newline/EOF) are technically malformed, but many
                -- real-world parsers accept them.  We accept them too.
                field[#field + 1] = ch
                state = IN_UNQUOTED_FIELD
            end
        end

        i = i + 1
    end

    -- -----------------------------------------------------------------------
    -- End-of-input cleanup
    --
    -- After the loop, we may have a partially-built row that was never
    -- terminated by a newline (valid: the last row often omits the trailing
    -- newline).  We also need to handle the edge case where IN_QUOTED_MAYBE_END
    -- is the final state (the field ended with a closing quote at EOF).
    --
    -- Exception: if the very last character was a newline and we just pushed
    -- a row, `current_row` will be empty and `field` will be empty.  In that
    -- case we should NOT emit a spurious empty trailing row.
    -- -----------------------------------------------------------------------
    if state == IN_QUOTED_MAYBE_END then
        -- Closing quote was the last character: emit the field
        push_field()
        if #current_row > 0 or #field > 0 then
            rows[#rows + 1] = current_row
        end
    elseif #current_row > 0 or #field > 0 then
        -- There is content in the last (unterminated) row
        push_field()
        rows[#rows + 1] = current_row
    end
    -- If current_row is empty AND field is empty, we were in FIELD_START right
    -- after a newline: the trailing newline is not a new row, so we do nothing.

    return rows
end

return M
