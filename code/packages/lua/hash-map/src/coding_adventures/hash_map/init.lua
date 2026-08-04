--- Hash map with separate-chaining and open-addressing collision strategies.

local hash_functions = require("coding_adventures.hash_functions")

local M = {}

local DEFAULT_CAPACITY = 16
local EMPTY = {}
local TOMBSTONE = {}

local function require_positive_integer(value, name, level)
    if type(value) ~= "number" or math.type(value) ~= "integer" or value <= 0 then
        error(name .. " must be a positive integer", level or 3)
    end
    return value
end

local function normalize_strategy(strategy, level)
    strategy = strategy == nil and "chaining" or strategy
    if strategy == "chaining" then
        return strategy
    end
    if strategy == "open_addressing"
        or strategy == "open-addressing"
        or strategy == "open"
    then
        return "open_addressing"
    end
    error("strategy must be 'chaining' or 'open_addressing'", level or 3)
end

local function normalize_hash_fn(hash_fn, level)
    hash_fn = hash_fn == nil and "fnv1a" or hash_fn
    if hash_fn == "fnv1a" or hash_fn == "fnv1a_32" then
        return "fnv1a"
    end
    if hash_fn == "murmur3" or hash_fn == "murmur3_32" then
        return "murmur3"
    end
    if hash_fn == "djb2" then
        return hash_fn
    end
    error("hash_fn must be 'fnv1a', 'murmur3', or 'djb2'", level or 3)
end

local function serialize_key(key)
    local kind = type(key)
    if kind == "string" then
        return "string:" .. key
    end
    if kind == "number" then
        return "number:" .. tostring(key)
    end
    if kind == "boolean" then
        return key and "boolean:true" or "boolean:false"
    end
    return kind .. ":" .. tostring(key)
end

local function apply_hash(data, hash_fn)
    if hash_fn == "murmur3" then
        return hash_functions.murmur3_32(data)
    end
    if hash_fn == "djb2" then
        return hash_functions.djb2(data)
    end
    return hash_functions.fnv1a_32(data)
end

local HashMap = {}
HashMap.__index = HashMap

local function initialize_storage(map)
    if map._strategy == "chaining" then
        map._buckets = {}
        for index = 1, map._capacity do
            map._buckets[index] = {}
        end
        map._slots = nil
    else
        map._slots = {}
        for index = 1, map._capacity do
            map._slots[index] = EMPTY
        end
        map._buckets = nil
    end
end

function HashMap.new(capacity, strategy, hash_fn)
    capacity = capacity == nil and DEFAULT_CAPACITY or capacity
    require_positive_integer(capacity, "capacity", 2)
    local map = setmetatable({
        _capacity = capacity,
        _size = 0,
        _strategy = normalize_strategy(strategy, 2),
        _hash_fn = normalize_hash_fn(hash_fn, 2),
    }, HashMap)
    initialize_storage(map)
    return map
end

function HashMap:_bucket_index(key)
    return (apply_hash(serialize_key(key), self._hash_fn) % self._capacity) + 1
end

function HashMap:_set_chaining(key, value)
    local bucket = self._buckets[self:_bucket_index(key)]
    for _, entry in ipairs(bucket) do
        if entry.key == key then
            entry.value = value
            return
        end
    end
    bucket[#bucket + 1] = { key = key, value = value }
    self._size = self._size + 1
end

function HashMap:_set_open_addressing(key, value)
    local start = self:_bucket_index(key)
    local first_tombstone = nil
    for probe = 0, self._capacity - 1 do
        local index = ((start - 1 + probe) % self._capacity) + 1
        local slot = self._slots[index]
        if slot == EMPTY then
            local insert_at = first_tombstone or index
            self._slots[insert_at] = { key = key, value = value }
            self._size = self._size + 1
            return
        elseif slot == TOMBSTONE then
            first_tombstone = first_tombstone or index
        elseif slot.key == key then
            slot.value = value
            return
        end
    end
    if first_tombstone then
        self._slots[first_tombstone] = { key = key, value = value }
        self._size = self._size + 1
        return
    end
    error("hash map is full; resize should have happened earlier", 2)
end

function HashMap:_set_without_resize(key, value)
    if self._strategy == "chaining" then
        self:_set_chaining(key, value)
    else
        self:_set_open_addressing(key, value)
    end
end

function HashMap:_needs_resize()
    local threshold = self._strategy == "chaining" and 1.0 or 0.75
    return self:load_factor() > threshold
end

function HashMap:_resize(new_capacity)
    local old_entries = self:entries()
    self._capacity = new_capacity
    self._size = 0
    initialize_storage(self)
    for _, entry in ipairs(old_entries) do
        self:_set_without_resize(entry[1], entry[2])
    end
end

--- Insert or update a key in place and return this map.
function HashMap:set(key, value)
    if key == nil then
        error("key must not be nil", 2)
    end
    self:_set_without_resize(key, value)
    if self:_needs_resize() then
        self:_resize(self._capacity * 2)
    end
    return self
end

function HashMap:get(key)
    if key == nil then
        return nil
    end
    local start = self:_bucket_index(key)
    if self._strategy == "chaining" then
        for _, entry in ipairs(self._buckets[start]) do
            if entry.key == key then
                return entry.value
            end
        end
        return nil
    end
    for probe = 0, self._capacity - 1 do
        local index = ((start - 1 + probe) % self._capacity) + 1
        local slot = self._slots[index]
        if slot == EMPTY then
            return nil
        end
        if slot ~= TOMBSTONE and slot.key == key then
            return slot.value
        end
    end
    return nil
end

function HashMap:has(key)
    if key == nil then
        return false
    end
    local start = self:_bucket_index(key)
    if self._strategy == "chaining" then
        for _, entry in ipairs(self._buckets[start]) do
            if entry.key == key then
                return true
            end
        end
        return false
    end
    for probe = 0, self._capacity - 1 do
        local index = ((start - 1 + probe) % self._capacity) + 1
        local slot = self._slots[index]
        if slot == EMPTY then
            return false
        end
        if slot ~= TOMBSTONE and slot.key == key then
            return true
        end
    end
    return false
end

function HashMap:delete(key)
    if key == nil then
        return false
    end
    local start = self:_bucket_index(key)
    if self._strategy == "chaining" then
        local bucket = self._buckets[start]
        for index, entry in ipairs(bucket) do
            if entry.key == key then
                table.remove(bucket, index)
                self._size = self._size - 1
                return true
            end
        end
        return false
    end
    for probe = 0, self._capacity - 1 do
        local index = ((start - 1 + probe) % self._capacity) + 1
        local slot = self._slots[index]
        if slot == EMPTY then
            return false
        end
        if slot ~= TOMBSTONE and slot.key == key then
            self._slots[index] = TOMBSTONE
            self._size = self._size - 1
            return true
        end
    end
    return false
end

function HashMap:entries()
    local result = {}
    if self._strategy == "chaining" then
        for _, bucket in ipairs(self._buckets) do
            for _, entry in ipairs(bucket) do
                result[#result + 1] = { entry.key, entry.value, n = 2 }
            end
        end
    else
        for _, slot in ipairs(self._slots) do
            if slot ~= EMPTY and slot ~= TOMBSTONE then
                result[#result + 1] = { slot.key, slot.value, n = 2 }
            end
        end
    end
    return result
end

function HashMap:keys()
    local result = {}
    for _, entry in ipairs(self:entries()) do
        result[#result + 1] = entry[1]
    end
    return result
end

function HashMap:values()
    local result = { n = 0 }
    for _, entry in ipairs(self:entries()) do
        result.n = result.n + 1
        result[result.n] = entry[2]
    end
    return result
end

function HashMap:size()
    return self._size
end

function HashMap:capacity()
    return self._capacity
end

function HashMap:load_factor()
    return self._size / self._capacity
end

function HashMap:strategy()
    return self._strategy
end

function HashMap:hash_fn()
    return self._hash_fn
end

function HashMap:clone()
    local copy = HashMap.new(self._capacity, self._strategy, self._hash_fn)
    for _, entry in ipairs(self:entries()) do
        copy:_set_without_resize(entry[1], entry[2])
    end
    return copy
end

function HashMap:clear()
    self._size = 0
    initialize_storage(self)
    return self
end

HashMap.__len = HashMap.size
HashMap.__tostring = function(self)
    return string.format(
        "HashMap(strategy=%s, hash_fn=%s, size=%d, capacity=%d)",
        self._strategy,
        self._hash_fn,
        self._size,
        self._capacity
    )
end

function M.from_entries(entries, capacity, strategy, hash_fn)
    if type(entries) ~= "table" then
        error("entries must be an array of key-value pairs", 2)
    end
    local map = HashMap.new(capacity, strategy, hash_fn)
    for _, entry in ipairs(entries) do
        if type(entry) ~= "table" or entry[1] == nil then
            error("each entry must be a key-value pair with a non-nil key", 2)
        end
        map:set(entry[1], entry[2])
    end
    return map
end

function M.merge(left, right)
    local result = HashMap.new(
        math.max(left:capacity(), right:capacity()),
        left:strategy(),
        left:hash_fn()
    )
    for _, entry in ipairs(left:entries()) do
        result:set(entry[1], entry[2])
    end
    for _, entry in ipairs(right:entries()) do
        result:set(entry[1], entry[2])
    end
    return result
end

--- Functional wrappers clone before writes, leaving the input map unchanged.
function M.set(map, key, value)
    return map:clone():set(key, value)
end

function M.delete(map, key)
    local copy = map:clone()
    copy:delete(key)
    return copy
end

function M.get(map, key) return map:get(key) end
function M.has(map, key) return map:has(key) end
function M.keys(map) return map:keys() end
function M.values(map) return map:values() end
function M.entries(map) return map:entries() end
function M.size(map) return map:size() end
function M.capacity(map) return map:capacity() end
function M.load_factor(map) return map:load_factor() end

M.HashMap = HashMap
M.new = HashMap.new
M.new_map = HashMap.new
M.DEFAULT_CAPACITY = DEFAULT_CAPACITY

return M
