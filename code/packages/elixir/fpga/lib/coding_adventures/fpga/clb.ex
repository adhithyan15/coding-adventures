defmodule CodingAdventures.FPGA.CLB do
  @moduledoc """
  CLB — Configurable Logic Block, the primary logic resource in an FPGA.

  ## What is a CLB?

  A Configurable Logic Block (CLB) is the main repeating logic tile in an
  FPGA. Each CLB contains two Slices, giving it a total of:

    - 4 LUTs (2 per slice)
    - 4 Flip-Flops (2 per slice)
    - 2 Carry Chains (1 per slice)

  CLBs are arranged in a grid across the FPGA, connected by the
  programmable routing network (switch matrices). The FPGA bitstream
  configures each CLB's LUT contents, flip-flop usage, and carry chain
  settings.

  ## CLB Layout

      ┌─────────────────────────────────────┐
      │           CLB (row, col)            │
      │  ┌──────────┐    ┌──────────┐      │
      │  │ Slice 0  │    │ Slice 1  │      │
      │  │ LUT_A    │    │ LUT_A    │      │
      │  │ LUT_B    │    │ LUT_B    │      │
      │  │ FF_A     │    │ FF_A     │      │
      │  │ FF_B     │    │ FF_B     │      │
      │  │ Carry    │───→│ Carry    │───→  │
      │  └──────────┘    └──────────┘      │
      └─────────────────────────────────────┘

  The carry chain can optionally propagate from Slice 0 to Slice 1,
  enabling efficient multi-bit arithmetic across the full CLB.
  """

  alias CodingAdventures.FPGA.Slice

  defstruct [:slice_0, :slice_1, :row, :col]

  @type t :: %__MODULE__{
          slice_0: Slice.t(),
          slice_1: Slice.t(),
          row: non_neg_integer(),
          col: non_neg_integer()
        }

  @doc """
  Creates a new CLB at the given grid position.

  Options:
    - `:lut_inputs` (default 4) — number of inputs per LUT
    - Slice options can be passed as `:slice_0_opts` and `:slice_1_opts`

  ## Examples

      iex> clb = CodingAdventures.FPGA.CLB.new(0, 0)
      iex> clb.row
      0
  """
  @spec new(non_neg_integer(), non_neg_integer(), keyword()) :: t()
  def new(row, col, opts \\ []) do
    lut_inputs = Keyword.get(opts, :lut_inputs, 4)
    s0_opts = Keyword.get(opts, :slice_0_opts, [])
    s1_opts = Keyword.get(opts, :slice_1_opts, [])

    %__MODULE__{
      slice_0: Slice.new(Keyword.put_new(s0_opts, :lut_inputs, lut_inputs)),
      slice_1: Slice.new(Keyword.put_new(s1_opts, :lut_inputs, lut_inputs)),
      row: row,
      col: col
    }
  end

  @doc """
  Configures both slices in the CLB.

  Takes a map with optional keys `:slice_0` and `:slice_1`, each being
  a configuration map passed to `Slice.configure/2`.

  ## Examples

      iex> clb = CodingAdventures.FPGA.CLB.new(0, 0, lut_inputs: 2)
      iex> clb = CodingAdventures.FPGA.CLB.configure(clb, %{
      ...>   slice_0: %{lut_a: [0, 0, 0, 1], lut_b: [0, 1, 1, 0]},
      ...>   slice_1: %{lut_a: [1, 1, 1, 0], lut_b: [1, 0, 0, 1]}
      ...> })
      iex> clb.slice_0.lut_a.truth_table
      [0, 0, 0, 1]
  """
  @spec configure(t(), map()) :: t()
  def configure(%__MODULE__{} = clb, config) when is_map(config) do
    clb =
      if Map.has_key?(config, :slice_0) do
        %{clb | slice_0: Slice.configure(clb.slice_0, config.slice_0)}
      else
        clb
      end

    if Map.has_key?(config, :slice_1) do
      %{clb | slice_1: Slice.configure(clb.slice_1, config.slice_1)}
    else
      clb
    end
  end

  @doc """
  Evaluates the CLB.

  Takes inputs for all four LUTs (2 per slice), clock, and carry_in.
  The carry chain propagates from Slice 0 to Slice 1 if both slices
  have carry_enable set.

  Returns `{outputs, carry_out, new_clb}` where outputs is a list of
  4 output bits [slice0_a, slice0_b, slice1_a, slice1_b].

  ## Examples

      iex> clb = CodingAdventures.FPGA.CLB.new(0, 0, lut_inputs: 2)
      iex> clb = CodingAdventures.FPGA.CLB.configure(clb, %{
      ...>   slice_0: %{lut_a: [0, 0, 0, 1]},
      ...>   slice_1: %{lut_a: [0, 1, 1, 0]}
      ...> })
      iex> {outputs, _carry, _clb} = CodingAdventures.FPGA.CLB.evaluate(
      ...>   clb,
      ...>   %{s0_a: [1, 1], s0_b: [0, 0], s1_a: [0, 1], s1_b: [0, 0]},
      ...>   0, 0)
      iex> outputs
      [1, 0, 1, 0]
  """
  @spec evaluate(t(), map(), 0 | 1, 0 | 1) :: {[0 | 1], 0 | 1, t()}
  def evaluate(%__MODULE__{} = clb, inputs, clock, carry_in) do
    # Evaluate Slice 0
    {s0_a, s0_b, carry_mid, new_slice_0} =
      Slice.evaluate(clb.slice_0, inputs.s0_a, inputs.s0_b, clock, carry_in)

    # Evaluate Slice 1 — carry_mid feeds into carry_in of Slice 1
    {s1_a, s1_b, carry_out, new_slice_1} =
      Slice.evaluate(clb.slice_1, inputs.s1_a, inputs.s1_b, clock, carry_mid)

    outputs = [s0_a, s0_b, s1_a, s1_b]
    new_clb = %{clb | slice_0: new_slice_0, slice_1: new_slice_1}
    {outputs, carry_out, new_clb}
  end
end
