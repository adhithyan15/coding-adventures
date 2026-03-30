-- json_value — JSON AST evaluator and serializer
-- ================================================
--
-- This package is part of the coding-adventures monorepo.  It sits one layer
-- above `json_parser`: whereas `json_parser` produces an Abstract Syntax Tree
-- (AST) that faithfully mirrors the JSON grammar, `json_value` *evaluates*
-- that AST into native Lua data structures.
--
-- # The evaluation model
--
-- JSON has six value types.  After evaluation, each maps to a Lua equivalent:
--
--   JSON type   │  Lua type
--   ────────────┼────────────────────────────────
--   object      │  table   (string keys)
--   array       │  table   (integer keys 1..N)
--   string      │  string
--   number      │  number  (integer or float)
--   boolean     │  boolean (true / false)
--   null        │  M.null  (unique sentinel table)
--
-- # The null problem
--
-- Lua has `nil`, but `nil` cannot be stored in a table — assigning `t[k]=nil`
-- removes the key entirely.  JSON `null` must survive a round-trip through a
-- Lua table, so we use a *sentinel*: a unique empty table with a custom
-- `__tostring` metamethod.  `M.is_null(v)` tests for this sentinel using
-- identity comparison (`v == M.null`).
--
-- # Serialization
--
-- `M.to_json(value, indent)` walks native Lua values and produces a JSON
-- string.  It distinguishes arrays (consecutive integer keys starting at 1)
-- from objects (all other tables).  The optional `indent` argument enables
-- pretty-printing: each level is indented by `indent` spaces.
--
-- # Architecture
--
--   json_value  ← this package
--        ↓
--   json_parser  (provides parse() → ASTNode)
--        ↓
--   parser, grammar_tools, json_lexer, lexer, state_machine, directed_graph

local json_parser = require("coding_adventures.json_parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Null sentinel
-- =========================================================================
--
-- JSON `null` cannot be represented as Lua `nil` inside a table.  We create
-- a unique table blessed with a `__tostring` metamethod so that
-- `tostring(json_value.null)` gives `"null"` rather than the default
-- `"table: 0x..."`.
--
-- Callers should *never* create their own copies; always use `M.null` directly.
-- Identity comparison (`v == M.null`) is the only reliable test.

M.null = setmetatable({}, {
    __tostring = function() return "null" end,
})

-- =========================================================================
-- is_null(v)
-- =========================================================================
--
-- Returns true when `v` is the JSON null sentinel.
--
-- Example:
--   local v = json_value.from_string("null")
--   json_value.is_null(v)   -- true
--   json_value.is_null(nil) -- false (Lua nil ≠ JSON null)
--
-- @param v  any  The value to test.
-- @return boolean

function M.is_null(v)
    return v == M.null
end

-- =========================================================================
-- Internal: string unescaping
-- =========================================================================
--
-- JSON strings are enclosed in double quotes and may contain escape sequences:
--
--   \"  →  "     (double quote)
--   \\  →  \     (backslash)
--   \/  →  /     (forward slash — allowed per spec, rarely needed)
--   \n  →  LF    (line feed, U+000A)
--   \t  →  HT    (horizontal tab, U+0009)
--   \r  →  CR    (carriage return, U+000D)
--   \f  →  FF    (form feed, U+000C)
--   \b  →  BS    (backspace, U+0008)
--   \uXXXX → the Unicode code point encoded as UTF-8
--
-- We strip the surrounding quotes, then apply each escape in a single pass
-- using `gsub`.
--
-- # Why handle \uXXXX?
--
-- JSON is a Unicode text format.  `\uXXXX` is the only way to represent code
-- points that cannot appear literally in the source.  We convert them to
-- UTF-8 bytes so that the resulting Lua string is a valid UTF-8 encoded
-- Unicode string — which is what most Lua consumers expect.

--- Convert a Unicode code point (integer) to its UTF-8 byte sequence.
-- UTF-8 encoding rules:
--
--   0x0000–0x007F → 1 byte:  0xxxxxxx
--   0x0080–0x07FF → 2 bytes: 110xxxxx 10xxxxxx
--   0x0800–0xFFFF → 3 bytes: 1110xxxx 10xxxxxx 10xxxxxx
--
-- (We only need BMP code points since JSON \uXXXX is limited to U+0000–U+FFFF.)
--
-- @param cp  number  A Unicode code point in range 0–0xFFFF.
-- @return string     The UTF-8 encoding of that code point.
local function codepoint_to_utf8(cp)
    if cp < 0x80 then
        -- 7-bit ASCII: one byte unchanged
        return string.char(cp)
    elseif cp < 0x800 then
        -- Two-byte sequence: 110xxxxx 10xxxxxx
        return string.char(
            0xC0 + math.floor(cp / 0x40),
            0x80 + (cp % 0x40)
        )
    else
        -- Three-byte sequence: 1110xxxx 10xxxxxx 10xxxxxx
        return string.char(
            0xE0 + math.floor(cp / 0x1000),
            0x80 + math.floor((cp % 0x1000) / 0x40),
            0x80 + (cp % 0x40)
        )
    end
end

--- Unescape a raw JSON string token value into a plain Lua string.
--
-- `raw` is the token value as stored by the lexer.  The `json.tokens` grammar
-- sets `escapes: none`, which tells the GrammarLexer to strip the surrounding
-- double-quote characters but leave all `\X` escape sequences as literal text.
-- So `raw` is already unquoted: for JSON source `"hello\nworld"`, the token
-- value is `hello\nworld` (no enclosing quotes, backslash-n still two chars).
--
-- Steps:
--   1. Replace each recognised `\X` escape with its decoded equivalent.
--   2. Replace `\uXXXX` Unicode escapes with UTF-8 byte sequences.
--
-- @param raw  string  The raw JSON string token (quotes already stripped by lexer).
-- @return string      The decoded Lua string.
local function unescape_string(raw)
    -- Quotes are already stripped by the GrammarLexer (escapes: none mode).
    local s = raw

    -- Replace each escape sequence in a single gsub pass.
    -- The pattern `\\(.)` matches a literal backslash followed by any char.
    -- The replacement function is called with just the char after the slash.
    s = s:gsub("\\(.)", function(c)
        if c == '"'  then return '"'
        elseif c == "\\" then return "\\"
        elseif c == "/"  then return "/"
        elseif c == "n"  then return "\n"
        elseif c == "t"  then return "\t"
        elseif c == "r"  then return "\r"
        elseif c == "f"  then return "\f"
        elseif c == "b"  then return "\b"
        else
            -- Not a recognised single-character escape.
            -- Must not happen for well-formed JSON (the lexer won't produce it),
            -- but we fall through and keep the backslash + char.
            return "\\" .. c
        end
    end)

    -- Handle \uXXXX sequences separately: they are four hex digits.
    -- We use a two-pass approach: after the simple escapes above, scan for
    -- any remaining `\uXXXX` patterns (the `\\` above already stripped the
    -- leading backslash, so we now look for the literal `u` left behind).
    --
    -- Wait — after the gsub above, `\u0041` becomes `u0041` (the `\` was
    -- consumed but `u` was not a recognised escape so we returned `\u`).
    -- We re-scan for `\uXXXX` directly in the *original* replacement output.
    --
    -- Actually: the gsub above matches `\\(.)` and returns `\\ .. c` for
    -- unknown escapes, so `\u0041` → `\u0041` (preserved).  A second pass
    -- can now match `\uXXXX`.
    s = s:gsub("\\u(%x%x%x%x)", function(hex)
        local cp = tonumber(hex, 16)
        return codepoint_to_utf8(cp)
    end)

    return s
end

-- =========================================================================
-- evaluate(ast)
-- =========================================================================
--
-- Recursively walk an ASTNode (produced by `json_parser.parse`) and convert
-- it to a native Lua value.
--
-- # Walking the AST
--
-- The AST produced by `json_parser` mirrors the JSON grammar exactly:
--
--   value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL
--   object = LBRACE [ pair { COMMA pair } ] RBRACE
--   pair   = STRING COLON value
--   array  = LBRACKET [ value { COMMA value } ] RBRACKET
--
-- Each ASTNode has:
--   node.rule_name  — the grammar rule ("value", "object", "pair", "array")
--                     or "token" for leaf nodes
--   node.children   — array of child ASTNodes
--   node:is_leaf()  — true when the node wraps a single token
--   node:token()    — the wrapped token (only valid for leaf nodes)
--
-- # Dispatch strategy
--
-- We dispatch on `rule_name`.  Each case knows exactly which children are
-- semantically interesting and which are punctuation (brackets, commas, etc.)
-- that can be ignored during evaluation.

--- Evaluate an ASTNode and return the corresponding native Lua value.
--
-- @param ast  ASTNode  Root (or sub-root) of the JSON AST.
-- @return     any      Native Lua value (table, string, number, boolean, M.null).
-- @error               Raises if the AST contains an unrecognised node type.
function M.evaluate(ast)
    local rule = ast.rule_name

    -- ------------------------------------------------------------------
    -- "value" node
    --
    -- A value node always has exactly one semantically meaningful child:
    --   - another rule node (object, array), or
    --   - a leaf token node (STRING, NUMBER, TRUE, FALSE, NULL)
    --
    -- The value node is just a wrapper; we skip straight to the child.
    -- ------------------------------------------------------------------
    if rule == "value" then
        -- Find the first non-punctuation child.  In the grammar-driven AST,
        -- value has exactly one child: the matched alternative.
        --
        -- The child is either:
        --   (a) an ASTNode (rule match for "object" or "array") — has rule_name
        --   (b) a raw token table (token match for STRING/NUMBER/TRUE/FALSE/NULL)
        --       — has "type" and "value" fields, but NO rule_name
        for _, child in ipairs(ast.children) do
            if type(child) ~= "table" then goto next_value_child end
            if child.rule_name then
                return M.evaluate(child)
            elseif child.type then
                -- raw token child (STRING, NUMBER, TRUE, FALSE, NULL)
                local ttype = child.type
                local tval  = child.value
                if ttype == "STRING" then
                    return unescape_string(tval)
                elseif ttype == "NUMBER" then
                    return tonumber(tval)
                elseif ttype == "TRUE" then
                    return true
                elseif ttype == "FALSE" then
                    return false
                elseif ttype == "NULL" then
                    return M.null
                end
            end
            ::next_value_child::
        end
        error("json_value.evaluate: empty value node")

    -- ------------------------------------------------------------------
    -- "object" node
    --
    -- Structure: LBRACE [ pair { COMMA pair } ] RBRACE
    --
    -- We iterate the children, skipping LBRACE, RBRACE, and COMMA tokens
    -- (which are leaf nodes with rule_name "token").  Only "pair" children
    -- are semantically meaningful.
    -- ------------------------------------------------------------------
    elseif rule == "object" then
        local result = {}
        for _, child in ipairs(ast.children) do
            if type(child) == "table" and child.rule_name == "pair" then
                -- Delegate pair extraction to the "pair" branch below.
                local k, v = M._evaluate_pair(child)
                result[k] = v
            end
        end
        return result

    -- ------------------------------------------------------------------
    -- "pair" node
    --
    -- Structure: STRING COLON value
    --
    -- The pair node has three children:
    --   [1] leaf token (STRING) — the key, with surrounding quotes
    --   [2] leaf token (COLON)  — punctuation, ignored
    --   [3] value node          — the associated value
    --
    -- We expose this as a helper so that the object case can call it,
    -- but it can also be called directly for testing.
    -- ------------------------------------------------------------------
    elseif rule == "pair" then
        -- Standalone evaluate of a pair returns a table {key, value}.
        -- Normally called via _evaluate_pair which returns two values.
        local k, v = M._evaluate_pair(ast)
        return { k, v }

    -- ------------------------------------------------------------------
    -- "array" node
    --
    -- Structure: LBRACKET [ value { COMMA value } ] RBRACKET
    --
    -- We iterate children, skipping the bracket and comma leaf tokens.
    -- Only "value" children are collected.
    -- ------------------------------------------------------------------
    elseif rule == "array" then
        local result = {}
        for _, child in ipairs(ast.children) do
            if type(child) == "table" and child.rule_name == "value" then
                result[#result + 1] = M.evaluate(child)
            end
        end
        return result

    -- ------------------------------------------------------------------
    -- Leaf token nodes
    --
    -- A leaf node wraps a single lexer token.  We dispatch on the token
    -- type to produce the appropriate Lua value.
    -- ------------------------------------------------------------------
    elseif rule == "token" then
        local tok = type(ast.token) == "function" and ast:token() or ast.token
        if not tok then
            -- Fallback: some AST implementations store token directly
            tok = ast._token or ast[1]
        end
        if not tok then
            error("json_value.evaluate: leaf node has no token")
        end

        local ttype = tok.type
        local tval  = tok.value

        if ttype == "STRING" then
            return unescape_string(tval)

        elseif ttype == "NUMBER" then
            -- JSON numbers are either integers or floats.
            -- `tonumber` handles both, as well as scientific notation (1e10).
            return tonumber(tval)

        elseif ttype == "TRUE" then
            return true

        elseif ttype == "FALSE" then
            return false

        elseif ttype == "NULL" then
            return M.null

        else
            -- Punctuation tokens (LBRACE, RBRACE, COMMA, etc.) should never
            -- reach here during normal evaluation — they appear as children
            -- of object/array/pair nodes but are skipped by those handlers.
            error("json_value.evaluate: unexpected token type: " .. tostring(ttype))
        end

    else
        error("json_value.evaluate: unknown rule_name: " .. tostring(rule))
    end
end

--- Extract the key and value from a "pair" ASTNode.
--
-- This is a private helper called by the "object" evaluation branch.
-- Returning two values (key, value) is more efficient than building an
-- intermediate table.
--
-- @param pair_node  ASTNode  A node with rule_name "pair".
-- @return string, any        The decoded key string and evaluated value.
function M._evaluate_pair(pair_node)
    -- Children of a pair node (in order): STRING leaf, COLON leaf, value node
    local key_node   = nil
    local value_node = nil

    for _, child in ipairs(pair_node.children) do
        if type(child) ~= "table" then goto continue end

        if child.rule_name then
            -- ASTNode child (rule match): only "value" is semantically meaningful.
            if child.rule_name == "value" then
                value_node = child
            end
        else
            -- Raw token child (token match): STRING is the key; COLON is ignored.
            if child.type == "STRING" and not key_node then
                key_node = child
            end
        end

        ::continue::
    end

    if not key_node then
        error("json_value._evaluate_pair: no STRING key found in pair node")
    end
    if not value_node then
        error("json_value._evaluate_pair: no value found in pair node")
    end

    -- key_node is a raw token table — access .value directly
    local key = unescape_string(key_node.value)
    local val = M.evaluate(value_node)
    return key, val
end

-- =========================================================================
-- from_string(json_str)
-- =========================================================================
--
-- Convenience function that combines parsing and evaluation in one call.
--
-- Internally:
--   1. Calls `json_parser.parse(json_str)` to get the root ASTNode.
--   2. Calls `M.evaluate(ast)` to convert it to a native Lua value.
--
-- This is the primary entry point for most callers — they do not need to
-- know about the intermediate AST.
--
-- @param json_str  string  A JSON-encoded string.
-- @return any              The decoded native Lua value.
-- @error                   Raises on lexer, parser, or evaluator errors.
--
-- Example:
--
--   local jv = require("coding_adventures.json_value")
--   local t = jv.from_string('{"name": "Alice", "age": 30}')
--   print(t.name)   -- Alice
--   print(t.age)    -- 30

function M.from_string(json_str)
    local ast = json_parser.parse(json_str)
    return M.evaluate(ast)
end

-- =========================================================================
-- to_json(value, indent)
-- =========================================================================
--
-- Serialize a native Lua value back to a JSON string.
--
-- # Type mapping (reverse of evaluate)
--
--   Lua type              │  JSON output
--   ──────────────────────┼────────────────────────────
--   nil                   │  "null"
--   M.null (sentinel)     │  "null"
--   boolean               │  "true" or "false"
--   number (integer)      │  decimal integer, e.g. "42"
--   number (float)        │  decimal float, e.g. "3.14"
--   string                │  double-quoted, with escapes
--   table (array-like)    │  JSON array  [...]
--   table (object-like)   │  JSON object {...}
--
-- # Array detection
--
-- A Lua table is treated as a JSON *array* when:
--   - It is non-empty, AND
--   - All keys are consecutive integers starting at 1
--     (i.e. 1, 2, 3, …, #t — the standard Lua sequence)
--
-- Any other table (including empty tables) is treated as a JSON *object*.
--
-- Note: empty `{}` → `"{}"`.  If you need an empty array, represent it with
-- a table that happens to be detected as an array (e.g. `{[1]=...}`), but
-- there is no reliable way to distinguish an intended-empty-array from an
-- intended-empty-object at the Lua level.  This is a fundamental limitation
-- of storing JSON arrays as Lua tables.
--
-- # Pretty-printing
--
-- When `indent` is a positive integer, each nested level is indented by
-- `indent` additional spaces.  `indent = 0` (or nil) gives compact output.
--
-- @param value   any      The Lua value to serialize.
-- @param indent  number   Spaces per indentation level (default 0 = compact).
-- @param _depth  number   (internal) Current nesting depth; do not pass.
-- @return string          JSON-encoded string.

function M.to_json(value, indent, _depth)
    -- Normalise indent: nil → 0 (compact mode)
    indent = indent or 0
    -- _depth tracks the current nesting level for pretty-printing.
    _depth = _depth or 0

    -- ------------------------------------------------------------------
    -- nil and the null sentinel → JSON null
    -- ------------------------------------------------------------------
    if value == nil or value == M.null then
        return "null"

    -- ------------------------------------------------------------------
    -- boolean
    -- ------------------------------------------------------------------
    elseif type(value) == "boolean" then
        return value and "true" or "false"

    -- ------------------------------------------------------------------
    -- number
    --
    -- Lua does not distinguish integers from floats at the language level
    -- (both are `number`), but JSON consumers expect integers without a
    -- decimal point.
    --
    -- Strategy:
    --   - If the number equals its own floor (i.e. no fractional part) AND
    --     is within the safe integer range (±2^53), format it as an integer.
    --   - Otherwise use Lua's default `tostring`, which gives "3.14", "-1.5",
    --     or scientific notation for very large/small values.
    --
    -- We also handle the special case of `-0`: JSON has no negative zero, so
    -- we output `0`.
    -- ------------------------------------------------------------------
    elseif type(value) == "number" then
        -- Check for NaN and infinity (not valid in JSON)
        if value ~= value then   -- NaN check: NaN is not equal to itself
            return "null"        -- JSON has no NaN; map to null
        end
        if value == math.huge or value == -math.huge then
            return "null"        -- JSON has no Infinity; map to null
        end
        -- Integer vs float detection
        if math.floor(value) == value and math.abs(value) < 2^53 then
            return string.format("%d", value)
        else
            -- `tostring` in Lua 5.3+ uses up to 14 significant digits.
            -- For older versions, %.14g gives a compact representation.
            return string.format("%.14g", value)
        end

    -- ------------------------------------------------------------------
    -- string
    --
    -- JSON strings must:
    --   - Be enclosed in double quotes.
    --   - Escape: " → \", \ → \\, control characters (U+0000–U+001F).
    --
    -- We also escape U+0008 (BS), U+0009 (HT), U+000A (LF), U+000C (FF),
    -- U+000D (CR) with their short-form escapes for readability.
    -- ------------------------------------------------------------------
    elseif type(value) == "string" then
        local escaped = value
            :gsub("\\", "\\\\")    -- \ → \\ (must come first!)
            :gsub('"',  '\\"')     -- " → \"
            :gsub("\b", "\\b")     -- backspace
            :gsub("\f", "\\f")     -- form feed
            :gsub("\n", "\\n")     -- newline
            :gsub("\r", "\\r")     -- carriage return
            :gsub("\t", "\\t")     -- tab

        -- Escape remaining control characters (U+0000–U+001F) that do not
        -- have a short-form JSON escape.
        escaped = escaped:gsub("[\x00-\x1f]", function(c)
            return string.format("\\u%04x", c:byte(1))
        end)

        return '"' .. escaped .. '"'

    -- ------------------------------------------------------------------
    -- table: could be a JSON array or a JSON object
    -- ------------------------------------------------------------------
    elseif type(value) == "table" then
        return M._table_to_json(value, indent, _depth)

    -- ------------------------------------------------------------------
    -- Unsupported Lua types: function, userdata, thread, etc.
    -- JSON has no representation for these; map to null.
    -- ------------------------------------------------------------------
    else
        return "null"
    end
end

--- Serialize a Lua table to a JSON array or object string.
-- This is an internal helper extracted from `to_json` for clarity.
--
-- @param tbl     table   The Lua table to serialize.
-- @param indent  number  Spaces per indentation level.
-- @param depth   number  Current nesting depth.
-- @return string         JSON string (array or object syntax).
function M._table_to_json(tbl, indent, depth)
    -- Determine whether the table looks like a JSON array.
    -- A Lua "sequence" has keys 1, 2, 3, …, n with no gaps.
    -- An empty table is treated as an object (see note in to_json docstring).
    local n = #tbl           -- Lua's length operator counts sequence elements
    local is_array = n > 0   -- non-empty AND…

    if is_array then
        -- Verify there are no extra (non-integer) keys.
        -- If any non-sequence key exists, treat as object.
        local count = 0
        for _ in pairs(tbl) do count = count + 1 end
        is_array = (count == n)
    end

    if is_array then
        return M._array_to_json(tbl, n, indent, depth)
    else
        return M._object_to_json(tbl, indent, depth)
    end
end

--- Serialize a Lua sequence table to a JSON array string.
-- @param tbl     table   Array-like Lua table.
-- @param n       number  Length of the array.
-- @param indent  number  Spaces per indentation level.
-- @param depth   number  Current nesting depth.
-- @return string
function M._array_to_json(tbl, n, indent, depth)
    if n == 0 then
        return "[]"
    end

    local items = {}
    for i = 1, n do
        items[i] = M.to_json(tbl[i], indent, depth + 1)
    end

    if indent > 0 then
        -- Pretty-print: one element per line, indented
        local inner_pad = string.rep(" ", indent * (depth + 1))
        local outer_pad = string.rep(" ", indent * depth)
        return "[\n" .. inner_pad
            .. table.concat(items, ",\n" .. inner_pad)
            .. "\n" .. outer_pad .. "]"
    else
        return "[" .. table.concat(items, ",") .. "]"
    end
end

--- Serialize a Lua table (non-sequence) to a JSON object string.
-- Keys are sorted for deterministic output.
-- @param tbl     table   Object-like Lua table.
-- @param indent  number  Spaces per indentation level.
-- @param depth   number  Current nesting depth.
-- @return string
function M._object_to_json(tbl, indent, depth)
    -- Collect all string keys and sort them for deterministic output.
    -- Non-string keys (integers not forming a sequence, etc.) are ignored —
    -- JSON object keys must be strings.
    local keys = {}
    for k in pairs(tbl) do
        if type(k) == "string" then
            keys[#keys + 1] = k
        end
    end
    table.sort(keys)

    if #keys == 0 then
        return "{}"
    end

    local pairs_list = {}
    for _, k in ipairs(keys) do
        local kj = M.to_json(k, 0, 0)             -- keys are always compact
        local vj = M.to_json(tbl[k], indent, depth + 1)
        if indent > 0 then
            pairs_list[#pairs_list + 1] = kj .. ": " .. vj
        else
            pairs_list[#pairs_list + 1] = kj .. ":" .. vj
        end
    end

    if indent > 0 then
        local inner_pad = string.rep(" ", indent * (depth + 1))
        local outer_pad = string.rep(" ", indent * depth)
        return "{\n" .. inner_pad
            .. table.concat(pairs_list, ",\n" .. inner_pad)
            .. "\n" .. outer_pad .. "}"
    else
        return "{" .. table.concat(pairs_list, ",") .. "}"
    end
end

return M
