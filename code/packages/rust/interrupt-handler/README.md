# Interrupt Handler (Rust)

The interrupt handler manages the full interrupt lifecycle for the
coding-adventures simulated computer: IDT, ISR registry, interrupt controller,
and CPU context save/restore.

## Layer Position

```
S03 — Interrupt Handler
  Depends on: nothing (standalone crate)
  Used by:    S04 Kernel, S06 SystemBoard
```

## Components

- **IDT:** 256 entries mapping interrupt numbers to ISR addresses
- **ISR Registry:** Maps interrupt numbers to boxed closure handlers
- **Interrupt Controller:** Pending queue, mask register, global enable/disable
- **Interrupt Frame:** Save/restore 32 registers + PC + MStatus + MCause

## Usage

```rust
use interrupt_handler::*;

let mut ic = InterruptController::new();

// Install timer handler
ic.idt.set_entry(INT_TIMER, IDTEntry {
    isr_address: 0x20100,
    present: true,
    privilege_level: 0,
});
ic.registry.register(INT_TIMER, Box::new(|frame| {
    println!("tick: mcause={}", frame.mcause);
}));

// Raise and dispatch
ic.raise_interrupt(INT_TIMER);
if ic.has_pending() {
    let num = ic.next_pending().unwrap();
    let mut frame = save_context(cpu_regs, cpu_pc, cpu_mstatus, num as u32);
    ic.disable();
    ic.registry.dispatch(num, &mut frame);
    ic.acknowledge(num);
    let (regs, pc, mstatus) = restore_context(&frame);
    ic.enable();
}
```

## Testing

```bash
cargo test -p interrupt-handler
```
