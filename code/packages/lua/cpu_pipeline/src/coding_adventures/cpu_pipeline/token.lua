-- =============================================================================
-- PipelineToken — a unit of work flowing through the pipeline
-- =============================================================================
--
-- Each token represents one instruction moving through the N stages.  It is
-- ISA-agnostic: the ISA decoder fills in opcode/rs1/rs2/rd/immediate, but the
-- pipeline itself only cares about the control flags (is_halt, is_bubble, …).
--
-- Think of a token like a traveller's passport: it is stamped at each stage
-- (stage_entered records the cycle number), carries all the information needed
-- by downstream stages, and is returned when the journey completes (writeback).
--
-- Fields mirror the Python dataclass spec (D04) exactly so that Lua and Python
-- implementations remain in sync.

local Token = {}
Token.__index = Token

-- ---------------------------------------------------------------------------
-- Token.new() — create a fresh, real instruction token
-- ---------------------------------------------------------------------------
function Token.new()
    local self = setmetatable({}, Token)

    -- Program counter and raw bits
    self.pc              = 0
    self.raw_instruction = 0

    -- Decoded fields (filled by ID/decode callback)
    self.opcode    = ""
    self.rs1       = -1   -- source register 1 (-1 = unused)
    self.rs2       = -1   -- source register 2 (-1 = unused)
    self.rd        = -1   -- destination register (-1 = unused)
    self.immediate = 0

    -- Control signals
    self.reg_write = false
    self.mem_read  = false
    self.mem_write = false
    self.is_branch = false
    self.is_halt   = false

    -- Computed values (filled by EX / MEM callbacks)
    self.alu_result    = 0
    self.mem_data      = 0
    self.write_data    = 0
    self.branch_taken  = false
    self.branch_target = 0

    -- Pipeline metadata
    self.is_bubble      = false
    self.stage_entered  = {}   -- {stage_name -> cycle_number}
    self.forwarded_from = ""   -- which stage provided a forwarded value

    return self
end

-- ---------------------------------------------------------------------------
-- Token.new_bubble() — create a NOP / bubble token
-- ---------------------------------------------------------------------------
-- A bubble occupies a pipeline slot without doing any work.  It is inserted
-- during stalls (load-use) and flushes (branch misprediction) so that later
-- stages see a harmless NOP rather than a stale real instruction.
--
-- In real hardware this corresponds to asserting the pipeline flush / bubble
-- insertion signal on the rising clock edge.
function Token.new_bubble()
    local self = Token.new()
    self.is_bubble = true
    self.opcode = "BUBBLE"
    return self
end

-- ---------------------------------------------------------------------------
-- Token:to_string() — human-readable representation for debugging
-- ---------------------------------------------------------------------------
function Token:to_string()
    if self.is_bubble then
        return string.format("Token[BUBBLE pc=0x%04X]", self.pc)
    end
    return string.format(
        "Token[pc=0x%04X op=%s rd=%d rs1=%d rs2=%d halt=%s]",
        self.pc,
        self.opcode == "" and "?" or self.opcode,
        self.rd, self.rs1, self.rs2,
        self.is_halt and "T" or "F"
    )
end

-- ---------------------------------------------------------------------------
-- Token.clone(tok) — deep-copy a token for snapshot history
-- ---------------------------------------------------------------------------
-- Snapshots store a copy of the pipeline state so that trace() can replay
-- execution history.  Without cloning, all snapshot entries would alias the
-- same mutable table.
function Token.clone(tok)
    if tok == nil then return nil end
    local c = setmetatable({}, Token)
    -- Shallow-copy all scalar fields
    for k, v in pairs(tok) do
        c[k] = v
    end
    -- Deep-copy stage_entered (it is the only nested table)
    c.stage_entered = {}
    for k, v in pairs(tok.stage_entered or {}) do
        c.stage_entered[k] = v
    end
    return c
end

return Token
