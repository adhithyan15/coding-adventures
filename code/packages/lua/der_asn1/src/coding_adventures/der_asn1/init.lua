-- Bounded, payload-blind typed ASN.1 DER values.

local der_tlv = require("coding_adventures.der_tlv")

local M = { VERSION = "0.1.0" }

local U32_MAX = 0xffffffff
local TWO32 = 0x100000000
local DEFAULT_MAX_DEPTH = 32
local DEFAULT_MAX_TOTAL_ELEMENTS = 16384
local DEFAULT_MAX_OID_ARCS = 128

local decoder_state = setmetatable({}, { __mode = "k" })
local element_state = setmetatable({}, { __mode = "k" })
local cursor_state = setmetatable({}, { __mode = "k" })
local integer_state = setmetatable({}, { __mode = "k" })
local bit_string_state = setmetatable({}, { __mode = "k" })
local oid_state = setmetatable({}, { __mode = "k" })
local error_state = setmetatable({}, { __mode = "k" })

local function hidden_pairs()
    return next, {}, nil
end

local function immutable_newindex()
    error("value is immutable", 0)
end

local Error = {}
local Error_methods = {}
local Error_mt = {
    __index = Error_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
    __tostring = function(self)
        local state = error_state[self]
        if state == nil then return "invalid DER ASN.1 error" end
        local suffix = state.framing_kind and (" (" .. state.framing_kind .. ")") or ""
        return string.format("DER ASN.1 error %s%s at byte %d", state.kind, suffix, state.offset)
    end,
}

function Error_methods:kind()
    local state = assert(error_state[self], "invalid DER ASN.1 error")
    return state.kind
end

function Error_methods:offset()
    local state = assert(error_state[self], "invalid DER ASN.1 error")
    return state.offset
end

function Error_methods:framing_kind()
    local state = assert(error_state[self], "invalid DER ASN.1 error")
    return state.framing_kind
end

local function fail(kind, offset, framing_kind)
    local result = setmetatable({}, Error_mt)
    error_state[result] = { kind = kind, offset = offset, framing_kind = framing_kind }
    error(result, 0)
end

function M.is_error(value)
    return error_state[value] ~= nil
end

local function lower_call(fn, ...)
    local results = table.pack(pcall(fn, ...))
    if not results[1] then
        local problem = results[2]
        if type(problem) == "table"
            and type(problem.kind) == "string"
            and math.type(problem.offset) == "integer"
        then
            fail("framing", problem.offset, problem.kind)
        end
        error(problem, 0)
    end
    return table.unpack(results, 2, results.n)
end

local function copy_table(source)
    local result = {}
    for key, value in pairs(source) do result[key] = value end
    return result
end

local function validate_nonnegative_integer(name, value)
    if math.type(value) ~= "integer" or value < 0 then
        error(name .. " must be a non-negative integer", 0)
    end
end

local function normalize_limits(provided)
    if provided ~= nil and type(provided) ~= "table" then
        error("limits must be a table", 0)
    end
    local defaults = {
        der = der_tlv.default_limits(),
        max_depth = DEFAULT_MAX_DEPTH,
        max_total_elements = DEFAULT_MAX_TOTAL_ELEMENTS,
        max_oid_arcs = DEFAULT_MAX_OID_ARCS,
    }
    local result = copy_table(defaults)
    if provided ~= nil then
        for name, value in pairs(provided) do
            if defaults[name] == nil then error("unknown limit " .. tostring(name), 0) end
            result[name] = value
        end
    end

    if type(result.der) ~= "table" then error("der must be a table", 0) end
    local der_defaults = der_tlv.default_limits()
    local der = copy_table(der_defaults)
    for name, value in pairs(result.der) do
        if der_defaults[name] == nil then error("unknown DER limit " .. tostring(name), 0) end
        der[name] = value
    end
    for name, value in pairs(der) do validate_nonnegative_integer("der." .. name, value) end
    if der.max_tag_number > U32_MAX then error("der.max_tag_number must fit u32", 0) end

    validate_nonnegative_integer("max_depth", result.max_depth)
    validate_nonnegative_integer("max_total_elements", result.max_total_elements)
    validate_nonnegative_integer("max_oid_arcs", result.max_oid_arcs)
    result.der = der
    return result
end

local function copy_limits(limits)
    return {
        der = copy_table(limits.der),
        max_depth = limits.max_depth,
        max_total_elements = limits.max_total_elements,
        max_oid_arcs = limits.max_oid_arcs,
    }
end

function M.default_limits()
    return normalize_limits(nil)
end

local function validate_tag_number(number)
    if math.type(number) ~= "integer" or number < 0 or number > U32_MAX then
        error("tag_number must be a u32 integer", 0)
    end
end

local Element = {}
local Element_methods = {}
local Element_mt = {
    __index = Element_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function require_element(element)
    local state = element_state[element]
    if state == nil then error("element must be a validated Element", 0) end
    return state
end

function Element_methods:tag()
    local tag = require_element(self).tag
    return { class = tag.class, constructed = tag.constructed, number = tag.number }
end

function Element_methods:header() return require_element(self).header end
function Element_methods:value() return require_element(self).value end
function Element_methods:encoded() return require_element(self).encoded end
function Element_methods:depth() return require_element(self).depth end
function Element_methods:value_offset() return #require_element(self).header end

local function wrap_element(framed, depth, owner)
    local tag = framed.tag
    local result = setmetatable({}, Element_mt)
    element_state[result] = {
        tag = { class = tag.class, constructed = tag.constructed, number = tag.number },
        header = framed:header(),
        value = framed:value(),
        encoded = framed:encoded(),
        depth = depth,
        owner = owner,
    }
    return result
end

local function expect_tag(element, tag_class, constructed, number)
    validate_tag_number(number)
    local state = require_element(element)
    local tag = state.tag
    if tag.class ~= tag_class or tag.constructed ~= constructed or tag.number ~= number then
        fail("unexpected-tag", 0)
    end
    return state
end

local function primitive(element, tag_class, number)
    return expect_tag(element, tag_class, false, number).value
end

local Decoder = {}
local Decoder_methods = {}
local Decoder_mt = {
    __index = Decoder_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function require_decoder(decoder)
    local state = decoder_state[decoder]
    if state == nil then error("decoder must be a Decoder", 0) end
    return state
end

function Decoder.new(limits)
    local result = setmetatable({}, Decoder_mt)
    decoder_state[result] = {
        limits = normalize_limits(limits),
        elements_read = 0,
        owner = {},
    }
    return result
end

M.new_decoder = Decoder.new

function Decoder_methods:limits()
    return copy_limits(require_decoder(self).limits)
end

function Decoder_methods:elements_read()
    return require_decoder(self).elements_read
end

function Decoder_methods:decode_exact(input)
    local state = require_decoder(self)
    if type(input) ~= "string" then error("input must be a string", 0) end
    if state.limits.max_depth == 0 then fail("depth-limit-exceeded", 0) end
    if state.elements_read >= state.limits.max_total_elements then
        fail("element-limit-exceeded", 0)
    end
    if #input > state.limits.der.max_input_len then
        fail("framing", 0, "input-limit-exceeded")
    end
    local framed = lower_call(der_tlv.decode_exact, input, state.limits.der)
    local element = wrap_element(framed, 0, state.owner)
    state.elements_read = state.elements_read + 1
    return element
end

local function check_container(decoder, element, tag_class, number)
    local decoder_data = require_decoder(decoder)
    local element_data = expect_tag(element, tag_class, true, number)
    if element_data.owner ~= decoder_data.owner then fail("decoder-limit-mismatch", 0) end
    if element_data.depth + 1 >= decoder_data.limits.max_depth then
        fail("depth-limit-exceeded", 0)
    end
    return decoder_data, element_data
end

local Cursor = {}
local Cursor_methods = {}
local Cursor_mt = {
    __index = Cursor_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function new_cursor(decoder, element, tag_class, number)
    local decoder_data, element_data = check_container(decoder, element, tag_class, number)
    local framing = lower_call(der_tlv.new_cursor, element_data.value, decoder_data.limits.der)
    local result = setmetatable({}, Cursor_mt)
    cursor_state[result] = {
        framing = framing,
        depth = element_data.depth + 1,
        owner = decoder_data.owner,
    }
    return result
end

function Decoder_methods:sequence(element)
    return new_cursor(self, element, "universal", 16)
end

function Decoder_methods:set(element)
    return new_cursor(self, element, "universal", 17)
end

function Decoder_methods:explicit(element, tag_number)
    local decoder_data, element_data = check_container(self, element, "context-specific", tag_number)
    if decoder_data.elements_read >= decoder_data.limits.max_total_elements then
        fail("element-limit-exceeded", #element_data.header)
    end
    local framed = lower_call(der_tlv.decode_exact, element_data.value, decoder_data.limits.der)
    local child = wrap_element(framed, element_data.depth + 1, decoder_data.owner)
    decoder_data.elements_read = decoder_data.elements_read + 1
    return child
end

local function require_cursor(cursor)
    local state = cursor_state[cursor]
    if state == nil then error("cursor must be a Cursor", 0) end
    return state
end

function Cursor_methods:remaining()
    return require_cursor(self).framing:remaining()
end

function Cursor_methods:read(decoder)
    local state = require_cursor(self)
    if state.framing:remaining() == "" then return nil end
    local decoder_data = require_decoder(decoder)
    if state.owner ~= decoder_data.owner then fail("decoder-limit-mismatch", 0) end
    if decoder_data.elements_read >= decoder_data.limits.max_total_elements then
        fail("element-limit-exceeded", 0)
    end
    local framed = lower_call(state.framing.read, state.framing)
    if framed == nil then return nil end
    local child = wrap_element(framed, state.depth, state.owner)
    decoder_data.elements_read = decoder_data.elements_read + 1
    return child
end

function Cursor_methods:finish()
    return lower_call(require_cursor(self).framing.finish, require_cursor(self).framing)
end

local DerInteger = {}
local DerInteger_methods = {}
local DerInteger_mt = {
    __index = DerInteger_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function require_integer(value)
    local state = integer_state[value]
    if state == nil then error("value must be a DerInteger", 0) end
    return state
end

function DerInteger_methods:signed_bytes() return require_integer(self).bytes end
function DerInteger_methods:negative() return (require_integer(self).bytes:byte(1) & 0x80) ~= 0 end
DerInteger_methods.is_negative = DerInteger_methods.negative

local function wide_mul_add(hi, lo, base, add, kind, offset)
    local low_product = lo * base + add
    local carry = low_product // TWO32
    local next_lo = low_product % TWO32
    if hi > (U32_MAX - carry) // base then fail(kind, offset) end
    return hi * base + carry, next_lo
end

local function wide_decimal(hi, lo)
    if hi == 0 and lo == 0 then return "0" end
    local digits = {}
    while hi ~= 0 or lo ~= 0 do
        local next_hi = hi // 10
        local combined = (hi % 10) * TWO32 + lo
        local next_lo = combined // 10
        digits[#digits + 1] = string.char(48 + (combined % 10))
        hi, lo = next_hi, next_lo
    end
    local result = {}
    for index = #digits, 1, -1 do result[#result + 1] = digits[index] end
    return table.concat(result)
end

function DerInteger_methods:to_u64_decimal()
    local state = require_integer(self)
    local bytes = state.bytes
    if (bytes:byte(1) & 0x80) ~= 0 then fail("negative-integer", state.value_offset) end
    if #bytes > 1 and bytes:byte(1) == 0 then bytes = bytes:sub(2) end
    if #bytes > 8 then fail("integer-overflow", state.value_offset) end
    local hi, lo = 0, 0
    for index = 1, #bytes do
        hi, lo = wide_mul_add(hi, lo, 256, bytes:byte(index), "integer-overflow", state.value_offset)
    end
    return wide_decimal(hi, lo)
end

local DerBitString = {}
local DerBitString_methods = {}
local DerBitString_mt = {
    __index = DerBitString_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function require_bit_string(value)
    local state = bit_string_state[value]
    if state == nil then error("value must be a DerBitString", 0) end
    return state
end

function DerBitString_methods:bytes() return require_bit_string(self).bytes end
function DerBitString_methods:unused_bits() return require_bit_string(self).unused_bits end
function DerBitString_methods:bit_length() return require_bit_string(self).bit_length end

local ObjectIdentifier = {}
local ObjectIdentifier_methods = {}
local ObjectIdentifier_mt = {
    __index = ObjectIdentifier_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function require_oid(value)
    local state = oid_state[value]
    if state == nil then error("value must be an ObjectIdentifier", 0) end
    return state
end

function ObjectIdentifier_methods:encoded() return require_oid(self).encoded end
function ObjectIdentifier_methods:arcs()
    local result = {}
    for index, arc in ipairs(require_oid(self).arcs) do result[index] = arc end
    return result
end
function ObjectIdentifier_methods:arc_count() return #require_oid(self).arcs end
function ObjectIdentifier_methods:equals(expected)
    if type(expected) ~= "table" then return false end
    local arcs = require_oid(self).arcs
    if #expected ~= #arcs then return false end
    for index, arc in ipairs(arcs) do
        if tostring(expected[index]) ~= arc then return false end
    end
    return true
end
function ObjectIdentifier_methods:equal(other)
    local other_state = oid_state[other]
    if other_state == nil then return false end
    local state = require_oid(self)
    if state.encoded ~= other_state.encoded or #state.arcs ~= #other_state.arcs then return false end
    for index, arc in ipairs(state.arcs) do
        if arc ~= other_state.arcs[index] then return false end
    end
    return true
end

function M.decode_boolean(element)
    local value = primitive(element, "universal", 1)
    local offset = #require_element(element).header
    if #value ~= 1 then fail("invalid-boolean-length", offset) end
    if value:byte(1) == 0 then return false end
    if value:byte(1) == 0xff then return true end
    fail("invalid-boolean-value", offset)
end

function M.decode_integer(element)
    local value = primitive(element, "universal", 2)
    local offset = #require_element(element).header
    if #value == 0 then fail("empty-integer", offset) end
    if #value > 1 then
        local first, second = value:byte(1), value:byte(2)
        if (first == 0 and (second & 0x80) == 0)
            or (first == 0xff and (second & 0x80) ~= 0)
        then
            fail("non-minimal-integer", offset)
        end
    end
    local result = setmetatable({}, DerInteger_mt)
    integer_state[result] = { bytes = value, value_offset = offset }
    return result
end

function M.decode_bit_string(element)
    local value = primitive(element, "universal", 3)
    local offset = #require_element(element).header
    if #value == 0 then fail("missing-unused-bit-count", offset) end
    local unused = value:byte(1)
    local payload = value:sub(2)
    if unused > 7 or (#payload == 0 and unused ~= 0) then
        fail("invalid-unused-bit-count", offset)
    end
    if #payload > 0 and unused > 0 and (payload:byte(-1) & ((1 << unused) - 1)) ~= 0 then
        fail("non-zero-bit-padding", offset + #value - 1)
    end
    local threshold = math.maxinteger // 8
    if unused > 0 then threshold = threshold + 1 end
    if #payload > threshold then fail("bit-length-overflow", offset) end
    local result = setmetatable({}, DerBitString_mt)
    bit_string_state[result] = {
        bytes = payload,
        unused_bits = unused,
        bit_length = (#payload * 8) - unused,
    }
    return result
end

function M.decode_octet_string(element)
    return primitive(element, "universal", 4)
end

function M.decode_implicit_octet_string(element, tag_number)
    return primitive(element, "context-specific", tag_number)
end

local function decode_ia5(element, tag_class, number)
    local value = primitive(element, tag_class, number)
    local offset = #require_element(element).header
    for index = 1, #value do
        if value:byte(index) > 0x7f then fail("non-ascii-ia5-string", offset + index - 1) end
    end
    return value
end

function M.decode_ia5_string(element) return decode_ia5(element, "universal", 22) end
function M.decode_implicit_ia5_string(element, tag_number)
    return decode_ia5(element, "context-specific", tag_number)
end

function M.decode_null(element)
    local value = primitive(element, "universal", 5)
    if #value ~= 0 then fail("non-empty-null", #require_element(element).header) end
end

local function oid_mul_add(hi, lo, extra, octet, allow_first_extra, offset)
    local low_product = lo * 128 + (octet & 0x7f)
    local next_lo = low_product % TWO32
    local high_product = hi * 128 + (low_product // TWO32)
    local next_hi = high_product % TWO32
    local next_extra = extra * 128 + (high_product // TWO32)
    local valid_extra = allow_first_extra
        and next_extra <= 1
        and (next_extra == 0 or (next_hi == 0 and next_lo <= 79))
    if next_extra ~= 0 and not valid_extra then
        fail("object-identifier-overflow", offset)
    end
    return next_hi, next_lo, next_extra
end

local function oid_subidentifier(value, start, value_offset, allow_first_extra)
    if value:byte(start) == 0x80 then
        fail("non-minimal-object-identifier", value_offset + start - 1)
    end
    local hi, lo, extra = 0, 0, 0
    local index = start
    while true do
        if index > #value then fail("unterminated-object-identifier", value_offset + index - 1) end
        local octet = value:byte(index)
        hi, lo, extra = oid_mul_add(
            hi, lo, extra, octet, allow_first_extra, value_offset + index - 1
        )
        index = index + 1
        if (octet & 0x80) == 0 then return hi, lo, extra, index end
    end
end

local function wide_sub_small(hi, lo, amount)
    if lo >= amount then return hi, lo - amount end
    return hi - 1, (lo + TWO32) - amount
end

local function decode_oid(element, tag_class, number, limits)
    local value = primitive(element, tag_class, number)
    local value_offset = #require_element(element).header
    local configured = normalize_limits(limits)
    if #value == 0 then fail("empty-object-identifier", value_offset) end
    local first_hi, first_lo, first_extra, index = oid_subidentifier(value, 1, value_offset, true)
    local arcs
    if first_extra == 0 and first_hi == 0 and first_lo < 40 then
        arcs = { "0", tostring(first_lo) }
    elseif first_extra == 0 and first_hi == 0 and first_lo < 80 then
        arcs = { "1", tostring(first_lo - 40) }
    elseif first_extra == 1 then
        arcs = { "2", wide_decimal(U32_MAX, (TWO32 + first_lo) - 80) }
    else
        local arc_hi, arc_lo = wide_sub_small(first_hi, first_lo, 80)
        arcs = { "2", wide_decimal(arc_hi, arc_lo) }
    end
    if #arcs > configured.max_oid_arcs then fail("oid-arc-limit-exceeded", value_offset) end
    while index <= #value do
        local arc_start = index
        local hi, lo, extra
        hi, lo, extra, index = oid_subidentifier(value, index, value_offset, false)
        assert(extra == 0)
        if #arcs >= configured.max_oid_arcs then
            fail("oid-arc-limit-exceeded", value_offset + arc_start - 1)
        end
        arcs[#arcs + 1] = wide_decimal(hi, lo)
    end
    local result = setmetatable({}, ObjectIdentifier_mt)
    oid_state[result] = { encoded = value, arcs = arcs }
    return result
end

function M.decode_object_identifier(element, limits)
    return decode_oid(element, "universal", 6, limits)
end

function M.decode_implicit_object_identifier(element, tag_number, limits)
    return decode_oid(element, "context-specific", tag_number, limits)
end

M.Error = Error
M.Decoder = Decoder
M.Element = Element
M.Cursor = Cursor
M.DerInteger = DerInteger
M.DerBitString = DerBitString
M.ObjectIdentifier = ObjectIdentifier

return M
