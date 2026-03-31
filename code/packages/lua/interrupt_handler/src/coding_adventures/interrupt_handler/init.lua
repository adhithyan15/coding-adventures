-- ============================================================================
-- interrupt_handler — Hardware Interrupt Controller and Handler
-- ============================================================================
--
-- Without interrupts, a CPU can only execute instructions sequentially.
-- It cannot respond to external events (a keystroke, a timer tick), cannot
-- multitask, and cannot provide system services.
--
-- **Interrupts** transform a calculator into a computer.  They are signals
-- that say "stop what you are doing, handle this event, then resume."
--
-- ## Analogy
--
-- Think of cooking while waiting for a phone call.  You are focused on
-- your recipe (the main program).  When the phone rings (interrupt), you:
--   1. Put down your spoon and note what step you are on (save context).
--   2. Answer the phone and handle the call (Interrupt Service Routine).
--   3. Hang up and return to the exact step you were on (restore context).
--
-- If you did not save your place, you would return and wonder "did I already
-- add the salt?"  The CPU saves its entire register state for the same reason.
--
-- ## Three Types of Interrupts
--
--   +----------+----------+--------------------------------------------+
--   | Type     | Trigger  | Examples                                   |
--   +----------+----------+--------------------------------------------+
--   | Hardware | External | Timer tick, keyboard, disk I/O complete    |
--   | Software | Trap     | System call (ecall), debug breakpoint       |
--   | Exception| CPU err  | Division by zero, invalid opcode, page fault|
--   +----------+----------+--------------------------------------------+
--
-- All three types use the same mechanism:
--   1. Look up the handler in the Interrupt Descriptor Table (IDT)
--   2. Save CPU registers into an Interrupt Frame
--   3. Call the Interrupt Service Routine (ISR)
--   4. Restore registers from the frame
--
-- ## Module Structure
--
--   interrupt_handler
--   ├── IDT           — maps interrupt numbers (0..255) to ISR addresses
--   ├── ISRRegistry   — maps interrupt numbers to handler functions
--   ├── Controller    — pending queue, mask register, enable/disable
--   └── Frame         — saved CPU registers at time of interrupt
--
-- ## Usage
--
--   local IH = require("coding_adventures.interrupt_handler")
--
--   -- Build the controller
--   local ctrl = IH.Controller.new()
--
--   -- Register a handler for interrupt 32 (timer)
--   ctrl = IH.Controller.register(ctrl, 32, function(frame, kernel)
--     kernel.ticks = kernel.ticks + 1
--     return kernel
--   end)
--
--   -- Raise the interrupt
--   ctrl = IH.Controller.raise(ctrl, 32)
--
--   -- Dispatch it
--   local kernel = { ticks = 0 }
--   local frame  = IH.Frame.new(0x1000, {}, 0, 32)
--   ctrl, kernel = IH.Controller.dispatch(ctrl, frame, kernel)
--
-- ============================================================================

local M = {}

-- ============================================================================
-- IDT — Interrupt Descriptor Table
-- ============================================================================
--
-- The IDT is an array of 256 entries stored at address 0x00000000 during boot.
-- Each entry maps an interrupt number (0..255) to the address of its handler
-- (the ISR).  The BIOS populates initial entries; the kernel may modify them.
--
-- Layout:
--
--   IDT[0]  = { isr_address=0x1000, present=true,  privilege_level=0 }  ← Divide-by-zero
--   IDT[1]  = { isr_address=0x1010, present=true,  privilege_level=0 }  ← Debug
--   IDT[2]  = { isr_address=0x0000, present=false, privilege_level=0 }  ← (unused)
--   ...
--   IDT[32] = { isr_address=0x2000, present=true,  privilege_level=0 }  ← Timer
--   IDT[33] = { isr_address=0x2010, present=true,  privilege_level=0 }  ← Keyboard
--
-- Reserved interrupt numbers (Intel x86 convention):
--   0..31   CPU exceptions (divide by zero, page fault, etc.)
--   32..47  Hardware IRQs (timer=32, keyboard=33, disk=34, …)
--   48..255 Software interrupts (system calls, etc.)

M.IDT = {}
M.IDT.__index = M.IDT

--- Create a new, empty IDT (all 256 entries absent/not-present).
function M.IDT.new()
  return setmetatable({ entries = {} }, M.IDT)
end

--- Set an IDT entry.
-- @param number  Interrupt number (0..255)
-- @param entry   Table with fields: isr_address, present, privilege_level
function M.IDT:set_entry(number, entry)
  if number < 0 or number > 255 then
    error("IDT entry number must be 0..255, got " .. tostring(number))
  end
  local idt = M.IDT.new()
  for k, v in pairs(self.entries) do idt.entries[k] = v end
  idt.entries[number] = {
    isr_address     = entry.isr_address     or 0,
    present         = entry.present         ~= false,  -- default true
    privilege_level = entry.privilege_level or 0,
  }
  return idt
end

--- Get an IDT entry.  Returns a default "not present" entry if unset.
-- @param number  Interrupt number (0..255)
function M.IDT:get_entry(number)
  if number < 0 or number > 255 then
    error("IDT entry number must be 0..255, got " .. tostring(number))
  end
  return self.entries[number] or { isr_address = 0, present = false, privilege_level = 0 }
end

-- ============================================================================
-- ISRRegistry — Interrupt Service Routine Registry
-- ============================================================================
--
-- While the IDT stores hardware addresses (for the real CPU to jump to), the
-- ISRRegistry maps interrupt numbers to Lua callback functions.  This is the
-- software-simulation equivalent.
--
-- When an interrupt fires:
--   handler(frame, kernel) → new_kernel
--
-- The handler receives:
--   - frame:  the saved CPU state at the time the interrupt occurred
--   - kernel: the OS kernel state (process table, memory, I/O buffers)
--
-- It returns the updated kernel state.

M.ISRRegistry = {}
M.ISRRegistry.__index = M.ISRRegistry

--- Create a new, empty ISR registry.
function M.ISRRegistry.new()
  return setmetatable({ handlers = {} }, M.ISRRegistry)
end

--- Register a handler function for an interrupt number.
-- @param number   Interrupt number (0..255)
-- @param handler  function(frame, kernel) → kernel
function M.ISRRegistry:register(number, handler)
  local r = M.ISRRegistry.new()
  for k, v in pairs(self.handlers) do r.handlers[k] = v end
  r.handlers[number] = handler
  return r
end

--- Dispatch: call the handler for the given interrupt number.
-- Raises an error if no handler is registered.
-- @param number   Interrupt number
-- @param frame    The current interrupt frame
-- @param kernel   The current kernel state
-- @return new_kernel
function M.ISRRegistry:dispatch(number, frame, kernel)
  local handler = self.handlers[number]
  if not handler then
    error("no ISR handler registered for interrupt " .. tostring(number))
  end
  return handler(frame, kernel)
end

--- Check whether a handler is registered for the given interrupt number.
function M.ISRRegistry:has_handler(number)
  return self.handlers[number] ~= nil
end

-- ============================================================================
-- Controller — Interrupt Controller
-- ============================================================================
--
-- The interrupt controller (PIC — Programmable Interrupt Controller, or APIC
-- on modern systems) sits between the hardware and the CPU.  It:
--
--   1. Receives interrupt signals from devices.
--   2. Queues them (pending list) if the CPU is busy.
--   3. Masks individual IRQs (mask_register bitmask, bits 0..31).
--   4. Respects a global enable/disable flag (cli/sti in x86).
--   5. Dispatches one interrupt at a time in priority order (lowest number
--      first — lower number = higher priority).
--
-- ## Mask Register
--
-- The mask register is a 32-bit integer where each bit corresponds to one
-- interrupt number.  If bit N is 1, interrupt N is masked (suppressed).
--
--   mask = 0b00000000_00000000_00000000_00000001   ← interrupt 0 masked
--   mask = 0b00000000_00000000_00000000_00000011   ← 0 and 1 masked
--
-- ## Pending Queue
--
-- Interrupts are queued in order of priority (lowest IRQ number first).
-- The next_pending() function returns the highest-priority unmasked interrupt.
--
-- ## Example Flow
--
--   ctrl = Controller.new()
--   ctrl = Controller.register(ctrl, 32, timer_isr)
--   ctrl = Controller.raise(ctrl, 32)           -- timer fires
--   ctrl = Controller.raise(ctrl, 33)           -- keyboard fires while handling timer
--   -- Controller processes 32 first (lower number = higher priority)
--   -- then 33

M.Controller = {}
M.Controller.__index = M.Controller

--- Create a new Controller with empty state.
function M.Controller.new()
  local c = setmetatable({
    idt           = M.IDT.new(),
    registry      = M.ISRRegistry.new(),
    pending       = {},   -- list of pending interrupt numbers, sorted
    mask_register = 0,    -- 32-bit bitmask of masked interrupt lines
    enabled       = true, -- global interrupt enable flag (cli/sti)
  }, M.Controller)
  return c
end

-- Internal: copy a controller (immutable-style updates)
local function copy_ctrl(c)
  local new_pending = {}
  for i, v in ipairs(c.pending) do new_pending[i] = v end
  return setmetatable({
    idt           = c.idt,
    registry      = c.registry,
    pending       = new_pending,
    mask_register = c.mask_register,
    enabled       = c.enabled,
  }, M.Controller)
end

--- Register an ISR handler for an interrupt number.
-- This is a convenience wrapper around ISRRegistry.register.
function M.Controller:register(number, handler)
  local c = copy_ctrl(self)
  c.registry = c.registry:register(number, handler)
  return c
end

--- Raise (signal) an interrupt.  Adds it to the pending queue if not already
-- present.  Interrupts are kept sorted by number (lower = higher priority).
-- @param number  Interrupt number to raise
function M.Controller:raise(number)
  -- Check if already pending
  for _, v in ipairs(self.pending) do
    if v == number then return self end
  end
  local c = copy_ctrl(self)
  table.insert(c.pending, number)
  table.sort(c.pending)
  return c
end

--- Check whether there are any unmasked, enabled interrupts pending.
function M.Controller:has_pending()
  if not self.enabled then return false end
  for _, irq in ipairs(self.pending) do
    if not self:is_masked(irq) then return true end
  end
  return false
end

--- Return the highest-priority pending interrupt that is unmasked and enabled.
-- Returns -1 if none.
function M.Controller:next_pending()
  if not self.enabled then return -1 end
  for _, irq in ipairs(self.pending) do
    if not self:is_masked(irq) then return irq end
  end
  return -1
end

--- Acknowledge (remove) an interrupt from the pending queue after handling.
function M.Controller:acknowledge(number)
  local c = copy_ctrl(self)
  local new_pending = {}
  for _, v in ipairs(c.pending) do
    if v ~= number then table.insert(new_pending, v) end
  end
  c.pending = new_pending
  return c
end

--- Mask or unmask a specific interrupt line.
-- @param number  Interrupt number (0..31 for maskable IRQs)
-- @param masked  true to mask (suppress), false to unmask
function M.Controller:set_mask(number, masked)
  if number < 0 or number > 31 then return self end
  local c = copy_ctrl(self)
  if masked then
    c.mask_register = c.mask_register | (1 << number)
  else
    c.mask_register = c.mask_register & ~(1 << number)
  end
  return c
end

--- Check whether interrupt number is currently masked.
function M.Controller:is_masked(number)
  if number < 0 or number > 31 then return false end
  return (self.mask_register & (1 << number)) ~= 0
end

--- Enable all interrupts globally (like x86 `sti` — Set Interrupt Flag).
function M.Controller:enable()
  local c = copy_ctrl(self)
  c.enabled = true
  return c
end

--- Disable all interrupts globally (like x86 `cli` — Clear Interrupt Flag).
function M.Controller:disable()
  local c = copy_ctrl(self)
  c.enabled = false
  return c
end

--- Return the number of pending (unacknowledged) interrupts.
function M.Controller:pending_count()
  return #self.pending
end

--- Clear all pending interrupts (used during system reset).
function M.Controller:clear_all()
  local c = copy_ctrl(self)
  c.pending = {}
  return c
end

--- Dispatch the highest-priority pending interrupt.
-- Saves context into a Frame, calls the ISR, then returns updated controller
-- and kernel state.
-- @param frame   Interrupt frame (saved CPU context)
-- @param kernel  Current kernel state passed to the ISR
-- @return new_ctrl, new_kernel
function M.Controller:dispatch(frame, kernel)
  local irq = self:next_pending()
  if irq == -1 then
    return self, kernel
  end
  local new_ctrl = self:acknowledge(irq)
  local new_kernel = new_ctrl.registry:dispatch(irq, frame, kernel)
  return new_ctrl, new_kernel
end

-- ============================================================================
-- Frame — Interrupt Frame (Saved CPU Context)
-- ============================================================================
--
-- When an interrupt fires, the CPU must save its current state so it can
-- resume after the ISR returns.  The Frame struct holds this snapshot.
--
-- ## What Gets Saved
--
--   +--------------------+--------------------------------------------------+
--   | Field              | Contents                                         |
--   +--------------------+--------------------------------------------------+
--   | pc                 | Program counter — where to resume after the ISR  |
--   | registers          | Table of register name → value                   |
--   | mstatus            | Machine status register (RISC-V) / EFLAGS (x86) |
--   | mcause             | Machine cause register — which interrupt fired   |
--   +--------------------+--------------------------------------------------+
--
-- ## Why Save Registers?
--
-- The ISR is ordinary code.  It uses CPU registers for its own computation.
-- If we did not save the interrupted program's registers, the ISR would
-- overwrite them.  When the interrupted program resumed, its variables would
-- contain garbage.
--
-- This is the hardware equivalent of a function call: caller-save registers
-- must be preserved across the call.  The interrupt frame IS the call frame.

M.Frame = {}
M.Frame.__index = M.Frame

--- Create a new interrupt frame.
-- @param pc         Program counter at time of interrupt
-- @param registers  Table of register values at time of interrupt
-- @param mstatus    Machine status register value
-- @param mcause     Machine cause register value (interrupt number)
function M.Frame.new(pc, registers, mstatus, mcause)
  return setmetatable({
    pc        = pc        or 0,
    registers = registers or {},
    mstatus   = mstatus   or 0,
    mcause    = mcause    or 0,
  }, M.Frame)
end

--- Save context: create an interrupt frame from current CPU state.
function M.Frame.save_context(registers, pc, mstatus, mcause)
  return M.Frame.new(pc, registers, mstatus, mcause)
end

--- Restore context: extract saved state from the frame.
-- @return registers, pc, mstatus
function M.Frame:restore_context()
  return self.registers, self.pc, self.mstatus
end

return M
