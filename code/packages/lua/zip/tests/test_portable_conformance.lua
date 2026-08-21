-- Language-neutral ZIP-owned raw RFC 1951 conformance.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local json = require("dkjson")
local LibDeflate = require("LibDeflate")
local zip = require("coding_adventures.zip")

local EXPECTED_ERROR_IDS = {
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
}

local function read_fixture()
    local relative = "specs/fixtures/zip-raw-rfc1951-v1/cases.json"
    local candidates = {
        "../../../../" .. relative,
        "../../../" .. relative,
    }
    for _, path in ipairs(candidates) do
        local handle = io.open(path, "rb")
        if handle ~= nil then
            local contents = handle:read("*a")
            handle:close()
            local decoded, _, decode_error = json.decode(contents)
            assert.is_nil(decode_error)
            return decoded
        end
    end
    error("unable to locate ZIP raw RFC 1951 fixture")
end

local FIXTURE = read_fixture()

local function hex_to_bytes(value)
    return (value:gsub("..", function(pair)
        return string.char(tonumber(pair, 16))
    end))
end

local function expected_bytes(specification)
    if specification.hex ~= nil then
        return hex_to_bytes(specification.hex)
    end
    return string.rep(hex_to_bytes(specification.repeat_hex), specification.count)
end

local function find_case(id)
    for _, case in ipairs(FIXTURE.cases) do
        if case.id == id then
            return case
        end
    end
    error("missing fixture case " .. id)
end

local function le16(value)
    return string.char(value & 0xff, (value >> 8) & 0xff)
end

local function le32(value)
    return string.char(
        value & 0xff,
        (value >> 8) & 0xff,
        (value >> 16) & 0xff,
        (value >> 24) & 0xff
    )
end

local function raw_zip(name, compressed, plain, declared_size)
    declared_size = declared_size or #plain
    local crc = zip.crc32(plain)
    local local_header = table.concat({
        le32(0x04034b50), le16(20), le16(0x0800), le16(8), le16(0), le16(0),
        le32(crc), le32(#compressed), le32(declared_size), le16(#name), le16(0),
        name, compressed,
    })
    local central = table.concat({
        le32(0x02014b50), le16(0x031e), le16(20), le16(0x0800), le16(8),
        le16(0), le16(0), le32(crc), le32(#compressed), le32(declared_size),
        le16(#name), le16(0), le16(0), le16(0), le16(0), le32(0), le32(0), name,
    })
    local eocd = table.concat({
        le32(0x06054b50), le16(0), le16(0), le16(1), le16(1),
        le32(#central), le32(#local_header), le16(0),
    })
    return local_header .. central .. eocd
end

describe("ZIP raw RFC 1951 v1 metadata", function()
    it("pins the closed profile", function()
        assert.equal(1, FIXTURE.schema_version)
        assert.equal("zip-owned-raw-rfc1951-v1", FIXTURE.profile)
        assert.equal(268435456, FIXTURE.limits.default_max_output)
        assert.equal(268435456, FIXTURE.limits.hard_max_output)
        assert.equal(34, #FIXTURE.cases)
        assert.same(EXPECTED_ERROR_IDS, FIXTURE.error_ids)
        assert.equal(268435456, zip.RAW_INFLATE_MAX_OUTPUT)
        assert.same(EXPECTED_ERROR_IDS, zip.RAW_INFLATE_ERROR_CODES)
    end)
end)

describe("ZIP raw RFC 1951 v1 cases", function()
    for _, case in ipairs(FIXTURE.cases) do
        if case.operation == "inflate" then
            it(case.id, function()
                local input = hex_to_bytes(case.input_hex)
                local limit = case.max_output or zip.RAW_INFLATE_MAX_OUTPUT
                local result, decode_error = zip.raw_inflate_counted(input, limit)
                assert.is_nil(decode_error)
                assert.is_table(result)
                assert.equal(expected_bytes(case.expected.output), result.output)
                assert.equal(case.expected.bytes_consumed, result.bytes_consumed)
                assert.equal(result.output, assert(zip.raw_inflate(input, limit)))
            end)
        elseif case.operation == "inflate-error" then
            it(case.id, function()
                local input = hex_to_bytes(case.input_hex)
                local limit = case.max_output
                local result, error_id
                if limit == nil then
                    result, error_id = zip.raw_inflate_counted(input)
                else
                    result, error_id = zip.raw_inflate_counted(input, limit)
                end
                assert.is_nil(result)
                assert.equal(case.expected.error_id, error_id)
            end)
        elseif case.operation == "deflate-interoperability" then
            it(case.id, function()
                local input = hex_to_bytes(case.input_hex)
                local encoded = zip.raw_deflate(input)
                local decoded, trailing = LibDeflate:DecompressDeflate(encoded)
                assert.equal(expected_bytes(case.expected.output), decoded)
                assert.equal(0, trailing)
            end)
        elseif case.operation == "crc32" then
            it(case.id, function()
                local checksum = tonumber(case.initial_crc32_hex or "00000000", 16)
                for _, chunk in ipairs(case.chunks_hex) do
                    checksum = zip.crc32(hex_to_bytes(chunk), checksum)
                end
                assert.equal(tonumber(case.expected.crc32_hex, 16), checksum)
            end)
        else
            error("unsupported fixture operation " .. tostring(case.operation))
        end
    end
end)

describe("raw RFC 1951 integration boundaries", function()
    it("inflates a multi-megabyte overlapping stream through bounded storage #slow", function()
        local seed = ("portable-window-"):rep(2048)
        local expected = seed:rep(64)
        local stream = LibDeflate:CompressDeflate(expected, {level = 9})
        local result = assert(zip.raw_inflate_counted(stream, #expected))
        assert.equal(expected, result.output)
        assert.equal(#stream, result.bytes_consumed)
    end)

    it("decodes an independently generated full 32 KiB window stream #slow", function()
        local prefix = {}
        for index = 0, 32767 do
            prefix[#prefix + 1] = string.char(((index * 73) + (index // 251)) & 0xff)
        end
        local half = table.concat(prefix)
        local expected = half .. half
        local stream = LibDeflate:CompressDeflate(expected, {level = 9})
        assert.equal(expected, assert(zip.raw_inflate(stream, #expected)))
    end)

    it("reads a dynamic-Huffman ZIP entry", function()
        local case = find_case("zip-raw-v1-inflate-dynamic-foreign")
        local compressed = hex_to_bytes(case.input_hex)
        local plain = expected_bytes(case.expected.output)
        local reader = assert(zip.new_reader(raw_zip("dynamic.bin", compressed, plain)))
        assert.equal(plain, assert(zip.reader_read(reader, zip.reader_entries(reader)[1])))
    end)

    it("rejects a compressed suffix cavity", function()
        local case = find_case("zip-raw-v1-inflate-dynamic-foreign")
        local compressed = hex_to_bytes(case.input_hex) .. "\xde\xad"
        local plain = expected_bytes(case.expected.output)
        local reader = assert(zip.new_reader(raw_zip("suffix.bin", compressed, plain)))
        local output, read_error = zip.reader_read(reader, zip.reader_entries(reader)[1])
        assert.is_nil(output)
        assert.equal("zip: compressed payload contains trailing bytes", read_error)
    end)

    it("rejects a declared uncompressed-size mismatch", function()
        local case = find_case("zip-raw-v1-inflate-dynamic-foreign")
        local compressed = hex_to_bytes(case.input_hex)
        local plain = expected_bytes(case.expected.output)
        local reader = assert(zip.new_reader(raw_zip("size.bin", compressed, plain, #plain + 1)))
        local output, read_error = zip.reader_read(reader, zip.reader_entries(reader)[1])
        assert.is_nil(output)
        assert.equal("zip: uncompressed size does not match the directory", read_error)
    end)

    it("rejects non-integral output limits before decoding", function()
        local stream = hex_to_bytes("010000ffff")
        for _, limit in ipairs({-1, 1.5, zip.RAW_INFLATE_MAX_OUTPUT + 1}) do
            local output, error_id = zip.raw_inflate_counted(stream, limit)
            assert.is_nil(output)
            assert.equal("invalid-output-limit", error_id)
        end
    end)

    it("preserves the historical writer and reader API", function()
        local plain = string.rep("compatibility ", 128)
        local archive = zip.zip({{"compat.txt", plain}}, true)
        assert.equal(plain, zip.unzip(archive)["compat.txt"])
    end)
end)
