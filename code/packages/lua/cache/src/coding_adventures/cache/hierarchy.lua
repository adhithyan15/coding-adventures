local cache_mod = require("coding_adventures.cache.cache")

local Cache = cache_mod.Cache

local function hierarchy_access(fields)
    return fields
end

local CacheHierarchy = {}
CacheHierarchy.__index = CacheHierarchy

local function append_level(levels, name, cache)
    if cache ~= nil then
        table.insert(levels, { name = name, cache = cache })
    end
end

function CacheHierarchy.new(opts)
    opts = opts or {}
    local self = setmetatable({}, CacheHierarchy)
    self.l1i = opts.l1i
    self.l1d = opts.l1d
    self.l2 = opts.l2
    self.l3 = opts.l3
    self.main_memory_latency = opts.main_memory_latency or 100
    self._data_levels = {}
    self._instr_levels = {}
    append_level(self._data_levels, "L1D", self.l1d)
    append_level(self._data_levels, "L2", self.l2)
    append_level(self._data_levels, "L3", self.l3)
    append_level(self._instr_levels, "L1I", self.l1i)
    append_level(self._instr_levels, "L2", self.l2)
    append_level(self._instr_levels, "L3", self.l3)
    return self
end

function CacheHierarchy:_get_levels(is_instruction)
    if is_instruction then
        return self._instr_levels
    end
    return self._data_levels
end

function CacheHierarchy:_line_size(levels)
    if #levels == 0 then
        return 64
    end
    return levels[1].cache.config.line_size
end

function CacheHierarchy:read(address, is_instruction, cycle)
    is_instruction = is_instruction or false
    cycle = cycle or 0
    local levels = self:_get_levels(is_instruction)
    if #levels == 0 then
        return hierarchy_access({
            address = address,
            served_by = "memory",
            total_cycles = self.main_memory_latency,
            hit_at_level = 0,
            level_accesses = {},
        })
    end

    local total_cycles = 0
    local accesses = {}
    local served_by = "memory"
    local hit_level = #levels

    for index, level in ipairs(levels) do
        local access = level.cache:read(address, 1, cycle)
        total_cycles = total_cycles + level.cache.config.access_latency
        accesses[#accesses + 1] = access
        if access.hit then
            served_by = level.name
            hit_level = index - 1
            break
        end
    end

    if served_by == "memory" then
        total_cycles = total_cycles + self.main_memory_latency
    end

    local fill_data = {}
    for i = 1, self:_line_size(levels) do
        fill_data[i] = 0
    end
    for fill_index = hit_level, 1, -1 do
        levels[fill_index].cache:fill_line(address, fill_data, cycle)
    end

    return hierarchy_access({
        address = address,
        served_by = served_by,
        total_cycles = total_cycles,
        hit_at_level = hit_level,
        level_accesses = accesses,
    })
end

function CacheHierarchy:write(address, data, cycle)
    cycle = cycle or 0
    data = data or {}
    local levels = self._data_levels
    if #levels == 0 then
        return hierarchy_access({
            address = address,
            served_by = "memory",
            total_cycles = self.main_memory_latency,
            hit_at_level = 0,
            level_accesses = {},
        })
    end

    local first = levels[1]
    local access = first.cache:write(address, data, cycle)
    if access.hit then
        return hierarchy_access({
            address = address,
            served_by = first.name,
            total_cycles = first.cache.config.access_latency,
            hit_at_level = 0,
            level_accesses = { access },
        })
    end

    local total_cycles = first.cache.config.access_latency
    local accesses = { access }
    local served_by = "memory"
    local hit_level = #levels

    for index = 2, #levels do
        local level = levels[index]
        local level_access = level.cache:read(address, 1, cycle)
        total_cycles = total_cycles + level.cache.config.access_latency
        accesses[#accesses + 1] = level_access
        if level_access.hit then
            served_by = level.name
            hit_level = index - 1
            break
        end
    end

    if served_by == "memory" then
        total_cycles = total_cycles + self.main_memory_latency
    end

    return hierarchy_access({
        address = address,
        served_by = served_by,
        total_cycles = total_cycles,
        hit_at_level = hit_level,
        level_accesses = accesses,
    })
end

function CacheHierarchy:invalidate_all()
    for _, cache in pairs({ self.l1i, self.l1d, self.l2, self.l3 }) do
        if cache ~= nil then
            cache:invalidate()
        end
    end
end

function CacheHierarchy:reset_stats()
    for _, cache in pairs({ self.l1i, self.l1d, self.l2, self.l3 }) do
        if cache ~= nil then
            cache.stats:reset()
        end
    end
end

return {
    CacheHierarchy = CacheHierarchy,
    HierarchyAccess = hierarchy_access,
}
