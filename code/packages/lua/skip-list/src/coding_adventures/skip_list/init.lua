local M = {VERSION = "0.1.0"}

local MODULUS = 2147483647
local MULTIPLIER = 48271

local function default_compare(left, right)
    if left == right then
        return 0
    end
    if left < right then
        return -1
    end
    return 1
end

local function new_node(key, value, height)
    local spans = {}
    for level = 1, height do
        spans[level] = 0
    end
    return {
        key = key,
        value = value,
        height = height,
        forward = {},
        span = spans,
    }
end

local SkipList = {}
SkipList.__index = SkipList

function SkipList.new(max_level, probability, compare, seed)
    local levels = max_level or 16
    local promotion = probability or 0.5
    local comparator = compare or default_compare
    local initial_seed = seed or 1

    if type(levels) ~= "number" or levels < 1 or levels % 1 ~= 0 then
        error("max_level must be a positive integer", 2)
    end
    if type(promotion) ~= "number" or promotion <= 0 or promotion >= 1 then
        error("probability must be between 0 and 1", 2)
    end
    if type(comparator) ~= "function" then
        error("compare must be a function", 2)
    end
    if type(initial_seed) ~= "number" or initial_seed % 1 ~= 0 then
        error("seed must be an integer", 2)
    end

    local normalized_seed = math.floor(math.abs(initial_seed)) % (MODULUS - 1) + 1
    local head = new_node(nil, nil, levels)
    return setmetatable({
        _head = head,
        _max_level = levels,
        _probability = promotion,
        _compare = comparator,
        _rng_state = normalized_seed,
        _current_level = 1,
        _size = 0,
    }, SkipList)
end

function SkipList.from_entries(entries, max_level, probability, compare, seed)
    if type(entries) ~= "table" then
        error("entries must be a table", 2)
    end
    local list = SkipList.new(max_level, probability, compare, seed)
    for index, entry in ipairs(entries) do
        if type(entry) ~= "table" then
            error("entry at index " .. index .. " must be a table", 2)
        end
        if entry[1] == nil then
            error("key at index " .. index .. " must not be nil", 2)
        end
        list:insert(entry[1], entry[2])
    end
    return list
end

function SkipList:_random()
    self._rng_state = (self._rng_state * MULTIPLIER) % MODULUS
    return self._rng_state / MODULUS
end

function SkipList:_random_level()
    local level = 1
    while level < self._max_level and self:_random() < self._probability do
        level = level + 1
    end
    return level
end

function SkipList:_find_predecessors(key)
    local update = {}
    local ranks = {}
    local node = self._head
    local cumulative_rank = 0

    for level = self._current_level, 1, -1 do
        local next_node = node.forward[level]
        while next_node ~= nil and self._compare(next_node.key, key) < 0 do
            cumulative_rank = cumulative_rank + node.span[level]
            node = next_node
            next_node = node.forward[level]
        end
        update[level] = node
        ranks[level] = cumulative_rank
    end

    return update, ranks
end

function SkipList:insert(key, value)
    if key == nil then
        error("key must not be nil", 2)
    end

    local update, ranks = self:_find_predecessors(key)
    local candidate = update[1].forward[1]
    if candidate ~= nil and self._compare(candidate.key, key) == 0 then
        candidate.value = value
        return false
    end

    local height = self:_random_level()
    if height > self._current_level then
        for level = self._current_level + 1, height do
            update[level] = self._head
            ranks[level] = 0
            self._head.span[level] = self._size
        end
        self._current_level = height
    end

    local node = new_node(key, value, height)
    local node_rank = ranks[1] + 1
    for level = 1, height do
        local predecessor = update[level]
        local old_span = predecessor.span[level]
        local span_to_node = node_rank - ranks[level]
        node.forward[level] = predecessor.forward[level]
        node.span[level] = old_span - span_to_node + 1
        predecessor.forward[level] = node
        predecessor.span[level] = span_to_node
    end

    for level = height + 1, self._current_level do
        update[level].span[level] = update[level].span[level] + 1
    end

    self._size = self._size + 1
    return true
end

function SkipList:delete(key)
    if key == nil then
        return false
    end
    local update = self:_find_predecessors(key)
    local target = update[1].forward[1]
    if target == nil or self._compare(target.key, key) ~= 0 then
        return false
    end

    for level = 1, self._current_level do
        local predecessor = update[level]
        if predecessor.forward[level] == target then
            predecessor.span[level] = predecessor.span[level] + target.span[level] - 1
            predecessor.forward[level] = target.forward[level]
        else
            predecessor.span[level] = predecessor.span[level] - 1
        end
    end

    while self._current_level > 1 and self._head.forward[self._current_level] == nil do
        self._current_level = self._current_level - 1
    end
    self._size = self._size - 1
    return true
end

SkipList.remove = SkipList.delete

function SkipList:search(key)
    if key == nil then
        return nil
    end
    local update = self:_find_predecessors(key)
    local candidate = update[1].forward[1]
    if candidate ~= nil and self._compare(candidate.key, key) == 0 then
        return candidate.value
    end
    return nil
end

SkipList.get = SkipList.search

function SkipList:contains(key)
    if key == nil then
        return false
    end
    local update = self:_find_predecessors(key)
    local candidate = update[1].forward[1]
    return candidate ~= nil and self._compare(candidate.key, key) == 0
end

SkipList.has = SkipList.contains

function SkipList:rank(key)
    if key == nil then
        return nil
    end
    local update, ranks = self:_find_predecessors(key)
    local candidate = update[1].forward[1]
    if candidate ~= nil and self._compare(candidate.key, key) == 0 then
        return ranks[1]
    end
    return nil
end

function SkipList:by_rank(rank)
    if type(rank) ~= "number" or rank < 0 or rank % 1 ~= 0 or rank >= self._size then
        return nil
    end

    local target_rank = rank + 1
    local traversed = 0
    local node = self._head
    for level = self._current_level, 1, -1 do
        while node.forward[level] ~= nil
            and traversed + node.span[level] <= target_rank do
            traversed = traversed + node.span[level]
            node = node.forward[level]
        end
    end
    if node ~= self._head and traversed == target_rank then
        return node.key
    end
    return nil
end

function SkipList:kth_smallest(k)
    if type(k) ~= "number" or k < 1 or k % 1 ~= 0 then
        return nil
    end
    return self:by_rank(k - 1)
end

function SkipList:range_query(minimum, maximum, inclusive)
    if minimum == nil then
        error("minimum must not be nil", 2)
    end
    if maximum == nil then
        error("maximum must not be nil", 2)
    end
    local include_bounds = inclusive
    if include_bounds == nil then
        include_bounds = true
    elseif type(include_bounds) ~= "boolean" then
        error("inclusive must be a boolean", 2)
    end
    if self._compare(minimum, maximum) > 0 then
        return {}
    end

    local update = self:_find_predecessors(minimum)
    local node = update[1].forward[1]
    if not include_bounds and node ~= nil and self._compare(node.key, minimum) == 0 then
        node = node.forward[1]
    end

    local result = {}
    while node ~= nil do
        local upper_order = self._compare(node.key, maximum)
        if upper_order > 0 or (not include_bounds and upper_order == 0) then
            break
        end
        result[#result + 1] = {node.key, node.value}
        node = node.forward[1]
    end
    return result
end

SkipList.range = SkipList.range_query

function SkipList:to_list()
    local result = {}
    local node = self._head.forward[1]
    while node ~= nil do
        result[#result + 1] = node.key
        node = node.forward[1]
    end
    return result
end

SkipList.to_array = SkipList.to_list
SkipList.to_sorted_array = SkipList.to_list

function SkipList:entries()
    local result = {}
    local node = self._head.forward[1]
    while node ~= nil do
        result[#result + 1] = {node.key, node.value}
        node = node.forward[1]
    end
    return result
end

function SkipList:iterator()
    local node = self._head.forward[1]
    return function()
        if node == nil then
            return nil
        end
        local key = node.key
        local value = node.value
        node = node.forward[1]
        return key, value
    end
end

function SkipList:values()
    local node = self._head.forward[1]
    return function()
        if node == nil then
            return nil
        end
        local key = node.key
        node = node.forward[1]
        return key
    end
end

function SkipList:size()
    return self._size
end

SkipList.length = SkipList.size

function SkipList:is_empty()
    return self._size == 0
end

function SkipList:min()
    local node = self._head.forward[1]
    return node and node.key or nil
end

function SkipList:max()
    local node = self._head.forward[1]
    if node == nil then
        return nil
    end
    while node.forward[1] ~= nil do
        node = node.forward[1]
    end
    return node.key
end

function SkipList:max_level()
    return self._max_level
end

function SkipList:probability()
    return self._probability
end

function SkipList:current_level()
    return self._current_level
end

function SkipList:is_valid_skip_list()
    local positions = {[self._head] = 0}
    local count = 0
    local previous = nil
    local node = self._head.forward[1]
    while node ~= nil do
        if previous ~= nil and self._compare(previous.key, node.key) >= 0 then
            return false
        end
        count = count + 1
        positions[node] = count
        previous = node
        node = node.forward[1]
    end
    if count ~= self._size then
        return false
    end

    for level = 1, self._current_level do
        node = self._head
        while node ~= nil do
            local next_node = node.forward[level]
            local position = positions[node]
            local expected_span
            if next_node ~= nil then
                if positions[next_node] == nil or next_node.height < level then
                    return false
                end
                expected_span = positions[next_node] - position
                if expected_span <= 0 then
                    return false
                end
            else
                expected_span = self._size - position
            end
            if node.span[level] ~= expected_span then
                return false
            end
            node = next_node
        end
    end
    return true
end

function SkipList:__len()
    return self._size
end

function SkipList:__tostring()
    local rendered = {}
    for index, key in ipairs(self:to_list()) do
        rendered[index] = tostring(key)
    end
    return "SkipList([" .. table.concat(rendered, ", ") .. "])"
end

M.SkipList = SkipList
M.new = SkipList.new
M.from_entries = SkipList.from_entries
M.default_compare = default_compare

return M
