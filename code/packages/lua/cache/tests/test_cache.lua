package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local cache = require("coding_adventures.cache")

local Cache = cache.Cache
local CacheConfig = cache.CacheConfig
local CacheHierarchy = cache.CacheHierarchy
local CacheLine = cache.CacheLine
local CacheSet = cache.CacheSet
local CacheStats = cache.CacheStats

describe("CacheLine", function()
    it("starts invalid and can be filled", function()
        local line = CacheLine.new(8)
        assert.is_false(line.valid)
        line:fill(42, { 1, 2, 3, 4, 5, 6, 7, 8 }, 9)
        assert.is_true(line.valid)
        assert.are.equal(42, line.tag)
        assert.are.equal(9, line.last_access)
        assert.are.same({ 1, 2, 3, 4, 5, 6, 7, 8 }, line.data)
    end)
end)

describe("CacheConfig", function()
    it("computes line and set counts", function()
        local config = CacheConfig.new("L1D", 1024, 64, 4, 1)
        assert.are.equal(16, config.num_lines)
        assert.are.equal(4, config.num_sets)
    end)

    it("rejects invalid line sizes", function()
        assert.has_error(function()
            CacheConfig.new("bad", 256, 48, 1, 1)
        end)
    end)
end)

describe("CacheSet", function()
    it("looks up filled lines", function()
        local cache_set = CacheSet.new(2, 8)
        cache_set.lines[1]:fill(10, { 0, 0, 0, 0, 0, 0, 0, 0 }, 1)
        local hit, way = cache_set:lookup(10)
        assert.is_true(hit)
        assert.are.equal(1, way)
    end)

    it("returns dirty evictions", function()
        local cache_set = CacheSet.new(1, 8)
        cache_set:allocate(10, { 1, 1, 1, 1, 1, 1, 1, 1 }, 1)
        cache_set.lines[1].dirty = true
        local evicted = cache_set:allocate(20, { 0, 0, 0, 0, 0, 0, 0, 0 }, 2)
        assert.is_not_nil(evicted)
        assert.is_true(evicted.dirty)
        assert.are.equal(10, evicted.tag)
    end)
end)

describe("CacheStats", function()
    it("tracks hit and miss rates", function()
        local stats = CacheStats.new()
        stats:record_read(true)
        stats:record_write(false)
        assert.are.equal(2, stats:total_accesses())
        assert.are.equal(0.5, stats:hit_rate())
        assert.are.equal(0.5, stats:miss_rate())
    end)
end)

describe("Cache", function()
    local function make_cache(write_policy)
        return Cache.new(CacheConfig.new("test", 256, 64, 2, 3, write_policy or "write-back"))
    end

    it("decomposes addresses into tag, set, and offset", function()
        local c = make_cache()
        local tag, set_index, offset = c:_decompose_address(0x100)
        assert.are.equal(2, tag)
        assert.are.equal(0, set_index)
        assert.are.equal(0, offset)
    end)

    it("misses on the first read and hits on the second", function()
        local c = make_cache()
        local first = c:read(0x100, 1, 0)
        local second = c:read(0x100, 1, 1)
        assert.is_false(first.hit)
        assert.is_true(second.hit)
        assert.are.equal(2, c.stats.reads)
        assert.are.equal(1, c.stats.hits)
        assert.are.equal(1, c.stats.misses)
    end)

    it("writes data and marks lines dirty for write-back", function()
        local c = make_cache("write-back")
        c:write(0x100, { 0xDE, 0xAD }, 0)
        local tag, set_index, offset = c:_decompose_address(0x100)
        local hit, way = c.sets[set_index + 1]:lookup(tag)
        assert.is_true(hit)
        local line = c.sets[set_index + 1].lines[way]
        assert.are.equal(0xDE, line.data[offset + 1])
        assert.are.equal(0xAD, line.data[offset + 2])
        assert.is_true(line.dirty)
    end)

    it("keeps lines clean for write-through", function()
        local c = make_cache("write-through")
        c:write(0x100, { 0xAB }, 0)
        local tag, set_index = c:_decompose_address(0x100)
        local _, way = c.sets[set_index + 1]:lookup(tag)
        assert.is_false(c.sets[set_index + 1].lines[way].dirty)
    end)

    it("reports dirty evictions", function()
        local c = Cache.new(CacheConfig.new("tiny", 64, 64, 1, 1, "write-back"))
        c:write(0, { 0xFF }, 0)
        local access = c:read(64, 1, 1)
        assert.is_false(access.hit)
        assert.is_not_nil(access.evicted)
        assert.is_true(access.evicted.dirty)
    end)
end)

describe("CacheHierarchy", function()
    local function make_l1d()
        return Cache.new(CacheConfig.new("L1D", 256, 64, 2, 1))
    end

    local function make_l2()
        return Cache.new(CacheConfig.new("L2", 1024, 64, 4, 10))
    end

    it("goes to memory on a cold read and then fills L1", function()
        local hierarchy = CacheHierarchy.new({
            l1d = make_l1d(),
            l2 = make_l2(),
            main_memory_latency = 100,
        })
        local first = hierarchy:read(0x1000, false, 0)
        local second = hierarchy:read(0x1000, false, 1)
        assert.are.equal("memory", first.served_by)
        assert.are.equal(111, first.total_cycles)
        assert.are.equal("L1D", second.served_by)
        assert.are.equal(1, second.total_cycles)
    end)

    it("hits L2 when L1 misses and L2 is primed", function()
        local l1d = make_l1d()
        local l2 = make_l2()
        l2:fill_line(0x1000, { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }, 0)
        local hierarchy = CacheHierarchy.new({
            l1d = l1d,
            l2 = l2,
            main_memory_latency = 100,
        })
        local result = hierarchy:read(0x1000, false, 1)
        assert.are.equal("L2", result.served_by)
        assert.are.equal(11, result.total_cycles)
        local follow_up = hierarchy:read(0x1000, false, 2)
        assert.are.equal("L1D", follow_up.served_by)
    end)

    it("writes through L1 when already cached", function()
        local l1d = make_l1d()
        local hierarchy = CacheHierarchy.new({ l1d = l1d, main_memory_latency = 100 })
        hierarchy:read(0x1000, false, 0)
        local result = hierarchy:write(0x1000, { 0xAB }, 1)
        assert.are.equal("L1D", result.served_by)
        assert.are.equal(1, result.total_cycles)
    end)

    it("can invalidate caches and reset stats", function()
        local l1d = make_l1d()
        local l2 = make_l2()
        local hierarchy = CacheHierarchy.new({ l1d = l1d, l2 = l2, main_memory_latency = 100 })
        hierarchy:read(0x1000, false, 0)
        hierarchy:reset_stats()
        assert.are.equal(0, l1d.stats:total_accesses())
        assert.are.equal(0, l2.stats:total_accesses())
        hierarchy:invalidate_all()
        local result = hierarchy:read(0x1000, false, 1)
        assert.are.equal("memory", result.served_by)
    end)
end)
