--- Pure non-cryptographic hash functions and deterministic analysis helpers.
---
--- Lua 5.4 integers are signed 64-bit values. Functions that produce a
--- 64-bit word therefore return the exact two's-complement bit pattern, which
--- may be negative. `uint64_hex` exposes the corresponding unsigned bits.

local M = {}

local MASK32 = 0xffffffff
local FNV32_OFFSET_BASIS = 0x811c9dc5
local FNV32_PRIME = 0x01000193
local FNV64_OFFSET_BASIS = 0xcbf29ce484222325
local FNV64_PRIME = 0x00000100000001b3
local POLYNOMIAL_DEFAULT_BASE = 31
local POLYNOMIAL_DEFAULT_MODULUS = (1 << 61) - 1
local MURMUR3_C1 = 0xcc9e2d51
local MURMUR3_C2 = 0x1b873593

M.FNV32_OFFSET_BASIS = FNV32_OFFSET_BASIS
M.FNV32_PRIME = FNV32_PRIME
M.FNV64_OFFSET_BASIS = FNV64_OFFSET_BASIS
M.FNV64_PRIME = FNV64_PRIME
M.POLYNOMIAL_ROLLING_DEFAULT_BASE = POLYNOMIAL_DEFAULT_BASE
M.POLYNOMIAL_ROLLING_DEFAULT_MODULUS = POLYNOMIAL_DEFAULT_MODULUS

local function require_string(data, level)
    if type(data) ~= "string" then
        error("data must be a string", level or 3)
    end
    return data
end

local function require_integer(value, name, level)
    if type(value) ~= "number" or math.type(value) ~= "integer" then
        error(name .. " must be an integer", level or 3)
    end
    return value
end

local function add_mod(left, right, modulus)
    -- left and right are already normalized into [0, modulus). Comparing
    -- before adding avoids signed 64-bit overflow.
    local distance = modulus - right
    if left >= distance then
        return left - distance
    end
    return left + right
end

local function multiply_mod(left, right, modulus)
    left = left % modulus
    right = right % modulus
    local result = 0
    while right > 0 do
        if (right & 1) ~= 0 then
            result = add_mod(result, left, modulus)
        end
        right = right >> 1
        if right > 0 then
            left = add_mod(left, left, modulus)
        end
    end
    return result
end

--- Compute the 32-bit Fowler-Noll-Vo FNV-1a hash.
function M.fnv1a_32(data)
    data = require_string(data, 2)
    local hash = FNV32_OFFSET_BASIS
    for index = 1, #data do
        hash = ((hash ~ string.byte(data, index)) * FNV32_PRIME) & MASK32
    end
    return hash
end

--- Compute the 64-bit Fowler-Noll-Vo FNV-1a bit pattern.
function M.fnv1a_64(data)
    data = require_string(data, 2)
    local hash = FNV64_OFFSET_BASIS
    for index = 1, #data do
        hash = (hash ~ string.byte(data, index)) * FNV64_PRIME
    end
    return hash
end

--- Compute Dan Bernstein's DJB2 hash, bounded to 64 bits.
function M.djb2(data)
    data = require_string(data, 2)
    local hash = 5381
    for index = 1, #data do
        hash = (hash << 5) + hash + string.byte(data, index)
    end
    return hash
end

--- Format a signed Lua integer as its exact unsigned 64-bit hexadecimal word.
function M.uint64_hex(value)
    require_integer(value, "value", 2)
    return string.format("%016x", value)
end

--- Compute a polynomial rolling hash with overflow-safe modular arithmetic.
function M.polynomial_rolling(data, base, modulus)
    data = require_string(data, 2)
    base = base == nil and POLYNOMIAL_DEFAULT_BASE or base
    modulus = modulus == nil and POLYNOMIAL_DEFAULT_MODULUS or modulus
    require_integer(base, "base", 2)
    require_integer(modulus, "modulus", 2)
    if modulus <= 0 then
        error("modulus must be positive", 2)
    end

    local normalized_base = base % modulus
    local hash = 0
    for index = 1, #data do
        hash = multiply_mod(hash, normalized_base, modulus)
        hash = add_mod(hash, string.byte(data, index) % modulus, modulus)
    end
    return hash
end

local function rotate_left_32(value, count)
    return ((value << count) | (value >> (32 - count))) & MASK32
end

local function fmix32(hash)
    hash = (hash ~ (hash >> 16)) & MASK32
    hash = (hash * 0x85ebca6b) & MASK32
    hash = (hash ~ (hash >> 13)) & MASK32
    hash = (hash * 0xc2b2ae35) & MASK32
    return (hash ~ (hash >> 16)) & MASK32
end

--- Compute Austin Appleby's MurmurHash3 32-bit variant.
function M.murmur3_32(data, seed)
    data = require_string(data, 2)
    seed = seed == nil and 0 or seed
    require_integer(seed, "seed", 2)

    local length = #data
    local hash = seed & MASK32
    local block_count = length >> 2

    for block_index = 0, block_count - 1 do
        local offset = block_index * 4 + 1
        local k = string.byte(data, offset)
            | (string.byte(data, offset + 1) << 8)
            | (string.byte(data, offset + 2) << 16)
            | (string.byte(data, offset + 3) << 24)

        k = (k * MURMUR3_C1) & MASK32
        k = rotate_left_32(k, 15)
        k = (k * MURMUR3_C2) & MASK32

        hash = hash ~ k
        hash = rotate_left_32(hash, 13)
        hash = (hash * 5 + 0xe6546b64) & MASK32
    end

    local tail_offset = block_count * 4 + 1
    local remaining = length & 3
    local k = 0
    if remaining >= 3 then
        k = k ~ (string.byte(data, tail_offset + 2) << 16)
    end
    if remaining >= 2 then
        k = k ~ (string.byte(data, tail_offset + 1) << 8)
    end
    if remaining >= 1 then
        k = k ~ string.byte(data, tail_offset)
        k = (k * MURMUR3_C1) & MASK32
        k = rotate_left_32(k, 15)
        k = (k * MURMUR3_C2) & MASK32
        hash = hash ~ k
    end

    hash = hash ~ length
    return fmix32(hash)
end

local function deterministic_bytes(sample_index)
    local state = (0x9e3779b9 ~ sample_index) & MASK32
    local bytes = {}
    for index = 1, 8 do
        state = (state * 1664525 + 1013904223) & MASK32
        bytes[index] = state & 0xff
    end
    return bytes
end

local function hash_result(hash_fn, data)
    local value = hash_fn(data)
    if type(value) ~= "number" or math.type(value) ~= "integer" then
        error("hash function must return an integer", 4)
    end
    return value
end

local function popcount_width(value, width)
    local count = 0
    for _ = 1, width do
        count = count + (value & 1)
        value = value >> 1
    end
    return count
end

--- Estimate the fraction of output bits flipped by one input-bit change.
function M.avalanche_score(hash_fn, output_bits, sample_size)
    if type(hash_fn) ~= "function" then
        error("hash_fn must be a function", 2)
    end
    require_integer(output_bits, "output_bits", 2)
    if output_bits < 1 or output_bits > 64 then
        error("output_bits must be in 1..64", 2)
    end
    sample_size = sample_size == nil and 100 or sample_size
    require_integer(sample_size, "sample_size", 2)
    if sample_size <= 0 then
        error("sample_size must be positive", 2)
    end

    local total_bit_flips = 0
    local total_trials = 0
    for sample_index = 0, sample_size - 1 do
        local bytes = deterministic_bytes(sample_index)
        local original = hash_result(hash_fn, string.char(table.unpack(bytes)))
        for bit_position = 0, 63 do
            local byte_index = (bit_position >> 3) + 1
            local bit_mask = 1 << (bit_position & 7)
            bytes[byte_index] = bytes[byte_index] ~ bit_mask
            local changed = hash_result(hash_fn, string.char(table.unpack(bytes)))
            bytes[byte_index] = bytes[byte_index] ~ bit_mask
            total_bit_flips = total_bit_flips
                + popcount_width(original ~ changed, output_bits)
            total_trials = total_trials + output_bits
        end
    end
    return total_bit_flips / total_trials
end

local function power_of_two_mod(exponent, modulus)
    local result = 1 % modulus
    for _ = 1, exponent do
        result = add_mod(result, result, modulus)
    end
    return result
end

--- Return the chi-squared statistic for a hash function's bucket spread.
function M.distribution_test(hash_fn, inputs, num_buckets)
    if type(hash_fn) ~= "function" then
        error("hash_fn must be a function", 2)
    end
    if type(inputs) ~= "table" or #inputs == 0 then
        error("inputs must be a non-empty array", 2)
    end
    require_integer(num_buckets, "num_buckets", 2)
    if num_buckets <= 0 then
        error("num_buckets must be positive", 2)
    end

    local counts = {}
    for bucket = 1, num_buckets do
        counts[bucket] = 0
    end
    local unsigned_wrap = power_of_two_mod(64, num_buckets)
    for index = 1, #inputs do
        require_string(inputs[index], 2)
        local value = hash_result(hash_fn, inputs[index])
        local bucket = value % num_buckets
        if value < 0 then
            bucket = add_mod(bucket, unsigned_wrap, num_buckets)
        end
        counts[bucket + 1] = counts[bucket + 1] + 1
    end

    local expected = #inputs / num_buckets
    local chi_squared = 0.0
    for bucket = 1, num_buckets do
        local difference = counts[bucket] - expected
        chi_squared = chi_squared + difference * difference / expected
    end
    return chi_squared
end

return M
