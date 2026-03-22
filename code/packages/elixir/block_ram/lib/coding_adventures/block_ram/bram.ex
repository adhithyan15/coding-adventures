defmodule CodingAdventures.BlockRam.ConfigurableBRAM do
  @moduledoc """
  Configurable Block RAM — FPGA-style memory with flexible dimensions.

  ## What is Configurable BRAM?

  In real FPGAs (like Xilinx or Intel/Altera), Block RAM primitives have
  a fixed total capacity (e.g., 18 Kbits or 36 Kbits) but can be configured
  into different aspect ratios:

      Configuration    │ Depth    │ Width │ Total bits
      ─────────────────┼──────────┼───────┼───────────
      Deep & narrow    │  16384   │   1   │   16384
      Balanced         │   2048   │   8   │   16384
      Wide & shallow   │    512   │  32   │   16384

  The total capacity stays the same — you're just rearranging how many
  addresses you have versus how many bits per address.

  ## Supported Modes

  This module supports three modes:
    - `:single_port` — one read/write port (uses SinglePortRAM)
    - `:dual_port` — two independent ports (uses DualPortRAM)
    - `:simple_dual_port` — one write-only port + one read-only port

  ## Configuration

  A `ConfigurableBRAM` is created by specifying:
    - Total capacity in bits
    - Desired word width
    - Operating mode

  The depth is automatically calculated as `total_bits / width`.

  ## Initialization

  BRAMs can optionally be initialized with data (common for ROM-style
  lookup tables in FPGAs). By default, all memory is initialized to zeros.
  """

  alias CodingAdventures.BlockRam.{SinglePortRAM, DualPortRAM}

  defstruct [:mode, :total_bits, :depth, :width, :ram]

  @type mode :: :single_port | :dual_port | :simple_dual_port
  @type t :: %__MODULE__{
          mode: mode(),
          total_bits: pos_integer(),
          depth: pos_integer(),
          width: pos_integer(),
          ram: SinglePortRAM.t() | DualPortRAM.t()
        }

  @doc """
  Creates a new configurable BRAM.

  Options:
    - `:total_bits` (required) — total storage capacity in bits
    - `:width` (required) — bits per word
    - `:mode` (optional, default `:single_port`) — operating mode

  The depth is calculated as `total_bits / width`. Both `total_bits` and
  `width` must be positive integers, and `total_bits` must be evenly
  divisible by `width`.

  ## Examples

      iex> bram = CodingAdventures.BlockRam.ConfigurableBRAM.new(total_bits: 64, width: 8)
      iex> bram.depth
      8
      iex> bram.mode
      :single_port
  """
  @spec new(keyword()) :: t()
  def new(opts) do
    total_bits = Keyword.fetch!(opts, :total_bits)
    width = Keyword.fetch!(opts, :width)
    mode = Keyword.get(opts, :mode, :single_port)

    if not is_integer(total_bits) or total_bits <= 0 do
      raise ArgumentError, "total_bits must be a positive integer, got #{inspect(total_bits)}"
    end

    if not is_integer(width) or width <= 0 do
      raise ArgumentError, "width must be a positive integer, got #{inspect(width)}"
    end

    if rem(total_bits, width) != 0 do
      raise ArgumentError,
            "total_bits (#{total_bits}) must be evenly divisible by width (#{width})"
    end

    if mode not in [:single_port, :dual_port, :simple_dual_port] do
      raise ArgumentError,
            "mode must be :single_port, :dual_port, or :simple_dual_port, got #{inspect(mode)}"
    end

    depth = div(total_bits, width)

    ram =
      case mode do
        :single_port -> SinglePortRAM.new(depth, width)
        :dual_port -> DualPortRAM.new(depth, width)
        :simple_dual_port -> DualPortRAM.new(depth, width)
      end

    %__MODULE__{
      mode: mode,
      total_bits: total_bits,
      depth: depth,
      width: width,
      ram: ram
    }
  end

  @doc """
  Initializes the BRAM with data.

  Takes a map of `%{address => [bits]}` and writes each entry.
  Returns the updated BRAM.

  ## Examples

      iex> bram = CodingAdventures.BlockRam.ConfigurableBRAM.new(total_bits: 32, width: 4)
      iex> bram = CodingAdventures.BlockRam.ConfigurableBRAM.init_data(bram, %{0 => [1,0,1,0], 2 => [0,1,0,1]})
      iex> {data, _bram} = CodingAdventures.BlockRam.ConfigurableBRAM.read(bram, 0)
      iex> data
      [1, 0, 1, 0]
  """
  @spec init_data(t(), %{non_neg_integer() => [0 | 1]}) :: t()
  def init_data(%__MODULE__{} = bram, data_map) when is_map(data_map) do
    Enum.reduce(data_map, bram, fn {address, data}, acc ->
      {_out, updated} = write(acc, address, data)
      updated
    end)
  end

  @doc """
  Reads a word from the BRAM at the given address.

  Returns `{data_out, bram}`. The BRAM state is unchanged for reads.

  ## Examples

      iex> bram = CodingAdventures.BlockRam.ConfigurableBRAM.new(total_bits: 32, width: 4)
      iex> {data, _bram} = CodingAdventures.BlockRam.ConfigurableBRAM.read(bram, 0)
      iex> data
      [0, 0, 0, 0]
  """
  @spec read(t(), non_neg_integer()) :: {[0 | 1], t()}
  def read(%__MODULE__{mode: :single_port, ram: ram, width: width} = bram, address) do
    dummy_data = List.duplicate(0, width)
    {data_out, new_ram} = SinglePortRAM.access(ram, address, dummy_data, 0, 1)
    {data_out, %{bram | ram: new_ram}}
  end

  def read(%__MODULE__{mode: mode, ram: ram, width: width} = bram, address)
      when mode in [:dual_port, :simple_dual_port] do
    dummy_data = List.duplicate(0, width)
    # Use port A for reads
    port_a = %{address: address, data_in: dummy_data, write_enable: 0, chip_enable: 1}
    port_b = %{address: 0, data_in: dummy_data, write_enable: 0, chip_enable: 0}
    {data_out, _b_out, new_ram} = DualPortRAM.access(ram, port_a, port_b)
    {data_out, %{bram | ram: new_ram}}
  end

  @doc """
  Writes a word to the BRAM at the given address.

  Returns `{data_out, new_bram}` where data_out is the written data
  (write-through).

  ## Examples

      iex> bram = CodingAdventures.BlockRam.ConfigurableBRAM.new(total_bits: 32, width: 4)
      iex> {out, _bram} = CodingAdventures.BlockRam.ConfigurableBRAM.write(bram, 0, [1, 0, 1, 0])
      iex> out
      [1, 0, 1, 0]
  """
  @spec write(t(), non_neg_integer(), [0 | 1]) :: {[0 | 1], t()}
  def write(%__MODULE__{mode: :single_port, ram: ram} = bram, address, data) do
    {data_out, new_ram} = SinglePortRAM.access(ram, address, data, 1, 1)
    {data_out, %{bram | ram: new_ram}}
  end

  def write(%__MODULE__{mode: mode, ram: ram, width: width} = bram, address, data)
      when mode in [:dual_port, :simple_dual_port] do
    dummy_data = List.duplicate(0, width)
    # Use port A for writes
    port_a = %{address: address, data_in: data, write_enable: 1, chip_enable: 1}
    port_b = %{address: 0, data_in: dummy_data, write_enable: 0, chip_enable: 0}
    {data_out, _b_out, new_ram} = DualPortRAM.access(ram, port_a, port_b)
    {data_out, %{bram | ram: new_ram}}
  end

  @doc """
  Performs dual-port access (only available in :dual_port mode).

  Takes two port specifications and returns results for both ports.
  Raises ArgumentError if the BRAM is not in :dual_port mode.

  Returns `{data_out_a, data_out_b, new_bram}`.

  ## Examples

      iex> bram = CodingAdventures.BlockRam.ConfigurableBRAM.new(total_bits: 32, width: 4, mode: :dual_port)
      iex> port_a = %{address: 0, data_in: [1,1,0,0], write_enable: 1, chip_enable: 1}
      iex> port_b = %{address: 1, data_in: [0,0,1,1], write_enable: 1, chip_enable: 1}
      iex> {_a, _b, _bram} = CodingAdventures.BlockRam.ConfigurableBRAM.dual_access(bram, port_a, port_b)
  """
  @spec dual_access(t(), DualPortRAM.port_spec(), DualPortRAM.port_spec()) ::
          {[0 | 1 | nil], [0 | 1 | nil], t()}
  def dual_access(%__MODULE__{mode: :dual_port, ram: ram} = bram, port_a, port_b) do
    {a_out, b_out, new_ram} = DualPortRAM.access(ram, port_a, port_b)
    {a_out, b_out, %{bram | ram: new_ram}}
  end

  def dual_access(%__MODULE__{mode: mode}, _port_a, _port_b) do
    raise ArgumentError,
          "dual_access requires :dual_port mode, current mode is #{inspect(mode)}"
  end
end
