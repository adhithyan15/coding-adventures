-- wasm_leb128 — LEB128 variable-length integer encoding for WebAssembly
--
-- LEB128 (Little-Endian Base-128) is a variable-length encoding for integers.
-- It was originally designed for the DWARF debugging format and is now used
-- extensively in WebAssembly's binary format to encode integers compactly:
-- small values use 1 byte, larger values use 2, 3, … bytes as needed.
--
-- WHY VARIABLE-LENGTH ENCODING?
-- ──────────────────────────────
-- A fixed 4-byte (int32) encoding always uses 4 bytes. The number 1 doesn't
-- need 4 bytes — it only needs 1. In WebAssembly, most integers are small
-- (function indices, local variable counts, etc.), so LEB128 saves significant
-- space in binary modules.
--
-- HOW LEB128 WORKS — THE IDEA
-- ────────────────────────────
-- Each byte carries 7 bits of the value in its low 7 bits (the "payload").
-- The high bit (bit 7) is the "continuation bit":
--   1 → more bytes follow
--   0 → this is the last byte
--
-- Example: encoding 624485 (0x98765) as unsigned LEB128
--   624485 in binary: 10011000011101100101
--   Split into 7-bit groups (LSB first): 1100101 | 0001101 | 0011000
--   Groups: 101 (5 bits), then: 0100110, 0001011 (shifted)
--   Wait — let's do it properly:
--     Step 1: value = 624485 = 0b10011000011101100101
--     byte 1: low 7 bits = 1100101 = 0x65 → set continuation → 0xE5
--     value >>= 7 → 0b100110000111011 = 4915 (still > 0)
--     byte 2: low 7 bits = 0001110 = 0x0E → set continuation → 0x8E
--     value >>= 7 → 0b1001100 = 38 (still > 0)
--     byte 3: low 7 bits = 0100110 = 0x26 → no continuation → 0x26
--   Result: {0xE5, 0x8E, 0x26}
--
-- UNSIGNED vs SIGNED LEB128
-- ──────────────────────────
-- Unsigned: the loop terminates when value == 0.
-- Signed: the loop terminates when:
--   - value == 0 AND bit 6 of the current byte is 0 (positive number done)
--   - value == -1 AND bit 6 of the current byte is 1 (negative number done)
-- On decode, signed LEB128 sign-extends: if the last byte has bit 6 set and
-- we haven't filled all the value bits, the remaining high bits are 1.
--
-- WEBASSEMBLY USES BOTH
-- ──────────────────────
-- WebAssembly uses:
--   unsigned LEB128 for: section sizes, type indices, function indices, etc.
--   signed LEB128 for: i32.const values, i64.const values, etc.
--
-- OFFSET PARAMETER (Lua 1-based)
-- ────────────────────────────────
-- decode_unsigned and decode_signed accept an optional `offset` parameter.
-- This is 1-based (Lua convention) and defaults to 1.
-- It allows decoding starting from the middle of a byte sequence, which is
-- useful when parsing a WebAssembly binary stream.
--
-- Usage:
--   local leb = require("coding_adventures.wasm_leb128")
--
--   local bytes = leb.encode_unsigned(624485)  -- {0xE5, 0x8E, 0x26}
--   local bytes = leb.encode_signed(-2)         -- {0x7E}
--
--   local value, count = leb.decode_unsigned({0xE5, 0x8E, 0x26})  -- 624485, 3
--   local value, count = leb.decode_signed({0x7E})                  -- -2, 1
--
--   local value, count = leb.decode_unsigned({0x00, 0xE5, 0x8E, 0x26}, 2)
--   -- Starts at index 2 → decodes 624485 from bytes 2,3,4; count = 3
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Internal constants
-- ---------------------------------------------------------------------------

-- Bit 7: the continuation flag — set if more bytes follow
local CONTINUATION_BIT = 0x80

-- Bits 0–6: the payload — the actual 7-bit data
local PAYLOAD_MASK = 0x7F

-- Maximum bytes for a 32-bit value (ceil(32/7) = 5)
local MAX_BYTES_U32 = 5

-- ---------------------------------------------------------------------------
-- encode_unsigned(value) → byte array (list of integers 0..255)
--
-- Encodes a non-negative integer as unsigned LEB128.
-- Each output byte carries 7 bits of the value (LSB group first).
-- The continuation bit (0x80) is set on all bytes except the last.
--
-- Example: encode_unsigned(300)
--   300 = 0b100101100
--   byte 1: 300 & 0x7F = 0b0101100 = 44 (0x2C), more bytes → 0xAC
--   300 >> 7 = 2
--   byte 2: 2 & 0x7F = 2, no more bytes → 0x02
--   Result: {0xAC, 0x02}
-- ---------------------------------------------------------------------------
function M.encode_unsigned(value)
    if type(value) ~= "number" then
        error("wasm_leb128.encode_unsigned: expected number, got " .. type(value))
    end
    value = math.floor(value)
    if value < 0 then
        error("wasm_leb128.encode_unsigned: value must be non-negative, got " .. tostring(value))
    end

    local bytes = {}
    repeat
        local byte = value & PAYLOAD_MASK   -- take the low 7 bits
        value = value >> 7                  -- shift the remaining value right
        if value ~= 0 then
            byte = byte | CONTINUATION_BIT  -- set continuation bit
        end
        bytes[#bytes + 1] = byte
    until value == 0

    return bytes
end

-- ---------------------------------------------------------------------------
-- sar(x, n) — arithmetic (sign-extending) right shift by n bits
--
-- IMPORTANT: In Lua 5.4 and 5.5, the `>>` operator performs a LOGICAL
-- (unsigned) right shift on integers. This means `-1 >> 1` gives a large
-- positive number (0x7FFFFFFFFFFFFFFF) rather than -1 as expected for
-- signed arithmetic.
--
-- For signed LEB128 encoding we need arithmetic right shift (SAR): the sign
-- bit is replicated as we shift right, so -1 >> anything remains -1.
--
-- Implementation: if x < 0, logical-shift and then OR in the sign bits.
--   ~(~0 >> n) constructs a mask of n high 1-bits:
--     ~0         = 0xFFFFFFFFFFFFFFFF  (all 1s)
--     ~0 >> n    = 0x00FFFFFFFFFFFFFF  (n=8: logical shift fills with 0)
--     ~(~0 >> n) = 0xFF00000000000000  (complement → n high 1-bits)
-- ---------------------------------------------------------------------------
local function sar(x, n)
    if x >= 0 then
        return x >> n
    else
        -- Logical right shift, then fill the vacated high bits with 1s
        return (x >> n) | (~(~0 >> n))
    end
end

-- ---------------------------------------------------------------------------
-- encode_signed(value) → byte array (list of integers 0..255)
--
-- Encodes a signed integer (positive or negative) as signed LEB128.
-- The termination condition is different from unsigned:
--   - For positive numbers: done when value == 0 and bit 6 of byte is 0
--     (bit 6 is the sign bit in the last 7-bit group; 0 means positive)
--   - For negative numbers: done when value == -1 and bit 6 of byte is 1
--     (all remaining bits are 1 in two's complement; bit 6 = 1 confirms)
--
-- Example: encode_signed(-2)
--   -2 in two's complement (64-bit): ...11111110
--   byte 1: -2 & 0x7F = 0x7E = 0b1111110
--   sar(-2, 7) = -1 (arithmetic shift)
--   bit 6 of 0x7E is 1 (0x7E & 0x40 = 0x40 ≠ 0), and value == -1 → done
--   Result: {0x7E}  ✓
-- ---------------------------------------------------------------------------
function M.encode_signed(value)
    if type(value) ~= "number" then
        error("wasm_leb128.encode_signed: expected number, got " .. type(value))
    end
    value = math.tointeger(value)
    if value == nil then
        error("wasm_leb128.encode_signed: value must be an integer")
    end

    local bytes = {}
    local more = true
    while more do
        local byte = value & PAYLOAD_MASK   -- take low 7 bits (as unsigned 7-bit)
        value = sar(value, 7)               -- arithmetic right shift (sign-extending)

        -- Check if we're done:
        -- If value is now 0 and the sign bit (bit 6) of `byte` is 0 → positive, done
        -- If value is now -1 and the sign bit (bit 6) of `byte` is 1 → negative, done
        local sign_bit = byte & 0x40        -- bit 6 of the payload byte

        if (value == 0 and sign_bit == 0) or (value == -1 and sign_bit ~= 0) then
            more = false
            -- No continuation bit on the last byte
        else
            byte = byte | CONTINUATION_BIT
        end

        bytes[#bytes + 1] = byte
    end

    return bytes
end

-- ---------------------------------------------------------------------------
-- decode_unsigned(bytes, offset) → value, count
--
-- Decodes an unsigned LEB128 integer from `bytes` starting at `offset`
-- (1-based, defaults to 1).
--
-- Returns:
--   value  — the decoded integer
--   count  — number of bytes consumed
--
-- Errors if the byte sequence is unterminated (no byte with clear continuation bit)
-- within the reasonable range for a 64-bit integer (10 bytes).
-- ---------------------------------------------------------------------------
function M.decode_unsigned(bytes, offset)
    offset = offset or 1

    local result = 0
    local shift  = 0
    local count  = 0
    local i      = offset

    while true do
        if i > #bytes then
            error("wasm_leb128.decode_unsigned: unterminated LEB128 sequence")
        end

        local byte = bytes[i]
        i     = i + 1
        count = count + 1

        -- Accumulate 7 payload bits at the current shift position
        result = result | ((byte & PAYLOAD_MASK) << shift)
        shift  = shift + 7

        -- If the continuation bit is clear, we've read the last byte
        if (byte & CONTINUATION_BIT) == 0 then
            break
        end

        -- Safety limit: 64-bit integers need at most 10 LEB128 bytes
        if shift >= 70 then
            error("wasm_leb128.decode_unsigned: LEB128 sequence too long")
        end
    end

    return result, count
end

-- ---------------------------------------------------------------------------
-- decode_signed(bytes, offset) → value, count
--
-- Decodes a signed LEB128 integer from `bytes` starting at `offset`
-- (1-based, defaults to 1).
--
-- After reading the last byte (continuation bit clear), if the sign bit
-- (bit 6) of that byte is set AND we haven't shifted through all 64 bits,
-- we sign-extend by ORing in all-1s above the current shift position.
-- This correctly reconstructs negative two's-complement values.
--
-- Example: {0x7E} → byte = 0x7E = 0b01111110
--   payload = 0x7E & 0x7F = 0x7E = 0b1111110 = 126
--   shift = 7
--   bit 6 of last byte (0x40 & 0x7E) = 0x40 ≠ 0 → sign-extend
--   result |= (~0 << 7) = result | 0xFFFFFFFFFFFFFF80
--   126 | 0xFFFFFFFFFFFFFF80 = 0xFFFFFFFFFFFFFFFE = -2  ✓
-- ---------------------------------------------------------------------------
function M.decode_signed(bytes, offset)
    offset = offset or 1

    local result = 0
    local shift  = 0
    local count  = 0
    local i      = offset
    local last_byte = 0

    while true do
        if i > #bytes then
            error("wasm_leb128.decode_signed: unterminated LEB128 sequence")
        end

        local byte = bytes[i]
        i          = i + 1
        count      = count + 1
        last_byte  = byte

        -- Accumulate 7 payload bits
        result = result | ((byte & PAYLOAD_MASK) << shift)
        shift  = shift + 7

        -- Check continuation bit
        if (byte & CONTINUATION_BIT) == 0 then
            break
        end

        if shift >= 70 then
            error("wasm_leb128.decode_signed: LEB128 sequence too long")
        end
    end

    -- Sign-extend if the sign bit (bit 6) of the last payload byte is set
    -- and we haven't consumed all 64 bits.
    if shift < 64 and (last_byte & 0x40) ~= 0 then
        -- ~0 = all 1s in Lua (64-bit); shift left to make a sign-extension mask
        result = result | (~0 << shift)
    end

    return result, count
end

return M
