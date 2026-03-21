# S03 — Interrupt Handler

## Overview

The interrupt handler manages the full interrupt lifecycle: the Interrupt
Descriptor Table (IDT), Interrupt Service Routines (ISRs), the interrupt
controller, and the save/restore of CPU context. It extends the D05 shell
`InterruptController` into a complete, working system.

Without interrupts, a CPU can only do one thing: execute instructions
sequentially from top to bottom. It cannot respond to external events
(keyboard presses, timer ticks), cannot multitask, and cannot provide system
services. Interrupts are what transform a calculator into a computer.

**Analogy:** Interrupts are like a phone ringing while you are cooking. You
pause cooking (save context — remember what step you were on, what burner is
on, what you were about to add). You answer the phone (handle the interrupt).
When the call ends, you resume cooking exactly where you left off (restore
context). If you did not save your place, you would come back and wonder
"did I already add the salt?"

## Layer Position

```
Core (D05)
│
├── Pipeline (D04)
│     └── IF stage checks: any pending interrupts?
│
├── Interrupt Handler (S03) ← YOU ARE HERE
│     ├── IDT: maps interrupt numbers → ISR addresses
│     ├── ISR Registry: maps interrupt numbers → Go handler functions
│     ├── Controller: pending queue, mask register, enable flag
│     └── Context: save/restore CPU registers on interrupt
│
├── OS Kernel (S04) ← registers ISRs for timer, keyboard, syscall
│
└── Hardware
      ├── Timer ← raises interrupt 32 every N cycles
      └── Keyboard ← raises interrupt 33 on keystroke
```

**Depends on:** D05 Core (provides CPU registers that must be saved/restored)
**Used by:** S04 Kernel (registers handlers), S06 SystemBoard (wires
interrupts to hardware devices)

## Key Concepts

### What Is an Interrupt?

An interrupt is a signal that tells the CPU: "stop what you are doing and
handle this event." The CPU finishes its current instruction, saves its
state, and jumps to a pre-registered handler function.

There are three types:

```
Type           Trigger              Examples
────────────────────────────────────────────────────────────
Hardware       External device      Timer tick, keyboard press,
(external)     sends signal         disk I/O complete

Software       Program executes     System call (ecall on RISC-V),
(trap)         special instruction  breakpoint

Exception      CPU detects error    Division by zero, invalid
(fault)        during execution     opcode, page fault
```

All three types use the same mechanism: look up the handler in the IDT,
save context, jump to handler, restore context when done.

### Interrupt Descriptor Table (IDT)

The IDT is an array of 256 entries stored at address `0x00000000`. Each entry
maps an interrupt number to the address of its handler (ISR). The BIOS (S01)
populates the IDT during boot; the kernel (S04) may modify entries later.

```
IDT Layout in Memory (at 0x00000000):
┌─────────────────────────────────────────────────────────┐
│ Entry 0:  Division by zero handler                       │ 8 bytes
│ Entry 1:  Debug exception handler                        │ 8 bytes
│ Entry 2:  Non-maskable interrupt handler                 │ 8 bytes
│ ...                                                      │
│ Entry 31: Reserved CPU exceptions                        │ 8 bytes
│ Entry 32: Timer interrupt handler                        │ 8 bytes
│ Entry 33: Keyboard interrupt handler                     │ 8 bytes
│ ...                                                      │
│ Entry 128: System call handler                           │ 8 bytes
│ ...                                                      │
│ Entry 255: (last entry)                                  │ 8 bytes
└─────────────────────────────────────────────────────────┘
Total: 256 entries x 8 bytes = 2,048 bytes (2 KB)

Each IDT Entry (8 bytes):
┌──────────────────┬──────────┬──────────────┬──────────┐
│ ISR Address      │ Present  │ Privilege    │ Reserved │
│ (4 bytes)        │ (1 byte) │ Level(1byte) │ (2 bytes)│
└──────────────────┴──────────┴──────────────┴──────────┘

ISR Address:     Where to jump when this interrupt fires.
Present bit:     1 = valid entry, 0 = unused (triggers double fault if fired).
Privilege Level: 0 = kernel only. User programs cannot invoke this directly.
```

**Why 256 entries?** This matches x86 convention and provides plenty of room.
Entries 0-31 are reserved for CPU exceptions, 32-47 for hardware devices,
and 128 for system calls. Most entries are unused (present = 0).

### Interrupt Numbers

```
Number  Name                    Source        Description
──────────────────────────────────────────────────────────────
0       Division by Zero        CPU           Divide instruction with divisor 0
1       Debug Exception         CPU           Single-step breakpoint
2       NMI                     Hardware      Non-maskable (cannot be blocked)
3       Breakpoint              CPU           ebreak instruction
4       Overflow                CPU           Arithmetic overflow detected
5       Invalid Opcode          CPU           Unrecognized instruction encoding
6-31    (Reserved)              CPU           Future CPU exceptions
32      Timer                   Timer chip    Clock tick — drives scheduler
33      Keyboard                Keyboard      Host keystroke injected
34-47   (Available)             Hardware      Future device interrupts
48-127  (Available)             —             Unassigned
128     System Call             Software      ecall instruction (user → kernel)
129-255 (Available)             —             Unassigned
```

### Interrupt Frame (Saved Context)

When an interrupt fires, the CPU must save **everything** needed to resume
the interrupted code later. This is called the interrupt frame (or trap
frame). It is pushed onto the kernel stack.

```
Interrupt Frame on Stack:
┌─────────────────────────────────────────┐ ← old sp
│ PC (return address)         │ 4 bytes   │
│ MStatus register            │ 4 bytes   │
│ MCause register             │ 4 bytes   │
│ x1  (ra — return address)   │ 4 bytes   │
│ x2  (sp — stack pointer)    │ 4 bytes   │
│ x3  (gp — global pointer)   │ 4 bytes   │
│ ...                         │           │
│ x31 (t6)                    │ 4 bytes   │
├─────────────────────────────┤
│ Total: 3 + 31 = 34 words    = 136 bytes │
└─────────────────────────────────────────┘ ← new sp

Why save ALL registers? The ISR is arbitrary code — it might use any
register. If we only saved "some" registers, the ISR could corrupt the
interrupted program's state. Saving everything is safe and simple.

Real CPUs optimize this: the ISR saves only the registers it actually
uses (callee-saved convention). We save all 32 for correctness.
```

### The Interrupt Lifecycle

This is the complete sequence of events when an interrupt fires. Each step
maps to real hardware behavior:

```
Step 1: Interrupt Raised
─────────────────────────
  A device (timer, keyboard) or instruction (ecall) sends a signal
  to the InterruptController.

  Timer chip every N cycles:  controller.RaiseInterrupt(32)
  Keyboard on keystroke:      controller.RaiseInterrupt(33)
  ecall instruction:          controller.RaiseInterrupt(128)

Step 2: Controller Checks Mask
──────────────────────────────
  The controller has a 32-bit mask register. Each bit corresponds to
  one interrupt number (for interrupts 0-31; higher numbers are always
  unmasked).

  Mask Register: 0b...0000_0000_0000_0100
                                      ^
                          Interrupt 2 is masked (blocked)

  If the interrupt is masked, it stays in the pending queue but is
  NOT dispatched. This is how the kernel temporarily blocks interrupts
  during critical sections (e.g., while modifying the process table).

  If the global enable flag is off (Enabled = false), ALL interrupts
  are blocked. This is used during interrupt handling itself (prevent
  interrupts from interrupting the interrupt handler).

Step 3: CPU Saves Context
─────────────────────────
  At the end of the current instruction (not mid-instruction!), the
  CPU pushes an InterruptFrame onto the kernel stack:

  1. sp -= 136 (make room for frame)
  2. Store PC, MStatus, MCause, x1-x31 at [sp]
  3. Set MCause to the interrupt number
  4. Disable interrupts (Enabled = false) — prevent nesting

  Why "end of current instruction"? Interrupts are checked between
  instructions, never mid-instruction. This ensures the interrupted
  instruction completes atomically.

Step 4: CPU Looks Up IDT
─────────────────────────
  The CPU reads IDT[interrupt_number]:

  entry = memory[IDT_BASE + interrupt_number * 8]

  If entry.Present == false → double fault (unhandled interrupt).
  If entry.Present == true  → ISR address = entry.ISRAddress.

Step 5: CPU Jumps to ISR
─────────────────────────
  PC = entry.ISRAddress

  The CPU is now executing kernel code (the interrupt handler).
  The interrupted program is frozen on the stack.

Step 6: ISR Handles the Interrupt
─────────────────────────────────
  What happens here depends on the interrupt type:

  Timer (32):    Call scheduler, possibly switch processes
  Keyboard (33): Read keystroke from I/O port, add to buffer
  Syscall (128): Read syscall number from a7, dispatch

  The ISR is a Go function registered with the ISRRegistry.

Step 7: ISR Returns (mret)
──────────────────────────
  The ISR signals completion. In real hardware, this is the mret
  instruction (machine return). In our simulation, the ISR function
  returns, and the controller restores context.

Step 8: CPU Restores Context
────────────────────────────
  1. Read InterruptFrame from kernel stack at [sp]
  2. Restore x1-x31, PC, MStatus
  3. sp += 136 (pop the frame)
  4. Re-enable interrupts (Enabled = true)

Step 9: CPU Resumes Original Code
──────────────────────────────────
  PC is now the saved return address. The CPU continues executing
  the interrupted program as if nothing happened.
```

### Interrupt Priority

When multiple interrupts are pending simultaneously, which one gets handled
first? Lower interrupt numbers have higher priority:

```
Priority:  0 (highest) ──────────────────────► 255 (lowest)

Multiple pending: [32 (timer), 128 (syscall), 33 (keyboard)]
Dispatch order:   32 first, then 33, then 128

Why? CPU exceptions (0-31) are most urgent — they indicate errors that
must be handled immediately. Hardware interrupts (32-47) are next because
devices have time-sensitive buffers. Software interrupts (128+) are least
urgent because the requesting program is already waiting.
```

### Nested Interrupts (Simplified)

Our implementation does NOT support nested interrupts. When an ISR is
running, interrupts are globally disabled. This simplifies the implementation
enormously:

- No need for interrupt priority levels within handlers
- No risk of stack overflow from deeply nested interrupts
- The kernel stack only needs space for one interrupt frame at a time

Real operating systems DO support nested interrupts (a higher-priority
interrupt can preempt a lower-priority handler), but this adds significant
complexity that is not needed for our boot-to-hello-world trace.

## Public API

```go
// --- Interrupt Types ---

type InterruptType int

const (
    InterruptFault    InterruptType = iota  // CPU exception (0-31)
    InterruptTimer                          // Clock tick (32)
    InterruptKeyboard                       // External keystroke (33)
    InterruptSyscall                        // Software ecall (128)
)

// Well-known interrupt numbers
const (
    IntDivisionByZero  = 0
    IntDebug           = 1
    IntNMI             = 2
    IntBreakpoint      = 3
    IntOverflow        = 4
    IntInvalidOpcode   = 5
    IntTimer           = 32
    IntKeyboard        = 33
    IntSyscall         = 128
)

// --- IDT Entry ---

type IDTEntry struct {
    ISRAddress     uint32  // Address of the interrupt service routine
    Present        bool    // True if this entry is valid
    PrivilegeLevel int     // 0 = kernel only
}

// --- Interrupt Descriptor Table ---

type InterruptDescriptorTable struct {
    Entries [256]IDTEntry
}

// NewIDT creates an empty IDT with all entries marked as not present.
func NewIDT() *InterruptDescriptorTable

// SetEntry installs a handler at the given interrupt number.
func (idt *InterruptDescriptorTable) SetEntry(number int, entry IDTEntry)

// GetEntry returns the entry for the given interrupt number.
func (idt *InterruptDescriptorTable) GetEntry(number int) IDTEntry

// WriteToMemory serializes the IDT to a byte slice at the given base address.
// Each entry occupies 8 bytes: 4 (address) + 1 (present) + 1 (privilege) + 2 (reserved).
func (idt *InterruptDescriptorTable) WriteToMemory(memory []byte, baseAddress uint32)

// LoadFromMemory deserializes the IDT from a byte slice at the given base address.
func (idt *InterruptDescriptorTable) LoadFromMemory(memory []byte, baseAddress uint32)

// --- Interrupt Frame ---

// InterruptFrame holds all CPU state needed to resume after an interrupt.
type InterruptFrame struct {
    PC        uint32       // Saved program counter (where to resume)
    Registers [32]uint32   // All 32 RISC-V general-purpose registers
    MStatus   uint32       // Machine status register
    MCause    uint32       // What caused the interrupt (interrupt number)
}

// --- ISR Registry ---

// ISRHandler is the Go function signature for interrupt service routines.
// The frame contains the saved CPU state. The kernel parameter provides
// access to kernel facilities (syscall dispatch, process table, etc.).
type ISRHandler func(frame *InterruptFrame, kernel interface{})

// ISRRegistry maps interrupt numbers to Go handler functions.
type ISRRegistry struct {
    handlers map[int]ISRHandler
}

// NewISRRegistry creates an empty ISR registry.
func NewISRRegistry() *ISRRegistry

// Register installs a handler for the given interrupt number.
// Overwrites any previously registered handler for this number.
func (r *ISRRegistry) Register(interruptNumber int, handler ISRHandler)

// Dispatch calls the registered handler for the given interrupt number.
// Panics if no handler is registered (should not happen if IDT is correct).
func (r *ISRRegistry) Dispatch(interruptNumber int, frame *InterruptFrame, kernel interface{})

// HasHandler returns true if a handler is registered for the given number.
func (r *ISRRegistry) HasHandler(interruptNumber int) bool

// --- Interrupt Controller ---

// InterruptController manages the full interrupt lifecycle: pending queue,
// masking, enable/disable, and context save/restore.
type InterruptController struct {
    IDT          *InterruptDescriptorTable
    Registry     *ISRRegistry
    Pending      []int       // Queue of pending interrupt numbers (sorted by priority)
    MaskRegister uint32      // Bitmask: bit N = 1 means interrupt N is masked (blocked)
    Enabled      bool        // Global interrupt enable flag
}

// NewInterruptController creates a controller with an empty IDT and registry.
func NewInterruptController() *InterruptController

// RaiseInterrupt adds an interrupt to the pending queue.
// If the interrupt is already pending, it is not added again (no duplicates).
func (ic *InterruptController) RaiseInterrupt(number int)

// HasPending returns true if there are any unmasked, pending interrupts
// and the global enable flag is set.
func (ic *InterruptController) HasPending() bool

// NextPending returns the highest-priority (lowest-numbered) unmasked
// pending interrupt, or -1 if none.
func (ic *InterruptController) NextPending() int

// Acknowledge removes the given interrupt from the pending queue.
// Called after the ISR completes.
func (ic *InterruptController) Acknowledge(number int)

// SetMask sets or clears the mask for a specific interrupt number.
// masked=true blocks the interrupt; masked=false allows it.
func (ic *InterruptController) SetMask(number int, masked bool)

// IsMasked returns true if the given interrupt number is currently masked.
func (ic *InterruptController) IsMasked(number int) bool

// Enable sets the global interrupt enable flag. Interrupts can be dispatched.
func (ic *InterruptController) Enable()

// Disable clears the global interrupt enable flag. No interrupts dispatch.
func (ic *InterruptController) Disable()

// SaveContext creates an InterruptFrame from the current CPU state.
func (ic *InterruptController) SaveContext(
    registers [32]uint32, pc uint32, mstatus uint32,
) InterruptFrame

// RestoreContext extracts CPU state from an InterruptFrame.
func (ic *InterruptController) RestoreContext(
    frame InterruptFrame,
) (registers [32]uint32, pc uint32, mstatus uint32)

// PendingCount returns the number of pending interrupts.
func (ic *InterruptController) PendingCount() int

// ClearAll removes all pending interrupts.
func (ic *InterruptController) ClearAll()
```

## Data Structures

### IDT Entry Binary Format

```go
// IDT entries are 8 bytes each in memory:
//
// Byte 0-3: ISR address (little-endian uint32)
// Byte 4:   Present (0x00 or 0x01)
// Byte 5:   Privilege level (0x00 = kernel)
// Byte 6-7: Reserved (0x00, 0x00)
//
// Total IDT size: 256 entries * 8 bytes = 2048 bytes

const IDTEntrySize = 8
const IDTSize = 256 * IDTEntrySize  // 2048 bytes
const IDTBaseAddress = 0x00000000
```

### Pending Queue

```go
// The pending queue is kept sorted by interrupt number (ascending).
// Lower numbers = higher priority = dispatched first.
//
// Example:
//   Pending: [5, 32, 33, 128]
//   NextPending() returns 5
//   After Acknowledge(5): [32, 33, 128]
//   NextPending() returns 32
```

### Mask Register Layout

```go
// The mask register is a 32-bit value where each bit corresponds to
// interrupt numbers 0-31. Interrupts 32+ are controlled differently
// (always unmasked in our simplified model, unless globally disabled).
//
// Bit 0 = interrupt 0 (division by zero)
// Bit 1 = interrupt 1 (debug)
// ...
// Bit 31 = interrupt 31
//
// 1 = masked (blocked), 0 = unmasked (allowed)
//
// Default: 0x00000000 (all unmasked)
```

## Test Strategy

### IDT Tests

- **NewIDT**: all 256 entries have Present=false
- **SetEntry/GetEntry**: set entry 32 with ISR address 0x00020100, read back,
  verify all fields match
- **Boundary entries**: set entry 0, entry 255, verify no off-by-one errors
- **Overwrite**: set entry 32 twice with different addresses, verify second
  value wins

### IDT Serialization Tests

- **WriteToMemory**: create IDT, set 3 entries, write to byte slice, verify
  bytes at expected offsets match the entry fields
- **LoadFromMemory**: write known bytes to a slice, load into IDT, verify
  entries match
- **Roundtrip**: set entries, write to memory, create new IDT, load from
  same memory, verify all entries identical
- **Endianness**: verify ISR addresses are stored little-endian (RISC-V
  convention)

### ISR Registry Tests

- **Register and dispatch**: register a handler for interrupt 32, dispatch
  interrupt 32, verify handler was called exactly once
- **Handler receives frame**: register handler, dispatch with a known
  InterruptFrame, verify handler received the correct frame
- **HasHandler**: register for 32, verify HasHandler(32)=true,
  HasHandler(33)=false
- **Overwrite**: register two different handlers for the same number, verify
  only the second is called

### InterruptController Tests

- **RaiseInterrupt**: raise interrupt 32, verify PendingCount()=1
- **HasPending**: raise interrupt, verify HasPending()=true
- **NextPending**: raise 33 and 32, verify NextPending()=32 (lower = higher
  priority)
- **Acknowledge**: raise 32, acknowledge 32, verify PendingCount()=0
- **No duplicates**: raise 32 twice, verify PendingCount()=1
- **Mask**: mask interrupt 5, raise interrupt 5, verify HasPending()=false
  (it is pending but masked)
- **Unmask**: mask 5, raise 5, unmask 5, verify HasPending()=true
- **Global disable**: disable interrupts, raise 32, verify HasPending()=false
- **Global enable**: disable, raise, enable, verify HasPending()=true

### Context Save/Restore Tests

- **Roundtrip**: create known register values and PC, save to frame, restore
  from frame, verify all values identical
- **All registers**: set all 32 registers to different values, save, restore,
  verify each
- **MStatus preserved**: set MStatus to a known value, save, restore, verify
- **MCause**: save with MCause=32, verify frame.MCause==32

### Priority Tests

- **Multiple pending**: raise interrupts 128, 33, 5, 32. Verify dispatch
  order is 5, 32, 33, 128
- **Acknowledge and next**: raise 5 and 32. NextPending()=5. Acknowledge(5).
  NextPending()=32

### Full Lifecycle Test

- **Complete cycle**: raise interrupt 32, verify HasPending, save context
  with known registers, look up IDT entry, dispatch ISR (verify handler
  called), acknowledge interrupt, restore context, verify registers match
  original values, verify PendingCount()=0

## Future Extensions

- **Nested interrupts**: allow higher-priority interrupts to preempt lower-
  priority handlers (requires per-handler priority levels and stack management)
- **Edge vs. level triggered**: model edge-triggered (fire once per signal
  edge) vs. level-triggered (fire continuously while signal is asserted)
- **Interrupt coalescing**: batch multiple rapid interrupts into fewer handler
  invocations (used by network cards for high-throughput I/O)
- **MSI/MSI-X**: Message Signaled Interrupts — modern PCI devices write to
  a memory address instead of asserting a physical interrupt line
- **APIC simulation**: Advanced Programmable Interrupt Controller for
  multi-core interrupt routing
