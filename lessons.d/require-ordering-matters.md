---
category: Ruby
---

# Require ordering matters

Ruby loads files in order — if a config file references `RomBios::BIOSConfig`, `require "coding_adventures_rom_bios"` must come BEFORE `require_relative` of your own modules in the entry point.
