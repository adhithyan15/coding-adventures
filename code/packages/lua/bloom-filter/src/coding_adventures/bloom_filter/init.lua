--- Space-efficient probabilistic set membership with no false negatives.

local hash_functions = require("coding_adventures.hash_functions")

local M = {}

local DEFAULT_EXPECTED_ITEMS = 1000
local DEFAULT_FALSE_POSITIVE_RATE = 0.01
local MASK32 = 0xffffffff

local function require_positive_integer(value, name, level)
    if type(value) ~= "number" or math.type(value) ~= "integer" or value <= 0 then
        error(name .. " must be a positive integer", level or 3)
    end
    return value
end

local function require_nonnegative_integer(value, name, level)
    if type(value) ~= "number" or math.type(value) ~= "integer" or value < 0 then
        error(name .. " must be a nonnegative integer", level or 3)
    end
    return value
end

local function require_false_positive_rate(value, level)
    if type(value) ~= "number"
        or value ~= value
        or value == math.huge
        or value == -math.huge
        or value <= 0
        or value >= 1
    then
        error("false_positive_rate must be in the open interval (0, 1)", level or 3)
    end
    return value
end

local function stable_encode(value, seen)
    local kind = type(value)
    if kind == "string" then
        return value
    elseif kind == "number" then
        return tostring(value)
    elseif kind == "boolean" then
        return value and "true" or "false"
    elseif kind == "nil" then
        return "nil"
    elseif kind ~= "table" then
        error("element must be nil, a scalar, or a table", 4)
    end

    if seen[value] then
        error("element tables must not contain cycles", 4)
    end
    seen[value] = true
    local entries = {}
    for key, item in pairs(value) do
        local encoded_key = stable_encode(key, seen)
        local encoded_value = stable_encode(item, seen)
        entries[#entries + 1] = #encoded_key
            .. ":"
            .. encoded_key
            .. "="
            .. #encoded_value
            .. ":"
            .. encoded_value
    end
    seen[value] = nil
    table.sort(entries)
    return "{" .. table.concat(entries, ",") .. "}"
end

local function element_bytes(value)
    return stable_encode(value, {})
end

local function fmix32(value)
    value = (value ~ (value >> 16)) & MASK32
    value = (value * 0x85ebca6b) & MASK32
    value = (value ~ (value >> 13)) & MASK32
    value = (value * 0xc2b2ae35) & MASK32
    return (value ~ (value >> 16)) & MASK32
end

function M.optimal_m(expected_items, false_positive_rate)
    require_positive_integer(expected_items, "expected_items", 2)
    require_false_positive_rate(false_positive_rate, 2)
    local logarithm = math.log(2)
    return math.ceil(
        -expected_items * math.log(false_positive_rate) / (logarithm * logarithm)
    )
end

function M.optimal_k(bit_count, expected_items)
    require_positive_integer(bit_count, "bit_count", 2)
    require_positive_integer(expected_items, "expected_items", 2)
    local rounded = math.floor((bit_count / expected_items) * math.log(2) + 0.5)
    return math.max(1, rounded)
end

function M.capacity_for_memory(memory_bytes, false_positive_rate)
    require_nonnegative_integer(memory_bytes, "memory_bytes", 2)
    require_false_positive_rate(false_positive_rate, 2)
    local logarithm = math.log(2)
    return math.floor(
        -(memory_bytes * 8) * (logarithm * logarithm)
            / math.log(false_positive_rate)
    )
end

local BloomFilter = {}
BloomFilter.__index = BloomFilter

local function from_parts(bit_count, hash_count, expected_items)
    local byte_count = (bit_count + 7) // 8
    local bits = {}
    for index = 1, byte_count do
        bits[index] = 0
    end
    return setmetatable({
        _bit_count = bit_count,
        _hash_count = hash_count,
        _expected_items = expected_items,
        _bits = bits,
        _bits_set = 0,
        _items_added = 0,
    }, BloomFilter)
end

function BloomFilter.new(expected_items, false_positive_rate)
    expected_items = expected_items == nil and DEFAULT_EXPECTED_ITEMS or expected_items
    false_positive_rate = false_positive_rate == nil
        and DEFAULT_FALSE_POSITIVE_RATE
        or false_positive_rate
    require_positive_integer(expected_items, "expected_items", 2)
    require_false_positive_rate(false_positive_rate, 2)
    local bit_count = M.optimal_m(expected_items, false_positive_rate)
    local hash_count = M.optimal_k(bit_count, expected_items)
    return from_parts(bit_count, hash_count, expected_items)
end

function BloomFilter.from_params(bit_count, hash_count)
    require_positive_integer(bit_count, "bit_count", 2)
    require_positive_integer(hash_count, "hash_count", 2)
    return from_parts(bit_count, hash_count, 0)
end

function BloomFilter:_hash_indices(element)
    local raw = element_bytes(element)
    local first = fmix32(hash_functions.fnv1a_32(raw))
    local second_word = hash_functions.djb2(raw)
    local second = fmix32((second_word ~ (second_word >> 32)) & MASK32) | 1
    local indices = {}
    for index = 0, self._hash_count - 1 do
        indices[index + 1] = (first + index * second) % self._bit_count
    end
    return indices
end

function BloomFilter:add(element)
    for _, bit_index in ipairs(self:_hash_indices(element)) do
        local byte_index = (bit_index // 8) + 1
        local bit_mask = 1 << (bit_index & 7)
        if (self._bits[byte_index] & bit_mask) == 0 then
            self._bits[byte_index] = self._bits[byte_index] | bit_mask
            self._bits_set = self._bits_set + 1
        end
    end
    self._items_added = self._items_added + 1
end

function BloomFilter:contains(element)
    for _, bit_index in ipairs(self:_hash_indices(element)) do
        local byte_index = (bit_index // 8) + 1
        local bit_mask = 1 << (bit_index & 7)
        if (self._bits[byte_index] & bit_mask) == 0 then
            return false
        end
    end
    return true
end

function BloomFilter:bit_count()
    return self._bit_count
end

function BloomFilter:hash_count()
    return self._hash_count
end

function BloomFilter:bits_set()
    return self._bits_set
end

function BloomFilter:fill_ratio()
    return self._bits_set / self._bit_count
end

function BloomFilter:estimated_false_positive_rate()
    if self._bits_set == 0 then
        return 0.0
    end
    return self:fill_ratio() ^ self._hash_count
end

function BloomFilter:is_over_capacity()
    return self._expected_items > 0 and self._items_added > self._expected_items
end

function BloomFilter:size_bytes()
    return #self._bits
end

function BloomFilter:to_string()
    return string.format(
        "BloomFilter(m=%d, k=%d, bits_set=%d/%d (%.2f%%), ~fp=%.4f%%)",
        self._bit_count,
        self._hash_count,
        self._bits_set,
        self._bit_count,
        self:fill_ratio() * 100,
        self:estimated_false_positive_rate() * 100
    )
end

BloomFilter.__tostring = BloomFilter.to_string

M.BloomFilter = BloomFilter
M.new = BloomFilter.new
M.from_params = BloomFilter.from_params
M.DEFAULT_EXPECTED_ITEMS = DEFAULT_EXPECTED_ITEMS
M.DEFAULT_FALSE_POSITIVE_RATE = DEFAULT_FALSE_POSITIVE_RATE

return M
