local CacheStats = {}
CacheStats.__index = CacheStats

function CacheStats.new()
    local self = setmetatable({}, CacheStats)
    self.reads = 0
    self.writes = 0
    self.hits = 0
    self.misses = 0
    self.evictions = 0
    self.writebacks = 0
    return self
end

function CacheStats:total_accesses()
    return self.reads + self.writes
end

function CacheStats:hit_rate()
    local total = self:total_accesses()
    if total == 0 then
        return 0.0
    end
    return self.hits / total
end

function CacheStats:miss_rate()
    local total = self:total_accesses()
    if total == 0 then
        return 0.0
    end
    return self.misses / total
end

function CacheStats:record_read(hit)
    self.reads = self.reads + 1
    if hit then
        self.hits = self.hits + 1
    else
        self.misses = self.misses + 1
    end
end

function CacheStats:record_write(hit)
    self.writes = self.writes + 1
    if hit then
        self.hits = self.hits + 1
    else
        self.misses = self.misses + 1
    end
end

function CacheStats:record_eviction(dirty)
    self.evictions = self.evictions + 1
    if dirty then
        self.writebacks = self.writebacks + 1
    end
end

function CacheStats:reset()
    self.reads = 0
    self.writes = 0
    self.hits = 0
    self.misses = 0
    self.evictions = 0
    self.writebacks = 0
end

return CacheStats
