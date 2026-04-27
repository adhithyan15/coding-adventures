-- ============================================================================
-- CodingAdventures.LZW
-- ============================================================================
--
-- LZW lossless compression algorithm (Welch, 1984).
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is LZW?
-- ------------
--
-- LZW is LZ78 with a crucial optimisation: the dictionary is pre-seeded with
-- all 256 single-byte entries before encoding begins. This eliminates LZ78's
-- mandatory `next_char` byte that followed every token — because every possible
-- byte is already in the dictionary from the start, the encoder never needs to
-- transmit raw literals alongside codes.
--
-- Pre-seeding shifts all the savings to the decoder side: it reconstructs the
-- same dictionary in lockstep with the encoder, so no dictionary is transmitted
-- over the wire at all. This is the same trick GIF uses.
--
-- Reserved Codes
-- --------------
--
--   0–255:   Pre-seeded single-byte entries (byte b → code b).
--   256:     CLEAR_CODE — tells the decoder to reset dictionary + code size.
--   257:     STOP_CODE  — marks the end of the code stream.
--   258+:    Dynamically assigned as new sequences are discovered.
--
-- Variable-Width Bit Packing
-- --------------------------
--
-- Since codes only go up to 255 initially, 8 bits would suffice — but the
-- dictionary grows past 255 immediately. LZW therefore starts codes at 9 bits
-- and widens the code size whenever the next code would overflow the current
-- bit width:
--
--   Codes 0–511    → 9 bits  (INITIAL_CODE_SIZE = 9)
--   Codes 512–1023 → 10 bits
--   ...
--   Codes 32768–65535 → 16 bits  (MAX_CODE_SIZE = 16)
--
-- Widening happens when next_code crosses the power-of-two boundary. Bits are
-- packed LSB-first inside each byte, which is the GIF / Unix compress convention.
--
-- The Tricky Token
-- ----------------
--
-- The encoder may emit a code C before the decoder has added that entry to its
-- dictionary. This happens in the pattern xyx...x, where the encoder's prefix
-- becomes [x,y,x,...,x] and the code emitted equals next_dict_slot.
--
-- The decoder can still recover: since the new entry must start with the same
-- byte as the previous entry, the decoder constructs:
--
--   entry = dict[prev_code] .. string.char(dict[prev_code][1])
--
-- — that is, the previous entry with its own first byte repeated at the end.
--
-- Wire Format (CMP03)
-- -------------------
--
--   Bytes 0–3:  original_length  (big-endian uint32)
--   Bytes 4+:   bit-packed variable-width codes, LSB-first
--
-- No block_count header is needed — the STOP_CODE terminates the stream.
--
-- The Series: CMP00 → CMP05
-- --------------------------
--
-- CMP00 (LZ77,     1977) — Sliding-window backreferences.
-- CMP01 (LZ78,     1978) — Explicit dictionary (trie).
-- CMP02 (LZSS,     1982) — LZ77 + flag bits; eliminates wasted literals.
-- CMP03 (LZW,      1984) — Pre-initialised dictionary; powers GIF. This module.
-- CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
-- CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ── Constants ─────────────────────────────────────────────────────────────────

-- CLEAR_CODE instructs the decoder to reset its dictionary and code size.
local CLEAR_CODE = 256

-- STOP_CODE marks the end of the compressed code stream.
local STOP_CODE = 257

-- INITIAL_NEXT_CODE is the first dynamically assigned dictionary code.
local INITIAL_NEXT_CODE = 258

-- INITIAL_CODE_SIZE is the starting bit-width for codes (covers 0–511).
local INITIAL_CODE_SIZE = 9

-- MAX_CODE_SIZE is the maximum bit-width; dictionary caps at 65536 entries.
local MAX_CODE_SIZE = 16

-- Expose constants on the module for testing and documentation.
M.CLEAR_CODE         = CLEAR_CODE
M.STOP_CODE          = STOP_CODE
M.INITIAL_NEXT_CODE  = INITIAL_NEXT_CODE
M.INITIAL_CODE_SIZE  = INITIAL_CODE_SIZE
M.MAX_CODE_SIZE      = MAX_CODE_SIZE

-- ── Bit I/O ───────────────────────────────────────────────────────────────────

-- new_bit_writer creates a stateful bit-writer that accumulates variable-width
-- codes into a list of byte values, LSB-first.
--
-- LSB-first packing: each code's least-significant bit is placed into the
-- lowest available bit position of the current accumulator byte. When the
-- accumulator reaches 8 bits it is flushed as a byte.
--
-- Example: writing code 0b110100101 (9 bits = 421) into an empty writer
-- fills buffer bits 0–8 with 1,0,1,0,0,1,0,1,1 (LSB first). After 8 bits
-- are consumed, byte 0b10100101 = 165 is emitted; bit 0 of the next byte is
-- the 9th input bit.
--
-- @return table with :write(code, code_size) and :flush() methods.
local function new_bit_writer()
    local bw = {buffer = 0, bit_pos = 0, bytes = {}}

    -- write appends `code_size` bits of `code` to the output, LSB-first.
    --
    -- @param code      (integer) the code value to write.
    -- @param code_size (integer) number of bits to write.
    function bw:write(code, code_size)
        -- Place code into the accumulator starting at the current bit position.
        self.buffer = self.buffer | (code << self.bit_pos)
        self.bit_pos = self.bit_pos + code_size

        -- Flush complete bytes.
        while self.bit_pos >= 8 do
            self.bytes[#self.bytes + 1] = self.buffer & 0xFF
            self.buffer = self.buffer >> 8
            self.bit_pos = self.bit_pos - 8
        end
    end

    -- flush emits any remaining partial byte.  Must be called once after all
    -- codes have been written.
    function bw:flush()
        if self.bit_pos > 0 then
            self.bytes[#self.bytes + 1] = self.buffer & 0xFF
            self.buffer = 0
            self.bit_pos = 0
        end
    end

    return bw
end

-- new_bit_reader creates a stateful bit-reader that extracts variable-width
-- codes from a string, LSB-first (matching the writer above).
--
-- @param data (string) the raw bit-packed bytes to read from.
-- @return table with :read(code_size) → integer method.
local function new_bit_reader(data)
    local br = {data = data, pos = 1, buffer = 0, bit_pos = 0}

    -- read extracts the next `code_size`-bit code.
    -- Returns nil when the stream is exhausted before enough bits are available.
    --
    -- @param code_size (integer) number of bits to read.
    -- @return (integer|nil) decoded code, or nil on EOF.
    function br:read(code_size)
        -- Refill the accumulator until we have enough bits.
        while self.bit_pos < code_size do
            if self.pos > #self.data then
                return nil  -- stream exhausted
            end
            -- Load the next byte into the high end of the accumulator.
            self.buffer = self.buffer | (self.data:byte(self.pos) << self.bit_pos)
            self.pos = self.pos + 1
            self.bit_pos = self.bit_pos + 8
        end

        -- Extract the lowest code_size bits.
        local mask = (1 << code_size) - 1
        local code = self.buffer & mask
        self.buffer = self.buffer >> code_size
        self.bit_pos = self.bit_pos - code_size
        return code
    end

    -- exhausted returns true when no more bits remain.
    function br:exhausted()
        return self.pos > #self.data and self.bit_pos == 0
    end

    return br
end

-- ── Encoder ───────────────────────────────────────────────────────────────────

-- encode_codes encodes a byte array into a list of LZW codes.
--
-- Algorithm outline:
--
--   1. Seed the encode dictionary with all 256 single-byte sequences.
--   2. Emit CLEAR_CODE to tell the decoder to reset.
--   3. Maintain a working prefix `w` (initially empty).
--   4. For each byte b in the input:
--        - If w..b exists in the dictionary → extend: w = w..b
--        - Otherwise:
--            * Emit the code for w.
--            * Add w..b to the dictionary (if space remains).
--            * If the dictionary is full, emit CLEAR_CODE and reset.
--            * Reset w = {b}  (start a new prefix with just b).
--   5. After all bytes, emit the code for the remaining prefix w.
--   6. Emit STOP_CODE.
--
-- Why reset on full dictionary?
-- When next_code reaches 2^MAX_CODE_SIZE (65536) the dictionary is full. Rather
-- than stopping compression improvement, we emit CLEAR_CODE and start fresh.
-- The decoder follows the same logic: on CLEAR_CODE it resets its dictionary.
--
-- @param data (table) 1-indexed integer byte array (each entry 0–255).
-- @return (table) 1-indexed array of integer codes.
local function encode_codes(data)
    -- Seed the encode dictionary: single-byte string → integer code.
    -- We use Lua strings as keys because Lua tables require hashable keys;
    -- string.char(b) is the compact 1-byte key for byte value b.
    local enc_dict = {}
    for b = 0, 255 do
        enc_dict[string.char(b)] = b
    end

    local next_code   = INITIAL_NEXT_CODE
    local max_entries = 1 << MAX_CODE_SIZE  -- 65536

    local codes = {}
    codes[#codes + 1] = CLEAR_CODE  -- always begin with a CLEAR

    -- w_str is the current working prefix as a Lua string (for dict lookup).
    -- w_bytes is the same prefix as a byte array (needed for building new keys).
    local w_str   = ""
    local w_bytes = {}

    for _, b in ipairs(data) do
        local wb_str = w_str .. string.char(b)

        if enc_dict[wb_str] then
            -- Prefix + b is already known — extend the working prefix.
            w_str = wb_str
            w_bytes[#w_bytes + 1] = b
        else
            -- Emit code for current prefix.
            codes[#codes + 1] = enc_dict[w_str]

            if next_code < max_entries then
                -- Add the new sequence to the dictionary.
                enc_dict[wb_str] = next_code
                next_code = next_code + 1
            elseif next_code == max_entries then
                -- Dictionary full — reset both sides.
                codes[#codes + 1] = CLEAR_CODE
                enc_dict = {}
                for i = 0, 255 do
                    enc_dict[string.char(i)] = i
                end
                next_code = INITIAL_NEXT_CODE
            end

            -- Start a new prefix with just the current byte.
            w_str   = string.char(b)
            w_bytes = {b}
        end
    end

    -- Flush the remaining prefix (if any input was provided).
    if #w_bytes > 0 then
        codes[#codes + 1] = enc_dict[w_str]
    end

    codes[#codes + 1] = STOP_CODE
    return codes
end

-- ── Decoder ───────────────────────────────────────────────────────────────────

-- decode_codes decodes a list of LZW codes back to a byte array.
--
-- Algorithm outline:
--
--   1. Seed the decode dictionary with all 256 single-byte byte-arrays.
--      Slots 257 (CLEAR_CODE+1) and 258 (STOP_CODE+1) are left as placeholders.
--   2. On CLEAR_CODE: reset dictionary + next_code, clear prev_code.
--   3. On STOP_CODE: stop.
--   4. For each data code C:
--        a. Look up `entry = dict[C+1]` (1-indexed Lua array).
--        b. If C is not yet in the dictionary (the "tricky token"):
--              entry = dict[prev_code+1] with its first byte appended.
--        c. Append entry to output.
--        d. If prev_code is valid, add `dict[prev_code+1] .. {entry[1]}`
--           to the dictionary.
--        e. Set prev_code = C.
--
-- The tricky token arises in inputs like "ABABABA..." where the encoder emits
-- a code for a sequence the decoder hasn't added yet. The invariant is: that
-- code's entry always begins and ends with the same byte as `dict[prev_code+1]`,
-- so we can construct it from what we already know.
--
-- @param codes (table) 1-indexed array of integer codes from encode_codes().
-- @return (table) 1-indexed integer byte array.
local function decode_codes(codes)
    -- Initialise decode dictionary: 1-indexed Lua array of byte-arrays.
    -- dec_dict[code + 1] = {byte, byte, ...}
    -- We use code+1 as the Lua index throughout because Lua arrays are 1-indexed
    -- while LZW codes start at 0.
    local dec_dict = {}
    for b = 0, 255 do
        dec_dict[b + 1] = {b}
    end
    -- Slots 257 and 258 (CLEAR_CODE+1 and STOP_CODE+1) are placeholders.
    dec_dict[CLEAR_CODE + 1] = nil
    dec_dict[STOP_CODE  + 1] = nil

    local next_code = INITIAL_NEXT_CODE

    local output    = {}
    local NO_PREV   = -1   -- sentinel: no previous code yet
    local prev_code = NO_PREV

    for _, code in ipairs(codes) do
        if code == CLEAR_CODE then
            -- Reset the dictionary and state.
            dec_dict = {}
            for b = 0, 255 do
                dec_dict[b + 1] = {b}
            end
            dec_dict[CLEAR_CODE + 1] = nil
            dec_dict[STOP_CODE  + 1] = nil
            next_code = INITIAL_NEXT_CODE
            prev_code = NO_PREV

        elseif code == STOP_CODE then
            break

        else
            -- Determine the entry for this code.
            local entry

            if dec_dict[code + 1] then
                -- Normal case: code is already in the dictionary.
                entry = dec_dict[code + 1]

            elseif code == next_code then
                -- Tricky token: the decoder hasn't added this entry yet.
                -- The entry must be dict[prev_code] with its own first byte
                -- repeated at the end. This is provably correct because the
                -- encoder emitted this code for the sequence prev..first(prev).
                if prev_code == NO_PREV then
                    -- Malformed stream — skip.
                    goto continue
                end
                local prev_entry = dec_dict[prev_code + 1]
                entry = {}
                for i = 1, #prev_entry do
                    entry[i] = prev_entry[i]
                end
                entry[#entry + 1] = prev_entry[1]

            else
                -- Invalid code — skip.
                goto continue
            end

            -- Append entry bytes to the output.
            for _, byte_val in ipairs(entry) do
                output[#output + 1] = byte_val
            end

            -- Add a new dictionary entry: dict[prev_code] .. {entry[1]}.
            if prev_code ~= NO_PREV and next_code < (1 << MAX_CODE_SIZE) then
                local prev_entry = dec_dict[prev_code + 1]
                local new_entry  = {}
                for i = 1, #prev_entry do
                    new_entry[i] = prev_entry[i]
                end
                new_entry[#new_entry + 1] = entry[1]
                dec_dict[next_code + 1] = new_entry
                next_code = next_code + 1
            end

            prev_code = code
        end

        ::continue::
    end

    return output
end

-- ── Serialisation ─────────────────────────────────────────────────────────────

-- write_uint32_be writes `n` as a big-endian uint32 into the `parts` list.
--
-- Big-endian means the most-significant byte comes first. For n = 0x01020304:
--   byte 0 = 0x01, byte 1 = 0x02, byte 2 = 0x03, byte 3 = 0x04
--
-- We use integer arithmetic (floor + modulo) to be safe across Lua versions
-- and to avoid floating-point issues with large values.
--
-- @param parts (table) list of strings to append to.
-- @param n     (integer) value 0–4294967295.
local function write_uint32_be(parts, n)
    parts[#parts + 1] = string.char(
        math.floor(n / 16777216) % 256,  -- byte 3 (most significant)
        math.floor(n / 65536)    % 256,  -- byte 2
        math.floor(n / 256)      % 256,  -- byte 1
        n                        % 256   -- byte 0 (least significant)
    )
end

-- read_uint32_be reads a big-endian uint32 from string `data` at position `pos`.
--
-- @param data (string) binary string.
-- @param pos  (integer) 1-indexed starting position.
-- @return (integer, integer) the value and the position after the 4 bytes.
local function read_uint32_be(data, pos)
    local b1, b2, b3, b4 = data:byte(pos, pos + 3)
    return b1 * 16777216 + b2 * 65536 + b3 * 256 + b4, pos + 4
end

-- pack_codes packs a list of LZW codes into the CMP03 wire format.
--
-- The code size starts at INITIAL_CODE_SIZE (9 bits) and grows when `next_code`
-- crosses the next power-of-two boundary. CLEAR_CODE resets the code size back
-- to 9 bits.
--
-- Both the encoder (here) and decoder track `next_code` independently and
-- increment it for every DATA code (not CLEAR, not STOP). This keeps both sides
-- in lockstep for code-size bumping — no explicit code-size field is needed in
-- the wire format.
--
-- @param codes           (table) 1-indexed array of integer codes.
-- @param original_length (integer) byte length of the original uncompressed data.
-- @return (string) binary string in CMP03 wire format.
local function pack_codes(codes, original_length)
    local bw        = new_bit_writer()
    local code_size = INITIAL_CODE_SIZE
    local next_code = INITIAL_NEXT_CODE

    for _, code in ipairs(codes) do
        bw:write(code, code_size)

        if code == CLEAR_CODE then
            -- After a CLEAR_CODE both sides reset.
            code_size = INITIAL_CODE_SIZE
            next_code = INITIAL_NEXT_CODE

        elseif code ~= STOP_CODE then
            -- Data code: increment next_code and possibly widen code_size.
            if next_code < (1 << MAX_CODE_SIZE) then
                next_code = next_code + 1
                if next_code > (1 << code_size) and code_size < MAX_CODE_SIZE then
                    code_size = code_size + 1
                end
            end
        end
    end

    bw:flush()

    -- Prepend the 4-byte big-endian original_length header.
    -- Convert bw.bytes (a table of integers) to a string in 256-element chunks
    -- to avoid hitting Lua's C stack limit when table.unpack is called on very
    -- large arrays.
    local parts = {}
    write_uint32_be(parts, original_length)

    local byte_count = #bw.bytes
    local CHUNK = 256
    local i = 1
    while i <= byte_count do
        local j = math.min(i + CHUNK - 1, byte_count)
        parts[#parts + 1] = string.char(table.unpack(bw.bytes, i, j))
        i = j + 1
    end

    return table.concat(parts)
end

-- unpack_codes reads CMP03 wire-format bytes into a list of LZW codes.
--
-- Mirrors pack_codes exactly: same code_size widening logic, same next_code
-- tracking. Returns the codes list and the original_length from the header.
--
-- @param data (string) binary string from pack_codes / compress.
-- @return (table, integer) codes array and original_length.
local function unpack_codes(data)
    -- Need at least a 4-byte header.
    if #data < 4 then
        return {CLEAR_CODE, STOP_CODE}, 0
    end

    local original_length, body_start = read_uint32_be(data, 1)
    local body = data:sub(body_start)

    local br        = new_bit_reader(body)
    local code_size = INITIAL_CODE_SIZE
    local next_code = INITIAL_NEXT_CODE
    local codes     = {}

    while not br:exhausted() do
        local code = br:read(code_size)
        if code == nil then break end

        codes[#codes + 1] = code

        if code == STOP_CODE then
            break

        elseif code == CLEAR_CODE then
            code_size = INITIAL_CODE_SIZE
            next_code = INITIAL_NEXT_CODE

        else
            -- Data code: mirror the encoder's next_code tracking.
            if next_code < (1 << MAX_CODE_SIZE) then
                next_code = next_code + 1
                if next_code > (1 << code_size) and code_size < MAX_CODE_SIZE then
                    code_size = code_size + 1
                end
            end
        end
    end

    return codes, original_length
end

-- ── One-shot API ──────────────────────────────────────────────────────────────

-- compress compresses a Lua string using LZW and returns CMP03 wire-format bytes.
--
-- Usage:
--   local compressed = lzw.compress("ABABABABAB")
--   -- compressed is a binary string starting with a 4-byte length header
--
-- @param str (string) input string.
-- @return (string) compressed binary string.
function M.compress(str)
    -- Convert the Lua string to a 1-indexed byte array.
    local data = {str:byte(1, #str)}

    local codes           = encode_codes(data)
    local original_length = #data

    return pack_codes(codes, original_length)
end

-- decompress decompresses data produced by compress().
--
-- Usage:
--   local original = lzw.decompress(compressed)
--
-- @param data (string) compressed binary string in CMP03 wire format.
-- @return (string) original uncompressed string.
function M.decompress(data)
    local codes, original_length = unpack_codes(data)
    local bytes                  = decode_codes(codes)

    -- Trim to original_length in case of any over-production.
    if #bytes > original_length then
        local trimmed = {}
        for i = 1, original_length do trimmed[i] = bytes[i] end
        bytes = trimmed
    end

    if #bytes == 0 then return "" end

    -- Convert bytes (a table of integers) to a string in 256-element chunks
    -- to avoid hitting Lua's C stack limit on large outputs.
    local parts   = {}
    local n       = #bytes
    local CHUNK   = 256
    local i       = 1
    while i <= n do
        local j = math.min(i + CHUNK - 1, n)
        parts[#parts + 1] = string.char(table.unpack(bytes, i, j))
        i = j + 1
    end
    return table.concat(parts)
end

return M
