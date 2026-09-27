package.path = table.concat({
    "../src/?.lua;../src/?/init.lua;",
    "../../der_asn1/src/?.lua;../../der_asn1/src/?/init.lua;",
    "../../der_tlv/src/?.lua;../../der_tlv/src/?/init.lua;",
    package.path,
})

local json = require("dkjson")
local der_asn1 = require("coding_adventures.der_asn1")
local der_tlv = require("coding_adventures.der_tlv")
local x509_extension = require("coding_adventures.x509_extension")

local function read_json(path)
    local handle = assert(io.open(path, "rb"))
    local contents = handle:read("*a")
    handle:close()
    local decoded, _, problem = json.decode(contents)
    assert.is_nil(problem)
    return decoded
end

local FIXTURE = read_json("../../../../specs/fixtures/x509-extension-v1/cases.json")
local UPSTREAM = read_json("../../../../specs/fixtures/der-asn1-v1/cases.json")

local function hex_to_bytes(value)
    return (value:gsub("..", function(pair) return string.char(tonumber(pair, 16)) end))
end

local function bytes_to_hex(value)
    return (value:gsub(".", function(byte) return string.format("%02x", string.byte(byte)) end))
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

local function shallow_copy(source)
    local result = {}
    for key, value in pairs(source) do result[key] = value end
    return result
end

local function deeply_equal(left, right)
    if type(left) ~= type(right) then return false end
    if type(left) ~= "table" then return left == right end
    for key, value in pairs(left) do
        if not deeply_equal(value, right[key]) then return false end
    end
    for key in pairs(right) do
        if left[key] == nil then return false end
    end
    return true
end

local function case_limits(test_case)
    local override = test_case.limits or {}
    local result = shallow_copy(UPSTREAM.defaults)
    for key, value in pairs(override) do result[key] = value end
    result.der = shallow_copy(UPSTREAM.defaults.der)
    for key, value in pairs(override.der or {}) do result.der[key] = value end
    if result.der.max_value_len == "host-max" then result.der.max_value_len = der_tlv.HOST_MAX end
    return result
end

local function capture(operation)
    local values = table.pack(pcall(operation))
    if not values[1] then return nil, values[2] end
    return table.pack(table.unpack(values, 2, values.n)), nil
end

local function project_error(problem, decoder)
    local result = {
        outcome = "error",
        error_id = problem:kind(),
        offset = problem:offset(),
        offset_scope = "extension-element",
        elements_read = decoder:elements_read(),
    }
    if problem:asn1_kind() ~= nil then result.asn1_error_id = problem:asn1_kind() end
    if problem:framing_kind() ~= nil then result.framing_error_id = problem:framing_kind() end
    return result
end

local function attempt(decoder, root)
    local values, problem = capture(function() return x509_extension.decode_extension(decoder, root) end)
    if problem ~= nil then
        assert.is_true(x509_extension.is_error(problem))
        return project_error(problem, decoder), problem
    end
    local extension = values[1]
    return {
        outcome = "value",
        extension_id_arcs_decimal = extension:extension_id():arcs(),
        critical = extension:critical(),
        extension_value_hex = bytes_to_hex(extension:extension_value()),
        elements_read = decoder:elements_read(),
    }, nil
end

local function run_case(test_case)
    local decoder = der_asn1.Decoder.new(case_limits(test_case))
    local root = decoder:decode_exact(materialize(test_case.input))
    if test_case.operation ~= "extension-script" then return attempt(decoder, root) end
    local events = {}
    for _ in ipairs(test_case.actions) do
        events[#events + 1] = attempt(decoder, root)
    end
    return { outcome = "script", events = events }
end

describe("portable X.509 Extension conformance", function()
    it("executes the complete language-neutral fixture", function()
        assert.equals(48, #FIXTURE.cases)
        assert.equals(8, #FIXTURE.error_ids)
        for _, test_case in ipairs(FIXTURE.cases) do
            local actual, problem = run_case(test_case)
            assert.is_true(deeply_equal(test_case.expected, actual), test_case.id .. ": " .. json.encode(actual))
            if test_case.redacted_input_hex ~= nil then
                assert.is_nil(json.encode(actual):find(test_case.redacted_input_hex, 1, true))
                assert.is_not_nil(problem)
                assert.is_nil(tostring(problem):lower():find(test_case.redacted_input_hex:lower(), 1, true))
            end
        end
    end)

    it("keeps values private, immutable, and payload-blind", function()
        local input = hex_to_bytes("30090603551d1104023000")
        local decoder = der_asn1.Decoder.new()
        local root = decoder:decode_exact(input)
        local extension = x509_extension.decode_extension(decoder, root)
        assert.same({ "2", "5", "29", "17" }, extension:extension_id():arcs())
        assert.is_false(extension:critical())
        assert.equals("3000", bytes_to_hex(extension:extension_value()))
        assert.is_false(getmetatable(extension))
        assert.same({}, (function()
            local exposed = {}
            for key in pairs(extension) do exposed[#exposed + 1] = key end
            return exposed
        end)())
        assert.has_error(function() extension.extra = true end, "value is immutable")
        local arcs = extension:extension_id():arcs()
        arcs[1] = "999"
        assert.same({ "2", "5", "29", "17" }, extension:extension_id():arcs())
        assert.has_error(function() return x509_extension.Extension:extension_value() end)
        assert.has_error(function() return x509_extension.Error:kind() end)
        assert.is_false(x509_extension.is_error({}))

        local hostile_decoder = der_asn1.Decoder.new()
        local hostile = hostile_decoder:decode_exact(hex_to_bytes("30080601800403deadbe"))
        local _, problem = capture(function()
            return x509_extension.decode_extension(hostile_decoder, hostile)
        end)
        assert.is_true(x509_extension.is_error(problem))
        assert.equals("invalid-extension-id", problem:kind())
        assert.equals("non-minimal-object-identifier", problem:asn1_kind())
        assert.equals(4, problem:offset())
        assert.is_nil(tostring(problem):lower():find("deadbe", 1, true))
        assert.has_error(function() problem.extra = true end, "value is immutable")
    end)

    it("does not convert unexpected programmer errors", function()
        local decoder = der_asn1.Decoder.new()
        local _, problem = capture(function()
            return x509_extension.decode_extension(decoder, {})
        end)
        assert.is_false(x509_extension.is_error(problem))
        assert.is_truthy(tostring(problem):find("validated Element", 1, true))
    end)
end)
