defmodule CodingAdventures.ArrayWorkload do
  import Bitwise

  @moduledoc """
  Indexed and enumerable full-array checksum loops over an Erlang array. Input
  construction stays outside the measured callback; both callbacks read it.
  """
  @min_i64 -(1 <<< 63)
  @max_i64 (1 <<< 63) - 1

  def make_values, do: 0..4095 |> Enum.map(&rem(17 * &1 + 3, 251)) |> :array.from_list()

  def indexed_batch(values, operations), do: run(values, operations, :indexed)
  def iterator_batch(values, operations), do: run(values, operations, :iterator)

  defp run(_values, operations, _mode)
       when not is_integer(operations) or operations < 0 or operations > @max_i64,
       do: raise(ArgumentError, "operations must fit nonnegative signed 64-bit")

  defp run(_values, 0, _mode), do: 0

  defp run(values, operations, mode) when is_tuple(values) do
    if :array.size(values) == 0, do: raise(ArgumentError, "values must not be empty")

    Enum.reduce(1..operations//1, 0, fn _iteration, total ->
      one =
        if mode == :indexed do
          Enum.reduce(0..(:array.size(values) - 1)//1, 0, fn index, sum ->
            checked(sum + :array.get(index, values))
          end)
        else
          :array.foldl(fn _index, value, sum -> checked(sum + value) end, 0, values)
        end

      checked(total + one)
    end)
  end

  defp run(_values, _operations, _mode),
    do: raise(ArgumentError, "values must be an Erlang array")

  defp checked(value) when value < @min_i64 or value > @max_i64,
    do: raise(ArgumentError, "signed 64-bit checksum overflow")

  defp checked(value), do: value
end
