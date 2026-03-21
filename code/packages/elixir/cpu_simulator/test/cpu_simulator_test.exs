defmodule CodingAdventures.CpuSimulatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CpuSimulator.{Memory, RegisterFile, SparseMemory}

  describe "Memory" do
    test "read/write byte" do
      mem = Memory.new(1024) |> Memory.write_byte(100, 42)
      assert Memory.read_byte(mem, 100) == 42
      assert Memory.read_byte(mem, 0) == 0
    end

    test "read/write word little-endian" do
      mem = Memory.new(1024) |> Memory.write_word(0, 0x12345678)
      assert Memory.read_word(mem, 0) == 0x12345678
      assert Memory.read_byte(mem, 0) == 0x78
      assert Memory.read_byte(mem, 3) == 0x12
    end

    test "load_bytes and dump" do
      mem = Memory.new(1024) |> Memory.load_bytes(10, [0xAA, 0xBB, 0xCC])
      assert Memory.dump(mem, 10, 3) == [0xAA, 0xBB, 0xCC]
    end
  end

  describe "RegisterFile" do
    test "read/write" do
      rf = RegisterFile.new(32, 32) |> RegisterFile.write(1, 42)
      assert RegisterFile.read(rf, 1) == 42
      assert RegisterFile.read(rf, 0) == 0
    end

    test "masks value to bit width" do
      rf = RegisterFile.new(4, 8) |> RegisterFile.write(0, 256)
      assert RegisterFile.read(rf, 0) == 0
    end

    test "dump returns all values" do
      rf = RegisterFile.new(4, 32) |> RegisterFile.write(1, 5)
      dump = RegisterFile.dump(rf)
      assert dump["R1"] == 5
    end
  end

  describe "SparseMemory" do
    test "basic read/write" do
      mem = SparseMemory.new([%{base: 0, size: 0x1000, name: "RAM"}])
      mem = SparseMemory.write_byte(mem, 0x100, 42)
      assert SparseMemory.read_byte(mem, 0x100) == 42
    end

    test "read-only region ignores writes" do
      mem = SparseMemory.new([%{base: 0, size: 0x1000, name: "ROM", read_only: true}])
      mem = SparseMemory.load_bytes(mem, 0, [0xAB])
      mem = SparseMemory.write_byte(mem, 0, 0xFF)
      assert SparseMemory.read_byte(mem, 0) == 0xAB
    end

    test "unmapped address raises" do
      mem = SparseMemory.new([%{base: 0, size: 0x100, name: "tiny"}])
      assert_raise RuntimeError, ~r/unmapped/, fn -> SparseMemory.read_byte(mem, 0x200) end
    end

    test "word read/write" do
      mem = SparseMemory.new([%{base: 0, size: 0x1000, name: "RAM"}])
      mem = SparseMemory.write_word(mem, 0, 0xDEADBEEF)
      assert SparseMemory.read_word(mem, 0) == 0xDEADBEEF
    end

    test "region_count" do
      mem =
        SparseMemory.new([
          %{base: 0, size: 100, name: "A"},
          %{base: 0x1000, size: 100, name: "B"}
        ])

      assert SparseMemory.region_count(mem) == 2
    end
  end
end
