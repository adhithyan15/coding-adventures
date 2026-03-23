defmodule CodingAdventures.BlockRam.SRAMCell do
  @moduledoc """
  SRAM Cell — the smallest unit of static RAM.

  ## What is an SRAM Cell?

  An SRAM (Static Random Access Memory) cell stores a single bit of data.
  In real hardware, it is built from 6 transistors arranged as two
  cross-coupled inverters (which hold the bit) plus two access transistors
  (which connect the cell to the bit lines when the word line is active).

      Word Line ────────────────────────
                    │              │
                 Access T       Access T
                    │              │
      Bit Line ── Inverter ←→ Inverter ── Bit Line (complement)

  The cross-coupled inverters form a bistable circuit — they can hold
  either 0 or 1 indefinitely, as long as power is supplied. This is
  why it's called "static" RAM — it doesn't need periodic refreshing
  like DRAM (Dynamic RAM).

  ## Operations

  - `read/2` — reads the stored bit when enable=1, returns nil when enable=0
  - `write/3` — overwrites the stored bit when enable=1, no-op when enable=0

  ## Functional Model

  We model the cell as a struct with a single `value` field. The `read` and
  `write` functions use pattern matching on the enable signal to determine
  behavior, mirroring how the access transistors gate read/write operations
  in real hardware.
  """

  defstruct value: 0

  @type t :: %__MODULE__{value: 0 | 1}

  @doc """
  Creates a new SRAM cell initialized to 0.

  In real hardware, SRAM cells power up in an indeterminate state.
  We initialize to 0 for predictability in simulation.

  ## Examples

      iex> CodingAdventures.BlockRam.SRAMCell.new()
      %CodingAdventures.BlockRam.SRAMCell{value: 0}
  """
  @spec new() :: t()
  def new, do: %__MODULE__{}

  @doc """
  Creates a new SRAM cell initialized to the given value.

  ## Examples

      iex> CodingAdventures.BlockRam.SRAMCell.new(1)
      %CodingAdventures.BlockRam.SRAMCell{value: 1}
  """
  @spec new(0 | 1) :: t()
  def new(initial_value) when initial_value in [0, 1] do
    %__MODULE__{value: initial_value}
  end

  def new(value) do
    raise ArgumentError, "initial value must be 0 or 1, got #{inspect(value)}"
  end

  @doc """
  Reads the cell's stored value when enabled.

  When enable=1, returns the stored bit (simulating the word line
  being active, connecting the cell to the bit lines).

  When enable=0, returns nil (simulating the word line being inactive,
  meaning the cell is disconnected from the bit lines — high impedance).

  ## Examples

      iex> cell = CodingAdventures.BlockRam.SRAMCell.new(1)
      iex> CodingAdventures.BlockRam.SRAMCell.read(cell, 1)
      1
      iex> CodingAdventures.BlockRam.SRAMCell.read(cell, 0)
      nil
  """
  @spec read(t(), 0 | 1) :: 0 | 1 | nil
  def read(%__MODULE__{value: v}, 1), do: v
  def read(%__MODULE__{}, 0), do: nil

  def read(%__MODULE__{}, enable) do
    raise ArgumentError, "enable must be 0 or 1, got #{inspect(enable)}"
  end

  @doc """
  Writes a bit to the cell when enabled.

  When enable=1, the cell's value is overwritten with the new bit.
  When enable=0, the cell is unchanged (the access transistors are off).

  Returns the (possibly updated) cell.

  ## Examples

      iex> cell = CodingAdventures.BlockRam.SRAMCell.new(0)
      iex> CodingAdventures.BlockRam.SRAMCell.write(cell, 1, 1)
      %CodingAdventures.BlockRam.SRAMCell{value: 1}
      iex> CodingAdventures.BlockRam.SRAMCell.write(cell, 0, 1)
      %CodingAdventures.BlockRam.SRAMCell{value: 0}
  """
  @spec write(t(), 0 | 1, 0 | 1) :: t()
  def write(%__MODULE__{} = cell, 1, bit) when bit in [0, 1] do
    %{cell | value: bit}
  end

  def write(%__MODULE__{} = cell, 0, bit) when bit in [0, 1] do
    cell
  end

  def write(%__MODULE__{}, enable, bit) do
    if enable not in [0, 1] do
      raise ArgumentError, "enable must be 0 or 1, got #{inspect(enable)}"
    end

    raise ArgumentError, "bit must be 0 or 1, got #{inspect(bit)}"
  end
end

defmodule CodingAdventures.BlockRam.SRAMArray do
  @moduledoc """
  SRAM Array — a 2D grid of SRAM cells forming a raw memory block.

  ## What is an SRAM Array?

  An SRAM array organizes individual SRAM cells into a grid of rows and
  columns. Each row represents one "word" of memory, and each column
  represents one bit position within a word.

  In real memory chips, the array is the core structure:

      Column 0   Column 1   Column 2   ... Column (W-1)
      ────────   ────────   ────────       ──────────────
      Cell[0,0]  Cell[0,1]  Cell[0,2]  ... Cell[0,W-1]    ← Row 0 (Word 0)
      Cell[1,0]  Cell[1,1]  Cell[1,2]  ... Cell[1,W-1]    ← Row 1 (Word 1)
      ...
      Cell[D-1,0] ...                      Cell[D-1,W-1]  ← Row D-1 (Word D-1)

  The "depth" (D) is the number of words (addresses), and the "width" (W)
  is the number of bits per word.

  ## Address Decoding

  To read or write a word, the address is decoded to activate one row
  (word line). All cells in that row are then connected to their
  respective bit lines for reading or writing.

  ## Implementation

  We store the array as a map of `{row, col} => SRAMCell` for efficient
  access by any cell coordinate. This is functionally equivalent to a
  2D array but more idiomatic in Elixir.
  """

  alias CodingAdventures.BlockRam.SRAMCell

  defstruct [:depth, :width, :cells]

  @type t :: %__MODULE__{
          depth: pos_integer(),
          width: pos_integer(),
          cells: %{{non_neg_integer(), non_neg_integer()} => SRAMCell.t()}
        }

  @doc """
  Creates a new SRAM array with the given depth (number of words) and
  width (bits per word). All cells are initialized to 0.

  ## Examples

      iex> array = CodingAdventures.BlockRam.SRAMArray.new(4, 8)
      iex> array.depth
      4
      iex> array.width
      8
  """
  @spec new(pos_integer(), pos_integer()) :: t()
  def new(depth, width) when is_integer(depth) and depth > 0 and is_integer(width) and width > 0 do
    cells =
      for row <- 0..(depth - 1),
          col <- 0..(width - 1),
          into: %{} do
        {{row, col}, SRAMCell.new()}
      end

    %__MODULE__{depth: depth, width: width, cells: cells}
  end

  def new(depth, width) do
    raise ArgumentError,
          "depth and width must be positive integers, got depth=#{inspect(depth)}, width=#{inspect(width)}"
  end

  @doc """
  Reads a word (list of bits) from the given row address.

  When enable=1, returns the list of bits stored at the given row.
  When enable=0, returns a list of nils (high impedance).

  Raises ArgumentError if the address is out of range.

  ## Examples

      iex> array = CodingAdventures.BlockRam.SRAMArray.new(4, 4)
      iex> CodingAdventures.BlockRam.SRAMArray.read(array, 0, 1)
      [0, 0, 0, 0]
  """
  @spec read(t(), non_neg_integer(), 0 | 1) :: [0 | 1 | nil]
  def read(%__MODULE__{} = array, address, enable) when enable in [0, 1] do
    validate_address!(array, address)

    Enum.map(0..(array.width - 1), fn col ->
      SRAMCell.read(array.cells[{address, col}], enable)
    end)
  end

  @doc """
  Writes a word (list of bits) to the given row address.

  When enable=1, all cells at the given row are updated with the
  corresponding bits from the data list. When enable=0, no change occurs.

  Returns the updated array.

  ## Examples

      iex> array = CodingAdventures.BlockRam.SRAMArray.new(4, 4)
      iex> array = CodingAdventures.BlockRam.SRAMArray.write(array, 0, [1, 0, 1, 0], 1)
      iex> CodingAdventures.BlockRam.SRAMArray.read(array, 0, 1)
      [1, 0, 1, 0]
  """
  @spec write(t(), non_neg_integer(), [0 | 1], 0 | 1) :: t()
  def write(%__MODULE__{} = array, address, data, enable) when enable in [0, 1] and is_list(data) do
    validate_address!(array, address)

    if length(data) != array.width do
      raise ArgumentError,
            "data length (#{length(data)}) must match array width (#{array.width})"
    end

    updated_cells =
      data
      |> Enum.with_index()
      |> Enum.reduce(array.cells, fn {bit, col}, cells ->
        cell = cells[{address, col}]
        updated = SRAMCell.write(cell, enable, bit)
        Map.put(cells, {address, col}, updated)
      end)

    %{array | cells: updated_cells}
  end

  defp validate_address!(%__MODULE__{depth: depth}, address) do
    if not is_integer(address) or address < 0 or address >= depth do
      raise ArgumentError,
            "address must be in range 0..#{depth - 1}, got #{inspect(address)}"
    end
  end
end
