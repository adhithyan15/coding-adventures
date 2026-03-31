--- coding_adventures.virtual_memory — Virtual memory with paging and TLB
--
-- # What is Virtual Memory?
--
-- Virtual memory is one of the most important abstractions in computer science.
-- It gives every process the **illusion** that it has the entire address space
-- to itself — from address 0 to some large upper limit — even though the
-- physical machine has limited RAM shared among many processes.
--
-- ## The Apartment Building Analogy
--
-- Imagine an apartment building. Each tenant thinks their "Room 1" is their
-- bedroom, "Room 2" is their kitchen, etc. But the building manager (the MMU)
-- knows that Tenant A's "Room 1" is actually physical room 401, and Tenant B's
-- "Room 1" is physical room 712. The tenants never need to know their physical
-- room numbers. They just say "go to Room 1" and the building manager
-- translates.
--
-- Without virtual memory, every program would need to know exactly where in
-- physical RAM it was loaded. If process A uses addresses 0x1000-0x2000 and
-- process B also wants 0x1000-0x2000, they would overwrite each other!
--
-- ## Pages and Frames
--
-- Virtual memory divides both address spaces into fixed-size chunks:
--
--   Virtual page  — a chunk of virtual address space (4 KB = 4096 bytes)
--   Physical frame — a chunk of physical RAM (same size as a page)
--
-- Why 4 KB? It has been the standard since the Intel 386 in 1985. RISC-V uses
-- 4 KB pages too. Smaller pages waste less memory (less internal fragmentation)
-- but require larger page tables. 4 KB is a good compromise.
--
-- ## Address Translation
--
-- A 32-bit virtual address is split into two parts:
--
--   ┌──────────────────────────┬────────────────┐
--   │ Virtual Page Number (VPN)│ Page Offset    │
--   │ bits 31-12 (20 bits)     │ bits 11-0      │
--   │                          │ (12 bits)      │
--   └──────────────────────────┴────────────────┘
--
--   VPN    = address >> 12      (upper 20 bits)
--   offset = address & 0xFFF    (lower 12 bits)
--
-- The page table maps VPN → physical frame number.
-- Physical address = (frame << 12) | offset
--
-- ## Two-Level Page Table (Sv32)
--
-- A flat page table for a 32-bit address space needs 2^20 = 1,048,576 entries.
-- Even at 4 bytes per entry, that is 4 MB per process — wasteful if the process
-- only uses a small portion of its address space.
--
-- Two-level page tables (RISC-V Sv32) split the 20-bit VPN into two 10-bit parts:
--
--   ┌────────────┬────────────┬────────────────┐
--   │ VPN[1]     │ VPN[0]     │ Page Offset    │
--   │ bits 31-22 │ bits 21-12 │ bits 11-0      │
--   └────────────┴────────────┴────────────────┘
--
-- VPN[1] indexes into the page directory (up to 1024 entries, only allocated
-- for 4 MB regions that are actually in use). VPN[0] indexes within that
-- second-level table. Most of the directory entries are nil — no memory wasted!
--
-- ## TLB (Translation Lookaside Buffer)
--
-- Walking the page table takes 2-3 extra memory accesses per instruction.
-- The TLB is a small, fast cache that remembers recent translations:
--
--   TLB hit:  VPN already cached → physical address in one cycle
--   TLB miss: Walk page table, cache the result, use it
--
-- Programs exhibit temporal locality (accessing the same pages repeatedly),
-- so a 64-entry TLB achieves >95% hit rates in practice.
--
-- @module coding_adventures.virtual_memory

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Constants
-- ============================================================================

-- 4 KB page size: standard since Intel 386 (1985) and RISC-V.
M.PAGE_SIZE         = 4096

-- 12 bits to address every byte in a 4 KB page (2^12 = 4096).
M.PAGE_OFFSET_BITS  = 12

-- Mask for the lower 12 bits of an address.
M.PAGE_OFFSET_MASK  = 0xFFF

-- Default TLB capacity (64 entries is realistic; real TLBs have 32-256).
M.DEFAULT_TLB_CAPACITY = 64

-- ============================================================================
-- PageTableEntry
-- ============================================================================

--- Create a new Page Table Entry (PTE).
--
-- A PTE describes the mapping for one virtual page. It mirrors the fields
-- in RISC-V's Sv32 PTE format:
--
--   V (valid/present)   — is this page in physical memory?
--   W (writable)        — can it be written to?
--   X (executable)      — can it contain machine code?
--   U (user-accessible) — can user-mode code access it?
--   A (accessed)        — has it been read or written recently?
--   D (dirty)           — has it been written since it was loaded?
--
-- The frame_number field holds the physical frame this page maps to.
--
-- @param frame_number  integer — physical frame index
-- @param opts          table   — optional flags
-- @return table (PTE)
function M.new_pte(frame_number, opts)
    opts = opts or {}
    return {
        frame_number   = frame_number,
        present        = opts.present        ~= false,  -- default true
        dirty          = opts.dirty          or false,
        accessed       = opts.accessed       or false,
        writable       = opts.writable       or false,
        executable     = opts.executable     or false,
        user_accessible = opts.user_accessible or false,
    }
end

-- ============================================================================
-- Single-Level Page Table
-- ============================================================================

--- Create a new single-level page table.
-- Maps VPN (integer) → PTE (table).
-- This is the simplest possible implementation: a Lua table used as a hash map.
--
-- @return table (page table)
function M.new_page_table()
    return { entries = {} }
end

--- Look up the PTE for a virtual page number.
-- @param pt   table — page table
-- @param vpn  integer — virtual page number
-- @return table|nil — PTE if found, nil otherwise
function M.pt_lookup(pt, vpn)
    return pt.entries[vpn]
end

--- Insert or update a mapping in the page table.
-- @param pt           table  — page table (mutated)
-- @param vpn          integer — virtual page number
-- @param frame_number integer — physical frame number
-- @param flags        table   — optional PTE flags
function M.pt_map(pt, vpn, frame_number, flags)
    flags = flags or {}
    pt.entries[vpn] = M.new_pte(frame_number, flags)
end

--- Remove a mapping from the page table.
-- @param pt   table   — page table (mutated)
-- @param vpn  integer — virtual page number
-- @return table|nil — the removed PTE, or nil
function M.pt_unmap(pt, vpn)
    local pte = pt.entries[vpn]
    pt.entries[vpn] = nil
    return pte
end

--- Insert a PTE directly (for update use cases).
function M.pt_insert(pt, vpn, pte)
    pt.entries[vpn] = pte
end

--- Return count of mapped pages.
function M.pt_mapped_count(pt)
    local count = 0
    for _ in pairs(pt.entries) do count = count + 1 end
    return count
end

-- ============================================================================
-- Two-Level Page Table (Sv32)
-- ============================================================================

--- Create a new two-level page table (RISC-V Sv32 style).
-- The directory has up to 1024 entries; each points to a second-level
-- page table (or is nil). Second-level tables are created on demand.
--
-- @return table (two-level page table)
function M.new_two_level_pt()
    return { directory = {} }
end

--- Split a 32-bit virtual address into VPN[1], VPN[0], and page offset.
--
--   address = 0x00012ABC
--   vpn = 0x00012ABC >> 12 = 0x12 = 18
--   vpn1 = (18 >> 10) & 0x3FF = 0
--   vpn0 = 18 & 0x3FF = 18
--   offset = 0xABC
--
-- @param vaddr  integer — 32-bit virtual address
-- @return vpn1, vpn0, offset  (all integers)
function M.split_address(vaddr)
    local addr = vaddr % 4294967296  -- ensure unsigned 32-bit
    local vpn = math.floor(addr / M.PAGE_SIZE)
    local vpn1 = math.floor(vpn / 1024) % 1024  -- upper 10 bits of VPN
    local vpn0 = vpn % 1024                       -- lower 10 bits of VPN
    local offset = addr % M.PAGE_SIZE
    return vpn1, vpn0, offset
end

--- Map a virtual address to a physical frame in the two-level page table.
-- Creates the second-level table on demand if not present.
--
-- @param tpt          table   — two-level page table (mutated)
-- @param vaddr        integer — virtual address (used to compute VPN)
-- @param frame_number integer — physical frame
-- @param flags        table   — optional PTE flags
function M.tpt_map(tpt, vaddr, frame_number, flags)
    local vpn1, vpn0, _ = M.split_address(vaddr)
    local table2 = tpt.directory[vpn1]
    if table2 == nil then
        table2 = M.new_page_table()
        tpt.directory[vpn1] = table2
    end
    M.pt_map(table2, vpn0, frame_number, flags)
end

--- Translate a virtual address to {physical_address, pte} or nil.
-- Returns nil if there is no mapping or if present=false.
--
-- @param tpt    table   — two-level page table
-- @param vaddr  integer — virtual address
-- @return integer, table | nil  (physical_addr, pte) or nil
function M.tpt_translate(tpt, vaddr)
    local vpn1, vpn0, offset = M.split_address(vaddr)
    local table2 = tpt.directory[vpn1]
    if table2 == nil then return nil end
    local pte = M.pt_lookup(table2, vpn0)
    if pte == nil or not pte.present then return nil end
    local phys = pte.frame_number * M.PAGE_SIZE + offset
    return phys, pte
end

--- Look up the PTE for a virtual address (without computing physical address).
function M.tpt_lookup_pte(tpt, vaddr)
    local vpn1, vpn0, _ = M.split_address(vaddr)
    local table2 = tpt.directory[vpn1]
    if table2 == nil then return nil end
    return M.pt_lookup(table2, vpn0)
end

--- Remove a virtual address mapping.
-- @return table|nil — removed PTE or nil
function M.tpt_unmap(tpt, vaddr)
    local vpn1, vpn0, _ = M.split_address(vaddr)
    local table2 = tpt.directory[vpn1]
    if table2 == nil then return nil end
    return M.pt_unmap(table2, vpn0)
end

--- Update the PTE at a virtual address by applying a function to it.
-- @param tpt       table    — two-level page table (mutated)
-- @param vaddr     integer  — virtual address
-- @param update_fn function — takes old PTE, returns new PTE
function M.tpt_update_pte(tpt, vaddr, update_fn)
    local vpn1, vpn0, _ = M.split_address(vaddr)
    local table2 = tpt.directory[vpn1]
    if table2 == nil then return end
    local pte = M.pt_lookup(table2, vpn0)
    if pte == nil then return end
    M.pt_insert(table2, vpn0, update_fn(pte))
end

--- Return all mappings as an array of {vaddr, pte} pairs.
function M.tpt_all_mappings(tpt)
    local result = {}
    for vpn1, table2 in pairs(tpt.directory) do
        for vpn0, pte in pairs(table2.entries) do
            local vpn = vpn1 * 1024 + vpn0
            local vaddr = vpn * M.PAGE_SIZE
            result[#result + 1] = {vaddr = vaddr, pte = pte}
        end
    end
    return result
end

-- ============================================================================
-- TLB (Translation Lookaside Buffer)
-- ============================================================================

--- Create a new TLB.
--
-- The TLB is a small, fast cache of recent virtual-to-physical translations.
-- It is keyed by {pid, vpn} so different processes can coexist without
-- their translations colliding.
--
-- Eviction policy: LRU (least recently used). The `order` array tracks
-- insertion/access order; the front is the least recently used.
--
-- @param capacity  integer — max entries (default 64)
-- @return table (TLB)
function M.new_tlb(capacity)
    capacity = capacity or M.DEFAULT_TLB_CAPACITY
    return {
        capacity = capacity,
        entries  = {},  -- key = "pid:vpn", value = {frame=..., pte=...}
        order    = {},  -- array of keys, front = LRU
        hits     = 0,
        misses   = 0,
    }
end

local function tlb_key(pid, vpn)
    return tostring(pid) .. ":" .. tostring(vpn)
end

--- Look up a cached translation.
-- @param tlb  table   — TLB (mutated: updates hits/misses and order)
-- @param pid  integer — process ID
-- @param vpn  integer — virtual page number
-- @return table|nil — {frame, pte} on hit, nil on miss
function M.tlb_lookup(tlb, pid, vpn)
    local key = tlb_key(pid, vpn)
    local entry = tlb.entries[key]
    if entry == nil then
        tlb.misses = tlb.misses + 1
        return nil
    end
    -- Move to end of order (most recently used)
    for i, k in ipairs(tlb.order) do
        if k == key then
            table.remove(tlb.order, i)
            break
        end
    end
    tlb.order[#tlb.order + 1] = key
    tlb.hits = tlb.hits + 1
    return entry
end

--- Insert a translation into the TLB.
-- Evicts the LRU entry if at capacity.
-- @param tlb    table   — TLB (mutated)
-- @param pid    integer — process ID
-- @param vpn    integer — virtual page number
-- @param frame  integer — physical frame number
-- @param pte    table   — page table entry
function M.tlb_insert(tlb, pid, vpn, frame, pte)
    local key = tlb_key(pid, vpn)
    -- Remove existing entry for this key if present
    if tlb.entries[key] ~= nil then
        tlb.entries[key] = nil
        for i, k in ipairs(tlb.order) do
            if k == key then
                table.remove(tlb.order, i)
                break
            end
        end
    end
    -- Evict LRU if at capacity
    local count = 0
    for _ in pairs(tlb.entries) do count = count + 1 end
    if count >= tlb.capacity then
        local lru_key = tlb.order[1]
        table.remove(tlb.order, 1)
        tlb.entries[lru_key] = nil
    end
    tlb.entries[key] = {frame = frame, pte = pte}
    tlb.order[#tlb.order + 1] = key
end

--- Invalidate a specific TLB entry.
function M.tlb_invalidate(tlb, pid, vpn)
    local key = tlb_key(pid, vpn)
    tlb.entries[key] = nil
    for i, k in ipairs(tlb.order) do
        if k == key then
            table.remove(tlb.order, i)
            break
        end
    end
end

--- Flush ALL TLB entries (called on context switch).
-- When the kernel switches to a new process, the old process's translations
-- are all stale. Flushing prevents the new process from reading the old one's
-- memory — a critical security measure!
function M.tlb_flush(tlb)
    tlb.entries = {}
    tlb.order   = {}
end

--- Compute the TLB hit rate.
-- @return number — hits / (hits + misses), or 0.0 if no lookups
function M.tlb_hit_rate(tlb)
    local total = tlb.hits + tlb.misses
    if total == 0 then return 0.0 end
    return tlb.hits / total
end

--- Return the number of cached entries.
function M.tlb_size(tlb)
    local count = 0
    for _ in pairs(tlb.entries) do count = count + 1 end
    return count
end

-- ============================================================================
-- Physical Frame Allocator
-- ============================================================================

--- Create a new physical frame allocator.
--
-- Manages which physical frames are free and which are in use.
-- Uses a Lua table as a "bitmap": allocated[frame] = true means in use.
--
-- allocate() scans linearly for the first free frame — O(n) but simple.
-- Real OS allocators use free lists or buddy systems for O(1) performance.
--
-- @param total_frames  integer — total number of physical frames
-- @return table (allocator)
function M.new_frame_allocator(total_frames)
    return {
        total_frames = total_frames,
        allocated    = {},  -- frame_number → true if allocated
        free_count   = total_frames,
    }
end

--- Allocate the first free frame.
-- @param alloc  table — allocator (mutated)
-- @return integer|nil — frame number, or nil if out of memory
function M.alloc_frame(alloc)
    for i = 0, alloc.total_frames - 1 do
        if not alloc.allocated[i] then
            alloc.allocated[i] = true
            alloc.free_count = alloc.free_count - 1
            return i
        end
    end
    return nil  -- out of memory
end

--- Free a physical frame.
-- @param alloc         table   — allocator (mutated)
-- @param frame_number  integer — frame to free
function M.free_frame(alloc, frame_number)
    if frame_number < 0 or frame_number >= alloc.total_frames then
        error(string.format("Frame %d out of range [0, %d)", frame_number, alloc.total_frames))
    end
    if not alloc.allocated[frame_number] then
        error(string.format("Double-free: frame %d is already free", frame_number))
    end
    alloc.allocated[frame_number] = nil
    alloc.free_count = alloc.free_count + 1
end

--- Check if a frame is allocated.
function M.frame_is_allocated(alloc, frame_number)
    if frame_number < 0 or frame_number >= alloc.total_frames then
        error(string.format("Frame %d out of range", frame_number))
    end
    return alloc.allocated[frame_number] == true
end

-- ============================================================================
-- Page Replacement Policies
-- ============================================================================

--- FIFO (First-In, First-Out) replacement policy.
--
-- The simplest policy: evict the frame that has been in memory the longest.
-- Simple but can be pathological — it might evict a frequently used page.
--
-- @return table (FIFO policy)
function M.new_fifo_policy()
    return {
        type  = "fifo",
        queue = {},  -- array of frame numbers, oldest at front (index 1)
    }
end

function M.policy_add_frame(policy, frame)
    if policy.type == "fifo" then
        policy.queue[#policy.queue + 1] = frame
    elseif policy.type == "lru" then
        policy.access_order[#policy.access_order + 1] = frame
    elseif policy.type == "clock" then
        policy.frames[#policy.frames + 1] = frame
        policy.use_bits[frame] = true
    end
end

function M.policy_record_access(policy, frame)
    if policy.type == "fifo" then
        -- FIFO ignores access patterns
    elseif policy.type == "lru" then
        -- Move frame to end (most recently used)
        for i, f in ipairs(policy.access_order) do
            if f == frame then
                table.remove(policy.access_order, i)
                break
            end
        end
        policy.access_order[#policy.access_order + 1] = frame
    elseif policy.type == "clock" then
        -- Set use bit
        policy.use_bits[frame] = true
    end
end

function M.policy_select_victim(policy)
    if policy.type == "fifo" then
        if #policy.queue == 0 then return nil end
        local victim = policy.queue[1]
        table.remove(policy.queue, 1)
        return victim
    elseif policy.type == "lru" then
        if #policy.access_order == 0 then return nil end
        local victim = policy.access_order[1]
        table.remove(policy.access_order, 1)
        return victim
    elseif policy.type == "clock" then
        return M._clock_select_victim(policy)
    end
    return nil
end

function M.policy_remove_frame(policy, frame)
    if policy.type == "fifo" then
        for i, f in ipairs(policy.queue) do
            if f == frame then
                table.remove(policy.queue, i)
                return
            end
        end
    elseif policy.type == "lru" then
        for i, f in ipairs(policy.access_order) do
            if f == frame then
                table.remove(policy.access_order, i)
                return
            end
        end
    elseif policy.type == "clock" then
        for i, f in ipairs(policy.frames) do
            if f == frame then
                table.remove(policy.frames, i)
                policy.use_bits[frame] = nil
                if #policy.frames > 0 then
                    policy.hand = policy.hand % #policy.frames
                else
                    policy.hand = 0
                end
                return
            end
        end
    end
end

--- LRU (Least Recently Used) replacement policy.
--
-- Evict the page that has not been accessed for the longest time.
-- Based on temporal locality: recently used pages are likely to be used again.
--
-- @return table (LRU policy)
function M.new_lru_policy()
    return {
        type         = "lru",
        access_order = {},  -- front = LRU, back = MRU
    }
end

--- Clock (Second-Chance) replacement policy.
--
-- A practical approximation of LRU. A clock hand sweeps around a circular
-- buffer of pages. Pages with use_bit=1 get a second chance (bit cleared,
-- hand advances). Pages with use_bit=0 are evicted.
--
-- Used by most real operating systems because it is cheap to implement.
--
-- @return table (Clock policy)
function M.new_clock_policy()
    return {
        type      = "clock",
        frames    = {},         -- circular list of frame numbers
        use_bits  = {},         -- frame → boolean
        hand      = 0,          -- current hand position (0-based index into frames)
    }
end

-- Internal: clock algorithm victim selection.
function M._clock_select_victim(policy)
    if #policy.frames == 0 then return nil end
    local max_scans = #policy.frames * 2
    for _ = 1, max_scans do
        local hand_idx = policy.hand + 1  -- 1-based
        if hand_idx > #policy.frames then hand_idx = 1 end
        policy.hand = hand_idx - 1

        local frame = policy.frames[hand_idx]
        if not policy.use_bits[frame] then
            -- Evict this frame
            table.remove(policy.frames, hand_idx)
            policy.use_bits[frame] = nil
            if #policy.frames > 0 then
                policy.hand = policy.hand % #policy.frames
            else
                policy.hand = 0
            end
            return frame
        else
            -- Second chance: clear use bit and advance
            policy.use_bits[frame] = false
            policy.hand = hand_idx % #policy.frames
        end
    end
    return nil
end

-- ============================================================================
-- MMU (Memory Management Unit)
-- ============================================================================

--- Create a new MMU.
--
-- The MMU ties everything together:
--   - One two-level page table per process (keyed by PID)
--   - One shared TLB
--   - A physical frame allocator
--   - A configurable page replacement policy
--   - Reference counts for copy-on-write sharing
--
-- @param total_frames   integer — total physical frames available
-- @param policy_type    string  — "fifo", "lru", or "clock" (default "lru")
-- @param tlb_capacity   integer — TLB size (default 64)
-- @return table (MMU)
function M.new_mmu(total_frames, policy_type, tlb_capacity)
    policy_type = policy_type or "lru"
    local policy
    if policy_type == "fifo" then
        policy = M.new_fifo_policy()
    elseif policy_type == "clock" then
        policy = M.new_clock_policy()
    else
        policy = M.new_lru_policy()
    end
    return {
        page_tables     = {},  -- pid → two-level page table
        tlb             = M.new_tlb(tlb_capacity),
        frame_allocator = M.new_frame_allocator(total_frames),
        policy          = policy,
        policy_type     = policy_type,
        frame_refcounts = {},  -- frame → reference count
    }
end

--- Create a new empty address space for a process.
function M.mmu_create_address_space(mmu, pid)
    mmu.page_tables[pid] = M.new_two_level_pt()
end

--- Destroy a process's address space, freeing all owned frames.
function M.mmu_destroy_address_space(mmu, pid)
    local tpt = mmu.page_tables[pid]
    if tpt == nil then return end
    local mappings = M.tpt_all_mappings(tpt)
    for _, m in ipairs(mappings) do
        if m.pte.present then
            M._mmu_decrement_refcount(mmu, m.pte.frame_number)
        end
    end
    mmu.page_tables[pid] = nil
end

--- Map a virtual address to a newly allocated physical frame.
-- @param mmu    table   — MMU (mutated)
-- @param pid    integer — process ID
-- @param vaddr  integer — virtual address (page-aligned is typical)
-- @param flags  table   — optional PTE flags
-- @return integer|nil — allocated frame number, or nil if out of memory
function M.mmu_map_page(mmu, pid, vaddr, flags)
    local tpt = mmu.page_tables[pid]
    if tpt == nil then
        error(string.format("No address space for PID %d", pid))
    end
    local frame = M.alloc_frame(mmu.frame_allocator)
    if frame == nil then return nil end
    M.tpt_map(tpt, vaddr, frame, flags)
    mmu.frame_refcounts[frame] = 1
    M.policy_add_frame(mmu.policy, frame)
    return frame
end

--- Translate a virtual address to a physical address.
--
-- This is the core MMU operation. Every memory access goes through here.
-- Steps:
--   1. Extract VPN and page offset from virtual address.
--   2. Check TLB (fast path — O(1) lookup).
--   3. On TLB miss, walk the two-level page table (slow path).
--   4. On page fault (not present), allocate a frame.
--   5. Update accessed/dirty bits.
--   6. Cache translation in TLB.
--   7. Compute physical address = (frame << 12) | offset.
--
-- @param mmu       table   — MMU (mutated)
-- @param pid       integer — process ID
-- @param vaddr     integer — virtual address
-- @param is_write  boolean — is this a write access? (updates dirty bit)
-- @return integer|nil — physical address, or nil on fatal fault
function M.mmu_translate(mmu, pid, vaddr, is_write)
    local addr = vaddr % 4294967296
    local vpn = math.floor(addr / M.PAGE_SIZE)
    local offset = addr % M.PAGE_SIZE

    -- Step 1: Check TLB
    local tlb_entry = M.tlb_lookup(mmu.tlb, pid, vpn)
    if tlb_entry ~= nil then
        -- TLB hit!
        local frame = tlb_entry.frame
        local pte = tlb_entry.pte
        if is_write and not pte.writable then
            -- COW or permission fault — handled below
            return M._mmu_handle_cow(mmu, pid, vaddr, is_write)
        end
        pte.accessed = true
        if is_write then pte.dirty = true end
        -- Re-insert with updated PTE
        M.tlb_insert(mmu.tlb, pid, vpn, frame, pte)
        -- Update PTE in page table
        local tpt = mmu.page_tables[pid]
        if tpt then M.tpt_update_pte(tpt, addr, function(_) return pte end) end
        M.policy_record_access(mmu.policy, frame)
        return frame * M.PAGE_SIZE + offset
    end

    -- Step 2: TLB miss — walk the page table
    local tpt = mmu.page_tables[pid]
    if tpt == nil then
        error(string.format("No address space for PID %d", pid))
    end
    local phys, pte = M.tpt_translate(tpt, addr)
    if phys == nil then
        -- Page fault: allocate a frame
        local new_frame = M.mmu_handle_page_fault(mmu, pid, vaddr)
        if new_frame == nil then return nil end
        -- Re-translate after fault resolution
        phys, pte = M.tpt_translate(mmu.page_tables[pid], addr)
        if phys == nil then return nil end
    end
    if is_write and not pte.writable then
        return M._mmu_handle_cow(mmu, pid, vaddr, is_write)
    end
    pte.accessed = true
    if is_write then pte.dirty = true end
    M.tpt_update_pte(tpt, addr, function(_) return pte end)
    M.tlb_insert(mmu.tlb, pid, vpn, pte.frame_number, pte)
    M.policy_record_access(mmu.policy, pte.frame_number)
    return pte.frame_number * M.PAGE_SIZE + offset
end

--- Handle a page fault by allocating a physical frame.
-- @return integer|nil — newly allocated frame number, or nil on OOM
function M.mmu_handle_page_fault(mmu, pid, vaddr)
    local tpt = mmu.page_tables[pid]
    if tpt == nil then return nil end
    local addr = vaddr % 4294967296
    local pte = M.tpt_lookup_pte(tpt, addr)
    if pte == nil then
        -- Segfault: page was never mapped — this would kill the process
        return nil
    end
    if pte.present then
        -- Already present (race condition in concurrent code): nothing to do
        return pte.frame_number
    end
    -- Allocate a physical frame
    local frame = M.alloc_frame(mmu.frame_allocator)
    if frame == nil then return nil end
    pte.frame_number = frame
    pte.present = true
    pte.accessed = true
    mmu.frame_refcounts[frame] = 1
    M.policy_add_frame(mmu.policy, frame)
    -- Flush any stale TLB entry
    local vpn = math.floor(addr / M.PAGE_SIZE)
    M.tlb_invalidate(mmu.tlb, pid, vpn)
    return frame
end

--- Clone an address space (copy-on-write fork semantics).
-- Both parent and child share the same frames. All shared pages are marked
-- read-only. On write, a COW fault triggers a private copy.
function M.mmu_clone_address_space(mmu, src_pid, dst_pid)
    mmu.page_tables[dst_pid] = M.new_two_level_pt()
    local src_tpt = mmu.page_tables[src_pid]
    local dst_tpt = mmu.page_tables[dst_pid]
    if src_tpt == nil then return end
    local mappings = M.tpt_all_mappings(src_tpt)
    for _, m in ipairs(mappings) do
        -- Mark both copies read-only for COW
        local src_pte = m.pte
        local cow_pte = M.new_pte(src_pte.frame_number, {
            present        = src_pte.present,
            dirty          = src_pte.dirty,
            accessed       = src_pte.accessed,
            writable       = false,   -- COW: read-only until written
            executable     = src_pte.executable,
            user_accessible = src_pte.user_accessible,
        })
        -- Update source to read-only too
        M.tpt_update_pte(src_tpt, m.vaddr, function(_) return cow_pte end)
        M.tpt_map(dst_tpt, m.vaddr, src_pte.frame_number, {
            present        = src_pte.present,
            dirty          = false,
            accessed       = false,
            writable       = false,
            executable     = src_pte.executable,
            user_accessible = src_pte.user_accessible,
        })
        -- Increment reference count for the shared frame
        local rc = mmu.frame_refcounts[src_pte.frame_number] or 1
        mmu.frame_refcounts[src_pte.frame_number] = rc + 1
    end
    -- Flush TLB for both PIDs (translations are now read-only)
    M.tlb_flush(mmu.tlb)
end

--- Handle a copy-on-write fault: allocate a new frame and copy data.
-- For simulation, we just allocate a new frame and remap the page as writable.
-- (A real OS would copy the contents of the old frame into the new one.)
function M._mmu_handle_cow(mmu, pid, vaddr, is_write)
    local addr = vaddr % 4294967296
    local tpt = mmu.page_tables[pid]
    if tpt == nil then return nil end
    local pte = M.tpt_lookup_pte(tpt, addr)
    if pte == nil then return nil end
    local old_frame = pte.frame_number
    -- Allocate a new private frame
    local new_frame = M.alloc_frame(mmu.frame_allocator)
    if new_frame == nil then return nil end
    -- Remap the page to the new frame with write permission
    M.tpt_update_pte(tpt, addr, function(p)
        local np = M.new_pte(new_frame, {
            present        = true,
            dirty          = true,
            accessed       = true,
            writable       = true,
            executable     = p.executable,
            user_accessible = p.user_accessible,
        })
        return np
    end)
    mmu.frame_refcounts[new_frame] = 1
    M.policy_add_frame(mmu.policy, new_frame)
    -- Decrement refcount on old frame
    M._mmu_decrement_refcount(mmu, old_frame)
    -- Invalidate TLB for this VPN
    local vpn = math.floor(addr / M.PAGE_SIZE)
    M.tlb_invalidate(mmu.tlb, pid, vpn)
    local offset = addr % M.PAGE_SIZE
    return new_frame * M.PAGE_SIZE + offset
end

function M._mmu_decrement_refcount(mmu, frame)
    local rc = (mmu.frame_refcounts[frame] or 1) - 1
    if rc <= 0 then
        mmu.frame_refcounts[frame] = nil
        M.policy_remove_frame(mmu.policy, frame)
        M.free_frame(mmu.frame_allocator, frame)
    else
        mmu.frame_refcounts[frame] = rc
    end
end

return M
