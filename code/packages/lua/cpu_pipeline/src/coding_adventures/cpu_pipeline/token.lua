-- token.lua — PipelineToken and related types
--
-- A PipelineToken is the unit of work that flows through the pipeline.
-- Think of it as a tray on a factory assembly line: each workstation
-- (stage) picks it up, does its job, and passes it to the next station.
--
-- The tray starts nearly empty at the IF (Instruction Fetch) stage:
--   - The fetch callback fills in `pc` and `raw_instruction`
-- At the ID (Instruction Decode) stage:
--   - The decode callback fills in `opcode`, registers, control signals
-- At EX (Execute):
--   - The execute callback fills in `alu_result`, branch info
-- At MEM (Memory Access):
--   - The memory callback fills in `mem_data` for load instructions
-- At WB (Write Back):
--   - The writeback callback commits results to the register file
--
-- BUBBLES: A bubble is a "do-nothing" token that occupies a stage without
-- performing useful work. Bubbles are inserted when:
--   1. The pipeline STALLS — to fill the gap left by frozen earlier stages
--   2. The pipeline FLUSHES — to replace discarded speculative instructions
--
-- In hardware, a bubble corresponds to a NOP instruction being fed into
-- the pipeline. In our simulator, it is a token with `is_bubble = true`.

local Token = {}
Token.__index = Token

--- Creates a new empty pipeline token.
--
-- All register fields default to -1 (meaning "unused / not applicable").
-- Control signals default to false. The fetch callback will fill in `pc`
-- and `raw_instruction` when the token enters the IF stage.
--
-- @return table  A new PipelineToken
function Token.new()
    return setmetatable({
        pc              = 0,    -- Program counter of this instruction
        raw_instruction = 0,    -- Raw instruction bits from memory
        opcode          = "",   -- Decoded opcode name (e.g., "ADD", "LDR")

        -- Register operands (-1 = not used by this instruction)
        rs1 = -1,   -- Source register 1
        rs2 = -1,   -- Source register 2
        rd  = -1,   -- Destination register

        immediate = 0,  -- Sign-extended immediate value

        -- Control signals (filled by decode stage)
        reg_write  = false,  -- Does this instruction write a register?
        mem_read   = false,  -- Does this instruction read from memory?
        mem_write  = false,  -- Does this instruction write to memory?
        is_branch  = false,  -- Is this a branch instruction?
        is_halt    = false,  -- Is this a halt instruction?

        -- Computed values (filled during execution)
        alu_result    = 0,      -- Output of the ALU
        mem_data      = 0,      -- Data loaded from memory (for LDR)
        write_data    = 0,      -- Data to write to register file in WB
        branch_taken  = false,  -- Was the branch actually taken?
        branch_target = 0,      -- Actual branch target address

        -- Pipeline metadata
        is_bubble     = false,  -- True if this is a NOP/bubble (no-op)
        stage_entered = {},     -- stage_name → cycle number (when entered)
        forwarded_from = "",    -- If forwarded, which stage provided the value
    }, Token)
end

--- Creates a new bubble token.
--
-- A bubble is an invisible NOP that keeps the pipeline stages in sync
-- when a stall or flush occurs. It travels through every stage without
-- doing anything useful.
--
-- Analogously: when a worker is absent from the factory line, the supervisor
-- puts an empty tray on the conveyor so downstream workers aren't confused.
--
-- @return table  A new bubble PipelineToken
function Token.new_bubble()
    local t = Token.new()
    t.is_bubble = true
    return t
end

--- Returns a human-readable representation of the token.
--
-- Examples:
--   "---"         — a bubble
--   "ADD@100"     — an ADD instruction at PC=100
--   "instr@200"   — a not-yet-decoded instruction at PC=200
--
-- @param self  The token
-- @return string
function Token:to_string()
    if self.is_bubble then return "---" end
    if self.opcode ~= "" then
        return self.opcode .. "@" .. tostring(self.pc)
    end
    return "instr@" .. tostring(self.pc)
end

--- Returns a shallow copy of the token.
--
-- We copy stage_entered deeply to avoid aliasing bugs.
--
-- @param self  The token (or nil)
-- @return table|nil
function Token.clone(tok)
    if tok == nil then return nil end
    local c = {}
    for k, v in pairs(tok) do c[k] = v end
    -- Deep copy stage_entered table
    c.stage_entered = {}
    for k, v in pairs(tok.stage_entered) do
        c.stage_entered[k] = v
    end
    return setmetatable(c, Token)
end

return Token
