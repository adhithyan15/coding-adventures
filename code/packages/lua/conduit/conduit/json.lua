--- conduit/json.lua — Minimal JSON encoder/decoder for Conduit.
---
--- Supports: strings, numbers, booleans, nil/null, arrays, objects.
--- No third-party dependencies — the repo avoids external LuaRocks packages
--- beyond test tooling.
---
--- ## Encoder
---
--- Tables are treated as JSON arrays when all keys are consecutive integers
--- starting at 1 (#t == number of keys). Otherwise they become JSON objects.
--- Object keys must be strings. Keys are sorted for deterministic output.
---
---   json.encode({a=1, b=2})        → '{"a":1,"b":2}'
---   json.encode({10, 20, 30})      → '[10,20,30]'
---   json.encode("hello\nworld")    → '"hello\\nworld"'
---
--- ## Decoder
---
--- Handles: null → nil, true/false, numbers, strings with escape sequences,
--- arrays, objects. Raises an error (string) on malformed input.
---
---   json.decode('{"x":1}')   → {x = 1}
---   json.decode('[1,2]')     → {1, 2}
---   json.decode('"hi"')      → "hi"

local M = {}

-- ==========================================================================
-- Encoder
-- ==========================================================================

--- Return true if table t should be encoded as a JSON array.
--- An array has consecutive integer keys 1..#t with no extra keys.
local function is_array(t)
    local count = 0
    for _ in pairs(t) do count = count + 1 end
    return count == #t
end

--- Encode a single Lua value to a JSON string fragment (recursive).
local function encode_val(v, depth)
    depth = depth or 0
    if depth > 50 then error("json.encode: table nesting too deep (circular reference?)") end

    local tp = type(v)

    if tp == "nil" then
        return "null"

    elseif tp == "boolean" then
        return v and "true" or "false"

    elseif tp == "number" then
        -- NaN and infinities are not valid JSON; emit null (same as many libs).
        if v ~= v or v == math.huge or v == -math.huge then return "null" end
        if math.type(v) == "integer" then return tostring(v) end
        return string.format("%.17g", v)

    elseif tp == "string" then
        -- Escape control characters and the mandatory JSON specials.
        return '"' .. v:gsub('[\\"/%z\1-\31]', function(c)
            local special = {
                ['"']  = '\\"',
                ['\\'] = '\\\\',
                ['/']  = '\\/',
                ['\n'] = '\\n',
                ['\r'] = '\\r',
                ['\t'] = '\\t',
                ['\b'] = '\\b',
                ['\f'] = '\\f',
            }
            return special[c] or string.format('\\u%04x', c:byte())
        end) .. '"'

    elseif tp == "table" then
        if is_array(v) then
            -- Array path: iterate by ipairs to preserve order.
            local parts = {}
            for _, item in ipairs(v) do
                parts[#parts + 1] = encode_val(item, depth + 1)
            end
            return '[' .. table.concat(parts, ',') .. ']'
        else
            -- Object path: keys must be strings.
            local parts = {}
            for k, item in pairs(v) do
                if type(k) ~= 'string' then
                    error('json.encode: non-string object key: ' .. tostring(k))
                end
                parts[#parts + 1] = encode_val(k, 0) .. ':' .. encode_val(item, depth + 1)
            end
            table.sort(parts)  -- deterministic order for reproducible test output
            return '{' .. table.concat(parts, ',') .. '}'
        end

    else
        error('json.encode: cannot encode type ' .. tp)
    end
end

--- Encode a Lua value as a JSON string.
---@param v any  Value to encode (nil, boolean, number, string, or table)
---@return string
function M.encode(v)
    return encode_val(v, 0)
end

-- ==========================================================================
-- Decoder
-- ==========================================================================

-- Forward declaration so decode_val can call decode_string/array/object.
local decode_val

--- Skip ASCII whitespace and return the position of the next non-space char.
local function skip(s, i)
    return (s:match('^%s*()', i))
end

--- Decode a JSON string starting at position i (which points at the '"').
--- Returns (string_value, next_position).
local function decode_string(s, i)
    local result = {}
    local j = i + 1  -- skip opening "
    while j <= #s do
        local c = s:sub(j, j)
        if c == '"' then
            return table.concat(result), j + 1
        elseif c == '\\' then
            local esc = s:sub(j + 1, j + 1)
            local simple = { ['"'] = '"', ['\\'] = '\\', ['/'] = '/',
                             ['n'] = '\n', ['r'] = '\r', ['t'] = '\t',
                             ['b'] = '\b', ['f'] = '\f' }
            if simple[esc] then
                result[#result + 1] = simple[esc]
                j = j + 2
            elseif esc == 'u' then
                local hex = s:sub(j + 2, j + 5)
                if #hex ~= 4 then error("json.decode: invalid \\u escape") end
                local cp = tonumber(hex, 16)
                if not cp then error("json.decode: invalid \\u hex: " .. hex) end
                -- Encode as UTF-8 using utf8.char (Lua 5.3+).
                result[#result + 1] = utf8.char(cp)
                j = j + 6
            else
                error("json.decode: unknown escape \\" .. esc)
            end
        else
            result[#result + 1] = c
            j = j + 1
        end
    end
    error("json.decode: unterminated string")
end

--- Decode a JSON number starting at position i.
local function decode_number(s, i)
    local tok, j = s:match('^(-?[0-9]+%.?[0-9]*[eE]?[+-]?[0-9]*)()', i)
    if not tok then error("json.decode: invalid number at position " .. i) end
    local n = tonumber(tok)
    if not n then error("json.decode: malformed number: " .. tok) end
    return n, j
end

--- Decode a JSON array starting at '['.
local function decode_array(s, i)
    local arr = {}
    local j = skip(s, i + 1)
    if s:sub(j, j) == ']' then return arr, j + 1 end
    while true do
        local val, nj = decode_val(s, j)
        arr[#arr + 1] = val
        j = skip(s, nj)
        local c = s:sub(j, j)
        if c == ']' then return arr, j + 1
        elseif c == ',' then j = skip(s, j + 1)
        else error("json.decode: expected ',' or ']' in array") end
    end
end

--- Decode a JSON object starting at '{'.
local function decode_object(s, i)
    local obj = {}
    local j = skip(s, i + 1)
    if s:sub(j, j) == '}' then return obj, j + 1 end
    while true do
        j = skip(s, j)
        if s:sub(j, j) ~= '"' then error("json.decode: expected string key in object") end
        local key, kj = decode_string(s, j)
        j = skip(s, kj)
        if s:sub(j, j) ~= ':' then error("json.decode: expected ':' after key") end
        j = skip(s, j + 1)
        local val, vj = decode_val(s, j)
        obj[key] = val
        j = skip(s, vj)
        local c = s:sub(j, j)
        if c == '}' then return obj, j + 1
        elseif c == ',' then j = skip(s, j + 1)
        else error("json.decode: expected ',' or '}' in object") end
    end
end

--- Decode a single JSON value at position i.
decode_val = function(s, i)
    i = skip(s, i)
    local c = s:sub(i, i)
    if c == '"' then
        return decode_string(s, i)
    elseif c == '{' then
        return decode_object(s, i)
    elseif c == '[' then
        return decode_array(s, i)
    elseif s:sub(i, i + 3) == 'true' then
        return true, i + 4
    elseif s:sub(i, i + 4) == 'false' then
        return false, i + 5
    elseif s:sub(i, i + 3) == 'null' then
        return nil, i + 4
    else
        return decode_number(s, i)
    end
end

--- Decode a JSON string into a Lua value.
--- Raises an error string on invalid input (pcall-able).
---@param s string  JSON text
---@return any  Decoded value (nil for JSON null)
function M.decode(s)
    if type(s) ~= 'string' then error("json.decode: expected string, got " .. type(s)) end
    local val, j = decode_val(s, skip(s, 1))
    j = skip(s, j or #s + 1)
    if j and j <= #s then
        error("json.decode: trailing garbage at position " .. j .. ": " .. s:sub(j))
    end
    return val
end

return M
