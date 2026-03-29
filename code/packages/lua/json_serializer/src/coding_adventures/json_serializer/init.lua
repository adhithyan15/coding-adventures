-- json_serializer — Schema-aware JSON serializer/deserializer
-- ===========================================================
--
-- This package is part of the coding-adventures monorepo.  It sits one layer
-- above `json_value`: whereas `json_value` provides a direct AST-to-native
-- round-trip, `json_serializer` adds a richer API layer on top:
--
--   1. `encode(value, opts)` — robust encoding with options (indent, sort_keys,
--      allow_nan, max_depth)
--   2. `decode(json_str, opts)` — decoding with preprocessing options (strict
--      mode, allow_comments, trailing comma handling)
--   3. `validate(value, schema)` — validate a native Lua value against a
--      JSON-Schema-inspired schema subset
--   4. `schema_encode(value, schema)` — encode with schema-guided coercion
--      (drop extra fields, coerce types)
--
-- # What is JSON Schema?
--
-- JSON Schema is a vocabulary that describes the structure of JSON documents.
-- Think of it as the type system for JSON.  A schema like:
--
--   { type = "object",
--     properties = {
--       name = { type = "string" },
--       age  = { type = "number", minimum = 0 },
--     },
--     required = { "name" },
--   }
--
-- declares that a valid document must be an object with at least a "name"
-- field of type string, and optionally an "age" field that is a non-negative
-- number.  This package implements the most commonly needed subset of the
-- JSON Schema Draft 7 specification.
--
-- # Streaming / incremental generation
--
-- True streaming requires coroutines or callbacks and is complex to wire up
-- correctly.  Instead, `encode` handles the incremental-style concern by
-- enforcing a configurable `max_depth` limit: extremely deep structures are
-- caught early rather than silently producing corrupt output or crashing the
-- Lua stack.  Callers who need to produce very large documents should split
-- them into top-level chunks and call `encode` on each.
--
-- # Architecture
--
--   json_serializer  ← this package
--        ↓
--   json_value  (provides from_string, to_json, null, is_null)
--        ↓
--   json_parser, json_lexer, parser, lexer, grammar_tools, state_machine, directed_graph

local json_value = require("coding_adventures.json_value")

local M = {}
M.VERSION = "0.1.0"

-- Re-export the null sentinel and is_null from json_value so callers only
-- need to require this one package.
M.null    = json_value.null
M.is_null = json_value.is_null

-- =========================================================================
-- Internal utilities
-- =========================================================================

--- Count all entries in a table (works for both array and hash tables).
-- Unlike `#t`, this counts every key in the table.
-- @param t  table
-- @return   number
local function table_count(t)
    local n = 0
    for _ in pairs(t) do n = n + 1 end
    return n
end

--- Return true when `t` is a Lua sequence (keys 1..#t with no gaps).
-- An empty table is NOT considered a sequence here — callers treat it as
-- an object, mirroring the behaviour of json_value.to_json.
-- @param t  table
-- @return   boolean
local function is_sequence(t)
    local n = #t
    if n == 0 then return false end
    return table_count(t) == n
end

-- =========================================================================
-- Comment & trailing-comma stripping
-- =========================================================================
--
-- Standard JSON (RFC 8259) does not allow comments or trailing commas.
-- Many config file formats (JSONC, JSON5) extend JSON with these conveniences.
-- When `allow_comments = true` we pre-process the input to remove comments
-- before handing it to the strict JSON parser.
--
-- # Single-line comments
--
--   // everything until end-of-line is a comment
--
-- # Multi-line comments
--
--   /* everything between these markers, including newlines, is a comment */
--
-- # Trailing commas
--
--   { "a": 1, }    →  { "a": 1 }    (trailing comma in object)
--   [ 1, 2, ]      →  [ 1, 2 ]      (trailing comma in array)
--
-- Implementation note: we strip string literals before applying comment
-- patterns so that `//` or `/*` inside a string value are not removed.
-- We use a simple character-by-character scanner for correctness.

--- Strip // and /* */ comments from a JSON-like string.
-- String literals are preserved verbatim.
--
-- Algorithm:
--   - Walk character by character.
--   - When inside a string (between unescaped `"`), copy characters as-is.
--   - When encountering `//`, skip until end of line (replace with nothing).
--   - When encountering `/*`, skip until `*/` (replace with a single space
--     to preserve token boundaries).
--   - Otherwise, copy the character.
--
-- @param s   string  Input that may contain comments.
-- @return    string  Input with all comments removed.
local function strip_comments(s)
    local result = {}
    local i = 1
    local len = #s

    while i <= len do
        local c = s:sub(i, i)

        -- ----------------------------------------------------------------
        -- String literal: copy verbatim, handling escape sequences
        -- ----------------------------------------------------------------
        if c == '"' then
            result[#result + 1] = c
            i = i + 1
            while i <= len do
                local sc = s:sub(i, i)
                result[#result + 1] = sc
                if sc == '\\' then
                    -- Escaped character: copy it too, then continue
                    i = i + 1
                    if i <= len then
                        result[#result + 1] = s:sub(i, i)
                    end
                elseif sc == '"' then
                    -- End of string
                    break
                end
                i = i + 1
            end
            i = i + 1

        -- ----------------------------------------------------------------
        -- Single-line comment: // ... \n
        -- ----------------------------------------------------------------
        elseif c == '/' and s:sub(i + 1, i + 1) == '/' then
            -- Skip everything until (but not including) the newline
            i = i + 2
            while i <= len and s:sub(i, i) ~= '\n' do
                i = i + 1
            end
            -- Leave the newline in place (it acts as whitespace)

        -- ----------------------------------------------------------------
        -- Multi-line comment: /* ... */
        -- ----------------------------------------------------------------
        elseif c == '/' and s:sub(i + 1, i + 1) == '*' then
            -- Skip until */ found
            i = i + 2
            while i <= len do
                if s:sub(i, i) == '*' and s:sub(i + 1, i + 1) == '/' then
                    i = i + 2
                    break
                end
                i = i + 1
            end
            -- Insert a space to avoid merging adjacent tokens
            result[#result + 1] = ' '

        -- ----------------------------------------------------------------
        -- Ordinary character
        -- ----------------------------------------------------------------
        else
            result[#result + 1] = c
            i = i + 1
        end
    end

    return table.concat(result)
end

--- Remove trailing commas before `}` or `]` in a JSON string.
-- This turns `{ "a": 1, }` into `{ "a": 1 }`.
--
-- We use simple pattern-based replacement.  The pattern matches optional
-- whitespace, a comma, optional whitespace, then a closing bracket.
-- We preserve the closing bracket in the replacement.
--
-- This approach is naive (it does not parse the structure) but is safe
-- for well-formed input with only trailing-comma relaxations:
-- a comma followed solely by whitespace before `}` or `]`.
--
-- @param s   string  JSON string (possibly with trailing commas).
-- @return    string  JSON string with trailing commas removed.
local function strip_trailing_commas(s)
    -- Replace  ,<whitespace>}  →  }
    -- Replace  ,<whitespace>]  →  ]
    -- The loop handles multiple passes in case of nested structures,
    -- although a single pass suffices for most inputs.
    local prev
    repeat
        prev = s
        s = s:gsub(",%s*}", "}")
        s = s:gsub(",%s*%]", "]")
    until s == prev
    return s
end

-- =========================================================================
-- encode(value, opts)
-- =========================================================================
--
-- Serialize a native Lua value to a JSON string with rich options.
--
-- # Options
--
--   indent     (number, default 0)     — spaces per indentation level;
--                                        0 means compact (no whitespace)
--   sort_keys  (boolean, default true) — sort object keys alphabetically
--   allow_nan  (boolean, default false)— if true, emit NaN/Infinity as
--                                        literal strings rather than null
--   max_depth  (number, default 100)   — maximum nesting depth; raises an
--                                        error if exceeded (prevents stack
--                                        overflow on circular or excessively
--                                        deep structures)
--
-- # Why sort_keys?
--
-- JSON objects are *unordered* by the spec (RFC 8259 §4).  Lua tables have
-- no guaranteed iteration order either.  When serializing for storage,
-- comparison, or signing, non-deterministic key order is a problem: two
-- logically identical objects would produce different JSON strings.
-- `sort_keys = true` guarantees lexicographic key order, making output
-- reproducible.  Setting it to `false` gives slightly faster serialization
-- when order does not matter.
--
-- # Why max_depth?
--
-- Deeply recursive `encode` calls consume Lua stack frames.  A circular
-- reference or pathologically deep nested table will exhaust the stack and
-- crash the process.  By tracking depth and raising early, we give callers
-- a useful error message instead of a segfault.
--
-- @param value   any      The Lua value to encode.
-- @param opts    table    Options (see above).  May be nil.
-- @return string          JSON-encoded string.
-- @error                  On depth exceeded, NaN/Infinity when not allowed.

function M.encode(value, opts)
    opts = opts or {}
    local indent    = opts.indent    or 0
    local sort_keys = opts.sort_keys
    if sort_keys == nil then sort_keys = true end   -- default true
    local allow_nan = opts.allow_nan or false
    local max_depth = opts.max_depth or 100

    -- Delegate to the internal recursive encoder.
    return M._encode_value(value, indent, 0, sort_keys, allow_nan, max_depth)
end

--- Internal recursive encoder.
-- @param value      any
-- @param indent     number   Spaces per level
-- @param depth      number   Current nesting depth
-- @param sort_keys  boolean
-- @param allow_nan  boolean
-- @param max_depth  number
-- @return string
function M._encode_value(value, indent, depth, sort_keys, allow_nan, max_depth)
    -- ----------------------------------------------------------------
    -- Depth guard
    -- ----------------------------------------------------------------
    if depth > max_depth then
        error(string.format(
            "json_serializer.encode: max_depth %d exceeded", max_depth))
    end

    -- ----------------------------------------------------------------
    -- nil and the null sentinel
    -- ----------------------------------------------------------------
    if value == nil or json_value.is_null(value) then
        return "null"

    -- ----------------------------------------------------------------
    -- boolean
    -- ----------------------------------------------------------------
    elseif type(value) == "boolean" then
        return value and "true" or "false"

    -- ----------------------------------------------------------------
    -- number
    --
    -- JSON does not support NaN or Infinity.  By default we map them to
    -- `null` (matching the json_value behaviour).  When `allow_nan = true`
    -- we emit the strings "NaN", "Infinity", "-Infinity" — useful when the
    -- consumer is JavaScript, which accepts them.
    --
    -- Integer vs float heuristic: if the number equals its floor AND is
    -- within the 53-bit safe integer range, format without decimal point.
    -- This matches the behaviour consumers expect: JSON `42`, not `42.0`.
    -- ----------------------------------------------------------------
    elseif type(value) == "number" then
        -- NaN check: NaN is the only value not equal to itself
        if value ~= value then
            if allow_nan then return '"NaN"' end
            return "null"
        end
        if value == math.huge then
            if allow_nan then return '"Infinity"' end
            return "null"
        end
        if value == -math.huge then
            if allow_nan then return '"-Infinity"' end
            return "null"
        end
        -- Integer detection
        if math.floor(value) == value and math.abs(value) < 2^53 then
            return string.format("%d", value)
        else
            return string.format("%.14g", value)
        end

    -- ----------------------------------------------------------------
    -- string
    -- ----------------------------------------------------------------
    elseif type(value) == "string" then
        return M._encode_string(value)

    -- ----------------------------------------------------------------
    -- table: dispatch to array or object encoder
    -- ----------------------------------------------------------------
    elseif type(value) == "table" then
        if is_sequence(value) then
            return M._encode_array(
                value, indent, depth, sort_keys, allow_nan, max_depth)
        else
            return M._encode_object(
                value, indent, depth, sort_keys, allow_nan, max_depth)
        end

    -- ----------------------------------------------------------------
    -- Unsupported types (function, userdata, thread)
    -- ----------------------------------------------------------------
    else
        return "null"
    end
end

--- Encode a Lua string as a JSON string literal with proper escaping.
-- All characters that would be invalid inside a JSON string are escaped:
--   "  →  \"
--   \  →  \\
--   control chars (U+0000–U+001F) → \uXXXX (or short form where defined)
-- @param s  string
-- @return   string  The JSON-encoded string literal (including quotes).
function M._encode_string(s)
    local escaped = s
        :gsub("\\", "\\\\")    -- backslash must come first
        :gsub('"',  '\\"')
        :gsub("\b", "\\b")
        :gsub("\f", "\\f")
        :gsub("\n", "\\n")
        :gsub("\r", "\\r")
        :gsub("\t", "\\t")
    -- Remaining control characters U+0000–U+001F without a short escape form
    escaped = escaped:gsub("[\x00-\x1f]", function(c)
        return string.format("\\u%04x", c:byte(1))
    end)
    return '"' .. escaped .. '"'
end

--- Encode a Lua sequence table as a JSON array.
function M._encode_array(tbl, indent, depth, sort_keys, allow_nan, max_depth)
    local n = #tbl
    if n == 0 then return "[]" end

    local items = {}
    for i = 1, n do
        items[i] = M._encode_value(
            tbl[i], indent, depth + 1, sort_keys, allow_nan, max_depth)
    end

    if indent > 0 then
        local inner_pad = string.rep(" ", indent * (depth + 1))
        local outer_pad = string.rep(" ", indent * depth)
        return "[\n" .. inner_pad
            .. table.concat(items, ",\n" .. inner_pad)
            .. "\n" .. outer_pad .. "]"
    else
        return "[" .. table.concat(items, ",") .. "]"
    end
end

--- Encode a Lua table (non-sequence) as a JSON object.
-- Keys are sorted when `sort_keys` is true (default).
function M._encode_object(tbl, indent, depth, sort_keys, allow_nan, max_depth)
    local keys = {}
    for k in pairs(tbl) do
        if type(k) == "string" then
            keys[#keys + 1] = k
        end
    end

    -- Sort or leave in arbitrary iteration order
    if sort_keys then
        table.sort(keys)
    end

    if #keys == 0 then return "{}" end

    local pairs_list = {}
    for _, k in ipairs(keys) do
        local kj = M._encode_string(k)
        local vj = M._encode_value(
            tbl[k], indent, depth + 1, sort_keys, allow_nan, max_depth)
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

-- =========================================================================
-- decode(json_str, opts)
-- =========================================================================
--
-- Decode a JSON (or JSON-like) string to a native Lua value.
--
-- This wraps `json_value.from_string` with optional pre-processing:
--
--   allow_comments  (boolean, default false) — strip // and /* */ comments
--                   before parsing (JSONC-style input)
--   strict          (boolean, default false) — when false (the default),
--                   strip trailing commas before parsing; when true, any
--                   trailing comma is a parse error
--
-- # Why non-strict mode?
--
-- Many hand-written JSON config files include trailing commas because they
-- are natural to add when copying, editing, or re-ordering list entries.
-- Non-strict mode silently normalises them, accepting input that a strict
-- RFC 8259 parser would reject.  This is a deliberate usability trade-off;
-- use `strict = true` when interoperability with other parsers matters.
--
-- @param json_str  string  The JSON string to decode.
-- @param opts      table   Options (see above).  May be nil.
-- @return any              Native Lua value.
-- @error                   On parse error.

function M.decode(json_str, opts)
    opts = opts or {}
    local allow_comments = opts.allow_comments or false
    local strict         = opts.strict         or false

    local s = json_str

    -- Pre-process: strip comments first (before trailing commas, because a
    -- line comment `// ,` would otherwise leave a trailing comma behind)
    if allow_comments then
        s = strip_comments(s)
    end

    -- Pre-process: strip trailing commas (unless strict mode)
    if not strict then
        s = strip_trailing_commas(s)
    end

    return json_value.from_string(s)
end

-- =========================================================================
-- validate(value, schema)
-- =========================================================================
--
-- Validate a native Lua value against a JSON-Schema-inspired schema subset.
--
-- Returns `true, nil` on success, or `false, errors_table` on failure.
-- `errors_table` is an array of human-readable error strings, each describing
-- one validation failure.  Multiple errors may be reported for a single call
-- (we collect them all rather than stopping at the first failure).
--
-- # Supported schema keywords
--
--   type         — "string" | "number" | "integer" | "boolean" | "null" |
--                  "object" | "array"
--   properties   — object: sub-schema for each named property
--   required     — array: list of property names that must be present
--   additional_properties — boolean: if false, forbid extra keys
--   items        — schema for each element of an array
--   minItems     — minimum array length
--   maxItems     — maximum array length
--   minimum      — minimum numeric value (inclusive)
--   maximum      — maximum numeric value (inclusive)
--   minLength    — minimum string length (in bytes)
--   maxLength    — maximum string length (in bytes)
--   pattern      — Lua pattern that the string must match
--   enum         — array: the value must be one of these literals
--
-- # JSON Schema primer
--
-- JSON Schema (https://json-schema.org) is a vocabulary for annotating and
-- validating JSON documents.  Think of it as a type system: a schema is a
-- JSON value that *describes* what other JSON values should look like.
--
-- Example schema — a "user" record:
--
--   {
--     type = "object",
--     properties = {
--       username = { type = "string", minLength = 1, maxLength = 20 },
--       age      = { type = "integer", minimum = 0, maximum = 150 },
--       email    = { type = "string", pattern = "@" },
--       roles    = { type = "array", items = { type = "string" } },
--     },
--     required = { "username", "age" },
--     additional_properties = false,
--   }
--
-- @param value   any    Native Lua value to validate.
-- @param schema  table  Schema description (Lua table, as above).
-- @return boolean, table  (true, nil) on success; (false, errors) on failure.

function M.validate(value, schema)
    local errors = {}
    M._validate_value(value, schema, "$", errors)
    if #errors == 0 then
        return true, nil
    else
        return false, errors
    end
end

--- Internal recursive validator.
-- Appends error messages to `errors`.  The `path` argument is a dotted
-- path string like `"$.address.city"` — it helps pinpoint which field
-- in a nested document failed validation.
--
-- @param value   any
-- @param schema  table
-- @param path    string   Dotted path for error messages
-- @param errors  table    Mutable array of error strings
function M._validate_value(value, schema, path, errors)
    -- ----------------------------------------------------------------
    -- type check
    --
    -- JSON Schema "type" maps to a Lua type as follows:
    --   "string"  → type(v) == "string"
    --   "number"  → type(v) == "number"
    --   "integer" → type(v) == "number" AND math.floor(v) == v
    --   "boolean" → type(v) == "boolean"
    --   "null"    → json_value.is_null(v) OR v == nil
    --   "object"  → type(v) == "table" AND not is_sequence(v)
    --   "array"   → type(v) == "table" AND is_sequence or #v == 0
    -- ----------------------------------------------------------------
    if schema.type then
        local ok = M._check_type(value, schema.type)
        if not ok then
            errors[#errors + 1] = string.format(
                "%s: expected type %q, got %s",
                path, schema.type, M._describe_type(value))
            -- If the type is wrong, many other checks become meaningless.
            -- We still run them in case they produce useful additional info.
        end
    end

    -- ----------------------------------------------------------------
    -- enum
    --
    -- The value must be strictly equal (==) to one of the listed values.
    -- For JSON null, we use is_null().
    -- ----------------------------------------------------------------
    if schema.enum then
        local found = false
        for _, candidate in ipairs(schema.enum) do
            if json_value.is_null(value) and json_value.is_null(candidate) then
                found = true; break
            elseif value == candidate then
                found = true; break
            end
        end
        if not found then
            errors[#errors + 1] = string.format(
                "%s: value not in enum", path)
        end
    end

    -- ----------------------------------------------------------------
    -- string constraints
    -- ----------------------------------------------------------------
    if type(value) == "string" then
        if schema.minLength and #value < schema.minLength then
            errors[#errors + 1] = string.format(
                "%s: string length %d < minLength %d",
                path, #value, schema.minLength)
        end
        if schema.maxLength and #value > schema.maxLength then
            errors[#errors + 1] = string.format(
                "%s: string length %d > maxLength %d",
                path, #value, schema.maxLength)
        end
        if schema.pattern then
            -- Guard against ReDoS: reject patterns longer than 200 characters.
            -- A well-formed JSON Schema pattern should be concise; a 200-char
            -- limit stops adversarial catastrophic-backtracking patterns like
            -- (a+)+$ without impacting legitimate schema validation use cases.
            if #schema.pattern > 200 then
                errors[#errors + 1] = string.format(
                    "%s: schema pattern too long (max 200 chars)",
                    path)
            elseif not value:match(schema.pattern) then
                errors[#errors + 1] = string.format(
                    "%s: string does not match pattern %q",
                    path, schema.pattern)
            end
        end
    end

    -- ----------------------------------------------------------------
    -- number constraints
    -- ----------------------------------------------------------------
    if type(value) == "number" then
        if schema.minimum and value < schema.minimum then
            errors[#errors + 1] = string.format(
                "%s: value %g < minimum %g",
                path, value, schema.minimum)
        end
        if schema.maximum and value > schema.maximum then
            errors[#errors + 1] = string.format(
                "%s: value %g > maximum %g",
                path, value, schema.maximum)
        end
    end

    -- ----------------------------------------------------------------
    -- array constraints
    --
    -- Apply array constraints when the table is a sequence, OR when it
    -- is empty AND the schema explicitly requests array type.
    -- We do NOT apply array constraints to empty tables with schema.type
    -- == "object" or with schema.required (which implies an object).
    -- ----------------------------------------------------------------
    local treat_as_array = is_sequence(value)
        or (table_count(value) == 0
            and schema.type == "array"
            and not schema.required
            and not schema.properties)
    if type(value) == "table" and treat_as_array then
        local n = #value
        if schema.minItems and n < schema.minItems then
            errors[#errors + 1] = string.format(
                "%s: array length %d < minItems %d",
                path, n, schema.minItems)
        end
        if schema.maxItems and n > schema.maxItems then
            errors[#errors + 1] = string.format(
                "%s: array length %d > maxItems %d",
                path, n, schema.maxItems)
        end
        if schema.items then
            for i = 1, n do
                M._validate_value(
                    value[i], schema.items,
                    path .. "[" .. i .. "]", errors)
            end
        end
    end

    -- ----------------------------------------------------------------
    -- object constraints
    --
    -- Objects are non-sequence tables.  We check:
    --   required    — all listed keys must be present
    --   properties  — each present key is validated against its sub-schema
    --   additional_properties = false — no key outside `properties` allowed
    -- ----------------------------------------------------------------
    if type(value) == "table" and not is_sequence(value) and table_count(value) > 0 then
        -- required properties
        if schema.required then
            for _, req_key in ipairs(schema.required) do
                if value[req_key] == nil then
                    errors[#errors + 1] = string.format(
                        "%s: missing required property %q",
                        path, req_key)
                end
            end
        end

        -- property sub-schemas
        if schema.properties then
            for k, sub_schema in pairs(schema.properties) do
                if value[k] ~= nil then
                    M._validate_value(
                        value[k], sub_schema,
                        path .. "." .. k, errors)
                end
            end
        end

        -- additional properties
        if schema.additional_properties == false and schema.properties then
            for k in pairs(value) do
                if schema.properties[k] == nil then
                    errors[#errors + 1] = string.format(
                        "%s: additional property %q not allowed",
                        path, k)
                end
            end
        end
    end

    -- ----------------------------------------------------------------
    -- Empty-table special case for object schemas
    --
    -- `{}` is an empty table.  When schema.type == "object" and the schema
    -- has `required` keys, those checks must run even though is_sequence
    -- returns false for empty tables.  We handle this by re-running
    -- required checks for empty tables when `schema.type == "object"`.
    -- ----------------------------------------------------------------
    if type(value) == "table" and table_count(value) == 0 then
        if schema.required then
            for _, req_key in ipairs(schema.required) do
                errors[#errors + 1] = string.format(
                    "%s: missing required property %q",
                    path, req_key)
            end
        end
    end
end

--- Check whether `value` satisfies a JSON Schema type string.
-- @param value  any
-- @param t      string  JSON Schema type name
-- @return       boolean
function M._check_type(value, t)
    if t == "string"  then return type(value) == "string"
    elseif t == "number"  then return type(value) == "number"
    elseif t == "integer" then
        return type(value) == "number" and math.floor(value) == value
    elseif t == "boolean" then return type(value) == "boolean"
    elseif t == "null"    then
        return json_value.is_null(value) or value == nil
    elseif t == "array"   then
        -- An array is a Lua table that is either a sequence or empty
        if type(value) ~= "table" then return false end
        local n = #value
        if n == 0 then return true end  -- empty table is a valid empty array
        return table_count(value) == n
    elseif t == "object"  then
        if type(value) ~= "table" then return false end
        -- An empty table is ambiguous; we accept it as an object here
        -- because JSON {} is an empty object.
        local n = #value
        if n > 0 then return false end  -- it looks like an array
        return true
    else
        return true  -- unknown type keyword: don't fail
    end
end

--- Return a human-readable description of the Lua type of `value`.
-- Used in error messages.
-- @param value  any
-- @return       string
function M._describe_type(value)
    if json_value.is_null(value) then
        return "null"
    elseif type(value) == "table" then
        if is_sequence(value) then
            return "array"
        else
            return "object"
        end
    else
        return type(value)
    end
end

-- =========================================================================
-- schema_encode(value, schema)
-- =========================================================================
--
-- Encode a Lua value to JSON, guided by the provided schema.
--
-- Before encoding, this function applies two kinds of schema-driven
-- transformations to the *value*:
--
--   1. Type coercion:  If the schema says `type = "string"` for a field
--      but the actual value is a number, we coerce it via `tostring`.
--      This is useful when a value was stored as a number for computation
--      but the API contract requires a string.
--
--   2. Property filtering:  If `additional_properties = false` in the
--      schema, keys not listed in `properties` are dropped silently.
--      This prevents accidentally leaking internal state fields.
--
-- After coercion and filtering, the value is encoded with `M.encode`.
--
-- # Why coercion?
--
-- Real-world code often works with polymorphic data: a "price" might be
-- stored as a float for arithmetic but serialized as a string ("9.99")
-- for a payment API that requires string decimal values.  Schema-driven
-- coercion separates "what type do I compute with?" from "what type do
-- I send over the wire?".
--
-- @param value   any    Native Lua value to encode.
-- @param schema  table  Schema to guide coercion.
-- @param opts    table  Encoding options (passed to M.encode).
-- @return string        JSON-encoded string.

function M.schema_encode(value, schema, opts)
    local coerced = M._coerce_value(value, schema)
    return M.encode(coerced, opts)
end

--- Recursively apply schema-driven coercions and filtering.
-- @param value   any
-- @param schema  table
-- @return any    Possibly transformed value
function M._coerce_value(value, schema)
    if not schema then return value end

    -- ----------------------------------------------------------------
    -- Type coercion for primitive values
    --
    -- When schema.type == "string" and the value is a number, coerce
    -- the number to a string.  Other primitive-to-primitive coercions
    -- could be added here as needed.
    -- ----------------------------------------------------------------
    if schema.type == "string" and type(value) == "number" then
        -- Use %.14g to match our number serialization precision,
        -- then strip trailing .0 for integers to give "42" not "42.0".
        if math.floor(value) == value and math.abs(value) < 2^53 then
            return string.format("%d", value)
        else
            return string.format("%.14g", value)
        end
    end

    -- ----------------------------------------------------------------
    -- Object: recurse into properties, apply filtering
    -- ----------------------------------------------------------------
    if type(value) == "table" and not is_sequence(value) and schema.type == "object" then
        local result = {}

        for k, v in pairs(value) do
            -- If additional_properties = false, drop unknown keys
            if schema.additional_properties == false and schema.properties then
                if not schema.properties[k] then
                    goto continue
                end
            end
            -- Recurse with the property's sub-schema (if any)
            local sub_schema = schema.properties and schema.properties[k]
            result[k] = M._coerce_value(v, sub_schema)
            ::continue::
        end

        return result
    end

    -- ----------------------------------------------------------------
    -- Array: recurse into items
    -- ----------------------------------------------------------------
    if type(value) == "table" and is_sequence(value) and schema.type == "array" then
        local result = {}
        for i, item in ipairs(value) do
            result[i] = M._coerce_value(item, schema.items)
        end
        return result
    end

    -- No transformation needed
    return value
end

return M
