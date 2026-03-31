-- =============================================================================
-- Pipeline — configurable N-stage instruction pipeline engine
-- =============================================================================
--
-- The pipeline is the central execution engine.  It manages the flow of
-- PipelineTokens through the stages without knowing anything about what the
-- tokens mean (that is the ISA decoder's job).
--
-- ASSEMBLY LINE ANALOGY
-- =====================
-- Imagine a car factory with 5 stations:
--   Station 1: weld chassis
--   Station 2: install engine
--   Station 3: attach wheels
--   Station 4: paint
--   Station 5: quality check
--
-- Each car moves one station per shift.  Multiple cars are in production
-- simultaneously — while one is being painted, another is getting wheels.
-- This is exactly how a CPU pipeline works.
--
-- CLOCK-DRIVEN MODEL
-- ==================
-- Each call to step() corresponds to one rising clock edge.  All stage
-- transitions happen "simultaneously" (in hardware, latched by flip-flops).
-- In software we model this by computing the NEXT state of every slot before
-- committing any change.
--
-- STAGE CALLBACKS
-- ===============
-- The pipeline fires a callback for each stage category:
--   fetch_fn(pc)           -> raw_instruction (int)
--   decode_fn(raw, token)  -> token (with opcode/rs1/rs2/rd filled in)
--   execute_fn(token)      -> token (with alu_result/branch_taken filled in)
--   memory_fn(token)       -> token (with mem_data filled in)
--   writeback_fn(token)    -> nil  (writes to register file, no return)
--
-- Optional:
--   hazard_fn(stages_list) -> HazardResponse
--   predict_fn(pc)         -> next_pc (branch prediction)

local Token        = require("coding_adventures.cpu_pipeline.token")
local config_mod   = require("coding_adventures.cpu_pipeline.config")
local PipelineConfig = config_mod.PipelineConfig
local HazardResponse = config_mod.HazardResponse
local PipelineStats  = config_mod.PipelineStats
local Snapshot       = config_mod.Snapshot

local Pipeline = {}
Pipeline.__index = Pipeline

-- ---------------------------------------------------------------------------
-- Pipeline.new(config, fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn)
-- ---------------------------------------------------------------------------
-- Returns {ok=true, pipeline=<Pipeline>} or {ok=false, err="reason"}.
--
-- All five stage callbacks are required.  Hazard and predict callbacks are
-- optional and can be set later with set_hazard_fn / set_predict_fn.
function Pipeline.new(config, fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn)
    local ok, err = PipelineConfig.validate(config)
    if not ok then
        return {ok = false, err = err}
    end

    local n = PipelineConfig.num_stages(config)

    -- Build the initial slot list: every stage starts empty (nil)
    local stages = {}
    for i = 1, n do
        stages[i] = nil
    end

    local self = setmetatable({}, Pipeline)
    self.config       = config
    self.stages       = stages      -- stages[i] = Token or nil
    self.pc           = 0
    self.cycle        = 0
    self.halted       = false
    self.stats        = PipelineStats.new()
    self.history      = {}          -- list of Snapshot (oldest-first for trace())

    -- Callbacks (required)
    self.fetch_fn     = fetch_fn
    self.decode_fn    = decode_fn
    self.execute_fn   = execute_fn
    self.memory_fn    = memory_fn
    self.writeback_fn = writeback_fn

    -- Optional callbacks
    self.hazard_fn    = nil
    self.predict_fn   = nil

    return {ok = true, pipeline = self}
end

-- Attach an optional hazard detection callback.
-- hazard_fn(stages_list) -> HazardResponse
function Pipeline:set_hazard_fn(fn)
    self.hazard_fn = fn
end

-- Attach an optional branch prediction callback.
-- predict_fn(pc) -> next_pc
function Pipeline:set_predict_fn(fn)
    self.predict_fn = fn
end

-- Set the program counter (called before the first step).
function Pipeline:set_pc(pc)
    self.pc = pc
end

function Pipeline:get_pc()
    return self.pc
end

function Pipeline:is_halted()
    return self.halted
end

function Pipeline:get_cycle()
    return self.cycle
end

function Pipeline:get_stats()
    return self.stats
end

function Pipeline:get_config()
    return self.config
end

-- ---------------------------------------------------------------------------
-- Pipeline:get_trace() — return snapshot history in chronological order
-- ---------------------------------------------------------------------------
-- The history list is stored in insertion order (oldest first) and returned
-- directly.  This matches the Elixir reference which stores newest-first in
-- history but reverses for trace().
function Pipeline:get_trace()
    local result = {}
    for i = 1, #self.history do
        result[i] = self.history[i]
    end
    return result
end

-- ---------------------------------------------------------------------------
-- Pipeline:snapshot() — current state without advancing the clock
-- ---------------------------------------------------------------------------
function Pipeline:snapshot()
    return self:_build_snapshot(false, false)
end

-- ---------------------------------------------------------------------------
-- Pipeline:step() — advance by one clock cycle
-- ---------------------------------------------------------------------------
-- Returns the Snapshot for this cycle.
--
-- Phase 1: Check for hazards
-- Phase 2: Compute the next stage contents (stall / flush / normal shift)
-- Phase 3: Commit new stage contents
-- Phase 4: Fire stage callbacks (decode, execute, memory)
-- Phase 5: Retire the last stage (writeback)
-- Phase 6: Record snapshot
function Pipeline:step()
    if self.halted then
        return self:snapshot()
    end

    self.cycle = self.cycle + 1
    self.stats.total_cycles = self.stats.total_cycles + 1

    local n = PipelineConfig.num_stages(self.config)

    -- -----------------------------------------------------------------------
    -- Phase 1: Hazard detection
    -- -----------------------------------------------------------------------
    local hazard
    if self.hazard_fn then
        hazard = self.hazard_fn(self.stages)
    else
        hazard = HazardResponse.new({action = "none"})
    end

    -- -----------------------------------------------------------------------
    -- Phase 2 & 3: Compute and commit next stage contents
    -- -----------------------------------------------------------------------
    local stalled  = false
    local flushing = false

    if hazard.action == "flush" then
        stalled, flushing = false, true
        self:_apply_flush(hazard, n)
    elseif hazard.action == "stall" then
        stalled, flushing = true, false
        self:_apply_stall(hazard, n)
    else
        -- forward_from_ex, forward_from_mem, or none
        if hazard.action == "forward_from_ex" or hazard.action == "forward_from_mem" then
            self:_apply_forwarding(hazard)
        end
        self:_shift_stages(n)
    end

    -- -----------------------------------------------------------------------
    -- Phase 4: Fire stage callbacks
    -- -----------------------------------------------------------------------
    -- We iterate from last to first so that writeback (which we handle in
    -- Phase 5) does not interfere with earlier stages reading it in the same
    -- cycle.  Callbacks only run on real (non-bubble) tokens.
    for i = n, 1, -1 do
        local tok = self.stages[i]
        if tok ~= nil and not tok.is_bubble then
            local stage = self.config.stages[i]

            -- Record the cycle this token first entered this stage
            if not tok.stage_entered[stage.name] then
                tok.stage_entered[stage.name] = self.cycle
            end

            local cat = stage.category
            if cat == "decode" then
                -- Only decode once (opcode == "" means not yet decoded)
                if tok.opcode == "" then
                    local decoded = self.decode_fn(tok.raw_instruction, tok)
                    self.stages[i] = decoded
                end
            elseif cat == "execute" then
                if tok.stage_entered[stage.name] == self.cycle then
                    local executed = self.execute_fn(tok)
                    self.stages[i] = executed
                end
            elseif cat == "memory" then
                if tok.stage_entered[stage.name] == self.cycle then
                    local result = self.memory_fn(tok)
                    self.stages[i] = result
                end
            end
            -- fetch: handled inside _shift_stages / _apply_flush
            -- writeback: handled in Phase 5
        end
    end

    -- -----------------------------------------------------------------------
    -- Phase 5: Retire the last stage (writeback)
    -- -----------------------------------------------------------------------
    local last_tok = self.stages[n]
    if last_tok ~= nil and not last_tok.is_bubble then
        self.writeback_fn(last_tok)
        self.stats.instructions_completed = self.stats.instructions_completed + 1
        if last_tok.is_halt then
            self.halted = true
        end
    end

    -- Count bubble occupancy
    for i = 1, n do
        local tok = self.stages[i]
        if tok ~= nil and tok.is_bubble then
            self.stats.bubble_cycles = self.stats.bubble_cycles + 1
        end
    end

    -- -----------------------------------------------------------------------
    -- Phase 6: Build and record snapshot
    -- -----------------------------------------------------------------------
    local snap = self:_build_snapshot(stalled, flushing)
    self.history[#self.history + 1] = snap

    return snap
end

-- ---------------------------------------------------------------------------
-- Pipeline:run(max_cycles) — run until halt or max_cycles
-- ---------------------------------------------------------------------------
-- Returns PipelineStats.
function Pipeline:run(max_cycles)
    max_cycles = max_cycles or 10000
    while not self.halted and self.cycle < max_cycles do
        self:step()
    end
    return self.stats
end

-- ---------------------------------------------------------------------------
-- Internal helpers
-- ---------------------------------------------------------------------------

-- _fetch_new_instruction() — fetch one instruction from memory into IF slot
local function fetch_instruction(pipeline)
    local tok = Token.new()
    tok.pc              = pipeline.pc
    tok.raw_instruction = pipeline.fetch_fn(pipeline.pc)
    -- Record when this token entered the first (IF) stage
    tok.stage_entered[pipeline.config.stages[1].name] = pipeline.cycle
    return tok
end

-- _advance_pc() — move PC to next instruction address
local function advance_pc(pipeline)
    if pipeline.predict_fn then
        pipeline.pc = pipeline.predict_fn(pipeline.pc)
    else
        pipeline.pc = pipeline.pc + 4
    end
end

-- _determine_flush_count — how many front stages to replace with bubbles?
-- If the HazardResponse specifies flush_count > 0, use that.
-- Otherwise, flush up to (but not including) the first execute stage.
local function determine_flush_count(hazard, config, n)
    if hazard.flush_count > 0 then
        return math.min(hazard.flush_count, n)
    end
    -- Find the first execute-category stage index (1-based)
    for i, stage in ipairs(config.stages) do
        if stage.category == "execute" then
            return math.min(i - 1, n)
        end
    end
    return math.min(1, n)
end

-- _determine_stall_point — index of stage where bubble is inserted (1-based)
local function determine_stall_point(hazard, config, n)
    if hazard.stall_stages > 0 then
        return math.min(hazard.stall_stages, n)
    end
    for i, stage in ipairs(config.stages) do
        if stage.category == "execute" then
            return math.min(i, n)
        end
    end
    return math.min(2, n)
end

-- _apply_flush: redirect PC, insert bubbles in front stages, shift the rest
function Pipeline:_apply_flush(hazard, n)
    self.stats.flush_cycles = self.stats.flush_cycles + 1

    local flush_count = determine_flush_count(hazard, self.config, n)
    local next_stages = {}

    for i = 1, n do
        if i <= flush_count then
            -- Replace flushed stage with bubble
            local b = Token.new_bubble()
            b.stage_entered[self.config.stages[i].name] = self.cycle
            next_stages[i] = b
        elseif i > flush_count + 1 then
            -- Shift from previous
            next_stages[i] = self.stages[i - 1]
        else
            -- Boundary: insert bubble
            local b = Token.new_bubble()
            b.stage_entered[self.config.stages[i].name] = self.cycle
            next_stages[i] = b
        end
    end

    -- Redirect PC to branch target
    self.pc = hazard.redirect_pc

    -- Fetch from the correct path
    local tok = fetch_instruction(self)
    next_stages[1] = tok
    advance_pc(self)

    self.stages = next_stages
end

-- _apply_stall: freeze stages 1..stall_point, insert bubble at stall_point,
--               advance stages stall_point+1..n normally
function Pipeline:_apply_stall(hazard, n)
    self.stats.stall_cycles = self.stats.stall_cycles + 1

    local stall_point = determine_stall_point(hazard, self.config, n)
    local next_stages = {}

    for i = 1, n do
        if i > stall_point then
            -- Advance from previous
            next_stages[i] = self.stages[i - 1]
        elseif i == stall_point then
            -- Insert bubble here
            local b = Token.new_bubble()
            b.stage_entered[self.config.stages[i].name] = self.cycle
            next_stages[i] = b
        else
            -- Freeze: keep current content
            next_stages[i] = self.stages[i]
        end
    end

    -- PC does NOT advance during a stall
    self.stages = next_stages
end

-- _apply_forwarding: update the decode-stage token's alu_result with
--                    the forwarded value from EX or MEM
function Pipeline:_apply_forwarding(hazard)
    for i, tok in ipairs(self.stages) do
        if tok ~= nil and not tok.is_bubble then
            local stage = self.config.stages[i]
            if stage.category == "decode" then
                tok.alu_result      = hazard.forward_value
                tok.forwarded_from  = hazard.forward_source
            end
        end
    end
end

-- _shift_stages: normal (no hazard) advance — token at stage i moves to i+1
function Pipeline:_shift_stages(n)
    local next_stages = {}
    for i = 1, n do
        if i > 1 then
            next_stages[i] = self.stages[i - 1]
        else
            next_stages[i] = nil
        end
    end

    -- Fetch new instruction into IF (stage 1)
    local tok = fetch_instruction(self)
    next_stages[1] = tok
    advance_pc(self)

    self.stages = next_stages
end

-- _build_snapshot — deep-copy current stage contents into a Snapshot
function Pipeline:_build_snapshot(stalled, flushing)
    local stage_map = {}
    for i, stage in ipairs(self.config.stages) do
        local tok = self.stages[i]
        if tok ~= nil then
            stage_map[stage.name] = Token.clone(tok)
        end
    end
    return Snapshot.new(self.cycle, stage_map, stalled or false, flushing or false, self.pc)
end

return Pipeline
