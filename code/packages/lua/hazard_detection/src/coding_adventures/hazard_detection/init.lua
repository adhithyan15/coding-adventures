-- init.lua — Pipeline Hazard Detection and Forwarding
--
-- Pipelining overlaps instruction execution — while one instruction executes,
-- the next is decoded, and the next is fetched. This creates HAZARDS:
-- situations where the pipeline cannot proceed correctly because one
-- instruction depends on data from another that hasn't finished yet.
--
-- This package provides pure detection logic (no pipeline state of its own).
-- You call detect() with a snapshot of the current pipeline registers and
-- get back a HazardResult describing what action the pipeline must take.
--
-- THE THREE HAZARD TYPES:
--
-- 1. DATA HAZARD (RAW — Read After Write):
--
--      Instruction A writes register R1 in WB (cycle 5).
--      Instruction B reads register R1 in ID (cycle 3).
--      B reads R1 BEFORE A has written it — WRONG VALUE!
--
--      Solutions:
--        a) Forwarding  — route A's result directly to B's ALU input
--        b) Stalling    — freeze the pipeline for 1-2 cycles
--
-- 2. CONTROL HAZARD (branch misprediction):
--
--      A branch is fetched at cycle 1. Its outcome is not known until EX
--      (cycle 3). Meanwhile, cycles 2 and 3 already fetched sequential
--      instructions — those must be FLUSHED if the branch is taken.
--
-- 3. STRUCTURAL HAZARD:
--
--      Two instructions need the same hardware resource simultaneously.
--      Example: two memory accesses in the same cycle when there is only
--      one memory port. Usually avoided by hardware duplication.
--
-- FORWARDING PATHS:
--
--   EX-to-EX:  A computed a result in EX (cycle 3). B needs it in EX (cycle 4).
--              Forward from EX/MEM pipeline register to B's ALU input.
--
--   MEM-to-EX: A's result was 2 cycles ago (in MEM/WB). B needs it in EX.
--              Forward from MEM/WB pipeline register.
--
--   Load-use:  A's LOAD result is in MEM (cycle 4). B needs it in EX (cycle 4).
--              Forwarding is IMPOSSIBLE — must STALL for one cycle.
--
-- PipelineSlot: a snapshot of one pipeline stage.
-- HazardResult: the decision returned by a detector.
--
-- See DataHazardDetector, ControlHazardDetector, and StructuralHazardDetector.

-- ========================================================================
-- PipelineSlot — a snapshot of one pipeline stage
-- ========================================================================
--
-- When you call detect(), you pass in PipelineSlot objects describing what
-- is currently in each pipeline stage (ID, EX, MEM stages).
--
-- Fields:
--   valid              — is there a real instruction here? (false = bubble/empty)
--   pc                 — program counter of this instruction
--   dest_reg           — register this instruction writes (-1 = none)
--   dest_value         — current computed value (for forwarding)
--   source_regs        — list of registers this instruction reads
--   mem_read           — is this a LOAD instruction?
--   is_branch          — is this a branch instruction?
--   branch_taken       — was the branch actually taken?
--   branch_predicted_taken — what did the predictor guess?
--   branch_target      — actual branch target address

local PipelineSlot = {}
PipelineSlot.__index = PipelineSlot

function PipelineSlot.new(opts)
    opts = opts or {}
    return setmetatable({
        valid                  = opts.valid                  ~= false,  -- default true
        pc                     = opts.pc                     or 0,
        dest_reg               = opts.dest_reg               or -1,
        dest_value             = opts.dest_value             or 0,
        source_regs            = opts.source_regs            or {},
        mem_read               = opts.mem_read               or false,
        is_branch              = opts.is_branch              or false,
        branch_taken           = opts.branch_taken           or false,
        branch_predicted_taken = opts.branch_predicted_taken or false,
        branch_target          = opts.branch_target          or 0,
    }, PipelineSlot)
end

--- Creates an empty (bubble/no-instruction) slot.
function PipelineSlot.empty()
    return PipelineSlot.new({ valid = false })
end

-- ========================================================================
-- HazardResult — the detector's decision
-- ========================================================================
--
-- The detector returns a HazardResult. The pipeline reads it to decide
-- what to do this cycle.
--
-- Actions:
--   "none"        — no hazard, proceed normally
--   "stall"       — insert a bubble and freeze earlier stages
--   "flush"       — discard speculative instructions (branch misprediction)
--   "forward_ex"  — forward value from EX stage to the ALU input
--   "forward_mem" — forward value from MEM stage to the ALU input
--
-- Priority: flush > stall > forward_ex > forward_mem > none

local HazardResult = {}
HazardResult.__index = HazardResult

function HazardResult.new(opts)
    opts = opts or {}
    return setmetatable({
        action           = opts.action           or "none",
        stall_cycles     = opts.stall_cycles     or 0,
        forwarded_value  = opts.forwarded_value  or 0,
        forwarded_from   = opts.forwarded_from   or "",
        flush_target     = opts.flush_target     or 0,
        reason           = opts.reason           or "no hazard detected",
    }, HazardResult)
end

-- Priority levels for picking the "worst" (highest priority) hazard
local PRIORITY = {
    none        = 0,
    forward_mem = 1,
    forward_ex  = 2,
    stall       = 3,
    flush       = 4,
}

local function pick_higher_priority(a, b)
    local pa = PRIORITY[a.action] or 0
    local pb = PRIORITY[b.action] or 0
    if pb > pa then return b else return a end
end

-- ========================================================================
-- DataHazardDetector
-- ========================================================================
--
-- Detects RAW (Read After Write) hazards between the ID stage and the
-- EX/MEM stages. Returns the highest-priority action needed.
--
-- Algorithm:
--   For each source register read by the ID-stage instruction:
--     1. Does EX have a pending write to that register?
--        → If EX instruction is a LOAD: stall (MEM-to-EX forward would be
--          too late — the value won't be ready until MEM ends, but we need
--          it in EX which is EARLIER)
--        → Otherwise: forward from EX
--     2. Does MEM have a pending write to that register?
--        → Forward from MEM
--     3. No dependency → no action needed
--   Return the highest-priority result across all source registers.

local DataHazardDetector = {}
DataHazardDetector.__index = DataHazardDetector

function DataHazardDetector.new()
    return setmetatable({}, DataHazardDetector)
end

--- Detects data hazards between the instruction in the ID stage and the
--- instructions currently in EX and MEM.
--
-- @param id_slot   PipelineSlot  The instruction being decoded
-- @param ex_slot   PipelineSlot  The instruction currently executing
-- @param mem_slot  PipelineSlot  The instruction in the memory access stage
-- @return HazardResult
function DataHazardDetector:detect(id_slot, ex_slot, mem_slot)
    if not id_slot.valid then
        return HazardResult.new({ reason = "ID stage is empty (bubble)" })
    end

    if #id_slot.source_regs == 0 then
        return HazardResult.new({ reason = "instruction has no source registers" })
    end

    -- Check all source registers and pick the worst hazard
    local worst = HazardResult.new({ reason = "no data dependencies detected" })
    for _, src_reg in ipairs(id_slot.source_regs) do
        local result = self:_check_single_register(src_reg, ex_slot, mem_slot)
        worst = pick_higher_priority(worst, result)
    end
    return worst
end

function DataHazardDetector:_check_single_register(src_reg, ex_slot, mem_slot)
    -- EX stage: load-use hazard (cannot forward — must stall)
    if ex_slot.valid and ex_slot.dest_reg == src_reg and ex_slot.mem_read then
        return HazardResult.new({
            action      = "stall",
            stall_cycles = 1,
            reason      = string.format(
                "load-use hazard: R%d is being loaded by instruction at PC=0x%04X — must stall 1 cycle",
                src_reg, ex_slot.pc
            ),
        })
    end

    -- EX stage: normal RAW hazard (forward from EX)
    if ex_slot.valid and ex_slot.dest_reg == src_reg then
        return HazardResult.new({
            action          = "forward_ex",
            forwarded_value = ex_slot.dest_value,
            forwarded_from  = "EX",
            reason          = string.format(
                "RAW hazard on R%d: forwarding value %d from EX stage (PC=0x%04X)",
                src_reg, ex_slot.dest_value, ex_slot.pc
            ),
        })
    end

    -- MEM stage: RAW hazard (forward from MEM)
    if mem_slot.valid and mem_slot.dest_reg == src_reg then
        return HazardResult.new({
            action          = "forward_mem",
            forwarded_value = mem_slot.dest_value,
            forwarded_from  = "MEM",
            reason          = string.format(
                "RAW hazard on R%d: forwarding value %d from MEM stage (PC=0x%04X)",
                src_reg, mem_slot.dest_value, mem_slot.pc
            ),
        })
    end

    return HazardResult.new({
        reason = string.format("R%d has no pending writes in EX or MEM", src_reg),
    })
end

-- ========================================================================
-- ControlHazardDetector
-- ========================================================================
--
-- Detects branch mispredictions. When a branch instruction in the EX stage
-- is resolved, we compare the actual outcome with the predictor's guess.
-- If they differ, the pipeline must flush the speculative instructions that
-- were fetched after the branch.
--
-- In a 5-stage pipeline, the branch is resolved in EX (stage 3).
-- Two instructions have been fetched after the branch (in IF and ID).
-- On a misprediction, those two stages are flushed and the PC is redirected
-- to the correct target.

local ControlHazardDetector = {}
ControlHazardDetector.__index = ControlHazardDetector

function ControlHazardDetector.new()
    return setmetatable({}, ControlHazardDetector)
end

--- Detects a branch misprediction in the EX stage.
--
-- @param ex_slot  PipelineSlot  The instruction in the EX stage
-- @return HazardResult
function ControlHazardDetector:detect(ex_slot)
    if not ex_slot.valid then
        return HazardResult.new({ reason = "EX stage is empty (bubble)" })
    end

    if not ex_slot.is_branch then
        return HazardResult.new({ reason = "EX stage instruction is not a branch" })
    end

    if ex_slot.branch_predicted_taken == ex_slot.branch_taken then
        local dir = ex_slot.branch_taken and "taken" or "not taken"
        return HazardResult.new({
            reason = string.format("branch correctly predicted as %s at PC=0x%04X", dir, ex_slot.pc),
        })
    end

    -- Misprediction detected → flush
    return HazardResult.new({
        action      = "flush",
        flush_target = ex_slot.branch_taken and ex_slot.branch_target or (ex_slot.pc + 4),
        reason      = string.format(
            "branch mispredicted at PC=0x%04X: predicted %s but actually %s — flushing pipeline, redirecting to 0x%04X",
            ex_slot.pc,
            ex_slot.branch_predicted_taken and "taken" or "not taken",
            ex_slot.branch_taken and "taken" or "not taken",
            ex_slot.branch_taken and ex_slot.branch_target or (ex_slot.pc + 4)
        ),
    })
end

-- ========================================================================
-- StructuralHazardDetector
-- ========================================================================
--
-- Detects resource conflicts. In practice, most structural hazards are
-- eliminated in hardware by:
--   - Split L1 cache (instruction + data caches)
--   - Multiple register file read/write ports
--   - Dedicated execution units (ALU, FPU, load/store unit)
--
-- We detect two simple cases:
--   1. Both IF and MEM stages need memory simultaneously (unified cache)
--   2. Two instructions write back to the register file in the same cycle

local StructuralHazardDetector = {}
StructuralHazardDetector.__index = StructuralHazardDetector

function StructuralHazardDetector.new()
    return setmetatable({}, StructuralHazardDetector)
end

--- Detects structural hazards.
--
-- @param mem_slot  PipelineSlot  The instruction in the MEM stage
-- @param wb_slot   PipelineSlot  The instruction in the WB stage
-- @param has_split_cache  boolean  True if L1I and L1D are separate (no conflict)
-- @return HazardResult
function StructuralHazardDetector:detect(mem_slot, wb_slot, has_split_cache)
    has_split_cache = has_split_cache ~= false  -- default: split cache (no hazard)

    -- Check memory port conflict (IF + MEM both accessing a unified cache)
    if not has_split_cache and mem_slot.valid and mem_slot.mem_read then
        return HazardResult.new({
            action = "stall",
            stall_cycles = 1,
            reason = "structural hazard: unified cache — IF and MEM both need memory",
        })
    end

    -- Check register file write port conflict
    if mem_slot.valid and wb_slot.valid and
       mem_slot.dest_reg >= 0 and wb_slot.dest_reg >= 0 and
       mem_slot.dest_reg == wb_slot.dest_reg then
        return HazardResult.new({
            action = "stall",
            stall_cycles = 1,
            reason = string.format(
                "structural hazard: both MEM and WB want to write R%d — need two write ports",
                mem_slot.dest_reg
            ),
        })
    end

    return HazardResult.new({ reason = "no structural hazard detected" })
end

-- ========================================================================
-- Module exports
-- ========================================================================

return {
    PipelineSlot             = PipelineSlot,
    HazardResult             = HazardResult,
    DataHazardDetector       = DataHazardDetector,
    ControlHazardDetector    = ControlHazardDetector,
    StructuralHazardDetector = StructuralHazardDetector,
}
