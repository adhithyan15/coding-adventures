defmodule CodingAdventures.LogicGates.Sequential do
  @moduledoc """
  Sequential Logic — memory elements built from logic gates.

  ## From Gates to Memory

  Combinational gates (AND, OR, NOT) have no memory — their output depends
  only on their current inputs. But computers need to remember things:
  register values, program counters, cached data.

  Sequential logic creates memory by feeding a gate's output back into
  its own input. This creates a feedback loop that can "hold" a value
  even after the input changes. The simplest example is the SR latch:
  two NOR gates cross-coupled so each gate's output feeds into the
  other gate's input.

  ## The Memory Hierarchy Built from Latches

      SR Latch          → simplest memory (1 bit, set/reset)
        │
      D Latch           → controlled memory (1 bit, data + enable)
        │
      D Flip-Flop       → edge-triggered (1 bit, data + clock)
        │
      Register          → N-bit word storage (parallel flip-flops)
        │
      Shift Register    → serial-to-parallel conversion
        │
      Counter           → binary counting

  Each level builds on the previous one. A CPU register file is just
  a collection of registers. A register is just a row of flip-flops.
  A flip-flop is just two latches. A latch is just two cross-coupled
  NOR gates. It's gates all the way down.
  """

  alias CodingAdventures.LogicGates.Gates

  # ===========================================================================
  # SR LATCH
  # ===========================================================================

  @doc """
  SR (Set-Reset) latch — the simplest memory element.

  An SR latch is built from two NOR gates with cross-coupled feedback:
  each gate's output feeds into the other gate's input.

      Set ────┐
              NOR ──── Q (output)
        ┌───┘     └───┐
        │             │ (cross-coupled feedback)
        └───┐     ┌───┘
              NOR ──── Q̄ (complement)
      Reset ──┘

  Behavior:
    - set=1, reset=0 → Q=1, Q̄=0 (SET the latch)
    - set=0, reset=1 → Q=0, Q̄=1 (RESET the latch)
    - set=0, reset=0 → Q and Q̄ hold their previous values (MEMORY)
    - set=1, reset=1 → INVALID (both outputs forced to 0)

  The feedback is simulated iteratively — we run the NOR gates in a
  loop until the outputs stabilize (typically 2-3 iterations).

  Returns `{q, q_bar}` as a tuple.

  ## Examples

      iex> CodingAdventures.LogicGates.Sequential.sr_latch(1, 0, 0, 1)
      {1, 0}
      iex> CodingAdventures.LogicGates.Sequential.sr_latch(0, 1, 1, 0)
      {0, 1}
      iex> CodingAdventures.LogicGates.Sequential.sr_latch(0, 0, 1, 0)
      {1, 0}
  """
  @spec sr_latch(0 | 1, 0 | 1, 0 | 1, 0 | 1) :: {0 | 1, 0 | 1}
  def sr_latch(set_, reset, q, q_bar) do
    Gates.validate_bit!(set_, "set")
    Gates.validate_bit!(reset, "reset")
    Gates.validate_bit!(q, "q")
    Gates.validate_bit!(q_bar, "q_bar")

    # Simulate the feedback loop: iterate until outputs stabilize.
    # In real hardware this happens continuously; we simulate with
    # a fixed number of iterations (3 is sufficient for convergence).
    stabilize_sr(set_, reset, q, q_bar, 3)
  end

  defp stabilize_sr(_set_, _reset, q, q_bar, 0), do: {q, q_bar}

  defp stabilize_sr(set_, reset, q, q_bar, iterations) do
    # Top NOR gate: Q = NOR(Reset, Q̄)
    new_q = Gates.nor_gate(reset, q_bar)
    # Bottom NOR gate: Q̄ = NOR(Set, Q)
    new_q_bar = Gates.nor_gate(set_, new_q)

    if new_q == q and new_q_bar == q_bar do
      {q, q_bar}
    else
      stabilize_sr(set_, reset, new_q, new_q_bar, iterations - 1)
    end
  end

  # ===========================================================================
  # D LATCH
  # ===========================================================================

  @doc """
  D (Data) latch — controlled 1-bit memory.

  A D latch wraps an SR latch with additional gates to provide a
  single data input and an enable signal:

      Data ──── AND ──── Set
                  │
      Enable ────┤
                  │
      NOT(Data)── AND ──── Reset

  When enable=1: the latch is "transparent" — Q follows Data.
  When enable=0: the latch "holds" — Q keeps its previous value
  regardless of what Data does.

  Returns `{q, q_bar}` as a tuple.

  ## Examples

      iex> CodingAdventures.LogicGates.Sequential.d_latch(1, 1, 0, 1)
      {1, 0}
      iex> CodingAdventures.LogicGates.Sequential.d_latch(0, 0, 1, 0)
      {1, 0}
  """
  @spec d_latch(0 | 1, 0 | 1, 0 | 1, 0 | 1) :: {0 | 1, 0 | 1}
  def d_latch(data, enable, q, q_bar) do
    Gates.validate_bit!(data, "data")
    Gates.validate_bit!(enable, "enable")
    Gates.validate_bit!(q, "q")
    Gates.validate_bit!(q_bar, "q_bar")

    set_ = Gates.and_gate(data, enable)
    reset = Gates.and_gate(Gates.not_gate(data), enable)
    sr_latch(set_, reset, q, q_bar)
  end

  # ===========================================================================
  # D FLIP-FLOP
  # ===========================================================================

  @doc """
  D flip-flop — edge-triggered 1-bit memory (master-slave design).

  A D flip-flop captures data only on the rising edge of the clock,
  not while the clock is high. This prevents the "transparency"
  problem of the D latch, where data can ripple through multiple
  latches in a pipeline during a single clock cycle.

  Implementation: two D latches in series (master-slave).
    - Master latch: enabled when clock=0 (captures data)
    - Slave latch: enabled when clock=1 (presents data to output)

  The state is a map with master and slave latch states:
  `%{master_q: 0|1, master_q_bar: 0|1, slave_q: 0|1, slave_q_bar: 0|1}`

  Returns `{q, q_bar, new_state}`.

  ## Examples

      iex> state = %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1}
      iex> {_q, _q_bar, _state} = CodingAdventures.LogicGates.Sequential.d_flip_flop(1, 0, state)
  """
  @spec d_flip_flop(0 | 1, 0 | 1, map()) :: {0 | 1, 0 | 1, map()}
  def d_flip_flop(data, clock, state) do
    Gates.validate_bit!(data, "data")
    Gates.validate_bit!(clock, "clock")

    # Master latch: enabled when clock is LOW (NOT clock)
    not_clock = Gates.not_gate(clock)
    {master_q, master_q_bar} =
      d_latch(data, not_clock, state.master_q, state.master_q_bar)

    # Slave latch: enabled when clock is HIGH
    {slave_q, slave_q_bar} =
      d_latch(master_q, clock, state.slave_q, state.slave_q_bar)

    new_state = %{
      master_q: master_q,
      master_q_bar: master_q_bar,
      slave_q: slave_q,
      slave_q_bar: slave_q_bar
    }

    {slave_q, slave_q_bar, new_state}
  end

  # ===========================================================================
  # REGISTER
  # ===========================================================================

  @doc """
  N-bit register — parallel array of D flip-flops.

  A register stores an N-bit word. Each bit position has its own
  D flip-flop, and all flip-flops share the same clock signal.
  When the clock edge arrives, all bits are captured simultaneously.

  This is exactly how CPU registers work: the program counter,
  general-purpose registers (R0-R15), and status flags are all
  registers built from flip-flops.

  The `data` is a list of N bits. The `state` is a list of N
  flip-flop state maps.

  Returns `{output_bits, new_state}`.

  ## Examples

      iex> state = Enum.map(1..4, fn _ -> %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1} end)
      iex> {bits, _state} = CodingAdventures.LogicGates.Sequential.register([1, 0, 1, 1], 1, state)
      iex> bits
      [1, 0, 1, 1]
  """
  @spec register([0 | 1], 0 | 1, [map()]) :: {[0 | 1], [map()]}
  def register(data, clock, state) when is_list(data) and is_list(state) do
    if length(data) != length(state) do
      raise ArgumentError,
            "data length (#{length(data)}) must match state length (#{length(state)})"
    end

    results =
      Enum.zip(data, state)
      |> Enum.map(fn {bit, ff_state} ->
        {q, _q_bar, new_ff_state} = d_flip_flop(bit, clock, ff_state)
        {q, new_ff_state}
      end)

    {Enum.map(results, &elem(&1, 0)), Enum.map(results, &elem(&1, 1))}
  end

  # ===========================================================================
  # SHIFT REGISTER
  # ===========================================================================

  @doc """
  N-bit shift register — serial-to-parallel conversion.

  A shift register moves data through a chain of flip-flops one bit
  at a time. Each clock edge shifts all bits one position, and a new
  bit enters at one end.

  Options:
    - `:direction` — `:left` (default) or `:right`

  The `state` is a list of flip-flop state maps.

  Returns `{output_bits, new_state}`.

  ## Examples

      iex> state = Enum.map(1..4, fn _ -> %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1} end)
      iex> {bits, _state} = CodingAdventures.LogicGates.Sequential.shift_register(1, 1, state)
      iex> bits
      [0, 0, 0, 1]
  """
  @spec shift_register(0 | 1, 0 | 1, [map()], keyword()) :: {[0 | 1], [map()]}
  def shift_register(serial_in, clock, state, opts \\ []) do
    Gates.validate_bit!(serial_in, "serial_in")
    Gates.validate_bit!(clock, "clock")

    direction = Keyword.get(opts, :direction, :left)

    case direction do
      :left -> shift_left(serial_in, clock, state)
      :right -> shift_right(serial_in, clock, state)
    end
  end

  defp shift_left(serial_in, clock, state) do
    # Shift left: bit 0 gets serial_in, bit N gets bit N-1's old output
    {_output_bits, new_states} =
      Enum.reduce(Enum.with_index(state), {serial_in, []}, fn {ff_state, _idx},
                                                               {input_bit, acc} ->
        {q, _q_bar, new_ff_state} = d_flip_flop(input_bit, clock, ff_state)
        {q, acc ++ [new_ff_state]}
      end)

    output_bits = Enum.map(new_states, & &1.slave_q)
    {output_bits, new_states}
  end

  defp shift_right(serial_in, clock, state) do
    # Shift right: last bit gets serial_in, bit N gets bit N+1's old output
    reversed = Enum.reverse(state)

    {_output_bits, new_states_rev} =
      Enum.reduce(Enum.with_index(reversed), {serial_in, []}, fn {ff_state, _idx},
                                                                   {input_bit, acc} ->
        {q, _q_bar, new_ff_state} = d_flip_flop(input_bit, clock, ff_state)
        {q, acc ++ [new_ff_state]}
      end)

    new_states = Enum.reverse(new_states_rev)
    output_bits = Enum.map(new_states, & &1.slave_q)
    {output_bits, new_states}
  end

  # ===========================================================================
  # COUNTER
  # ===========================================================================

  @doc """
  N-bit binary counter.

  A counter increments a binary value on each clock edge. It is built
  from a chain of flip-flops connected with half-adder logic:

    - Bit 0 toggles every clock (XOR with clock)
    - Bit 1 toggles when bit 0 carries (AND of clock and bit 0)
    - Bit N toggles when all lower bits carry

  When reset=1, the counter resets to all zeros.

  The `state` is a list of flip-flop state maps.

  Returns `{output_bits, new_state}`.

  ## Examples

      iex> state = Enum.map(1..4, fn _ -> %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1} end)
      iex> {bits, _state} = CodingAdventures.LogicGates.Sequential.counter(1, 0, state)
      iex> bits
      [0, 0, 0, 1]
  """
  @spec counter(0 | 1, 0 | 1, [map()]) :: {[0 | 1], [map()]}
  def counter(clock, reset, state) when is_list(state) do
    Gates.validate_bit!(clock, "clock")
    Gates.validate_bit!(reset, "reset")

    if reset == 1 do
      # Reset all flip-flops to 0
      zero_state =
        Enum.map(state, fn _ff ->
          %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1}
        end)

      zero_bits = Enum.map(zero_state, fn _ -> 0 end)
      {zero_bits, zero_state}
    else
      # Ripple-carry counter using half-adder chain:
      # carry starts as the clock signal, each bit XORs with carry,
      # and the carry propagates as AND(current_bit, carry).
      {new_states, _carry} =
        state
        |> Enum.reverse()
        |> Enum.reduce({[], clock}, fn ff_state, {acc, carry} ->
          current_bit = ff_state.slave_q
          toggled = Gates.xor_gate(current_bit, carry)
          new_carry = Gates.and_gate(current_bit, carry)

          {_q, _q_bar, new_ff_state} = d_flip_flop(toggled, 1, ff_state)
          # Force the slave to hold the computed value
          forced_state = %{new_ff_state | slave_q: toggled, slave_q_bar: Gates.not_gate(toggled)}
          {[forced_state | acc], new_carry}
        end)

      output_bits = Enum.map(new_states, & &1.slave_q)
      {output_bits, new_states}
    end
  end
end
