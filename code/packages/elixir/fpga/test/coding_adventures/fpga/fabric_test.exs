defmodule CodingAdventures.FPGA.FabricTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.{Fabric, Bitstream, IOBlock}

  describe "new/3" do
    test "creates fabric with correct grid" do
      fabric = Fabric.new(2, 3)
      assert fabric.rows == 2
      assert fabric.cols == 3
      assert map_size(fabric.clbs) == 6
      assert map_size(fabric.switch_matrices) == 6
    end

    test "creates I/O blocks around perimeter" do
      fabric = Fabric.new(2, 2)
      # Top: 2, Bottom: 2, Left: 2, Right: 2 = 8
      assert map_size(fabric.io_blocks) == 8
      assert Map.has_key?(fabric.io_blocks, "top_0")
      assert Map.has_key?(fabric.io_blocks, "bottom_1")
      assert Map.has_key?(fabric.io_blocks, "left_0")
      assert Map.has_key?(fabric.io_blocks, "right_1")
    end

    test "custom LUT inputs" do
      fabric = Fabric.new(1, 1, lut_inputs: 2)
      assert fabric.clbs["0_0"].slice_0.lut_a.num_inputs == 2
    end
  end

  describe "load_bitstream/2" do
    test "configures CLBs from bitstream" do
      fabric = Fabric.new(1, 1, lut_inputs: 2)

      bs =
        Bitstream.from_map(%{
          "clbs" => %{
            "0_0" => %{
              "slice_0" => %{"lut_a" => [0, 0, 0, 1]},
              "slice_1" => %{"lut_b" => [1, 0, 0, 1]}
            }
          }
        })

      fabric = Fabric.load_bitstream(fabric, bs)
      assert fabric.clbs["0_0"].slice_0.lut_a.truth_table == [0, 0, 0, 1]
      assert fabric.clbs["0_0"].slice_1.lut_b.truth_table == [1, 0, 0, 1]
    end

    test "configures routing from bitstream" do
      fabric = Fabric.new(1, 1)

      bs =
        Bitstream.from_map(%{
          "routing" => %{"0_0" => %{"out_0" => "in_2"}}
        })

      fabric = Fabric.load_bitstream(fabric, bs)
      assert fabric.switch_matrices["0_0"].connections == %{"out_0" => "in_2"}
    end

    test "configures I/O from bitstream" do
      fabric = Fabric.new(1, 1)

      bs =
        Bitstream.from_map(%{
          "io" => %{"top_0" => %{"direction" => "bidirectional"}}
        })

      fabric = Fabric.load_bitstream(fabric, bs)
      assert fabric.io_blocks["top_0"].direction == :bidirectional
    end

    test "handles empty bitstream" do
      fabric = Fabric.new(1, 1)
      bs = Bitstream.from_map(%{})
      fabric2 = Fabric.load_bitstream(fabric, bs)
      assert fabric2.rows == 1
    end
  end

  describe "set_input/3 and read_output/2" do
    test "sets input pin" do
      fabric = Fabric.new(1, 1)
      fabric = Fabric.set_input(fabric, "top_0", 1)
      assert IOBlock.read_fabric(fabric.io_blocks["top_0"]) == 1
    end

    test "read_output returns nil for unset output" do
      fabric = Fabric.new(1, 1)
      assert Fabric.read_output(fabric, "bottom_0") == nil
    end
  end

  describe "evaluate/2" do
    test "evaluates without error" do
      fabric = Fabric.new(2, 2, lut_inputs: 2)

      bs =
        Bitstream.from_map(%{
          "clbs" => %{
            "0_0" => %{"slice_0" => %{"lut_a" => [0, 0, 0, 1]}}
          }
        })

      fabric = Fabric.load_bitstream(fabric, bs)
      fabric = Fabric.evaluate(fabric, 0)
      # Should not raise
      assert fabric.rows == 2
    end

    test "evaluate with clock=1" do
      fabric = Fabric.new(1, 1, lut_inputs: 2)
      fabric = Fabric.evaluate(fabric, 1)
      assert fabric.rows == 1
    end
  end

  describe "summary/1" do
    test "returns correct resource counts" do
      fabric = Fabric.new(2, 3)
      summary = Fabric.summary(fabric)

      assert summary.rows == 2
      assert summary.cols == 3
      assert summary.clb_count == 6
      assert summary.lut_count == 24
      assert summary.ff_count == 24
      assert summary.switch_matrix_count == 6
      assert summary.io_block_count == 10
    end

    test "1x1 fabric summary" do
      fabric = Fabric.new(1, 1, lut_inputs: 2)
      summary = Fabric.summary(fabric)

      assert summary.clb_count == 1
      assert summary.lut_count == 4
      assert summary.lut_inputs == 2
      assert summary.io_block_count == 4
    end
  end
end
