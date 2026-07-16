package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    package.path,
}, ";")

local module = require("coding_adventures.hyperloglog")
local HyperLogLog = module.HyperLogLog

local function assert_within(actual, expected, tolerance)
    assert.is_true(
        math.abs(actual - expected) <= tolerance,
        string.format("expected %d within %d of %d", actual, tolerance, expected)
    )
end

describe("HyperLogLog", function()
    it("counts an empty sketch and repeated values", function()
        local sketch = HyperLogLog.new()
        assert.equals(0, sketch:count())
        assert.is_true(sketch:is_empty())

        for _ = 1, 1000 do
            sketch:add("same-value")
        end
        assert.equals(1, sketch:count())
        assert.is_false(sketch:is_empty())
    end)

    it("estimates distinct cardinality with bounded error", function()
        local sketch = HyperLogLog.new(10)
        for value = 1, 10000 do
            sketch:add("user-" .. value)
        end
        assert_within(sketch:count(), 10000, 1000)
    end)

    it("merges disjoint and overlapping sketches without mutating inputs", function()
        local left = HyperLogLog.new(10)
        local right = HyperLogLog.new(10)
        for value = 1, 1000 do
            left:add("left-" .. value)
            right:add("right-" .. value)
        end

        local left_count = left:count()
        local merged = left:merge(right)
        assert.equals(left_count, left:count())
        assert_within(merged:count(), 2000, 300)

        local overlap = HyperLogLog.new(10)
        for value = 1, 1000 do
            overlap:add("left-" .. value)
        end
        assert_within(left:merge(overlap):count(), 1000, 150)
    end)

    it("reports precision, memory, error rate, and defensive registers", function()
        local sketch = module.new(10)
        assert.equals(10, sketch:precision())
        assert.equals(1024, sketch:num_registers())
        assert.equals(768, sketch:memory_bytes())
        assert.is_true(math.abs(sketch:error_rate() - 0.0325) < 0.0001)

        local registers = sketch:registers()
        registers[1] = 99
        assert.equals(0, sketch:registers()[1])
        assert.is_truthy(tostring(sketch):match("precision=10"))
    end)

    it("clears state and validates public inputs", function()
        local sketch = HyperLogLog.new(4)
        sketch:add("value"):clear()
        assert.is_true(sketch:is_empty())
        assert.equals(0, #sketch)

        assert.has_error(function()
            HyperLogLog.new(3)
        end, "precision must be an integer between 4 and 16")
        assert.has_error(function()
            HyperLogLog.new(17)
        end, "precision must be an integer between 4 and 16")
        assert.has_error(function()
            HyperLogLog.new(4.5)
        end, "precision must be an integer between 4 and 16")
        assert.has_error(function()
            sketch:merge(HyperLogLog.new(5))
        end, "cannot merge HyperLogLog sketches with different precisions")
        assert.has_error(function()
            sketch:merge({})
        end, "other must be a HyperLogLog")
    end)

    it("hashes the same input deterministically", function()
        local left = HyperLogLog.new(8)
        local right = HyperLogLog.new(8)
        for value = 1, 100 do
            left:add("item-" .. value)
            right:add("item-" .. value)
        end
        assert.are.same(left:registers(), right:registers())
    end)
end)
