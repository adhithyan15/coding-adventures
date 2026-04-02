-- config.lua — Pipeline stage and configuration definitions
--
-- A pipeline's character is defined by its stage configuration:
--   - How many stages are there?
--   - What is each stage called?
--   - What category of work does each stage do?
--
-- This module provides PipelineStage (a single stage definition) and
-- PipelineConfig (the full configuration), plus two preset configurations:
--
--   PipelineConfig.classic_5_stage()  — the textbook RISC pipeline
--   PipelineConfig.deep_13_stage()    — inspired by ARM Cortex-A78
--
-- STAGE CATEGORIES: Every stage falls into one of five categories, which
-- tells the pipeline which callback to invoke:
--
--   "fetch"     — reads instruction bits from the instruction cache
--   "decode"    — decodes the instruction and reads register values
--   "execute"   — runs the ALU or resolves a branch
--   "memory"    — accesses data memory (loads and stores)
--   "writeback" — writes results back to the register file
--
-- A 5-stage pipeline has one stage per category. A 13-stage pipeline
-- may have 3 fetch stages, 2 decode stages, 3 execute stages, etc.

-- ========================================================================
-- PipelineStage
-- ========================================================================

local PipelineStage = {}
PipelineStage.__index = PipelineStage

--- Creates a new pipeline stage definition.
--
-- @param name        string  Short name for diagrams (e.g., "IF", "EX1")
-- @param description string  Human-readable description
-- @param category    string  One of: "fetch","decode","execute","memory","writeback"
-- @return table  A new PipelineStage
function PipelineStage.new(name, description, category)
    return setmetatable({
        name        = name,
        description = description,
        category    = category or "execute",
    }, PipelineStage)
end

function PipelineStage:to_string()
    return self.name
end

-- ========================================================================
-- PipelineConfig
-- ========================================================================

local PipelineConfig = {}
PipelineConfig.__index = PipelineConfig

--- Creates a new pipeline configuration.
--
-- @param stages          table  List of PipelineStage objects
-- @param execution_width number  Instructions per cycle (1 = scalar)
-- @return table  A new PipelineConfig
function PipelineConfig.new(stages, execution_width)
    return setmetatable({
        stages          = stages or {},
        execution_width = execution_width or 1,
    }, PipelineConfig)
end

--- Returns the number of stages.
function PipelineConfig:num_stages()
    return #self.stages
end

--- Validates the pipeline configuration.
--
-- Rules enforced:
--   - At least 2 stages
--   - execution_width >= 1
--   - All stage names must be unique
--   - Must have at least one fetch stage and one writeback stage
--
-- @return true, nil           if valid
-- @return false, string       error message if invalid
function PipelineConfig:validate()
    if #self.stages < 2 then
        return false, "pipeline must have at least 2 stages, got " .. #self.stages
    end
    if self.execution_width < 1 then
        return false, "execution_width must be >= 1, got " .. self.execution_width
    end

    -- Check unique names
    local seen = {}
    for _, stage in ipairs(self.stages) do
        if seen[stage.name] then
            return false, "duplicate stage name: \"" .. stage.name .. "\""
        end
        seen[stage.name] = true
    end

    -- Check for required categories
    local has_fetch = false
    local has_writeback = false
    for _, stage in ipairs(self.stages) do
        if stage.category == "fetch"     then has_fetch     = true end
        if stage.category == "writeback" then has_writeback = true end
    end
    if not has_fetch then
        return false, "pipeline must have at least one fetch stage"
    end
    if not has_writeback then
        return false, "pipeline must have at least one writeback stage"
    end

    return true, nil
end

-- ========================================================================
-- Preset Configurations
-- ========================================================================

--- The classic 5-stage RISC pipeline.
--
-- This is THE teaching pipeline — described in every computer architecture
-- textbook. It was popularized by the MIPS R2000 in 1985 and forms the
-- conceptual foundation for every modern CPU pipeline.
--
--   IF (Instruction Fetch)  → reads instruction from memory/cache
--   ID (Instruction Decode) → decodes fields, reads registers
--   EX (Execute)            → runs the ALU or computes branch
--   MEM (Memory Access)     → loads from or stores to data memory
--   WB (Write Back)         → writes result to the register file
--
-- Pipeline diagram (5 instructions in flight simultaneously):
--
--   Cycle:  1    2    3    4    5    6    7    8    9
--   Inst1: [IF] [ID] [EX] [ME] [WB]
--   Inst2:      [IF] [ID] [EX] [ME] [WB]
--   Inst3:           [IF] [ID] [EX] [ME] [WB]
--   Inst4:                [IF] [ID] [EX] [ME] [WB]
--   Inst5:                     [IF] [ID] [EX] [ME] [WB]
--
-- After the pipeline fills (cycle 5+), one instruction completes per cycle.
--
-- @return PipelineConfig
function PipelineConfig.classic_5_stage()
    return PipelineConfig.new({
        PipelineStage.new("IF",  "Instruction Fetch",  "fetch"),
        PipelineStage.new("ID",  "Instruction Decode", "decode"),
        PipelineStage.new("EX",  "Execute",            "execute"),
        PipelineStage.new("MEM", "Memory Access",      "memory"),
        PipelineStage.new("WB",  "Write Back",         "writeback"),
    }, 1)
end

--- A 13-stage pipeline inspired by the ARM Cortex-A78.
--
-- Modern high-performance CPUs split the classic 5 stages into many
-- sub-stages. This allows higher clock frequencies (each stage does less
-- work per cycle), but increases the branch misprediction penalty.
--
-- ARM Cortex-A78 is used in Snapdragon 888 and Dimensity 9000 (2020-2021).
-- The real A78 uses ~13 stages, is 4-wide, and runs out-of-order. Our model
-- captures the stage depth but stays in-order and 1-wide.
--
-- Tradeoff: where a 5-stage pipeline loses 2 cycles on a branch misprediction,
-- this 13-stage design loses 10 cycles. Higher clock speed must compensate.
--
-- @return PipelineConfig
function PipelineConfig.deep_13_stage()
    return PipelineConfig.new({
        PipelineStage.new("IF1",  "Fetch 1 - TLB lookup",        "fetch"),
        PipelineStage.new("IF2",  "Fetch 2 - cache read",         "fetch"),
        PipelineStage.new("IF3",  "Fetch 3 - align/buffer",       "fetch"),
        PipelineStage.new("ID1",  "Decode 1 - pre-decode",        "decode"),
        PipelineStage.new("ID2",  "Decode 2 - full decode",       "decode"),
        PipelineStage.new("ID3",  "Decode 3 - register read",     "decode"),
        PipelineStage.new("EX1",  "Execute 1 - ALU",              "execute"),
        PipelineStage.new("EX2",  "Execute 2 - shift/multiply",   "execute"),
        PipelineStage.new("EX3",  "Execute 3 - result select",    "execute"),
        PipelineStage.new("MEM1", "Memory 1 - address calc",      "memory"),
        PipelineStage.new("MEM2", "Memory 2 - cache access",      "memory"),
        PipelineStage.new("MEM3", "Memory 3 - data align",        "memory"),
        PipelineStage.new("WB",   "Write Back",                   "writeback"),
    }, 1)
end

-- ========================================================================
-- HazardResponse
-- ========================================================================
--
-- The hazard detection callback returns a HazardResponse that tells the
-- pipeline what action to take this cycle.
--
-- Priority order (highest to lowest):
--   flush > stall > forward_from_ex > forward_from_mem > none

local HazardResponse = {}
HazardResponse.__index = HazardResponse

--- Creates a new HazardResponse.
--
-- @param action         string  "none"|"stall"|"flush"|"forward_from_ex"|"forward_from_mem"
-- @param forward_value  number  The value to forward (for forwarding actions)
-- @param forward_source string  Which stage is providing the value (e.g., "EX")
-- @param stall_stages   number  Where to insert the bubble (stage index)
-- @param flush_count    number  How many front stages to flush (0 = auto)
-- @param redirect_pc    number  Where to redirect the PC on flush
-- @return table  A new HazardResponse
function HazardResponse.new(opts)
    opts = opts or {}
    return setmetatable({
        action         = opts.action         or "none",
        forward_value  = opts.forward_value  or 0,
        forward_source = opts.forward_source or "",
        stall_stages   = opts.stall_stages   or 0,
        flush_count    = opts.flush_count    or 0,
        redirect_pc    = opts.redirect_pc    or 0,
    }, HazardResponse)
end

-- ========================================================================
-- PipelineStats
-- ========================================================================
--
-- Tracks pipeline performance counters — the same metrics that hardware
-- performance counters measure in real CPUs.
--
-- KEY METRIC — IPC (Instructions Per Cycle):
--
--   IPC = instructions_completed / total_cycles
--
--   Ideal:       IPC = 1.0  (one instruction finishes every cycle)
--   With stalls: IPC < 1.0  (some cycles produce no completion)
--   Superscalar: IPC > 1.0  (multiple completions per cycle)
--
-- CPI (Cycles Per Instruction) is the inverse: CPI = 1 / IPC.

local PipelineStats = {}
PipelineStats.__index = PipelineStats

function PipelineStats.new()
    return setmetatable({
        total_cycles           = 0,
        instructions_completed = 0,
        stall_cycles           = 0,
        flush_cycles           = 0,
        bubble_cycles          = 0,
    }, PipelineStats)
end

--- Returns instructions per cycle.
function PipelineStats:ipc()
    if self.total_cycles == 0 then return 0.0 end
    return self.instructions_completed / self.total_cycles
end

--- Returns cycles per instruction.
function PipelineStats:cpi()
    if self.instructions_completed == 0 then return 0.0 end
    return self.total_cycles / self.instructions_completed
end

function PipelineStats:to_string()
    return string.format(
        "PipelineStats{cycles=%d, completed=%d, IPC=%.3f, CPI=%.3f, stalls=%d, flushes=%d, bubbles=%d}",
        self.total_cycles, self.instructions_completed,
        self:ipc(), self:cpi(),
        self.stall_cycles, self.flush_cycles, self.bubble_cycles
    )
end

-- ========================================================================
-- Snapshot
-- ========================================================================
--
-- A Snapshot captures the complete state of the pipeline at one clock
-- cycle. It is used for debugging, visualization, and testing.
--
-- Example (cycle 7, 5-stage pipeline):
--
--   [cycle 7] PC=28
--     IF:  instr@28   ← fetching instruction at PC=28
--     ID:  ADD@24     ← decoding an ADD at PC=24
--     EX:  SUB@20     ← executing a SUB at PC=20
--     MEM: ---        ← bubble (pipeline was stalled)
--     WB:  LDR@12     ← writing back a load result

local Snapshot = {}
Snapshot.__index = Snapshot

function Snapshot.new(cycle, stages_map, stalled, flushing, pc)
    return setmetatable({
        cycle    = cycle,
        stages   = stages_map,  -- { stage_name → token (or nil) }
        stalled  = stalled  or false,
        flushing = flushing or false,
        pc       = pc       or 0,
    }, Snapshot)
end

function Snapshot:to_string()
    return string.format("[cycle %d] PC=%d stalled=%s flushing=%s",
        self.cycle, self.pc,
        tostring(self.stalled), tostring(self.flushing))
end

-- ========================================================================
-- Module exports
-- ========================================================================

return {
    PipelineStage  = PipelineStage,
    PipelineConfig = PipelineConfig,
    HazardResponse = HazardResponse,
    PipelineStats  = PipelineStats,
    Snapshot       = Snapshot,
}
