local M = {}

M.VERSION = "0.1.0"

local function default_compare(left, right)
    if left == right then
        return 0
    end
    return left < right and -1 or 1
end

M.default_compare = default_compare

local Heap = {}
Heap.__index = Heap

function Heap:len()
    return #self._data
end

function Heap:size()
    return #self._data
end

function Heap:is_empty()
    return #self._data == 0
end

function Heap:peek()
    return self._data[1]
end

function Heap:to_array()
    local result = {}
    for index, value in ipairs(self._data) do
        result[index] = value
    end
    return result
end

function Heap:_higher_priority(_left, _right)
    error("Heap:_higher_priority must be implemented by subclasses")
end

function Heap:_sift_up(index)
    while index > 1 do
        local parent = math.floor(index / 2)
        if self:_higher_priority(self._data[index], self._data[parent]) then
            self._data[index], self._data[parent] = self._data[parent], self._data[index]
            index = parent
        else
            break
        end
    end
end

function Heap:_sift_down(index)
    local size = #self._data
    while true do
        local left = index * 2
        local right = left + 1
        local best = index

        if left <= size and self:_higher_priority(self._data[left], self._data[best]) then
            best = left
        end
        if right <= size and self:_higher_priority(self._data[right], self._data[best]) then
            best = right
        end

        if best == index then
            break
        end

        self._data[index], self._data[best] = self._data[best], self._data[index]
        index = best
    end
end

function Heap:_build_from_iterable(items)
    self._data = {}
    if items == nil then
        return
    end

    for _, value in ipairs(items) do
        self._data[#self._data + 1] = value
    end

    for index = math.floor(#self._data / 2), 1, -1 do
        self:_sift_down(index)
    end
end

function Heap:push(value)
    self._data[#self._data + 1] = value
    self:_sift_up(#self._data)
    return self
end

function Heap:pop()
    if #self._data == 0 then
        return nil
    end

    local root = self._data[1]
    if #self._data == 1 then
        self._data[1] = nil
        return root
    end

    self._data[1] = self._data[#self._data]
    self._data[#self._data] = nil
    self:_sift_down(1)
    return root
end

local MinHeap = setmetatable({}, {__index = Heap})
MinHeap.__index = MinHeap

function MinHeap.new(compare)
    return setmetatable({
        _compare = compare or default_compare,
        _data = {},
    }, MinHeap)
end

function MinHeap.from_iterable(items, compare)
    local heap = MinHeap.new(compare)
    heap:_build_from_iterable(items)
    return heap
end

function MinHeap:_higher_priority(left, right)
    return self._compare(left, right) < 0
end

local MaxHeap = setmetatable({}, {__index = Heap})
MaxHeap.__index = MaxHeap

function MaxHeap.new(compare)
    return setmetatable({
        _compare = compare or default_compare,
        _data = {},
    }, MaxHeap)
end

function MaxHeap.from_iterable(items, compare)
    local heap = MaxHeap.new(compare)
    heap:_build_from_iterable(items)
    return heap
end

function MaxHeap:_higher_priority(left, right)
    return self._compare(left, right) > 0
end

M.MinHeap = MinHeap
M.MaxHeap = MaxHeap

return M
