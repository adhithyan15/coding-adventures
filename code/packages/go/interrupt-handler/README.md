# Interrupt Handler (Go)

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

## What It Does

Without interrupts, a CPU can only execute instructions sequentially. The
interrupt handler enables the CPU to respond to external events (keyboard
presses, timer ticks), provide system services (syscalls), and handle errors
(division by zero).

### Components

- **IDT (Interrupt Descriptor Table):** 256 entries mapping interrupt numbers
  to ISR addresses. Lives at address 0x00000000 in memory (2 KB).
- **ISR Registry:** Maps interrupt numbers to Go handler functions.
- **Interrupt Controller:** Manages the pending queue, mask register, global
  enable/disable, and priority dispatch.
- **Interrupt Frame:** Saves/restores all 32 CPU registers + PC + MStatus +
  MCause for clean interrupt entry/exit.

### Interrupt Numbers

| Number | Name              | Source   |
|--------|-------------------|----------|
| 0      | Division by Zero  | CPU      |
| 1      | Debug Exception   | CPU      |
| 2      | NMI               | Hardware |
| 3      | Breakpoint        | CPU      |
| 5      | Invalid Opcode    | CPU      |
| 32     | Timer             | Timer    |
| 33     | Keyboard          | Keyboard |
| 128    | System Call       | Software |

## Usage

```go
import ih "github.com/adhithyan15/coding-adventures/code/packages/go/interrupt-handler"

// Create controller
ic := ih.NewInterruptController()

// Install a timer handler
ic.IDT.SetEntry(ih.IntTimer, ih.IDTEntry{
    ISRAddress: 0x00020100,
    Present:    true,
})
ic.Registry.Register(ih.IntTimer, func(frame *ih.InterruptFrame, kernel interface{}) {
    // Handle timer tick
})

// Raise interrupt (from hardware device)
ic.RaiseInterrupt(ih.IntTimer)

// Check and dispatch (from pipeline)
if ic.HasPending() {
    num := ic.NextPending()
    frame := ih.SaveContext(cpuRegs, cpuPC, cpuMStatus, uint32(num))
    ic.Disable()
    ic.Registry.Dispatch(num, &frame, kernel)
    ic.Acknowledge(num)
    regs, pc, mstatus := ih.RestoreContext(frame)
    ic.Enable()
}
```

## Testing

```bash
go test ./... -v -cover
```
