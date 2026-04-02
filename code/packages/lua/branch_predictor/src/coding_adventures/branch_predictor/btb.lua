-- branch_predictor/btb.lua — Branch Target Buffer
--
-- The direction predictor answers "WILL this branch be taken?"
-- The BTB answers "WHERE does it go?"
--
-- Both are needed for high-performance fetch. Without a BTB, even a
-- perfect direction predictor causes a 1-cycle bubble: the predictor
-- says "taken" in the fetch stage, but the target address isn't known
-- until decode. With a BTB, the target is available in the SAME cycle
-- as the prediction, enabling zero-bubble fetch redirection.
--
-- ## How it works in the pipeline
--
--   Cycle 1 (Fetch):
--     1. Read PC
--     2. Direction predictor: "taken" or "not taken"?
--     3. BTB lookup: if "taken", WHERE does it go?
--     4. Redirect fetch to BTB target (hit) or PC+4 (not taken / BTB miss)
--
--   Cycle 2+ (Decode, Execute):
--     Branch resolves. If BTB was wrong → flush and update BTB.
--
-- ## Organization
--
-- Direct-mapped cache indexed by: index = pc % size
--
-- Each entry stores:
--   tag         — the full PC (detects aliasing conflicts)
--   target      — the branch target address
--   branch_type — metadata ("conditional", "unconditional", "call", "return")
--
-- On lookup: check tag match. Miss if no entry or tag mismatch.
-- On update: overwrite the entry at index (direct-mapped eviction).
--
-- ## Real-world BTB sizes
--
--   Intel Skylake : 4096 (L1) + 4096 (L2) entries
--   ARM Cortex-A72: 64 (micro) + 4096 (main) entries
--   AMD Zen 2     : 512 (L1) + 7168 (L2) entries

local BTB = {}
BTB.__index = BTB

-- Create a new Branch Target Buffer.
--
-- Parameters:
--   size  (number) — number of entries (default: 256)
--                    Should be a power of 2 for efficient hardware.
function BTB.new(size)
    size = size or 256
    return setmetatable({
        size    = size,
        entries = {},    -- map: index -> {tag, target, branch_type}
        lookups = 0,
        hits    = 0,
        misses  = 0,
    }, BTB)
end

local function index_of(btb, pc)
    return pc % btb.size
end

-- Look up the predicted target for a branch at PC.
--
-- Returns: target (number or nil), new_btb
--   target = branch target address on a hit
--   target = nil on a miss (entry not present or tag mismatch)
--
-- A miss occurs when:
--   1. Entry has never been written (compulsory miss)
--   2. Entry's tag doesn't match the PC (conflict/aliasing miss)
function BTB:lookup(pc)
    local idx   = index_of(self, pc)
    local entry = self.entries[idx]

    local new_btb = setmetatable({
        size    = self.size,
        entries = self.entries,
        lookups = self.lookups + 1,
        hits    = self.hits,
        misses  = self.misses,
    }, BTB)

    if entry ~= nil and entry.tag == pc then
        new_btb.hits = self.hits + 1
        return entry.target, new_btb
    else
        new_btb.misses = self.misses + 1
        return nil, new_btb
    end
end

-- Record a branch target after execution.
--
-- Writes the target and metadata into the BTB. If another branch was
-- occupying this index (aliasing), it gets evicted (direct-mapped).
--
-- Parameters:
--   pc          (number) — program counter of the branch instruction
--   target      (number) — actual target address of the branch
--   branch_type (string) — "conditional", "unconditional", "call", "return"
--                          (default: "conditional")
function BTB:update(pc, target, branch_type)
    branch_type = branch_type or "conditional"
    local idx = index_of(self, pc)

    local new_entries = {}
    for k, v in pairs(self.entries) do new_entries[k] = v end
    new_entries[idx] = { tag = pc, target = target, branch_type = branch_type }

    return setmetatable({
        size    = self.size,
        entries = new_entries,
        lookups = self.lookups,
        hits    = self.hits,
        misses  = self.misses,
    }, BTB)
end

-- Inspect the BTB entry for a given PC (for testing/debugging).
-- Returns the entry table {tag, target, branch_type} on a hit, nil on miss.
-- Does NOT update stats counters.
function BTB:get_entry(pc)
    local idx   = index_of(self, pc)
    local entry = self.entries[idx]
    if entry ~= nil and entry.tag == pc then
        return entry
    end
    return nil
end

-- BTB hit rate as a percentage (0.0 to 100.0).
-- Returns 0.0 if no lookups have been performed.
function BTB:hit_rate()
    if self.lookups == 0 then return 0.0 end
    return self.hits / self.lookups * 100.0
end

-- Reset all BTB state — entries and statistics.
function BTB:reset()
    return BTB.new(self.size)
end

return BTB
