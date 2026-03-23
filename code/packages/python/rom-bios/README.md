# ROM & BIOS (Python)

The ROM & BIOS package implements the very first code that runs when the simulated computer powers on. ROM (Read-Only Memory) is a memory region at address `0xFFFF0000` that cannot be modified. It contains the BIOS firmware -- a RISC-V program that initializes hardware and hands off control to the bootloader.

## Layer Position

```
System Board (S06)
+-- Bootloader (S02) <-- BIOS jumps here
+-- ROM / BIOS (S01) <-- THIS PACKAGE
=== hardware / software boundary ===
D05 Core (executes firmware)
```

## Usage

```python
from rom_bios import BIOSFirmware, DefaultBIOSConfig, ROM, DefaultROMConfig

# Create BIOS firmware
bios = BIOSFirmware(DefaultBIOSConfig())
firmware = bios.generate()

# Load into ROM
rom = ROM(DefaultROMConfig(), firmware)

# Read instructions (like a CPU would)
first_instruction = rom.read_word(0xFFFF0000)

# Get annotated output for debugging
for inst in bios.generate_with_comments():
    print(f"0x{inst.address:08X}  {inst.machine_code:08X}  "
          f"{inst.assembly:<30s}  ; {inst.comment}")
```

## Testing

```bash
pip install -e ".[dev]"
pytest tests/ -v --cov
```
