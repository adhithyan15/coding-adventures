package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local json = require("dkjson")
local der_tlv = require("coding_adventures.der_tlv")

local function read_fixture()
    local handle = assert(io.open("../../../../specs/fixtures/der-tlv-v1/cases.json", "rb"))
    local contents = handle:read("*a")
    handle:close()
    local decoded, _, decode_error = json.decode(contents)
    assert.is_nil(decode_error)
    return decoded
end

local FIXTURE = read_fixture()

local function hex_to_bytes(value)
    return (value:gsub("..", function(pair) return string.char(tonumber(pair, 16)) end))
end

local function materialize(segments)
    local parts = {}
    for _, segment in ipairs(segments) do
        if segment.hex ~= nil then
            parts[#parts + 1] = hex_to_bytes(segment.hex)
        else
            parts[#parts + 1] = string.rep(hex_to_bytes(segment.repeat_hex), segment.count)
        end
    end
    return table.concat(parts)
end

local function case_limits(test_case)
    local result = {}
    for name, value in pairs(FIXTURE.defaults) do result[name] = value end
    for name, value in pairs(test_case.limits or {}) do result[name] = value end
    if result.max_value_len == "host-max" then result.max_value_len = der_tlv.HOST_MAX end
    return result
end

local function element_projection(element, offset)
    return {
        outcome = "element",
        element_offset = offset,
        tag = element.tag,
        header_len = #element:header(),
        encoded_len = #element:encoded(),
        remainder_offset = offset + #element:encoded(),
    }
end

local function error_projection(problem)
    return { outcome = "error", error_id = problem.kind, offset = problem.offset }
end

local function capture(operation)
    local ok, first, second = pcall(operation)
    if ok then return first, second, nil end
    return nil, nil, first
end

local function run_decode(test_case, input, limits)
    local element, remainder, problem = capture(function()
        if test_case.operation == "decode-one" then
            return der_tlv.decode_one(input, limits)
        end
        return der_tlv.decode_exact(input, limits)
    end)
    if problem ~= nil then return error_projection(problem) end
    local result = element_projection(element, 0)
    if remainder ~= nil then
        assert.equals(result.remainder_offset, #input - #remainder)
    end
    return result
end

local function run_cursor(test_case, input, limits)
    local cursor = der_tlv.new_cursor(input, limits)
    local events = {}
    for _, action in ipairs(test_case.actions) do
        if action == "finish" then
            local _, _, problem = capture(function() return cursor:finish() end)
            events[#events + 1] = problem == nil
                and { outcome = "finished" }
                or error_projection(problem)
        else
            local offset = #input - #cursor:remaining()
            local element, _, problem = capture(function() return cursor:read() end)
            if problem ~= nil then
                events[#events + 1] = error_projection(problem)
            elseif element == nil then
                events[#events + 1] = { outcome = "end" }
            else
                events[#events + 1] = element_projection(element, offset)
            end
        end
    end
    return {
        events = events,
        elements_read = cursor.elements_read,
        remaining_offset = #input - #cursor:remaining(),
    }
end

describe("portable DER TLV conformance", function()
    it("pins the closed fixture", function()
        assert.equals(54, #FIXTURE.cases)
        assert.equals(17, #FIXTURE.error_ids)
    end)

    for _, test_case in ipairs(FIXTURE.cases) do
        it(test_case.id, function()
            local input = materialize(test_case.input)
            local actual = test_case.operation == "cursor"
                and run_cursor(test_case, input, case_limits(test_case))
                or run_decode(test_case, input, case_limits(test_case))
            assert.same(test_case.expected, actual)
            if test_case.redacted_input_hex ~= nil then
                assert.is_nil(json.encode(actual):find(test_case.redacted_input_hex, 1, true))
            end
        end)
    end

    it("exposes exact slices and public defaults", function()
        local element = der_tlv.decode_exact("\x04\x01\x2a")
        assert.equals("\x04\x01", element:header())
        assert.equals("\x2a", element:value())
        assert.equals(4096, der_tlv.default_limits().max_elements)
    end)

    it("rejects invalid limits and inputs", function()
        assert.has_error(function() der_tlv.decode_exact("", { max_elements = -1 }) end)
        assert.has_error(function()
            der_tlv.decode_exact("", { max_tag_number = 0x100000000 })
        end)
        assert.has_error(function() der_tlv.decode_exact({}) end)
        assert.has_error(function() der_tlv.new_cursor({}) end)
    end)
end)
