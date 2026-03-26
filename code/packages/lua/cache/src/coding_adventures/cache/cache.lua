local cache_set_mod = require("coding_adventures.cache.cache_set")
local CacheConfig = cache_set_mod.CacheConfig
local CacheSet = cache_set_mod.CacheSet
local CacheStats = require("coding_adventures.cache.stats")

local function integer_log2(value)
    local bits = 0
    while value > 1 do
        value = value >> 1
        bits = bits + 1
    end
    return bits
end

local function copy_bytes(bytes)
    local result = {}
    for i = 1, #bytes do
        result[i] = bytes[i]
    end
    return result
end

local function cache_access(fields)
    return fields
end

local Cache = {}
Cache.__index = Cache

function Cache.new(config)
    if getmetatable(config) ~= CacheConfig then
        error("config must be a CacheConfig", 2)
    end

    local self = setmetatable({}, Cache)
    self.config = config
    self.stats = CacheStats.new()
    self.sets = {}
    for i = 1, config.num_sets do
        self.sets[i] = CacheSet.new(config.associativity, config.line_size)
    end
    self._offset_bits = integer_log2(config.line_size)
    self._set_bits = config.num_sets > 1 and integer_log2(config.num_sets) or 0
    self._set_mask = config.num_sets - 1
    return self
end

function Cache:_decompose_address(address)
    local offset = address & ((1 << self._offset_bits) - 1)
    local set_index = (address >> self._offset_bits) & self._set_mask
    local tag = address >> (self._offset_bits + self._set_bits)
    return tag, set_index, offset
end

function Cache:_record_eviction(cache_set, evicted)
    if evicted ~= nil then
        self.stats:record_eviction(true)
        return
    end
    local all_valid = true
    for _, line in ipairs(cache_set.lines) do
        if not line.valid then
            all_valid = false
            break
        end
    end
    if all_valid then
        self.stats:record_eviction(false)
    end
end

function Cache:read(address, size, cycle)
    size = size or 1
    cycle = cycle or 0
    local tag, set_index, offset = self:_decompose_address(address)
    local cache_set = self.sets[set_index + 1]
    local hit, line = cache_set:access(tag, cycle)

    if hit then
        self.stats:record_read(true)
        return cache_access({
            address = address,
            hit = true,
            tag = tag,
            set_index = set_index,
            offset = offset,
            cycles = self.config.access_latency,
            evicted = nil,
            size = size,
        })
    end

    self.stats:record_read(false)
    local fill_data = {}
    for i = 1, self.config.line_size do
        fill_data[i] = 0
    end
    local evicted = cache_set:allocate(tag, fill_data, cycle)
    self:_record_eviction(cache_set, evicted)
    return cache_access({
        address = address,
        hit = false,
        tag = tag,
        set_index = set_index,
        offset = offset,
        cycles = self.config.access_latency,
        evicted = evicted,
        size = size,
    })
end

function Cache:write(address, data, cycle)
    cycle = cycle or 0
    data = data or {}
    local tag, set_index, offset = self:_decompose_address(address)
    local cache_set = self.sets[set_index + 1]
    local hit, line = cache_set:access(tag, cycle)

    if hit then
        self.stats:record_write(true)
        for i = 1, #data do
            local target_index = offset + i
            if target_index <= #line.data then
                line.data[target_index] = data[i]
            end
        end
        if self.config.write_policy == "write-back" then
            line.dirty = true
        end
        return cache_access({
            address = address,
            hit = true,
            tag = tag,
            set_index = set_index,
            offset = offset,
            cycles = self.config.access_latency,
            evicted = nil,
        })
    end

    self.stats:record_write(false)
    local fill_data = {}
    for i = 1, self.config.line_size do
        fill_data[i] = 0
    end
    for i = 1, #data do
        local target_index = offset + i
        if target_index <= #fill_data then
            fill_data[target_index] = data[i]
        end
    end
    local evicted = cache_set:allocate(tag, fill_data, cycle)
    self:_record_eviction(cache_set, evicted)
    local new_hit, new_line = cache_set:access(tag, cycle)
    if new_hit and self.config.write_policy == "write-back" then
        new_line.dirty = true
    end
    return cache_access({
        address = address,
        hit = false,
        tag = tag,
        set_index = set_index,
        offset = offset,
        cycles = self.config.access_latency,
        evicted = evicted,
    })
end

function Cache:invalidate()
    for _, cache_set in ipairs(self.sets) do
        for _, line in ipairs(cache_set.lines) do
            line:invalidate()
        end
    end
end

function Cache:fill_line(address, data, cycle)
    cycle = cycle or 0
    local tag, set_index = self:_decompose_address(address)
    local cache_set = self.sets[set_index + 1]
    return cache_set:allocate(tag, copy_bytes(data), cycle)
end

return {
    Cache = Cache,
    CacheAccess = cache_access,
}
