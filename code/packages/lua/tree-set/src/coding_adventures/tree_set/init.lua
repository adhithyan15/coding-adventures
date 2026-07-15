local avl = require("coding_adventures.avl_tree")

local M = {VERSION = "0.1.0"}

local function lower_bound(items, value, compare)
    local low = 1
    local high = #items + 1
    while low < high do
        local middle = math.floor((low + high) / 2)
        if compare(items[middle], value) < 0 then
            low = middle + 1
        else
            high = middle
        end
    end
    return low
end

local function upper_bound(items, value, compare)
    local low = 1
    local high = #items + 1
    while low < high do
        local middle = math.floor((low + high) / 2)
        if compare(items[middle], value) <= 0 then
            low = middle + 1
        else
            high = middle
        end
    end
    return low
end

local function merge_unique(left, right, compare)
    local result = {}
    local left_index = 1
    local right_index = 1
    while left_index <= #left and right_index <= #right do
        local order = compare(left[left_index], right[right_index])
        if order < 0 then
            result[#result + 1] = left[left_index]
            left_index = left_index + 1
        elseif order > 0 then
            result[#result + 1] = right[right_index]
            right_index = right_index + 1
        else
            result[#result + 1] = left[left_index]
            left_index = left_index + 1
            right_index = right_index + 1
        end
    end
    while left_index <= #left do
        result[#result + 1] = left[left_index]
        left_index = left_index + 1
    end
    while right_index <= #right do
        result[#result + 1] = right[right_index]
        right_index = right_index + 1
    end
    return result
end

local function intersection_sorted(left, right, compare)
    local result = {}
    local left_index = 1
    local right_index = 1
    while left_index <= #left and right_index <= #right do
        local order = compare(left[left_index], right[right_index])
        if order < 0 then
            left_index = left_index + 1
        elseif order > 0 then
            right_index = right_index + 1
        else
            result[#result + 1] = left[left_index]
            left_index = left_index + 1
            right_index = right_index + 1
        end
    end
    return result
end

local function difference_sorted(left, right, compare)
    local result = {}
    local left_index = 1
    local right_index = 1
    while left_index <= #left and right_index <= #right do
        local order = compare(left[left_index], right[right_index])
        if order < 0 then
            result[#result + 1] = left[left_index]
            left_index = left_index + 1
        elseif order > 0 then
            right_index = right_index + 1
        else
            left_index = left_index + 1
            right_index = right_index + 1
        end
    end
    while left_index <= #left do
        result[#result + 1] = left[left_index]
        left_index = left_index + 1
    end
    return result
end

local function symmetric_difference_sorted(left, right, compare)
    local result = {}
    local left_index = 1
    local right_index = 1
    while left_index <= #left and right_index <= #right do
        local order = compare(left[left_index], right[right_index])
        if order < 0 then
            result[#result + 1] = left[left_index]
            left_index = left_index + 1
        elseif order > 0 then
            result[#result + 1] = right[right_index]
            right_index = right_index + 1
        else
            left_index = left_index + 1
            right_index = right_index + 1
        end
    end
    while left_index <= #left do
        result[#result + 1] = left[left_index]
        left_index = left_index + 1
    end
    while right_index <= #right do
        result[#result + 1] = right[right_index]
        right_index = right_index + 1
    end
    return result
end

local function is_subset_sorted(left, right, compare)
    local left_index = 1
    local right_index = 1
    while left_index <= #left and right_index <= #right do
        local order = compare(left[left_index], right[right_index])
        if order < 0 then
            return false
        end
        if order > 0 then
            right_index = right_index + 1
        else
            left_index = left_index + 1
            right_index = right_index + 1
        end
    end
    return left_index > #left
end

local function is_disjoint_sorted(left, right, compare)
    local left_index = 1
    local right_index = 1
    while left_index <= #left and right_index <= #right do
        local order = compare(left[left_index], right[right_index])
        if order < 0 then
            left_index = left_index + 1
        elseif order > 0 then
            right_index = right_index + 1
        else
            return false
        end
    end
    return true
end

local TreeSet = {}
TreeSet.__index = TreeSet

local function is_tree_set(value)
    return type(value) == "table" and getmetatable(value) == TreeSet
end

local function require_tree_set(value)
    if not is_tree_set(value) then
        error("other must be a TreeSet", 3)
    end
end

function TreeSet.new(values, compare)
    local initial = values
    if initial == nil then
        initial = {}
    elseif type(initial) ~= "table" then
        error("values must be a table", 2)
    end
    local comparator = compare or avl.default_compare
    if type(comparator) ~= "function" then
        error("compare must be a function", 2)
    end

    local self = setmetatable({
        _tree = avl.AVLTree.empty(comparator),
        _compare = comparator,
    }, TreeSet)
    for _, value in ipairs(initial) do
        self:add(value)
    end
    return self
end

function TreeSet.empty(compare)
    return TreeSet.new({}, compare)
end

function TreeSet.from_values(values, compare)
    return TreeSet.new(values, compare)
end

function TreeSet:backend()
    return self._tree
end

function TreeSet:add(value)
    if value == nil then
        error("value must not be nil", 2)
    end
    self._tree = self._tree:insert(value)
    return self
end

TreeSet.insert = TreeSet.add

function TreeSet:delete(value)
    if value == nil then
        return false
    end
    if not self._tree:contains(value) then
        return false
    end
    self._tree = self._tree:delete(value)
    return true
end

TreeSet.remove = TreeSet.delete
TreeSet.discard = TreeSet.delete

function TreeSet:has(value)
    if value == nil then
        return false
    end
    return self._tree:contains(value)
end

TreeSet.contains = TreeSet.has

function TreeSet:size()
    return self._tree:size()
end

function TreeSet:length()
    return self:size()
end

function TreeSet:is_empty()
    return self:size() == 0
end

function TreeSet:min()
    return self._tree:min_value()
end

function TreeSet:max()
    return self._tree:max_value()
end

TreeSet.first = TreeSet.min
TreeSet.last = TreeSet.max

function TreeSet:predecessor(value)
    return self._tree:predecessor(value)
end

function TreeSet:successor(value)
    return self._tree:successor(value)
end

function TreeSet:rank(value)
    return self._tree:rank(value)
end

function TreeSet:by_rank(rank)
    if type(rank) ~= "number" or rank < 0 or rank % 1 ~= 0 then
        return nil
    end
    return self._tree:kth_smallest(rank + 1)
end

function TreeSet:kth_smallest(k)
    return self._tree:kth_smallest(k)
end

function TreeSet:to_list()
    return self._tree:to_sorted_array()
end

TreeSet.to_sorted_array = TreeSet.to_list
TreeSet.to_array = TreeSet.to_list

function TreeSet:range(minimum, maximum, inclusive)
    local include_bounds = inclusive
    if include_bounds == nil then
        include_bounds = true
    elseif type(include_bounds) ~= "boolean" then
        error("inclusive must be a boolean", 2)
    end
    if self._compare(minimum, maximum) > 0 then
        return {}
    end

    local values = self:to_list()
    local first = include_bounds
        and lower_bound(values, minimum, self._compare)
        or upper_bound(values, minimum, self._compare)
    local last = include_bounds
        and upper_bound(values, maximum, self._compare) - 1
        or lower_bound(values, maximum, self._compare) - 1
    local result = {}
    for index = first, last do
        result[#result + 1] = values[index]
    end
    return result
end

function TreeSet:union(other)
    require_tree_set(other)
    return TreeSet.from_values(merge_unique(self:to_list(), other:to_list(), self._compare), self._compare)
end

function TreeSet:intersection(other)
    require_tree_set(other)
    return TreeSet.from_values(intersection_sorted(self:to_list(), other:to_list(), self._compare), self._compare)
end

function TreeSet:difference(other)
    require_tree_set(other)
    return TreeSet.from_values(difference_sorted(self:to_list(), other:to_list(), self._compare), self._compare)
end

function TreeSet:symmetric_difference(other)
    require_tree_set(other)
    return TreeSet.from_values(
        symmetric_difference_sorted(self:to_list(), other:to_list(), self._compare),
        self._compare
    )
end

function TreeSet:is_subset(other)
    require_tree_set(other)
    return is_subset_sorted(self:to_list(), other:to_list(), self._compare)
end

function TreeSet:is_superset(other)
    require_tree_set(other)
    return other:is_subset(self)
end

function TreeSet:is_disjoint(other)
    require_tree_set(other)
    return is_disjoint_sorted(self:to_list(), other:to_list(), self._compare)
end

function TreeSet:equals(other)
    if not is_tree_set(other) or self:size() ~= other:size() then
        return false
    end
    local left = self:to_list()
    local right = other:to_list()
    for index = 1, #left do
        if self._compare(left[index], right[index]) ~= 0 then
            return false
        end
    end
    return true
end

function TreeSet:values()
    local values = self:to_list()
    local index = 0
    return function()
        index = index + 1
        return values[index]
    end
end

function TreeSet:__len()
    return self:size()
end

function TreeSet:__tostring()
    local rendered = {}
    for index, value in ipairs(self:to_list()) do
        rendered[index] = tostring(value)
    end
    return "TreeSet([" .. table.concat(rendered, ", ") .. "])"
end

M.TreeSet = TreeSet
M.from_values = TreeSet.from_values
M.default_compare = avl.default_compare

return M
