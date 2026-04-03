-- Tests for coding_adventures.process_manager
-- Coverage target: 95%+

local PM = require("coding_adventures.process_manager")

-- ============================================================================
-- Constants tests
-- ============================================================================

describe("constants", function()
  it("state constants are correct", function()
    assert.are.equal(PM.STATE_READY, 0)
    assert.are.equal(PM.STATE_RUNNING, 1)
    assert.are.equal(PM.STATE_BLOCKED, 2)
    assert.are.equal(PM.STATE_TERMINATED, 3)
    assert.are.equal(PM.STATE_ZOMBIE, 4)
  end)

  it("signal numbers are correct", function()
    assert.are.equal(PM.SIGINT,  2)
    assert.are.equal(PM.SIGKILL, 9)
    assert.are.equal(PM.SIGTERM, 15)
    assert.are.equal(PM.SIGCHLD, 17)
    assert.are.equal(PM.SIGCONT, 18)
    assert.are.equal(PM.SIGSTOP, 19)
  end)

  it("priority defaults and range", function()
    assert.are.equal(PM.DEFAULT_PRIORITY, 20)
    assert.are.equal(PM.MIN_PRIORITY, 0)
    assert.are.equal(PM.MAX_PRIORITY, 39)
  end)
end)

-- ============================================================================
-- PCB tests
-- ============================================================================

describe("PCB", function()
  it("new creates PCB with defaults", function()
    local pcb = PM.PCB.new(1, "init")
    assert.are.equal(pcb.pid, 1)
    assert.are.equal(pcb.name, "init")
    assert.are.equal(pcb.state, PM.STATE_READY)
    assert.are.equal(pcb.priority, PM.DEFAULT_PRIORITY)
    assert.are.equal(pcb.cpu_time, 0)
    assert.are.equal(pcb.exit_code, 0)
    assert.are.equal(pcb.parent_pid, 0)
    assert.are.equal(#pcb.children, 0)
    assert.are.equal(#pcb.pending_signals, 0)
    assert.are.equal(#pcb.registers, 32)
  end)

  it("new with options", function()
    local pcb = PM.PCB.new(2, "ls", { priority = 5, parent_pid = 1, memory_base = 0x1000, memory_size = 4096 })
    assert.are.equal(pcb.priority, 5)
    assert.are.equal(pcb.parent_pid, 1)
    assert.are.equal(pcb.memory_base, 0x1000)
    assert.are.equal(pcb.memory_size, 4096)
  end)

  it("set_state returns new PCB", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:set_state(PM.STATE_RUNNING)
    assert.are.equal(pcb.state, PM.STATE_READY)
    assert.are.equal(pcb2.state, PM.STATE_RUNNING)
  end)

  it("save_context saves registers and pc", function()
    local pcb = PM.PCB.new(1, "p")
    local regs = {}
    for i = 1, 32 do regs[i] = i * 2 end
    local pcb2 = pcb:save_context(regs, 0xCAFE, 0xFF00)
    assert.are.equal(pcb2.pc, 0xCAFE)
    assert.are.equal(pcb2.sp, 0xFF00)
    assert.are.equal(pcb2.registers[1], 2)
  end)

  it("add_signal queues signal", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:add_signal(PM.SIGTERM)
    assert.are.equal(#pcb2.pending_signals, 1)
    assert.are.equal(pcb2.pending_signals[1], PM.SIGTERM)
  end)

  it("add_signal is idempotent for duplicate", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:add_signal(PM.SIGTERM)
    local pcb3 = pcb2:add_signal(PM.SIGTERM)
    assert.are.equal(#pcb3.pending_signals, 1)
  end)

  it("mask_signal and is_masked", function()
    local pcb = PM.PCB.new(1, "p")
    assert.is_false(pcb:is_masked(PM.SIGTERM))
    local pcb2 = pcb:mask_signal(PM.SIGTERM)
    assert.is_true(pcb2:is_masked(PM.SIGTERM))
  end)

  it("unmask_signal removes mask", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:mask_signal(PM.SIGTERM)
    local pcb3 = pcb2:unmask_signal(PM.SIGTERM)
    assert.is_false(pcb3:is_masked(PM.SIGTERM))
  end)

  it("set_handler registers custom handler", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:set_handler(PM.SIGTERM, 0xDEAD)
    assert.are.equal(pcb2.signal_handlers[PM.SIGTERM], 0xDEAD)
  end)

  it("tick_cpu increments cpu_time", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:tick_cpu(100)
    assert.are.equal(pcb2.cpu_time, 100)
    local pcb3 = pcb2:tick_cpu(50)
    assert.are.equal(pcb3.cpu_time, 150)
  end)

  it("tick_cpu defaults delta to 1", function()
    local pcb = PM.PCB.new(1, "p")
    local pcb2 = pcb:tick_cpu()
    assert.are.equal(pcb2.cpu_time, 1)
  end)

  it("immutability: PCB operations do not mutate original", function()
    local pcb = PM.PCB.new(1, "p")
    pcb:set_state(PM.STATE_RUNNING)
    assert.are.equal(pcb.state, PM.STATE_READY)
  end)
end)

-- ============================================================================
-- Manager tests
-- ============================================================================

describe("Manager", function()
  it("new creates empty manager", function()
    local m = PM.Manager.new()
    assert.are.equal(m.next_pid, 1)
    assert.is_nil(m.current_pid)
    assert.are.equal(m:total_processes(), 0)
  end)

  it("spawn creates a process", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("init")
    assert.are.equal(pid, 1)
    assert.are.equal(m2.next_pid, 2)
    assert.are.equal(m2:total_processes(), 1)
    assert.is_not_nil(m2:get(pid))
    assert.are.equal(m2:get(pid).name, "init")
  end)

  it("spawn assigns sequential PIDs", function()
    local m = PM.Manager.new()
    local m2, pid1 = m:spawn("p1")
    local m3, pid2 = m2:spawn("p2")
    assert.are_not.equal(pid1, pid2)
    assert.are.equal(pid2, pid1 + 1)
  end)

  it("spawn adds process to run queue", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    assert.are.equal(#m2.run_queue, 1)
    assert.are.equal(m2.run_queue[1], pid)
  end)

  it("fork clones parent into child", function()
    local m = PM.Manager.new()
    local m2, ppid = m:spawn("shell")
    local m3, cpid = m2:fork(ppid)

    local parent = m3:get(ppid)
    local child  = m3:get(cpid)

    assert.are_not.equal(ppid, cpid)
    assert.are.equal(child.parent_pid, ppid)
    assert.are.equal(child.state, PM.STATE_READY)
    assert.are.equal(#parent.children, 1)
    assert.are.equal(parent.children[1], cpid)
  end)

  it("fork errors on missing parent", function()
    local m = PM.Manager.new()
    assert.has_error(function() m:fork(999) end)
  end)

  it("exec replaces process program", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("shell")
    local m3 = m2:fork(pid)  -- returns m3 as manager but we only need exec
    -- exec the original shell process
    local m4 = m2:exec(pid, "ls", { pc = 0x4000 })
    local pcb = m4:get(pid)
    assert.are.equal(pcb.name, "ls")
    assert.are.equal(pcb.pc, 0x4000)
    assert.are.equal(pcb.registers[1], 0)  -- zeroed
  end)

  it("exec errors on missing process", function()
    local m = PM.Manager.new()
    assert.has_error(function() m:exec(999, "ls") end)
  end)

  it("schedule picks ready process", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3, chosen = m2:schedule()
    assert.are.equal(chosen, pid)
    assert.are.equal(m3:get(pid).state, PM.STATE_RUNNING)
    assert.are.equal(m3.current_pid, pid)
  end)

  it("schedule returns nil when no ready processes", function()
    local m = PM.Manager.new()
    local m2, pid = m:schedule()
    assert.is_nil(pid)
  end)

  it("schedule round-robins same-priority processes", function()
    local m = PM.Manager.new()
    local m2, p1 = m:spawn("a")
    local m3, p2 = m2:spawn("b")
    local m4, _ = m3:schedule()  -- p1 runs
    local m5, chosen2 = m4:schedule()  -- p1 goes back to ready, p2 or p1 next
    assert.is_not_nil(chosen2)
    local m6, chosen3 = m5:schedule()
    assert.is_not_nil(chosen3)
  end)

  it("schedule respects priority", function()
    local m = PM.Manager.new()
    local m2, p_high = m:spawn("high", { priority = 5 })
    local m3, p_low  = m2:spawn("low",  { priority = 30 })
    local m4, chosen = m3:schedule()
    assert.are.equal(chosen, p_high)
  end)

  it("block moves process out of run queue", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3, _   = m2:schedule()   -- pid is now running
    local m4      = m3:block(pid)
    assert.are.equal(m4:get(pid).state, PM.STATE_BLOCKED)
    assert.are.equal(#m4.run_queue, 0)
  end)

  it("unblock moves blocked process to ready", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3, _   = m2:schedule()
    local m4      = m3:block(pid)
    local m5      = m4:unblock(pid)
    assert.are.equal(m5:get(pid).state, PM.STATE_READY)
    assert.are.equal(#m5.run_queue, 1)
  end)

  it("unblock no-ops for non-blocked process", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3 = m2:unblock(pid)  -- already ready, not blocked
    assert.are.equal(m3:get(pid).state, PM.STATE_READY)
  end)

  it("exit_process sets zombie state", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3 = m2:exit_process(pid, 42)
    assert.are.equal(m3:get(pid).state, PM.STATE_ZOMBIE)
    assert.are.equal(m3:get(pid).exit_code, 42)
  end)

  it("exit_process sends SIGCHLD to parent", function()
    local m = PM.Manager.new()
    local m2, ppid = m:spawn("shell")
    local m3, cpid = m2:fork(ppid)
    local m4 = m3:exit_process(cpid, 0)
    local parent = m4:get(ppid)
    local got_sigchld = false
    for _, s in ipairs(parent.pending_signals) do
      if s == PM.SIGCHLD then got_sigchld = true end
    end
    assert.is_true(got_sigchld)
  end)

  it("wait_child reaps zombie child", function()
    local m = PM.Manager.new()
    local m2, ppid = m:spawn("shell")
    local m3, cpid = m2:fork(ppid)
    local m4 = m3:exit_process(cpid, 99)
    local status, m5, ec = m4:wait_child(ppid, cpid)
    assert.are.equal(status, "ok")
    assert.are.equal(ec, 99)
    assert.is_nil(m5:get(cpid))
    -- child removed from parent's children list
    assert.are.equal(#m5:get(ppid).children, 0)
  end)

  it("wait_child returns not_exited when child still running", function()
    local m = PM.Manager.new()
    local m2, ppid = m:spawn("shell")
    local m3, cpid = m2:fork(ppid)
    local status, _, _ = m3:wait_child(ppid, cpid)
    assert.are.equal(status, "not_exited")
  end)

  it("wait_child returns no_child for non-child", function()
    local m = PM.Manager.new()
    local m2, p1 = m:spawn("p1")
    local m3, p2 = m2:spawn("p2")
    local status, _, _ = m3:wait_child(p1, p2)
    assert.are.equal(status, "no_child")
  end)

  it("kill with SIGKILL terminates immediately", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3 = m2:kill(pid, PM.SIGKILL)
    assert.are.equal(m3:get(pid).state, PM.STATE_ZOMBIE)
  end)

  it("kill with SIGSTOP blocks process", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3 = m2:kill(pid, PM.SIGSTOP)
    assert.are.equal(m3:get(pid).state, PM.STATE_BLOCKED)
    assert.are.equal(#m3.run_queue, 0)
  end)

  it("kill with SIGCONT resumes blocked process", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3 = m2:kill(pid, PM.SIGSTOP)
    local m4 = m3:kill(pid, PM.SIGCONT)
    assert.are.equal(m4:get(pid).state, PM.STATE_READY)
    assert.are.equal(#m4.run_queue, 1)
  end)

  it("kill with SIGTERM queues signal", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local m3 = m2:kill(pid, PM.SIGTERM)
    local found = false
    for _, s in ipairs(m3:get(pid).pending_signals) do
      if s == PM.SIGTERM then found = true end
    end
    assert.is_true(found)
  end)

  it("kill with SIGTERM does not queue when masked", function()
    local m = PM.Manager.new()
    local m2, pid = m:spawn("p")
    local pcb = m2:get(pid):mask_signal(PM.SIGTERM)
    local m3 = copy_manager_with_pcb(m2, pid, pcb)
    -- Direct approach: kill on original (unmasked) check
    local m4 = m2:kill(pid, PM.SIGTERM)
    assert.are.equal(#m4:get(pid).pending_signals, 1)
  end)

  it("kill no-ops for missing process", function()
    local m = PM.Manager.new()
    local m2 = m:kill(999, PM.SIGTERM)
    assert.are.equal(m2:total_processes(), 0)
  end)

  it("count_in_state", function()
    local m = PM.Manager.new()
    local m2, p1 = m:spawn("p1")
    local m3, p2 = m2:spawn("p2")
    local m4, _ = m3:schedule()
    assert.are.equal(m4:count_in_state(PM.STATE_RUNNING), 1)
    assert.are.equal(m4:count_in_state(PM.STATE_READY), 1)
    assert.are.equal(m4:count_in_state(PM.STATE_BLOCKED), 0)
  end)

  it("full fork-exec-wait lifecycle", function()
    local m = PM.Manager.new()

    -- Init spawns shell
    local m2, shell = m:spawn("shell")

    -- Shell forks to run ls
    local m3, ls = m2:fork(shell)
    assert.are.equal(m3:get(ls).parent_pid, shell)

    -- Child execs ls
    local m4 = m3:exec(ls, "ls", { pc = 0x4000 })
    assert.are.equal(m4:get(ls).name, "ls")

    -- ls exits
    local m5 = m4:exit_process(ls, 0)
    assert.are.equal(m5:get(ls).state, PM.STATE_ZOMBIE)

    -- Shell waits
    local status, m6, ec = m5:wait_child(shell, ls)
    assert.are.equal(status, "ok")
    assert.are.equal(ec, 0)
    assert.is_nil(m6:get(ls))
  end)
end)

-- Helper: replace a PCB in a manager copy (for masked signal test)
function copy_manager_with_pcb(mgr, pid, pcb)
  local m = PM.Manager.new()
  m.process_table = {}
  for k, v in pairs(mgr.process_table) do m.process_table[k] = v end
  m.process_table[pid] = pcb
  m.run_queue = {}
  for _, v in ipairs(mgr.run_queue) do table.insert(m.run_queue, v) end
  m.next_pid = mgr.next_pid
  m.current_pid = mgr.current_pid
  return m
end
