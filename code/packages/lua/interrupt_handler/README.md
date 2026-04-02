# interrupt_handler (Lua)

Hardware interrupt controller and handler for the coding-adventures simulated computer.

## What It Does

Implements the complete interrupt lifecycle:

- **IDT** (Interrupt Descriptor Table) — maps interrupt numbers 0..255 to ISR addresses
- **ISRRegistry** — maps interrupt numbers to Lua handler functions
- **Controller** — pending queue, mask register, global enable/disable, priority dispatch
- **Frame** — saved CPU context (registers, PC, status) at the moment of interrupt

## Why Interrupts Matter

Without interrupts, a CPU executes instructions sequentially forever. It cannot respond to a keystroke, a timer tick, or a disk completion. Interrupts are the mechanism that transforms a calculator into a computer capable of multitasking.

## Usage

```lua
local IH = require("coding_adventures.interrupt_handler")

-- Create the interrupt controller
local ctrl = IH.Controller.new()

-- Register a timer ISR (interrupt 32)
ctrl = ctrl:register(32, function(frame, kernel)
  kernel.ticks = kernel.ticks + 1
  return kernel
end)

-- Hardware raises interrupt 32
ctrl = ctrl:raise(32)

-- Dispatch the interrupt
local frame  = IH.Frame.new(0x1000, {}, 0, 32)
local kernel = { ticks = 0 }
ctrl, kernel = ctrl:dispatch(frame, kernel)
-- kernel.ticks == 1
```

## Interrupt Numbers

| Range   | Type                  | Examples                        |
|---------|-----------------------|---------------------------------|
| 0..31   | CPU exceptions        | Divide-by-zero (0), page fault  |
| 32..47  | Hardware IRQs         | Timer (32), Keyboard (33)       |
| 48..255 | Software interrupts   | System calls, user-defined      |

## Stack Position

```
D05 Core (CPU registers, program counter)
    │
    └── S03 Interrupt Handler  ← this package
          │
          └── S04 OS Kernel (registers ISRs for timer, keyboard, syscall)
```
