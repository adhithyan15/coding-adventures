--- Persistent hash set built on the DT18 hash map package.

local hash_map = require("coding_adventures.hash_map")

local M = {}
local PRESENT = true

local HashSet = {}
HashSet.__index = HashSet

local function wrap(map)
    return setmetatable({ _map = map }, HashSet)
end

local function require_elements(elements, level)
    if type(elements) ~= "table" then
        error("elements must be an array", level or 3)
    end
    return elements
end

local function require_element(element, level)
    if element == nil then
        error("element must not be nil", level or 3)
    end
    return element
end

local function empty_like(set)
    return wrap(hash_map.new(set:capacity(), set:strategy(), set:hash_fn()))
end

function HashSet.new(capacity, strategy, hash_fn)
    return wrap(hash_map.new(capacity, strategy, hash_fn))
end

function HashSet.with_options(capacity, strategy, hash_fn)
    return HashSet.new(capacity, strategy, hash_fn)
end

function HashSet.from_list(elements, capacity, strategy, hash_fn)
    require_elements(elements, 2)
    local map = hash_map.new(capacity, strategy, hash_fn)
    for _, element in ipairs(elements) do
        map:set(require_element(element, 2), PRESENT)
    end
    return wrap(map)
end

function HashSet.from_list_with_options(elements, capacity, strategy, hash_fn)
    return HashSet.from_list(elements, capacity, strategy, hash_fn)
end

function HashSet:clone()
    return wrap(self._map:clone())
end

--- Return a new set containing element, leaving this set unchanged.
function HashSet:add(element)
    require_element(element, 2)
    return wrap(hash_map.set(self._map, element, PRESENT))
end

--- Return a new set without element. Missing elements are ignored.
function HashSet:remove(element)
    return wrap(hash_map.delete(self._map, element))
end

HashSet.discard = HashSet.remove

function HashSet:contains(element)
    return self._map:has(element)
end

HashSet.has = HashSet.contains

function HashSet:size()
    return self._map:size()
end

HashSet.len = HashSet.size

function HashSet:is_empty()
    return self:size() == 0
end

function HashSet:to_list()
    return self._map:keys()
end

function HashSet:capacity()
    return self._map:capacity()
end

function HashSet:strategy()
    return self._map:strategy()
end

function HashSet:hash_fn()
    return self._map:hash_fn()
end

function HashSet:union(other)
    local result = self:clone()
    for _, element in ipairs(other:to_list()) do
        result._map:set(element, PRESENT)
    end
    return result
end

function HashSet:intersection(other)
    local smaller = self:size() <= other:size() and self or other
    local larger = smaller == self and other or self
    local result = empty_like(self)
    for _, element in ipairs(smaller:to_list()) do
        if larger:contains(element) then
            result._map:set(element, PRESENT)
        end
    end
    return result
end

function HashSet:difference(other)
    local result = empty_like(self)
    for _, element in ipairs(self:to_list()) do
        if not other:contains(element) then
            result._map:set(element, PRESENT)
        end
    end
    return result
end

function HashSet:symmetric_difference(other)
    local result = empty_like(self)
    for _, element in ipairs(self:to_list()) do
        if not other:contains(element) then
            result._map:set(element, PRESENT)
        end
    end
    for _, element in ipairs(other:to_list()) do
        if not self:contains(element) then
            result._map:set(element, PRESENT)
        end
    end
    return result
end

function HashSet:is_subset(other)
    if self:size() > other:size() then
        return false
    end
    for _, element in ipairs(self:to_list()) do
        if not other:contains(element) then
            return false
        end
    end
    return true
end

function HashSet:is_superset(other)
    return other:is_subset(self)
end

function HashSet:is_disjoint(other)
    local smaller = self:size() <= other:size() and self or other
    local larger = smaller == self and other or self
    for _, element in ipairs(smaller:to_list()) do
        if larger:contains(element) then
            return false
        end
    end
    return true
end

function HashSet:equals(other)
    return self:size() == other:size() and self:is_subset(other)
end

HashSet.__len = HashSet.size
HashSet.__tostring = function(self)
    return string.format(
        "HashSet(strategy=%s, hash_fn=%s, size=%d, capacity=%d)",
        self:strategy(),
        self:hash_fn(),
        self:size(),
        self:capacity()
    )
end

function M.add(set, element) return set:add(element) end
function M.remove(set, element) return set:remove(element) end
function M.discard(set, element) return set:discard(element) end
function M.contains(set, element) return set:contains(element) end
function M.has(set, element) return set:has(element) end
function M.size(set) return set:size() end
function M.is_empty(set) return set:is_empty() end
function M.to_list(set) return set:to_list() end
function M.union(set, other) return set:union(other) end
function M.intersection(set, other) return set:intersection(other) end
function M.difference(set, other) return set:difference(other) end
function M.symmetric_difference(set, other) return set:symmetric_difference(other) end
function M.is_subset(set, other) return set:is_subset(other) end
function M.is_superset(set, other) return set:is_superset(other) end
function M.is_disjoint(set, other) return set:is_disjoint(other) end
function M.equals(set, other) return set:equals(other) end

M.HashSet = HashSet
M.new = HashSet.new
M.new_set = HashSet.new
M.with_options = HashSet.with_options
M.from_list = HashSet.from_list
M.from_list_with_options = HashSet.from_list_with_options

return M
