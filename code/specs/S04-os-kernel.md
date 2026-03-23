# S04 — OS Kernel

## Overview

This is a minimal monolithic kernel. It manages two processes (an idle loop
and a hello-world program), handles system calls, and drives a round-robin
scheduler via timer interrupts. The kernel is intentionally minimal — its
only job is to demonstrate the full boot-to-display trace, proving that every
layer of the stack (from NAND gates to "Hello World" on screen) actually
works together.

Despite its simplicity, the kernel contains the essential building blocks of
every real operating system: process management, scheduling, memory
management, system call dispatch, and device drivers.

**Analogy:** The kernel is the manager of a small office. There are only two
employees (the idle process and hello-world), but the manager handles
scheduling ("your turn to use the desk"), memory allocation ("you get rooms
3 and 4"), and responds to requests ("I need to print something" — that is a
system call). The manager never does "real work" — it only coordinates.

## Layer Position

```
SystemBoard (S06) — top-level integration
│
├── Core (D05) — executes instructions
│
├── Interrupt Handler (S03) — delivers events to kernel
│     ├── Timer tick (32) → kernel scheduler
│     ├── Keyboard (33)   → kernel keyboard handler
│     └── Syscall (128)   → kernel syscall dispatcher
│
├── OS Kernel (S04) ← YOU ARE HERE
│     ├── Process Table — PCBs for idle + hello-world
│     ├── Scheduler — round-robin via timer interrupts
│     ├── Memory Manager — region-based allocation
│     ├── Syscall Handler — sys_exit, sys_write, sys_read, sys_yield
│     └── Device interface — display driver, keyboard buffer
│
├── Display Driver (S05) — framebuffer writes
│
└── User Programs — idle loop, hello-world
```

**Depends on:** S03 (interrupt handler delivers events), S05 (display driver
for sys_write output), S02 (bootloader loads kernel into memory)

**Used by:** S06 SystemBoard (creates and boots the kernel), user programs
(via system calls)

## Key Concepts

### What Does a Kernel Do?

A kernel is the bridge between hardware and software. User programs cannot
directly access hardware (memory, display, keyboard) — they must ask the
kernel, which checks permissions and performs the operation on their behalf.
This is the fundamental security boundary in every operating system.

```
User Program                    Kernel                     Hardware
─────────────                   ──────                     ────────
"Print H"  ──ecall──►  sys_write(1, &'H', 1)  ──►  Display framebuffer
"Read key"  ──ecall──►  sys_read(0, &buf, 1)  ──►  Keyboard I/O port
"I'm done"  ──ecall──►  sys_exit(0)            ──►  Process table update
```

### Process Control Block (PCB)

Every process has a PCB — a data structure that stores everything the kernel
needs to know about it. When the kernel switches from one process to another
(context switch), it saves the outgoing process's CPU registers into its PCB
and loads the incoming process's registers from its PCB.

```
Process Control Block:
┌─────────────────────────────────────────────────┐
│ PID: 1                                           │
│ Name: "hello-world"                              │
│ State: Running                                   │
│                                                  │
│ Saved Registers:                                 │
│   x0=0  x1=0x00040010  x2=0x0005FFF0  ...      │
│   x10=1  x17=1  (a0=fd, a7=syscall_number)     │
│                                                  │
│ Saved PC: 0x00040024                             │
│ Stack Pointer: 0x0005FFF0                        │
│                                                  │
│ Memory Region:                                   │
│   Base: 0x00040000                               │
│   Size: 0x00010000 (64 KB)                       │
│   Permissions: R+W+X                             │
└─────────────────────────────────────────────────┘
```

**Process States:**

```
                    ┌───────────┐
        create ────►│   Ready   │◄──── timer tick (preempted)
                    └─────┬─────┘
                          │ scheduled
                          ▼
                    ┌───────────┐
                    │  Running  │
                    └──┬──┬──┬──┘
           sys_yield   │  │  │   timer tick
          ┌────────────┘  │  └────────────┐
          ▼               │               ▼
    ┌───────────┐         │         ┌───────────┐
    │   Ready   │         │         │   Ready   │
    └───────────┘         │         └───────────┘
                          │ sys_exit
                          ▼
                    ┌───────────┐
                    │Terminated │
                    └───────────┘

    (Blocked state exists for future I/O wait support
     but is unused in the hello-world demo.)
```

### The Two Processes

Our kernel runs exactly two processes:

```
PID 0: Idle Process
───────────────────
Purpose: Keep the CPU busy when no real work exists.
Code:    infinite loop: { sys_yield(); }
Memory:  0x00030000 - 0x0003FFFF (64 KB)

The idle process is always Ready (never Terminated). When the scheduler
has no other process to run, it runs idle. This is how real OSes work —
Linux has a "swapper" process (PID 0) that runs when the CPU is idle.

PID 1: Hello-World Process
──────────────────────────
Purpose: Print "Hello World\n" and exit.
Code:    sys_write(1, "Hello World\n", 12); sys_exit(0);
Memory:  0x00040000 - 0x0004FFFF (64 KB)

The hello-world process runs once, prints to stdout (file descriptor 1),
and terminates. After it exits, only the idle process remains, and the
system becomes idle.
```

### Round-Robin Scheduling

The scheduler decides which process runs next. We use the simplest possible
algorithm: **round-robin**. Each process gets an equal time slice (driven by
timer interrupts), and processes take turns in order.

```
Timeline with 2 processes and timer interrupts every 100 cycles:

Cycle:    0          100        200        300        400
          │          │          │          │          │
Process:  [  PID 1  ][ PID 0  ][  PID 1  ][ PID 0  ][  PID 1  ]
          hello-wrld  idle       hello-wrld  idle       hello-wrld
          │          │          │          │          │
Event:    boot    timer→     timer→     timer→     timer→
                  switch     switch     switch     switch

When the timer fires (interrupt 32):
1. Save current process registers to its PCB
2. Set current process state = Ready
3. Pick next Ready process (round-robin order)
4. Load next process registers from its PCB
5. Set next process state = Running
6. Return from interrupt → CPU now runs the next process
```

In practice, hello-world will complete in far fewer than 100 cycles (it
only needs to execute a few instructions), so the timeline above is
simplified. Once hello-world calls `sys_exit`, the scheduler only has the
idle process left.

### Memory Manager

Our memory manager uses **region-based allocation** — the simplest possible
scheme. There is no paging, no virtual memory, no MMU. Each process gets a
fixed region of physical memory assigned at creation time.

```
Memory Layout:
┌──────────────────────────────────┐ 0xFFFFFFFF
│ I/O Devices (Display, Keyboard)  │ 0xFFFB0000+
├──────────────────────────────────┤
│ (Gap)                            │
├──────────────────────────────────┤
│ Disk Image (memory-mapped)       │ 0x10000000+
├──────────────────────────────────┤
│ (Gap)                            │
├──────────────────────────────────┤
│ Kernel Stack                     │ 0x00060000 - 0x0006FFFF
├──────────────────────────────────┤
│ Process 1 (hello-world)          │ 0x00040000 - 0x0004FFFF  R+W+X
├──────────────────────────────────┤
│ Process 0 (idle)                 │ 0x00030000 - 0x0003FFFF  R+W+X
├──────────────────────────────────┤
│ Kernel Code + Data               │ 0x00020000 - 0x0002FFFF  R+W+X
├──────────────────────────────────┤
│ Bootloader                       │ 0x00010000 - 0x0001FFFF
├──────────────────────────────────┤
│ Boot Protocol                    │ 0x00001000 - 0x00001FFF
├──────────────────────────────────┤
│ IDT                              │ 0x00000000 - 0x000007FF
└──────────────────────────────────┘
```

Each memory region has permissions (read, write, execute). A process can
only access memory within its own region. If it tries to access memory
outside its region, the kernel could (in a future extension) raise a fault.
For now, our simulation does not enforce this — but the data structure
tracks it for educational value.

### System Call Interface

System calls are how user programs request services from the kernel. On
RISC-V, the convention is:

```
Registers for System Calls:
  a7 (x17): syscall number
  a0 (x10): first argument / return value
  a1 (x11): second argument
  a2 (x12): third argument
  a3-a6 (x13-x16): additional arguments (unused in our 4 syscalls)

Trigger: the ecall instruction (raises interrupt 128)

Sequence:
  User code:    li a7, 1          # syscall number = sys_write
                li a0, 1          # fd = stdout
                la a1, message    # buffer address
                li a2, 12         # length = 12 bytes
                ecall             # trap to kernel!

  Kernel:       (interrupt 128 fires)
                read a7 → syscall_number = 1
                dispatch to sys_write handler
                sys_write reads a0, a1, a2
                writes bytes to display
                set a0 = 12 (bytes written, return value)
                return from interrupt

  User code:    (resumes, a0 now contains return value)
```

### Syscall Table

```
┌────────┬───────────┬────────────────────┬─────────────────────────────────┐
│ Number │   Name    │       Args         │          Description            │
├────────┼───────────┼────────────────────┼─────────────────────────────────┤
│   0    │ sys_exit  │ a0 = exit code     │ Terminate current process.      │
│        │           │                    │ Set state = Terminated.         │
│        │           │                    │ Schedule next process.          │
├────────┼───────────┼────────────────────┼─────────────────────────────────┤
│   1    │ sys_write │ a0 = fd (1=stdout) │ Write bytes to display.         │
│        │           │ a1 = buffer addr   │ Reads bytes from process memory │
│        │           │ a2 = length        │ at [a1..a1+a2), sends to        │
│        │           │                    │ DisplayDriver. Returns bytes    │
│        │           │                    │ written in a0.                  │
├────────┼───────────┼────────────────────┼─────────────────────────────────┤
│   2    │ sys_read  │ a0 = fd (0=stdin)  │ Read from keyboard buffer.      │
│        │           │ a1 = buffer addr   │ Copies up to a2 bytes from      │
│        │           │ a2 = max length    │ kernel keyboard buffer to       │
│        │           │                    │ process memory at a1. Returns   │
│        │           │                    │ bytes read in a0.               │
├────────┼───────────┼────────────────────┼─────────────────────────────────┤
│   3    │ sys_yield │ (none)             │ Voluntarily give up CPU.        │
│        │           │                    │ Current process → Ready.        │
│        │           │                    │ Scheduler picks next process.   │
└────────┴───────────┴────────────────────┴─────────────────────────────────┘
```

## Public API

```go
// --- Process ---

type ProcessState int

const (
    ProcessReady      ProcessState = iota  // Waiting to be scheduled
    ProcessRunning                         // Currently executing on CPU
    ProcessBlocked                         // Waiting for I/O (future use)
    ProcessTerminated                      // Finished execution
)

type ProcessControlBlock struct {
    PID            int              // Unique process identifier
    State          ProcessState     // Current execution state
    SavedRegisters [32]uint32       // Saved RISC-V registers (for context switch)
    SavedPC        uint32           // Saved program counter
    StackPointer   uint32           // Top of this process's stack
    MemoryBase     uint32           // Start of process memory region
    MemorySize     uint32           // Size of process memory region
    Name           string           // Human-readable process name
    ExitCode       int              // Set by sys_exit
}

// --- Memory Manager ---

type MemoryPermission uint8

const (
    PermRead    MemoryPermission = 1 << iota  // 0x01
    PermWrite                                 // 0x02
    PermExecute                               // 0x04
)

type MemoryRegion struct {
    Base        uint32            // Start address
    Size        uint32            // Size in bytes
    Permissions MemoryPermission  // R/W/X flags
    Owner       int               // PID, or -1 for kernel
    Name        string            // Human-readable name ("kernel code", "PID 1 memory")
}

type MemoryManager struct {
    Regions []MemoryRegion
}

// NewMemoryManager creates a memory manager with the given pre-defined regions.
func NewMemoryManager(regions []MemoryRegion) *MemoryManager

// FindRegion returns the memory region containing the given address, or nil.
func (mm *MemoryManager) FindRegion(address uint32) *MemoryRegion

// CheckAccess verifies that the given PID can access the given address
// with the given permissions. Returns true if allowed.
func (mm *MemoryManager) CheckAccess(pid int, address uint32, perm MemoryPermission) bool

// AllocateRegion adds a new memory region for the given PID.
func (mm *MemoryManager) AllocateRegion(pid int, base, size uint32, perm MemoryPermission, name string)

// --- Scheduler ---

type Scheduler struct {
    ProcessTable []*ProcessControlBlock
    Current      int  // Index into ProcessTable (PID of running process)
}

// NewScheduler creates a scheduler with the given process table.
func NewScheduler(processTable []*ProcessControlBlock) *Scheduler

// Schedule picks the next Ready process using round-robin.
// Returns the PID of the next process to run.
// If only the idle process (PID 0) is Ready, returns 0.
func (s *Scheduler) Schedule() int

// ContextSwitch saves the CPU state to the outgoing process's PCB and
// loads the incoming process's PCB into the CPU state.
func (s *Scheduler) ContextSwitch(from, to int)

// --- Kernel ---

type KernelConfig struct {
    TimerInterval int              // Cycles between timer interrupts (default: 100)
    MaxProcesses  int              // Max process table size (default: 16)
    MemoryLayout  []MemoryRegion   // Pre-defined memory regions
}

// DefaultKernelConfig returns a configuration suitable for the hello-world demo.
func DefaultKernelConfig() KernelConfig

type Kernel struct {
    Config         KernelConfig
    ProcessTable   []*ProcessControlBlock
    CurrentProcess int                       // PID of running process
    Scheduler      *Scheduler
    MemoryManager  *MemoryManager
    InterruptCtrl  *InterruptController      // From S03
    Display        *DisplayDriver            // From S05
    KeyboardBuffer []byte                    // Keystrokes waiting to be read
    Booted         bool                      // True after Boot() completes
}

// NewKernel creates a kernel with the given configuration and hardware references.
func NewKernel(
    config KernelConfig,
    interruptCtrl *InterruptController,
    display *DisplayDriver,
) *Kernel

// Boot initializes all subsystems, creates the idle and hello-world processes,
// registers ISRs, and starts the scheduler. This is the kernel's main entry point,
// called by the bootloader's jump to 0x00020000.
func (k *Kernel) Boot()

// CreateProcess creates a new process with the given binary loaded at memBase.
// Returns the PID assigned to the new process.
func (k *Kernel) CreateProcess(name string, binary []byte, memBase uint32) int

// HandleSyscall is the ISR for interrupt 128 (ecall).
// Reads a7 from the frame, dispatches to the appropriate syscall handler.
func (k *Kernel) HandleSyscall(frame *InterruptFrame)

// HandleTimer is the ISR for interrupt 32 (timer tick).
// Saves current process state, calls scheduler, switches to next process.
func (k *Kernel) HandleTimer(frame *InterruptFrame)

// HandleKeyboard is the ISR for interrupt 33 (keyboard).
// Reads the keystroke from the I/O port and appends to KeyboardBuffer.
func (k *Kernel) HandleKeyboard(frame *InterruptFrame)

// IsIdle returns true when only the idle process (PID 0) is Ready and all
// other processes are Terminated. This signals the system has completed
// all useful work.
func (k *Kernel) IsIdle() bool

// ProcessInfo returns a summary of a process for snapshots and debugging.
func (k *Kernel) ProcessInfo(pid int) ProcessInfo

type ProcessInfo struct {
    PID      int
    Name     string
    State    ProcessState
    PC       uint32
}
```

## Kernel Boot Sequence

The boot sequence is the first code the kernel executes after the bootloader
jumps to `0x00020000`. It initializes all subsystems in order:

```
Kernel Boot Sequence:
─────────────────────

Step 1: Initialize Memory Manager
  - Create regions for kernel code, kernel stack, idle process, hello-world
  - No dynamic allocation — all regions are pre-defined

Step 2: Initialize Process Table
  - Allocate empty table (capacity = MaxProcesses)
  - No processes exist yet

Step 3: Register ISRs with Interrupt Handler (S03)
  - ISR 32  (timer)    → k.HandleTimer
  - ISR 33  (keyboard) → k.HandleKeyboard
  - ISR 128 (syscall)  → k.HandleSyscall

Step 4: Enable Timer Interrupt
  - Tell interrupt controller: unmask interrupt 32
  - Timer will fire every KernelConfig.TimerInterval cycles

Step 5: Create Idle Process (PID 0)
  - Generate a small RISC-V binary: loop { ecall(sys_yield) }
  - CreateProcess("idle", idleBinary, 0x00030000)
  - Process state = Ready

Step 6: Load Hello-World from Disk
  - Read hello-world binary from disk image
  - CreateProcess("hello-world", hwBinary, 0x00040000)
  - Process state = Ready

Step 7: Start Scheduler
  - Set CurrentProcess = 1 (hello-world)
  - Set PID 1 state = Running
  - Context switch: load PID 1 registers into CPU
  - Return → CPU begins executing hello-world

From this point on, the kernel only runs in response to interrupts
(timer ticks, keystrokes, system calls). It never runs "in the background."
```

## Data Structures

### Process State Transitions

```go
// State transition table:
//
// Current State   Event              New State
// ─────────────   ─────              ─────────
// (none)          CreateProcess()    Ready
// Ready           Scheduled          Running
// Running         Timer tick         Ready       (preempted)
// Running         sys_yield          Ready       (voluntary)
// Running         sys_exit           Terminated
// Running         sys_read (empty)   Blocked     (future: wait for input)
// Blocked         Data available     Ready       (future: keyboard input)
```

### Syscall Dispatch Table

```go
// syscallHandlers maps syscall numbers to handler functions.
// Each handler receives the interrupt frame and modifies a0 for the return value.
var syscallHandlers = map[int]func(k *Kernel, frame *InterruptFrame){
    0: (*Kernel).sysExit,
    1: (*Kernel).sysWrite,
    2: (*Kernel).sysRead,
    3: (*Kernel).sysYield,
}
```

### Hello-World Binary

The hello-world user program is a sequence of RISC-V instructions generated
programmatically (similar to the bootloader):

```
hello_world:
    # "Hello World\n" is stored in the data section at the end of the binary
    lui  a1, %hi(message)     # a1 = address of "Hello World\n"
    addi a1, a1, %lo(message)
    li   a0, 1                # a0 = 1 (stdout)
    li   a2, 12               # a2 = 12 (length of "Hello World\n")
    li   a7, 1                # a7 = 1 (sys_write)
    ecall                     # trap to kernel

    li   a0, 0                # a0 = 0 (exit code)
    li   a7, 0                # a7 = 0 (sys_exit)
    ecall                     # trap to kernel

message:
    .ascii "Hello World\n"    # 12 bytes of string data
```

## Test Strategy

### Process Management Tests

- **CreateProcess**: create a process, verify PID assigned, PCB fields set
  correctly (name, state=Ready, memory base/size)
- **Process state**: create process, verify state=Ready; schedule it, verify
  state=Running; call sys_exit, verify state=Terminated
- **Multiple processes**: create 2 processes, verify distinct PIDs
- **Max processes**: attempt to create more than MaxProcesses, verify error

### Syscall Dispatch Tests

- **sys_write**: simulate ecall with a7=1, a0=1, a1=address_of_string,
  a2=5. Verify 5 characters appear in the display framebuffer
- **sys_write return value**: verify a0 is set to number of bytes written
- **sys_exit**: simulate ecall with a7=0, a0=42. Verify process
  state=Terminated and exit code=42
- **sys_yield**: simulate ecall with a7=3. Verify process state changes to
  Ready and scheduler is invoked
- **sys_read**: populate keyboard buffer with "AB", simulate ecall with
  a7=2, a0=0, a1=buf, a2=10. Verify 2 bytes copied, a0=2
- **sys_read empty buffer**: simulate sys_read with empty keyboard buffer,
  verify a0=0
- **Unknown syscall**: simulate ecall with a7=99, verify process is
  terminated with error

### Scheduler Tests

- **Round-robin**: with processes [idle(Ready), hello(Ready)], verify
  Schedule() alternates between them
- **Skip terminated**: with processes [idle(Ready), hello(Terminated)],
  verify Schedule() always returns PID 0
- **Only idle**: when all non-idle processes are Terminated, verify
  Schedule() returns 0 and IsIdle() returns true
- **Context switch**: verify ContextSwitch saves registers to outgoing PCB
  and loads from incoming PCB

### Timer Handler Tests

- **Timer fires**: raise interrupt 32, verify HandleTimer is called
- **Context save on timer**: set known register values, fire timer, verify
  registers saved to current process PCB
- **Process switch on timer**: with 2 Ready processes, fire timer, verify
  CurrentProcess changed

### Keyboard Handler Tests

- **Keystroke arrives**: raise interrupt 33 with 'A' in the keyboard port,
  verify 'A' appended to KeyboardBuffer
- **Multiple keystrokes**: inject 'H', 'i', verify buffer contains "Hi"

### Integration Tests

- **Full boot**: call Boot(), verify process table has 2 processes, ISRs
  registered, timer enabled
- **Boot to hello world**: run boot sequence through enough cycles for
  hello-world to complete. Verify display contains "Hello World"
- **Boot to idle**: after hello-world terminates, verify IsIdle()=true
- **IsIdle**: verify IsIdle()=false during hello-world, true after it exits

### Memory Manager Tests

- **FindRegion**: create regions, find by address, verify correct region
  returned
- **FindRegion miss**: query address in unmapped gap, verify nil returned
- **CheckAccess**: verify kernel PID can access kernel memory, process PID
  can access its own memory
- **CheckAccess denied**: verify process PID cannot access another process's
  memory (future enforcement)

## Future Extensions

- **Priority scheduling**: assign priority levels, higher-priority processes
  preempt lower ones
- **Time slicing**: variable-length time slices based on process behavior
  (I/O-bound processes get shorter slices)
- **Virtual memory**: page tables, TLB, demand paging, page fault handler
- **File system**: simple in-memory file system for persistent storage
- **Inter-process communication**: pipes, message queues, shared memory
- **Signals**: SIGTERM, SIGKILL, SIGINT for process control
- **Dynamic process creation**: fork() and exec() system calls
- **Process accounting**: track CPU time, memory usage per process
