# S06 — System Integration (SystemBoard)

## Overview

The SystemBoard is the top-level integration package — the actual computer.
It composes ROM/BIOS (S01), Bootloader (S02), Interrupt Handler (S03), OS
Kernel (S04), Display (S05), and a D05 Core with RISC-V decoder into a
complete simulated computer. It provides the power-on-to-hello-world boot
trace and host keystroke injection.

Every other package in the S-series and D-series is a component. The
SystemBoard is the thing you plug in and turn on. It is the entry point for
anyone who wants to see the full stack in action: call `PowerOn()`, call
`Run(100000)`, and read `DisplaySnapshot()` to see "Hello World" — proof
that logic gates, a pipeline, a cache hierarchy, a bootloader, an OS kernel,
and a display driver all work together.

**Analogy:** The SystemBoard is the actual computer — the physical box on
your desk. The CPU (D05 Core), RAM, hard drive (DiskImage), monitor (Display),
keyboard, and motherboard firmware (BIOS) are all components inside it. You
do not interact with those components directly — you press the power button
and the SystemBoard orchestrates everything.

## Layer Position

```
Host Program (Go test, CLI tool, web visualizer)
│
├── SystemBoard (S06) ← YOU ARE HERE — the top-level entry point
│     │
│     ├── PowerOn()             → start the boot sequence
│     ├── Run(maxCycles)        → execute until idle or budget
│     ├── Step()                → execute one CPU cycle
│     ├── InjectKeystroke('A')  → simulate keyboard input
│     ├── DisplaySnapshot()     → read the screen
│     └── GetBootTrace()        → full boot event log
│
├── Subcomponents (all instantiated and wired by SystemBoard):
│     ├── SparseMemory          → full 32-bit address space
│     ├── ROM / BIOS (S01)      → hardware initialization
│     ├── Bootloader (S02)      → kernel loading
│     ├── InterruptController (S03) → event delivery
│     ├── Kernel (S04)          → process management, syscalls
│     ├── DisplayDriver (S05)   → text framebuffer
│     ├── Core (D05)            → CPU pipeline + RISC-V decoder
│     ├── Cache Hierarchy (D01) → L1I, L1D, L2
│     ├── Branch Predictor (D02)→ direction + BTB
│     └── Hazard Detection (D03)→ forwarding + stalls
│
└── DiskImage                   → pre-loaded with kernel + user programs
```

**Depends on:** Every S-series and D-series package (this is the integration
layer)

**Used by:** Host programs (tests, CLI tools, web visualizers, educational
demos)

## Key Concepts

### The Boot Trace

The boot trace is the complete record of the system's journey from power-on
to idle. It captures every phase transition with cycle counts, state
snapshots, and human-readable descriptions. This is the primary output for
educational and debugging purposes.

```
Boot Trace Example:
──────────────────

[Cycle 0]     Phase: PowerOn
              PC: 0xFFFE0000 (ROM base)
              Event: "System powered on, PC set to ROM entry"

[Cycle 1-150] Phase: BIOS
              Events:
                "IDT populated: 256 entries at 0x00000000"
                "Memory test: 1048576 bytes OK"
                "Boot protocol written to 0x00001000"
                "Jump to bootloader at 0x00010000"

[Cycle 151-6300] Phase: Bootloader
              Events:
                "Boot protocol validated (magic = 0xB007CAFE)"
                "Copying kernel: 4096 bytes from disk to 0x00020000"
                "Copy complete: 1024 words transferred"
                "Stack pointer set to 0x0006FFF0"
                "Jump to kernel at 0x00020000"

[Cycle 6301-6500] Phase: KernelInit
              Events:
                "Memory manager initialized: 6 regions"
                "ISRs registered: timer(32), keyboard(33), syscall(128)"
                "Idle process created: PID 0 at 0x00030000"
                "Hello-world loaded: PID 1 at 0x00040000"
                "Scheduler started, running PID 1"

[Cycle 6501-6550] Phase: UserProgram
              Events:
                "PID 1: sys_write(1, 0x00040030, 12)"
                "Display: 'Hello World\\n' written to framebuffer"
                "PID 1: sys_exit(0)"
                "PID 1 terminated, exit code 0"

[Cycle 6551+] Phase: Idle
              Events:
                "Only idle process (PID 0) running"
                "System idle — all user programs complete"
```

### Boot Phases

The system progresses through a strict sequence of phases:

```
┌──────────┐     ┌──────┐     ┌────────────┐     ┌────────────┐     ┌─────────────┐     ┌──────┐
│ PowerOn  │────►│ BIOS │────►│ Bootloader │────►│ KernelInit │────►│ UserProgram │────►│ Idle │
└──────────┘     └──────┘     └────────────┘     └────────────┘     └─────────────┘     └──────┘

PowerOn:      PC set to ROM, all memory cleared, components instantiated.
BIOS:         IDT populated, hardware tested, boot protocol written.
Bootloader:   Kernel copied from disk to RAM, stack set, jump to kernel.
KernelInit:   Processes created, ISRs registered, scheduler started.
UserProgram:  Hello-world runs, calls sys_write, calls sys_exit.
Idle:         Only idle process remains. System has completed all work.

Each phase transition is detected automatically by the SystemBoard
based on the PC value and kernel state.
```

### System Snapshot

A SystemSnapshot captures the complete state of the system at a single
point in time. It is used in boot trace events and can be requested at
any time by the host program.

```
System Snapshot:
┌─────────────────────────────────────────────────┐
│ Cycle: 6520                                      │
│                                                  │
│ CPU State:                                       │
│   PC: 0x00040018                                 │
│   x10 (a0) = 1       (fd = stdout)              │
│   x11 (a1) = 0x40030 (buffer address)           │
│   x12 (a2) = 12      (length)                   │
│   x17 (a7) = 1       (sys_write)                │
│                                                  │
│ Display:                                         │
│   Row 0: "Hello World"                           │
│   Cursor: (1, 0)                                 │
│                                                  │
│ Processes:                                       │
│   PID 0: idle     (Ready)                        │
│   PID 1: hello    (Running, PC=0x00040018)       │
│                                                  │
│ Pipeline:                                        │
│   IF:  0x00040018  ecall                         │
│   ID:  0x00040014  li a7, 1                      │
│   EX:  0x00040010  li a2, 12                     │
│   MEM: 0x0004000C  li a1, 0x40030                │
│   WB:  0x00040008  li a0, 1                      │
│                                                  │
│ Cache:                                           │
│   L1I: 45 hits, 3 misses (93.8%)                │
│   L1D: 12 hits, 1 miss  (92.3%)                 │
│                                                  │
│ Interrupts:                                      │
│   Enabled: true                                  │
│   Pending: []                                    │
└─────────────────────────────────────────────────┘
```

### Keystroke Injection

The host program (test harness, CLI tool) can simulate keyboard input by
calling `InjectKeystroke`. This writes the character to a keyboard I/O port
at `0xFFFC0000` and raises interrupt 33 (keyboard). The kernel's keyboard
handler reads the port and appends the character to its buffer.

```
Host Program                SystemBoard               Kernel
────────────                ───────────               ──────
InjectKeystroke('A') ──►  memory[0xFFFC0000] = 'A'
                          InterruptCtrl.Raise(33) ──► HandleKeyboard()
                                                      reads 0xFFFC0000
                                                      KeyboardBuffer += 'A'
```

This enables interactive demos: type a character in the host, see it appear
in the simulated system's keyboard buffer (and eventually on screen if the
kernel echoes input).

### Sparse Memory

The SystemBoard uses a sparse memory implementation for the full 32-bit
address space (4 GB). Rather than allocating a 4 GB array, it allocates
memory pages on demand. Only pages that are actually written to consume
real host memory.

```
Sparse Memory:
  Page size: 4096 bytes (4 KB)
  Total pages possible: 4 GB / 4 KB = 1,048,576 pages
  Typical pages allocated: ~50 (for kernel, processes, framebuffer)
  Actual host memory used: ~200 KB instead of 4 GB

  Address → Page number = address >> 12
          → Page offset = address & 0xFFF

  Read from unallocated page → returns 0x00 (clean memory)
  Write to unallocated page  → allocates page, then writes
```

## Public API

```go
// --- Boot Phases ---

type BootPhase int

const (
    PhasePowerOn    BootPhase = iota  // System just powered on
    PhaseBIOS                         // BIOS executing POST, IDT setup
    PhaseBootloader                   // Bootloader copying kernel
    PhaseKernelInit                   // Kernel initializing subsystems
    PhaseUserProgram                  // User program(s) running
    PhaseIdle                         // All user programs terminated
)

// String returns a human-readable name for the phase.
func (p BootPhase) String() string

// --- Boot Events ---

type BootEvent struct {
    Phase       BootPhase       // Which phase this event belongs to
    Cycle       int             // CPU cycle when this event occurred
    Description string          // Human-readable description
    Snapshot    SystemSnapshot  // Full system state at this point
}

type BootTrace struct {
    Events []BootEvent
}

// Phases returns the distinct phases that occurred, in order.
func (t *BootTrace) Phases() []BootPhase

// EventsInPhase returns all events belonging to the given phase.
func (t *BootTrace) EventsInPhase(phase BootPhase) []BootEvent

// TotalCycles returns the cycle count of the last event.
func (t *BootTrace) TotalCycles() int

// PhaseStartCycle returns the cycle at which the given phase began.
func (t *BootTrace) PhaseStartCycle(phase BootPhase) int

// --- System Snapshot ---

type CPUState struct {
    PC        uint32
    Registers [32]uint32
    Cycle     int
}

type ProcessInfo struct {
    PID   int
    Name  string
    State ProcessState
    PC    uint32
}

type PipelineSnapshot struct {
    IF  string  // Instruction in Fetch stage (disassembled)
    ID  string  // Instruction in Decode stage
    EX  string  // Instruction in Execute stage
    MEM string  // Instruction in Memory stage
    WB  string  // Instruction in Writeback stage
}

type CacheStats struct {
    L1IHits   int
    L1IMisses int
    L1DHits   int
    L1DMisses int
    L2Hits    int
    L2Misses  int
}

type InterruptSnapshot struct {
    Enabled bool
    Pending []int
}

type SystemSnapshot struct {
    Cycle          int
    Phase          BootPhase
    CPUState       CPUState
    DisplayContent DisplaySnapshot
    ProcessTable   []ProcessInfo
    PipelineState  PipelineSnapshot
    CacheStats     CacheStats
    InterruptState InterruptSnapshot
}

// --- System Configuration ---

type SystemConfig struct {
    MemorySize       int               // Total addressable RAM (default: 1 MB)
    CoreConfig       CoreConfig        // D05 Core settings
    DisplayConfig    DisplayConfig     // S05 display settings
    BIOSConfig       BIOSConfig        // S01 BIOS settings
    BootloaderConfig BootloaderConfig  // S02 bootloader settings
    KernelConfig     KernelConfig      // S04 kernel settings
    UserProgram      []byte            // Binary for the user program (hello-world)
}

// DefaultSystemConfig returns a configuration with sensible defaults for the
// hello-world demo. All addresses, sizes, and intervals are pre-configured
// so that PowerOn() + Run(100000) produces "Hello World" on the display.
func DefaultSystemConfig() SystemConfig

// --- SystemBoard ---

type SystemBoard struct {
    Config        SystemConfig
    Core          *Core               // D05 Core with pipeline + RISC-V decoder
    Memory        *SparseMemory       // Full 32-bit address space
    ROM           *ROM                // S01 ROM/BIOS
    DiskImage     *DiskImage          // S02 simulated storage
    Display       *DisplayDriver      // S05 text framebuffer
    InterruptCtrl *InterruptController // S03 interrupt handling
    Kernel        *Kernel             // S04 OS kernel
    BootTrace     *BootTrace          // Accumulated boot events
    Powered       bool                // True after PowerOn()
    Cycle         int                 // Current cycle count
    CurrentPhase  BootPhase           // Current boot phase
}

// NewSystemBoard creates a system board with all components instantiated
// but not yet powered on. Components are wired together (memory shared,
// interrupt controller connected to core, display mapped to memory).
func NewSystemBoard(config SystemConfig) *SystemBoard

// PowerOn initializes all components and begins the boot sequence.
// Sets PC to ROM base address. The BIOS begins executing on the next Step().
func (b *SystemBoard) PowerOn()

// Step executes exactly one CPU cycle and returns the current system snapshot.
// Records boot events when phase transitions occur.
func (b *SystemBoard) Step() SystemSnapshot

// Run executes cycles until the system is idle (only idle process running)
// or the maxCycles budget is exhausted. Returns the complete boot trace.
func (b *SystemBoard) Run(maxCycles int) BootTrace

// InjectKeystroke simulates a keyboard press by writing the character to the
// keyboard I/O port and raising interrupt 33.
func (b *SystemBoard) InjectKeystroke(char byte)

// DisplaySnapshot returns the current state of the text display.
func (b *SystemBoard) DisplaySnapshot() DisplaySnapshot

// GetBootTrace returns the accumulated boot trace so far.
func (b *SystemBoard) GetBootTrace() BootTrace

// GetSnapshot returns the current system snapshot without advancing a cycle.
func (b *SystemBoard) GetSnapshot() SystemSnapshot

// IsIdle returns true when the kernel reports that only the idle process
// remains (all user programs have terminated).
func (b *SystemBoard) IsIdle() bool

// GetCycleCount returns the total number of CPU cycles executed since PowerOn.
func (b *SystemBoard) GetCycleCount() int

// GetCurrentPhase returns the current boot phase.
func (b *SystemBoard) GetCurrentPhase() BootPhase
```

## Data Structures

### Sparse Memory

```go
// SparseMemory implements a 32-bit address space using on-demand page allocation.
type SparseMemory struct {
    pages    map[uint32][]byte  // pageNumber → 4096-byte page
    pageSize uint32             // Always 4096
}

func NewSparseMemory() *SparseMemory
func (m *SparseMemory) ReadByte(address uint32) byte
func (m *SparseMemory) WriteByte(address uint32, value byte)
func (m *SparseMemory) ReadWord(address uint32) uint32       // Little-endian
func (m *SparseMemory) WriteWord(address uint32, value uint32)
func (m *SparseMemory) ReadSlice(address uint32, length int) []byte
func (m *SparseMemory) WriteSlice(address uint32, data []byte)
func (m *SparseMemory) AllocatedPages() int  // Number of pages in use
```

### Address Space Constants

```go
const (
    // ROM
    ROMBase         = 0xFFFE0000  // BIOS ROM (read-only)
    ROMSize         = 0x00020000  // 128 KB

    // Boot infrastructure
    IDTBase         = 0x00000000  // Interrupt Descriptor Table
    BootProtocolAddr = 0x00001000 // BIOS → Bootloader communication
    BootloaderBase  = 0x00010000  // Bootloader code
    KernelBase      = 0x00020000  // Kernel code + data

    // Process memory
    IdleProcessBase = 0x00030000  // PID 0
    UserProcessBase = 0x00040000  // PID 1 (hello-world)
    KernelStackTop  = 0x0006FFF0  // Kernel stack (grows down)

    // Disk image (memory-mapped)
    DiskMappedBase  = 0x10000000

    // I/O devices
    FramebufferBase = 0xFFFB0000  // Display (80x25x2 = 4000 bytes)
    KeyboardPort    = 0xFFFC0000  // Keyboard I/O (1 byte)
)
```

### Default Configuration Values

```go
func DefaultSystemConfig() SystemConfig {
    return SystemConfig{
        MemorySize:    1024 * 1024,  // 1 MB
        CoreConfig:    DefaultCoreConfig(),
        DisplayConfig: DefaultDisplayConfig(),  // 80x25 VGA text mode
        BIOSConfig:    DefaultBIOSConfig(),
        BootloaderConfig: DefaultBootloaderConfig(),
        KernelConfig: KernelConfig{
            TimerInterval: 100,
            MaxProcesses:  16,
        },
        UserProgram: GenerateHelloWorldBinary(),
    }
}
```

## Test Strategy

### Power-On Tests

- **NewSystemBoard**: create with DefaultSystemConfig, verify all components
  are non-nil
- **PowerOn**: call PowerOn(), verify Powered=true, PC set to ROM base,
  CurrentPhase=PhasePowerOn
- **Double PowerOn**: call PowerOn() twice, verify no crash (idempotent or
  error)

### Phase Transition Tests

- **BIOS phase**: run until PC leaves ROM region, verify phase changed to
  PhaseBIOS then PhaseBootloader
- **Bootloader phase**: verify phase changes when PC enters bootloader region
  and then kernel region
- **KernelInit phase**: verify phase changes when kernel Boot() executes
- **UserProgram phase**: verify phase changes when a user process starts
  running
- **Idle phase**: verify phase changes when all user processes terminate

### Boot-to-Hello-World Integration Test

This is THE critical test — the proof that the entire stack works:

```
1. board := NewSystemBoard(DefaultSystemConfig())
2. board.PowerOn()
3. trace := board.Run(100000)
4. Verify trace contains all 6 phases in order
5. display := board.DisplaySnapshot()
6. Verify display.Contains("Hello World")
7. Verify board.IsIdle()
```

### Keystroke Injection Tests

- **Inject and handle**: boot to idle, inject 'A', step, verify kernel's
  KeyboardBuffer contains 'A'
- **Interrupt raised**: inject keystroke, verify interrupt 33 was raised
- **Multiple keystrokes**: inject 'H', 'i', step after each, verify buffer
  contains "Hi"

### Display Tests

- **Display after boot**: run to completion, verify DisplaySnapshot() is not
  empty
- **Hello World visible**: run to completion, verify
  DisplaySnapshot().Contains("Hello World")
- **Snapshot consistency**: take two snapshots at the same cycle, verify they
  are identical

### Boot Trace Tests

- **Phase ordering**: verify phases appear in order: PowerOn, BIOS,
  Bootloader, KernelInit, UserProgram, Idle
- **Cycle counts**: verify each phase starts at a later cycle than the
  previous one
- **Events have descriptions**: verify every BootEvent has a non-empty
  Description
- **TotalCycles**: verify TotalCycles() returns the cycle of the last event
- **PhaseStartCycle**: verify PhaseStartCycle(PhaseBIOS) < PhaseStartCycle(PhaseBootloader)

### Snapshot Tests

- **CPU state**: verify snapshot contains valid PC and register values
- **Process table**: verify snapshot lists processes with correct PIDs and states
- **Pipeline state**: verify snapshot shows instructions in pipeline stages
- **Cache stats**: verify cache hit/miss counts are non-negative and consistent
- **Interrupt state**: verify interrupt enabled flag and pending list

### Sparse Memory Tests

- **Clean read**: read from unallocated address, verify returns 0
- **Write and read**: write byte, read back, verify identical
- **Word read/write**: write uint32, read back, verify correct (little-endian)
- **Slice operations**: write byte slice, read back, verify identical
- **Page allocation**: write to 3 addresses in different pages, verify
  AllocatedPages()==3
- **Large address**: write to 0xFFFB0000 (framebuffer), verify works

### Full Trace Verification Test

This test verifies that the complete execution chain is visible:

```
1. Logic gates: ALU operations visible in pipeline EX stage snapshots
2. Cache: L1I hits/misses tracked during instruction fetch
3. Pipeline: instructions flowing through IF → ID → EX → MEM → WB
4. Branch predictor: predictions visible in pipeline state
5. Hazard detection: stalls or forwarding visible in pipeline state
6. Interrupts: ecall triggers interrupt 128, context save/restore
7. Syscall: sys_write dispatched, display driver called
8. Framebuffer: bytes written to memory-mapped display region
9. Display: "Hello World" readable via DisplaySnapshot()
```

### Default Config Tests

- **DefaultSystemConfig**: verify returns a valid config with all fields
  populated
- **Default produces working system**: NewSystemBoard(DefaultSystemConfig()),
  PowerOn(), Run(100000), verify "Hello World" displayed
- **Cycle budget**: verify Run(100000) completes within budget (system does
  not need more than 100000 cycles)

### Error Handling Tests

- **Step before PowerOn**: call Step() without PowerOn(), verify error or
  no-op
- **Run before PowerOn**: call Run() without PowerOn(), verify error or
  no-op
- **Inject before boot**: inject keystroke before boot, verify no crash
- **Excessive cycles**: Run(0), verify returns empty trace (no cycles to run)

## Future Extensions

- **Multi-core**: instantiate multiple D05 Cores sharing L3 cache and memory,
  with inter-processor interrupts (IPI) for cross-core communication
- **DMA controller**: direct memory access for fast disk-to-memory transfers
  (bypass CPU for bulk copies)
- **Network interface**: simulated NIC with packet send/receive for networking
  demos
- **Sound card**: memory-mapped audio buffer for simple beep/tone generation
- **Boot device selection**: support multiple disk images, BIOS selects boot
  device
- **Power management**: sleep states, wake-on-interrupt
- **Performance counters**: cycle-accurate tracking of IPC, cache miss rate,
  branch misprediction rate, pipeline stall cycles
- **Web visualizer**: real-time web-based display showing pipeline state, memory
  contents, and display output as the system executes
- **Checkpoint/restore**: save complete system state to disk, restore later
  (like VM snapshots)
