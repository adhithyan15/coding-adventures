-- Tests for sha512
--
-- Validates the pure-Lua SHA-512 implementation against FIPS 180-4
-- test vectors and other well-known reference values.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local m = require("coding_adventures.sha512")

local function fields(line)
    local result, start = {}, 1
    while true do
        local stop = line:find("|", start, true)
        if not stop then result[#result + 1] = line:sub(start); return result end
        result[#result + 1] = line:sub(start, stop - 1)
        start = stop + 1
    end
end

local function lower_hex(value, length)
    return #value == length and value:match("^[0-9a-f]*$") ~= nil
end

local function parse_fixture(raw)
    assert(#raw > 0 and #raw <= 4096, "fixture byte size outside bounds")
    local lines = {}
    for line in raw:gmatch("[^\r\n]+") do
        if line:sub(1, 1) ~= "#" then lines[#lines + 1] = line end
    end
    assert(#lines > 0 and #lines <= 64, "fixture row count outside bounds")
    for _, line in ipairs(lines) do
        local row = fields(line)
        assert(#row == 5 and row[2]:match("^[a-z0-9%-]+$") and not row[2]:match("^%-")
            and not row[2]:match("%-$") and not row[2]:match("%-%-"), "malformed fixture row")
        if row[1] == "V" then
            assert(#row[3] <= 8192 and #row[3] % 2 == 0 and row[3]:match("^[0-9a-f]*$") and lower_hex(row[5], 128), "malformed vector row")
            local byte_count = #row[3] / 2
            assert(byte_count <= 4096 and ((byte_count == 0) == (row[4] == "")), "malformed vector size")
            local count, sum = 0, 0
            if row[4] ~= "" then
                assert(row[4]:match("^[0-9,]+$") and not row[4]:match("^,") and not row[4]:match(",$") and not row[4]:match(",,"), "malformed chunks")
                for raw_size in row[4]:gmatch("[^,]+") do
                    assert(raw_size:match("^[1-9][0-9]*$") and #raw_size <= 4, "malformed chunk")
                    local size = assert(tonumber(raw_size, 10))
                    assert(size <= 4096, "malformed chunk")
                    count, sum = count + 1, sum + size
                end
            end
            assert(count <= 64 and sum == byte_count, "malformed chunks")
        elseif row[1] == "L" then
            assert(lower_hex(row[3], 16) and lower_hex(row[4], 16) and (row[5] == "ERROR" or lower_hex(row[5], 32)), "malformed length row")
        else
            error("unknown fixture row tag")
        end
    end
    return lines
end

local function fixture_lines(path)
    local handle = assert(io.open(path or "../../../../specs/fixtures/sha512-v1/cases.txt", "rb"))
    local raw = handle:read(4097) or ""
    handle:close()
    local lines = parse_fixture(raw)
    local vectors, lengths = 0, 0
    for _, line in ipairs(lines) do
        if line:sub(1, 2) == "V|" then vectors = vectors + 1 else lengths = lengths + 1 end
    end
    assert.equals(9, vectors)
    assert.equals(6, lengths)
    return lines
end

local function decode_hex(value)
    assert.equals(0, #value % 2)
    return (value:gsub("..", function(pair) return string.char(tonumber(pair, 16)) end))
end

local function bytes_hex(bytes)
    local parts = {}
    for _, byte in ipairs(bytes) do parts[#parts + 1] = string.format("%02x", byte) end
    return table.concat(parts)
end

describe("sha512", function()

    it("rejects malformed fixture rows", function()
        local digest = string.rep("0", 128)
        for _, row in ipairs({"X|id|||", "V|UPPER|00|1|" .. digest, "V|bad|0|1|" .. digest,
            "V|bad|00|2|" .. digest, "L|bad|000000000000000G|0000000000000000|ERROR",
            "L|bad|0000000000000000|0000000000000000|ABC"}) do
            assert.has_error(function() parse_fixture(row) end)
        end
    end)

    it("rejects an oversized fixture through the real file reader", function()
        local path = os.tmpname()
        local handle = assert(io.open(path, "wb"))
        assert(handle:write(string.rep("x", 4097)))
        handle:close()
        local ok = pcall(fixture_lines, path)
        os.remove(path)
        assert.is_false(ok)
    end)

    it("matches the bounded HF04 portable corpus", function()
        for _, line in ipairs(fixture_lines()) do
            if line:sub(1, 2) == "V|" then
                local row = fields(line)
                local input = decode_hex(row[3])
                assert.equals(row[5], m.hex(input), row[2])
            end
        end
    end)

    it("encodes the full 128-bit length field without huge allocation", function()
        for _, line in ipairs(fixture_lines()) do
            if line:sub(1, 2) == "L|" then
                local row = fields(line)
                local w3 = assert(tonumber(row[3]:sub(1, 8), 16))
                local w2 = assert(tonumber(row[3]:sub(9, 16), 16))
                local w1 = assert(tonumber(row[4]:sub(1, 8), 16))
                local w0 = assert(tonumber(row[4]:sub(9, 16), 16))
                if row[5] == "ERROR" then
                    assert.has_error(function() m._encode_bit_length_words(w3, w2, w1, w0) end,
                        "sha512: message length must be less than 2^128 bits")
                else
                    assert.equals(row[5], bytes_hex(m._encode_bit_length_words(w3, w2, w1, w0)), row[2])
                end
            end
        end
    end)

    -- -----------------------------------------------------------------------
    -- Meta / version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    it("exposes hex function", function()
        assert.is_function(m.hex)
    end)

    it("exposes digest function", function()
        assert.is_function(m.digest)
    end)

    -- -----------------------------------------------------------------------
    -- FIPS 180-4 / well-known test vectors
    -- -----------------------------------------------------------------------

    it('sha512("") matches FIPS 180-4', function()
        assert.equals(
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce"
            .. "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
            m.hex("")
        )
    end)

    it('sha512("abc") matches FIPS 180-4', function()
        assert.equals(
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
            .. "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            m.hex("abc")
        )
    end)

    it('sha512("hello") matches known value', function()
        assert.equals(
            "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7"
            .. "2323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043",
            m.hex("hello")
        )
    end)

    it('sha512("The quick brown fox...") matches known value', function()
        assert.equals(
            "07e547d9586f6a73f73fbac0435ed76951218fb7d0c8d788a309d785436bbb64"
            .. "2e93a252a954f23912547d1e8a3b5ed6e1bfd7097821233fa0538f3db854fee6",
            m.hex("The quick brown fox jumps over the lazy dog")
        )
    end)

    it('sha512 FIPS 180-4 two-block test vector', function()
        -- "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
        assert.equals(
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018"
            .. "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
            m.hex("abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu")
        )
    end)

    it('sha512 output is always 128 hex characters', function()
        local h = m.hex("SHA512 test")
        assert.equals(128, #h)
    end)

    it("sha512 single 'a'", function()
        assert.equals(
            "1f40fc92da241694750979ee6cf582f2d5d7d28e18335de05abc54d0560e0f53"
            .. "02860c652bf08d560252aa5e74210546f369fbbbce8c12cfc7957b2652fe9a75",
            m.hex("a")
        )
    end)

    -- -----------------------------------------------------------------------
    -- digest() function -- raw byte array
    -- -----------------------------------------------------------------------

    it("digest returns 64 integers", function()
        local raw = m.digest("hello")
        assert.equals(64, #raw)
    end)

    it("digest bytes are in 0..255 range", function()
        local raw = m.digest("abc")
        for _, b in ipairs(raw) do
            assert.is_true(b >= 0 and b <= 255)
        end
    end)

    it("digest of empty string first 4 bytes match", function()
        -- cf83e135...
        local raw = m.digest("")
        assert.equals(0xcf, raw[1])
        assert.equals(0x83, raw[2])
        assert.equals(0xe1, raw[3])
        assert.equals(0x35, raw[4])
    end)

    -- -----------------------------------------------------------------------
    -- Multi-block message (> 128 bytes)
    -- -----------------------------------------------------------------------

    it("handles message longer than one 128-byte block", function()
        -- 200 'a' characters well beyond the 128-byte block boundary
        local s = string.rep("a", 200)
        local h = m.hex(s)
        assert.equals(128, #h)
        -- Verify it is a valid lowercase hex string
        assert.truthy(h:match("^[0-9a-f]+$"))
    end)

    -- -----------------------------------------------------------------------
    -- Block boundary edge cases
    -- -----------------------------------------------------------------------

    it("handles exactly 111 bytes (padding fits in same block)", function()
        -- 111 bytes + 1 (0x80) + 0 padding + 16 (length) = 128 = 1 block
        local s = string.rep("x", 111)
        local h = m.hex(s)
        assert.equals(128, #h)
    end)

    it("handles exactly 112 bytes (padding spills to next block)", function()
        -- 112 bytes + 1 (0x80) needs > 128 - 16 = 112 bytes before length
        local s = string.rep("x", 112)
        local h = m.hex(s)
        assert.equals(128, #h)
    end)

    it("handles exactly 128 bytes (exact block boundary)", function()
        local s = string.rep("x", 128)
        local h = m.hex(s)
        assert.equals(128, #h)
    end)

    -- -----------------------------------------------------------------------
    -- Error handling
    -- -----------------------------------------------------------------------

    it("errors when passed a non-string to hex()", function()
        assert.has_error(function() m.hex(42) end)
    end)

    it("errors when passed a non-string to digest()", function()
        assert.has_error(function() m.digest(nil) end)
    end)

    -- -----------------------------------------------------------------------
    -- Consistency: hex and digest agree
    -- -----------------------------------------------------------------------

    it("hex and digest are consistent", function()
        local msg = "sha512 consistency test"
        local raw = m.digest(msg)
        local hex = m.hex(msg)
        local rebuilt = ""
        for _, b in ipairs(raw) do
            rebuilt = rebuilt .. string.format("%02x", b)
        end
        assert.equals(hex, rebuilt)
    end)

    -- -----------------------------------------------------------------------
    -- Determinism
    -- -----------------------------------------------------------------------

    it("is deterministic", function()
        local h1 = m.hex("determinism test")
        local h2 = m.hex("determinism test")
        assert.equals(h1, h2)
    end)

    -- -----------------------------------------------------------------------
    -- Avalanche effect
    -- -----------------------------------------------------------------------

    it("one-bit difference changes the output", function()
        local h1 = m.hex("a")
        local h2 = m.hex("b")
        assert.not_equals(h1, h2)
    end)

end)
