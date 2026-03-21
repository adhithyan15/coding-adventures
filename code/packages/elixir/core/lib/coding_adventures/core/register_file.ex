defmodule CodingAdventures.Core.RegisterFile do
  @moduledoc """
  General-purpose register file for the Core.

  Fast, small storage that the pipeline reads and writes every cycle.

  ## Zero Register Convention

  In RISC-V and MIPS, register x0 (or $zero) is hardwired to the value 0.
  Writes to it are silently discarded. This simplifies instruction encoding:

      MOV Rd, Rs  = ADD Rd, Rs, x0   (add zero)
      NOP         = ADD x0, x0, x0   (write nothing to zero register)
      NEG Rd, Rs  = SUB Rd, x0, Rs   (subtract from zero)

  ARM does NOT have a zero register. The `zero_register` config controls this.
  """

  alias CodingAdventures.Core.Config.RegisterFileConfig

  @type t :: %__MODULE__{
          config: RegisterFileConfig.t(),
          values: %{non_neg_integer() => integer()},
          mask: integer()
        }

  defstruct config: %RegisterFileConfig{}, values: %{}, mask: 0xFFFFFFFF

  @doc """
  Creates a new register file from the given configuration.

  All registers are initialized to 0. If config is nil, the default
  configuration is used (16 registers, 32-bit, zero register enabled).
  """
  @spec new(RegisterFileConfig.t() | nil) :: t()
  def new(config \\ nil) do
    cfg = config || %RegisterFileConfig{count: 16, width: 32, zero_register: true}

    mask =
      if cfg.width >= 64 do
        # Max safe integer (Elixir has arbitrary precision, but we mask anyway)
        Bitwise.bsr(Bitwise.bnot(0), 1)
      else
        Bitwise.bsl(1, cfg.width) - 1
      end

    %__MODULE__{
      config: cfg,
      values: Map.new(0..(cfg.count - 1), fn i -> {i, 0} end),
      mask: mask
    }
  end

  @doc """
  Reads the value of the register at the given index.

  If the zero register convention is enabled, reading register 0 always
  returns 0. Returns 0 for out-of-range indices.
  """
  @spec read(t(), integer()) :: integer()
  def read(%__MODULE__{config: config}, index) when index < 0 or index >= config.count, do: 0
  def read(%__MODULE__{config: %{zero_register: true}}, 0), do: 0
  def read(%__MODULE__{values: values}, index), do: Map.get(values, index, 0)

  @doc """
  Writes a value to the register at the given index.

  The value is masked to the register width. Writes to register 0 are
  silently ignored when the zero register convention is enabled.
  Returns the updated register file.
  """
  @spec write(t(), integer(), integer()) :: t()
  def write(%__MODULE__{config: config} = rf, index, _value) when index < 0 or index >= config.count, do: rf
  def write(%__MODULE__{config: %{zero_register: true}} = rf, 0, _value), do: rf
  def write(%__MODULE__{values: values, mask: mask} = rf, index, value) do
    %{rf | values: Map.put(values, index, Bitwise.band(value, mask))}
  end

  @doc "Returns all register values as a list (for inspection and debugging)."
  @spec values(t()) :: [integer()]
  def values(%__MODULE__{config: config, values: vals}) do
    Enum.map(0..(config.count - 1), fn i -> Map.get(vals, i, 0) end)
  end

  @doc "Returns the number of registers."
  @spec count(t()) :: non_neg_integer()
  def count(%__MODULE__{config: config}), do: config.count

  @doc "Returns the bit width of each register."
  @spec width(t()) :: non_neg_integer()
  def width(%__MODULE__{config: config}), do: config.width

  @doc "Returns the register file configuration."
  @spec config(t()) :: RegisterFileConfig.t()
  def config(%__MODULE__{config: config}), do: config

  @doc "Resets all registers to zero."
  @spec reset(t()) :: t()
  def reset(%__MODULE__{config: config} = rf) do
    %{rf | values: Map.new(0..(config.count - 1), fn i -> {i, 0} end)}
  end

  @doc "Returns a human-readable dump of all registers."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{config: config, values: vals}) do
    regs =
      Enum.reduce(0..(config.count - 1), "", fn i, acc ->
        val = Map.get(vals, i, 0)
        if val != 0, do: acc <> " R#{i}=#{val}", else: acc
      end)

    "RegisterFile(#{config.count}x#{config.width}):#{regs}"
  end
end
