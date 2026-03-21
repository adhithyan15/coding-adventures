# ROM & BIOS (Go)

The ROM & BIOS package implements the very first code that runs when the simulated computer powers on. ROM (Read-Only Memory) is a memory region at address `0xFFFF0000` that cannot be modified by normal program execution. It contains the BIOS firmware -- a RISC-V program that initializes hardware and hands off control to the bootloader.

## Layer Position

```
System Board (S06)
├── OS Kernel (S04)
├── Bootloader (S02) ← BIOS jumps here after initialization
├── ROM / BIOS (S01) ← THIS PACKAGE
│
════════════════════════ hardware / software boundary ════════════
D05 Core (executes firmware code)
```

## What It Does

On power-on, the CPU's program counter (PC) starts at `0xFFFF0000`. The BIOS firmware executes and performs:

1. **Memory Probe** -- Discovers how much RAM is installed by writing/reading test patterns
2. **IDT Initialization** -- Sets up 256 interrupt descriptor table entries with default handlers
3. **HardwareInfo** -- Writes a status report struct at `0x00001000` for the bootloader
4. **Jump to Bootloader** -- Transfers control to `0x00010000`

## Usage

```go
import rombios "github.com/adhithyan15/coding-adventures/code/packages/go/rom-bios"

// Create BIOS firmware with default config
bios := rombios.NewBIOSFirmware(rombios.DefaultBIOSConfig())
firmware := bios.Generate()

// Load into ROM
rom := rombios.NewROM(rombios.DefaultROMConfig(), firmware)

// Read instructions from ROM (like a CPU would)
firstInstruction := rom.ReadWord(0xFFFF0000)

// Get annotated output for debugging
annotated := bios.GenerateWithComments()
for _, inst := range annotated {
    fmt.Printf("0x%08X  %08X  %-30s  ; %s\n",
        inst.Address, inst.MachineCode, inst.Assembly, inst.Comment)
}
```

## Testing

```bash
go test ./... -v -cover
```
