-- test_cpu_pipeline.lua — Tests for the CPU pipeline package
--
-- These tests verify the configurable N-stage instruction pipeline.
-- We test at multiple levels:
--
--   1. Token creation (new, bubble)
--   2. PipelineConfig validation
--   3. Preset configurations (5-stage, 13-stage)
--   4. Pipeline step() — basic advancement
--   5. Stall behavior (bubble insertion, frozen stages)
--   6. Flush behavior (bubble replacement, PC redirect)
--   7. Forwarding signals
--   8. Halt propagation
--   9. Statistics (IPC, CPI, stall counts)
--  10. Trace / snapshot accuracy

-- Add the src/ directory to the module search path so tests can run
-- without a luarocks install in the local development environment.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local cpu_pipeline   = require("coding_adventures.cpu_pipeline")
local Pipeline       = cpu_pipeline.Pipeline
local PipelineConfig = cpu_pipeline.PipelineConfig
local PipelineStage  = cpu_pipeline.PipelineStage
local HazardResponse = cpu_pipeline.HazardResponse
local Token          = cpu_pipeline.Token

-- ========================================================================
-- Test helpers
-- ========================================================================

-- Build a minimal no-op pipeline over the given config.
-- fetch always returns 0; decode/execute/memory are identity;
-- writeback is a no-op.
local function noop_pipeline(config)
    local function fetch(pc)    return 0 end
    local function decode(r, t) return t end
    local function execute(t)   return t end
    local function memory(t)    return t end
    local function writeback(t) end

    local result = Pipeline.new(config, fetch, decode, execute, memory, writeback)
    assert.is_true(result.ok, result.err)
    return result.pipeline
end

-- Build a no-op 5-stage pipeline.
local function make_5stage()
    return noop_pipeline(PipelineConfig.classic_5_stage())
end

-- ========================================================================
-- Token tests
-- ========================================================================

describe("Token", function()

    it("new() creates a token with default values", function()
        local t = Token.new()
        assert.are.equal(0,     t.pc)
        assert.are.equal(0,     t.raw_instruction)
        assert.are.equal("",    t.opcode)
        assert.are.equal(-1,    t.rs1)
        assert.are.equal(-1,    t.rs2)
        assert.are.equal(-1,    t.rd)
        assert.is_false(t.is_bubble)
        assert.is_false(t.reg_write)
        assert.is_false(t.is_halt)
    end)

    it("new_bubble() creates a bubble token", function()
        local b = Token.new_bubble()
        assert.is_true(b.is_bubble)
        assert.are.equal("---", b:to_string())
    end)

    it("to_string() for normal token shows opcode@pc", function()
        local t = Token.new()
        t.opcode = "ADD"
        t.pc     = 100
        assert.are.equal("ADD@100", t:to_string())
    end)

    it("to_string() for undecoded token shows instr@pc", function()
        local t = Token.new()
        t.pc = 200
        assert.are.equal("instr@200", t:to_string())
    end)

    it("clone() produces an independent copy", function()
        local t = Token.new()
        t.pc     = 42
        t.opcode = "SUB"
        t.stage_entered["IF"] = 5

        local c = Token.clone(t)
        assert.are.equal(42,    c.pc)
        assert.are.equal("SUB", c.opcode)
        assert.are.equal(5,     c.stage_entered["IF"])

        -- Mutating original should not affect clone
        t.pc = 99
        t.stage_entered["IF"] = 99
        assert.are.equal(42, c.pc)
        assert.are.equal(5,  c.stage_entered["IF"])
    end)

    it("clone(nil) returns nil", function()
        assert.is_nil(Token.clone(nil))
    end)

end)

-- ========================================================================
-- PipelineConfig tests
-- ========================================================================

describe("PipelineConfig", function()

    it("classic_5_stage() has 5 stages with correct names", function()
        local cfg = PipelineConfig.classic_5_stage()
        assert.are.equal(5, cfg:num_stages())
        assert.are.equal("IF",  cfg.stages[1].name)
        assert.are.equal("ID",  cfg.stages[2].name)
        assert.are.equal("EX",  cfg.stages[3].name)
        assert.are.equal("MEM", cfg.stages[4].name)
        assert.are.equal("WB",  cfg.stages[5].name)
    end)

    it("classic_5_stage() has correct categories", function()
        local cfg = PipelineConfig.classic_5_stage()
        assert.are.equal("fetch",     cfg.stages[1].category)
        assert.are.equal("decode",    cfg.stages[2].category)
        assert.are.equal("execute",   cfg.stages[3].category)
        assert.are.equal("memory",    cfg.stages[4].category)
        assert.are.equal("writeback", cfg.stages[5].category)
    end)

    it("deep_13_stage() has 13 stages", function()
        local cfg = PipelineConfig.deep_13_stage()
        assert.are.equal(13, cfg:num_stages())
    end)

    it("validate() rejects pipeline with fewer than 2 stages", function()
        local cfg = PipelineConfig.new({
            PipelineStage.new("IF", "fetch", "fetch"),
        })
        local ok, err = cfg:validate()
        assert.is_false(ok)
        assert.truthy(err:find("at least 2"))
    end)

    it("validate() rejects duplicate stage names", function()
        local cfg = PipelineConfig.new({
            PipelineStage.new("IF", "fetch",     "fetch"),
            PipelineStage.new("IF", "duplicate", "writeback"),
        })
        local ok, err = cfg:validate()
        assert.is_false(ok)
        assert.truthy(err:find("duplicate"))
    end)

    it("validate() rejects config without fetch stage", function()
        local cfg = PipelineConfig.new({
            PipelineStage.new("EX", "execute",   "execute"),
            PipelineStage.new("WB", "writeback", "writeback"),
        })
        local ok, err = cfg:validate()
        assert.is_false(ok)
        assert.truthy(err:find("fetch"))
    end)

    it("validate() rejects config without writeback stage", function()
        local cfg = PipelineConfig.new({
            PipelineStage.new("IF", "fetch",   "fetch"),
            PipelineStage.new("EX", "execute", "execute"),
        })
        local ok, err = cfg:validate()
        assert.is_false(ok)
        assert.truthy(err:find("writeback"))
    end)

    it("validate() accepts a valid 5-stage config", function()
        local cfg = PipelineConfig.classic_5_stage()
        local ok, err = cfg:validate()
        assert.is_true(ok)
        assert.is_nil(err)
    end)

end)

-- ========================================================================
-- Pipeline construction tests
-- ========================================================================

describe("Pipeline.new()", function()

    it("returns ok=true for a valid config", function()
        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            function(pc) return 0 end,
            function(r, t) return t end,
            function(t) return t end,
            function(t) return t end,
            function(t) end
        )
        assert.is_true(result.ok)
        assert.is_nil(result.err)
    end)

    it("returns ok=false for an invalid config", function()
        local bad_cfg = PipelineConfig.new({
            PipelineStage.new("IF", "only", "fetch"),
        })
        local result = Pipeline.new(
            bad_cfg,
            function(pc) return 0 end,
            function(r, t) return t end,
            function(t) return t end,
            function(t) return t end,
            function(t) end
        )
        assert.is_false(result.ok)
        assert.truthy(result.err)
    end)

    it("starts with cycle=0 and not halted", function()
        local p = make_5stage()
        assert.are.equal(0,     p:get_cycle())
        assert.is_false(p:is_halted())
    end)

    it("starts with PC=0 by default", function()
        local p = make_5stage()
        assert.are.equal(0, p:get_pc())
    end)

end)

-- ========================================================================
-- Basic step() tests
-- ========================================================================

describe("Pipeline.step()", function()

    it("advances the cycle counter by 1 each call", function()
        local p = make_5stage()
        p:step()
        assert.are.equal(1, p:get_cycle())
        p:step()
        assert.are.equal(2, p:get_cycle())
        p:step()
        assert.are.equal(3, p:get_cycle())
    end)

    it("advances PC by 4 each cycle (default no predictor)", function()
        local p = make_5stage()
        p:set_pc(0)
        p:step()
        -- After step, PC should have advanced once (for the fetch)
        assert.are.equal(4, p:get_pc())
        p:step()
        assert.are.equal(8, p:get_pc())
    end)

    it("fetches instruction into IF stage on first step", function()
        local p = make_5stage()
        p:step()
        -- After first step, IF stage should have been fetched and shifted to ID
        -- (IF was filled then shifted). Let's check snapshot includes something
        local snap = p:snapshot()
        -- At cycle 1: IF was filled → it moved to slot 2 (ID). Slot 1 is new fetch.
        -- The snapshot stages map should have at least one entry
        local count = 0
        for _ in pairs(snap.stages) do count = count + 1 end
        assert.are.equal(1, snap.cycle)
    end)

    it("snapshot returns correct cycle number", function()
        local p = make_5stage()
        local snap = p:step()
        assert.are.equal(1, snap.cycle)
        snap = p:step()
        assert.are.equal(2, snap.cycle)
    end)

    it("does not advance when halted", function()
        local p = make_5stage()
        p.halted = true
        p:step()
        assert.are.equal(0, p:get_cycle())
    end)

    it("uses custom predict_fn for PC advance", function()
        local p = make_5stage()
        p:set_predict_fn(function(pc) return pc + 2 end)
        p:set_pc(0)
        p:step()
        assert.are.equal(2, p:get_pc())
        p:step()
        assert.are.equal(4, p:get_pc())
    end)

end)

-- ========================================================================
-- Halt propagation tests
-- ========================================================================

describe("Halt propagation", function()

    it("halt instruction sets halted=true after reaching WB", function()
        -- Use a decode callback that marks every instruction as halt
        local function fetch(pc)    return 0 end
        local function decode(r, t)
            t.opcode  = "HALT"
            t.is_halt = true
            return t
        end
        local function execute(t) return t end
        local function memory(t)  return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        -- Run 5 cycles to let the halt instruction reach WB
        for i = 1, 5 do
            p:step()
            if p:is_halted() then break end
        end

        assert.is_true(p:is_halted())
    end)

    it("stats count only non-bubble instructions as completed", function()
        -- Use noop pipeline and run 10 cycles
        local p = make_5stage()
        p:run(10)
        local stats = p:get_stats()
        assert.are.equal(10, stats.total_cycles)
        -- instructions_completed counts non-bubbles in WB — first 4 cycles WB is nil
        -- so 10 - 4 = 6 instructions complete, but our noop decode doesn't mark halt
        -- so pipeline runs all 10 cycles
        assert.are.equal(6, stats.instructions_completed)
    end)

end)

-- ========================================================================
-- Stall tests
-- ========================================================================

describe("Stall behavior", function()

    it("stall inserts a bubble and freezes earlier stages", function()
        local stall_on_first = true

        local function fetch(pc) return 0 end
        local function decode(r, t) return t end
        local function execute(t) return t end
        local function memory(t)  return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        -- Hazard fn: stall on cycle 2 (once only)
        local stalled_count = 0
        p:set_hazard_fn(function(stages)
            if stalled_count < 1 and stages[2] ~= nil and not stages[2].is_bubble then
                stalled_count = stalled_count + 1
                return HazardResponse.new({ action = "stall" })
            end
            return HazardResponse.new({ action = "none" })
        end)

        -- Step once to fill IF
        p:step()
        -- Step again — this triggers the stall
        local snap = p:step()

        assert.is_true(snap.stalled)
        local stats = p:get_stats()
        assert.are.equal(1, stats.stall_cycles)
    end)

    it("stall increments stall_cycles counter", function()
        local stall_count = 0
        local function fetch(pc)    return 0 end
        local function decode(r, t) return t end
        local function execute(t)   return t end
        local function memory(t)    return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        p:set_hazard_fn(function(stages)
            if stall_count < 2 then
                stall_count = stall_count + 1
                return HazardResponse.new({ action = "stall" })
            end
            return HazardResponse.new({ action = "none" })
        end)

        p:run(5)
        assert.are.equal(2, p:get_stats().stall_cycles)
    end)

end)

-- ========================================================================
-- Flush tests
-- ========================================================================

describe("Flush behavior", function()

    it("flush redirects PC to redirect_pc", function()
        local flushed = false
        local function fetch(pc)    return 0 end
        local function decode(r, t) return t end
        local function execute(t)   return t end
        local function memory(t)    return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        p:set_hazard_fn(function(stages)
            if not flushed then
                flushed = true
                return HazardResponse.new({
                    action      = "flush",
                    redirect_pc = 100,
                })
            end
            return HazardResponse.new({ action = "none" })
        end)

        p:step()  -- trigger flush
        -- After flush, PC should be 100 + 4 = 104
        assert.are.equal(104, p:get_pc())
    end)

    it("flush increments flush_cycles counter", function()
        local flush_count = 0
        local function fetch(pc)    return 0 end
        local function decode(r, t) return t end
        local function execute(t)   return t end
        local function memory(t)    return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        p:set_hazard_fn(function(stages)
            if flush_count < 3 then
                flush_count = flush_count + 1
                return HazardResponse.new({ action = "flush", redirect_pc = 0 })
            end
            return HazardResponse.new({ action = "none" })
        end)

        p:run(6)
        assert.are.equal(3, p:get_stats().flush_cycles)
    end)

    it("snapshot.flushing is true during flush cycle", function()
        local flushed = false
        local function fetch(pc)    return 0 end
        local function decode(r, t) return t end
        local function execute(t)   return t end
        local function memory(t)    return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        p:set_hazard_fn(function(stages)
            if not flushed then
                flushed = true
                return HazardResponse.new({ action = "flush", redirect_pc = 0 })
            end
            return HazardResponse.new({ action = "none" })
        end)

        local snap = p:step()
        assert.is_true(snap.flushing)
    end)

end)

-- ========================================================================
-- Forwarding tests
-- ========================================================================

describe("Forwarding", function()

    it("forward_from_ex action is passed to hazard callback", function()
        local forwarded = false
        local function fetch(pc)    return 0 end
        local function decode(r, t) return t end
        local function execute(t)   return t end
        local function memory(t)    return t end
        local function writeback(t) end

        local result = Pipeline.new(
            PipelineConfig.classic_5_stage(),
            fetch, decode, execute, memory, writeback
        )
        local p = result.pipeline

        p:set_hazard_fn(function(stages)
            if not forwarded then
                forwarded = true
                return HazardResponse.new({
                    action         = "forward_from_ex",
                    forward_value  = 42,
                    forward_source = "EX",
                })
            end
            return HazardResponse.new({ action = "none" })
        end)

        -- Just verify the pipeline doesn't crash and doesn't stall
        local snap = p:step()
        assert.is_false(snap.stalled)
        assert.is_false(snap.flushing)
        local stats = p:get_stats()
        assert.are.equal(0, stats.stall_cycles)
    end)

end)

-- ========================================================================
-- Statistics tests
-- ========================================================================

describe("PipelineStats", function()

    it("IPC is 0 before any cycles", function()
        local p = make_5stage()
        local stats = p:get_stats()
        assert.are.equal(0.0, stats:ipc())
    end)

    it("CPI is 0 before any instructions complete", function()
        local p = make_5stage()
        -- Step 3 times — not enough to complete any instruction (need 5 to fill)
        for i = 1, 3 do p:step() end
        local stats = p:get_stats()
        -- 0 instructions completed yet
        assert.are.equal(0, stats.instructions_completed)
        assert.are.equal(0.0, stats:cpi())
    end)

    it("IPC approaches 1.0 for independent instructions", function()
        local p = make_5stage()
        p:run(100)
        local stats = p:get_stats()
        -- 100 cycles total, 96 instructions completed (first 4 cycles fill the pipe)
        assert.are.equal(100, stats.total_cycles)
        assert.are.equal(96,  stats.instructions_completed)
        local ipc = stats:ipc()
        assert.is_true(ipc > 0.9, "IPC should be > 0.9 for independent instructions")
    end)

    it("to_string() returns non-empty string", function()
        local p = make_5stage()
        p:run(10)
        local s = p:get_stats():to_string()
        assert.truthy(#s > 0)
        assert.truthy(s:find("IPC"))
    end)

end)

-- ========================================================================
-- Trace / snapshot tests
-- ========================================================================

describe("Trace and snapshots", function()

    it("trace() returns one snapshot per step", function()
        local p = make_5stage()
        for i = 1, 5 do p:step() end
        local trace = p:get_trace()
        assert.are.equal(5, #trace)
    end)

    it("trace snapshots have incrementing cycle numbers", function()
        local p = make_5stage()
        for i = 1, 3 do p:step() end
        local trace = p:get_trace()
        for i, snap in ipairs(trace) do
            assert.are.equal(i, snap.cycle)
        end
    end)

    it("snapshot.pc matches the pipeline PC at that cycle", function()
        local p = make_5stage()
        p:set_pc(0)
        local snap = p:step()
        -- After first step, PC advanced to 4
        assert.are.equal(4, snap.pc)
    end)

    it("stage_contents() returns nil for empty stages", function()
        local p = make_5stage()
        -- Before any step, all stages are empty
        assert.is_nil(p:stage_contents("WB"))
    end)

    it("to_string() on snapshot contains cycle info", function()
        local p = make_5stage()
        local snap = p:step()
        local s = snap:to_string()
        assert.truthy(s:find("cycle 1"))
    end)

end)

-- ========================================================================
-- 3-stage pipeline configuration test
-- ========================================================================

describe("Custom 3-stage pipeline", function()

    it("runs without error with a 3-stage config", function()
        local cfg = PipelineConfig.new({
            PipelineStage.new("IF", "Instruction Fetch",      "fetch"),
            PipelineStage.new("EX", "Decode + Execute",       "execute"),
            PipelineStage.new("WB", "Memory + Write Back",    "writeback"),
        })
        local p = noop_pipeline(cfg)
        p:run(10)
        local stats = p:get_stats()
        assert.are.equal(10, stats.total_cycles)
        -- 10 - 2 = 8 instructions complete (2 cycles to fill 3-stage pipe)
        assert.are.equal(8, stats.instructions_completed)
    end)

end)
