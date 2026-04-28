-- ============================================================================
-- Tests for coding_adventures.zstd — CMP07 ZStd compression
-- ============================================================================
--
-- Covers TC-1 through TC-10 from the CMP07 specification:
--   TC-1  Empty input round-trip
--   TC-2  Single-byte round-trip
--   TC-3  All 256 byte values round-trip
--   TC-4  RLE block (1024 identical bytes)
--   TC-5  English prose compression ratio
--   TC-6  LCG pseudo-random data round-trip
--   TC-7  200 KB single-byte run
--   TC-8  300 KB repetitive text
--   TC-9  Bad magic number rejected
--   TC-10 Truncated input (magic only, no FHD) rejected
--
-- Both compress() and decompress() accept Lua strings and return Lua strings.
-- LUA_PATH must include both the zstd and lzss src trees; see BUILD.
--
-- Framework: Busted (https://olivinelabs.com/busted/)

describe("CodingAdventures.Zstd", function()
    -- -------------------------------------------------------------------------
    -- Module load
    -- -------------------------------------------------------------------------
    -- The LUA_PATH set by the BUILD script exposes both src trees:
    --   "../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;;"
    -- We also extend package.path here so the file is runnable directly with
    --   lua test_zstd.lua  (useful for interactive debugging).

    package.path = "../src/?.lua;../src/?/init.lua;"
                .. "../../lzss/src/?.lua;../../lzss/src/?/init.lua;"
                .. package.path

    local zstd = require("coding_adventures.zstd")

    -- =========================================================================
    -- TC-1: Empty input round-trip
    -- =========================================================================
    --
    -- Compressing the empty string must produce a valid ZStd frame that
    -- decompresses back to the empty string.  Internally the compressor emits
    -- one empty Raw block (size = 0).
    --
    -- Why this matters: many callers guard against nil but not ""; the boundary
    -- case must work without error.

    describe("TC-1: empty input round-trip", function()
        it("compress('') then decompress gives ''", function()
            local compressed   = zstd.compress("")
            -- A valid frame must be a string with at least the 4-byte magic,
            -- 1-byte FHD, 8-byte FCS, and 3-byte empty block header = 16 bytes.
            assert.is_string(compressed)
            assert.is_true(#compressed >= 16,
                "compressed empty should still produce a frame (" .. #compressed .. " bytes)")

            local decompressed = zstd.decompress(compressed)
            assert.are.equal("", decompressed)
        end)
    end)

    -- =========================================================================
    -- TC-2: Single-byte round-trip
    -- =========================================================================
    --
    -- A single byte with value 0x42 ('B') must survive compression and
    -- decompression unchanged.  Because there are no repeating substrings (the
    -- block is only 1 byte long), LZSS produces a single Literal token, and the
    -- block should be stored as a Raw block.

    describe("TC-2: single-byte round-trip", function()
        it("compress and decompress '\\x42'", function()
            local original     = "\x42"
            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)
    end)

    -- =========================================================================
    -- TC-3: All 256 byte values round-trip
    -- =========================================================================
    --
    -- Build a 256-byte string containing every possible byte value (0x00–0xFF)
    -- in order.  This exercises the full byte range and confirms that no byte
    -- value is mistakenly transformed or dropped.
    --
    -- Construction: string.char(0) .. string.char(1) .. ... .. string.char(255)

    describe("TC-3: all 256 byte values", function()
        it("round-trips a string containing every byte 0x00–0xFF", function()
            local parts = {}
            for i = 0, 255 do
                parts[i + 1] = string.char(i)
            end
            local original     = table.concat(parts)
            assert.are.equal(256, #original)

            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)
    end)

    -- =========================================================================
    -- TC-4: RLE block (1024 identical bytes)
    -- =========================================================================
    --
    -- 1024 bytes all equal to 'A' (0x41) must:
    --   (a) round-trip correctly, AND
    --   (b) compress to fewer than 30 bytes.
    --
    -- ZStd's RLE block type stores the repeated byte once and records the
    -- count in the block header — 3-byte header + 1-byte payload = 4 bytes total
    -- for the block, plus the 13-byte frame header = 17 bytes.  The 30-byte
    -- budget gives generous headroom for any frame-level overhead.

    describe("TC-4: RLE block (1024 'A' bytes)", function()
        local original = string.rep("A", 1024)

        it("round-trips 1024 identical bytes", function()
            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)

        it("compressed size is less than 30 bytes", function()
            local compressed = zstd.compress(original)
            assert.is_true(
                #compressed < 30,
                string.format("expected < 30 bytes, got %d", #compressed))
        end)
    end)

    -- =========================================================================
    -- TC-5: English prose compression ratio
    -- =========================================================================
    --
    -- The pangram "the quick brown fox jumps over the lazy dog " repeated 25
    -- times yields 1125 bytes of natural-language text with high repetition.
    -- ZStd (LZ77 + FSE) should compress this well below 80 % of the original.
    --
    -- Ratio check: compressed_length < 0.80 * original_length
    --              = 0.80 * 1125 = 900 bytes.

    describe("TC-5: English prose compression ratio", function()
        local original = ("the quick brown fox jumps over the lazy dog "):rep(25)

        it("round-trips repeated pangram", function()
            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)

        it("compressed length is less than 80% of original", function()
            local compressed = zstd.compress(original)
            local limit      = math.floor(0.80 * #original)
            assert.is_true(
                #compressed < limit,
                string.format("expected < %d bytes (80%% of %d), got %d",
                    limit, #original, #compressed))
        end)
    end)

    -- =========================================================================
    -- TC-6: LCG pseudo-random data round-trip
    -- =========================================================================
    --
    -- Collect 512 bytes from a Linear Congruential Generator (LCG) with seed 42.
    -- LCG update: seed = (seed * 1664525 + 1013904223) % 2^32
    -- Take the lowest byte of each state value.
    --
    -- Random-looking data should still round-trip losslessly, though compression
    -- will be minimal (the encoder will fall back to Raw blocks).

    describe("TC-6: LCG pseudo-random round-trip", function()
        it("compresses and decompresses 512 pseudo-random bytes correctly", function()
            -- Generate 512 bytes from the LCG.
            local seed  = 42
            local parts = {}
            for i = 1, 512 do
                seed      = (seed * 1664525 + 1013904223) % (2^32)
                parts[i]  = string.char(seed & 0xFF)
            end
            local original     = table.concat(parts)
            assert.are.equal(512, #original)

            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)
    end)

    -- =========================================================================
    -- TC-7: 200 KB single-byte run
    -- =========================================================================
    --
    -- string.rep("\xAB", 200*1024) = 204800 identical bytes.
    -- Must:
    --   (a) round-trip correctly, AND
    --   (b) compress to fewer than 100 bytes.
    --
    -- Each 128 KB chunk is encoded as a single RLE block (4 bytes).  Two chunks
    -- → 2 × 4 = 8 block bytes, plus the 13-byte frame header = 21 bytes total.
    -- The 100-byte limit is generous.

    describe("TC-7: 200 KB single-byte run", function()
        local original = string.rep("\xAB", 200 * 1024)

        it("round-trips 200 KB of byte 0xAB", function()
            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)

        it("compressed size is less than 100 bytes", function()
            local compressed = zstd.compress(original)
            assert.is_true(
                #compressed < 100,
                string.format("expected < 100 bytes for 200 KB RLE, got %d", #compressed))
        end)
    end)

    -- =========================================================================
    -- TC-8: 300 KB repetitive text
    -- =========================================================================
    --
    -- string.rep("hello world and more text for compression testing!\n", 6000)
    -- = 6000 × 51 bytes = 306000 bytes.
    --
    -- This exceeds the 128 KB MAX_BLOCK_SIZE, so the data is split into at
    -- least three Compressed blocks.  The test confirms multi-block assembly
    -- and that the output is reassembled correctly.

    describe("TC-8: 300 KB repetitive text (multi-block)", function()
        local original = string.rep("hello world and more text for compression testing!\n", 6000)

        it("round-trips 300 KB of repetitive text", function()
            local compressed   = zstd.compress(original)
            local decompressed = zstd.decompress(compressed)
            assert.are.equal(original, decompressed)
        end)

        it("compressed is significantly smaller than original", function()
            local compressed = zstd.compress(original)
            -- Even conservative: must be less than 10% of original.
            local limit = math.floor(0.10 * #original)
            assert.is_true(
                #compressed < limit,
                string.format("expected < %d bytes (10%% of %d), got %d",
                    limit, #original, #compressed))
        end)
    end)

    -- =========================================================================
    -- TC-9: Bad magic number rejected
    -- =========================================================================
    --
    -- A frame that begins with 0x00 0x00 0x00 0x00 does not have the ZStd magic
    -- number (0x28 0xB5 0x2F 0xFD).  decompress() must raise an error.
    --
    -- We use pcall to catch the error without aborting the test suite.

    describe("TC-9: bad magic number rejected", function()
        it("decompress raises error for a frame with wrong magic", function()
            -- 12 bytes: wrong 4-byte magic + FHD + FCS + minimal block header.
            -- We give enough bytes so the frame-too-short check is not triggered first.
            local bad = "\x00\x00\x00\x00\xE0\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00"
            local ok, err = pcall(function() zstd.decompress(bad) end)
            assert.is_false(ok, "expected decompress to raise an error for bad magic")
            -- The error message should mention "magic" or contain the hex value.
            assert.is_string(err)
            assert.is_truthy(
                err:find("magic") or err:find("0x00000000"),
                "error message should mention bad magic, got: " .. tostring(err))
        end)
    end)

    -- =========================================================================
    -- TC-10: Truncated input (magic only, no FHD)
    -- =========================================================================
    --
    -- "\x28\xB5\x2F\xFD" is exactly the ZStd magic number but nothing else.
    -- The frame header descriptor (FHD) is missing, so the decoder must raise
    -- an error rather than reading past the end of the buffer.

    describe("TC-10: truncated input (magic only)", function()
        it("decompress raises error for a frame with only the magic bytes", function()
            local magic_only = "\x28\xB5\x2F\xFD"
            local ok, err = pcall(function() zstd.decompress(magic_only) end)
            assert.is_false(ok, "expected decompress to raise an error for truncated input")
            assert.is_string(err)
            -- The error message should indicate the frame is too short or truncated.
            assert.is_truthy(
                err:find("short") or err:find("truncat") or err:find("block"),
                "error message should mention truncation, got: " .. tostring(err))
        end)
    end)

end)
