defmodule CodingAdventures.RankStatistics do
  import Bitwise

  @moduledoc """
  Sample medians and exact distribution-free confidence intervals.

  Sorting a copy keeps caller data intact. Arbitrary-precision integer
  coefficients make the binomial cutoff exact at every supported sample size.
  """

  def median(values) when is_list(values) do
    with {:ok, sorted} <- sorted_finite(values) do
      count = length(sorted)
      middle = div(count, 2)

      if rem(count, 2) == 1 do
        {:ok, Enum.at(sorted, middle)}
      else
        {:ok, midpoint(Enum.at(sorted, middle - 1), Enum.at(sorted, middle))}
      end
    end
  end

  def median(_), do: {:error, :invalid_sample}

  def median_interval(values, confidence)
      when is_integer(confidence) and confidence >= 1 and confidence <= 99 do
    with {:ok, sorted} <- sorted_finite(values) do
      rank = exact_rank(length(sorted), confidence)

      if rank == 0 do
        {:ok, nil}
      else
        {:ok, {Enum.at(sorted, rank - 1), Enum.at(sorted, length(sorted) - rank)}}
      end
    end
  end

  def median_interval(_values, _confidence), do: {:error, :invalid_confidence}

  defp sorted_finite([]), do: {:error, :empty_sample}

  defp sorted_finite(values) do
    values
    |> Enum.reduce_while({:ok, []}, fn value, {:ok, normalized} ->
      case normalize_finite(value) do
        {:ok, number} -> {:cont, {:ok, [number | normalized]}}
        :error -> {:halt, {:error, :nonfinite_sample}}
      end
    end)
    |> case do
      {:ok, normalized} -> {:ok, normalized |> Enum.reverse() |> Enum.sort()}
      error -> error
    end
  end

  defp normalize_finite(value) when is_integer(value) do
    try do
      float = value * 1.0
      if finite?(float), do: {:ok, float}, else: :error
    rescue
      ArithmeticError -> :error
    end
  end

  defp normalize_finite(value) when is_float(value) do
    if finite?(value), do: {:ok, value}, else: :error
  end

  defp normalize_finite(_), do: :error

  defp finite?(value) when is_float(value),
    do: value == value and abs(value) <= 1.7976931348623157e308

  defp midpoint(low, high) when is_float(low) and is_float(high) do
    cond do
      low < 0 and high > 0 ->
        (low + high) / 2

      abs(low) > 1.7976931348623157e308 / 2 or abs(high) > 1.7976931348623157e308 / 2 ->
        low / 2 + high / 2

      true ->
        (low + high) / 2
    end
  end

  defp midpoint(low, high), do: (low + high) / 2

  defp exact_rank(n, confidence) do
    limit = (100 - confidence) <<< n

    {rank, _coefficient, _tail} =
      Enum.reduce_while(0..div(n, 2), {0, 1, 0}, fn j, {best, coefficient, tail} ->
        next_tail = tail + coefficient

        if 200 * next_tail > limit do
          {:halt, {best, coefficient, tail}}
        else
          next_coefficient = div(coefficient * (n - j), j + 1)
          {:cont, {j + 1, next_coefficient, next_tail}}
        end
      end)

    rank
  end

  def oracle_rank_qualifies?(n, rank, confidence)
      when n in 1..20 and rank >= 1 and confidence in 1..99 do
    tail = Enum.count(0..((1 <<< n) - 1), fn mask -> bit_count(mask) <= rank - 1 end)
    200 * tail <= (100 - confidence) <<< n
  end

  defp bit_count(0), do: 0
  defp bit_count(value), do: rem(value, 2) + bit_count(div(value, 2))
end
