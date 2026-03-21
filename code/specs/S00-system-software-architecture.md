# S00 — System Software Architecture Overview

## Overview

The System Software (S) series adds the layer that transforms a bare CPU into a
machine that can *do something useful*. The D-series gave us a complete RISC-V
core with pipelined execution, caches, branch prediction, and hazard detection —
but right now, that core just sits there. It has no firmware to initialize it, no
way to respond to external events, no operating system to manage programs, and no
display to show output.

Think of it this way: the D-series built a car engine. The S-series adds the
ignition system, dashboard, steering wheel, and the road itself.

The S-series packages implement:

- **ROM/BIOS firmware** (S01) — the first code that runs at power-on
- **Bootloader** (S02) — finds and loads the operating system
- **Interrupt handler** (S03) — responds to hardware events and software traps
- **OS kernel** (S04) — manages processes, memory, and system calls
- **Text-mode display** (S05) — a framebuffer that renders characters to screen
- **System board** (S06) — wires everything together into a bootable system

The end goal: **boot a simulated OS that displays "Hello World"** — with the
entire execution trace visible from NAND gates through CPU pipeline stages
through OS system calls to framebuffer output.

## Why This Matters

A CPU can execute instructions. But without system software, it is like a brain
with no body — it can think, but it cannot act on the world.

Consider what happens when you press a key on your keyboard:

1. The keyboard controller sends an electrical signal.
2. The **interrupt controller** notices and interrupts the CPU.
3. The CPU saves its current state and jumps to an **interrupt service routine**.
4. The ISR reads the keypress from a memory-mapped I/O register.
5. The **OS kernel** decides which program should receive the keypress.
6. The program processes it and calls a **system call** to display a character.
7. The kernel writes to the **framebuffer**.
8. The character appears on screen.

Every single step in that chain requires system software. Without firmware, the
CPU does not know where to start executing. Without interrupts, the CPU cannot
respond to the keyboard. Without an OS, there is no program to receive the
keypress. Without a display, there is nothing to see.

This is the layer that bridges the gap between "a chip that can add numbers" and
"a computer you can interact with."

### The Real-World Parallel

When you power on your laptop:

```
Power button pressed
    │
    ▼
UEFI firmware runs (our S01: ROM/BIOS)
    │ — initializes RAM, detects hardware, runs POST
    ▼
GRUB/Windows Boot Manager loads (our S02: Bootloader)
    │ — finds the OS on disk, loads it into memory
    ▼
Linux/Windows kernel initializes (our S04: OS Kernel)
    │ — sets up interrupts (S03), memory management, drivers
    ▼
Login screen appears (our S05: Display)
    │ — framebuffer + display driver render pixels
    ▼
You start typing (keyboard → interrupt → OS → application → display)
```

Our S-series implements a simplified version of this entire chain, using the
D-series CPU core as the execution engine.

## Package Composition

```
SystemBoard (S06) — top-level integration
│
├── OS Kernel (S04)
│   ├── Process Table & Scheduler
│   ├── Memory Manager
│   └── Syscall Table
│
├── Interrupt Handler (S03)
│   ├── Interrupt Descriptor Table (IDT)
│   ├── ISR Registry
│   └── Interrupt Controller (extends D05 shell)
│
├── ROM / BIOS (S01)
│
├── Bootloader (S02)
│
├── Display (S05) — text-mode framebuffer
│
════════════════════════════════════════════
│           HARDWARE / SOFTWARE BOUNDARY
════════════════════════════════════════════
│
├── D05 Core (with RISC-V decoder)
│   ├── D04 cpu-pipeline (IF → ID → EX → MEM → WB)
│   ├── D02 Branch Predictor
│   ├── D03 Hazard Detection + Forwarding
│   ├── D01 Cache (L1I + L1D + L2)
│   ├── Clock
│   └── Register File (32 × 32-bit, RISC-V)
│
├── Arithmetic (ALU from logic gates)
└── Logic Gates (NAND, AND, OR, NOT, XOR...)
```

The double line in the middle is the **hardware/software boundary** — the most
important line in all of computer science. Below it, everything is hardware
(simulated by our D-series packages). Above it, everything is software running
*on* that hardware.

In a real computer, this boundary is where transistors meet machine code. In our
simulation, it is where the D05 Core's `step()` method executes instructions
that were written by the S-series packages.

## Layer Position

```
Logic Gates (10) → Arithmetic (9) → FP Arithmetic (FP01)
                                          │
                                    Deep CPU Internals
                                    ├── Cache (D01)
                                    ├── Branch Predictor (D02)
                                    ├── Hazard Detection (D03)
                                    ├── CPU Pipeline (D04)
                                    └── Core (D05)
                                          │
                                    ISA: RISC-V RV32I + M-mode
                                          │
                                    System Software         ← YOU ARE HERE
                                    ├── ROM / BIOS (S01)
                                    ├── Bootloader (S02)
                                    ├── Interrupt Handler (S03)
                                    ├── OS Kernel (S04)
                                    ├── Display (S05)
                                    └── System Integration (S06)
```

The S-series sits directly above the D-series. It consumes the D05 Core as a
black box: give it a memory image and a reset vector, and it executes
instructions. The S-series packages *write those instructions* — firmware,
bootloader code, kernel code, and user programs — all as RISC-V machine code
loaded into the core's memory.

### Where RISC-V M-mode Fits

RISC-V defines privilege levels (like clearance levels in a building):

```
┌─────────────────────────────────────────────────────┐
│  U-mode (User)         — user programs              │
│  Privilege level 0     — least trusted              │
├─────────────────────────────────────────────────────┤
│  S-mode (Supervisor)   — OS kernel                  │
│  Privilege level 1     — can manage memory, traps   │
├─────────────────────────────────────────────────────┤
│  M-mode (Machine)      — firmware / BIOS            │
│  Privilege level 3     — full hardware access        │
└─────────────────────────────────────────────────────┘
```

Our system uses a simplified two-level model:

- **M-mode**: ROM/BIOS and bootloader (S01, S02) — full hardware access
- **U-mode**: Kernel and user programs (S04, user code) — restricted, must use
  `ecall` for privileged operations

The interrupt handler (S03) mediates the transitions between modes.

## Memory Map

The memory map defines which addresses correspond to which regions. This is the
"floor plan" of our system's address space — every byte of memory has a purpose.

```
Address Range          Size      Region              Description
─────────────────────────────────────────────────────────────────────────
0x00000000-0x000007FF  2 KB      IDT                 Interrupt Descriptor Table
0x00000800-0x00000FFF  2 KB      ISR Stubs           Default interrupt service routines
0x00001000-0x000010FF  256 B     Boot Protocol       Hardware info struct (RAM size, etc.)
0x00010000-0x0001FFFF  64 KB     Bootloader          Bootloader code + data
0x00020000-0x0003FFFF  128 KB    Kernel              Kernel code + data
0x00040000-0x0005FFFF  128 KB    User Space          User programs
0x00060000-0x0006FFFF  64 KB     Kernel Stack        Grows downward from 0x0006FFF0
0x00070000-0x0007FFFF  64 KB     User Stack          Grows downward from 0x0007FFF0
0x00080000-0x000FFFFF  512 KB    Disk Image          Simulated storage

    --- I/O Region (high addresses, memory-mapped) ---

0xFFFB0000-0xFFFB0F9F  4 KB      Framebuffer         80×25 text-mode display
0xFFFC0000-0xFFFC00FF  256 B     Keyboard I/O        Memory-mapped keyboard port
0xFFFE0000-0xFFFE00FF  256 B     Timer I/O           Memory-mapped timer control
0xFFFF0000-0xFFFFFFFF  64 KB     ROM / BIOS          Read-only firmware
```

### Why These Addresses?

- **Low memory (0x00000000+)**: Convention from early processors. The interrupt
  table goes at address zero because the CPU needs to find it immediately on
  reset. The bootloader, kernel, and user space follow in ascending order.

- **High memory (0xFFFF0000+)**: The ROM sits at the very top of the address
  space. When the CPU resets, its program counter is set to `0xFFFF0000` — the
  first instruction of the BIOS. This is a common pattern: x86 CPUs start at
  `0xFFFFFFF0`, and many ARM chips start at `0xFFFF0000`.

- **I/O region (0xFFFB0000+)**: Memory-mapped I/O lives just below the ROM.
  Writing to `0xFFFB0000` does not store a value in RAM — it sends a character
  to the display. This is how hardware peripherals work: they "pretend" to be
  memory, but reads and writes trigger side effects.

### Memory-Mapped I/O: An Analogy

Imagine a row of mailboxes in an apartment building. Most mailboxes hold letters
(regular memory). But mailbox #251 is special — when you put a letter in it, the
building's PA system reads it aloud (the display). Mailbox #252 has a little
window that shows the last key someone pressed (the keyboard). The mailboxes
look identical from the outside, but some of them are connected to devices
instead of storage.

That is memory-mapped I/O: certain addresses are wired to hardware instead of
RAM.

### Framebuffer Layout

The text-mode framebuffer at `0xFFFB0000` stores 80 columns × 25 rows of
character cells. Each cell is 2 bytes:

```
Byte 0: ASCII character code (0x00-0xFF)
Byte 1: Attribute byte
         ┌─ Bits 7-4: background color (0-15)
         └─ Bits 3-0: foreground color (0-15)

Total size: 80 × 25 × 2 = 4,000 bytes

Example — write 'H' in white-on-black at row 0, column 0:
  Address 0xFFFB0000 ← 0x48  (ASCII 'H')
  Address 0xFFFB0001 ← 0x0F  (black background, white foreground)
```

This is exactly how the VGA text buffer worked on IBM PCs. Every character on
screen is just two bytes in memory.

## Boot Sequence

The boot sequence is the chain of events from power-on to a running operating
system. Our system goes through five phases:

### Phase 1: Power On (Cycle 0)

```
What happens:  CPU reset. All registers set to zero except the program counter,
               which is set to 0xFFFF0000 (the ROM entry point).

Memory used:   None yet — the CPU just sets its internal state.

Cycles:        1 (the reset cycle)
```

This is the equivalent of turning the ignition key. The engine (CPU) is alive
but has not done anything yet. The program counter points to ROM, so the very
next instruction fetch will read from the BIOS.

### Phase 2: BIOS Initialization (Cycles 1 — ~500)

```
What happens:  The BIOS firmware executes from ROM. It:
               1. Runs POST (Power-On Self-Test) — verifies RAM is working
               2. Initializes the Interrupt Descriptor Table at 0x00000000
               3. Installs default ISR stubs at 0x00000800
               4. Detects available memory and writes a Boot Protocol struct
                  at 0x00001000 (total RAM, framebuffer address, etc.)
               5. Writes "BIOS OK" to the framebuffer
               6. Jumps to the bootloader at 0x00010000

Memory used:   ROM (read), IDT region (write), ISR stubs (write),
               Boot Protocol (write), Framebuffer (write)

Cycles:        ~200-500 (mostly memory writes to set up tables)
```

The BIOS is the first software that runs. It is burned into ROM — you cannot
modify it at runtime (just like real firmware). Its job is to get the system into
a known-good state and then hand off to the bootloader.

### Phase 3: Bootloader (Cycles ~500 — ~2,000)

```
What happens:  The bootloader reads the Boot Protocol struct to learn about
               the system. It then:
               1. Reads the kernel image from the simulated disk (at 0x00080000)
               2. Copies the kernel to its load address (0x00020000)
               3. Sets up the kernel stack pointer at 0x0006FFF0
               4. Writes "Loading kernel..." to the framebuffer
               5. Jumps to the kernel entry point at 0x00020000

Memory used:   Bootloader region (execute), Boot Protocol (read),
               Disk Image (read), Kernel region (write),
               Kernel Stack (initialize), Framebuffer (write)

Cycles:        ~1,000-1,500 (dominated by the disk-to-memory copy)
```

The bootloader is a small, simple program whose only job is to find the OS and
load it. In the real world, GRUB and Windows Boot Manager do this. Our
bootloader is much simpler — it knows exactly where the kernel is on the
simulated disk.

### Phase 4: Kernel Initialization (Cycles ~2,000 — ~5,000)

```
What happens:  The kernel takes control. It:
               1. Re-initializes the IDT with kernel-level ISRs
               2. Installs the syscall handler at interrupt 128
               3. Sets up the timer interrupt (interrupt 32) for scheduling
               4. Initializes the process table (empty, one slot for init)
               5. Sets up the memory manager (marks regions as used/free)
               6. Creates the "init" process (the Hello World program)
               7. Sets up the user stack at 0x0007FFF0
               8. Drops to U-mode and jumps to the user program at 0x00040000

Memory used:   Kernel region (execute), IDT (rewrite), Process Table (write),
               User Space (write — load init program), User Stack (initialize)

Cycles:        ~2,000-3,000 (table setup + process creation)
```

The kernel is the heart of the OS. Once it finishes initialization, the system
is fully operational: interrupts work, syscalls work, and user programs can run.

### Phase 5: Hello World and Idle (Cycles ~5,000+)

```
What happens:  The user program executes. It:
               1. Loads the address of the "Hello World\n" string
               2. Calls sys_write (syscall 1) via the ecall instruction
                  — a0 = 1 (stdout), a1 = string address, a2 = 12 (length)
                  — a7 = 1 (sys_write syscall number)
               3. The ecall triggers interrupt 128
               4. The kernel's syscall handler reads a7, dispatches to sys_write
               5. sys_write copies each character to the framebuffer
               6. The kernel returns to user mode
               7. The program calls sys_exit (syscall 0)
               8. The kernel marks the process as terminated
               9. The scheduler finds no runnable processes
              10. The kernel enters an idle loop (wfi — wait for interrupt)

Memory used:   User Space (execute), User Stack (function calls),
               Framebuffer (write — the actual "Hello World" output),
               Kernel (syscall handling)

Cycles:        ~1,000-2,000 (12 characters × ~100 cycles each for the
               full syscall round-trip per character)
```

And there it is: **"Hello World" on the screen**, with every single cycle
traceable from the user program through the syscall interface through the kernel
through the framebuffer to the display output. The entire path is visible.

### Boot Sequence Summary

```
Cycle 0         ~500          ~2,000        ~5,000        ~7,000
  │              │              │              │              │
  ▼              ▼              ▼              ▼              ▼
┌──────┐    ┌─────────┐    ┌───────────┐  ┌──────────┐  ┌──────┐
│RESET │───▶│  BIOS   │───▶│BOOTLOADER │─▶│ KERNEL   │─▶│HELLO │
│      │    │  (ROM)  │    │           │  │  INIT    │  │WORLD │
└──────┘    └─────────┘    └───────────┘  └──────────┘  └──────┘
   PC=         POST,          Load           IDT,         ecall,
 0xFFFF0000   IDT setup,     kernel         scheduler,   sys_write,
              jump to BL     to 0x20000     processes    framebuffer
```

## Interrupt Architecture

Interrupts are the mechanism by which hardware (and software) can get the CPU's
attention. Without interrupts, the CPU would have to constantly *poll* every
device: "Any keys pressed? No? How about now? Still no?" — wasting enormous
numbers of cycles. Interrupts flip this around: the device tells the CPU when
something happens.

### Interrupt Number Assignments

```
Number    Type              Source              Description
──────────────────────────────────────────────────────────────────
0-31      CPU Exceptions    CPU itself          Division by zero, invalid opcode,
                                                page fault, etc.
32        Timer             Hardware timer      Fires periodically for scheduling
33        Keyboard          Keyboard ctrl       Fires on keypress
34-127    Reserved          —                   Future hardware devices
128       Syscall           Software (ecall)    User program requests OS service
129-255   Reserved          —                   Future use
```

### Interrupt Lifecycle

When an interrupt fires, the CPU and system software cooperate through a precise
sequence of steps:

```
Step 1: RAISE
  Something triggers the interrupt.
  — Hardware: timer ticks, key pressed → interrupt controller signals CPU
  — Software: ecall instruction → CPU raises interrupt 128 internally

Step 2: MASK CHECK
  The CPU checks the interrupt mask register (part of the RISC-V mstatus CSR).
  — If this interrupt is masked (disabled), it is held pending. Stop here.
  — If this interrupt is enabled, proceed.

Step 3: CONTEXT SAVE
  The CPU saves the current execution state so it can resume later:
  — mepc  ← current program counter (where to return)
  — mcause ← interrupt number (what happened)
  — Push registers onto the kernel stack (done by the ISR prologue)

Step 4: IDT LOOKUP
  The CPU reads the Interrupt Descriptor Table entry for this interrupt number.
  — IDT base address: 0x00000000
  — Entry N is at address: 0x00000000 + (N × 8)
  — Each entry contains: handler address (4 bytes) + flags (4 bytes)

Step 5: ISR DISPATCH
  The CPU sets the program counter to the handler address from the IDT entry.
  The interrupt service routine executes:
  — For timer (32): increment tick counter, call scheduler
  — For keyboard (33): read keycode from 0xFFFC0000, buffer it
  — For syscall (128): read syscall number from a7, dispatch to handler

Step 6: CONTEXT RESTORE
  The ISR epilogue restores the saved registers from the kernel stack.
  The mret instruction restores the program counter from mepc.

Step 7: RESUME
  Execution continues from exactly where it was interrupted.
  The interrupted program never knows it was paused.
```

### How Host Keystrokes Become Interrupts

Since we are *simulating* a computer, there is no physical keyboard. Instead,
the host machine's keystrokes are translated into simulated interrupts:

```
Host machine                        Simulated machine
─────────────                       ──────────────────
User presses 'A'
    │
    ▼
Host OS delivers keypress
to our simulator process
    │
    ▼
Simulator writes 0x41 ('A')
to memory address 0xFFFC0000   ───▶  Keyboard I/O register updated
    │
    ▼
Simulator sets interrupt
pending flag for IRQ 33        ───▶  Interrupt controller sees pending IRQ
    │
    ▼
                                     CPU checks for pending interrupts
                                     at end of current instruction
                                         │
                                         ▼
                                     Normal interrupt lifecycle begins
                                     (mask check → save → IDT → ISR → restore)
```

## Syscall Convention

System calls are the interface between user programs and the kernel. When a user
program needs to do something it cannot do on its own (write to the display,
read input, exit), it asks the kernel via a syscall.

### The RISC-V Calling Convention

We follow the standard RISC-V convention for system calls:

```
Register   ABI Name   Purpose
─────────────────────────────────────
a7 (x17)   a7         Syscall number (which service to invoke)
a0 (x10)   a0         Argument 1 / Return value
a1 (x11)   a1         Argument 2
a2 (x12)   a2         Argument 3
a3 (x13)   a3         Argument 4
a4 (x14)   a4         Argument 5
a5 (x15)   a5         Argument 6
a6 (x16)   a6         Argument 7
```

The `ecall` instruction triggers interrupt 128. The kernel's syscall handler
reads the syscall number from `a7` and dispatches to the appropriate handler.

### Syscall Table

```
Number   Name        Arguments                         Returns    Description
─────────────────────────────────────────────────────────────────────────────────
0        sys_exit    a0 = exit code                    (none)     Terminate process
1        sys_write   a0 = fd, a1 = buf, a2 = len      a0 = n     Write bytes to fd
2        sys_read    a0 = fd, a1 = buf, a2 = len      a0 = n     Read bytes from fd
3        sys_yield   (none)                            (none)     Yield CPU to scheduler
```

### Syscall Example: "Hello World"

Here is how a user program writes "Hello World" to the display, shown as
RISC-V assembly:

```asm
# Load arguments for sys_write
li   a7, 1              # syscall number = sys_write
li   a0, 1              # fd = 1 (stdout)
la   a1, hello_string   # buf = address of "Hello World\n"
li   a2, 12             # len = 12 bytes

# Trigger the syscall
ecall                    # raises interrupt 128

# On return, a0 contains the number of bytes written (12)

# Exit the program
li   a7, 0              # syscall number = sys_exit
li   a0, 0              # exit code = 0 (success)
ecall                    # raises interrupt 128, kernel terminates process

hello_string:
    .ascii "Hello World\n"
```

What happens inside the CPU when `ecall` executes:

```
Cycle N:    ecall decoded in ID stage
Cycle N+1:  Pipeline flushed (interrupt changes control flow)
            mepc ← PC of ecall instruction
            mcause ← 128
            PC ← IDT[128].handler_address
Cycle N+2:  Fetching first instruction of syscall handler
Cycle N+3:  Handler reads a7 → dispatches to sys_write
...
Cycle N+K:  sys_write writes characters to framebuffer
            mret instruction → PC ← mepc + 4 (instruction after ecall)
```

## Dependencies Between S-series Packages

```
S01 (ROM/BIOS)          — depends on: D05 Core, cpu-simulator Memory
S02 (Bootloader)        — depends on: S01
S03 (Interrupt Handler) — depends on: D05 Core InterruptController
S04 (OS Kernel)         — depends on: S03, S05
S05 (Display)           — depends on: cpu-simulator Memory
S06 (System Board)      — depends on: S01, S02, S03, S04, S05, D05 Core
```

Visually:

```
                    ┌──────────────┐
                    │  S06 System  │
                    │    Board     │
                    └──────┬───────┘
           ┌───────┬───────┼───────┬────────┐
           ▼       ▼       ▼       ▼        ▼
        ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐
        │ S01 │ │ S02 │ │ S03 │ │ S04 │ │ S05 │
        │BIOS │ │Boot │ │ Int │ │ OS  │ │Disp │
        └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘
           │       │       │       │        │
           │       ▼       │       ▼        │
           │    ┌─────┐    │    ┌─────┐     │
           │    │ S01 │    │    │S03  │     │
           │    │     │    │    │S05  │     │
           │    └─────┘    │    └─────┘     │
           │               │                │
           └───────┬───────┘                │
                   ▼                        ▼
           ┌──────────────┐         ┌──────────────┐
           │   D05 Core   │         │cpu-sim Memory│
           └──────────────┘         └──────────────┘
```

## Spec Numbering

| Spec | Package             | Description                                       |
|------|---------------------|---------------------------------------------------|
| S00  | —                   | This architecture overview                        |
| S01  | `rom-bios`          | ROM memory region + BIOS firmware                 |
| S02  | `bootloader`        | Bootloader — loads kernel from disk to memory     |
| S03  | `interrupt-handler` | Interrupt system (IDT, ISR registry, lifecycle)   |
| S04  | `os-kernel`         | Minimal monolithic kernel (scheduler, memory, syscalls) |
| S05  | `display`           | Text-mode framebuffer display (80×25)             |
| S06  | `system-board`      | System integration — wires everything, boots, traces |

## Implementation Languages

Each package will be implemented in **Go, Python, Ruby, and Rust** — matching
the existing pattern in the repo. Go and Python are the primary implementations;
Ruby and Rust follow with equivalent functionality.

## The Full Trace

This is the key educational payoff of the entire project. When the system boots
and displays "Hello World", every single step is traceable:

```
Source code:    print("Hello World")
                         │
                         ▼
RISC-V binary:  li a7, 1          # sys_write
                la a1, hello_str
                li a2, 12
                ecall
                         │
                         ▼
D05 Core:       Fetch ecall from L1I cache
                         │ cache miss? → L2 → main memory
                         ▼
D04 Pipeline:   IF → ID (decode ecall) → pipeline flush
                         │
                         ▼
D02 Predictor:  (no branch to predict — interrupt is unconditional)
                         │
                         ▼
D03 Hazard:     Flush all in-flight instructions
                         │
                         ▼
S03 Interrupt:  Save context → IDT lookup → ISR dispatch
                         │
                         ▼
S04 Kernel:     Syscall handler reads a7=1 → sys_write
                         │
                         ▼
D04 Pipeline:   Execute sys_write loop body:
                  lbu → pipeline → D01 Cache → memory read
                  sb  → pipeline → D03 forward → memory write
                         │
                         ▼
S05 Display:    Character written to framebuffer at 0xFFFB0000+offset
                         │
                         ▼
Screen:         'H' appears at row 0, column 0
                ... repeat for 'e', 'l', 'l', 'o', ' ', 'W', 'o', 'r', 'l', 'd', '\n'
                         │
                         ▼
                "Hello World" displayed. Total: ~7,000 cycles.
```

At every stage, the simulation can report:

- **Logic gate level**: How many NAND gates fired to compute the ALU result
- **Pipeline level**: Which stages are active, any stalls or flushes
- **Cache level**: Hits, misses, evictions
- **Interrupt level**: Context save/restore, IDT lookups
- **OS level**: Syscall dispatch, process scheduling
- **Display level**: Framebuffer writes, character rendering

Someone studying this system can zoom in to any layer and see exactly what
happens. That is the goal: **no magic, no hidden steps, everything visible.**

## Future Extensions

- **Virtual memory**: page tables, TLB, page fault handler
- **Filesystem**: a simple flat filesystem on the simulated disk
- **Multiple processes**: round-robin scheduling with timer interrupts
- **Shell**: a command-line interface running as a user process
- **Networking**: a simulated network interface with interrupts
- **ELF loader**: load standard ELF binaries instead of raw memory images
