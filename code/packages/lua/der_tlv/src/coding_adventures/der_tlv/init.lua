-- Bounded, payload-blind DER tag-length-value framing.

local M = { VERSION = "0.1.0" }

local HOST_MAX = math.maxinteger
local U32_MAX = 0xffffffff
local TAG_CLASSES = { "universal", "application", "context-specific", "private" }
local DEFAULT_LIMITS = {
    max_input_len = 1048576,
    max_value_len = 1048576,
    max_elements = 4096,
    max_tag_number = U32_MAX,
}

M.HOST_MAX = HOST_MAX

local Error = {}
Error.__index = Error
Error.__tostring = function(self)
    return string.format("DER framing error %s at byte %d", self.kind, self.offset)
end

local function fail(kind, offset)
    error(setmetatable({ kind = kind, offset = offset }, Error), 0)
end

local function copy_limits(provided)
    local result = {}
    for name, value in pairs(DEFAULT_LIMITS) do result[name] = value end
    if provided ~= nil then
        for name, value in pairs(provided) do result[name] = value end
    end
    for name in pairs(DEFAULT_LIMITS) do
        local value = result[name]
        if math.type(value) ~= "integer" or value < 0 then
            error(name .. " must be a non-negative integer", 0)
        end
    end
    if result.max_tag_number > U32_MAX then
        error("max_tag_number must fit u32", 0)
    end
    return result
end

function M.default_limits()
    return copy_limits(nil)
end

local Element = {}
Element.__index = Element

function Element:header()
    return self._input:sub(self._start, self._start + self._header_len - 1)
end

function Element:value()
    local first = self._start + self._header_len
    return self._input:sub(first, self._start + self._encoded_len - 1)
end

function Element:encoded()
    return self._input:sub(self._start, self._start + self._encoded_len - 1)
end

local function decode_high_tag(input, start, available, limits)
    local number = 0
    local index = 1
    while true do
        if index >= available then fail("truncated-high-tag", start - 1 + index) end
        local octet = input:byte(start + index)
        local payload = octet & 0x7f
        if index == 1 and payload == 0 then
            fail("non-minimal-tag", start - 1 + index)
        end
        if number > (U32_MAX - payload) // 128 then
            fail("tag-overflow", start - 1 + index)
        end
        number = number * 128 + payload
        if number > limits.max_tag_number then
            fail("tag-limit-exceeded", start - 1 + index)
        end
        index = index + 1
        if (octet & 0x80) == 0 then break end
    end
    if number < 31 then fail("non-minimal-tag", start - 1) end
    return number, index
end

local function decode_length(input, start, available, identifier_len)
    local length_offset = start - 1 + identifier_len
    if identifier_len >= available then fail("truncated-length", length_offset) end
    local first = input:byte(start + identifier_len)
    if first < 0x80 then return first, 1, length_offset end
    if first == 0x80 then fail("indefinite-length", length_offset) end
    if first == 0xff then fail("reserved-length", length_offset) end

    local count = first & 0x7f
    if count > 8 then fail("length-too-wide", length_offset) end
    if identifier_len + 1 + count > available then
        fail("truncated-length", start - 1 + available)
    end
    local leading = input:byte(start + identifier_len + 1)
    if leading == 0 then fail("non-minimal-length", length_offset + 1) end
    if count == 8 and (leading & 0x80) ~= 0 then
        fail("length-host-overflow", length_offset)
    end

    local value = 0
    for index = 0, count - 1 do
        value = value * 256 + input:byte(start + identifier_len + 1 + index)
    end
    if value < 128 then fail("non-minimal-length", length_offset) end
    return value, count + 1, length_offset
end

local function decode_at(input, start, available, limits)
    if available > limits.max_input_len then fail("input-limit-exceeded", start - 1) end
    if available == 0 then fail("empty-input", start - 1) end

    local first = input:byte(start)
    local tag_class = TAG_CLASSES[(first >> 6) + 1]
    local constructed = (first & 0x20) ~= 0
    local low = first & 0x1f
    local number, identifier_len
    if low ~= 0x1f then
        number = low
        identifier_len = 1
        if number > limits.max_tag_number then fail("tag-limit-exceeded", start - 1) end
    else
        number, identifier_len = decode_high_tag(input, start, available, limits)
    end
    if tag_class == "universal" and number == 0 then
        fail("end-of-contents", start - 1)
    end

    local value_len, length_len, length_offset =
        decode_length(input, start, available, identifier_len)
    if value_len > limits.max_value_len then
        fail("value-limit-exceeded", length_offset)
    end
    local header_len = identifier_len + length_len
    if value_len > HOST_MAX - header_len then
        fail("length-host-overflow", length_offset)
    end
    local encoded_len = header_len + value_len
    if encoded_len > available then fail("truncated-value", start - 1 + available) end

    return setmetatable({
        tag = { class = tag_class, constructed = constructed, number = number },
        _input = input,
        _start = start,
        _header_len = header_len,
        _encoded_len = encoded_len,
    }, Element), start + encoded_len
end

function M.decode_one(input, provided)
    if type(input) ~= "string" then error("input must be a string", 0) end
    local element, next_start = decode_at(input, 1, #input, copy_limits(provided))
    return element, input:sub(next_start)
end

function M.decode_exact(input, provided)
    if type(input) ~= "string" then error("input must be a string", 0) end
    local element, next_start = decode_at(input, 1, #input, copy_limits(provided))
    if next_start ~= #input + 1 then fail("trailing-data", next_start - 1) end
    return element
end

local Cursor = {}
Cursor.__index = Cursor

function Cursor:remaining()
    return self._input:sub(self._offset + 1)
end

function Cursor:read()
    if self._offset == #self._input then return nil end
    if self.elements_read >= self._limits.max_elements then
        fail("element-limit-exceeded", self._offset)
    end
    local element, next_start = decode_at(
        self._input,
        self._offset + 1,
        #self._input - self._offset,
        self._limits
    )
    self._offset = next_start - 1
    self.elements_read = self.elements_read + 1
    return element
end

function Cursor:finish()
    if self._offset ~= #self._input then fail("trailing-data", self._offset) end
end

function M.new_cursor(input, provided)
    if type(input) ~= "string" then error("input must be a string", 0) end
    local limits = copy_limits(provided)
    if #input > limits.max_input_len then fail("input-limit-exceeded", 0) end
    return setmetatable({
        _input = input,
        _limits = limits,
        _offset = 0,
        elements_read = 0,
    }, Cursor)
end

return M
