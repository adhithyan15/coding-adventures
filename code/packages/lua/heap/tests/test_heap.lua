package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local heap_module = require("coding_adventures.heap")
local MinHeap = heap_module.MinHeap
local MaxHeap = heap_module.MaxHeap

describe("heap module", function()
    it("exposes VERSION and heap classes", function()
        assert.equals("0.1.0", heap_module.VERSION)
        assert.is_table(MinHeap)
        assert.is_table(MaxHeap)
    end)
end)

describe("MinHeap", function()
    it("pops values in ascending order", function()
        local heap = MinHeap.new()
        for _, value in ipairs({5, 3, 8, 1, 4}) do
            heap:push(value)
        end

        assert.equals(1, heap:peek())
        assert.equals(1, heap:pop())
        assert.equals(3, heap:pop())
        assert.equals(4, heap:pop())
        assert.equals(5, heap:pop())
        assert.equals(8, heap:pop())
        assert.is_nil(heap:pop())
    end)

    it("supports heapifying from an iterable", function()
        local heap = MinHeap.from_iterable({9, 2, 7, 1, 5})
        local popped = {}
        while not heap:is_empty() do
            popped[#popped + 1] = heap:pop()
        end

        assert.are.same({1, 2, 5, 7, 9}, popped)
    end)

    it("supports custom comparators", function()
        local heap = MinHeap.new(function(left, right)
            if left[1] ~= right[1] then
                return left[1] < right[1] and -1 or 1
            end
            if left[2] ~= right[2] then
                return left[2] < right[2] and -1 or 1
            end
            return 0
        end)

        heap:push({1, "b"})
        heap:push({1, "a"})
        heap:push({0, "z"})

        assert.are.same({0, "z"}, heap:pop())
        assert.are.same({1, "a"}, heap:pop())
        assert.are.same({1, "b"}, heap:pop())
    end)
end)

describe("MaxHeap", function()
    it("pops values in descending order", function()
        local heap = MaxHeap.new()
        for _, value in ipairs({5, 3, 8, 1, 4}) do
            heap:push(value)
        end

        assert.equals(8, heap:peek())
        assert.equals(8, heap:pop())
        assert.equals(5, heap:pop())
        assert.equals(4, heap:pop())
        assert.equals(3, heap:pop())
        assert.equals(1, heap:pop())
        assert.is_nil(heap:pop())
    end)
end)

describe("empty heap helpers", function()
    it("reports empty state correctly", function()
        local heap = MinHeap.new()
        assert.is_true(heap:is_empty())
        assert.equals(0, heap:len())
        assert.equals(0, heap:size())
        assert.is_nil(heap:peek())
        heap:push(42)
        assert.is_false(heap:is_empty())
        assert.equals(1, heap:len())
        assert.are.same({42}, heap:to_array())
    end)
end)
