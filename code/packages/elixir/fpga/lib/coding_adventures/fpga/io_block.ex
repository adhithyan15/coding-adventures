defmodule CodingAdventures.FPGA.IOBlock do
  @moduledoc """
  IOBlock — Input/Output interface between the FPGA fabric and external pins.

  ## What is an I/O Block?

  I/O blocks sit at the perimeter of the FPGA die and provide the interface
  between the internal logic fabric and the external package pins. Each I/O
  block can be configured as:

    - **Input** — reads an external signal into the FPGA
    - **Output** — drives an external pin from the FPGA
    - **Bidirectional** — can switch between input and output (tri-state)

  ## I/O Block Components

  A typical I/O block contains:

      External Pin
          │
      ┌───┴───┐
      │ Pad   │ ← Physical connection to package pin
      ├───────┤
      │ Input │ ← Input buffer with optional flip-flop
      │ Buffer│
      ├───────┤
      │Output │ ← Output buffer with optional flip-flop
      │Buffer │
      ├───────┤
      │Tri-St │ ← Output enable (for bidirectional I/O)
      │Control│
      └───────┘
          │
      To/From Internal Fabric

  ## Configuration

  Each I/O block is configured by the bitstream to specify:
    - Direction: `:input`, `:output`, or `:bidirectional`
    - Pull-up/pull-down resistors (not modeled here)
    - I/O standard (LVCMOS, LVDS, etc. — not modeled here)
    - Optional input/output registers
  """

  defstruct [:name, :direction, :pin_value, :fabric_value, :output_enable]

  @type direction :: :input | :output | :bidirectional
  @type t :: %__MODULE__{
          name: String.t(),
          direction: direction(),
          pin_value: 0 | 1 | nil,
          fabric_value: 0 | 1 | nil,
          output_enable: 0 | 1
        }

  @doc """
  Creates a new I/O block with the given name and direction.

  ## Examples

      iex> io = CodingAdventures.FPGA.IOBlock.new("pin_0", :input)
      iex> io.direction
      :input
  """
  @spec new(String.t(), direction()) :: t()
  def new(name, direction) when direction in [:input, :output, :bidirectional] do
    %__MODULE__{
      name: name,
      direction: direction,
      pin_value: nil,
      fabric_value: nil,
      output_enable: if(direction == :output, do: 1, else: 0)
    }
  end

  def new(_name, direction) do
    raise ArgumentError,
          "direction must be :input, :output, or :bidirectional, got #{inspect(direction)}"
  end

  @doc """
  Sets the external pin value (used for input and bidirectional blocks).

  This simulates an external device driving a signal onto the pin.

  ## Examples

      iex> io = CodingAdventures.FPGA.IOBlock.new("pin_0", :input)
      iex> io = CodingAdventures.FPGA.IOBlock.set_pin(io, 1)
      iex> io.pin_value
      1
  """
  @spec set_pin(t(), 0 | 1) :: t()
  def set_pin(%__MODULE__{direction: dir} = io, value) when value in [0, 1] do
    if dir == :output do
      raise ArgumentError, "cannot set pin on output-only I/O block"
    end

    %{io | pin_value: value}
  end

  @doc """
  Sets the fabric-side value (used for output and bidirectional blocks).

  This is the value that the internal FPGA logic wants to drive onto
  the external pin.

  ## Examples

      iex> io = CodingAdventures.FPGA.IOBlock.new("pin_0", :output)
      iex> io = CodingAdventures.FPGA.IOBlock.set_fabric(io, 1)
      iex> io.fabric_value
      1
  """
  @spec set_fabric(t(), 0 | 1) :: t()
  def set_fabric(%__MODULE__{direction: dir} = io, value) when value in [0, 1] do
    if dir == :input do
      raise ArgumentError, "cannot set fabric value on input-only I/O block"
    end

    %{io | fabric_value: value}
  end

  @doc """
  Sets the output enable signal (for bidirectional blocks).

  When output_enable=1, the block drives the pin from the fabric value.
  When output_enable=0, the block is in high-impedance state (input mode).

  ## Examples

      iex> io = CodingAdventures.FPGA.IOBlock.new("pin_0", :bidirectional)
      iex> io = CodingAdventures.FPGA.IOBlock.set_output_enable(io, 1)
      iex> io.output_enable
      1
  """
  @spec set_output_enable(t(), 0 | 1) :: t()
  def set_output_enable(%__MODULE__{direction: :bidirectional} = io, value)
      when value in [0, 1] do
    %{io | output_enable: value}
  end

  def set_output_enable(%__MODULE__{direction: dir}, _value) do
    raise ArgumentError,
          "output enable only applies to bidirectional I/O, got #{inspect(dir)}"
  end

  @doc """
  Reads the value available to the internal fabric.

  For input blocks: returns the pin value.
  For output blocks: returns the fabric value.
  For bidirectional blocks: returns pin_value if in input mode (oe=0),
  or fabric_value if in output mode (oe=1).

  ## Examples

      iex> io = CodingAdventures.FPGA.IOBlock.new("pin_0", :input) |> CodingAdventures.FPGA.IOBlock.set_pin(1)
      iex> CodingAdventures.FPGA.IOBlock.read_fabric(io)
      1
  """
  @spec read_fabric(t()) :: 0 | 1 | nil
  def read_fabric(%__MODULE__{direction: :input, pin_value: v}), do: v
  def read_fabric(%__MODULE__{direction: :output, fabric_value: v}), do: v

  def read_fabric(%__MODULE__{direction: :bidirectional, output_enable: 0, pin_value: v}),
    do: v

  def read_fabric(%__MODULE__{direction: :bidirectional, output_enable: 1, fabric_value: v}),
    do: v

  @doc """
  Reads the value on the external pin.

  For output blocks: returns the fabric value (what's being driven out).
  For input blocks: returns the pin value.
  For bidirectional blocks: returns fabric_value if oe=1, pin_value if oe=0.

  ## Examples

      iex> io = CodingAdventures.FPGA.IOBlock.new("pin_0", :output) |> CodingAdventures.FPGA.IOBlock.set_fabric(1)
      iex> CodingAdventures.FPGA.IOBlock.read_pin(io)
      1
  """
  @spec read_pin(t()) :: 0 | 1 | nil
  def read_pin(%__MODULE__{direction: :input, pin_value: v}), do: v
  def read_pin(%__MODULE__{direction: :output, fabric_value: v}), do: v

  def read_pin(%__MODULE__{direction: :bidirectional, output_enable: 1, fabric_value: v}),
    do: v

  def read_pin(%__MODULE__{direction: :bidirectional, output_enable: 0, pin_value: v}),
    do: v
end
