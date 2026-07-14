local M = {VERSION = "0.1.0"}

local FenwickTree = {}
FenwickTree.__index = FenwickTree

local function require_integer(value, name)
    if type(value) ~= "number" or value % 1 ~= 0 then
        error(name .. " must be an integer", 3)
    end
end

local function lowbit(index)
    return index & -index
end

local function check_index(self, index)
    require_integer(index, "index")
    if index < 1 or index > self._n then
        error(string.format("index %d out of range [1, %d]", index, self._n), 3)
    end
end

function FenwickTree.new(size)
    require_integer(size, "size")
    if size < 0 then
        error("size must be non-negative", 2)
    end

    local bit = {}
    for index = 0, size do
        bit[index] = 0
    end

    return setmetatable({_n = size, _bit = bit}, FenwickTree)
end

function FenwickTree.from_list(values)
    if type(values) ~= "table" then
        error("values must be a table", 2)
    end

    local tree = FenwickTree.new(#values)
    for index = 1, tree._n do
        local value = values[index]
        if type(value) ~= "number" then
            error(string.format("value at index %d must be a number", index), 2)
        end
        tree._bit[index] = tree._bit[index] + value
        local parent = index + lowbit(index)
        if parent <= tree._n then
            tree._bit[parent] = tree._bit[parent] + tree._bit[index]
        end
    end
    return tree
end

function FenwickTree:update(index, delta)
    check_index(self, index)
    if type(delta) ~= "number" then
        error("delta must be a number", 2)
    end

    local current = index
    while current <= self._n do
        self._bit[current] = self._bit[current] + delta
        current = current + lowbit(current)
    end
    return self
end

function FenwickTree:prefix_sum(index)
    require_integer(index, "index")
    if index < 0 or index > self._n then
        error(string.format("prefix index %d out of range [0, %d]", index, self._n), 2)
    end

    local total = 0
    local current = index
    while current > 0 do
        total = total + self._bit[current]
        current = current - lowbit(current)
    end
    return total
end

function FenwickTree:range_sum(left, right)
    require_integer(left, "left")
    require_integer(right, "right")
    if left > right then
        error("left must be <= right", 2)
    end
    check_index(self, left)
    check_index(self, right)
    return self:prefix_sum(right) - self:prefix_sum(left - 1)
end

function FenwickTree:point_query(index)
    check_index(self, index)
    return self:range_sum(index, index)
end

function FenwickTree:find_kth(target)
    if self._n == 0 then
        error("find_kth called on empty tree", 2)
    end
    if type(target) ~= "number" or target <= 0 then
        error("target must be positive", 2)
    end

    local total = self:prefix_sum(self._n)
    if target > total then
        error("target exceeds total sum", 2)
    end

    local index = 0
    local step = 1
    while step * 2 <= self._n do
        step = step * 2
    end

    while step > 0 do
        local next_index = index + step
        if next_index <= self._n and self._bit[next_index] < target then
            index = next_index
            target = target - self._bit[index]
        end
        step = math.floor(step / 2)
    end
    return index + 1
end

function FenwickTree:len()
    return self._n
end

function FenwickTree:size()
    return self._n
end

function FenwickTree:is_empty()
    return self._n == 0
end

function FenwickTree:bit_array()
    local result = {}
    for index = 1, self._n do
        result[index] = self._bit[index]
    end
    return result
end

function FenwickTree:__tostring()
    local values = self:bit_array()
    for index, value in ipairs(values) do
        values[index] = tostring(value)
    end
    return string.format("FenwickTree(n=%d, bit={%s})", self._n, table.concat(values, ", "))
end

M.FenwickTree = FenwickTree

return M
