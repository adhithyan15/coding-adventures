--- RESP2 typed values, binary-safe encoding, and incremental decoding.

local M = { VERSION = "0.1.0" }

local MAX_BULK_LENGTH = 512 * 1024 * 1024
local MAX_ARRAY_LENGTH = 1000000
local MAX_NESTING_DEPTH = 128

local valid_kinds = {
    simple_string = true,
    error = true,
    integer = true,
    bulk_string = true,
    array = true,
}

local Value = {}
Value.__index = Value

local function is_value(value)
    return type(value) == "table" and getmetatable(value) == Value
end

local function assert_string(value, name, level)
    if type(value) ~= "string" then
        error(name .. " must be a string", level or 3)
    end
end

local function assert_integer(value, name, level)
    if type(value) ~= "number" or math.type(value) ~= "integer" then
        error(name .. " must be an integer", level or 3)
    end
end

local function copy_values(values)
    local result = {}
    for index, value in ipairs(values) do
        if not is_value(value) then
            error(string.format("array value[%d] must be a RESP Value", index), 4)
        end
        result[index] = value
    end
    return result
end

function Value.new(kind, value)
    if not valid_kinds[kind] then
        error("invalid RESP kind: " .. tostring(kind), 2)
    end

    local result = { kind = kind, value = value, is_null = false }
    if kind == "simple_string" or kind == "error" then
        assert_string(value, kind .. " value", 2)
        if value:find("\r", 1, true) or value:find("\n", 1, true) then
            error(kind .. " value must not contain CR or LF", 2)
        end
        if kind == "error" then
            local error_type, detail = value:match("^(%S+)%s?(.*)$")
            result.error_type = error_type or ""
            result.detail = detail or ""
        end
    elseif kind == "integer" then
        assert_integer(value, "integer value", 2)
    elseif kind == "bulk_string" then
        if value == nil then
            result.is_null = true
        else
            assert_string(value, "bulk_string value", 2)
        end
    elseif kind == "array" then
        if value == nil then
            result.is_null = true
        elseif type(value) ~= "table" then
            error("array value must be a table or nil", 2)
        else
            result.value = copy_values(value)
        end
    end
    return setmetatable(result, Value)
end

function Value.simple_string(value)
    return Value.new("simple_string", value)
end

function Value.error(value)
    return Value.new("error", value)
end

function Value.integer(value)
    return Value.new("integer", value)
end

function Value.bulk_string(value)
    return Value.new("bulk_string", value)
end

function Value.null_bulk_string()
    return Value.new("bulk_string", nil)
end

function Value.array(value)
    return Value.new("array", value)
end

function Value.null_array()
    return Value.new("array", nil)
end

Value.__tostring = function(self)
    if self.is_null then
        return self.kind .. "(null)"
    end
    if self.kind == "array" then
        return string.format("array(%d values)", #self.value)
    end
    return string.format("%s(%s)", self.kind, tostring(self.value))
end

local function equal(left, right)
    if not is_value(left) or not is_value(right) then
        return false
    end
    if left.kind ~= right.kind or left.is_null ~= right.is_null then
        return false
    end
    if left.is_null then
        return true
    end
    if left.kind ~= "array" then
        return left.value == right.value
    end
    if #left.value ~= #right.value then
        return false
    end
    for index = 1, #left.value do
        if not equal(left.value[index], right.value[index]) then
            return false
        end
    end
    return true
end

Value.__eq = equal

local function encode_simple_string(value)
    return "+" .. Value.simple_string(value).value .. "\r\n"
end

local function encode_error(value)
    return "-" .. Value.error(value).value .. "\r\n"
end

local function encode_integer(value)
    assert_integer(value, "value", 2)
    return ":" .. tostring(value) .. "\r\n"
end

local function encode_bulk_string(value)
    if value == nil then
        return "$-1\r\n"
    end
    assert_string(value, "value", 2)
    if #value > MAX_BULK_LENGTH then
        error("bulk string exceeds maximum length", 2)
    end
    return "$" .. tostring(#value) .. "\r\n" .. value .. "\r\n"
end

local encode

local function encode_array(values)
    if values == nil then
        return "*-1\r\n"
    end
    if type(values) ~= "table" then
        error("values must be a table or nil", 2)
    end
    if #values > MAX_ARRAY_LENGTH then
        error("array exceeds maximum length", 2)
    end
    local parts = { "*", tostring(#values), "\r\n" }
    for _, value in ipairs(values) do
        parts[#parts + 1] = encode(value)
    end
    return table.concat(parts)
end

local function encode_value(value)
    if value.kind == "simple_string" then
        return encode_simple_string(value.value)
    elseif value.kind == "error" then
        return encode_error(value.value)
    elseif value.kind == "integer" then
        return encode_integer(value.value)
    elseif value.kind == "bulk_string" then
        return encode_bulk_string(value.is_null and nil or value.value)
    elseif value.kind == "array" then
        return encode_array(value.is_null and nil or value.value)
    end
    error("invalid RESP kind: " .. tostring(value.kind), 2)
end

encode = function(value)
    if is_value(value) then
        return encode_value(value)
    elseif value == nil then
        return encode_bulk_string(nil)
    elseif type(value) == "boolean" then
        return encode_integer(value and 1 or 0)
    elseif type(value) == "number" then
        return encode_integer(value)
    elseif type(value) == "string" then
        return encode_bulk_string(value)
    elseif type(value) == "table" then
        return encode_array(value)
    end
    error("cannot encode value of type " .. type(value), 2)
end

local function encode_many(values)
    if type(values) ~= "table" then
        error("values must be a table", 2)
    end
    local result = {}
    for index, value in ipairs(values) do
        result[index] = encode(value)
    end
    return table.concat(result)
end

local function read_line(data, offset)
    local line_end = data:find("\r\n", offset, true)
    if line_end == nil then
        return nil, offset
    end
    return data:sub(offset, line_end - 1), line_end + 2
end

local function parse_integer(text, context)
    if not text:match("^-?%d+$") then
        error("invalid " .. context .. ": " .. text, 3)
    end
    local value = tonumber(text)
    if value == nil or math.type(value) ~= "integer" then
        error(context .. " is outside the signed 64-bit range", 3)
    end
    return value
end

local decode_value

decode_value = function(data, offset, depth)
    if depth > MAX_NESTING_DEPTH then
        error("RESP array nesting exceeds maximum depth", 3)
    end
    if offset > #data then
        return nil, offset
    end

    local prefix = data:sub(offset, offset)
    if prefix == "+" or prefix == "-" or prefix == ":" then
        local line, next_offset = read_line(data, offset + 1)
        if line == nil then
            return nil, offset
        end
        if prefix == "+" then
            return Value.simple_string(line), next_offset
        elseif prefix == "-" then
            return Value.error(line), next_offset
        end
        return Value.integer(parse_integer(line, "RESP integer")), next_offset
    elseif prefix == "$" then
        local length_text, payload_offset = read_line(data, offset + 1)
        if length_text == nil then
            return nil, offset
        end
        local length = parse_integer(length_text, "bulk length")
        if length == -1 then
            return Value.null_bulk_string(), payload_offset
        elseif length < -1 then
            error("invalid negative bulk length: " .. tostring(length), 3)
        elseif length > MAX_BULK_LENGTH then
            error("bulk length exceeds maximum", 3)
        end

        local terminator_offset = payload_offset + length
        if terminator_offset + 1 > #data then
            return nil, offset
        end
        if data:sub(terminator_offset, terminator_offset + 1) ~= "\r\n" then
            error("bulk string missing trailing CRLF", 3)
        end
        local payload = data:sub(payload_offset, terminator_offset - 1)
        return Value.bulk_string(payload), terminator_offset + 2
    elseif prefix == "*" then
        local count_text, cursor = read_line(data, offset + 1)
        if count_text == nil then
            return nil, offset
        end
        local count = parse_integer(count_text, "array length")
        if count == -1 then
            return Value.null_array(), cursor
        elseif count < -1 then
            error("invalid negative array length: " .. tostring(count), 3)
        elseif count > MAX_ARRAY_LENGTH then
            error("array length exceeds maximum", 3)
        end

        local values = {}
        for index = 1, count do
            local value, next_offset = decode_value(data, cursor, depth + 1)
            if value == nil then
                return nil, offset
            end
            values[index] = value
            cursor = next_offset
        end
        return Value.array(values), cursor
    end

    local line, next_offset = read_line(data, offset)
    if line == nil then
        return nil, offset
    end
    local values = {}
    for token in line:gmatch("%S+") do
        values[#values + 1] = Value.bulk_string(token)
    end
    return Value.array(values), next_offset
end

local function decode(data, offset)
    assert_string(data, "data", 2)
    offset = offset == nil and 1 or offset
    assert_integer(offset, "offset", 2)
    if offset < 1 or offset > #data + 1 then
        error("offset is outside data", 2)
    end
    return decode_value(data, offset, 0)
end

local function decode_all(data, offset)
    assert_string(data, "data", 2)
    offset = offset == nil and 1 or offset
    assert_integer(offset, "offset", 2)
    if offset < 1 or offset > #data + 1 then
        error("offset is outside data", 2)
    end

    local values = {}
    local cursor = offset
    while cursor <= #data do
        local value, next_offset = decode_value(data, cursor, 0)
        if value == nil then
            break
        end
        values[#values + 1] = value
        cursor = next_offset
    end
    return values, cursor
end

local Decoder = {}
Decoder.__index = Decoder

function Decoder.new()
    return setmetatable({ _buffer = "", _queue = {} }, Decoder)
end

function Decoder:feed(data)
    assert_string(data, "data", 2)
    self._buffer = self._buffer .. data
    local values, next_offset = decode_all(self._buffer)
    for _, value in ipairs(values) do
        self._queue[#self._queue + 1] = value
    end
    if next_offset > 1 then
        self._buffer = self._buffer:sub(next_offset)
    end
    return self
end

function Decoder:has_message()
    return #self._queue > 0
end

function Decoder:get_message()
    if #self._queue == 0 then
        error("no decoded message is available", 2)
    end
    return table.remove(self._queue, 1)
end

function Decoder:drain()
    local result = self._queue
    self._queue = {}
    return result
end

function Decoder:decode_all(data)
    self:feed(data)
    return self:drain()
end

function Decoder:pending_bytes()
    return #self._buffer
end

M.Value = Value
M.Decoder = Decoder
M.is_value = is_value
M.equal = equal
M.encode = encode
M.encode_many = encode_many
M.encode_simple_string = encode_simple_string
M.encode_error = encode_error
M.encode_integer = encode_integer
M.encode_bulk_string = encode_bulk_string
M.encode_array = encode_array
M.decode = decode
M.decode_all = decode_all
M.MAX_BULK_LENGTH = MAX_BULK_LENGTH
M.MAX_ARRAY_LENGTH = MAX_ARRAY_LENGTH
M.MAX_NESTING_DEPTH = MAX_NESTING_DEPTH

return M
