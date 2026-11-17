-- tests/json_helper.lua
-- ============================================================================
-- Zero-dependency, pure-Lua JSON decoder for loading test fixtures.
-- Exact 64-bit integer parsing using wrapping signed 64-bit integers.
-- ============================================================================

local M = {}

local function parse_int64(str)
    local negative = str:sub(1, 1) == "-"
    if negative then str = str:sub(2) end
    local n = 0
    for i = 1, #str do
        n = n * 10 + (str:byte(i) - 48)
    end
    if negative then n = -n end
    return n
end

--- Decode a JSON string into Lua tables/values.
--- @param text string
--- @return any
function M.decode(text)
    local pos = 1
    local len = #text

    local function skip_whitespace()
        pos = text:find("[^ \t\r\n]", pos) or (len + 1)
    end

    local parse_value

    local function decode_string()
        local parts = {}
        local i = pos + 1
        while i <= len do
            local c = text:sub(i, i)
            if c == '"' then
                pos = i + 1
                return table.concat(parts)
            elseif c == '\\' then
                local esc = text:sub(i + 1, i + 1)
                if esc == '"' then parts[#parts + 1] = '"'; i = i + 2
                elseif esc == '\\' then parts[#parts + 1] = '\\'; i = i + 2
                elseif esc == '/' then parts[#parts + 1] = '/'; i = i + 2
                elseif esc == 'b' then parts[#parts + 1] = '\b'; i = i + 2
                elseif esc == 'f' then parts[#parts + 1] = '\f'; i = i + 2
                elseif esc == 'n' then parts[#parts + 1] = '\n'; i = i + 2
                elseif esc == 'r' then parts[#parts + 1] = '\r'; i = i + 2
                elseif esc == 't' then parts[#parts + 1] = '\t'; i = i + 2
                elseif esc == 'u' then
                    local hex = text:sub(i + 2, i + 5)
                    local cp = tonumber(hex, 16)
                    if not cp then error("malformed \\u escape in JSON at " .. i) end
                    parts[#parts + 1] = utf8.char(cp)
                    i = i + 6
                else
                    parts[#parts + 1] = esc
                    i = i + 2
                end
            else
                local next_special = text:find('[\\"]', i) or (len + 1)
                parts[#parts + 1] = text:sub(i, next_special - 1)
                i = next_special
            end
        end
        error("unterminated string in JSON")
    end

    local function decode_object()
        local obj = {}
        pos = pos + 1 -- skip '{'
        skip_whitespace()
        if text:sub(pos, pos) == "}" then
            pos = pos + 1
            return obj
        end
        while true do
            skip_whitespace()
            if text:sub(pos, pos) ~= '"' then
                error("expected string key in object at offset " .. pos)
            end
            local key = decode_string()
            skip_whitespace()
            if text:sub(pos, pos) ~= ":" then
                error("expected ':' after object key at offset " .. pos)
            end
            pos = pos + 1
            obj[key] = parse_value()
            skip_whitespace()
            local c = text:sub(pos, pos)
            pos = pos + 1
            if c == "}" then
                return obj
            elseif c ~= "," then
                error("expected ',' or '}' in object at offset " .. (pos - 1))
            end
        end
    end

    local function decode_array()
        local arr = {}
        pos = pos + 1 -- skip '['
        skip_whitespace()
        if text:sub(pos, pos) == "]" then
            pos = pos + 1
            return arr
        end
        while true do
            arr[#arr + 1] = parse_value()
            skip_whitespace()
            local c = text:sub(pos, pos)
            pos = pos + 1
            if c == "]" then
                return arr
            elseif c ~= "," then
                error("expected ',' or ']' in array at offset " .. (pos - 1))
            end
        end
    end

    parse_value = function()
        skip_whitespace()
        local c = text:sub(pos, pos)
        if c == "{" then
            return decode_object()
        elseif c == "[" then
            return decode_array()
        elseif c == '"' then
            return decode_string()
        elseif text:sub(pos, pos + 3) == "true" then
            pos = pos + 4
            return true
        elseif text:sub(pos, pos + 4) == "false" then
            pos = pos + 5
            return false
        elseif text:sub(pos, pos + 3) == "null" then
            pos = pos + 4
            return nil
        else
            -- Number
            local num_str = text:match("^-?%d+%.?%d*[eE]?[+-]?%d*", pos)
            if not num_str or num_str == "" then
                error("unexpected character in JSON at offset " .. pos .. ": " .. c)
            end
            pos = pos + #num_str
            if num_str:find(".", 1, true) or num_str:find("e", 1, true) or num_str:find("E", 1, true) then
                return tonumber(num_str)
            else
                return parse_int64(num_str)
            end
        end
    end

    local root = parse_value()
    skip_whitespace()
    if pos <= len then
        error("trailing garbage in JSON at offset " .. pos)
    end
    return root
end

return M
