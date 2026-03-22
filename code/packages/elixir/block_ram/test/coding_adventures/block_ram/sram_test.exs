defmodule CodingAdventures.BlockRam.SRAMCellTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BlockRam.SRAMCell

  describe "new/0" do
    test "creates a cell initialized to 0" do
      cell = SRAMCell.new()
      assert cell.value == 0
    end
  end

  describe "new/1" do
    test "creates a cell with given value" do
      assert SRAMCell.new(0).value == 0
      assert SRAMCell.new(1).value == 1
    end

    test "raises on invalid value" do
      assert_raise ArgumentError, fn -> SRAMCell.new(2) end
      assert_raise ArgumentError, fn -> SRAMCell.new(-1) end
    end
  end

  describe "read/2" do
    test "returns value when enabled" do
      cell = SRAMCell.new(1)
      assert SRAMCell.read(cell, 1) == 1
    end

    test "returns nil when disabled" do
      cell = SRAMCell.new(1)
      assert SRAMCell.read(cell, 0) == nil
    end

    test "reads 0 when cell is 0 and enabled" do
      cell = SRAMCell.new(0)
      assert SRAMCell.read(cell, 0) == nil
      assert SRAMCell.read(cell, 1) == 0
    end

    test "raises on invalid enable" do
      cell = SRAMCell.new()
      assert_raise ArgumentError, fn -> SRAMCell.read(cell, 2) end
    end
  end

  describe "write/3" do
    test "updates value when enabled" do
      cell = SRAMCell.new(0)
      updated = SRAMCell.write(cell, 1, 1)
      assert updated.value == 1
    end

    test "no-op when disabled" do
      cell = SRAMCell.new(0)
      unchanged = SRAMCell.write(cell, 0, 1)
      assert unchanged.value == 0
    end

    test "can write 0 over 1" do
      cell = SRAMCell.new(1)
      updated = SRAMCell.write(cell, 1, 0)
      assert updated.value == 0
    end

    test "raises on invalid enable" do
      cell = SRAMCell.new()
      assert_raise ArgumentError, fn -> SRAMCell.write(cell, 2, 0) end
    end

    test "raises on invalid bit" do
      cell = SRAMCell.new()
      assert_raise ArgumentError, fn -> SRAMCell.write(cell, 1, 2) end
    end
  end
end

defmodule CodingAdventures.BlockRam.SRAMArrayTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BlockRam.SRAMArray

  describe "new/2" do
    test "creates array with correct dimensions" do
      array = SRAMArray.new(4, 8)
      assert array.depth == 4
      assert array.width == 8
    end

    test "all cells initialized to 0" do
      array = SRAMArray.new(2, 4)
      assert SRAMArray.read(array, 0, 1) == [0, 0, 0, 0]
      assert SRAMArray.read(array, 1, 1) == [0, 0, 0, 0]
    end

    test "raises on invalid dimensions" do
      assert_raise ArgumentError, fn -> SRAMArray.new(0, 4) end
      assert_raise ArgumentError, fn -> SRAMArray.new(4, 0) end
      assert_raise ArgumentError, fn -> SRAMArray.new(-1, 4) end
    end
  end

  describe "read/3" do
    test "reads zeros from fresh array" do
      array = SRAMArray.new(4, 4)
      assert SRAMArray.read(array, 0, 1) == [0, 0, 0, 0]
    end

    test "returns nils when disabled" do
      array = SRAMArray.new(4, 4)
      assert SRAMArray.read(array, 0, 0) == [nil, nil, nil, nil]
    end

    test "raises on out-of-range address" do
      array = SRAMArray.new(4, 4)
      assert_raise ArgumentError, fn -> SRAMArray.read(array, 4, 1) end
      assert_raise ArgumentError, fn -> SRAMArray.read(array, -1, 1) end
    end
  end

  describe "write/4" do
    test "writes and reads back correctly" do
      array = SRAMArray.new(4, 4)
      array = SRAMArray.write(array, 0, [1, 0, 1, 0], 1)
      assert SRAMArray.read(array, 0, 1) == [1, 0, 1, 0]
    end

    test "writing to one address doesn't affect others" do
      array = SRAMArray.new(4, 4)
      array = SRAMArray.write(array, 1, [1, 1, 1, 1], 1)
      assert SRAMArray.read(array, 0, 1) == [0, 0, 0, 0]
      assert SRAMArray.read(array, 1, 1) == [1, 1, 1, 1]
      assert SRAMArray.read(array, 2, 1) == [0, 0, 0, 0]
    end

    test "write disabled does nothing" do
      array = SRAMArray.new(4, 4)
      array = SRAMArray.write(array, 0, [1, 1, 1, 1], 0)
      assert SRAMArray.read(array, 0, 1) == [0, 0, 0, 0]
    end

    test "raises on wrong data length" do
      array = SRAMArray.new(4, 4)
      assert_raise ArgumentError, fn -> SRAMArray.write(array, 0, [1, 0], 1) end
    end

    test "overwrites previous data" do
      array = SRAMArray.new(4, 4)
      array = SRAMArray.write(array, 0, [1, 1, 1, 1], 1)
      array = SRAMArray.write(array, 0, [0, 0, 0, 0], 1)
      assert SRAMArray.read(array, 0, 1) == [0, 0, 0, 0]
    end
  end
end
