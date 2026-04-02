-- =============================================================================
-- Pipeline configuration types
-- =============================================================================
--
-- PipelineStage     — definition of one stage (name, description, category)
-- PipelineConfig    — the full pipeline: ordered list of stages + width
-- HazardResponse    — what the hazard unit tells the pipeline to do
-- PipelineStats     — running counters (ipc, cpi, stalls, flushes, …)
-- Snapshot          — complete pipeline state at one point in time

-- ---------------------------------------------------------------------------
-- PipelineStage
-- ---------------------------------------------------------------------------
-- Category determines which callback the pipeline engine fires for this
-- stage:  :fetch / :decode / :execute / :memory / :writeback
--
-- Using string constants (not Lua enums) keeps serialisation simple.

local PipelineStage = {}
PipelineStage.__index = PipelineStage

-- new(name, description, category)
-- category defaults to "execute" when omitted
function PipelineStage.new(name, description, category)
    local self = setmetatable({}, PipelineStage)
    self.name        = name
    self.description = description or ""
    self.category    = category or "execute"
    return self
end

-- ---------------------------------------------------------------------------
-- PipelineConfig
-- ---------------------------------------------------------------------------
-- Wraps an ordered list of PipelineStage objects and the execution width
-- (1 = scalar; >1 = superscalar — future extension).

local PipelineConfig = {}
PipelineConfig.__index = PipelineConfig

function PipelineConfig.new(stages, execution_width)
    local self = setmetatable({}, PipelineConfig)
    self.stages          = stages or {}
    self.execution_width = execution_width or 1
    return self
end

-- validate(config) -> ok (bool), err (string|nil)
-- Returns true, nil on success; false, "reason" on failure.
-- The pipeline constructor calls this before creating the pipeline struct.
function PipelineConfig.validate(config)
    if config == nil then
        return false, "config is nil"
    end
    if type(config.stages) ~= "table" or #config.stages == 0 then
        return false, "stages must be a non-empty list"
    end
    for i, s in ipairs(config.stages) do
        if type(s.name) ~= "string" or s.name == "" then
            return false, string.format("stage %d has no name", i)
        end
        local cat = s.category
        local valid = cat == "fetch" or cat == "decode" or cat == "execute"
                   or cat == "memory" or cat == "writeback"
        if not valid then
            return false, string.format("stage '%s' has unknown category '%s'", s.name, tostring(cat))
        end
    end
    if type(config.execution_width) ~= "number" or config.execution_width < 1 then
        return false, "execution_width must be >= 1"
    end
    return true, nil
end

function PipelineConfig.num_stages(config)
    return #config.stages
end

-- ---------------------------------------------------------------------------
-- Preset: classic 5-stage RISC pipeline (MIPS R2000, 1985)
--
--   IF → ID → EX → MEM → WB
--
-- This is the pipeline every computer architecture textbook teaches.
-- Five stages, each taking exactly one clock cycle.
-- ---------------------------------------------------------------------------
function PipelineConfig.classic_5_stage()
    return PipelineConfig.new({
        PipelineStage.new("IF",  "Instruction Fetch",  "fetch"),
        PipelineStage.new("ID",  "Instruction Decode", "decode"),
        PipelineStage.new("EX",  "Execute",            "execute"),
        PipelineStage.new("MEM", "Memory Access",      "memory"),
        PipelineStage.new("WB",  "Write Back",         "writeback"),
    }, 1)
end

-- ---------------------------------------------------------------------------
-- Preset: 13-stage pipeline inspired by ARM Cortex-A78
--
-- Modern high-performance cores split the classic five stages into many
-- sub-stages to enable higher clock frequencies.  The tradeoff: a branch
-- misprediction now costs 10+ cycles instead of 2.
--
-- ARM Cortex-A78 (used in Snapdragon 888, Dimensity 9000)
--   3 fetch sub-stages → 3 decode sub-stages → 3 execute sub-stages
--   → 3 memory sub-stages → 1 writeback
-- ---------------------------------------------------------------------------
function PipelineConfig.deep_13_stage()
    return PipelineConfig.new({
        PipelineStage.new("IF1",  "Fetch 1 - TLB lookup",       "fetch"),
        PipelineStage.new("IF2",  "Fetch 2 - cache read",        "fetch"),
        PipelineStage.new("IF3",  "Fetch 3 - align/buffer",      "fetch"),
        PipelineStage.new("ID1",  "Decode 1 - pre-decode",       "decode"),
        PipelineStage.new("ID2",  "Decode 2 - full decode",      "decode"),
        PipelineStage.new("ID3",  "Decode 3 - register read",    "decode"),
        PipelineStage.new("EX1",  "Execute 1 - ALU",             "execute"),
        PipelineStage.new("EX2",  "Execute 2 - shift/multiply",  "execute"),
        PipelineStage.new("EX3",  "Execute 3 - result select",   "execute"),
        PipelineStage.new("MEM1", "Memory 1 - address calc",     "memory"),
        PipelineStage.new("MEM2", "Memory 2 - cache access",     "memory"),
        PipelineStage.new("MEM3", "Memory 3 - data align",       "memory"),
        PipelineStage.new("WB",   "Write Back",                  "writeback"),
    }, 1)
end

-- ---------------------------------------------------------------------------
-- HazardResponse — what the hazard detection callback returns
-- ---------------------------------------------------------------------------
-- action:        "none" | "stall" | "flush" | "forward_from_ex" | "forward_from_mem"
-- stall_stages:  insertion point (0 = let pipeline decide)
-- flush_count:   how many front stages to flush (0 = let pipeline decide)
-- redirect_pc:   target PC after a flush (for branch misprediction)
-- forward_value: the value to forward into the decode stage
-- forward_source: human-readable source tag ("EX", "MEM", …)

local HazardResponse = {}
HazardResponse.__index = HazardResponse

function HazardResponse.new(opts)
    opts = opts or {}
    local self = setmetatable({}, HazardResponse)
    self.action        = opts.action        or "none"
    self.stall_stages  = opts.stall_stages  or 0
    self.flush_count   = opts.flush_count   or 0
    self.redirect_pc   = opts.redirect_pc   or 0
    self.forward_value  = opts.forward_value  or 0
    self.forward_source = opts.forward_source or ""
    return self
end

-- ---------------------------------------------------------------------------
-- PipelineStats — execution counters
-- ---------------------------------------------------------------------------
-- These mirror the Python spec fields.  ipc() and cpi() are computed on
-- demand rather than stored, so they are always consistent.

local PipelineStats = {}
PipelineStats.__index = PipelineStats

function PipelineStats.new()
    local self = setmetatable({}, PipelineStats)
    self.total_cycles            = 0
    self.instructions_completed  = 0
    self.stall_cycles            = 0
    self.flush_cycles            = 0
    self.bubble_cycles           = 0
    return self
end

-- ipc() — Instructions Per Cycle
--   IPC = 1.0 for ideal pipelined execution (one instruction completes per cycle)
--   IPC < 1.0 when stalls/flushes reduce throughput
function PipelineStats:ipc()
    if self.total_cycles == 0 then return 0.0 end
    return self.instructions_completed / self.total_cycles
end

-- cpi() — Cycles Per Instruction (= 1 / IPC)
function PipelineStats:cpi()
    if self.instructions_completed == 0 then return 0.0 end
    return self.total_cycles / self.instructions_completed
end

function PipelineStats:to_string()
    return string.format(
        "PipelineStats{cycles=%d instr=%d stalls=%d flushes=%d ipc=%.3f}",
        self.total_cycles, self.instructions_completed,
        self.stall_cycles, self.flush_cycles, self:ipc()
    )
end

-- ---------------------------------------------------------------------------
-- Snapshot — the complete pipeline state at one clock cycle
-- ---------------------------------------------------------------------------

local Snapshot = {}
Snapshot.__index = Snapshot

-- stages is a table {stage_name -> Token (or nil)}
function Snapshot.new(cycle, stages, stalled, flushing, pc)
    local self = setmetatable({}, Snapshot)
    self.cycle    = cycle or 0
    self.stages   = stages or {}
    self.stalled  = stalled or false
    self.flushing = flushing or false
    self.pc       = pc or 0
    return self
end

return {
    PipelineStage  = PipelineStage,
    PipelineConfig = PipelineConfig,
    HazardResponse = HazardResponse,
    PipelineStats  = PipelineStats,
    Snapshot       = Snapshot,
}
