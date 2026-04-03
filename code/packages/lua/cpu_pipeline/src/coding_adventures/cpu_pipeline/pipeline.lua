-- pipeline.lua — The configurable N-stage instruction pipeline
--
-- This is the heart of the CPU simulator. The Pipeline manages the flow
-- of PipelineTokens through stages, handling stalls, flushes, and forwarding.
-- It does NOT know what instructions mean — that is the ISA decoder's job.
--
-- DESIGN: The pipeline is a table with a `stages` list (one slot per stage).
-- Each slot holds a PipelineToken (or nil). On each call to step(), the
-- pipeline:
--
--   1. Asks the hazard detector what to do this cycle
--   2. Computes the next state (shift, stall, or flush)
--   3. Runs stage callbacks (decode, execute, memory, writeback)
--   4. Records a snapshot for tracing
--   5. Returns the snapshot
--
-- CLOCK MODEL: All stage transitions happen "simultaneously" — we compute
-- the full next state before committing any changes. This matches real
-- hardware, where pipeline registers all update on the same clock edge.
--
-- ANALOGY — The Assembly Line:
--
--   A car factory has 5 workstations. Each car moves to the next station
--   every hour. While station 3 welds the chassis, station 2 installs
--   the engine, station 1 paints the body, etc.
--
--   If station 3 needs parts that station 4 hasn't delivered yet (a hazard),
--   the line STALLS: stations 1, 2, 3 freeze, and station 3 re-does its work
--   next hour. Stations 4 and 5 continue normally; a dummy car (bubble) is
--   inserted at station 3.
--
--   If the factory realizes it built the WRONG car for the last 2 stations
--   (a mispredicted branch), it FLUSHES those 2 stations: those cars are
--   scrapped and replaced with dummy cars, and the correct car is started.

local Token       = require("coding_adventures.cpu_pipeline.token")
local config_mod  = require("coding_adventures.cpu_pipeline.config")

local PipelineConfig = config_mod.PipelineConfig
local HazardResponse = config_mod.HazardResponse
local PipelineStats  = config_mod.PipelineStats
local Snapshot       = config_mod.Snapshot

-- ========================================================================
-- Pipeline
-- ========================================================================

local Pipeline = {}
Pipeline.__index = Pipeline

--- Creates a new pipeline.
--
-- All five stage callbacks are required. Hazard and predict callbacks are
-- optional (set with set_hazard_fn and set_predict_fn after construction).
--
-- Callback signatures:
--   fetch_fn(pc)           → raw_instruction (integer)
--   decode_fn(raw, token)  → decoded token
--   execute_fn(token)      → token with alu_result filled in
--   memory_fn(token)       → token with mem_data filled in (for loads)
--   writeback_fn(token)    → nil (writes to register file, no return)
--   hazard_fn(stages)      → HazardResponse
--   predict_fn(pc)         → next_pc (integer)
--
-- Returns {ok=true, pipeline=...} or {ok=false, err="..."}.
--
-- @param config       PipelineConfig
-- @param fetch_fn     function
-- @param decode_fn    function
-- @param execute_fn   function
-- @param memory_fn    function
-- @param writeback_fn function
-- @return table
function Pipeline.new(config, fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn)
    local ok, err = config:validate()
    if not ok then
        return { ok = false, err = err }
    end

    -- Initialize each stage slot to nil (empty)
    local stages = {}
    for i = 1, config:num_stages() do
        stages[i] = nil
    end

    local p = setmetatable({
        config       = config,
        stages       = stages,    -- list of Token|nil
        pc           = 0,
        cycle        = 0,
        halted       = false,
        stats        = PipelineStats.new(),
        history      = {},        -- list of Snapshot objects
        fetch_fn     = fetch_fn,
        decode_fn    = decode_fn,
        execute_fn   = execute_fn,
        memory_fn    = memory_fn,
        writeback_fn = writeback_fn,
        hazard_fn    = nil,
        predict_fn   = nil,
    }, Pipeline)

    return { ok = true, pipeline = p }
end

--- Sets the optional hazard detection callback.
function Pipeline:set_hazard_fn(fn)
    self.hazard_fn = fn
end

--- Sets the optional branch prediction callback (returns next PC).
function Pipeline:set_predict_fn(fn)
    self.predict_fn = fn
end

--- Sets the program counter.
function Pipeline:set_pc(pc)
    self.pc = pc
end

--- Returns the current program counter.
function Pipeline:get_pc()
    return self.pc
end

--- Returns true if a halt instruction has completed.
function Pipeline:is_halted()
    return self.halted
end

--- Returns the current cycle number.
function Pipeline:get_cycle()
    return self.cycle
end

--- Returns the current pipeline statistics.
function Pipeline:get_stats()
    return self.stats
end

--- Returns the pipeline configuration.
function Pipeline:get_config()
    return self.config
end

--- Returns the complete history of snapshots.
function Pipeline:get_trace()
    local result = {}
    for i = #self.history, 1, -1 do
        result[#result + 1] = self.history[i]
    end
    return result
end

--- Returns the token currently in the named stage (or nil).
function Pipeline:stage_contents(stage_name)
    for i, stage_def in ipairs(self.config.stages) do
        if stage_def.name == stage_name then
            return self.stages[i]
        end
    end
    return nil
end

--- Returns a snapshot of the current pipeline state without stepping.
function Pipeline:snapshot()
    return self:_take_snapshot()
end

-- ========================================================================
-- step() — advance the pipeline by one clock cycle
-- ========================================================================
--
-- This is the core simulation loop. Each call corresponds to one rising
-- clock edge in real hardware.
--
-- The step proceeds in five phases:
--   Phase 1: Query the hazard detector
--   Phase 2: Compute the next stage contents (shift/stall/flush)
--   Phase 3: Commit the new stage state
--   Phase 4: Run stage callbacks (decode, execute, memory, writeback)
--   Phase 5: Record snapshot and return it
--
-- Returns a Snapshot of the pipeline after this cycle.

function Pipeline:step()
    if self.halted then
        return self:_take_snapshot()
    end

    self.cycle = self.cycle + 1
    self.stats.total_cycles = self.stats.total_cycles + 1

    local num_stages = self.config:num_stages()

    -- ---- Phase 1: Check for hazards ----
    -- Build the "would-be next stages" (after a normal shift) so the hazard
    -- detector sees the pipeline state it will act upon — not the stale state.
    -- Stage i+1 receives what is currently in stage i; stage 1 gets nil (new
    -- fetch will fill it), shifted indices are {nil, stages[1], stages[2], ...}.
    local hazard
    if self.hazard_fn then
        local next_preview = {}
        next_preview[1] = nil  -- new instruction not yet fetched
        for i = 2, num_stages do
            next_preview[i] = self.stages[i - 1]
        end
        hazard = self.hazard_fn(next_preview)
    else
        hazard = HazardResponse.new({ action = "none" })
    end

    -- ---- Phase 2: Compute next state ----
    local stalled  = false
    local flushing = false

    if hazard.action == "flush" then
        self:_apply_flush(hazard, num_stages)
        flushing = true
    elseif hazard.action == "stall" then
        self:_apply_stall(hazard, num_stages)
        stalled = true
    else
        -- none / forward_from_ex / forward_from_mem
        if hazard.action == "forward_from_ex" or hazard.action == "forward_from_mem" then
            self:_apply_forwarding(hazard)
        end
        self:_shift_stages(num_stages)
    end

    -- ---- Phase 4: Run stage callbacks ----
    self:_execute_stage_callbacks(num_stages)

    -- ---- Phase 4b: Count bubble cycles ----
    for i = 1, num_stages do
        local tok = self.stages[i]
        if tok ~= nil and tok.is_bubble then
            self.stats.bubble_cycles = self.stats.bubble_cycles + 1
        end
    end

    -- ---- Phase 4c: Retire last stage ----
    self:_retire_last_stage(num_stages)

    -- ---- Phase 5: Snapshot ----
    local snap = Snapshot.new(self.cycle, self:_build_stage_map(), stalled, flushing, self.pc)
    -- history is stored in reverse (newest first), like Elixir
    table.insert(self.history, 1, snap)

    return snap
end

--- Runs the pipeline until halted or max_cycles reached.
-- Returns the final PipelineStats.
function Pipeline:run(max_cycles)
    max_cycles = max_cycles or 10000
    while not self.halted and self.cycle < max_cycles do
        self:step()
    end
    return self.stats
end

-- ========================================================================
-- Internal helpers
-- ========================================================================

-- Compute where to flush from and how many stages to flush.
-- Default: flush all stages up to (not including) the first execute stage.
local function determine_flush_count(hazard, config, num_stages)
    if hazard.flush_count and hazard.flush_count > 0 then
        return math.min(hazard.flush_count, num_stages)
    end
    -- Find the first execute stage index (0-based in spec, 1-based here)
    local idx = 0
    for i, stage in ipairs(config.stages) do
        if stage.category == "execute" then
            idx = i - 1  -- number of stages before it
            break
        end
    end
    local fc = idx > 0 and idx or 1
    return math.min(fc, num_stages)
end

-- Compute where to insert the bubble stall.
-- Default: at the first execute stage.
local function determine_stall_point(hazard, config, num_stages)
    if hazard.stall_stages and hazard.stall_stages > 0 then
        return math.min(hazard.stall_stages, num_stages - 1)
    end
    local idx = 1  -- default: index of first execute stage (1-based)
    for i, stage in ipairs(config.stages) do
        if stage.category == "execute" then
            idx = i
            break
        end
    end
    return math.min(idx, num_stages - 1)
end

-- Apply a flush: replace early stages with bubbles, shift the rest.
function Pipeline:_apply_flush(hazard, num_stages)
    self.stats.flush_cycles = self.stats.flush_cycles + 1

    local flush_count = determine_flush_count(hazard, self.config, num_stages)
    local old_stages  = self.stages
    local new_stages  = {}

    for i = 1, num_stages do
        if i <= flush_count then
            -- Flushed stage: bubble
            local b = Token.new_bubble()
            b.stage_entered[self.config.stages[i].name] = self.cycle
            new_stages[i] = b
        elseif i == flush_count + 1 then
            -- Boundary: bubble
            local b = Token.new_bubble()
            b.stage_entered[self.config.stages[i].name] = self.cycle
            new_stages[i] = b
        else
            -- Shift from previous
            new_stages[i] = old_stages[i - 1]
        end
    end

    -- Redirect PC and fetch from the correct target.
    self.pc = hazard.redirect_pc
    local tok = self:_fetch_new_instruction()
    new_stages[1] = tok
    self:_advance_pc()

    self.stages = new_stages
end

-- Apply a stall: freeze stages before stall_point, insert bubble at stall_point.
function Pipeline:_apply_stall(hazard, num_stages)
    self.stats.stall_cycles = self.stats.stall_cycles + 1

    local stall_point = determine_stall_point(hazard, self.config, num_stages)
    local old_stages  = self.stages
    local new_stages  = {}

    for i = 1, num_stages do
        if i > stall_point then
            -- Advance normally
            new_stages[i] = old_stages[i - 1]
        elseif i == stall_point then
            -- Insert bubble at stall point
            local b = Token.new_bubble()
            b.stage_entered[self.config.stages[i].name] = self.cycle
            new_stages[i] = b
        else
            -- Freeze: keep in place
            new_stages[i] = old_stages[i]
        end
    end

    -- PC does NOT advance during a stall
    self.stages = new_stages
end

-- Apply forwarding: update the token in the decode stage with the forwarded value.
function Pipeline:_apply_forwarding(hazard)
    for i, stage_def in ipairs(self.config.stages) do
        local tok = self.stages[i]
        if stage_def.category == "decode" and tok ~= nil and not tok.is_bubble then
            tok.alu_result     = hazard.forward_value
            tok.forwarded_from = hazard.forward_source
        end
    end
end

-- Normal advance: shift each token one stage forward.
function Pipeline:_shift_stages(num_stages)
    local old_stages = self.stages
    local new_stages = {}

    for i = 1, num_stages do
        if i > 1 then
            new_stages[i] = old_stages[i - 1]
        else
            new_stages[i] = nil  -- will be filled by fetch below
        end
    end

    -- Fetch new instruction into IF stage
    local tok = self:_fetch_new_instruction()
    new_stages[1] = tok
    self:_advance_pc()

    self.stages = new_stages
end

-- Fetch a new instruction token at the current PC.
function Pipeline:_fetch_new_instruction()
    local tok           = Token.new()
    tok.pc              = self.pc
    tok.raw_instruction = self.fetch_fn(self.pc)
    local stage_name    = self.config.stages[1].name
    tok.stage_entered[stage_name] = self.cycle
    return tok
end

-- Advance the PC (with optional branch prediction).
function Pipeline:_advance_pc()
    if self.predict_fn then
        self.pc = self.predict_fn(self.pc)
    else
        self.pc = self.pc + 4
    end
end

-- Run the stage callbacks in reverse order (last → first).
-- We iterate in reverse so later stages see earlier-stage results in
-- the same cycle only if they were already computed. In practice,
-- stage callbacks are independent for a given cycle.
function Pipeline:_execute_stage_callbacks(num_stages)
    for i = num_stages, 1, -1 do
        local tok      = self.stages[i]
        local stage_def = self.config.stages[i]

        if tok == nil or tok.is_bubble then
            -- Nothing to do for empty stages or bubbles
        else
            -- Record when this token entered this stage
            if not tok.stage_entered[stage_def.name] then
                tok.stage_entered[stage_def.name] = self.cycle
            end

            local cat = stage_def.category

            if cat == "fetch" then
                -- Already handled in _fetch_new_instruction
            elseif cat == "decode" then
                if tok.opcode == "" then
                    local decoded = self.decode_fn(tok.raw_instruction, tok)
                    self.stages[i] = decoded
                end
            elseif cat == "execute" then
                if tok.stage_entered[stage_def.name] == self.cycle then
                    local result = self.execute_fn(tok)
                    self.stages[i] = result
                end
            elseif cat == "memory" then
                if tok.stage_entered[stage_def.name] == self.cycle then
                    local result = self.memory_fn(tok)
                    self.stages[i] = result
                end
            elseif cat == "writeback" then
                -- Writeback is handled in _retire_last_stage
            end
        end
    end
end

-- Retire the last stage: call writeback, count instruction, check halt.
function Pipeline:_retire_last_stage(num_stages)
    local last_tok = self.stages[num_stages]
    if last_tok ~= nil and not last_tok.is_bubble then
        self.writeback_fn(last_tok)
        self.stats.instructions_completed = self.stats.instructions_completed + 1
        if last_tok.is_halt then
            self.halted = true
        end
    end
end

-- Build a map of stage_name → token clone (for snapshot).
function Pipeline:_build_stage_map()
    local m = {}
    for i, stage_def in ipairs(self.config.stages) do
        local tok = self.stages[i]
        if tok ~= nil then
            m[stage_def.name] = Token.clone(tok)
        end
    end
    return m
end

-- Take a snapshot of the current pipeline state.
function Pipeline:_take_snapshot()
    return Snapshot.new(self.cycle, self:_build_stage_map(), false, false, self.pc)
end

-- Export classic_5_stage and deep_13_stage as pipeline-level helpers
Pipeline.classic_5_stage = PipelineConfig.classic_5_stage
Pipeline.deep_13_stage   = PipelineConfig.deep_13_stage

return Pipeline
