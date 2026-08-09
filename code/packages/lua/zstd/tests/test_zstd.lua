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
-- NOTE: the local TC-9/TC-10/TC-11 labels below predate this file's
-- discovery that the CMP07 spec (code/specs/CMP07-zstd.md) reserves TC-9 for
-- a *different* test — real cross-implementation interop against the `zstd`
-- CLI — and TC-10 for a hand-built minimal wire-format frame. Renumbering
-- the existing (still valuable) local tests is out of scope here; the
-- spec's TC-9 interop test is added below as its own top-level describe
-- block, explicitly labelled "spec TC-9" to avoid confusion with the
-- differently-numbered local block above.
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

    -- =========================================================================
    -- TC-11: Trailing bytes after last block rejected
    -- =========================================================================
    --
    -- A valid ZStd frame ends exactly at the last block's payload. Any bytes
    -- remaining after that are garbage (corruption, a concatenated frame, etc.)
    -- and must be rejected rather than silently ignored.

    describe("TC-11: trailing bytes after last block rejected", function()
        it("decompress raises error when extra bytes follow the last block", function()
            local frame = "\x28\xB5\x2F\xFD"  -- magic
                       .. "\x20"               -- FHD: Single_Segment=1, FCS=1byte
                       .. "\x05"               -- FCS = 5
                       .. "\x29\x00\x00"       -- block header: last=1, raw, size=5
                       .. "hello"              -- raw payload
                       .. "\xAA\xBB\xCC"       -- 3 trailing garbage bytes

            local ok, err = pcall(function() zstd.decompress(frame) end)
            assert.is_false(ok, "expected decompress to raise an error for trailing bytes")
            assert.is_string(err)
            assert.is_truthy(
                err:find("trailing") or err:find("unexpected"),
                "error message should mention trailing data, got: " .. tostring(err))
        end)

        it("a clean frame with no trailing bytes decompresses without error", function()
            local frame = "\x28\xB5\x2F\xFD"
                       .. "\x20"
                       .. "\x05"
                       .. "\x29\x00\x00"
                       .. "hello"

            local ok, result = pcall(function() return zstd.decompress(frame) end)
            assert.is_true(ok, "clean frame should decompress without error")
            assert.are.equal("hello", result)
        end)
    end)

    -- =========================================================================
    -- Spec TC-9: Cross-language / interoperability (real `zstd` CLI)
    -- =========================================================================
    --
    -- code/specs/CMP07-zstd.md TC-9: compress with the standard `zstd` CLI,
    -- decompress with ours, AND compress with ours, decompress with the
    -- standard `zstd -d` CLI — both directions must round-trip exactly.
    --
    -- Why this matters (see lessons.md Lesson 96): a same-codebase
    -- round-trip test (compress-then-decompress with our OWN implementation)
    -- can never catch a systematic, symmetric protocol deviation, because
    -- both sides of the comparison are wrong in the identical way. Only
    -- testing against an INDEPENDENT, spec-conformant implementation can.
    -- This exact bug class (a fabricated two-pass FSE table-spread
    -- algorithm, wrong per-sequence field order, and a missing
    -- last-sequence state-update special case) passed every internal
    -- round-trip test in this package before being fixed, and was only
    -- caught by a test in this shape.
    --
    -- The test is skipped (not failed) if the `zstd` binary isn't on PATH,
    -- since CLI availability varies by environment.

    describe("Spec TC-9: cross-language interop via real zstd CLI", function()
        -- shell_ok normalises os.execute's return value across Lua versions:
        -- Lua 5.1 returns a plain exit-code number (0 = success); Lua 5.2+
        -- returns (bool_or_nil, "exit"/"signal", code).
        local function shell_ok(cmd)
            local a, _, c = os.execute(cmd)
            if type(a) == "number" then return a == 0 end
            if type(a) == "boolean" then return a end
            return c == 0
        end

        local is_windows = package.config:sub(1, 1) == "\\"

        -- shell_quote uses the native shell's argument rules. On Windows the
        -- paths come only from os.tmpname(); reject cmd.exe expansion/control
        -- characters rather than attempting a lossy escape. The remaining
        -- metacharacters are inert inside double quotes. POSIX shells use the
        -- standard single-quote escape.
        --
        -- NOTE: Lua's `%q` string.format specifier produces a *Lua
        -- source-literal* escape (safe for load()), NOT POSIX shell quoting
        -- — it does not escape `$`, backticks, or other shell-active
        -- characters inside double quotes. Every path built in this test
        -- file comes exclusively from os.tmpname() (never external or
        -- untrusted input), so this wasn't currently exploitable either
        -- way — but using real shell quoting here, rather than relying on
        -- %q, keeps that true even if this helper is ever reused with a
        -- filename or corpus derived from less trusted input.
        local function shell_quote(s)
            if is_windows then
                assert(not s:find('[%%!"\r\n]'), "unsafe Windows temp path")
                return '"' .. s .. '"'
            end
            return "'" .. s:gsub("'", "'\\''") .. "'"
        end

        local null_redirect = is_windows and ">NUL 2>&1" or ">/dev/null 2>&1"
        local stderr_redirect = is_windows and "2>NUL" or "2>/dev/null"
        local zstd_available = shell_ok("zstd --version " .. null_redirect)

        -- read_file/write_file: binary-safe whole-file I/O, used to shuttle
        -- data through the real zstd CLI via temp files (not stdin/stdout
        -- pipes, which are more awkward to keep binary-safe across
        -- io.popen's platform differences).
        local function write_file(path, data)
            local f = assert(io.open(path, "wb"))
            f:write(data)
            f:close()
        end

        local function read_file(path)
            local f = io.open(path, "rb")
            if not f then return nil end
            local data = f:read("*a")
            f:close()
            return data
        end

        -- A high-sequence-count input: semi-repetitive words from a small
        -- deterministic LCG-driven vocabulary. This exercises many LZ77
        -- matches (and therefore many FSE-coded sequences, well past a
        -- single sequence) — the exact shape of input that surfaced the
        -- three-bug FSE conformance failure described in lessons.md
        -- Lesson 96. A trivial one-or-two-sequence input is NOT sufficient:
        -- the missing last-sequence-skip bug in particular only misaligns
        -- the bitstream when there is a sequence *after* the one it
        -- mishandles.
        local function build_interop_corpus()
            local seed = 1234
            local function rnd(n)
                seed = (seed * 1103515245 + 12345) % 2147483648
                return seed % n
            end
            local words = {
                "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
            }
            local parts = {}
            for _ = 1, 3000 do
                parts[#parts + 1] = words[rnd(#words) + 1]
                parts[#parts + 1] = " "
            end
            return table.concat(parts)
        end

        it("compresses with ours, decompresses with the real zstd CLI", function()
            if not zstd_available then
                pending("zstd CLI not found on PATH; skipping interop test")
                return
            end

            local text = build_interop_corpus()
            local compressed = zstd.compress(text)

            -- Use os.tmpname()'s own paths directly (no derived/concatenated
            -- filenames): os.tmpname() atomically reserves the path it
            -- returns, but a path built by appending a suffix to it (e.g.
            -- `os.tmpname() .. ".zst"`) is never itself atomically reserved
            -- and would be vulnerable to a symlink race on a shared temp
            -- directory. The `zstd` CLI doesn't require a `.zst` extension
            -- when an explicit `-o` output path is given.
            local in_path  = os.tmpname()
            local out_path = os.tmpname()
            write_file(in_path, compressed)

            local ok = shell_ok(string.format(
                "zstd -d -f -q -o %s %s %s",
                shell_quote(out_path), shell_quote(in_path), stderr_redirect))
            local result = ok and read_file(out_path) or nil

            os.remove(in_path)
            os.remove(out_path)

            assert.is_true(ok, "real zstd CLI failed to decompress our output "
                .. "(likely FSE sequences-codec non-conformance)")
            assert.are.equal(text, result,
                "real zstd CLI decompressed our output to different bytes")
        end)

        it("compresses with the real zstd CLI, decompresses with ours", function()
            if not zstd_available then
                pending("zstd CLI not found on PATH; skipping interop test")
                return
            end

            local text = build_interop_corpus()

            local in_path  = os.tmpname()
            local out_path = os.tmpname()
            write_file(in_path, text)

            local ok = shell_ok(string.format(
                "zstd -f -q -o %s %s %s",
                shell_quote(out_path), shell_quote(in_path), stderr_redirect))
            local cli_compressed = ok and read_file(out_path) or nil

            os.remove(in_path)
            os.remove(out_path)

            assert.is_true(ok, "real zstd CLI failed to compress the input")
            assert.is_string(cli_compressed)

            -- The real zstd CLI writes a Content_Checksum by default (FHD
            -- bit 2 set — see lessons.md Lesson 95). Our decompress() must
            -- correctly locate and skip that trailing 4-byte checksum
            -- rather than rejecting it as unexpected trailing data.
            local decompressed = zstd.decompress(cli_compressed)
            assert.are.equal(text, decompressed,
                "our decompress() produced different bytes from real zstd's output")
        end)

        it("round-trips a plain English sentence through the real zstd CLI (both directions)", function()
            if not zstd_available then
                pending("zstd CLI not found on PATH; skipping interop test")
                return
            end

            -- The pangram repeated 25 times — deliberately the same corpus
            -- code/specs/CMP07-zstd.md's TC-9 example uses, and what the
            -- sibling java/zstd port's tc9CliInterop test uses.
            --
            -- NOTE: this codec's ENCODER still never emits RFC 8878's
            -- Repeat_Offset (R1/R2/R3) shortcut codes (matching the
            -- documented "no repeat-offset shortcuts" scope) — but the
            -- DECODER now fully understands them (see lessons.md Lesson 98
            -- and the dedicated "Repeated-Offset (R1/R2/R3) decode" describe
            -- block below), so inputs that make the real zstd CLI's encoder
            -- choose repeat-offset sequences are no longer a problem here.
            local text = ("the quick brown fox jumps over the lazy dog "):rep(25)

            -- ours -> CLI
            local compressed = zstd.compress(text)
            local in_path  = os.tmpname()
            local out_path = os.tmpname()
            write_file(in_path, compressed)
            local ok1 = shell_ok(string.format(
                "zstd -d -f -q -o %s %s %s",
                shell_quote(out_path), shell_quote(in_path), stderr_redirect))
            local result1 = ok1 and read_file(out_path) or nil
            os.remove(in_path)
            os.remove(out_path)
            assert.is_true(ok1, "real zstd CLI failed to decompress our output")
            assert.are.equal(text, result1)

            -- CLI -> ours
            local plain_path = os.tmpname()
            local cli_zst    = os.tmpname()
            write_file(plain_path, text)
            local ok2 = shell_ok(string.format(
                "zstd -f -q -o %s %s %s",
                shell_quote(cli_zst), shell_quote(plain_path), stderr_redirect))
            local cli_bytes = ok2 and read_file(cli_zst) or nil
            os.remove(plain_path)
            os.remove(cli_zst)
            assert.is_true(ok2, "real zstd CLI failed to compress the input")
            assert.are.equal(text, zstd.decompress(cli_bytes))
        end)

        -- =====================================================================
        -- Repeated-Offset (R1/R2/R3) decode — lessons.md Lesson 98
        -- =====================================================================
        --
        -- This codec's encoder never emits RFC 8878's Offset_Value <= 3
        -- repeat-offset shortcut codes (every offset it writes is explicit),
        -- so a round trip through ONLY this codec's own compress()/
        -- decompress() pair — and even the fixed prose corpus used by the
        -- interop tests above — can never exercise the decoder's
        -- repeat-offset path. But the real `zstd` CLI's encoder uses repeat
        -- offsets constantly (one of its principal entropy wins), so any
        -- decoder that only understands explicit offset codes will
        -- systematically fail to decode a meaningful fraction of real-world
        -- `.zst` files.
        --
        -- These tests feed the real zstd CLI's compressor inputs specifically
        -- shaped to trigger repeat-offset sequences (long constant-byte runs
        -- and content with several distinct repeating distances), then
        -- decode the CLI's actual output with our decoder — proving the gap
        -- described in lessons.md Lesson 98 (and originally found while
        -- building `c/zstd`, PR #9941) is fixed here too.
        it("decodes real zstd CLI output using a Repeated-Offset (R1) sequence "
            .. "(long constant-byte run)", function()
            if not zstd_available then
                pending("zstd CLI not found on PATH; skipping interop test")
                return
            end

            -- 4713 bytes of a single repeated byte: empirically (see
            -- lessons.md Lesson 98) the real zstd CLI encodes this as a
            -- single Compressed block containing one sequence with
            -- Offset_Value=1 ("reuse Repeated_Offset1", default value 1) —
            -- NOT the RLE block type this port's own encoder would choose
            -- for constant data. This is exactly the input that first
            -- surfaced the gap.
            local text = ("Z"):rep(4713)

            local plain_path = os.tmpname()
            local cli_zst    = os.tmpname()
            write_file(plain_path, text)
            local ok = shell_ok(string.format(
                "zstd -f -q -o %s %s %s",
                shell_quote(cli_zst), shell_quote(plain_path), stderr_redirect))
            local cli_bytes = ok and read_file(cli_zst) or nil
            os.remove(plain_path)
            os.remove(cli_zst)

            assert.is_true(ok, "real zstd CLI failed to compress the input")
            assert.is_string(cli_bytes)

            local decompressed = zstd.decompress(cli_bytes)
            assert.are.equal(text, decompressed,
                "our decompress() failed on real zstd's Repeat-Offset (R1) output "
                .. "(RFC 8878 §3.1.1.3.2.1.1 non-conformance — see lessons.md Lesson 98)")
        end)

        it("decodes real zstd CLI output using multiple distinct Repeated-Offset "
            .. "registers (R1/R2/R3 rotation across several match distances)", function()
            if not zstd_available then
                pending("zstd CLI not found on PATH; skipping interop test")
                return
            end

            -- Several distinct repeated-content regions at different
            -- distances, back to back, so the real zstd CLI's encoder has
            -- reason to populate and rotate through all three offset-history
            -- registers (not just R1) as it moves between them.
            local text = ("AAAA"):rep(50)
                .. ("the quick brown fox "):rep(80)
                .. ("BBBBBBBB"):rep(40)
                .. ("abcdefgh"):rep(60)
                .. ("AAAA"):rep(30)

            local plain_path = os.tmpname()
            local cli_zst    = os.tmpname()
            write_file(plain_path, text)
            local ok = shell_ok(string.format(
                "zstd -f -q -o %s %s %s",
                shell_quote(cli_zst), shell_quote(plain_path), stderr_redirect))
            local cli_bytes = ok and read_file(cli_zst) or nil
            os.remove(plain_path)
            os.remove(cli_zst)

            assert.is_true(ok, "real zstd CLI failed to compress the input")
            assert.is_string(cli_bytes)

            local decompressed = zstd.decompress(cli_bytes)
            assert.are.equal(text, decompressed,
                "our decompress() failed on real zstd's multi-offset Repeat-Offset output")
        end)
    end)

end)
