defmodule CodingAdventures.BlockRam.SinglePortRAMTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BlockRam.SinglePortRAM

  describe "new/2" do
    test "creates RAM with correct dimensions" do
      ram = SinglePortRAM.new(16, 8)
      assert ram.memory.depth == 16
      assert ram.memory.width == 8
    end
  end

  describe "access/5" do
    test "write then read" do
      ram = SinglePortRAM.new(4, 4)
      {_out, ram} = SinglePortRAM.access(ram, 0, [1, 0, 1, 0], 1, 1)
      {data, _ram} = SinglePortRAM.access(ram, 0, [0, 0, 0, 0], 0, 1)
      assert data == [1, 0, 1, 0]
    end

    test "read from unwritten address returns zeros" do
      ram = SinglePortRAM.new(4, 4)
      {data, _ram} = SinglePortRAM.access(ram, 2, [0, 0, 0, 0], 0, 1)
      assert data == [0, 0, 0, 0]
    end

    test "chip disabled returns nils" do
      ram = SinglePortRAM.new(4, 4)
      {data, _ram} = SinglePortRAM.access(ram, 0, [0, 0, 0, 0], 0, 0)
      assert data == [nil, nil, nil, nil]
    end

    test "chip disabled write does nothing" do
      ram = SinglePortRAM.new(4, 4)
      {_out, ram} = SinglePortRAM.access(ram, 0, [1, 1, 1, 1], 1, 0)
      {data, _ram} = SinglePortRAM.access(ram, 0, [0, 0, 0, 0], 0, 1)
      assert data == [0, 0, 0, 0]
    end

    test "write to multiple addresses" do
      ram = SinglePortRAM.new(4, 4)
      {_out, ram} = SinglePortRAM.access(ram, 0, [1, 0, 0, 0], 1, 1)
      {_out, ram} = SinglePortRAM.access(ram, 1, [0, 1, 0, 0], 1, 1)
      {_out, ram} = SinglePortRAM.access(ram, 2, [0, 0, 1, 0], 1, 1)
      {_out, ram} = SinglePortRAM.access(ram, 3, [0, 0, 0, 1], 1, 1)

      {d0, _} = SinglePortRAM.access(ram, 0, [0, 0, 0, 0], 0, 1)
      {d1, _} = SinglePortRAM.access(ram, 1, [0, 0, 0, 0], 0, 1)
      {d2, _} = SinglePortRAM.access(ram, 2, [0, 0, 0, 0], 0, 1)
      {d3, _} = SinglePortRAM.access(ram, 3, [0, 0, 0, 0], 0, 1)

      assert d0 == [1, 0, 0, 0]
      assert d1 == [0, 1, 0, 0]
      assert d2 == [0, 0, 1, 0]
      assert d3 == [0, 0, 0, 1]
    end

    test "write-through returns written data" do
      ram = SinglePortRAM.new(4, 4)
      {data, _ram} = SinglePortRAM.access(ram, 0, [1, 0, 1, 0], 1, 1)
      assert data == [1, 0, 1, 0]
    end
  end
end

defmodule CodingAdventures.BlockRam.DualPortRAMTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BlockRam.DualPortRAM

  defp make_port(address, data_in, we, ce) do
    %{address: address, data_in: data_in, write_enable: we, chip_enable: ce}
  end

  defp zero_data(width), do: List.duplicate(0, width)

  describe "new/2" do
    test "creates RAM with correct dimensions" do
      ram = DualPortRAM.new(16, 8)
      assert ram.memory.depth == 16
      assert ram.memory.width == 8
    end
  end

  describe "access/3" do
    test "simultaneous writes to different addresses" do
      ram = DualPortRAM.new(4, 4)
      pa = make_port(0, [1, 0, 1, 0], 1, 1)
      pb = make_port(1, [0, 1, 0, 1], 1, 1)
      {_a, _b, ram} = DualPortRAM.access(ram, pa, pb)

      # Read both back
      pa_r = make_port(0, zero_data(4), 0, 1)
      pb_r = make_port(1, zero_data(4), 0, 1)
      {a_data, b_data, _ram} = DualPortRAM.access(ram, pa_r, pb_r)
      assert a_data == [1, 0, 1, 0]
      assert b_data == [0, 1, 0, 1]
    end

    test "port A wins on same-address write conflict" do
      ram = DualPortRAM.new(4, 4)
      pa = make_port(0, [1, 1, 1, 1], 1, 1)
      pb = make_port(0, [0, 0, 0, 0], 1, 1)
      {_a, _b, ram} = DualPortRAM.access(ram, pa, pb)

      # Port A data should win
      pa_r = make_port(0, zero_data(4), 0, 1)
      pb_r = make_port(0, zero_data(4), 0, 1)
      {a_data, _b_data, _ram} = DualPortRAM.access(ram, pa_r, pb_r)
      assert a_data == [1, 1, 1, 1]
    end

    test "one port writes while other reads different address" do
      ram = DualPortRAM.new(4, 4)
      # Write to address 0 first
      pa = make_port(0, [1, 0, 1, 0], 1, 1)
      pb = make_port(0, zero_data(4), 0, 0)
      {_a, _b, ram} = DualPortRAM.access(ram, pa, pb)

      # Now write to address 1 with port A, read address 0 with port B
      pa2 = make_port(1, [0, 1, 0, 1], 1, 1)
      pb2 = make_port(0, zero_data(4), 0, 1)
      {_a, b_data, _ram} = DualPortRAM.access(ram, pa2, pb2)
      assert b_data == [1, 0, 1, 0]
    end

    test "disabled port returns nils and does not write" do
      ram = DualPortRAM.new(4, 4)
      pa = make_port(0, [1, 1, 1, 1], 1, 0)
      pb = make_port(0, zero_data(4), 0, 0)
      {a_data, b_data, ram} = DualPortRAM.access(ram, pa, pb)
      assert a_data == [nil, nil, nil, nil]
      assert b_data == [nil, nil, nil, nil]

      # Verify nothing was written
      pa_r = make_port(0, zero_data(4), 0, 1)
      pb_r = make_port(0, zero_data(4), 0, 0)
      {data, _b, _ram} = DualPortRAM.access(ram, pa_r, pb_r)
      assert data == [0, 0, 0, 0]
    end

    test "simultaneous reads from same address" do
      ram = DualPortRAM.new(4, 4)
      # Write first
      pa = make_port(0, [1, 0, 1, 0], 1, 1)
      pb = make_port(0, zero_data(4), 0, 0)
      {_a, _b, ram} = DualPortRAM.access(ram, pa, pb)

      # Both ports read same address
      pa_r = make_port(0, zero_data(4), 0, 1)
      pb_r = make_port(0, zero_data(4), 0, 1)
      {a_data, b_data, _ram} = DualPortRAM.access(ram, pa_r, pb_r)
      assert a_data == [1, 0, 1, 0]
      assert b_data == [1, 0, 1, 0]
    end
  end
end
