defmodule CodingAdventures.CpuSimulator.RegisterFile do
  @moduledoc "Fast CPU register storage."

  defstruct [:num_registers, :bit_width, :values, :max_value]

  def new(num_registers \\ 16, bit_width \\ 32) do
    import Bitwise
    max_value = if bit_width >= 32, do: 0xFFFFFFFF, else: (1 <<< bit_width) - 1

    %__MODULE__{
      num_registers: num_registers,
      bit_width: bit_width,
      values: :array.new(num_registers, default: 0),
      max_value: max_value
    }
  end

  def read(%__MODULE__{values: values, num_registers: n}, index)
      when index >= 0 and index < n do
    :array.get(index, values)
  end

  def write(%__MODULE__{values: values, num_registers: n, max_value: max} = rf, index, value)
      when index >= 0 and index < n do
    import Bitwise
    %{rf | values: :array.set(index, value &&& max, values)}
  end

  def dump(%__MODULE__{values: values, num_registers: n}) do
    for i <- 0..(n - 1), into: %{}, do: {"R#{i}", :array.get(i, values)}
  end
end
