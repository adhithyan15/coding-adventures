-- Tests for coding_adventures.interrupt_handler
-- Coverage target: 95%+

local IH = require("coding_adventures.interrupt_handler")

-- ============================================================================
-- IDT tests
-- ============================================================================

describe("IDT", function()
  it("creates empty IDT", function()
    local idt = IH.IDT.new()
    assert.are.equal(type(idt), "table")
  end)

  it("set_entry and get_entry round-trip", function()
    local idt = IH.IDT.new()
    idt = idt:set_entry(32, { isr_address = 0x2000, present = true, privilege_level = 0 })
    local e = idt:get_entry(32)
    assert.are.equal(e.isr_address, 0x2000)
    assert.is_true(e.present)
    assert.are.equal(e.privilege_level, 0)
  end)

  it("get_entry returns default for unset interrupt", function()
    local idt = IH.IDT.new()
    local e = idt:get_entry(5)
    assert.are.equal(e.isr_address, 0)
    assert.is_false(e.present)
    assert.are.equal(e.privilege_level, 0)
  end)

  it("set_entry is immutable — original unchanged", function()
    local idt1 = IH.IDT.new()
    local idt2 = idt1:set_entry(10, { isr_address = 0x5000, present = true })
    assert.are.equal(idt1:get_entry(10).present, false)
    assert.are.equal(idt2:get_entry(10).isr_address, 0x5000)
  end)

  it("set_entry rejects out-of-range number", function()
    local idt = IH.IDT.new()
    assert.has_error(function() idt:set_entry(256, { isr_address = 0 }) end)
    assert.has_error(function() idt:set_entry(-1, { isr_address = 0 }) end)
  end)

  it("get_entry rejects out-of-range number", function()
    local idt = IH.IDT.new()
    assert.has_error(function() idt:get_entry(256) end)
  end)

  it("set_entry overwrites existing entry", function()
    local idt = IH.IDT.new()
    idt = idt:set_entry(0, { isr_address = 0x100, present = true })
    idt = idt:set_entry(0, { isr_address = 0x200, present = true })
    assert.are.equal(idt:get_entry(0).isr_address, 0x200)
  end)

  it("multiple entries coexist", function()
    local idt = IH.IDT.new()
    for i = 0, 10 do
      idt = idt:set_entry(i, { isr_address = i * 16, present = true })
    end
    for i = 0, 10 do
      assert.are.equal(idt:get_entry(i).isr_address, i * 16)
    end
  end)
end)

-- ============================================================================
-- ISRRegistry tests
-- ============================================================================

describe("ISRRegistry", function()
  it("creates empty registry", function()
    local r = IH.ISRRegistry.new()
    assert.is_false(r:has_handler(0))
  end)

  it("register and has_handler", function()
    local r = IH.ISRRegistry.new()
    r = r:register(32, function() end)
    assert.is_true(r:has_handler(32))
    assert.is_false(r:has_handler(33))
  end)

  it("dispatch calls handler with frame and kernel", function()
    local r = IH.ISRRegistry.new()
    local called_with = {}
    r = r:register(32, function(frame, kernel)
      called_with.frame  = frame
      called_with.kernel = kernel
      return { ticks = kernel.ticks + 1 }
    end)
    local frame  = IH.Frame.new(0x1000, {}, 0, 32)
    local kernel = { ticks = 5 }
    local result = r:dispatch(32, frame, kernel)
    assert.are.equal(result.ticks, 6)
    assert.are.equal(called_with.frame.pc, 0x1000)
  end)

  it("dispatch raises error when no handler registered", function()
    local r = IH.ISRRegistry.new()
    assert.has_error(function()
      r:dispatch(99, IH.Frame.new(0, {}, 0, 99), {})
    end)
  end)

  it("register is immutable", function()
    local r1 = IH.ISRRegistry.new()
    local r2 = r1:register(1, function() end)
    assert.is_false(r1:has_handler(1))
    assert.is_true(r2:has_handler(1))
  end)

  it("overwrite handler with re-register", function()
    local r = IH.ISRRegistry.new()
    r = r:register(5, function(_, k) return { v = k.v + 1 } end)
    r = r:register(5, function(_, k) return { v = k.v + 10 } end)
    local result = r:dispatch(5, IH.Frame.new(0, {}, 0, 5), { v = 0 })
    assert.are.equal(result.v, 10)
  end)
end)

-- ============================================================================
-- Frame tests
-- ============================================================================

describe("Frame", function()
  it("new creates frame with all fields", function()
    local f = IH.Frame.new(0xCAFE, { x1 = 42 }, 0xDEAD, 32)
    assert.are.equal(f.pc, 0xCAFE)
    assert.are.equal(f.registers.x1, 42)
    assert.are.equal(f.mstatus, 0xDEAD)
    assert.are.equal(f.mcause, 32)
  end)

  it("new defaults zeros for missing args", function()
    local f = IH.Frame.new()
    assert.are.equal(f.pc, 0)
    assert.are.equal(f.mstatus, 0)
    assert.are.equal(f.mcause, 0)
  end)

  it("save_context is alias for new", function()
    local regs = { pc = 0, sp = 0xFF }
    local f = IH.Frame.save_context(regs, 0x1000, 0x3, 32)
    assert.are.equal(f.pc, 0x1000)
    assert.are.equal(f.mstatus, 0x3)
    assert.are.equal(f.mcause, 32)
  end)

  it("restore_context returns registers, pc, mstatus", function()
    local regs = { a0 = 7, a1 = 8 }
    local f = IH.Frame.new(0x2000, regs, 0xFF, 33)
    local r, pc, ms = f:restore_context()
    assert.are.equal(pc, 0x2000)
    assert.are.equal(ms, 0xFF)
    assert.are.equal(r.a0, 7)
  end)
end)

-- ============================================================================
-- Controller tests
-- ============================================================================

describe("Controller", function()
  it("new creates controller with defaults", function()
    local c = IH.Controller.new()
    assert.are.equal(c:pending_count(), 0)
    assert.is_false(c:has_pending())
    assert.are.equal(c:next_pending(), -1)
    assert.is_true(c.enabled)
  end)

  it("raise adds interrupt to pending", function()
    local c = IH.Controller.new()
    c = c:raise(32)
    assert.are.equal(c:pending_count(), 1)
  end)

  it("raise is idempotent — duplicate ignored", function()
    local c = IH.Controller.new()
    c = c:raise(32)
    c = c:raise(32)
    assert.are.equal(c:pending_count(), 1)
  end)

  it("raise sorts by priority (lower number = higher priority)", function()
    local c = IH.Controller.new()
    c = c:raise(33)
    c = c:raise(32)
    c = c:raise(35)
    assert.are.equal(c:next_pending(), 32)
  end)

  it("has_pending is true when interrupts queued and enabled", function()
    local c = IH.Controller.new()
    c = c:raise(10)
    assert.is_true(c:has_pending())
  end)

  it("has_pending is false when disabled", function()
    local c = IH.Controller.new()
    c = c:raise(10)
    c = c:disable()
    assert.is_false(c:has_pending())
  end)

  it("next_pending returns -1 when disabled", function()
    local c = IH.Controller.new()
    c = c:raise(5)
    c = c:disable()
    assert.are.equal(c:next_pending(), -1)
  end)

  it("acknowledge removes interrupt from pending", function()
    local c = IH.Controller.new()
    c = c:raise(32):raise(33)
    c = c:acknowledge(32)
    assert.are.equal(c:pending_count(), 1)
    assert.are.equal(c:next_pending(), 33)
  end)

  it("enable/disable toggles global interrupt flag", function()
    local c = IH.Controller.new()
    c = c:disable()
    assert.is_false(c.enabled)
    c = c:enable()
    assert.is_true(c.enabled)
  end)

  it("set_mask masks an IRQ line", function()
    local c = IH.Controller.new()
    c = c:raise(0)
    c = c:set_mask(0, true)
    assert.is_false(c:has_pending())
    assert.are.equal(c:next_pending(), -1)
  end)

  it("set_mask unmask restores IRQ", function()
    local c = IH.Controller.new()
    c = c:raise(1)
    c = c:set_mask(1, true)
    assert.is_false(c:has_pending())
    c = c:set_mask(1, false)
    assert.is_true(c:has_pending())
  end)

  it("is_masked returns true/false correctly", function()
    local c = IH.Controller.new()
    assert.is_false(c:is_masked(5))
    c = c:set_mask(5, true)
    assert.is_true(c:is_masked(5))
  end)

  it("is_masked returns false for numbers > 31", function()
    local c = IH.Controller.new()
    assert.is_false(c:is_masked(32))
    assert.is_false(c:is_masked(100))
  end)

  it("set_mask no-ops for numbers > 31", function()
    local c = IH.Controller.new()
    local c2 = c:set_mask(32, true)
    assert.are.equal(c2.mask_register, c.mask_register)
  end)

  it("clear_all empties pending queue", function()
    local c = IH.Controller.new()
    c = c:raise(10):raise(20):raise(30)
    c = c:clear_all()
    assert.are.equal(c:pending_count(), 0)
    assert.is_false(c:has_pending())
  end)

  it("dispatch calls ISR and acknowledges interrupt", function()
    local c = IH.Controller.new()
    c = c:register(32, function(frame, kernel)
      return { ticks = kernel.ticks + 1 }
    end)
    c = c:raise(32)
    local frame  = IH.Frame.new(0x1000, {}, 0, 32)
    local kernel = { ticks = 0 }
    local c2, k2 = c:dispatch(frame, kernel)
    assert.are.equal(k2.ticks, 1)
    assert.are.equal(c2:pending_count(), 0)
  end)

  it("dispatch returns unchanged state when no pending", function()
    local c = IH.Controller.new()
    local frame  = IH.Frame.new(0, {}, 0, 0)
    local kernel = { x = 99 }
    local c2, k2 = c:dispatch(frame, kernel)
    assert.are.equal(k2.x, 99)
  end)

  it("dispatch handles priority — dispatches lowest number first", function()
    local order = {}
    local c = IH.Controller.new()
    c = c:register(32, function(_, k) table.insert(order, 32); return k end)
    c = c:register(33, function(_, k) table.insert(order, 33); return k end)
    c = c:raise(33):raise(32)
    local frame = IH.Frame.new(0, {}, 0, 0)
    local k = {}
    c, k = c:dispatch(frame, k)
    c, k = c:dispatch(frame, k)
    assert.are.same(order, { 32, 33 })
  end)

  it("nested interrupt scenario: raise during dispatch", function()
    -- While handling IRQ 32, the handler raises IRQ 33
    local handled = {}
    local c = IH.Controller.new()
    c = c:register(32, function(_, k)
      table.insert(handled, 32)
      k.ctrl = k.ctrl:raise(33)
      return k
    end)
    c = c:register(33, function(_, k)
      table.insert(handled, 33)
      return k
    end)
    c = c:raise(32)
    local frame = IH.Frame.new(0, {}, 0, 0)
    local k = { ctrl = c }
    c, k = c:dispatch(frame, k)
    -- Now dispatch the nested 33 from k.ctrl
    c2, k = k.ctrl:dispatch(frame, k)
    assert.are.same(handled, { 32, 33 })
  end)

  it("register convenience wrapper", function()
    local c = IH.Controller.new()
    c = c:register(5, function(_, k) return k end)
    assert.is_true(c.registry:has_handler(5))
  end)

  it("immutability: raise does not mutate original", function()
    local c1 = IH.Controller.new()
    local c2 = c1:raise(10)
    assert.are.equal(c1:pending_count(), 0)
    assert.are.equal(c2:pending_count(), 1)
  end)
end)
