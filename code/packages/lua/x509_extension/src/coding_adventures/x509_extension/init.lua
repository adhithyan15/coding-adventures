-- Bounded, payload-blind decoding of the RFC 5280 Extension shape.

local der_asn1 = require("coding_adventures.der_asn1")

local M = { VERSION = "0.1.0", Decoder = der_asn1.Decoder }

local extension_state = setmetatable({}, { __mode = "k" })
local error_state = setmetatable({}, { __mode = "k" })

local function hidden_pairs()
    return next, {}, nil
end

local function immutable_newindex()
    error("value is immutable", 0)
end

local Extension = {}
local Extension_methods = {}
local Extension_mt = {
    __index = Extension_methods,
    __newindex = immutable_newindex,
    __pairs = hidden_pairs,
    __metatable = false,
}

local function require_extension(value)
    local state = extension_state[value]
    if state == nil then error("value must be a validated Extension", 0) end
    return state
end

function Extension_methods:extension_id()
    return require_extension(self).extension_id
end

function Extension_methods:critical()
    return require_extension(self).critical
end

function Extension_methods:extension_value()
    return require_extension(self).extension_value
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
        if state == nil then return "invalid X.509 extension error" end
        return string.format("X.509 extension error %s at byte %d", state.kind, state.offset)
    end,
}

local function require_error(value)
    local state = error_state[value]
    if state == nil then error("value must be an X.509 extension Error", 0) end
    return state
end

function Error_methods:kind() return require_error(self).kind end
function Error_methods:offset() return require_error(self).offset end
function Error_methods:asn1_kind() return require_error(self).asn1_kind end
function Error_methods:framing_kind() return require_error(self).framing_kind end

local function fail(kind, offset, asn1_kind, framing_kind)
    local result = setmetatable({}, Error_mt)
    error_state[result] = {
        kind = kind,
        offset = offset,
        asn1_kind = asn1_kind,
        framing_kind = framing_kind,
    }
    error(result, 0)
end

function M.is_error(value)
    return error_state[value] ~= nil
end

local function capture_asn1(operation)
    local values = table.pack(pcall(operation))
    if values[1] then return table.unpack(values, 2, values.n) end
    local problem = values[2]
    if der_asn1.is_error(problem) then return nil, problem end
    error(problem, 0)
end

local function child_offset(root, cursor)
    return root:value_offset() + #root:value() - #cursor:remaining()
end

local function read_child(decoder, root, cursor)
    local start = child_offset(root, cursor)
    local child, problem = capture_asn1(function() return cursor:read(decoder) end)
    if problem ~= nil then
        local offset = problem:kind() == "framing"
            and (root:value_offset() + problem:offset())
            or (start + problem:offset())
        fail("structure", offset, problem:kind(), problem:framing_kind())
    end
    return child
end

local function decode_typed(kind, offset, operation)
    local value, problem = capture_asn1(operation)
    if problem ~= nil then
        fail(kind, offset + problem:offset(), problem:kind(), problem:framing_kind())
    end
    return value
end

function M.decode_extension(decoder, root)
    local cursor, opening_problem = capture_asn1(function() return decoder:sequence(root) end)
    if opening_problem ~= nil then
        fail("structure", opening_problem:offset(), opening_problem:kind(), opening_problem:framing_kind())
    end

    local extension_id_offset = child_offset(root, cursor)
    local extension_id_element = read_child(decoder, root, cursor)
    if extension_id_element == nil then fail("missing-extension-id", extension_id_offset) end
    local extension_id = decode_typed("invalid-extension-id", extension_id_offset, function()
        return der_asn1.decode_object_identifier(extension_id_element, decoder:limits())
    end)

    local second_offset = child_offset(root, cursor)
    local second = read_child(decoder, root, cursor)
    if second == nil then fail("missing-extension-value", second_offset) end

    local critical = false
    local value_element = second
    local value_offset = second_offset
    if second:tag().number == 1 then
        critical = decode_typed("invalid-critical", second_offset, function()
            return der_asn1.decode_boolean(second)
        end)
        if not critical then fail("encoded-default-critical", second_offset) end
        value_offset = child_offset(root, cursor)
        value_element = read_child(decoder, root, cursor)
        if value_element == nil then fail("missing-extension-value", value_offset) end
    end

    local extension_value = decode_typed("invalid-extension-value", value_offset, function()
        return der_asn1.decode_octet_string(value_element)
    end)
    local trailing_offset = child_offset(root, cursor)
    local trailing = read_child(decoder, root, cursor)
    if trailing ~= nil then fail("trailing-element", trailing_offset) end

    local result = setmetatable({}, Extension_mt)
    extension_state[result] = {
        extension_id = extension_id,
        critical = critical,
        extension_value = "" .. extension_value,
    }
    return result
end

M.Extension = Extension
M.Error = Error

return M
