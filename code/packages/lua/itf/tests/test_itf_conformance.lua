package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    "../../sha256/src/?.lua;" ..
    "../../sha256/src/?/init.lua;" ..
    package.path
)

local json = require("dkjson")
local itf = require("coding_adventures.itf")
local sha256 = require("coding_adventures.sha256")

local function read_cases()
    local handle = assert(io.open("../../../../specs/fixtures/barcode-symbologies-v1/cases.json", "rb"))
    local raw = handle:read("*a")
    handle:close()
    local corpus = assert(json.decode(raw))
    local cases = {}
    for _, test_case in ipairs(corpus.cases) do
        if test_case.symbology == "itf" then
            cases[#cases + 1] = test_case
        end
    end
    return cases
end

local function materialize(input)
    if input.text ~= nil then
        return input.text
    end
    local repeat_spec = input["repeat"]
    return string.rep(repeat_spec.text, repeat_spec.count)
end

local function modules(data)
    local parts = {"1010"}
    for _, pair in ipairs(itf.encode_itf(data)) do
        parts[#parts + 1] = pair.binary_pattern
    end
    parts[#parts + 1] = "11101"
    return table.concat(parts)
end

local function runs(bits)
    local result = {}
    local previous = nil
    for index = 1, #bits do
        local bit = bits:sub(index, index)
        if bit == previous then
            result[#result] = result[#result] + 1
        else
            result[#result + 1] = 1
            previous = bit
        end
    end
    return result
end

describe("barcode-symbologies-v1 ITF corpus", function()
    it("executes all ten cases", function()
        local cases = read_cases()
        assert.equal(10, #cases)

        for _, test_case in ipairs(cases) do
            local data = materialize(test_case.input)
            local expected = test_case.expected
            if expected.error ~= nil then
                local ok, caught = pcall(itf.normalize_itf, data)
                assert.is_false(ok)
                assert.equal(expected.error, itf.error_id(caught))
            else
                local normalized = itf.normalize_itf(data)
                local encoded_modules = modules(data)
                local run_lengths = runs(encoded_modules)
                if expected.normalized ~= nil then
                    assert.equal(expected.normalized, normalized)
                    assert.equal(expected.modules, encoded_modules)
                    assert.same(expected.run_lengths, run_lengths)
                else
                    assert.equal(expected.normalized_sha256, sha256.sha256_hex(normalized))
                    assert.equal(expected.module_count, #encoded_modules)
                    assert.equal(expected.module_sha256, sha256.sha256_hex(encoded_modules))
                    assert.equal(expected.run_count, #run_lengths)
                    assert.equal(
                        expected.run_lengths_sha256,
                        sha256.sha256_hex(json.encode(run_lengths))
                    )
                end
            end
        end
    end)
end)
