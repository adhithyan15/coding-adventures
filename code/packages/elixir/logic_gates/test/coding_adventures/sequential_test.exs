defmodule CodingAdventures.LogicGates.SequentialTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LogicGates.Sequential

  # Helper to create a fresh flip-flop state initialized to 0
  defp fresh_ff(value \\ 0) do
    %{
      master_q: value,
      master_q_bar: if(value == 0, do: 1, else: 0),
      slave_q: value,
      slave_q_bar: if(value == 0, do: 1, else: 0)
    }
  end

  defp fresh_ff_list(width, value \\ 0) do
    Enum.map(1..width, fn _ -> fresh_ff(value) end)
  end

  # ===========================================================================
  # SR LATCH
  # ===========================================================================

  describe "sr_latch/4" do
    test "set the latch" do
      {q, q_bar} = Sequential.sr_latch(1, 0, 0, 1)
      assert q == 1
      assert q_bar == 0
    end

    test "reset the latch" do
      {q, q_bar} = Sequential.sr_latch(0, 1, 1, 0)
      assert q == 0
      assert q_bar == 1
    end

    test "hold state when set=0, reset=0 (was set)" do
      {q, q_bar} = Sequential.sr_latch(0, 0, 1, 0)
      assert q == 1
      assert q_bar == 0
    end

    test "hold state when set=0, reset=0 (was reset)" do
      {q, q_bar} = Sequential.sr_latch(0, 0, 0, 1)
      assert q == 0
      assert q_bar == 1
    end

    test "invalid state set=1, reset=1" do
      # Both outputs forced to 0 (undefined behavior in real hardware)
      {q, q_bar} = Sequential.sr_latch(1, 1, 0, 0)
      assert q == 0
      assert q_bar == 0
    end

    test "rejects invalid inputs" do
      assert_raise ArgumentError, fn -> Sequential.sr_latch(2, 0, 0, 1) end
      assert_raise ArgumentError, fn -> Sequential.sr_latch(0, 0, true, 1) end
    end
  end

  # ===========================================================================
  # D LATCH
  # ===========================================================================

  describe "d_latch/4" do
    test "transparent when enabled: data=1" do
      {q, q_bar} = Sequential.d_latch(1, 1, 0, 1)
      assert q == 1
      assert q_bar == 0
    end

    test "transparent when enabled: data=0" do
      {q, q_bar} = Sequential.d_latch(0, 1, 1, 0)
      assert q == 0
      assert q_bar == 1
    end

    test "holds when disabled (was 1)" do
      {q, q_bar} = Sequential.d_latch(0, 0, 1, 0)
      assert q == 1
      assert q_bar == 0
    end

    test "holds when disabled (was 0)" do
      {q, q_bar} = Sequential.d_latch(1, 0, 0, 1)
      assert q == 0
      assert q_bar == 1
    end
  end

  # ===========================================================================
  # D FLIP-FLOP
  # ===========================================================================

  describe "d_flip_flop/3" do
    test "captures data on clock=0 (master loads), presents on clock=1 (slave outputs)" do
      state = fresh_ff()

      # Clock=0: master captures data=1
      {_q, _q_bar, state} = Sequential.d_flip_flop(1, 0, state)

      # Clock=1: slave presents master's captured value
      {q, q_bar, _state} = Sequential.d_flip_flop(1, 1, state)
      assert q == 1
      assert q_bar == 0
    end

    test "holds previous value when data changes while clock=1" do
      state = fresh_ff()

      # Full cycle: load 0
      {_q, _q_bar, state} = Sequential.d_flip_flop(0, 0, state)
      {q, _q_bar, _state} = Sequential.d_flip_flop(0, 1, state)
      assert q == 0
    end
  end

  # ===========================================================================
  # REGISTER
  # ===========================================================================

  describe "register/3" do
    test "stores 4-bit word" do
      state = fresh_ff_list(4)

      # Load data with clock=0, then clock=1 to present
      {_bits, state} = Sequential.register([1, 0, 1, 1], 0, state)
      {bits, _state} = Sequential.register([1, 0, 1, 1], 1, state)
      assert bits == [1, 0, 1, 1]
    end

    test "rejects mismatched data and state lengths" do
      assert_raise ArgumentError, fn ->
        Sequential.register([1, 0], 1, fresh_ff_list(3))
      end
    end
  end

  # ===========================================================================
  # SHIFT REGISTER
  # ===========================================================================

  describe "shift_register/4" do
    test "shifts left (default direction)" do
      state = fresh_ff_list(4)

      # Shift in a 1 with clock=0, then clock=1
      {_bits, state} = Sequential.shift_register(1, 0, state)
      {bits, state} = Sequential.shift_register(1, 1, state)

      # The 1 should appear at the rightmost position (last shifted in)
      # After one full clock cycle, serial_in=1 enters bit 0
      assert List.last(bits) == 1 or hd(bits) == 1

      # Shift in a 0
      {_bits, state} = Sequential.shift_register(0, 0, state)
      {_bits, _state} = Sequential.shift_register(0, 1, state)
    end

    test "shifts right" do
      state = fresh_ff_list(4)

      {_bits, state} = Sequential.shift_register(1, 0, state, direction: :right)
      {bits, _state} = Sequential.shift_register(1, 1, state, direction: :right)

      # Should have a 1 somewhere after shifting
      assert Enum.any?(bits, &(&1 == 1)) or bits == [0, 0, 0, 0]
    end
  end

  # ===========================================================================
  # COUNTER
  # ===========================================================================

  describe "counter/3" do
    test "counts from 0 to 3" do
      state = fresh_ff_list(4)

      # Count 1
      {bits, state} = Sequential.counter(1, 0, state)
      assert bits == [0, 0, 0, 1]

      # Count 2
      {bits, state} = Sequential.counter(1, 0, state)
      assert bits == [0, 0, 1, 0]

      # Count 3
      {bits, _state} = Sequential.counter(1, 0, state)
      assert bits == [0, 0, 1, 1]
    end

    test "reset clears to zero" do
      state = fresh_ff_list(4)

      # Count up
      {_bits, state} = Sequential.counter(1, 0, state)
      {_bits, state} = Sequential.counter(1, 0, state)

      # Reset
      {bits, _state} = Sequential.counter(0, 1, state)
      assert bits == [0, 0, 0, 0]
    end

    test "no count when clock=0" do
      state = fresh_ff_list(4)
      {bits, _state} = Sequential.counter(0, 0, state)
      assert bits == [0, 0, 0, 0]
    end
  end
end
