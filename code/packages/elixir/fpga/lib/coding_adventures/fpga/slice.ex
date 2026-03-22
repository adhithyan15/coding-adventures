defmodule CodingAdventures.FPGA.Slice do
  @moduledoc """
  Slice — the basic compute unit within a CLB.

  ## What is a Slice?

  A slice is a grouping of related logic resources within a Configurable
  Logic Block (CLB). In Xilinx FPGAs, each CLB contains 2 slices, and
  each slice typically contains:

    - 2 LUTs (for combinational logic)
    - 2 Flip-Flops (for sequential logic / state storage)
    - 1 Carry Chain (for fast arithmetic)
    - MUX resources (for wide functions)

  ## How a Slice Works

  Each LUT computes a combinational function. The LUT output can either:
    1. Pass directly to the slice output (combinational path)
    2. Pass through a flip-flop first (registered path)

  The flip-flop captures the LUT output on the clock edge, creating a
  pipeline register. The `use_ff` flags control whether each LUT output
  is registered.

  The carry chain allows efficient implementation of adders and counters.
  When carry_enable is true, the carry input is XORed with LUT outputs
  to produce sum bits, and carry propagation is computed via AND/OR.

  ## Functional Model

  The slice state includes the flip-flop states for both LUT outputs.
  Operations return `{outputs, new_state}` tuples.
  """

  alias CodingAdventures.FPGA.LUT

  defstruct [
    :lut_a,
    :lut_b,
    :ff_a,
    :ff_b,
    :use_ff_a,
    :use_ff_b,
    :carry_enable
  ]

  @type ff_state :: %{
          master_q: 0 | 1,
          master_q_bar: 0 | 1,
          slave_q: 0 | 1,
          slave_q_bar: 0 | 1
        }

  @type t :: %__MODULE__{
          lut_a: LUT.t(),
          lut_b: LUT.t(),
          ff_a: ff_state(),
          ff_b: ff_state(),
          use_ff_a: boolean(),
          use_ff_b: boolean(),
          carry_enable: boolean()
        }

  @doc """
  Creates a new slice with two N-input LUTs.

  Options:
    - `:lut_inputs` (default 4) — number of inputs per LUT
    - `:use_ff_a` (default false) — register LUT A output
    - `:use_ff_b` (default false) — register LUT B output
    - `:carry_enable` (default false) — enable carry chain

  ## Examples

      iex> slice = CodingAdventures.FPGA.Slice.new()
      iex> slice.lut_a.num_inputs
      4
  """
  @spec new(keyword()) :: t()
  def new(opts \\ []) do
    lut_inputs = Keyword.get(opts, :lut_inputs, 4)

    %__MODULE__{
      lut_a: LUT.new(lut_inputs),
      lut_b: LUT.new(lut_inputs),
      ff_a: %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1},
      ff_b: %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1},
      use_ff_a: Keyword.get(opts, :use_ff_a, false),
      use_ff_b: Keyword.get(opts, :use_ff_b, false),
      carry_enable: Keyword.get(opts, :carry_enable, false)
    }
  end

  @doc """
  Configures the LUTs in this slice.

  Takes a map with optional keys `:lut_a` and `:lut_b`, each being
  a truth table (list of bits). Returns the updated slice.

  ## Examples

      iex> slice = CodingAdventures.FPGA.Slice.new(lut_inputs: 2)
      iex> slice = CodingAdventures.FPGA.Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]})
      iex> slice.lut_a.truth_table
      [0, 0, 0, 1]
  """
  @spec configure(t(), map()) :: t()
  def configure(%__MODULE__{} = slice, config) when is_map(config) do
    slice =
      if Map.has_key?(config, :lut_a) do
        %{slice | lut_a: LUT.configure(slice.lut_a, config.lut_a)}
      else
        slice
      end

    if Map.has_key?(config, :lut_b) do
      %{slice | lut_b: LUT.configure(slice.lut_b, config.lut_b)}
    else
      slice
    end
  end

  @doc """
  Evaluates the slice with given inputs and clock signal.

  Takes:
    - `inputs_a` — list of bits for LUT A
    - `inputs_b` — list of bits for LUT B
    - `clock` — clock signal (0 or 1)
    - `carry_in` — carry input (0 or 1), used when carry_enable is true

  Returns `{output_a, output_b, carry_out, new_slice}`.

  When `use_ff_a` is true, LUT A's output is registered (captured on
  clock edge). When false, it passes through combinationally.
  Same for `use_ff_b`.

  When `carry_enable` is true:
    - output_a = LUT_A_result XOR carry_in
    - carry_mid = (LUT_A_result AND carry_in) OR (LUT_A_result AND LUT_A_result)
                = LUT_A_result if carry_in=0, carry_in if LUT_A_result=1
    - output_b = LUT_B_result XOR carry_mid
    - carry_out = similar propagation

  ## Examples

      iex> slice = CodingAdventures.FPGA.Slice.new(lut_inputs: 2)
      iex> slice = CodingAdventures.FPGA.Slice.configure(slice, %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]})
      iex> {out_a, out_b, _carry, _slice} = CodingAdventures.FPGA.Slice.evaluate(slice, [1, 1], [0, 1], 0, 0)
      iex> {out_a, out_b}
      {1, 1}
  """
  @spec evaluate(t(), [0 | 1], [0 | 1], 0 | 1, 0 | 1) :: {0 | 1, 0 | 1, 0 | 1, t()}
  def evaluate(%__MODULE__{} = slice, inputs_a, inputs_b, clock, carry_in) do
    # Evaluate both LUTs
    lut_a_result = LUT.evaluate(slice.lut_a, inputs_a)
    lut_b_result = LUT.evaluate(slice.lut_b, inputs_b)

    # Apply carry chain if enabled
    {out_a_comb, carry_mid} =
      if slice.carry_enable do
        sum_a = Bitwise.bxor(lut_a_result, carry_in)
        # Carry propagation: generate OR (propagate AND carry_in)
        carry = Bitwise.bor(
          Bitwise.band(lut_a_result, carry_in),
          Bitwise.band(lut_a_result, lut_a_result)
        )
        {sum_a, carry}
      else
        {lut_a_result, 0}
      end

    {out_b_comb, carry_out} =
      if slice.carry_enable do
        sum_b = Bitwise.bxor(lut_b_result, carry_mid)
        carry = Bitwise.bor(
          Bitwise.band(lut_b_result, carry_mid),
          Bitwise.band(lut_b_result, lut_b_result)
        )
        {sum_b, carry}
      else
        {lut_b_result, 0}
      end

    # Apply flip-flops if enabled
    {output_a, new_ff_a} =
      if slice.use_ff_a do
        {q, _q_bar, new_state} =
          CodingAdventures.LogicGates.Sequential.d_flip_flop(out_a_comb, clock, slice.ff_a)
        {q, new_state}
      else
        {out_a_comb, slice.ff_a}
      end

    {output_b, new_ff_b} =
      if slice.use_ff_b do
        {q, _q_bar, new_state} =
          CodingAdventures.LogicGates.Sequential.d_flip_flop(out_b_comb, clock, slice.ff_b)
        {q, new_state}
      else
        {out_b_comb, slice.ff_b}
      end

    new_slice = %{slice | ff_a: new_ff_a, ff_b: new_ff_b}
    {output_a, output_b, carry_out, new_slice}
  end
end
