#!/usr/bin/env lua
-- hash-breaker — Demonstrating why MD5 is cryptographically broken.
--
-- Three attacks against MD5:
--   1. Known Collision Pairs (Wang & Yu, 2004)
--   2. Length Extension Attack (forge hash without secret)
--   3. Birthday Attack on truncated hash (birthday paradox)
--
-- Each attack prints educational output explaining the cryptographic concept.

-- ── Set up package path to find our MD5 library ────────────────────────────
package.path = "../../../packages/lua/md5/src/?.lua;"
    .. "../../../packages/lua/md5/src/?/init.lua;"
    .. package.path

local md5 = require("coding_adventures.md5")

-- ============================================================================
-- Utility functions
-- ============================================================================

local function hex_to_bytes(hex_str)
    local bytes = {}
    for i = 1, #hex_str, 2 do
        bytes[#bytes + 1] = tonumber(hex_str:sub(i, i + 1), 16)
    end
    return bytes
end

local function bytes_to_string(bytes)
    local chars = {}
    for i = 1, #bytes do
        chars[i] = string.char(bytes[i])
    end
    return table.concat(chars)
end

local function string_to_bytes(s)
    local bytes = {}
    for i = 1, #s do
        bytes[i] = s:byte(i)
    end
    return bytes
end

local function bytes_to_hex(bytes)
    local hex = {}
    for i = 1, #bytes do
        hex[i] = string.format("%02x", bytes[i])
    end
    return table.concat(hex)
end

local function hex_dump(bytes)
    local lines = {}
    for i = 1, #bytes, 16 do
        local row = {}
        for j = i, math.min(i + 15, #bytes) do
            row[#row + 1] = string.format("%02x", bytes[j])
        end
        lines[#lines + 1] = "  " .. table.concat(row)
    end
    return table.concat(lines, "\n")
end

-- ============================================================================
-- ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
-- ============================================================================

local collision_a_hex =
    "d131dd02c5e6eec4693d9a0698aff95c" ..
    "2fcab58712467eab4004583eb8fb7f89" ..
    "55ad340609f4b30283e488832571415a" ..
    "085125e8f7cdc99fd91dbdf280373c5b" ..
    "d8823e3156348f5bae6dacd436c919c6" ..
    "dd53e2b487da03fd02396306d248cda0" ..
    "e99f33420f577ee8ce54b67080a80d1e" ..
    "c69821bcb6a8839396f9652b6ff72a70"

local collision_b_hex =
    "d131dd02c5e6eec4693d9a0698aff95c" ..
    "2fcab50712467eab4004583eb8fb7f89" ..
    "55ad340609f4b30283e4888325f1415a" ..
    "085125e8f7cdc99fd91dbd7280373c5b" ..
    "d8823e3156348f5bae6dacd436c919c6" ..
    "dd53e23487da03fd02396306d248cda0" ..
    "e99f33420f577ee8ce54b67080280d1e" ..
    "c69821bcb6a8839396f965ab6ff72a70"

local MASK32 = 0xFFFFFFFF

local function attack_1()
    print(string.rep("=", 72))
    print("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)")
    print(string.rep("=", 72))
    print()
    print("Two different 128-byte messages that produce the SAME MD5 hash.")
    print("This was the breakthrough that proved MD5 is broken for security.")
    print()

    local bytes_a = hex_to_bytes(collision_a_hex)
    local bytes_b = hex_to_bytes(collision_b_hex)

    print("Block A (hex):")
    print(hex_dump(bytes_a))
    print()
    print("Block B (hex):")
    print(hex_dump(bytes_b))
    print()

    -- Show byte differences
    local diffs = {}
    for i = 1, #bytes_a do
        if bytes_a[i] ~= bytes_b[i] then
            diffs[#diffs + 1] = i - 1  -- 0-indexed for display
        end
    end
    local diff_strs = {}
    for _, d in ipairs(diffs) do diff_strs[#diff_strs + 1] = tostring(d) end
    print(string.format("Blocks differ at %d byte positions: [%s]", #diffs, table.concat(diff_strs, ", ")))
    for _, pos in ipairs(diffs) do
        print(string.format("  Byte %d: A=0x%02x  B=0x%02x", pos, bytes_a[pos + 1], bytes_b[pos + 1]))
    end
    print()

    local str_a = bytes_to_string(bytes_a)
    local str_b = bytes_to_string(bytes_b)
    local hash_a = md5.hex(str_a)
    local hash_b = md5.hex(str_b)
    print("MD5(A) = " .. hash_a)
    print("MD5(B) = " .. hash_b)
    local match_str = hash_a == hash_b and "YES — COLLISION!" or "No (unexpected)"
    print("Match?   " .. match_str)
    print()
    print("Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.")
    print()
end

-- ============================================================================
-- ATTACK 2: Length Extension Attack
-- ============================================================================

-- MD5 T-table
local T = {}
for i = 0, 63 do
    T[i] = math.floor(math.abs(math.sin(i + 1)) * 4294967296) & MASK32
end

-- MD5 shifts
local S = {
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
}

local function rotl32(x, n)
    return ((x << n) | (x >> (32 - n))) & MASK32
end

-- Inline MD5 compression for length extension.
local function md5_compress(state_a, state_b, state_c, state_d, block_str)
    -- Parse 16 little-endian 32-bit words from block
    local m = {}
    for i = 0, 15 do
        local offset = i * 4 + 1
        local b0 = block_str:byte(offset)
        local b1 = block_str:byte(offset + 1)
        local b2 = block_str:byte(offset + 2)
        local b3 = block_str:byte(offset + 3)
        m[i] = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    end

    local a, b, c, d = state_a, state_b, state_c, state_d
    local a0, b0, c0, d0 = a, b, c, d

    for i = 0, 63 do
        local f, g
        if i < 16 then
            f = (b & c) | (~b & d)
            g = i
        elseif i < 32 then
            f = (d & b) | (~d & c)
            g = (5 * i + 1) % 16
        elseif i < 48 then
            f = b ~ c ~ d
            g = (3 * i + 5) % 16
        else
            f = c ~ (b | ~d)
            g = (7 * i) % 16
        end
        f = f & MASK32

        local temp = d
        d = c
        c = b
        b = (b + rotl32((a + f + T[i] + m[g]) & MASK32, S[i + 1])) & MASK32
        a = temp
    end

    return (a0 + a) & MASK32, (b0 + b) & MASK32, (c0 + c) & MASK32, (d0 + d) & MASK32
end

local function md5_padding(message_len)
    local remainder = message_len % 64
    local pad_len = (55 - remainder) % 64
    local padding = string.char(0x80) .. string.rep("\0", pad_len)
    -- Append bit length as 64-bit little-endian
    local bit_len = message_len * 8
    local lo = bit_len & MASK32
    local hi = (bit_len >> 32) & MASK32
    -- Pack as two LE 32-bit integers
    padding = padding .. string.char(
        lo & 0xFF, (lo >> 8) & 0xFF, (lo >> 16) & 0xFF, (lo >> 24) & 0xFF,
        hi & 0xFF, (hi >> 8) & 0xFF, (hi >> 16) & 0xFF, (hi >> 24) & 0xFF
    )
    return padding
end

local function word_to_le_bytes(w)
    return string.char(w & 0xFF, (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF)
end

local function le_bytes_to_word(s, offset)
    return s:byte(offset) | (s:byte(offset + 1) << 8) | (s:byte(offset + 2) << 16) | (s:byte(offset + 3) << 24)
end

local function attack_2()
    print(string.rep("=", 72))
    print("ATTACK 2: Length Extension Attack")
    print(string.rep("=", 72))
    print()
    print("Given md5(secret + message) and len(secret + message), we can forge")
    print("md5(secret + message + padding + evil_data) WITHOUT knowing the secret!")
    print()

    local secret = "supersecretkey!!"
    local message = "amount=100&to=alice"
    local original_data = secret .. message
    local original_hash_bytes = md5.digest(original_data)
    local original_hash_str = bytes_to_string(original_hash_bytes)
    local original_hex = bytes_to_hex(original_hash_bytes)

    print('Secret (unknown to attacker): "' .. secret .. '"')
    print('Message:                      "' .. message .. '"')
    print("MAC = md5(secret || message): " .. original_hex)
    print("Length of (secret || message): " .. #original_data .. " bytes")
    print()

    local evil_data = "&amount=1000000&to=mallory"
    print('Evil data to append: "' .. evil_data .. '"')
    print()

    -- Step 1: Extract state from hash
    local a = le_bytes_to_word(original_hash_str, 1)
    local b = le_bytes_to_word(original_hash_str, 5)
    local c = le_bytes_to_word(original_hash_str, 9)
    local d = le_bytes_to_word(original_hash_str, 13)

    print("Step 1: Extract MD5 internal state from the hash")
    print(string.format("  A = 0x%08x, B = 0x%08x, C = 0x%08x, D = 0x%08x", a, b, c, d))
    print()

    -- Step 2: Compute padding
    local padding = md5_padding(#original_data)
    print("Step 2: Compute MD5 padding for the original message")
    print("  Padding (" .. #padding .. " bytes): " .. bytes_to_hex(string_to_bytes(padding)))
    print()

    local processed_len = #original_data + #padding
    print("Step 3: Total bytes processed so far: " .. processed_len)
    print()

    -- Step 4: Forge
    local forged_input = evil_data .. md5_padding(processed_len + #evil_data)
    local sa, sb, sc, sd = a, b, c, d
    for i = 1, #forged_input, 64 do
        if i + 63 <= #forged_input then
            sa, sb, sc, sd = md5_compress(sa, sb, sc, sd, forged_input:sub(i, i + 63))
        end
    end
    local forged_hex = bytes_to_hex(string_to_bytes(
        word_to_le_bytes(sa) .. word_to_le_bytes(sb) .. word_to_le_bytes(sc) .. word_to_le_bytes(sd)
    ))

    print("Step 4: Initialize hasher with extracted state, feed evil_data")
    print("  Forged hash: " .. forged_hex)
    print()

    -- Step 5: Verify
    local actual_full = original_data .. padding .. evil_data
    local actual_hex = md5.hex(actual_full)

    print("Step 5: Verify — compute actual md5(secret || message || padding || evil_data)")
    print("  Actual hash: " .. actual_hex)
    local match_str = forged_hex == actual_hex and "YES — FORGED!" or "No (bug)"
    print("  Match?       " .. match_str)
    print()
    print("The attacker forged a valid MAC without knowing the secret!")
    print()
    print("Why HMAC fixes this:")
    print("  HMAC = md5(key XOR opad || md5(key XOR ipad || message))")
    print("  The outer hash prevents length extension because the attacker")
    print("  cannot extend past the outer md5() boundary.")
    print()
end

-- ============================================================================
-- ATTACK 3: Birthday Attack (Truncated Hash)
-- ============================================================================

local function attack_3()
    print(string.rep("=", 72))
    print("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)")
    print(string.rep("=", 72))
    print()
    print("The birthday paradox: with N possible hash values, expect a collision")
    print("after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.")
    print()

    -- Deterministic xorshift32 PRNG
    local rng_state = 42
    local function xorshift32()
        rng_state = rng_state ~ (rng_state << 13)
        rng_state = rng_state & MASK32
        rng_state = rng_state ~ (rng_state >> 17)
        rng_state = rng_state & MASK32
        rng_state = rng_state ~ (rng_state << 5)
        rng_state = rng_state & MASK32
        return rng_state
    end

    local seen = {}  -- truncated_hex -> msg_hex
    local attempts = 0

    while true do
        attempts = attempts + 1
        -- Generate random 8-byte message
        local msg_bytes = {}
        for i = 1, 8 do
            msg_bytes[i] = xorshift32() & 0xFF
        end
        local msg_str = bytes_to_string(msg_bytes)
        local msg_hex = bytes_to_hex(msg_bytes)

        local hash_bytes = md5.digest(msg_str)
        local truncated_hex = bytes_to_hex({hash_bytes[1], hash_bytes[2], hash_bytes[3], hash_bytes[4]})

        if seen[truncated_hex] then
            local other_hex = seen[truncated_hex]
            if other_hex ~= msg_hex then
                print(string.format("COLLISION FOUND after %d attempts!", attempts))
                print()
                print("  Message 1: " .. other_hex)
                print("  Message 2: " .. msg_hex)
                print("  Truncated MD5 (4 bytes): " .. truncated_hex)
                -- Compute full hashes for display
                local other_bytes = hex_to_bytes(other_hex)
                print("  Full MD5 of msg1: " .. md5.hex(bytes_to_string(other_bytes)))
                print("  Full MD5 of msg2: " .. md5.hex(msg_str))
                print()
                print(string.format("  Expected ~65536 attempts (2^16), got %d", attempts))
                print(string.format("  Ratio: %.2fx the theoretical expectation", attempts / 65536))
                break
            end
        else
            seen[truncated_hex] = msg_hex
        end
    end

    print()
    print("This is a GENERIC attack — it works against any hash function.")
    print("The defense is a longer hash: SHA-256 has 2^128 birthday bound,")
    print("while MD5 has only 2^64 (and dedicated attacks are even faster).")
    print()
end

-- ============================================================================
-- Main
-- ============================================================================

print()
print("======================================================================")
print("           MD5 HASH BREAKER — Why MD5 Is Broken")
print("======================================================================")
print("  Three attacks showing MD5 must NEVER be used for security:")
print("    1. Known collision pairs (Wang & Yu, 2004)")
print("    2. Length extension attack (forge MAC without secret)")
print("    3. Birthday attack on truncated hash (birthday paradox)")
print("======================================================================")
print()

attack_1()
attack_2()
attack_3()

print(string.rep("=", 72))
print("CONCLUSION")
print(string.rep("=", 72))
print()
print("MD5 is broken in three distinct ways:")
print("  1. COLLISION RESISTANCE: known pairs exist (and can be generated)")
print("  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state")
print("  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)")
print()
print("Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.")
print()
