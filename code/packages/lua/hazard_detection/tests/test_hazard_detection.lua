-- Tests for coding_adventures.hazard_detection

local hd = require("coding_adventures.hazard_detection")
local PipelineSlot             = hd.PipelineSlot
local HazardResult             = hd.HazardResult
local DataHazardDetector       = hd.DataHazardDetector
local ControlHazardDetector    = hd.ControlHazardDetector
local StructuralHazardDetector = hd.StructuralHazardDetector

-- Helpers
local function empty() return PipelineSlot.empty() end
local function slot(opts) return PipelineSlot.new(opts) end

-- ---------------------------------------------------------------------------
-- DataHazardDetector
-- ---------------------------------------------------------------------------

describe("DataHazardDetector — no hazard", function()
    local det = DataHazardDetector.new()

    it("returns none for empty ID stage", function()
        local r = det:detect(empty(), empty(), empty())
        assert.equals("none", r.action)
    end)

    it("returns none when ID has no source regs", function()
        local id = slot({valid = true, source_regs = {}})
        local r = det:detect(id, empty(), empty())
        assert.equals("none", r.action)
    end)

    it("returns none when no dependency exists", function()
        local id  = slot({valid = true, source_regs = {1, 2}})
        local ex  = slot({valid = true, dest_reg = 5})
        local mem = slot({valid = true, dest_reg = 6})
        local r = det:detect(id, ex, mem)
        assert.equals("none", r.action)
    end)
end)

describe("DataHazardDetector — load-use stall", function()
    local det = DataHazardDetector.new()

    it("detects load-use when EX is a load writing dest_reg == source_reg", function()
        local id  = slot({valid = true, source_regs = {1}, pc = 0x8})
        local ex  = slot({valid = true, dest_reg = 1, mem_read = true, pc = 0x4})
        local mem = slot({valid = false})
        local r = det:detect(id, ex, mem)
        assert.equals("stall", r.action)
        assert.equals(1, r.stall_cycles)
    end)

    it("load-use takes priority over EX forwarding", function()
        -- A load in EX writing R1 should stall, not forward
        local id  = slot({valid = true, source_regs = {1}})
        local ex  = slot({valid = true, dest_reg = 1, mem_read = true})
        local mem = slot({valid = true, dest_reg = 1})
        local r = det:detect(id, ex, mem)
        assert.equals("stall", r.action)
    end)
end)

describe("DataHazardDetector — EX forwarding", function()
    local det = DataHazardDetector.new()

    it("forwards from EX when EX writes source reg (non-load)", function()
        local id  = slot({valid = true, source_regs = {3}})
        local ex  = slot({valid = true, dest_reg = 3, dest_value = 42})
        local mem = slot({valid = false})
        local r = det:detect(id, ex, mem)
        assert.equals("forward_ex", r.action)
        assert.equals(42, r.forwarded_value)
        assert.equals("EX", r.forwarded_from)
    end)

    it("EX forwarding takes priority over MEM forwarding for same register", function()
        local id  = slot({valid = true, source_regs = {3}})
        local ex  = slot({valid = true, dest_reg = 3, dest_value = 10})
        local mem = slot({valid = true, dest_reg = 3, dest_value = 5})
        local r = det:detect(id, ex, mem)
        assert.equals("forward_ex", r.action)
        assert.equals(10, r.forwarded_value)
    end)
end)

describe("DataHazardDetector — MEM forwarding", function()
    local det = DataHazardDetector.new()

    it("forwards from MEM when MEM writes source reg and EX does not", function()
        local id  = slot({valid = true, source_regs = {7}})
        local ex  = slot({valid = true, dest_reg = 9})  -- different reg
        local mem = slot({valid = true, dest_reg = 7, dest_value = 99})
        local r = det:detect(id, ex, mem)
        assert.equals("forward_mem", r.action)
        assert.equals(99, r.forwarded_value)
        assert.equals("MEM", r.forwarded_from)
    end)

    it("returns none when mem stage is empty", function()
        local id  = slot({valid = true, source_regs = {7}})
        local ex  = slot({valid = false})
        local mem = slot({valid = false})
        local r = det:detect(id, ex, mem)
        assert.equals("none", r.action)
    end)
end)

describe("DataHazardDetector — multiple source regs", function()
    local det = DataHazardDetector.new()

    it("returns highest priority when two source regs, one needs stall", function()
        -- rs1 = R2 (EX forward), rs2 = R4 (load-use stall)
        local id  = slot({valid = true, source_regs = {2, 4}})
        local ex  = slot({valid = true, dest_reg = 4, mem_read = true})
        local mem = slot({valid = true, dest_reg = 2, dest_value = 7})
        local r = det:detect(id, ex, mem)
        assert.equals("stall", r.action)  -- stall beats forward_mem
    end)
end)

-- ---------------------------------------------------------------------------
-- ControlHazardDetector
-- ---------------------------------------------------------------------------

describe("ControlHazardDetector — no branch", function()
    local det = ControlHazardDetector.new()

    it("returns none for empty EX stage", function()
        local r = det:detect(empty())
        assert.equals("none", r.action)
    end)

    it("returns none when EX instruction is not a branch", function()
        local ex = slot({valid = true, is_branch = false})
        local r = det:detect(ex)
        assert.equals("none", r.action)
    end)
end)

describe("ControlHazardDetector — correct prediction", function()
    local det = ControlHazardDetector.new()

    it("returns none when branch correctly predicted taken", function()
        local ex = slot({
            valid                  = true,
            is_branch              = true,
            branch_taken           = true,
            branch_predicted_taken = true,
        })
        local r = det:detect(ex)
        assert.equals("none", r.action)
    end)

    it("returns none when branch correctly predicted not-taken", function()
        local ex = slot({
            valid                  = true,
            is_branch              = true,
            branch_taken           = false,
            branch_predicted_taken = false,
        })
        local r = det:detect(ex)
        assert.equals("none", r.action)
    end)
end)

describe("ControlHazardDetector — misprediction", function()
    local det = ControlHazardDetector.new()

    it("flushes 2 stages when predicted not-taken but taken", function()
        local ex = slot({
            valid                  = true,
            is_branch              = true,
            branch_taken           = true,
            branch_predicted_taken = false,
            pc                     = 0x10,
        })
        local r = det:detect(ex)
        assert.equals("flush", r.action)
        assert.equals(2, r.flush_count)
        assert.truthy(r.reason:find("misprediction"))
    end)

    it("flushes 2 stages when predicted taken but not-taken", function()
        local ex = slot({
            valid                  = true,
            is_branch              = true,
            branch_taken           = false,
            branch_predicted_taken = true,
        })
        local r = det:detect(ex)
        assert.equals("flush", r.action)
        assert.equals(2, r.flush_count)
    end)
end)

-- ---------------------------------------------------------------------------
-- StructuralHazardDetector
-- ---------------------------------------------------------------------------

describe("StructuralHazardDetector — no conflict", function()
    local det = StructuralHazardDetector.new()

    it("returns none when both stages are empty", function()
        local r = det:detect(empty(), empty())
        assert.equals("none", r.action)
    end)

    it("returns none when instructions use different units", function()
        local id = slot({valid = true, uses_alu = true,  uses_fp = false})
        local ex = slot({valid = true, uses_alu = false, uses_fp = true})
        local r = det:detect(id, ex)
        assert.equals("none", r.action)
    end)

    it("split caches: no memory port conflict even with concurrent access", function()
        local det2 = StructuralHazardDetector.new({split_caches = true})
        local if_s   = slot({valid = true})
        local mem_s  = slot({valid = true, mem_read = true})
        local r = det2:detect(empty(), empty(), {if_stage = if_s, mem_stage = mem_s})
        assert.equals("none", r.action)
    end)
end)

describe("StructuralHazardDetector — ALU conflict", function()
    local det = StructuralHazardDetector.new({num_alus = 1})

    it("stalls when both ID and EX need the single ALU", function()
        local id = slot({valid = true, uses_alu = true})
        local ex = slot({valid = true, uses_alu = true})
        local r = det:detect(id, ex)
        assert.equals("stall", r.action)
        assert.equals(1, r.stall_cycles)
    end)

    it("no conflict with 2 ALUs", function()
        local det2 = StructuralHazardDetector.new({num_alus = 2})
        local id = slot({valid = true, uses_alu = true})
        local ex = slot({valid = true, uses_alu = true})
        local r = det2:detect(id, ex)
        assert.equals("none", r.action)
    end)
end)

describe("StructuralHazardDetector — memory port conflict", function()
    local det = StructuralHazardDetector.new({split_caches = false})

    it("stalls when unified cache has IF and MEM concurrent access", function()
        local if_s  = slot({valid = true, pc = 0x100})
        local mem_s = slot({valid = true, pc = 0x50, mem_read = true})
        local r = det:detect(empty(), empty(), {if_stage = if_s, mem_stage = mem_s})
        assert.equals("stall", r.action)
    end)

    it("no conflict when MEM stage is empty", function()
        local if_s  = slot({valid = true})
        local mem_s = slot({valid = false})
        local r = det:detect(empty(), empty(), {if_stage = if_s, mem_stage = mem_s})
        assert.equals("none", r.action)
    end)
end)

describe("HazardResult priority", function()
    it("higher priority result wins in pick comparisons via DataHazardDetector", function()
        -- Two source regs: one causes MEM forward, other causes stall
        -- stall should win
        local det = DataHazardDetector.new()
        local id  = slot({valid = true, source_regs = {1, 2}})
        local ex  = slot({valid = true, dest_reg = 2, mem_read = true})
        local mem = slot({valid = true, dest_reg = 1, dest_value = 0})
        local r = det:detect(id, ex, mem)
        assert.equals("stall", r.action)
    end)
end)
