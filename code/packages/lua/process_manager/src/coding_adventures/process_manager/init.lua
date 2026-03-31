-- ============================================================================
-- process_manager — Process Lifecycle Management
-- ============================================================================
--
-- Every program you run is a **process** — an instance of a program in
-- execution.  When you open a text editor, that's a process.  When you
-- type `ls` in a shell, that's a process.
--
-- But how are processes *created*?  Unix solved this elegantly with three
-- system calls: **fork**, **exec**, and **wait**.
--
-- ## The Restaurant Kitchen Analogy
--
-- The head chef (parent process) can:
--
--   fork()  — Clone themselves: two identical chefs with the same knowledge.
--             The clone (child) can immediately start different work.
--   exec()  — The clone throws away their recipe book and picks up a new one.
--             Same person (same PID), completely different work.
--   wait()  — The head chef pauses and watches the clone work.  Resumes when
--             the clone finishes and leaves.
--
-- Shell example (when you type `ls`):
--
--   Shell (PID 100)
--   │
--   ├── fork() → creates child (PID 101), exact copy of the shell
--   │   │
--   │   ├── [Child PID 101]:   exec("ls")
--   │   │     ls runs, prints files, exits.
--   │   │
--   │   └── [Parent PID 100]:  wait(101)
--   │         Pauses until ls exits.
--   │
--   └── Shell prompt reappears.
--
-- ## Process State Machine
--
--   fork() ──► ready ──[scheduled]──► running ──[exit()]──► zombie
--                ▲                       │                      │
--                │                       ▼                   [wait()]
--                └────── blocked ◄──[I/O wait]             REMOVED
--
--   SIGSTOP → blocked ──[SIGCONT]──► ready
--
--   States:
--     ready      (0) — Loaded in memory, waiting for CPU time
--     running    (1) — Currently executing on the CPU
--     blocked    (2) — Waiting for I/O or an event
--     terminated (3) — Finished execution (transient before zombie)
--     zombie     (4) — Exited, parent has not called wait() yet
--
-- ## Process Control Block (PCB)
--
-- The PCB is the kernel's "passport" for each process.  It stores everything
-- the kernel needs to track, suspend, and resume a process:
--
--   ┌──────────────────────────────────────────────────────────┐
--   │  PCB for PID 101                                         │
--   │  ┌──────────────┬─────────────────────────────────────┐  │
--   │  │ pid          │ 101                                 │  │
--   │  │ name         │ "ls"                                │  │
--   │  │ state        │ running                             │  │
--   │  │ registers    │ [r0=0, r1=0x1000, r2=0, ...]        │  │
--   │  │ pc           │ 0x00401234                           │  │
--   │  │ sp           │ 0x7FFF0000                           │  │
--   │  │ parent_pid   │ 100                                 │  │
--   │  │ priority     │ 20                                  │  │
--   │  └──────────────┴─────────────────────────────────────┘  │
--   └──────────────────────────────────────────────────────────┘
--
-- ## Signals
--
-- Signals are software interrupts sent between processes.  Common signals:
--
--   ┌──────────┬────────┬───────────────────┬─────────────┐
--   │ Name     │ Number │ Default Action    │ Catchable?  │
--   ├──────────┼────────┼───────────────────┼─────────────┤
--   │ SIGINT   │   2    │ Terminate         │ Yes         │
--   │ SIGKILL  │   9    │ Terminate         │ NO          │
--   │ SIGTERM  │  15    │ Terminate         │ Yes         │
--   │ SIGCHLD  │  17    │ Ignore (default)  │ Yes         │
--   │ SIGCONT  │  18    │ Continue          │ Yes         │
--   │ SIGSTOP  │  19    │ Stop              │ NO          │
--   └──────────┴────────┴───────────────────┴─────────────┘
--
-- SIGKILL and SIGSTOP cannot be caught or ignored because the kernel must
-- always be able to forcibly terminate or suspend any process.
--
-- ## Module Structure
--
--   process_manager
--   ├── PCB            — Process Control Block
--   └── Manager        — process table, fork/exec/wait/kill, scheduler
--
-- ============================================================================

local M = {}

-- ============================================================================
-- Constants
-- ============================================================================

-- Process states (numeric values for cross-language compatibility)
M.STATE_READY      = 0
M.STATE_RUNNING    = 1
M.STATE_BLOCKED    = 2
M.STATE_TERMINATED = 3
M.STATE_ZOMBIE     = 4

-- Signal numbers (POSIX standard)
M.SIGINT  =  2
M.SIGKILL =  9
M.SIGTERM = 15
M.SIGCHLD = 17
M.SIGCONT = 18
M.SIGSTOP = 19

-- Priority range: 0 (highest) to 39 (lowest), default 20 (normal)
M.DEFAULT_PRIORITY   = 20
M.MIN_PRIORITY       = 0
M.MAX_PRIORITY       = 39

-- ============================================================================
-- PCB — Process Control Block
-- ============================================================================
--
-- The PCB is every process's "passport": the kernel keeps one PCB per process
-- and uses it to suspend, resume, and identify the process.
--
-- Fields:
--   pid              Unique process ID (integer, assigned by Manager)
--   name             Human-readable name (e.g., "ls", "init", "shell")
--   state            Current lifecycle state (0..4, see STATE_* constants)
--   registers        Table of 32 register values (CPU state when suspended)
--   pc               Program counter — address of next instruction
--   sp               Stack pointer
--   memory_base      Base address of this process's memory region
--   memory_size      Size of memory region in bytes
--   parent_pid       PID of the process that created this one
--   children         List of child PIDs
--   pending_signals  List of signals queued but not yet delivered
--   signal_handlers  Map of signal_number → handler_address
--   signal_mask      Set of signal numbers that are blocked (table as set)
--   priority         Scheduling priority (0=highest, 39=lowest)
--   cpu_time         Total CPU cycles consumed (for accounting)
--   exit_code        Exit status — meaningful when state=zombie

M.PCB = {}
M.PCB.__index = M.PCB

--- Create a new PCB.
-- @param pid    Unique process ID
-- @param name   Human-readable process name
-- @param opts   Optional overrides: priority, memory_base, memory_size, parent_pid
function M.PCB.new(pid, name, opts)
  opts = opts or {}
  local regs = {}
  for i = 1, 32 do regs[i] = 0 end
  return setmetatable({
    pid             = pid,
    name            = name,
    state           = M.STATE_READY,
    registers       = regs,
    pc              = 0,
    sp              = 0,
    memory_base     = opts.memory_base  or 0,
    memory_size     = opts.memory_size  or 0,
    parent_pid      = opts.parent_pid   or 0,
    children        = {},
    pending_signals = {},
    signal_handlers = {},
    signal_mask     = {},   -- set: signal_number → true
    priority        = opts.priority or M.DEFAULT_PRIORITY,
    cpu_time        = 0,
    exit_code       = 0,
  }, M.PCB)
end

-- Internal: deep-copy a PCB
local function copy_pcb(pcb)
  local regs = {}
  for i = 1, #pcb.registers do regs[i] = pcb.registers[i] end
  local children = {}
  for _, v in ipairs(pcb.children) do table.insert(children, v) end
  local sigs = {}
  for _, v in ipairs(pcb.pending_signals) do table.insert(sigs, v) end
  local handlers = {}
  for k, v in pairs(pcb.signal_handlers) do handlers[k] = v end
  local mask = {}
  for k, v in pairs(pcb.signal_mask) do mask[k] = v end
  return setmetatable({
    pid             = pcb.pid,
    name            = pcb.name,
    state           = pcb.state,
    registers       = regs,
    pc              = pcb.pc,
    sp              = pcb.sp,
    memory_base     = pcb.memory_base,
    memory_size     = pcb.memory_size,
    parent_pid      = pcb.parent_pid,
    children        = children,
    pending_signals = sigs,
    signal_handlers = handlers,
    signal_mask     = mask,
    priority        = pcb.priority,
    cpu_time        = pcb.cpu_time,
    exit_code       = pcb.exit_code,
  }, M.PCB)
end

--- Set the process state.
function M.PCB:set_state(new_state)
  local p = copy_pcb(self)
  p.state = new_state
  return p
end

--- Save CPU context into the PCB (called on context switch or interrupt).
-- @param registers  Table of register values
-- @param pc         Program counter
-- @param sp         Stack pointer
function M.PCB:save_context(registers, pc, sp)
  local p = copy_pcb(self)
  p.registers = registers or p.registers
  p.pc = pc or p.pc
  p.sp = sp or p.sp
  return p
end

--- Add a signal to the pending queue.
-- @param sig  Signal number (e.g., M.SIGTERM)
function M.PCB:add_signal(sig)
  -- Do not duplicate pending signals
  for _, s in ipairs(self.pending_signals) do
    if s == sig then return self end
  end
  local p = copy_pcb(self)
  table.insert(p.pending_signals, sig)
  return p
end

--- Check if a signal is in the mask (blocked).
function M.PCB:is_masked(sig)
  return self.signal_mask[sig] == true
end

--- Mask (block) a signal.
function M.PCB:mask_signal(sig)
  local p = copy_pcb(self)
  p.signal_mask[sig] = true
  return p
end

--- Unmask (unblock) a signal.
function M.PCB:unmask_signal(sig)
  local p = copy_pcb(self)
  p.signal_mask[sig] = nil
  return p
end

--- Register a custom signal handler address.
function M.PCB:set_handler(sig, handler_addr)
  local p = copy_pcb(self)
  p.signal_handlers[sig] = handler_addr
  return p
end

--- Increment CPU time by delta cycles.
function M.PCB:tick_cpu(delta)
  local p = copy_pcb(self)
  p.cpu_time = p.cpu_time + (delta or 1)
  return p
end

-- ============================================================================
-- Manager — Process Table and Scheduler
-- ============================================================================
--
-- The Manager owns:
--   - process_table  Map of pid → PCB
--   - run_queue      List of ready-state PIDs (sorted by priority)
--   - next_pid       Counter for assigning new PIDs
--   - current_pid    PID of the currently running process (or nil)
--
-- ## Scheduler: Priority Round-Robin
--
-- We use priority-based scheduling with round-robin within each priority tier:
--
--   1. Among all ready processes, pick the one with the lowest priority number
--      (lowest number = highest priority).
--   2. If multiple processes share the same priority, they take turns in
--      round-robin order.
--
-- This is similar to Linux's O(1) scheduler without the dynamic priority
-- adjustments (nice values only).

M.Manager = {}
M.Manager.__index = M.Manager

--- Create a new Manager.
function M.Manager.new()
  return setmetatable({
    process_table = {},
    run_queue     = {},
    next_pid      = 1,
    current_pid   = nil,
  }, M.Manager)
end

-- Internal: shallow copy a manager
local function copy_mgr(m)
  local pt = {}
  for k, v in pairs(m.process_table) do pt[k] = v end
  local rq = {}
  for _, v in ipairs(m.run_queue) do table.insert(rq, v) end
  return setmetatable({
    process_table = pt,
    run_queue     = rq,
    next_pid      = m.next_pid,
    current_pid   = m.current_pid,
  }, M.Manager)
end

-- Internal: insert pid into run queue sorted by priority
local function enqueue(mgr, pid)
  local priority = mgr.process_table[pid] and mgr.process_table[pid].priority or M.DEFAULT_PRIORITY
  -- Remove if already present
  local rq = {}
  for _, v in ipairs(mgr.run_queue) do
    if v ~= pid then table.insert(rq, v) end
  end
  -- Find insertion point (stable sort: insert after peers of same priority)
  local inserted = false
  local result = {}
  for _, v in ipairs(rq) do
    local vp = mgr.process_table[v] and mgr.process_table[v].priority or M.DEFAULT_PRIORITY
    if not inserted and priority < vp then
      table.insert(result, pid)
      inserted = true
    end
    table.insert(result, v)
  end
  if not inserted then table.insert(result, pid) end
  mgr.run_queue = result
end

--- Get a PCB by PID. Returns nil if not found.
function M.Manager:get(pid)
  return self.process_table[pid]
end

--- Create a new process (fork without a parent — used for init, etc.).
-- Returns updated_manager, new_pid
function M.Manager:spawn(name, opts)
  local mgr  = copy_mgr(self)
  local pid  = mgr.next_pid
  mgr.next_pid = pid + 1
  local pcb = M.PCB.new(pid, name, opts)
  mgr.process_table[pid] = pcb
  enqueue(mgr, pid)
  return mgr, pid
end

--- Fork: clone a process into a child.
-- The child is an exact copy of the parent with:
--   - a new PID
--   - parent_pid set to the parent's PID
--   - empty children list
--   - ready state
-- Returns updated_manager, child_pid
function M.Manager:fork(parent_pid)
  local parent = self.process_table[parent_pid]
  if not parent then
    error("fork: no such process " .. tostring(parent_pid))
  end
  local mgr       = copy_mgr(self)
  local child_pid = mgr.next_pid
  mgr.next_pid    = child_pid + 1

  -- Clone the parent PCB
  local child = copy_pcb(parent)
  child.pid        = child_pid
  child.parent_pid = parent_pid
  child.children   = {}
  child.state      = M.STATE_READY
  child.cpu_time   = 0
  child.exit_code  = 0
  child.pending_signals = {}

  -- Record child in parent
  local p2 = copy_pcb(parent)
  table.insert(p2.children, child_pid)
  mgr.process_table[parent_pid] = p2
  mgr.process_table[child_pid]  = child
  enqueue(mgr, child_pid)
  return mgr, child_pid
end

--- Exec: replace a process's program.
-- Resets registers, PC, and name.  Keeps the PID, parent, and priority.
-- @param pid   Process ID to exec in
-- @param name  New program name
-- @param opts  Optional: pc (entry point), memory_base, memory_size
function M.Manager:exec(pid, name, opts)
  opts = opts or {}
  local pcb = self.process_table[pid]
  if not pcb then
    error("exec: no such process " .. tostring(pid))
  end
  local mgr = copy_mgr(self)
  local new_pcb = copy_pcb(pcb)
  new_pcb.name = name
  local regs = {}
  for i = 1, 32 do regs[i] = 0 end
  new_pcb.registers = regs
  new_pcb.pc = opts.pc or 0
  new_pcb.sp = 0
  new_pcb.memory_base = opts.memory_base or new_pcb.memory_base
  new_pcb.memory_size = opts.memory_size or new_pcb.memory_size
  mgr.process_table[pid] = new_pcb
  return mgr
end

--- Wait: wait for a specific child to exit (become zombie), then reap it.
-- If the child is already a zombie, reap immediately.
-- Returns updated_manager, exit_code  OR  "no_child", self, 0
function M.Manager:wait_child(parent_pid, child_pid)
  local parent = self.process_table[parent_pid]
  local child  = self.process_table[child_pid]
  if not parent or not child then
    return "no_child", self, 0
  end
  -- Check this is actually a child
  local is_child = false
  for _, cpid in ipairs(parent.children) do
    if cpid == child_pid then is_child = true; break end
  end
  if not is_child then
    return "no_child", self, 0
  end
  if child.state ~= M.STATE_ZOMBIE then
    return "not_exited", self, 0
  end
  -- Reap: remove child from table and parent's children list
  local exit_code = child.exit_code
  local mgr = copy_mgr(self)
  mgr.process_table[child_pid] = nil
  local p2 = copy_pcb(mgr.process_table[parent_pid])
  local new_children = {}
  for _, cpid in ipairs(p2.children) do
    if cpid ~= child_pid then table.insert(new_children, cpid) end
  end
  p2.children = new_children
  mgr.process_table[parent_pid] = p2
  return "ok", mgr, exit_code
end

--- Terminate (exit) a process.  Sets state to zombie with an exit code.
-- Sends SIGCHLD to parent.
-- Returns updated_manager
function M.Manager:exit_process(pid, exit_code)
  local pcb = self.process_table[pid]
  if not pcb then return self end
  local mgr = copy_mgr(self)
  local p2  = copy_pcb(pcb)
  p2.state     = M.STATE_ZOMBIE
  p2.exit_code = exit_code or 0
  mgr.process_table[pid] = p2

  -- Remove from run queue
  local rq = {}
  for _, v in ipairs(mgr.run_queue) do
    if v ~= pid then table.insert(rq, v) end
  end
  mgr.run_queue = rq

  -- Send SIGCHLD to parent
  if p2.parent_pid ~= 0 then
    local parent = mgr.process_table[p2.parent_pid]
    if parent then
      local pp = copy_pcb(parent)
      pp = pp:add_signal(M.SIGCHLD)
      mgr.process_table[p2.parent_pid] = pp
    end
  end

  if mgr.current_pid == pid then
    mgr.current_pid = nil
  end
  return mgr
end

--- Send a signal to a process.
-- SIGKILL and SIGSTOP are special (cannot be caught/masked).
-- SIGCONT resumes a blocked process.
-- Returns updated_manager
function M.Manager:kill(target_pid, sig)
  local pcb = self.process_table[target_pid]
  if not pcb then return self end
  local mgr = copy_mgr(self)

  if sig == M.SIGKILL then
    -- Uncatchable: terminate immediately
    return mgr:exit_process(target_pid, 137)  -- 128 + 9 (SIGKILL)
  end

  if sig == M.SIGSTOP then
    -- Uncatchable: block the process
    local p2 = copy_pcb(mgr.process_table[target_pid])
    if p2.state == M.STATE_RUNNING or p2.state == M.STATE_READY then
      p2.state = M.STATE_BLOCKED
      mgr.process_table[target_pid] = p2
      local rq = {}
      for _, v in ipairs(mgr.run_queue) do
        if v ~= target_pid then table.insert(rq, v) end
      end
      mgr.run_queue = rq
    end
    return mgr
  end

  if sig == M.SIGCONT then
    -- Resume a blocked process
    local p2 = copy_pcb(mgr.process_table[target_pid])
    if p2.state == M.STATE_BLOCKED then
      p2.state = M.STATE_READY
      mgr.process_table[target_pid] = p2
      enqueue(mgr, target_pid)
    end
    return mgr
  end

  -- Queue the signal (if not masked)
  local p2 = copy_pcb(mgr.process_table[target_pid])
  if not p2:is_masked(sig) then
    p2 = p2:add_signal(sig)
  end
  mgr.process_table[target_pid] = p2
  return mgr
end

--- Schedule: pick the next process to run (priority round-robin).
-- Sets the chosen process to running and returns updated_manager, chosen_pid.
-- Returns updated_manager, pid  OR  updated_manager, nil (if no ready processes)
function M.Manager:schedule()
  if #self.run_queue == 0 then
    return self, nil
  end
  local mgr = copy_mgr(self)

  -- Move current running process back to ready (context switch)
  if mgr.current_pid then
    local cur = mgr.process_table[mgr.current_pid]
    if cur and cur.state == M.STATE_RUNNING then
      local p2 = copy_pcb(cur)
      p2.state = M.STATE_READY
      mgr.process_table[mgr.current_pid] = p2
      enqueue(mgr, mgr.current_pid)
    end
  end

  -- Pick front of run queue (already sorted by priority)
  local chosen = mgr.run_queue[1]
  local rq = {}
  for i = 2, #mgr.run_queue do rq[i - 1] = mgr.run_queue[i] end
  mgr.run_queue = rq

  local p = copy_pcb(mgr.process_table[chosen])
  p.state = M.STATE_RUNNING
  mgr.process_table[chosen] = p
  mgr.current_pid = chosen

  return mgr, chosen
end

--- Block a process (waiting for I/O or an event).
function M.Manager:block(pid)
  local pcb = self.process_table[pid]
  if not pcb then return self end
  local mgr = copy_mgr(self)
  local p2  = copy_pcb(pcb)
  p2.state  = M.STATE_BLOCKED
  mgr.process_table[pid] = p2
  local rq = {}
  for _, v in ipairs(mgr.run_queue) do
    if v ~= pid then table.insert(rq, v) end
  end
  mgr.run_queue = rq
  if mgr.current_pid == pid then mgr.current_pid = nil end
  return mgr
end

--- Unblock a process (I/O complete, make it ready again).
function M.Manager:unblock(pid)
  local pcb = self.process_table[pid]
  if not pcb or pcb.state ~= M.STATE_BLOCKED then return self end
  local mgr = copy_mgr(self)
  local p2  = copy_pcb(pcb)
  p2.state  = M.STATE_READY
  mgr.process_table[pid] = p2
  enqueue(mgr, pid)
  return mgr
end

--- Count processes currently in a given state.
function M.Manager:count_in_state(state)
  local n = 0
  for _, pcb in pairs(self.process_table) do
    if pcb.state == state then n = n + 1 end
  end
  return n
end

--- Total number of processes in the table.
function M.Manager:total_processes()
  local n = 0
  for _ in pairs(self.process_table) do n = n + 1 end
  return n
end

return M
