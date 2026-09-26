package.path = "../src/?.lua;../src/?/init.lua;../../der_tlv/src/?.lua;../../der_tlv/src/?/init.lua;" .. package.path

local json = require("dkjson")
local der_asn1 = require("coding_adventures.der_asn1")
local der_tlv = require("coding_adventures.der_tlv")

local function read_json(path)
    local handle = assert(io.open(path, "rb"))
    local contents = handle:read("*a")
    handle:close()
    local decoded, _, problem = json.decode(contents)
    assert.is_nil(problem)
    return decoded
end

local FIXTURE = read_json("../../../../specs/fixtures/der-asn1-v1/cases.json")
local UPSTREAM = read_json("../../../../specs/fixtures/der-tlv-v1/cases.json")

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
    local result = shallow_copy(FIXTURE.defaults)
    for key, value in pairs(override) do result[key] = value end
    result.der = shallow_copy(FIXTURE.defaults.der)
    for key, value in pairs(override.der or {}) do result.der[key] = value end
    if result.der.max_value_len == "host-max" then result.der.max_value_len = der_tlv.HOST_MAX end
    return result
end

local function capture(operation)
    local values = table.pack(pcall(operation))
    if not values[1] then return nil, values[2] end
    return table.pack(table.unpack(values, 2, values.n)), nil
end

local function failure(problem, scope)
    local result = {
        outcome = "error",
        error_id = problem:kind(),
        offset = problem:offset(),
        offset_scope = scope or "operation-input",
    }
    if problem:framing_kind() ~= nil then result.framing_error_id = problem:framing_kind() end
    return result
end

local function tag(element)
    return element:tag()
end

local function upstream_case(test_case)
    local upstream
    for _, candidate in ipairs(UPSTREAM.cases) do
        if candidate.id == test_case.der_tlv_case_id then upstream = candidate end
    end
    assert.is_not_nil(upstream)
    local limits = shallow_copy(UPSTREAM.defaults)
    for key, value in pairs(upstream.limits or {}) do limits[key] = value end
    if limits.max_value_len == "host-max" then limits.max_value_len = der_tlv.HOST_MAX end
    local decoder = der_asn1.Decoder.new({ der = limits })
    local values, problem = capture(function() return decoder:decode_exact(materialize(upstream.input)) end)
    local actual
    if problem ~= nil then
        assert.is_true(der_asn1.is_error(problem))
        actual = { outcome = "error", error_id = problem:framing_kind(), offset = problem:offset() }
        if upstream.redacted_input_hex ~= nil then
            assert.is_nil(tostring(problem):find(upstream.redacted_input_hex, 1, true))
        end
    else
        local element = values[1]
        assert.equals(1, decoder:elements_read())
        actual = {
            outcome = "element",
            element_offset = 0,
            tag = tag(element),
            header_len = #element:header(),
            encoded_len = #element:encoded(),
            remainder_offset = #element:encoded(),
        }
    end
    if upstream.redacted_input_hex ~= nil then
        assert.is_nil(json.encode(actual):find(upstream.redacted_input_hex, 1, true))
    end
    if deeply_equal(actual, upstream.expected) then return { outcome = "upstream" } end
    return actual
end

local function primitive_result(operation, element, configured, tag_number)
    if operation == "decode-boolean" then
        return { outcome = "value", boolean = der_asn1.decode_boolean(element) }
    elseif operation == "decode-integer" or operation == "integer-to-u64" then
        local integer = der_asn1.decode_integer(element)
        local result = {
            outcome = "value",
            signed_hex = bytes_to_hex(integer:signed_bytes()),
            negative = integer:negative(),
        }
        if operation == "integer-to-u64" then result.u64_decimal = integer:to_u64_decimal() end
        return result
    elseif operation == "decode-bit-string" then
        local bits = der_asn1.decode_bit_string(element)
        return {
            outcome = "value",
            bytes_hex = bytes_to_hex(bits:bytes()),
            unused_bits = bits:unused_bits(),
            bit_length = bits:bit_length(),
        }
    elseif operation == "decode-octet-string" then
        return { outcome = "value", bytes_hex = bytes_to_hex(der_asn1.decode_octet_string(element)) }
    elseif operation == "decode-implicit-octet-string" then
        return {
            outcome = "value",
            bytes_hex = bytes_to_hex(der_asn1.decode_implicit_octet_string(element, tag_number)),
        }
    elseif operation == "decode-ia5-string" then
        return { outcome = "value", text = der_asn1.decode_ia5_string(element) }
    elseif operation == "decode-implicit-ia5-string" then
        return { outcome = "value", text = der_asn1.decode_implicit_ia5_string(element, tag_number) }
    elseif operation == "decode-null" then
        der_asn1.decode_null(element)
        return { outcome = "value" }
    elseif operation == "decode-object-identifier"
        or operation == "decode-implicit-object-identifier"
    then
        local oid = operation == "decode-object-identifier"
            and der_asn1.decode_object_identifier(element, configured)
            or der_asn1.decode_implicit_object_identifier(element, tag_number, configured)
        return {
            outcome = "value",
            bytes_hex = bytes_to_hex(oid:encoded()),
            arcs_decimal = oid:arcs(),
            arc_count = oid:arc_count(),
        }
    end
    error("unsupported operation " .. operation)
end

local function cursor_result(test_case, decoder, root)
    local cursor = decoder:sequence(root)
    local total = #cursor:remaining()
    local events = {}
    for _, action in ipairs(test_case.actions) do
        if action == "finish" then
            local _, problem = capture(function() return cursor:finish() end)
            events[#events + 1] = problem == nil
                and { outcome = "finished" }
                or failure(problem, "container-value")
        else
            local active = decoder
            if action == "read-with-different-limits" then
                local limits = decoder:limits()
                limits.max_total_elements = limits.max_total_elements + 1
                active = der_asn1.Decoder.new(limits)
            end
            local values, problem = capture(function() return cursor:read(active) end)
            if problem ~= nil then
                events[#events + 1] = failure(problem, "container-value")
            else
                local child = values[1]
                if action == "read-nested-sequence" then
                    assert.is_not_nil(child)
                    local nested_values, nested_problem = capture(function()
                        local nested = decoder:sequence(child)
                        local grandchild = nested:read(decoder)
                        assert.is_not_nil(grandchild)
                        nested:finish()
                        return grandchild
                    end)
                    if nested_problem ~= nil then
                        events[#events + 1] = failure(nested_problem, "container-value")
                    else
                        local grandchild = nested_values[1]
                        events[#events + 1] = {
                            outcome = "value",
                            tag = tag(grandchild),
                            depth = grandchild:depth(),
                        }
                    end
                elseif child == nil then
                    events[#events + 1] = { outcome = "end" }
                else
                    events[#events + 1] = {
                        outcome = "value",
                        tag = tag(child),
                        depth = child:depth(),
                    }
                end
            end
        end
    end
    return {
        outcome = "value",
        elements_read = decoder:elements_read(),
        remaining_offset = total - #cursor:remaining(),
        events = events,
    }
end

local function run_case(test_case)
    if test_case.der_tlv_case_id ~= nil then return upstream_case(test_case) end
    local configured = case_limits(test_case)
    local decoder = der_asn1.Decoder.new(configured)
    local operation = test_case.operation
    local values, problem = capture(function()
        local root = decoder:decode_exact(materialize(test_case.input))
        if operation == "decode-exact" then
            return {
                outcome = "value",
                tag = tag(root),
                header_hex = bytes_to_hex(root:header()),
                value_hex = bytes_to_hex(root:value()),
                encoded_hex = bytes_to_hex(root:encoded()),
                depth = root:depth(),
                elements_read = decoder:elements_read(),
            }
        elseif operation == "cursor-script" then
            return cursor_result(test_case, decoder, root)
        elseif operation == "sequence" or operation == "set" then
            local cursor = operation == "sequence" and decoder:sequence(root) or decoder:set(root)
            return {
                outcome = "value",
                elements_read = decoder:elements_read(),
                remaining_offset = #root:value() - #cursor:remaining(),
            }
        elseif operation == "explicit" then
            local child = decoder:explicit(root, test_case.tag_number)
            return {
                outcome = "value",
                tag = tag(child),
                value_hex = bytes_to_hex(child:value()),
                depth = child:depth(),
                elements_read = decoder:elements_read(),
            }
        end
        local result = primitive_result(operation, root, configured, test_case.tag_number)
        if test_case.expected.elements_read ~= nil then
            result.elements_read = decoder:elements_read()
        end
        return result
    end)
    if problem == nil then return values[1] end
    assert.is_true(der_asn1.is_error(problem), tostring(problem))
    local scope = operation == "explicit" and problem:kind() == "framing"
        and "container-value"
        or "operation-input"
    return failure(problem, scope)
end

describe("portable DER ASN.1 conformance", function()
    it("executes the closed neutral fixture", function()
        assert.equals(122, #FIXTURE.cases)
        assert.equals(22, #FIXTURE.error_ids)
        local references = {}
        for _, test_case in ipairs(FIXTURE.cases) do
            if test_case.der_tlv_case_id ~= nil then references[test_case.der_tlv_case_id] = true end
            local actual = run_case(test_case)
            assert.same(test_case.expected, actual, test_case.id)
            if test_case.redacted_input_hex ~= nil then
                assert.is_nil(json.encode(actual):find(test_case.redacted_input_hex, 1, true))
            end
        end
        local count = 0
        for _ in pairs(references) do count = count + 1 end
        assert.equals(46, count)
    end)

    it("seals wrappers, snapshots tags, and binds owners", function()
        local decoder = der_asn1.Decoder.new()
        local element = decoder:decode_exact("\x04\x01\x2a")
        local exposed = element:tag()
        exposed.number = 99
        rawset(element, "value", "secret")
        assert.equals("\x2a", der_asn1.decode_octet_string(element))
        assert.equals(4, element:tag().number)
        assert.has_error(function() der_asn1.decode_octet_string(setmetatable({}, {})) end)
        assert.has_error(function() der_asn1.Decoder.new():sequence(element) end)
    end)

    it("rejects invalid limits, tags, and forged typed values", function()
        assert.has_error(function() der_asn1.Decoder.new({ max_depth = -1 }) end)
        assert.has_error(function() der_asn1.Decoder.new({ max_oid_arcs = "1" }) end)
        assert.has_error(function() der_asn1.Decoder.new({ unknown = 1 }) end)
        assert.has_error(function() der_asn1.Decoder.new({ der = { unknown = 1 } }) end)
        assert.has_error(function() der_asn1.Decoder.new():decode_exact({}) end)
        local element = der_asn1.Decoder.new():decode_exact("\x04\x00")
        assert.has_error(function() der_asn1.decode_implicit_octet_string(element, -1) end)
        assert.has_error(function() der_asn1.decode_implicit_octet_string(element, 0x100000000) end)
        assert.has_error(function() der_asn1.DerInteger.to_u64_decimal({}) end)
    end)

    it("keeps budgets and cursor failures transactional", function()
        local decoder = der_asn1.Decoder.new({
            max_total_elements = 3,
            der = { max_elements = 1 },
        })
        local root = decoder:decode_exact("\x30\x04\x05\x00\x05\x00")
        local cursor = decoder:sequence(root)
        assert.is_not_nil(cursor:read(decoder))
        local before = cursor:remaining()
        local _, problem = capture(function() return cursor:read(decoder) end)
        assert.equals("framing", problem:kind())
        assert.equals("element-limit-exceeded", problem:framing_kind())
        assert.equals(before, cursor:remaining())
        assert.equals(2, decoder:elements_read())
    end)

    it("compares exact OIDs and redacts errors", function()
        local first = der_asn1.decode_object_identifier(
            der_asn1.Decoder.new():decode_exact("\x06\x03\x2a\x03\x04")
        )
        local second = der_asn1.decode_object_identifier(
            der_asn1.Decoder.new():decode_exact("\x06\x03\x2a\x03\x04")
        )
        assert.is_true(first:equal(second))
        assert.is_true(first:equals({ "1", "2", "3", "4" }))
        assert.is_false(first:equals({ "1", "2", "3", "5" }))
        local arcs = first:arcs()
        arcs[1] = "9"
        assert.equals("1", first:arcs()[1])

        local combined = hex_to_bytes("8280808080808080804f")
        local universal = der_asn1.Decoder.new():decode_exact("\x06\x0a" .. combined)
        local universal_oid = der_asn1.decode_object_identifier(universal)
        assert.same({ "2", "18446744073709551615" }, universal_oid:arcs())
        local implicit = der_asn1.Decoder.new():decode_exact("\x88\x0a" .. combined)
        local implicit_oid = der_asn1.decode_implicit_object_identifier(implicit, 8)
        assert.same({ "2", "18446744073709551615" }, implicit_oid:arcs())

        local _, problem = capture(function()
            return der_asn1.Decoder.new():decode_exact("\x04\x04dead")
        end)
        assert.is_nil(tostring(problem):find("dead", 1, true))
    end)
end)
