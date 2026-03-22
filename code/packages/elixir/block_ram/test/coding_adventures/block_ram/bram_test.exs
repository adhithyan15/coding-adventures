defmodule CodingAdventures.BlockRam.ConfigurableBRAMTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BlockRam.ConfigurableBRAM

  describe "new/1" do
    test "creates single-port BRAM with calculated depth" do
      bram = ConfigurableBRAM.new(total_bits: 64, width: 8)
      assert bram.depth == 8
      assert bram.width == 8
      assert bram.total_bits == 64
      assert bram.mode == :single_port
    end

    test "creates dual-port BRAM" do
      bram = ConfigurableBRAM.new(total_bits: 64, width: 8, mode: :dual_port)
      assert bram.mode == :dual_port
    end

    test "creates simple-dual-port BRAM" do
      bram = ConfigurableBRAM.new(total_bits: 64, width: 8, mode: :simple_dual_port)
      assert bram.mode == :simple_dual_port
    end

    test "raises when total_bits not divisible by width" do
      assert_raise ArgumentError, fn ->
        ConfigurableBRAM.new(total_bits: 10, width: 3)
      end
    end

    test "raises on invalid total_bits" do
      assert_raise ArgumentError, fn ->
        ConfigurableBRAM.new(total_bits: 0, width: 4)
      end
    end

    test "raises on invalid width" do
      assert_raise ArgumentError, fn ->
        ConfigurableBRAM.new(total_bits: 16, width: 0)
      end
    end

    test "raises on invalid mode" do
      assert_raise ArgumentError, fn ->
        ConfigurableBRAM.new(total_bits: 16, width: 4, mode: :invalid)
      end
    end
  end

  describe "read/2 and write/3" do
    test "write then read single-port" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4)
      {_out, bram} = ConfigurableBRAM.write(bram, 0, [1, 0, 1, 0])
      {data, _bram} = ConfigurableBRAM.read(bram, 0)
      assert data == [1, 0, 1, 0]
    end

    test "write then read dual-port" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4, mode: :dual_port)
      {_out, bram} = ConfigurableBRAM.write(bram, 2, [0, 1, 0, 1])
      {data, _bram} = ConfigurableBRAM.read(bram, 2)
      assert data == [0, 1, 0, 1]
    end

    test "write then read simple-dual-port" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4, mode: :simple_dual_port)
      {_out, bram} = ConfigurableBRAM.write(bram, 1, [1, 1, 0, 0])
      {data, _bram} = ConfigurableBRAM.read(bram, 1)
      assert data == [1, 1, 0, 0]
    end

    test "unwritten address returns zeros" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4)
      {data, _bram} = ConfigurableBRAM.read(bram, 3)
      assert data == [0, 0, 0, 0]
    end

    test "write-through returns written data" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4)
      {out, _bram} = ConfigurableBRAM.write(bram, 0, [1, 1, 1, 1])
      assert out == [1, 1, 1, 1]
    end
  end

  describe "init_data/2" do
    test "initializes multiple addresses" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4)

      bram =
        ConfigurableBRAM.init_data(bram, %{
          0 => [1, 0, 1, 0],
          2 => [0, 1, 0, 1],
          7 => [1, 1, 1, 1]
        })

      {d0, _} = ConfigurableBRAM.read(bram, 0)
      {d1, _} = ConfigurableBRAM.read(bram, 1)
      {d2, _} = ConfigurableBRAM.read(bram, 2)
      {d7, _} = ConfigurableBRAM.read(bram, 7)

      assert d0 == [1, 0, 1, 0]
      assert d1 == [0, 0, 0, 0]
      assert d2 == [0, 1, 0, 1]
      assert d7 == [1, 1, 1, 1]
    end
  end

  describe "dual_access/3" do
    test "simultaneous port access" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4, mode: :dual_port)
      pa = %{address: 0, data_in: [1, 0, 1, 0], write_enable: 1, chip_enable: 1}
      pb = %{address: 1, data_in: [0, 1, 0, 1], write_enable: 1, chip_enable: 1}
      {_a, _b, bram} = ConfigurableBRAM.dual_access(bram, pa, pb)

      # Read back
      pa_r = %{address: 0, data_in: [0, 0, 0, 0], write_enable: 0, chip_enable: 1}
      pb_r = %{address: 1, data_in: [0, 0, 0, 0], write_enable: 0, chip_enable: 1}
      {a_data, b_data, _bram} = ConfigurableBRAM.dual_access(bram, pa_r, pb_r)
      assert a_data == [1, 0, 1, 0]
      assert b_data == [0, 1, 0, 1]
    end

    test "raises in single-port mode" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 4, mode: :single_port)
      pa = %{address: 0, data_in: [0, 0, 0, 0], write_enable: 0, chip_enable: 1}
      pb = %{address: 0, data_in: [0, 0, 0, 0], write_enable: 0, chip_enable: 1}
      assert_raise ArgumentError, fn -> ConfigurableBRAM.dual_access(bram, pa, pb) end
    end
  end

  describe "different aspect ratios" do
    test "deep narrow: 32x1" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 1)
      assert bram.depth == 32
      {_out, bram} = ConfigurableBRAM.write(bram, 15, [1])
      {data, _bram} = ConfigurableBRAM.read(bram, 15)
      assert data == [1]
    end

    test "wide shallow: 2x16" do
      bram = ConfigurableBRAM.new(total_bits: 32, width: 16)
      assert bram.depth == 2
      data = [1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1]
      {_out, bram} = ConfigurableBRAM.write(bram, 0, data)
      {read_back, _bram} = ConfigurableBRAM.read(bram, 0)
      assert read_back == data
    end
  end
end
