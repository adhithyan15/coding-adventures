defmodule CodingAdventures.FPGA.IOBlockTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.IOBlock

  describe "new/2" do
    test "creates input block" do
      io = IOBlock.new("pin_0", :input)
      assert io.name == "pin_0"
      assert io.direction == :input
      assert io.output_enable == 0
    end

    test "creates output block with oe=1" do
      io = IOBlock.new("pin_0", :output)
      assert io.direction == :output
      assert io.output_enable == 1
    end

    test "creates bidirectional block" do
      io = IOBlock.new("pin_0", :bidirectional)
      assert io.direction == :bidirectional
      assert io.output_enable == 0
    end

    test "raises on invalid direction" do
      assert_raise ArgumentError, fn -> IOBlock.new("pin_0", :invalid) end
    end
  end

  describe "set_pin/2" do
    test "sets pin value on input block" do
      io = IOBlock.new("p", :input)
      io = IOBlock.set_pin(io, 1)
      assert io.pin_value == 1
    end

    test "raises on output-only block" do
      io = IOBlock.new("p", :output)
      assert_raise ArgumentError, fn -> IOBlock.set_pin(io, 1) end
    end

    test "works on bidirectional block" do
      io = IOBlock.new("p", :bidirectional)
      io = IOBlock.set_pin(io, 0)
      assert io.pin_value == 0
    end
  end

  describe "set_fabric/2" do
    test "sets fabric value on output block" do
      io = IOBlock.new("p", :output)
      io = IOBlock.set_fabric(io, 1)
      assert io.fabric_value == 1
    end

    test "raises on input-only block" do
      io = IOBlock.new("p", :input)
      assert_raise ArgumentError, fn -> IOBlock.set_fabric(io, 1) end
    end

    test "works on bidirectional block" do
      io = IOBlock.new("p", :bidirectional)
      io = IOBlock.set_fabric(io, 1)
      assert io.fabric_value == 1
    end
  end

  describe "set_output_enable/2" do
    test "sets oe on bidirectional block" do
      io = IOBlock.new("p", :bidirectional)
      io = IOBlock.set_output_enable(io, 1)
      assert io.output_enable == 1
    end

    test "raises on non-bidirectional block" do
      io = IOBlock.new("p", :input)
      assert_raise ArgumentError, fn -> IOBlock.set_output_enable(io, 1) end
    end
  end

  describe "read_fabric/1" do
    test "input block returns pin value" do
      io = IOBlock.new("p", :input) |> IOBlock.set_pin(1)
      assert IOBlock.read_fabric(io) == 1
    end

    test "output block returns fabric value" do
      io = IOBlock.new("p", :output) |> IOBlock.set_fabric(0)
      assert IOBlock.read_fabric(io) == 0
    end

    test "bidirectional in input mode returns pin value" do
      io = IOBlock.new("p", :bidirectional) |> IOBlock.set_pin(1)
      assert IOBlock.read_fabric(io) == 1
    end

    test "bidirectional in output mode returns fabric value" do
      io =
        IOBlock.new("p", :bidirectional)
        |> IOBlock.set_fabric(0)
        |> IOBlock.set_output_enable(1)

      assert IOBlock.read_fabric(io) == 0
    end
  end

  describe "read_pin/1" do
    test "output block returns fabric value" do
      io = IOBlock.new("p", :output) |> IOBlock.set_fabric(1)
      assert IOBlock.read_pin(io) == 1
    end

    test "input block returns pin value" do
      io = IOBlock.new("p", :input) |> IOBlock.set_pin(0)
      assert IOBlock.read_pin(io) == 0
    end

    test "bidirectional with oe=1 returns fabric value" do
      io =
        IOBlock.new("p", :bidirectional)
        |> IOBlock.set_fabric(1)
        |> IOBlock.set_output_enable(1)

      assert IOBlock.read_pin(io) == 1
    end

    test "bidirectional with oe=0 returns pin value" do
      io = IOBlock.new("p", :bidirectional) |> IOBlock.set_pin(0)
      assert IOBlock.read_pin(io) == 0
    end
  end
end
