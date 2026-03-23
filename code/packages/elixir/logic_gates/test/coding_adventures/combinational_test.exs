defmodule CodingAdventures.LogicGates.CombinationalTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LogicGates.Combinational

  # ===========================================================================
  # MUX2 TESTS
  # ===========================================================================

  describe "mux2/3" do
    test "sel=0 selects d0" do
      assert Combinational.mux2(0, 1, 0) == 0
      assert Combinational.mux2(1, 0, 0) == 1
    end

    test "sel=1 selects d1" do
      assert Combinational.mux2(0, 1, 1) == 1
      assert Combinational.mux2(1, 0, 1) == 0
    end

    test "exhaustive truth table" do
      # d0=0, d1=0 → always 0
      assert Combinational.mux2(0, 0, 0) == 0
      assert Combinational.mux2(0, 0, 1) == 0
      # d0=1, d1=1 → always 1
      assert Combinational.mux2(1, 1, 0) == 1
      assert Combinational.mux2(1, 1, 1) == 1
    end

    test "raises on invalid input" do
      assert_raise ArgumentError, fn -> Combinational.mux2(2, 0, 0) end
      assert_raise ArgumentError, fn -> Combinational.mux2(0, true, 0) end
    end
  end

  # ===========================================================================
  # MUX4 TESTS
  # ===========================================================================

  describe "mux4/5" do
    test "selects correct input for each address" do
      # d0=1, d1=0, d2=0, d3=0
      assert Combinational.mux4(1, 0, 0, 0, [0, 0]) == 1
      assert Combinational.mux4(1, 0, 0, 0, [0, 1]) == 0
      assert Combinational.mux4(1, 0, 0, 0, [1, 0]) == 0
      assert Combinational.mux4(1, 0, 0, 0, [1, 1]) == 0
    end

    test "selects d1 with sel=[0,1]" do
      assert Combinational.mux4(0, 1, 0, 0, [0, 1]) == 1
    end

    test "selects d2 with sel=[1,0]" do
      assert Combinational.mux4(0, 0, 1, 0, [1, 0]) == 1
    end

    test "selects d3 with sel=[1,1]" do
      assert Combinational.mux4(0, 0, 0, 1, [1, 1]) == 1
    end

    test "raises on invalid sel length" do
      assert_raise ArgumentError, fn -> Combinational.mux4(0, 0, 0, 0, [0]) end
    end
  end

  # ===========================================================================
  # MUX_N TESTS
  # ===========================================================================

  describe "mux_n/2" do
    test "2:1 mux" do
      assert Combinational.mux_n([0, 1], [0]) == 0
      assert Combinational.mux_n([0, 1], [1]) == 1
    end

    test "4:1 mux" do
      assert Combinational.mux_n([1, 0, 0, 0], [0, 0]) == 1
      assert Combinational.mux_n([0, 1, 0, 0], [0, 1]) == 1
      assert Combinational.mux_n([0, 0, 1, 0], [1, 0]) == 1
      assert Combinational.mux_n([0, 0, 0, 1], [1, 1]) == 1
    end

    test "8:1 mux selects correct input" do
      inputs = [0, 0, 0, 0, 0, 1, 0, 0]
      # Index 5 = binary 101
      assert Combinational.mux_n(inputs, [1, 0, 1]) == 1
    end

    test "raises on fewer than 2 inputs" do
      assert_raise ArgumentError, fn -> Combinational.mux_n([1], []) end
    end

    test "raises on wrong number of select bits" do
      assert_raise ArgumentError, fn -> Combinational.mux_n([0, 1, 0, 0], [0]) end
    end
  end

  # ===========================================================================
  # DEMUX TESTS
  # ===========================================================================

  describe "demux/3" do
    test "1-to-2 demux" do
      assert Combinational.demux(1, [0], 2) == [1, 0]
      assert Combinational.demux(1, [1], 2) == [0, 1]
    end

    test "1-to-4 demux" do
      assert Combinational.demux(1, [0, 0], 4) == [1, 0, 0, 0]
      assert Combinational.demux(1, [0, 1], 4) == [0, 1, 0, 0]
      assert Combinational.demux(1, [1, 0], 4) == [0, 0, 1, 0]
      assert Combinational.demux(1, [1, 1], 4) == [0, 0, 0, 1]
    end

    test "data=0 produces all zeros" do
      assert Combinational.demux(0, [1, 0], 4) == [0, 0, 0, 0]
    end

    test "raises on wrong number of select bits" do
      assert_raise ArgumentError, fn -> Combinational.demux(1, [0], 4) end
    end
  end

  # ===========================================================================
  # DECODER TESTS
  # ===========================================================================

  describe "decoder/1" do
    test "1-to-2 decoder" do
      assert Combinational.decoder([0]) == [1, 0]
      assert Combinational.decoder([1]) == [0, 1]
    end

    test "2-to-4 decoder" do
      assert Combinational.decoder([0, 0]) == [1, 0, 0, 0]
      assert Combinational.decoder([0, 1]) == [0, 1, 0, 0]
      assert Combinational.decoder([1, 0]) == [0, 0, 1, 0]
      assert Combinational.decoder([1, 1]) == [0, 0, 0, 1]
    end

    test "3-to-8 decoder activates correct output" do
      # Input 5 = binary 101
      result = Combinational.decoder([1, 0, 1])
      assert length(result) == 8
      assert Enum.at(result, 5) == 1
      assert Enum.sum(result) == 1
    end

    test "raises on empty input" do
      assert_raise ArgumentError, fn -> Combinational.decoder([]) end
    end
  end

  # ===========================================================================
  # ENCODER TESTS
  # ===========================================================================

  describe "encoder/1" do
    test "4-to-2 encoder" do
      assert Combinational.encoder([1, 0, 0, 0]) == [0, 0]
      assert Combinational.encoder([0, 1, 0, 0]) == [0, 1]
      assert Combinational.encoder([0, 0, 1, 0]) == [1, 0]
      assert Combinational.encoder([0, 0, 0, 1]) == [1, 1]
    end

    test "8-to-3 encoder" do
      # Input at index 5 = binary 101
      input = List.duplicate(0, 8) |> List.replace_at(5, 1)
      assert Combinational.encoder(input) == [1, 0, 1]
    end

    test "raises when no bit is set" do
      assert_raise ArgumentError, fn -> Combinational.encoder([0, 0, 0, 0]) end
    end

    test "raises when multiple bits are set" do
      assert_raise ArgumentError, fn -> Combinational.encoder([1, 0, 1, 0]) end
    end

    test "raises on fewer than 2 inputs" do
      assert_raise ArgumentError, fn -> Combinational.encoder([1]) end
    end
  end

  # ===========================================================================
  # PRIORITY ENCODER TESTS
  # ===========================================================================

  describe "priority_encoder/1" do
    test "no active inputs returns valid=0" do
      {bits, valid} = Combinational.priority_encoder([0, 0, 0, 0])
      assert valid == 0
      assert bits == [0, 0]
    end

    test "single active input" do
      {bits, valid} = Combinational.priority_encoder([0, 0, 1, 0])
      assert valid == 1
      assert bits == [1, 0]
    end

    test "highest priority wins when multiple active" do
      # Index 0 is highest priority
      {bits, valid} = Combinational.priority_encoder([1, 0, 1, 1])
      assert valid == 1
      assert bits == [0, 0]
    end

    test "second highest priority" do
      {bits, valid} = Combinational.priority_encoder([0, 1, 1, 0])
      assert valid == 1
      assert bits == [0, 1]
    end

    test "lowest priority only" do
      {bits, valid} = Combinational.priority_encoder([0, 0, 0, 1])
      assert valid == 1
      assert bits == [1, 1]
    end

    test "raises on fewer than 2 inputs" do
      assert_raise ArgumentError, fn -> Combinational.priority_encoder([1]) end
    end
  end

  # ===========================================================================
  # TRI-STATE BUFFER TESTS
  # ===========================================================================

  describe "tri_state/2" do
    test "enable=1 passes data through" do
      assert Combinational.tri_state(0, 1) == 0
      assert Combinational.tri_state(1, 1) == 1
    end

    test "enable=0 returns nil (high-impedance)" do
      assert Combinational.tri_state(0, 0) == nil
      assert Combinational.tri_state(1, 0) == nil
    end

    test "raises on invalid input" do
      assert_raise ArgumentError, fn -> Combinational.tri_state(2, 1) end
      assert_raise ArgumentError, fn -> Combinational.tri_state(0, true) end
    end
  end
end
