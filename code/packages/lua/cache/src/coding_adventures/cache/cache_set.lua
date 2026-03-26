local CacheLine = require("coding_adventures.cache.cache_line")

local local_floor = math.floor

local function is_power_of_two(value)
    return value > 0 and (value & (value - 1)) == 0
end

local CacheConfig = {}
CacheConfig.__index = CacheConfig

function CacheConfig.new(name, total_size, line_size, associativity, access_latency, write_policy)
    line_size = line_size or 64
    associativity = associativity or 4
    access_latency = access_latency or 1
    write_policy = write_policy or "write-back"

    if type(name) ~= "string" or name == "" then
        error("name must be a non-empty string", 2)
    end
    if type(total_size) ~= "number" or total_size <= 0 or total_size ~= local_floor(total_size) then
        error("total_size must be a positive integer", 2)
    end
    if type(line_size) ~= "number" or line_size ~= local_floor(line_size) or not is_power_of_two(line_size) then
        error("line_size must be a positive power of 2", 2)
    end
    if type(associativity) ~= "number" or associativity <= 0 or associativity ~= local_floor(associativity) then
        error("associativity must be a positive integer", 2)
    end
    if total_size % (line_size * associativity) ~= 0 then
        error("total_size must be divisible by line_size * associativity", 2)
    end
    if write_policy ~= "write-back" and write_policy ~= "write-through" then
        error("write_policy must be 'write-back' or 'write-through'", 2)
    end
    if type(access_latency) ~= "number" or access_latency < 0 or access_latency ~= local_floor(access_latency) then
        error("access_latency must be non-negative", 2)
    end

    local self = setmetatable({}, CacheConfig)
    self.name = name
    self.total_size = total_size
    self.line_size = line_size
    self.associativity = associativity
    self.access_latency = access_latency
    self.write_policy = write_policy
    self.num_lines = total_size // line_size
    self.num_sets = self.num_lines // associativity
    return self
end

local CacheSet = {}
CacheSet.__index = CacheSet

function CacheSet.new(associativity, line_size)
    local self = setmetatable({}, CacheSet)
    self.lines = {}
    for i = 1, associativity do
        self.lines[i] = CacheLine.new(line_size)
    end
    return self
end

function CacheSet:lookup(tag)
    for index, line in ipairs(self.lines) do
        if line.valid and line.tag == tag then
            return true, index
        end
    end
    return false, nil
end

function CacheSet:_find_lru()
    local best_index = 1
    local best_time = math.huge
    for index, line in ipairs(self.lines) do
        if not line.valid then
            return index
        end
        if line.last_access < best_time then
            best_time = line.last_access
            best_index = index
        end
    end
    return best_index
end

function CacheSet:access(tag, cycle)
    local hit, index = self:lookup(tag)
    if hit then
        local line = self.lines[index]
        line:touch(cycle)
        return true, line
    end
    return false, self.lines[self:_find_lru()]
end

function CacheSet:allocate(tag, data, cycle)
    for _, line in ipairs(self.lines) do
        if not line.valid then
            line:fill(tag, data, cycle)
            return nil
        end
    end

    local victim = self.lines[self:_find_lru()]
    local evicted = nil
    if victim.dirty then
        evicted = victim:clone()
    end
    victim:fill(tag, data, cycle)
    return evicted
end

return {
    CacheConfig = CacheConfig,
    CacheSet = CacheSet,
}
