# S02 — Bootloader

## Overview

The bootloader is the second piece of code to run after the BIOS (S01). It
lives at address `0x00010000`, placed there by the BIOS during POST. Its job
is deceptively simple but critically important: copy the operating system
kernel from "disk" (a byte slice simulating persistent storage) into kernel
memory at `0x00020000`, set the stack pointer, and jump to the kernel entry
point.

The bootloader is real RISC-V machine code, generated programmatically by the
`Bootloader.Generate()` method. It is not interpreted or emulated with special
hooks — it runs on the same D05 Core that will later run the kernel and user
programs.

**Analogy:** The bootloader is like a delivery person. The BIOS unlocked the
front door and turned on the lights (initialized hardware, set up the IDT).
Now the bootloader carries the OS into the building, sets it up in its office,
hands it the keys, and leaves. Once the kernel is running, the bootloader's
code is never executed again — it has served its purpose.

## Layer Position

```
Power On
│
├── ROM / BIOS (S01)
│     ├── POST: hardware self-test
│     ├── IDT: populate interrupt descriptor table
│     └── Boot Protocol: write hardware info to 0x00001000
│
├── Bootloader (S02) ← YOU ARE HERE
│     ├── Read boot protocol from 0x00001000
│     ├── Copy kernel from disk image to RAM at 0x00020000
│     ├── Set stack pointer (sp = 0x0006FFF0)
│     └── Jump to kernel entry point (0x00020000)
│
├── OS Kernel (S04) ← what the bootloader loads
│
└── User Programs ← what the kernel eventually runs
```

**Depends on:** S01 (BIOS leaves boot protocol at known address), D05 Core
(executes the bootloader's RISC-V instructions)

**Used by:** S04 Kernel (bootloader loads and transfers control to it),
S06 SystemBoard (orchestrates the full boot sequence)

## Key Concepts

### The Boot Protocol

The BIOS and bootloader need to communicate. They do this through a small
data structure written to a fixed memory address (`0x00001000`) by the BIOS.
The bootloader reads this structure to learn about the system.

Think of it like a note taped to the office door: "Here is what you need to
know about this building."

```
Boot Protocol (at 0x00001000):
┌─────────────────────────────────────────────────────┐
│ Offset 0x00: Magic number (0xB007_CAFE)             │  4 bytes
│ Offset 0x04: Total memory size (bytes)              │  4 bytes
│ Offset 0x08: Kernel disk offset                     │  4 bytes
│ Offset 0x0C: Kernel size (bytes)                    │  4 bytes
│ Offset 0x10: Kernel load address (0x00020000)       │  4 bytes
│ Offset 0x14: Stack base (0x0006FFF0)                │  4 bytes
│ Offset 0x18: Display framebuffer base               │  4 bytes
│ Offset 0x1C: Keyboard I/O port address              │  4 bytes
│ Offset 0x20: Timer interval (cycles)                │  4 bytes
│ Offset 0x24-0x3F: Reserved for future use           │
└─────────────────────────────────────────────────────┘
```

The magic number `0xB007CAFE` ("BOOT CAFE") serves as a sanity check — if
the bootloader reads this address and finds the wrong magic number, it halts
with an error rather than proceeding with corrupted data.

### The Disk Image

Real computers have hard drives, SSDs, or floppy disks. Our simulated
computer has a `DiskImage` — a byte slice that acts as persistent storage.
The disk image is pre-loaded with the kernel binary and (optionally) user
program binaries before the system powers on.

```
Disk Image Layout:
┌──────────────────────────────────────────────────────┐
│ Offset 0x00000000: Boot sector (512 bytes, unused)   │
│ Offset 0x00000200: Reserved                          │
│ Offset 0x00080000: Kernel binary                     │
│ Offset 0x00080000 + kernel_size: User programs       │
│ ...                                                  │
│ End of disk                                          │
└──────────────────────────────────────────────────────┘

Why 0x00080000? This is 512 KB into the disk — a conventional offset
that gives plenty of room for boot metadata. Real bootloaders (GRUB,
Windows Boot Manager) use similar conventions for locating the kernel
on disk.
```

The disk is not directly addressable by the CPU. The bootloader must
"read" from the disk by accessing a memory-mapped I/O region or by
using a simple simulated disk controller. In our implementation, we
simplify this: the `DiskImage.Data()` is mapped into a reserved region
of the address space (`0x10000000`+), and the bootloader copies from
that region into RAM.

### The Copy Loop

The core of the bootloader is a word-by-word copy loop. In RISC-V assembly,
it looks like this:

```
    # Registers:
    #   t0 = source address (disk region, e.g., 0x10080000)
    #   t1 = destination address (0x00020000)
    #   t2 = bytes remaining
    #   t3 = temporary for data word

copy_loop:
    lw   t3, 0(t0)       # Load 4 bytes from disk
    sw   t3, 0(t1)       # Store 4 bytes to RAM
    addi t0, t0, 4       # Advance source pointer
    addi t1, t1, 4       # Advance destination pointer
    addi t2, t2, -4      # Decrement remaining bytes
    bne  t2, zero, copy_loop  # Loop if bytes remain
```

This copies 4 bytes per iteration. For a 4 KB kernel, that is 1024
iterations. Not fast by modern standards (a real bootloader would use DMA),
but perfectly functional for our educational system.

### Memory Map During Boot

```
Address Space (32-bit, 4 GB total):
┌──────────────────────┐ 0xFFFFFFFF
│  I/O Mapped Region   │
│  (Display, Keyboard) │ 0xFFFB0000+
├──────────────────────┤
│                      │
│  (Unmapped gap)      │
│                      │
├──────────────────────┤ 0x10080000
│  Disk Image          │
│  (memory-mapped)     │ 0x10000000
├──────────────────────┤
│                      │
│  (Unmapped gap)      │
│                      │
├──────────────────────┤ 0x00070000
│  Kernel Stack        │ ← sp starts at 0x0006FFF0
│  (grows downward)    │ 0x00060000
├──────────────────────┤
│  User Program Space  │ 0x00040000
├──────────────────────┤
│  Kernel Code + Data  │ ← bootloader copies kernel here
│                      │ 0x00020000
├──────────────────────┤
│  Bootloader Code     │ ← BIOS jumped here
│                      │ 0x00010000
├──────────────────────┤
│  Boot Protocol       │ ← BIOS wrote hardware info here
│                      │ 0x00001000
├──────────────────────┤
│  IDT (256 entries)   │ ← BIOS populated this
│                      │ 0x00000000
└──────────────────────┘
```

### RISC-V Code Generation

The bootloader is not hand-assembled. The `Bootloader.Generate()` method
programmatically encodes RISC-V instructions into a byte slice. Each
instruction is a 32-bit value (4 bytes, little-endian) following the RISC-V
RV32I base instruction set encoding.

```
RISC-V Instruction Formats Used:

I-type (loads, addi):
┌─────────┬───────┬───────┬──────┬──────┬─────────┐
│ imm[11:0]│  rs1  │ funct3│  rd  │opcode│         │
│  12 bits │ 5 bits│ 3 bits│5 bits│7 bits│= 32 bits│
└─────────┴───────┴───────┴──────┴──────┴─────────┘

S-type (stores):
┌────────┬───────┬───────┬───────┬────────┬─────────┐
│imm[11:5]│  rs2  │  rs1  │funct3│imm[4:0]│ opcode  │
│ 7 bits  │5 bits │5 bits │3 bits│ 5 bits │ 7 bits  │
└────────┴───────┴───────┴───────┴────────┴─────────┘

B-type (branches):
┌──────┬────────┬───────┬───────┬───────┬──────┬────────┐
│imm[12]│imm[10:5]│ rs2  │  rs1  │funct3│imm[4:1]│opcode │
│      │        │      │       │      │imm[11]│       │
└──────┴────────┴───────┴───────┴───────┴──────┴────────┘

U-type (lui — load upper immediate):
┌─────────────────────┬──────┬─────────┐
│     imm[31:12]       │  rd  │ opcode  │
│      20 bits         │5 bits│ 7 bits  │
└─────────────────────┴──────┴─────────┘
```

The `GenerateWithComments()` method returns annotated instructions —
each instruction paired with a human-readable comment explaining what it
does. This is invaluable for debugging and for understanding the boot trace.

## Public API

```go
// --- Bootloader Configuration ---

type BootloaderConfig struct {
    EntryAddress     uint32  // Where bootloader lives (default: 0x00010000)
    KernelDiskOffset uint32  // Where kernel starts in disk (default: 0x00080000)
    KernelLoadAddress uint32 // Where to copy kernel to (default: 0x00020000)
    KernelSize       uint32  // Size of kernel in bytes
    StackBase        uint32  // Initial stack pointer (default: 0x0006FFF0)
}

// DefaultBootloaderConfig returns a configuration with conventional addresses.
func DefaultBootloaderConfig() BootloaderConfig

// --- Disk Image ---

// DiskImage simulates persistent storage (hard drive / SSD).
// Pre-loaded with kernel and user program binaries before boot.
type DiskImage struct {
    data []byte
}

// NewDiskImage creates an empty disk image of the given size.
func NewDiskImage(sizeBytes int) *DiskImage

// LoadKernel writes a kernel binary to the conventional disk offset.
func (d *DiskImage) LoadKernel(kernelBinary []byte)

// LoadUserProgram writes a user program at a specified disk offset.
func (d *DiskImage) LoadUserProgram(programBinary []byte, offset int)

// ReadWord reads a 32-bit word at the given disk offset.
func (d *DiskImage) ReadWord(offset int) uint32

// Data returns the raw byte slice for memory-mapping into the address space.
func (d *DiskImage) Data() []byte

// --- Annotated Instruction ---

// AnnotatedInstruction pairs a machine code word with a human-readable comment.
type AnnotatedInstruction struct {
    Address     uint32  // Memory address of this instruction
    MachineCode uint32  // The 32-bit RISC-V instruction
    Assembly    string  // Disassembled form, e.g., "lw t3, 0(t0)"
    Comment     string  // Human explanation, e.g., "Load 4 bytes from disk"
}

// --- Bootloader ---

type Bootloader struct {
    Config BootloaderConfig
}

// NewBootloader creates a bootloader with the given configuration.
func NewBootloader(config BootloaderConfig) *Bootloader

// Generate produces the bootloader as a byte slice of RISC-V machine code.
// This is the binary that gets loaded at BootloaderConfig.EntryAddress.
func (b *Bootloader) Generate() []byte

// GenerateWithComments produces annotated instructions for debugging
// and educational display.
func (b *Bootloader) GenerateWithComments() []AnnotatedInstruction

// InstructionCount returns the number of instructions in the bootloader.
func (b *Bootloader) InstructionCount() int

// EstimateCycles returns an estimate of total cycles to copy the kernel,
// based on the copy loop iteration count.
func (b *Bootloader) EstimateCycles() int
```

## Execution Flow

The bootloader executes in four phases. Each phase is a sequence of RISC-V
instructions that runs on the D05 Core.

```
Phase 1: Validate Boot Protocol
────────────────────────────────
  lui  t0, 0x00001         # t0 = 0x00001000 (boot protocol address)
  lw   t1, 0(t0)           # t1 = magic number at boot protocol
  lui  t2, 0xB0080         # t2 = expected magic (upper 20 bits)
  addi t2, t2, 0xAFE       # t2 = 0xB007CAFE (full magic)
  bne  t1, t2, halt        # If magic wrong, halt

  Why validate? If the BIOS did not run correctly (or at all), the boot
  protocol will contain garbage. Better to halt with a clear error than
  to copy random bytes and crash mysteriously later.

Phase 2: Read Boot Parameters
─────────────────────────────
  lw   t0, 8(t0)           # t0 = kernel disk offset
  lw   t1, 16(t0)          # t1 = kernel load address
  lw   t2, 12(t0)          # t2 = kernel size
  lw   t3, 20(t0)          # t3 = stack base

  These values come from the boot protocol. The bootloader does not
  hardcode them — it trusts the BIOS to provide correct values.

Phase 3: Copy Kernel (the main loop)
─────────────────────────────────────
  # t0 = source (disk mapped region + kernel offset)
  # t1 = destination (0x00020000)
  # t2 = bytes remaining

  copy_loop:
    lw   t4, 0(t0)         # Load word from disk
    sw   t4, 0(t1)         # Store word to kernel memory
    addi t0, t0, 4         # source += 4
    addi t1, t1, 4         # dest += 4
    addi t2, t2, -4        # remaining -= 4
    bne  t2, zero, copy_loop

  For a 4 KB kernel: 1024 iterations x ~6 instructions = ~6144 instructions.
  At one instruction per cycle (simplified): ~6144 cycles.

Phase 4: Set Stack and Jump
────────────────────────────
  lui  sp, 0x00070         # sp = 0x00070000 (approximate)
  addi sp, sp, -16         # sp = 0x0006FFF0 (exact)
  lui  t0, 0x00020         # t0 = 0x00020000 (kernel entry)
  jalr zero, t0, 0         # Jump to kernel, no return

  After this instruction, the CPU is executing kernel code. The bootloader
  is done. Its instructions remain in memory at 0x00010000 but are never
  executed again.
```

## Data Structures

### BootProtocol (in-memory format)

```go
// BootProtocol is the in-memory structure at 0x00001000.
// It is written by the BIOS (S01) and read by the bootloader (S02).
type BootProtocol struct {
    Magic             uint32  // 0xB007CAFE
    TotalMemory       uint32  // Total RAM in bytes
    KernelDiskOffset  uint32  // Offset of kernel in disk image
    KernelSize        uint32  // Kernel binary size in bytes
    KernelLoadAddress uint32  // Where to load kernel (0x00020000)
    StackBase         uint32  // Initial stack pointer (0x0006FFF0)
    FramebufferBase   uint32  // Display memory-mapped address
    KeyboardPort      uint32  // Keyboard I/O port address
    TimerInterval     uint32  // Cycles between timer interrupts
}

const BootProtocolAddress = 0x00001000
const BootProtocolMagic   = 0xB007CAFE
```

### Disk Image Internal Layout

```go
const (
    DiskBootSectorOffset = 0x00000000  // 512-byte boot sector (unused)
    DiskKernelOffset     = 0x00080000  // Default kernel location
    DiskUserProgramBase  = 0x00100000  // Default user program area
    DiskMemoryMapBase    = 0x10000000  // Where disk is mapped in address space
)
```

## Test Strategy

### Code Generation Tests

- **Generate produces valid bytes**: call `Generate()`, verify result length
  is a multiple of 4 (each instruction is 4 bytes)
- **Instruction decoding**: decode each generated word as a RISC-V instruction,
  verify opcodes and register fields are valid
- **Annotated output**: call `GenerateWithComments()`, verify each entry has
  non-empty Assembly and Comment fields
- **Deterministic**: calling `Generate()` twice with the same config produces
  identical output

### Disk Image Tests

- **NewDiskImage**: verify size and initial contents (all zeros)
- **LoadKernel**: load a kernel, verify bytes appear at the correct offset
- **LoadUserProgram**: load a program at a custom offset, verify bytes
- **ReadWord**: write known bytes, read back as uint32, verify endianness
- **Data()**: verify returns the full underlying byte slice

### Execution Tests (on simulated CPU)

These tests require a D05 Core (or a minimal instruction executor):

- **Copy verification**: populate disk image with known kernel bytes, run
  bootloader, verify kernel bytes appear at `0x00020000`
- **Stack pointer**: after bootloader completes, verify `sp` (x2) equals
  the configured stack base
- **PC after jump**: verify the CPU's PC is `0x00020000` after the bootloader's
  final `jalr`
- **Magic validation**: set wrong magic number in boot protocol, verify the
  bootloader halts (reaches the halt label, which is an infinite loop)
- **Variable kernel size**: test with 1 KB, 4 KB, and 16 KB kernels, verify
  all bytes copied correctly
- **Boot protocol fields**: verify the bootloader reads the correct fields
  from the boot protocol (not hardcoded values)

### Integration Tests

- **BIOS-to-bootloader handoff**: run BIOS (S01) first, then bootloader,
  verify the bootloader successfully reads the BIOS-written boot protocol
- **Boot trace**: verify the SystemBoard (S06) records the bootloader phase
  with correct cycle count

## Future Extensions

- **Multiple boot devices**: scan for bootable disk images (like BIOS boot
  device priority)
- **Kernel signature verification**: check a hash/checksum of the kernel
  before loading (secure boot)
- **Boot menu**: display a simple text menu to choose between multiple kernels
- **Compressed kernels**: decompress the kernel during loading (like real
  bootloaders with gzip/lz4 support)
- **ELF loader**: parse ELF headers instead of flat binary, supporting
  proper section layout and entry point detection
