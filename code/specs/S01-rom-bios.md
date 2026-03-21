# S01 — ROM & BIOS (Firmware)

## Overview

The ROM & BIOS package implements the very first code that runs when the
simulated computer powers on. ROM (Read-Only Memory) is a memory region at
address 0xFFFF0000 that cannot be modified by normal program execution. It
contains the BIOS firmware — a RISC-V program that initializes the hardware
and hands off control to the bootloader.

On power-on, the CPU's program counter (PC) is set to 0xFFFF0000 — the start
of ROM. The firmware executes and performs hardware initialization before any
operating system code runs. This is the bridge between raw hardware (the D-layer
packages) and the software stack (bootloader, kernel, applications).

Every real computer has this moment: the instant after the power button is
pressed, before any familiar software appears. The BIOS is the invisible
custodian that makes that transition possible.

## Layer Position

```
System Board (S06)
├── OS Kernel (S04)
├── Bootloader (S02) ← BIOS jumps here after initialization
├── ROM / BIOS (S01) ← YOU ARE HERE
│
════════════════════════ hardware / software boundary ════════════
D05 Core (executes firmware code)
├── D04 Pipeline
├── D03 Hazard Detection
├── D02 Branch Predictor
└── D01 Cache
```

**Depends on:** `D05 Core` (the CPU that executes the firmware instructions)
**Used by:** `S02 Bootloader` (BIOS jumps to the bootloader entry point),
`S06 System Board` (wires ROM into the memory map)

## Key Concepts

### ROM = Read-Only Memory

Real computers have a ROM chip soldered to the motherboard. It contains
firmware that cannot be modified by normal program execution. In our
simulation, ROM is a memory region where writes are silently ignored (or panic
in debug mode).

**Analogy:** ROM is like a recipe card laminated in plastic. You can read it
any number of times, but you cannot write on it. The recipe was "burned in"
at the factory (or in our case, when the ROM object is constructed).

```
Normal RAM:                         ROM:

  Read  → returns data              Read  → returns data
  Write → stores new data           Write → silently ignored
                                            (data unchanged)

Memory map:
┌──────────────────┐ 0xFFFF_FFFF
│    ROM (64 KB)   │ 0xFFFF_0000  ← PC starts here on power-on
├──────────────────┤
│   Framebuffer    │ 0xFFFB_0000
├──────────────────┤
│       ...        │
├──────────────────┤
│    Bootloader    │ 0x0001_0000  ← BIOS jumps here
├──────────────────┤
│   HardwareInfo   │ 0x0000_1000  ← BIOS writes hardware info here
├──────────────────┤
│       IDT        │ 0x0000_0000  ← BIOS writes interrupt table here
└──────────────────┘
```

### BIOS = The First Program

BIOS stands for Basic Input/Output System. It is the first code that runs —
before any operating system, before any bootloader, before anything the user
would recognize as "software." Its jobs:

1. **Probe memory** to discover how much RAM is available
2. **Initialize the Interrupt Descriptor Table (IDT)** with default handlers
3. **Write hardware information** to a known address (the HardwareInfo struct)
4. **Jump to the bootloader** entry point

**Analogy:** BIOS is like the building manager who arrives first thing in the
morning. They turn on the lights, check all the rooms, verify the heating
works, write a status report and leave it on the front desk, then unlock the
front door for the tenants (the OS). The tenants never see the building
manager, but nothing works without them.

```
Power-on sequence:

  ┌─────────┐     ┌──────────┐     ┌────────────┐     ┌──────────┐
  │ Power   │────→│  BIOS    │────→│ Bootloader │────→│  OS      │
  │ On      │     │ (S01)    │     │ (S02)      │     │ Kernel   │
  │         │     │          │     │            │     │ (S04)    │
  │ PC =    │     │ Probes   │     │ Loads      │     │ Runs     │
  │ 0xFFFF  │     │ memory,  │     │ kernel     │     │ programs │
  │ 0000    │     │ sets up  │     │ from disk  │     │          │
  │         │     │ IDT,     │     │ into RAM   │     │          │
  │         │     │ jumps to │     │            │     │          │
  │         │     │ 0x10000  │     │            │     │          │
  └─────────┘     └──────────┘     └────────────┘     └──────────┘
```

### Firmware as RISC-V Machine Code

The BIOS firmware is a RISC-V program. Rather than depending on the assembler
package (which would create a circular dependency — the assembler might need
the system to be running), each firmware routine is generated programmatically
using instruction encoding helpers (like `EncodeAddi`, `EncodeLui`, etc.).

The assembly source is included as comments for documentation, so a reader
can understand both the machine code and its human-readable equivalent:

```
Address    Machine Code    Assembly               Comment
────────────────────────────────────────────────────────────────
0xFFFF0000  0x000012B7     lui x5, 0x00001        ; x5 = 0x00001000 (HardwareInfo base)
0xFFFF0004  0x00028293     addi x5, x5, 0         ; (no offset needed)
0xFFFF0008  0xDEADB337     lui x6, 0xDEADB        ; x6 = test pattern (upper)
0xFFFF000C  0xEEF30313     addi x6, x6, 0xEEF     ; x6 = 0xDEADBEEF
...
```

This approach has two benefits:
1. Zero external dependencies — the firmware is self-contained
2. Educational value — the reader sees exactly how high-level operations
   (like "write 0xDEADBEEF to address X") translate to individual RISC-V
   instructions

## Public API

```go
package firmware

// ═══════════════════════════════════════════════════════════════
// ROM — Read-Only Memory
// ═══════════════════════════════════════════════════════════════

// ROMConfig defines the read-only memory region.
//
// The default configuration places ROM at the top of the 32-bit address
// space (0xFFFF0000), which is the conventional reset vector for many
// architectures. The CPU's program counter is set to this address on
// power-on, so whatever code lives here executes first.
type ROMConfig struct {
    BaseAddress uint32  // Start address (default: 0xFFFF0000)
    Size        int     // Size in bytes (default: 65536 = 64KB)
}

// ROM represents a read-only memory region.
//
// Once created with a firmware image, the contents cannot be changed.
// Write operations are silently ignored — this models the behavior of
// real ROM chips, which are programmed at the factory and cannot be
// rewritten by the CPU.
//
// Example:
//
//     bios := NewBIOSFirmware(DefaultBIOSConfig())
//     rom := NewROM(DefaultROMConfig(), bios.Generate())
//
//     // Reading works normally:
//     firstByte := rom.Read(0xFFFF0000)
//     firstWord := rom.ReadWord(0xFFFF0000)
//
//     // Writing is silently ignored:
//     rom.Write(0xFFFF0000, 0xFF)
//     // rom.Read(0xFFFF0000) still returns the original firmware byte
//
type ROM struct {
    Config  ROMConfig
    data    []byte
}

// NewROM creates a ROM loaded with the given firmware bytes.
// The firmware is copied into internal storage — the caller's slice
// is not retained. If len(firmware) < config.Size, the remaining
// bytes are zero-filled. If len(firmware) > config.Size, it panics.
func NewROM(config ROMConfig, firmware []byte) *ROM

// Read returns a single byte from the given absolute address.
// The address must fall within [BaseAddress, BaseAddress+Size).
// Out-of-range addresses return 0.
func (r *ROM) Read(address uint32) byte

// ReadWord returns a 32-bit little-endian word starting at the given
// absolute address. This is the primary access pattern since RISC-V
// instructions are 32 bits wide.
func (r *ROM) ReadWord(address uint32) uint32

// Write attempts to write a byte to ROM. Since ROM is read-only,
// this operation is silently ignored. In debug builds, it may log
// a warning or panic to help catch firmware bugs.
func (r *ROM) Write(address uint32, value byte)

// Size returns the total size of the ROM in bytes.
func (r *ROM) Size() int

// ═══════════════════════════════════════════════════════════════
// HardwareInfo — Boot Protocol Structure
// ═══════════════════════════════════════════════════════════════

// HardwareInfo is the structure written by BIOS at address 0x00001000.
// The bootloader and kernel read this to learn about the hardware
// configuration. Think of it as the "status report" the building
// manager leaves on the front desk.
//
// Memory layout (all fields are little-endian uint32, 28 bytes total):
//
//     Offset  Field             Default
//     ──────────────────────────────────────────
//     0x00    MemorySize        (probed at boot)
//     0x04    DisplayColumns    80
//     0x08    DisplayRows       25
//     0x0C    FramebufferBase   0xFFFB0000
//     0x10    IDTBase           0x00000000
//     0x14    IDTEntries        256
//     0x18    BootloaderEntry   0x00010000
//
type HardwareInfo struct {
    MemorySize      uint32  // Total RAM in bytes (discovered by memory probe)
    DisplayColumns  uint32  // Text display width (default: 80)
    DisplayRows     uint32  // Text display height (default: 25)
    FramebufferBase uint32  // Framebuffer start address (default: 0xFFFB0000)
    IDTBase         uint32  // IDT start address (default: 0x00000000)
    IDTEntries      uint32  // Number of IDT entries (default: 256)
    BootloaderEntry uint32  // Where to jump after BIOS (default: 0x00010000)
}

// ═══════════════════════════════════════════════════════════════
// BIOS Firmware Generator
// ═══════════════════════════════════════════════════════════════

// BIOSFirmware generates the BIOS firmware as RISC-V machine code.
//
// Rather than writing assembly by hand and running it through an
// assembler, the firmware is constructed programmatically. Each
// instruction is emitted using encoding helpers (EncodeLui,
// EncodeAddi, etc.) that produce the correct 32-bit machine code.
//
// This keeps the firmware package self-contained with zero external
// dependencies, while the assembly comments make it readable.
//
// Example:
//
//     config := BIOSConfig{
//         MemorySize:      64 * 1024 * 1024,  // 64 MB
//         DisplayColumns:  80,
//         DisplayRows:     25,
//         FramebufferBase: 0xFFFB0000,
//         BootloaderEntry: 0x00010000,
//     }
//     bios := NewBIOSFirmware(config)
//     machineCode := bios.Generate()
//
//     // Load into ROM:
//     rom := NewROM(DefaultROMConfig(), machineCode)
//
//     // For debugging, get annotated output:
//     annotated := bios.GenerateWithComments()
//     for _, inst := range annotated {
//         fmt.Printf("0x%08X  %08X  %-30s  ; %s\n",
//             inst.Address, inst.MachineCode, inst.Assembly, inst.Comment)
//     }
//
type BIOSFirmware struct {
    Config BIOSConfig
}

// BIOSConfig controls what the BIOS firmware will do during
// initialization. These values determine the contents of the
// HardwareInfo struct and the final jump target.
type BIOSConfig struct {
    MemorySize      int     // RAM size to report (or 0 to probe)
    DisplayColumns  int     // Text display columns (default: 80)
    DisplayRows     int     // Text display rows (default: 25)
    FramebufferBase uint32  // Framebuffer address (default: 0xFFFB0000)
    BootloaderEntry uint32  // Where BIOS jumps after init (default: 0x00010000)
}

// NewBIOSFirmware creates a firmware generator with the given config.
func NewBIOSFirmware(config BIOSConfig) *BIOSFirmware

// Generate returns the BIOS firmware as raw RISC-V machine code bytes.
// The returned byte slice can be loaded directly into a ROM.
func (b *BIOSFirmware) Generate() []byte

// GenerateWithComments returns the firmware as annotated instructions.
// Each instruction includes its address, machine code, assembly text,
// and a human-readable comment explaining its role in the boot sequence.
// This is invaluable for debugging and education.
func (b *BIOSFirmware) GenerateWithComments() []AnnotatedInstruction

// DefaultBIOSConfig returns a sensible default configuration:
//   MemorySize: 0 (probe), DisplayColumns: 80, DisplayRows: 25,
//   FramebufferBase: 0xFFFB0000, BootloaderEntry: 0x00010000.
func DefaultBIOSConfig() BIOSConfig

// DefaultROMConfig returns the default ROM configuration:
//   BaseAddress: 0xFFFF0000, Size: 65536 (64KB).
func DefaultROMConfig() ROMConfig
```

## Data Structures

### AnnotatedInstruction

Each instruction in the firmware can be represented with full context,
making the boot sequence readable even to someone unfamiliar with RISC-V:

```go
// AnnotatedInstruction pairs a machine code instruction with its
// human-readable assembly and a comment explaining its purpose.
//
// Example:
//
//     AnnotatedInstruction{
//         Address:     0xFFFF0000,
//         MachineCode: 0x000012B7,
//         Assembly:    "lui x5, 0x00001",
//         Comment:     "Step 3: Load HardwareInfo base address (0x00001000) into x5",
//     }
//
type AnnotatedInstruction struct {
    Address     uint32   // Memory address where this instruction lives
    MachineCode uint32   // Raw 32-bit RISC-V instruction
    Assembly    string   // Human-readable assembly (e.g., "lui x5, 0xFFFB0")
    Comment     string   // What this instruction does in the boot sequence
}
```

### IDT Entry Format

The Interrupt Descriptor Table lives at address 0x00000000. Each entry is
8 bytes and describes one interrupt handler:

```
IDT Entry (8 bytes):
┌──────────────────────────────┬──────────────────────────────┐
│   ISR Address (4 bytes)      │   Flags (4 bytes)            │
│   (where to jump)            │   (handler type, privilege)  │
└──────────────────────────────┴──────────────────────────────┘

IDT layout in memory (starting at 0x00000000):
┌─────────┬────────────────────────────────────────────┐
│ Entry   │ Purpose                                    │
├─────────┼────────────────────────────────────────────┤
│ 0-31    │ CPU exception handlers (divide by zero,    │
│         │ invalid opcode, page fault, etc.)           │
│         │ → All point to default_fault_handler        │
│         │   (an infinite loop at 0x00000800)          │
├─────────┼────────────────────────────────────────────┤
│ 32      │ Timer interrupt ISR stub                   │
│         │ → Points to timer_isr at 0x00000808        │
├─────────┼────────────────────────────────────────────┤
│ 33      │ Keyboard interrupt ISR stub                │
│         │ → Points to keyboard_isr at 0x00000810     │
├─────────┼────────────────────────────────────────────┤
│ 34-127  │ Reserved (point to default_fault_handler)  │
├─────────┼────────────────────────────────────────────┤
│ 128     │ System call ISR stub                       │
│         │ → Points to syscall_isr at 0x00000818      │
├─────────┼────────────────────────────────────────────┤
│ 129-255 │ Reserved (point to default_fault_handler)  │
└─────────┴────────────────────────────────────────────┘

ISR stubs live at 0x00000800:
┌───────────┬────────────────────────────────────────────┐
│ Address   │ Code                                       │
├───────────┼────────────────────────────────────────────┤
│ 0x800     │ default_fault_handler: j 0x800 (inf loop)  │
│ 0x808     │ timer_isr: mret (return from interrupt)     │
│ 0x810     │ keyboard_isr: mret                          │
│ 0x818     │ syscall_isr: mret                           │
└───────────┴────────────────────────────────────────────┘
```

The ISR stubs are minimal placeholders. The bootloader or kernel will
overwrite them with real handlers later. The important thing is that every
IDT entry points to *something* valid, so an unexpected interrupt does not
crash the system — it just spins in place (which is detectable by a debugger).

### Memory Probe Algorithm

The memory probe determines how much RAM is available by writing a test
pattern to progressively higher addresses:

```
Memory probe algorithm:

  test_pattern = 0xDEADBEEF
  address = 0x00100000          ; start at 1 MB (skip low memory)
  step = 0x00100000             ; check every 1 MB

  loop:
    store test_pattern → [address]
    load value ← [address]
    if value != test_pattern:
      memory_size = address     ; this address did not "stick"
      break
    address += step
    if address >= 0xFFFB0000:   ; don't probe into framebuffer/ROM
      memory_size = address
      break

  store memory_size → HardwareInfo.MemorySize
```

**Why start at 1 MB?** The low memory region (0x00000000 - 0x000FFFFF) is
reserved for the IDT, HardwareInfo struct, and ISR stubs. Probing it could
corrupt those structures.

**Why 0xDEADBEEF?** It is a distinctive pattern that is unlikely to appear
by accident in uninitialized memory. The hex digits spell "DEAD BEEF," which
makes it immediately recognizable in memory dumps — a classic systems
programming convention.

## BIOS Execution Flow (Detailed)

The BIOS firmware executes four steps in sequence. Each step is a block of
RISC-V instructions generated by `BIOSFirmware.Generate()`.

### Step 1: Memory Probe

Discover how much RAM is installed by writing and reading test patterns:

```
Registers used:
  x5  = current test address
  x6  = test pattern (0xDEADBEEF)
  x7  = value read back
  x8  = memory size result
  x9  = probe limit (0xFFFB0000)
  x10 = probe step (0x00100000 = 1 MB)

Instructions:
  lui  x5, 0x00100          ; x5 = 0x00100000 (start at 1 MB)
  lui  x6, 0xDEADB          ; x6 = 0xDEADB000
  addi x6, x6, 0xEEF        ; x6 = 0xDEADBEEF (test pattern)
  lui  x9, 0xFFFB0          ; x9 = 0xFFFB0000 (probe limit)
  lui  x10, 0x00100         ; x10 = 0x00100000 (1 MB step)

probe_loop:
  sw   x6, 0(x5)            ; write test pattern to [x5]
  lw   x7, 0(x5)            ; read it back
  bne  x6, x7, probe_done   ; if mismatch → memory ends here
  add  x5, x5, x10          ; advance by 1 MB
  blt  x5, x9, probe_loop   ; keep going if below limit

probe_done:
  mv   x8, x5               ; x8 = detected memory size
```

If `BIOSConfig.MemorySize` is non-zero, the probe is skipped and the
configured value is used directly (useful for testing).

### Step 2: IDT Initialization

Write 256 interrupt descriptor table entries starting at address 0x00000000.
First, write the ISR stub routines at 0x00000800, then populate the table:

```
Registers used:
  x5  = IDT base address (0x00000000)
  x6  = current entry address
  x7  = default handler address (0x00000800)
  x8  = entry counter
  x9  = special ISR addresses
  x11 = flags word

Step 2a: Write ISR stubs at 0x00000800
  Address 0x800: JAL x0, 0       ; default_fault_handler: infinite loop
  Address 0x808: MRET             ; timer_isr: return from interrupt
  Address 0x810: MRET             ; keyboard_isr: return
  Address 0x818: MRET             ; syscall_isr: return

Step 2b: Write IDT entries
  for entry = 0 to 255:
    if entry == 32:  isr_addr = 0x00000808  (timer)
    elif entry == 33: isr_addr = 0x00000810  (keyboard)
    elif entry == 128: isr_addr = 0x00000818  (syscall)
    else: isr_addr = 0x00000800  (default fault handler)

    store isr_addr → [idt_base + entry * 8]
    store flags    → [idt_base + entry * 8 + 4]
```

### Step 3: Write HardwareInfo

Populate the HardwareInfo struct at address 0x00001000:

```
Registers used:
  x5  = HardwareInfo base (0x00001000)
  x8  = memory size (from Step 1)

Instructions:
  lui  x5, 0x00001            ; x5 = 0x00001000
  sw   x8, 0(x5)              ; HardwareInfo.MemorySize = x8
  li   x6, 80
  sw   x6, 4(x5)              ; HardwareInfo.DisplayColumns = 80
  li   x6, 25
  sw   x6, 8(x5)              ; HardwareInfo.DisplayRows = 25
  lui  x6, 0xFFFB0
  sw   x6, 12(x5)             ; HardwareInfo.FramebufferBase = 0xFFFB0000
  sw   x0, 16(x5)             ; HardwareInfo.IDTBase = 0x00000000
  li   x6, 256
  sw   x6, 20(x5)             ; HardwareInfo.IDTEntries = 256
  lui  x6, 0x00010
  sw   x6, 24(x5)             ; HardwareInfo.BootloaderEntry = 0x00010000
```

### Step 4: Jump to Bootloader

Transfer control to the bootloader at address 0x00010000:

```
  lui  x6, 0x00010            ; x6 = 0x00010000
  jalr x0, x6, 0              ; jump to bootloader (no return)
```

Using `jalr` with `x0` as the link register means this is a one-way jump —
the BIOS does not expect to get control back. The bootloader takes over
from here.

## Test Strategy

### ROM Read-Only Tests

- **Write ignored**: load firmware into ROM, write a different byte to an
  address, read it back — verify the original firmware byte is returned
- **Read byte**: load known firmware bytes, verify `Read(addr)` returns the
  correct byte at each position
- **Read word**: load a known 32-bit value at a specific offset, verify
  `ReadWord(addr)` returns it in little-endian order
- **Out-of-range read**: read an address outside the ROM region, verify it
  returns 0
- **Boundary conditions**: read the first byte, last byte, first word, last
  word of the ROM

### ROM Configuration Tests

- **Custom base address**: create ROM at a non-default address, verify reads
  work correctly
- **Custom size**: create ROM with non-default size, verify size boundary is
  respected
- **Firmware too large**: pass firmware larger than ROM size, verify it panics
- **Firmware smaller than ROM**: pass firmware smaller than ROM, verify
  remaining bytes are zero

### BIOS Firmware Generation Tests

- **Non-empty output**: `Generate()` returns a non-empty byte slice
- **Word-aligned**: output length is a multiple of 4 (RISC-V instructions
  are 32 bits)
- **Valid instructions**: each 4-byte word decodes to a valid RISC-V
  instruction (no all-zero words except intentional NOPs)
- **Deterministic**: calling `Generate()` twice with the same config produces
  identical output
- **Configurable**: different `BIOSConfig` values produce different firmware

### BIOS Execution Integration Tests

These tests load the firmware into a simulated CPU (D05 Core) and run it:

- **IDT populated**: after BIOS executes, memory at 0x00000000-0x000007FF
  contains valid IDT entries (non-zero ISR addresses)
- **ISR stubs present**: memory at 0x00000800 contains the expected
  instruction patterns (JAL for fault handler, MRET for ISR stubs)
- **HardwareInfo written**: after BIOS executes, memory at 0x00001000
  contains the HardwareInfo struct with correct values:
  - MemorySize matches configured or probed value
  - DisplayColumns = 80
  - DisplayRows = 25
  - FramebufferBase = 0xFFFB0000
  - IDTBase = 0x00000000
  - IDTEntries = 256
  - BootloaderEntry = 0x00010000
- **Jump to bootloader**: after BIOS executes, the program counter ends up
  at the bootloader entry address (0x00010000)
- **Memory probe accuracy**: configure simulated RAM of various sizes
  (1 MB, 16 MB, 64 MB, 256 MB), run BIOS, verify the probed MemorySize
  matches the actual RAM size

### Annotated Output Tests

- **Coverage**: `GenerateWithComments()` returns one entry per instruction in
  `Generate()` — no instructions are missing annotations
- **Address continuity**: addresses in the annotated output increase by 4
  for each instruction and start at the ROM base address
- **Machine code match**: the `MachineCode` field in each `AnnotatedInstruction`
  matches the corresponding 4 bytes in `Generate()` output
- **Non-empty annotations**: every instruction has a non-empty `Assembly`
  string and a non-empty `Comment` string
- **Human-readable assembly**: assembly strings contain recognizable RISC-V
  mnemonics (lui, addi, sw, lw, bne, jal, jalr, etc.)

### Edge Case Tests

- **Zero-size RAM**: what happens if the memory probe finds no usable RAM?
  BIOS should still write HardwareInfo with MemorySize = 0 and jump to
  bootloader (let the bootloader handle the error)
- **Maximum RAM**: probe with RAM extending up to the framebuffer boundary
  (0xFFFB0000), verify it does not probe into ROM/framebuffer space
- **Default config**: `DefaultBIOSConfig()` and `DefaultROMConfig()` produce
  valid firmware that runs to completion

## Future Extensions

- **Hardware self-test (POST)**: verify each hardware component responds
  correctly, display error codes on failure
- **Serial console output**: write boot messages to a simulated UART for
  debugging ("Memory: 64 MB OK", "IDT: 256 entries", "Jumping to 0x10000")
- **BIOS settings**: configurable boot device priority, display resolution,
  timer frequency — stored in simulated NVRAM (battery-backed)
- **UEFI-style firmware**: replace BIOS with a more modern firmware interface
  that provides runtime services to the OS
- **Secure boot**: verify the bootloader's cryptographic signature before
  transferring control
- **Device enumeration**: scan for I/O devices and build a device tree that
  the OS can query
