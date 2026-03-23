# ROM & BIOS (Rust)

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

```rust
use rom_bios::{BiosFirmware, BiosConfig, Rom, RomConfig};

let bios = BiosFirmware::new(BiosConfig::default());
let firmware = bios.generate();
let rom = Rom::new(RomConfig::default(), &firmware);
let first_word = rom.read_word(0xFFFF0000);
```

## Testing

```bash
cargo test -p rom-bios
```
