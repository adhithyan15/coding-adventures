defmodule CodingAdventures.FPGA.Fabric do
  @moduledoc """
  Fabric — the complete FPGA top-level module.

  ## What is the FPGA Fabric?

  The fabric is the complete FPGA device, tying together all the components:

    - A grid of CLBs (Configurable Logic Blocks)
    - Switch matrices for routing between CLBs
    - I/O blocks at the perimeter
    - Block RAM scattered through the grid

  ## Configuration Flow

  1. Create a fabric with specified grid dimensions
  2. Load a bitstream (configuration map)
  3. Set input pin values
  4. Evaluate (propagate signals through the fabric)
  5. Read output pin values

  ## Grid Layout

  The fabric is organized as an `rows x cols` grid:

      IO  IO  IO  IO
      IO [CLB][CLB] IO
      IO [CLB][CLB] IO
      IO  IO  IO  IO

  Each CLB position also has an associated switch matrix for routing.

  ## Simplifications

  This model makes several simplifications compared to real FPGAs:
    - Single-cycle evaluation (no modeling of routing delays)
    - Switch matrices are per-CLB (real FPGAs have more complex topologies)
    - No global clock tree modeling
    - No DSP blocks or other specialized resources
  """

  alias CodingAdventures.FPGA.{CLB, SwitchMatrix, IOBlock, Bitstream}

  defstruct [:rows, :cols, :clbs, :switch_matrices, :io_blocks, :lut_inputs]

  @type t :: %__MODULE__{
          rows: pos_integer(),
          cols: pos_integer(),
          clbs: %{String.t() => CLB.t()},
          switch_matrices: %{String.t() => SwitchMatrix.t()},
          io_blocks: %{String.t() => IOBlock.t()},
          lut_inputs: pos_integer()
        }

  @doc """
  Creates a new FPGA fabric with the given grid dimensions.

  Options:
    - `:lut_inputs` (default 4) — number of inputs per LUT
    - `:switch_size` (default 8) — ports per switch matrix side

  I/O blocks are automatically created around the perimeter.

  ## Examples

      iex> fabric = CodingAdventures.FPGA.Fabric.new(2, 2)
      iex> fabric.rows
      2
      iex> map_size(fabric.clbs)
      4
  """
  @spec new(pos_integer(), pos_integer(), keyword()) :: t()
  def new(rows, cols, opts \\ []) do
    lut_inputs = Keyword.get(opts, :lut_inputs, 4)
    switch_size = Keyword.get(opts, :switch_size, 8)

    # Create CLB grid
    clbs =
      for r <- 0..(rows - 1), c <- 0..(cols - 1), into: %{} do
        {"#{r}_#{c}", CLB.new(r, c, lut_inputs: lut_inputs)}
      end

    # Create switch matrices — one per CLB position
    switch_matrices =
      for r <- 0..(rows - 1), c <- 0..(cols - 1), into: %{} do
        {"#{r}_#{c}", SwitchMatrix.new(switch_size, switch_size)}
      end

    # Create I/O blocks around the perimeter
    # Top and bottom edges: one per column
    # Left and right edges: one per row
    io_blocks =
      create_io_blocks(rows, cols)

    %__MODULE__{
      rows: rows,
      cols: cols,
      clbs: clbs,
      switch_matrices: switch_matrices,
      io_blocks: io_blocks,
      lut_inputs: lut_inputs
    }
  end

  defp create_io_blocks(rows, cols) do
    top =
      for c <- 0..(cols - 1), into: %{} do
        name = "top_#{c}"
        {name, IOBlock.new(name, :input)}
      end

    bottom =
      for c <- 0..(cols - 1), into: %{} do
        name = "bottom_#{c}"
        {name, IOBlock.new(name, :output)}
      end

    left =
      for r <- 0..(rows - 1), into: %{} do
        name = "left_#{r}"
        {name, IOBlock.new(name, :input)}
      end

    right =
      for r <- 0..(rows - 1), into: %{} do
        name = "right_#{r}"
        {name, IOBlock.new(name, :output)}
      end

    Map.merge(top, bottom) |> Map.merge(left) |> Map.merge(right)
  end

  @doc """
  Loads a bitstream configuration into the fabric.

  Applies CLB configurations, routing configurations, and I/O configurations
  from the bitstream to the corresponding components.

  ## Examples

      iex> fabric = CodingAdventures.FPGA.Fabric.new(1, 1, lut_inputs: 2)
      iex> bs = CodingAdventures.FPGA.Bitstream.from_map(%{
      ...>   "clbs" => %{"0_0" => %{"slice_0" => %{"lut_a" => [0, 0, 0, 1]}}},
      ...>   "routing" => %{},
      ...>   "io" => %{}
      ...> })
      iex> fabric = CodingAdventures.FPGA.Fabric.load_bitstream(fabric, bs)
      iex> fabric.clbs["0_0"].slice_0.lut_a.truth_table
      [0, 0, 0, 1]
  """
  @spec load_bitstream(t(), Bitstream.t()) :: t()
  def load_bitstream(%__MODULE__{} = fabric, %Bitstream{} = bitstream) do
    fabric
    |> apply_clb_configs(bitstream)
    |> apply_routing_configs(bitstream)
    |> apply_io_configs(bitstream)
  end

  defp apply_clb_configs(fabric, bitstream) do
    updated_clbs =
      Enum.reduce(fabric.clbs, fabric.clbs, fn {key, clb}, acc ->
        case Bitstream.clb_config(bitstream, key) do
          nil ->
            acc

          config ->
            # Convert string keys to atom keys for Slice.configure
            clb_config = parse_clb_config(config)
            Map.put(acc, key, CLB.configure(clb, clb_config))
        end
      end)

    %{fabric | clbs: updated_clbs}
  end

  defp parse_clb_config(config) do
    result = %{}

    result =
      case Map.get(config, "slice_0") do
        nil -> result
        slice_config -> Map.put(result, :slice_0, parse_slice_config(slice_config))
      end

    case Map.get(config, "slice_1") do
      nil -> result
      slice_config -> Map.put(result, :slice_1, parse_slice_config(slice_config))
    end
  end

  defp parse_slice_config(config) do
    result = %{}

    result =
      case Map.get(config, "lut_a") do
        nil -> result
        table -> Map.put(result, :lut_a, table)
      end

    case Map.get(config, "lut_b") do
      nil -> result
      table -> Map.put(result, :lut_b, table)
    end
  end

  defp apply_routing_configs(fabric, bitstream) do
    updated_sms =
      Enum.reduce(fabric.switch_matrices, fabric.switch_matrices, fn {key, sm}, acc ->
        case Bitstream.routing_config(bitstream, key) do
          nil -> acc
          config -> Map.put(acc, key, SwitchMatrix.configure(sm, config))
        end
      end)

    %{fabric | switch_matrices: updated_sms}
  end

  defp apply_io_configs(fabric, bitstream) do
    updated_ios =
      Enum.reduce(fabric.io_blocks, fabric.io_blocks, fn {name, _io}, acc ->
        case Bitstream.io_config(bitstream, name) do
          nil ->
            acc

          config ->
            direction =
              case Map.get(config, "direction", "input") do
                "input" -> :input
                "output" -> :output
                "bidirectional" -> :bidirectional
              end

            Map.put(acc, name, IOBlock.new(name, direction))
        end
      end)

    %{fabric | io_blocks: updated_ios}
  end

  @doc """
  Sets an input pin value on the fabric.

  ## Examples

      iex> fabric = CodingAdventures.FPGA.Fabric.new(1, 1)
      iex> fabric = CodingAdventures.FPGA.Fabric.set_input(fabric, "top_0", 1)
      iex> CodingAdventures.FPGA.IOBlock.read_fabric(fabric.io_blocks["top_0"])
      1
  """
  @spec set_input(t(), String.t(), 0 | 1) :: t()
  def set_input(%__MODULE__{} = fabric, pin_name, value) do
    io = Map.fetch!(fabric.io_blocks, pin_name)
    updated_io = IOBlock.set_pin(io, value)
    %{fabric | io_blocks: Map.put(fabric.io_blocks, pin_name, updated_io)}
  end

  @doc """
  Reads an output pin value from the fabric.

  ## Examples

      iex> fabric = CodingAdventures.FPGA.Fabric.new(1, 1)
      iex> CodingAdventures.FPGA.Fabric.read_output(fabric, "bottom_0")
      nil
  """
  @spec read_output(t(), String.t()) :: 0 | 1 | nil
  def read_output(%__MODULE__{} = fabric, pin_name) do
    io = Map.fetch!(fabric.io_blocks, pin_name)
    IOBlock.read_pin(io)
  end

  @doc """
  Evaluates one clock cycle of the FPGA fabric.

  This performs a simplified single-pass evaluation:
  1. Read all input I/O block values
  2. Route signals through switch matrices
  3. Evaluate all CLBs
  4. Update output I/O blocks

  Returns the updated fabric.

  Note: This is a simplified model. Real FPGAs evaluate combinationally
  (signals propagate through multiple levels of logic and routing in a
  single cycle). Our model does a single pass, which is sufficient for
  simple circuits but may not correctly model deep combinational chains.
  """
  @spec evaluate(t(), 0 | 1) :: t()
  def evaluate(%__MODULE__{} = fabric, clock) do
    # For now, evaluate each CLB independently with zero inputs.
    # A full implementation would trace the signal paths through
    # switch matrices, but that requires the routing to be fully
    # specified. This provides the basic evaluation framework.

    lut_inputs = fabric.lut_inputs
    zero_inputs = List.duplicate(0, lut_inputs)

    updated_clbs =
      Enum.reduce(fabric.clbs, fabric.clbs, fn {key, clb}, acc ->
        inputs = %{
          s0_a: zero_inputs,
          s0_b: zero_inputs,
          s1_a: zero_inputs,
          s1_b: zero_inputs
        }

        {_outputs, _carry, new_clb} = CLB.evaluate(clb, inputs, clock, 0)
        Map.put(acc, key, new_clb)
      end)

    %{fabric | clbs: updated_clbs}
  end

  @doc """
  Returns a summary of the fabric's resources.

  ## Examples

      iex> fabric = CodingAdventures.FPGA.Fabric.new(2, 2)
      iex> summary = CodingAdventures.FPGA.Fabric.summary(fabric)
      iex> summary.clb_count
      4
  """
  @spec summary(t()) :: map()
  def summary(%__MODULE__{} = fabric) do
    clb_count = map_size(fabric.clbs)

    %{
      rows: fabric.rows,
      cols: fabric.cols,
      clb_count: clb_count,
      lut_count: clb_count * 4,
      ff_count: clb_count * 4,
      switch_matrix_count: map_size(fabric.switch_matrices),
      io_block_count: map_size(fabric.io_blocks),
      lut_inputs: fabric.lut_inputs
    }
  end
end
