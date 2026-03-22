defmodule CodingAdventures.FPGA.SwitchMatrixTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FPGA.SwitchMatrix

  describe "new/2" do
    test "creates switch matrix with correct dimensions" do
      sm = SwitchMatrix.new(4, 4)
      assert sm.num_inputs == 4
      assert sm.num_outputs == 4
      assert sm.connections == %{}
    end

    test "generates correct port names" do
      sm = SwitchMatrix.new(3, 2)
      assert sm.input_names == ["in_0", "in_1", "in_2"]
      assert sm.output_names == ["out_0", "out_1"]
    end
  end

  describe "configure/2" do
    test "sets up connections" do
      sm = SwitchMatrix.new(4, 4)
      sm = SwitchMatrix.configure(sm, %{"out_0" => "in_2", "out_1" => "in_0"})
      assert sm.connections == %{"out_0" => "in_2", "out_1" => "in_0"}
    end

    test "raises on invalid output port" do
      sm = SwitchMatrix.new(4, 4)
      assert_raise ArgumentError, fn ->
        SwitchMatrix.configure(sm, %{"out_99" => "in_0"})
      end
    end

    test "raises on invalid input port" do
      sm = SwitchMatrix.new(4, 4)
      assert_raise ArgumentError, fn ->
        SwitchMatrix.configure(sm, %{"out_0" => "in_99"})
      end
    end
  end

  describe "route/2" do
    test "routes connected signals" do
      sm = SwitchMatrix.new(4, 4)
      sm = SwitchMatrix.configure(sm, %{"out_0" => "in_2", "out_3" => "in_1"})

      signals = %{"in_0" => 0, "in_1" => 1, "in_2" => 1, "in_3" => 0}
      result = SwitchMatrix.route(sm, signals)

      assert result["out_0"] == 1
      assert result["out_3"] == 1
    end

    test "unconnected outputs return nil" do
      sm = SwitchMatrix.new(4, 4)
      sm = SwitchMatrix.configure(sm, %{"out_0" => "in_0"})

      signals = %{"in_0" => 1, "in_1" => 0, "in_2" => 0, "in_3" => 0}
      result = SwitchMatrix.route(sm, signals)

      assert result["out_0"] == 1
      assert result["out_1"] == nil
      assert result["out_2"] == nil
      assert result["out_3"] == nil
    end

    test "fan-out: multiple outputs from same input" do
      sm = SwitchMatrix.new(4, 4)
      sm = SwitchMatrix.configure(sm, %{"out_0" => "in_1", "out_2" => "in_1"})

      signals = %{"in_0" => 0, "in_1" => 1, "in_2" => 0, "in_3" => 0}
      result = SwitchMatrix.route(sm, signals)

      assert result["out_0"] == 1
      assert result["out_2"] == 1
    end

    test "empty connections produce all nils" do
      sm = SwitchMatrix.new(2, 2)
      result = SwitchMatrix.route(sm, %{"in_0" => 1, "in_1" => 0})
      assert result["out_0"] == nil
      assert result["out_1"] == nil
    end
  end
end
