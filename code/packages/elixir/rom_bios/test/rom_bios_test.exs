defmodule CodingAdventures.RomBiosTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.RomBios.{ROM, HardwareInfo}

  test "ROM reads firmware bytes" do
    rom = ROM.new(%{base_address: 0xFFFF0000, size: 65536}, [0xDE, 0xAD, 0xBE, 0xEF])
    assert ROM.read(rom, 0xFFFF0000) == 0xDE
    assert ROM.read(rom, 0xFFFF0003) == 0xEF
  end

  test "ROM readWord returns little-endian word" do
    rom = ROM.new(%{base_address: 0xFFFF0000, size: 65536}, [0x78, 0x56, 0x34, 0x12])
    assert ROM.read_word(rom, 0xFFFF0000) == 0x12345678
  end

  test "ROM write is ignored" do
    rom = ROM.new(%{base_address: 0xFFFF0000, size: 65536}, [0xAB])
    rom2 = ROM.write(rom, 0xFFFF0000, 0xFF)
    assert ROM.read(rom2, 0xFFFF0000) == 0xAB
  end

  test "ROM out-of-range returns 0" do
    rom = ROM.new(%{base_address: 0xFFFF0000, size: 65536}, [])
    assert ROM.read(rom, 0x00000000) == 0
  end

  test "ROM contains?" do
    rom = ROM.new(%{base_address: 0xFFFF0000, size: 65536}, [])
    assert ROM.contains?(rom, 0xFFFF0000)
    refute ROM.contains?(rom, 0x00000000)
  end

  test "HardwareInfo round-trip" do
    info = %HardwareInfo{memory_size: 1_048_576}
    bytes = HardwareInfo.to_bytes(info)
    assert byte_size(bytes) == 28
    restored = HardwareInfo.from_bytes(bytes)
    assert restored.memory_size == 1_048_576
    assert restored.display_columns == 80
    assert restored.bootloader_entry == 0x00010000
  end
end
