defmodule CodingAdventures.BootloaderTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.Bootloader
  alias CodingAdventures.Bootloader.{Config, DiskImage}

  test "generate produces non-empty machine code" do
    config = %Config{kernel_size: 64}
    code = Bootloader.generate(config)
    assert length(code) > 0
    assert rem(length(code), 4) == 0
  end

  test "instruction_count" do
    config = %Config{kernel_size: 128}
    assert Bootloader.instruction_count(config) > 3
  end

  test "DiskImage creates zero-filled disk" do
    disk = DiskImage.new(1024)
    assert DiskImage.size(disk) == 1024
  end

  test "DiskImage load_kernel places data at offset" do
    disk = DiskImage.new(1024 * 1024) |> DiskImage.load_kernel([0xDE, 0xAD, 0xBE, 0xEF])
    assert DiskImage.read_word(disk, 0x00080000) == 0xEFBEADDE
  end

  test "DiskImage load_at places data at offset" do
    disk = DiskImage.new(1024) |> DiskImage.load_at(100, [0x42])
    assert :binary.at(disk.data, 100) == 0x42
  end
end
