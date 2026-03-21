# frozen_string_literal: true

# Entry point for the coding_adventures_rom_bios gem.
#
# This gem provides ROM (read-only memory) and BIOS firmware generation
# for a simulated computer's power-on sequence. The BIOS firmware is
# generated as RISC-V machine code that:
#
#   1. Probes memory size (or uses a configured value)
#   2. Initializes the Interrupt Descriptor Table (IDT)
#   3. Writes HardwareInfo at address 0x00001000
#   4. Jumps to the bootloader entry point
#
# Modules:
#   ROM          - Read-only memory region (writes silently ignored)
#   BIOSConfig   - Configuration for BIOS firmware generation
#   BIOSFirmware - Generates RISC-V machine code for power-on
#   HardwareInfo - Boot protocol structure written by BIOS
#
# Usage:
#   require "coding_adventures_rom_bios"
#
#   config = CodingAdventures::RomBios::BIOSConfig.new
#   bios = CodingAdventures::RomBios::BIOSFirmware.new(config)
#   rom = CodingAdventures::RomBios::ROM.new(
#     CodingAdventures::RomBios::ROMConfig.new, bios.generate
#   )

require "coding_adventures_riscv_simulator"

require_relative "coding_adventures/rom_bios/version"
require_relative "coding_adventures/rom_bios/rom"
require_relative "coding_adventures/rom_bios/bios"
require_relative "coding_adventures/rom_bios/hardware_info"
