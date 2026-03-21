# Interrupt Handler (Ruby)

The interrupt handler manages the full interrupt lifecycle for the
coding-adventures simulated computer: IDT, ISR registry, interrupt controller,
and CPU context save/restore.

## Layer Position

```
S03 — Interrupt Handler
  Depends on: nothing (standalone package)
  Used by:    S04 Kernel, S06 SystemBoard
```

## Components

- **IDT:** 256 entries mapping interrupt numbers to ISR addresses
- **ISR Registry:** Maps interrupt numbers to Ruby lambdas/procs
- **Interrupt Controller:** Pending queue, mask register, global enable/disable
- **Interrupt Frame:** Save/restore 32 registers + PC + MStatus + MCause

## Usage

```ruby
require "coding_adventures/interrupt_handler"

ic = CodingAdventures::InterruptHandler::InterruptController.new

# Install timer handler
ic.idt.set_entry(32, CodingAdventures::InterruptHandler::IDTEntry.new(
  isr_address: 0x20100, present: true
))
ic.registry.register(32, ->(frame, kernel) { puts "tick" })

# Raise and dispatch
ic.raise_interrupt(32)
if ic.has_pending?
  num = ic.next_pending
  frame = CodingAdventures::InterruptHandler.save_context(regs, pc, mstatus, num)
  ic.disable
  ic.registry.dispatch(num, frame, kernel)
  ic.acknowledge(num)
  regs, pc, mstatus = CodingAdventures::InterruptHandler.restore_context(frame)
  ic.enable
end
```

## Testing

```bash
bundle install
bundle exec rake test
```
