--- A dependency-free HyperLogLog cardinality estimator.
---
--- Values are converted to strings and hashed into two independently seeded
--- 32-bit FNV-1a words. Keeping the words separate avoids depending on
--- floating-point representations of unsigned 64-bit integers.

local HyperLogLog = {}
HyperLogLog.__index = HyperLogLog

local UINT32_MASK = 0xffffffff
local FNV_OFFSET = 2166136261
local FNV_PRIME = 16777619
local SECOND_SEED = (FNV_OFFSET ~ 0x9e3779b9) & UINT32_MASK

local function validate_precision(precision)
    if type(precision) ~= "number"
        or precision ~= math.floor(precision)
        or precision < 4
        or precision > 16
    then
        error("precision must be an integer between 4 and 16", 3)
    end
end

local function avalanche32(hash)
    hash = (hash ~ (hash >> 16)) & UINT32_MASK
    hash = (hash * 1597334677) & UINT32_MASK
    hash = (hash ~ (hash >> 15)) & UINT32_MASK
    hash = (hash * 1226822519) & UINT32_MASK
    return (hash ~ (hash >> 16)) & UINT32_MASK
end

local function fnv1a32(payload, seed, reverse)
    local hash = seed
    if reverse then
        for index = #payload, 1, -1 do
            hash = ((hash ~ string.byte(payload, index)) * FNV_PRIME) & UINT32_MASK
        end
    else
        for index = 1, #payload do
            hash = ((hash ~ string.byte(payload, index)) * FNV_PRIME) & UINT32_MASK
        end
    end
    hash = ((hash ~ (#payload & 0xff)) * FNV_PRIME) & UINT32_MASK
    return avalanche32(hash)
end

local function hash64(value)
    local payload = tostring(value)
    return fnv1a32(payload, FNV_OFFSET, false), fnv1a32(payload, SECOND_SEED, true)
end

local function leading_zeros32(value)
    if value == 0 then
        return 32
    end
    local zeros = 0
    local bit = 0x80000000
    while (value & bit) == 0 do
        zeros = zeros + 1
        bit = bit >> 1
    end
    return zeros
end

local function alpha(register_count)
    if register_count == 16 then
        return 0.673
    elseif register_count == 32 then
        return 0.697
    elseif register_count == 64 then
        return 0.709
    end
    return 0.7213 / (1 + 1.079 / register_count)
end

function HyperLogLog.new(precision)
    precision = precision == nil and 10 or precision
    validate_precision(precision)

    local register_count = 1 << precision
    local registers = {}
    for index = 1, register_count do
        registers[index] = 0
    end

    return setmetatable({
        _precision = precision,
        _register_count = register_count,
        _registers = registers,
    }, HyperLogLog)
end

function HyperLogLog:add(value)
    local high, low = hash64(value)
    local bucket = (high & (self._register_count - 1)) + 1
    local upper = high >> self._precision
    local upper_width = 32 - self._precision
    local zero_count

    if upper ~= 0 then
        zero_count = leading_zeros32(upper) - self._precision
    else
        zero_count = upper_width + leading_zeros32(low)
    end

    local rank = zero_count + 1
    if rank > self._registers[bucket] then
        self._registers[bucket] = rank
    end
    return self
end

function HyperLogLog:count()
    local indicator = 0.0
    local empty_registers = 0
    for index = 1, self._register_count do
        local register = self._registers[index]
        indicator = indicator + (2.0 ^ (-register))
        if register == 0 then
            empty_registers = empty_registers + 1
        end
    end

    local m = self._register_count
    local estimate = alpha(m) * m * m / indicator
    if estimate <= 2.5 * m and empty_registers > 0 then
        estimate = m * math.log(m / empty_registers)
    end
    return math.floor(estimate + 0.5)
end

function HyperLogLog:merge(other)
    local merged = HyperLogLog.new(self._precision)
    for index = 1, self._register_count do
        merged._registers[index] = self._registers[index]
    end
    return merged:merge_in_place(other)
end

function HyperLogLog:merge_in_place(other)
    if getmetatable(other) ~= HyperLogLog then
        error("other must be a HyperLogLog", 2)
    end
    if other._precision ~= self._precision then
        error("cannot merge HyperLogLog sketches with different precisions", 2)
    end
    for index = 1, self._register_count do
        local candidate = other._registers[index]
        if candidate > self._registers[index] then
            self._registers[index] = candidate
        end
    end
    return self
end

function HyperLogLog:clear()
    for index = 1, self._register_count do
        self._registers[index] = 0
    end
    return self
end

function HyperLogLog:is_empty()
    for index = 1, self._register_count do
        if self._registers[index] ~= 0 then
            return false
        end
    end
    return true
end

function HyperLogLog:precision()
    return self._precision
end

function HyperLogLog:num_registers()
    return self._register_count
end

function HyperLogLog:error_rate()
    return 1.04 / math.sqrt(self._register_count)
end

function HyperLogLog:memory_bytes()
    return math.floor(self._register_count * 6 / 8)
end

function HyperLogLog:registers()
    local snapshot = {}
    for index = 1, self._register_count do
        snapshot[index] = self._registers[index]
    end
    return snapshot
end

HyperLogLog.__len = function(self)
    return self:count()
end

HyperLogLog.__tostring = function(self)
    return string.format(
        "HyperLogLog(precision=%d, registers=%d, estimate=%d)",
        self._precision,
        self._register_count,
        self:count()
    )
end

return {
    HyperLogLog = HyperLogLog,
    new = HyperLogLog.new,
}
