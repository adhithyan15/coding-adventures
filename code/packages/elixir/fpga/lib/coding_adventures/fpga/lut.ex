defmodule CodingAdventures.FPGA.LUT do
  @moduledoc """
  LUT — Lookup Table, the fundamental logic element of an FPGA.

  ## What is a LUT?

  A Lookup Table (LUT) is a small SRAM-based truth table that can implement
  ANY Boolean function of N inputs. In modern FPGAs, the most common size
  is a 4-input LUT (LUT4), which has 2^4 = 16 bits of configuration memory.

  ## How a LUT Works

  The key insight is that any Boolean function can be represented by its
  truth table. A 4-input function has 16 possible input combinations, so
  the truth table has 16 entries. We store these 16 output values in 16
  bits of SRAM. When inputs arrive, they form a 4-bit address that selects
  one of the 16 stored values — no gates needed!

  Example: implementing AND(a, b) with a 2-input LUT:

      Address (inputs) │ Stored Value (output)
      ─────────────────┼──────────────────────
         00 (a=0,b=0)  │         0
         01 (a=0,b=1)  │         0
         10 (a=1,b=0)  │         0
         11 (a=1,b=1)  │         1

  The LUT stores [0, 0, 0, 1] — the truth table for AND.

  ## Why LUTs?

  LUTs are the heart of FPGA flexibility. Any N-input Boolean function —
  AND, OR, XOR, majority vote, parity check, or any custom function —
  can be implemented by loading the right 2^N-bit pattern into the LUT.
  The FPGA bitstream (configuration file) specifies what pattern to load
  into each LUT, effectively "programming" the chip to implement any
  desired circuit.

  ## This Implementation

  We model a LUT as a struct with:
    - `num_inputs` — the number of input pins (typically 4 or 6)
    - `truth_table` — a list of 2^num_inputs output bits
  """

  defstruct [:num_inputs, :truth_table]

  @type t :: %__MODULE__{
          num_inputs: pos_integer(),
          truth_table: [0 | 1]
        }

  @doc """
  Creates a new unconfigured LUT with the given number of inputs.

  The truth table is initialized to all zeros (implements the constant-0
  function). Use `configure/2` to load a truth table.

  ## Examples

      iex> lut = CodingAdventures.FPGA.LUT.new(4)
      iex> lut.num_inputs
      4
      iex> length(lut.truth_table)
      16
  """
  @spec new(pos_integer()) :: t()
  def new(num_inputs) when is_integer(num_inputs) and num_inputs > 0 do
    table_size = Bitwise.bsl(1, num_inputs)

    %__MODULE__{
      num_inputs: num_inputs,
      truth_table: List.duplicate(0, table_size)
    }
  end

  @doc """
  Configures the LUT with a truth table.

  The truth table must be a list of exactly 2^num_inputs bits (0 or 1).
  Each entry corresponds to the output for the input combination whose
  binary value equals the entry's index.

  Returns the configured LUT.

  ## Examples

      iex> lut = CodingAdventures.FPGA.LUT.new(2)
      iex> lut = CodingAdventures.FPGA.LUT.configure(lut, [0, 0, 0, 1])
      iex> lut.truth_table
      [0, 0, 0, 1]
  """
  @spec configure(t(), [0 | 1]) :: t()
  def configure(%__MODULE__{num_inputs: n} = lut, truth_table) when is_list(truth_table) do
    expected_size = Bitwise.bsl(1, n)

    if length(truth_table) != expected_size do
      raise ArgumentError,
            "truth table must have #{expected_size} entries for #{n}-input LUT, got #{length(truth_table)}"
    end

    Enum.each(truth_table, fn bit ->
      if bit not in [0, 1] do
        raise ArgumentError, "truth table entries must be 0 or 1, got #{inspect(bit)}"
      end
    end)

    %{lut | truth_table: truth_table}
  end

  @doc """
  Evaluates the LUT for the given inputs.

  Takes a list of input bits (length must equal num_inputs). The inputs
  form a binary address into the truth table, and the corresponding
  entry is returned.

  The inputs are treated as MSB-first: the first element is the most
  significant bit of the address.

  ## Examples

      iex> lut = CodingAdventures.FPGA.LUT.new(2) |> CodingAdventures.FPGA.LUT.configure([0, 0, 0, 1])
      iex> CodingAdventures.FPGA.LUT.evaluate(lut, [1, 1])
      1
      iex> CodingAdventures.FPGA.LUT.evaluate(lut, [0, 1])
      0
  """
  @spec evaluate(t(), [0 | 1]) :: 0 | 1
  def evaluate(%__MODULE__{num_inputs: n, truth_table: table}, inputs) when is_list(inputs) do
    if length(inputs) != n do
      raise ArgumentError,
            "expected #{n} inputs, got #{length(inputs)}"
    end

    Enum.each(inputs, fn bit ->
      if bit not in [0, 1] do
        raise ArgumentError, "inputs must be 0 or 1, got #{inspect(bit)}"
      end
    end)

    # Convert the input bits to an index (MSB first)
    index =
      inputs
      |> Enum.reduce(0, fn bit, acc -> Bitwise.bsl(acc, 1) + bit end)

    Enum.at(table, index)
  end
end
