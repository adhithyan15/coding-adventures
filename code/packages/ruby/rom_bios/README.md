# ROM & BIOS (Ruby)

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

```ruby
require "coding_adventures/rom_bios/rom"
require "coding_adventures/rom_bios/bios"

bios = CodingAdventures::RomBios::BIOSFirmware.new(
  CodingAdventures::RomBios::BIOSConfig.new
)
firmware = bios.generate
rom = CodingAdventures::RomBios::ROM.new(
  CodingAdventures::RomBios::ROMConfig.new,
  firmware
)
first_word = rom.read_word(0xFFFF0000)
```

## Testing

```bash
bundle install
bundle exec rake test
```
