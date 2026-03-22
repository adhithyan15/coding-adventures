defmodule CodingAdventures.LogicGatesTest do
  @moduledoc """
  Tests for the root module delegations, ensuring the public API
  correctly delegates to the underlying Gates and Sequential modules.
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.LogicGates

  # Test each delegated gate function through the root module
  describe "root module delegations — gates" do
    test "not_gate" do
      assert LogicGates.not_gate(0) == 1
      assert LogicGates.not_gate(1) == 0
    end

    test "and_gate" do
      assert LogicGates.and_gate(0, 0) == 0
      assert LogicGates.and_gate(1, 1) == 1
    end

    test "or_gate" do
      assert LogicGates.or_gate(0, 0) == 0
      assert LogicGates.or_gate(0, 1) == 1
    end

    test "xor_gate" do
      assert LogicGates.xor_gate(0, 0) == 0
      assert LogicGates.xor_gate(0, 1) == 1
    end

    test "nand_gate" do
      assert LogicGates.nand_gate(1, 1) == 0
      assert LogicGates.nand_gate(0, 1) == 1
    end

    test "nor_gate" do
      assert LogicGates.nor_gate(0, 0) == 1
      assert LogicGates.nor_gate(1, 0) == 0
    end

    test "xnor_gate" do
      assert LogicGates.xnor_gate(0, 0) == 1
      assert LogicGates.xnor_gate(0, 1) == 0
    end

    test "nand_not" do
      assert LogicGates.nand_not(0) == 1
      assert LogicGates.nand_not(1) == 0
    end

    test "nand_and" do
      assert LogicGates.nand_and(1, 1) == 1
      assert LogicGates.nand_and(0, 1) == 0
    end

    test "nand_or" do
      assert LogicGates.nand_or(0, 0) == 0
      assert LogicGates.nand_or(0, 1) == 1
    end

    test "nand_xor" do
      assert LogicGates.nand_xor(0, 1) == 1
      assert LogicGates.nand_xor(1, 1) == 0
    end

    test "and_n" do
      assert LogicGates.and_n([1, 1, 1]) == 1
      assert LogicGates.and_n([1, 0, 1]) == 0
    end

    test "or_n" do
      assert LogicGates.or_n([0, 0, 1]) == 1
      assert LogicGates.or_n([0, 0, 0]) == 0
    end
  end

  describe "root module delegations — sequential" do
    test "sr_latch" do
      {q, q_bar} = LogicGates.sr_latch(1, 0, 0, 1)
      assert q == 1
      assert q_bar == 0
    end

    test "d_latch" do
      {q, q_bar} = LogicGates.d_latch(1, 1, 0, 1)
      assert q == 1
      assert q_bar == 0
    end

    test "d_flip_flop" do
      state = %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1}
      {_q, _q_bar, _new_state} = LogicGates.d_flip_flop(1, 0, state)
    end

    test "register" do
      state = Enum.map(1..4, fn _ -> %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1} end)
      {_bits, _state} = LogicGates.register([1, 0, 1, 1], 0, state)
    end

    test "shift_register" do
      state = Enum.map(1..4, fn _ -> %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1} end)
      {_bits, _state} = LogicGates.shift_register(1, 0, state)
    end

    test "counter" do
      state = Enum.map(1..4, fn _ -> %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1} end)
      {_bits, _state} = LogicGates.counter(1, 0, state)
    end
  end
end
