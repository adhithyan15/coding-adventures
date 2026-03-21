# Interrupt Handler (Python)

The interrupt handler manages the full interrupt lifecycle for the
coding-adventures simulated computer: the Interrupt Descriptor Table (IDT),
Interrupt Service Routines (ISRs), the interrupt controller, and save/restore
of CPU context.

## Layer Position

```
S03 — Interrupt Handler
  Depends on: nothing (standalone package)
  Used by:    S04 Kernel, S06 SystemBoard
```

## Components

- **IDT:** 256 entries mapping interrupt numbers to ISR addresses (2 KB at 0x00000000)
- **ISR Registry:** Maps interrupt numbers to Python handler callables
- **Interrupt Controller:** Pending queue, mask register, global enable/disable, priority
- **Interrupt Frame:** Save/restore 32 registers + PC + MStatus + MCause

## Usage

```python
from interrupt_handler import (
    InterruptController, IDTEntry, save_context, restore_context,
    INT_TIMER, InterruptFrame,
)

ic = InterruptController()

# Install timer handler
ic.idt.set_entry(INT_TIMER, IDTEntry(isr_address=0x20100, present=True))
ic.registry.register(INT_TIMER, lambda frame, kernel: print("tick"))

# Raise and dispatch
ic.raise_interrupt(INT_TIMER)
if ic.has_pending():
    num = ic.next_pending()
    frame = save_context(cpu_regs, cpu_pc, cpu_mstatus, num)
    ic.disable()
    ic.registry.dispatch(num, frame, kernel)
    ic.acknowledge(num)
    regs, pc, mstatus = restore_context(frame)
    ic.enable()
```

## Testing

```bash
pip install -e ".[dev]"
pytest tests/ -v
```
