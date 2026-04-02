-- =============================================================================
-- Hazard Detection — pipeline hazard detectors
-- =============================================================================
--
-- This module provides three detectors that together keep the pipeline
-- running correctly despite instruction dependencies:
--
--   DataHazardDetector     — RAW (Read After Write) hazards
--   ControlHazardDetector  — branch mispredictions (→ flush)
--   StructuralHazardDetector — resource conflicts (ALU, FP unit, memory port)
--
-- All three detectors are STATELESS: they examine the current pipeline slots
-- and return a HazardResult describing what action to take.  The pipeline
-- itself is responsible for acting on the result.
--
-- PRIORITY ORDER (highest wins when multiple hazards detected simultaneously):
--
--   flush > stall > forward_ex > forward_mem > none
--
-- This mirrors real hardware: a control hazard (flush) takes precedence over
-- a data hazard that was about to be forwarded, because the instructions being
-- forwarded may themselves be from the wrong path.

-- ---------------------------------------------------------------------------
-- PipelineSlot — the view of one pipeline stage as seen by hazard detectors
-- ---------------------------------------------------------------------------
-- The detectors operate on "slots" rather than full PipelineTokens.  This
-- keeps the hazard detection API independent of the exact token structure.
--
-- Fields:
--   valid             — is there a real instruction here? (false = bubble/empty)
--   pc                — program counter of the instruction in this slot
--   source_regs       — list of source register indices (rs1, rs2)
--   dest_reg          — destination register index (-1 = none)
--   dest_value        — current value that would be written to dest_reg
--   mem_read          — is this a load instruction?
--   mem_write         — is this a store instruction?
--   is_branch         — is this a branch instruction?
--   branch_taken      — was the branch actually taken? (resolved in EX)
--   branch_predicted_taken — what did the predictor guess?
--   uses_alu          — does this instruction use the ALU?
--   uses_fp           — does this instruction use the FP unit?

local PipelineSlot = {}
PipelineSlot.__index = PipelineSlot

function PipelineSlot.new(opts)
    opts = opts or {}
    local self = setmetatable({}, PipelineSlot)
    self.valid                  = opts.valid   ~= nil and opts.valid   or false
    self.pc                     = opts.pc      or 0
    self.source_regs            = opts.source_regs or {}
    self.dest_reg               = opts.dest_reg    ~= nil and opts.dest_reg or -1
    self.dest_value             = opts.dest_value  or 0
    self.mem_read               = opts.mem_read    or false
    self.mem_write              = opts.mem_write   or false
    self.is_branch              = opts.is_branch   or false
    self.branch_taken           = opts.branch_taken or false
    self.branch_predicted_taken = opts.branch_predicted_taken or false
    self.uses_alu               = opts.uses_alu or false
    self.uses_fp                = opts.uses_fp  or false
    return self
end

-- empty_slot() — a slot representing an empty (bubble) stage
function PipelineSlot.empty()
    return PipelineSlot.new({valid = false})
end

-- ---------------------------------------------------------------------------
-- HazardResult — what action the pipeline should take
-- ---------------------------------------------------------------------------
-- action:          "none" | "stall" | "flush" | "forward_ex" | "forward_mem"
-- stall_cycles:    how many cycles to stall (usually 1)
-- flush_count:     how many stages to flush (usually 2 for 5-stage pipeline)
-- forwarded_value: the value being forwarded
-- forwarded_from:  "EX" or "MEM"
-- reason:          human-readable explanation (for debugging)

local HazardResult = {}
HazardResult.__index = HazardResult

function HazardResult.new(opts)
    opts = opts or {}
    local self = setmetatable({}, HazardResult)
    self.action          = opts.action          or "none"
    self.stall_cycles    = opts.stall_cycles    or 0
    self.flush_count     = opts.flush_count     or 0
    self.forwarded_value  = opts.forwarded_value  or 0
    self.forwarded_from  = opts.forwarded_from  or ""
    self.reason          = opts.reason          or ""
    return self
end

-- Priority ordering for hazard actions (higher = more urgent)
local function action_priority(action)
    if action == "none"        then return 0 end
    if action == "forward_mem" then return 1 end
    if action == "forward_ex"  then return 2 end
    if action == "stall"       then return 3 end
    if action == "flush"       then return 4 end
    return 0
end

-- pick_higher_priority(a, b) → whichever HazardResult has higher priority
local function pick_higher_priority(a, b)
    if action_priority(b.action) > action_priority(a.action) then
        return b
    end
    return a
end

-- hex4(n) → "00AB" style hex string for PC display
local function hex4(n)
    return string.format("%04X", n or 0)
end

-- ---------------------------------------------------------------------------
-- DataHazardDetector — RAW (Read After Write) hazard detection
-- ---------------------------------------------------------------------------
-- The classic data hazard: instruction B reads a register that instruction A
-- has not yet written back.
--
-- FORWARDING PATHS (from newest to oldest):
--   EX forwarding:  instruction A is in EX, result available from EX/MEM reg
--   MEM forwarding: instruction A is in MEM, result available from MEM/WB reg
--
-- STALL: load-use hazard — A is a LOAD in EX; B in ID needs the value.
--   The load does not complete until MEM, which is one stage too late.
--   The pipeline must insert a bubble and wait one cycle.

local DataHazardDetector = {}
DataHazardDetector.__index = DataHazardDetector

function DataHazardDetector.new()
    return setmetatable({}, DataHazardDetector)
end

-- detect(id_slot, ex_slot, mem_slot) → HazardResult
-- id_slot  — instruction currently in ID (the consumer)
-- ex_slot  — instruction currently in EX (potential producer)
-- mem_slot — instruction currently in MEM (potential producer)
function DataHazardDetector:detect(id_slot, ex_slot, mem_slot)
    if not id_slot.valid then
        return HazardResult.new({reason = "ID stage is empty (bubble)"})
    end
    if #id_slot.source_regs == 0 then
        return HazardResult.new({reason = "instruction has no source registers"})
    end

    local worst = HazardResult.new({reason = "no data dependencies detected"})
    for _, src_reg in ipairs(id_slot.source_regs) do
        local result = self:_check_single_register(src_reg, ex_slot, mem_slot)
        worst = pick_higher_priority(worst, result)
    end
    return worst
end

function DataHazardDetector:_check_single_register(src_reg, ex_slot, mem_slot)
    -- Load-use hazard: EX is a load, and ID needs its result
    if ex_slot.valid and ex_slot.dest_reg == src_reg and ex_slot.mem_read then
        return HazardResult.new({
            action       = "stall",
            stall_cycles = 1,
            reason       = string.format(
                "load-use hazard: R%d is being loaded by instruction at PC=0x%s — must stall 1 cycle",
                src_reg, hex4(ex_slot.pc)),
        })
    end

    -- EX forwarding: EX has a result we can bypass
    if ex_slot.valid and ex_slot.dest_reg == src_reg then
        return HazardResult.new({
            action          = "forward_ex",
            forwarded_value  = ex_slot.dest_value,
            forwarded_from  = "EX",
            reason          = string.format(
                "RAW hazard on R%d: forwarding value %d from EX (PC=0x%s)",
                src_reg, ex_slot.dest_value, hex4(ex_slot.pc)),
        })
    end

    -- MEM forwarding: MEM has a result we can bypass
    if mem_slot.valid and mem_slot.dest_reg == src_reg then
        return HazardResult.new({
            action          = "forward_mem",
            forwarded_value  = mem_slot.dest_value,
            forwarded_from  = "MEM",
            reason          = string.format(
                "RAW hazard on R%d: forwarding value %d from MEM (PC=0x%s)",
                src_reg, mem_slot.dest_value, hex4(mem_slot.pc)),
        })
    end

    return HazardResult.new({reason = string.format("R%d has no pending writes", src_reg)})
end

-- ---------------------------------------------------------------------------
-- ControlHazardDetector — branch misprediction detection
-- ---------------------------------------------------------------------------
-- When a branch resolves in the EX stage and the prediction was wrong, the
-- instructions fetched from the wrong path (in IF and ID) must be flushed.
--
-- FLUSH PENALTY for a classic 5-stage pipeline resolving branches in EX:
--   2 cycles (the instructions in IF and ID are discarded)
--
-- For deeper pipelines, the penalty is higher (more stages between IF and EX).

local ControlHazardDetector = {}
ControlHazardDetector.__index = ControlHazardDetector

function ControlHazardDetector.new()
    return setmetatable({}, ControlHazardDetector)
end

-- detect(ex_slot) → HazardResult
function ControlHazardDetector:detect(ex_slot)
    if not ex_slot.valid then
        return HazardResult.new({reason = "EX stage is empty (bubble)"})
    end
    if not ex_slot.is_branch then
        return HazardResult.new({reason = "EX stage instruction is not a branch"})
    end
    if ex_slot.branch_predicted_taken == ex_slot.branch_taken then
        local dir = ex_slot.branch_taken and "taken" or "not taken"
        return HazardResult.new({
            reason = string.format("branch at PC=0x%s correctly predicted %s", hex4(ex_slot.pc), dir),
        })
    end

    -- Misprediction: flush IF and ID (2 stages)
    local dir
    if ex_slot.branch_taken then
        dir = "predicted not-taken, actually taken"
    else
        dir = "predicted taken, actually not-taken"
    end
    return HazardResult.new({
        action      = "flush",
        flush_count = 2,
        reason      = string.format(
            "branch misprediction at PC=0x%s: %s — flushing IF and ID stages",
            hex4(ex_slot.pc), dir),
    })
end

-- ---------------------------------------------------------------------------
-- StructuralHazardDetector — resource conflict detection
-- ---------------------------------------------------------------------------
-- Modern CPUs avoid most structural hazards through hardware duplication
-- (split L1 caches, multiple ALUs).  This detector models:
--   - Execution unit conflicts: two instructions both need the ALU (or FP unit)
--   - Memory port conflict: unified cache causes IF and MEM to contend

local StructuralHazardDetector = {}
StructuralHazardDetector.__index = StructuralHazardDetector

-- new(opts)
--   opts.num_alus      — number of ALU units (default 1)
--   opts.num_fp_units  — number of FP units (default 1)
--   opts.split_caches  — true = split L1I+L1D, no memory conflict (default true)
function StructuralHazardDetector.new(opts)
    opts = opts or {}
    local self = setmetatable({}, StructuralHazardDetector)
    self.num_alus     = opts.num_alus     or 1
    self.num_fp_units = opts.num_fp_units or 1
    -- Use explicit nil check so that passing split_caches=false is respected.
    -- The classic Lua idiom `x ~= nil and x or default` fails when x=false.
    if opts.split_caches ~= nil then
        self.split_caches = opts.split_caches
    else
        self.split_caches = true  -- default: split caches (no memory port conflict)
    end
    return self
end

-- detect(id_slot, ex_slot, opts) → HazardResult
--   opts.if_stage  — PipelineSlot for IF (used for memory conflict check)
--   opts.mem_stage — PipelineSlot for MEM
function StructuralHazardDetector:detect(id_slot, ex_slot, opts)
    opts = opts or {}

    local exec_result = self:_check_execution_unit(id_slot, ex_slot)
    if exec_result.action ~= "none" then
        return exec_result
    end

    if opts.if_stage and opts.mem_stage then
        return self:_check_memory_port(opts.if_stage, opts.mem_stage)
    end

    return HazardResult.new({reason = "no structural hazards — all resources available"})
end

function StructuralHazardDetector:_check_execution_unit(id_slot, ex_slot)
    if not id_slot.valid or not ex_slot.valid then
        return HazardResult.new({reason = "one or both stages are empty (bubble)"})
    end

    if id_slot.uses_alu and ex_slot.uses_alu and self.num_alus < 2 then
        return HazardResult.new({
            action       = "stall",
            stall_cycles = 1,
            reason       = string.format(
                "structural hazard: both ID (PC=0x%s) and EX (PC=0x%s) need the ALU, but only %d ALU available",
                hex4(id_slot.pc), hex4(ex_slot.pc), self.num_alus),
        })
    end

    if id_slot.uses_fp and ex_slot.uses_fp and self.num_fp_units < 2 then
        return HazardResult.new({
            action       = "stall",
            stall_cycles = 1,
            reason       = string.format(
                "structural hazard: both ID and EX need the FP unit, but only %d FP unit available",
                self.num_fp_units),
        })
    end

    return HazardResult.new({reason = "no execution unit conflict"})
end

function StructuralHazardDetector:_check_memory_port(if_slot, mem_slot)
    if self.split_caches then
        return HazardResult.new({reason = "split caches — no memory port conflict"})
    end

    if if_slot.valid and mem_slot.valid and (mem_slot.mem_read or mem_slot.mem_write) then
        local access = mem_slot.mem_read and "load" or "store"
        return HazardResult.new({
            action       = "stall",
            stall_cycles = 1,
            reason       = string.format(
                "structural hazard: IF (PC=0x%s) and MEM (%s at PC=0x%s) both need the shared memory bus",
                hex4(if_slot.pc), access, hex4(mem_slot.pc)),
        })
    end

    return HazardResult.new({reason = "no memory port conflict"})
end

return {
    PipelineSlot             = PipelineSlot,
    HazardResult             = HazardResult,
    DataHazardDetector       = DataHazardDetector,
    ControlHazardDetector    = ControlHazardDetector,
    StructuralHazardDetector = StructuralHazardDetector,
}
