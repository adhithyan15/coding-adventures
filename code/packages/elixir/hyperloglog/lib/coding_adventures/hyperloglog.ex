defmodule CodingAdventures.HyperLogLog do
  @moduledoc """
  Approximate cardinality estimator.
  """

  import Bitwise

  @min_precision 4
  @max_precision 16
  @default_precision 14

  @enforce_keys [:precision, :registers]
  defstruct [:precision, :registers]

  def new(opts \\ []) do
    precision = Keyword.get(opts, :precision, @default_precision)
    validate_precision!(precision)
    %__MODULE__{precision: precision, registers: List.duplicate(0, 1 <<< precision)}
  end

  def from_values(values, opts \\ []) do
    Enum.reduce(values, new(opts), &add(&2, &1))
  end

  def add(%__MODULE__{} = hll, element) do
    bytes = inspect(element, limit: :infinity, printable_limit: :infinity) |> :erlang.iolist_to_binary()
    hash = hash64(bytes) |> fmix64()
    bucket = hash >>> (64 - hll.precision)
    remaining_bits = 64 - hll.precision
    remaining =
      if remaining_bits == 64 do
        hash
      else
        hash &&& ((1 <<< remaining_bits) - 1)
      end
    rho = leading_zeros(remaining, remaining_bits) + 1
    current = Enum.at(hll.registers, bucket)
    if rho > current do
      %{hll | registers: List.replace_at(hll.registers, bucket, rho)}
    else
      hll
    end
  end

  def count(%__MODULE__{} = hll) do
    m = length(hll.registers)
    z_sum =
      hll.registers
      |> Enum.map(fn register -> :math.pow(2.0, -register) end)
      |> Enum.sum()

    alpha = alpha_for_registers(m)
    estimate = alpha * m * m / z_sum
    estimate = small_range_correction(estimate, hll.registers, m)
    estimate = large_range_correction(estimate)
    round(max(estimate, 0.0))
  end

  def len(hll), do: count(hll)
  def precision(%__MODULE__{precision: precision}), do: precision
  def registers(%__MODULE__{registers: registers}), do: registers
  def num_registers(hll), do: length(hll.registers)
  def error_rate(%__MODULE__{precision: precision}), do: 1.04 / :math.sqrt(1 <<< precision)
  def memory_bytes(precision), do: div((1 <<< precision) * 6, 8)

  def merge(%__MODULE__{} = left, %__MODULE__{} = right) do
    if left.precision != right.precision do
      raise ArgumentError, "precision mismatch"
    end

    registers =
      Enum.zip(left.registers, right.registers)
      |> Enum.map(fn {a, b} -> max(a, b) end)

    %{left | registers: registers}
  end

  def try_merge(left, right) do
    try do
      {:ok, merge(left, right)}
    rescue
      ArgumentError -> {:error, :precision_mismatch}
    end
  end

  defp validate_precision!(precision) when precision < @min_precision or precision > @max_precision do
    raise ArgumentError, "precision must be between #{@min_precision} and #{@max_precision}"
  end
  defp validate_precision!(_), do: :ok

  defp hash64(bytes) do
    <<value::unsigned-64, _::binary>> = :crypto.hash(:sha256, bytes)
    value
  end

  defp fmix64(k) do
    k = Bitwise.bxor(k, k >>> 33)
    k = rem(k * 0xFF51_AFD7_ED55_8CCD, 1 <<< 64)
    k = Bitwise.bxor(k, k >>> 33)
    k = rem(k * 0xC4CE_B9FE_1A85_EC53, 1 <<< 64)
    Bitwise.bxor(k, k >>> 33)
  end

  defp leading_zeros(0, bit_width), do: bit_width
  defp leading_zeros(value, bit_width), do: max(0, bit_width - bit_length(value))

  defp bit_length(0), do: 0
  defp bit_length(value), do: bit_length(value, 0)
  defp bit_length(0, acc), do: acc
  defp bit_length(value, acc), do: bit_length(value >>> 1, acc + 1)

  defp alpha_for_registers(16), do: 0.673
  defp alpha_for_registers(32), do: 0.697
  defp alpha_for_registers(64), do: 0.709
  defp alpha_for_registers(m), do: 0.7213 / (1.0 + 1.079 / m)

  defp small_range_correction(estimate, registers, m) when estimate <= 2.5 * m do
    zeros = Enum.count(registers, &(&1 == 0))
    if zeros > 0, do: m * :math.log(m / zeros), else: estimate
  end
  defp small_range_correction(estimate, _registers, _m), do: estimate

  defp large_range_correction(estimate) do
    two_32 = :math.pow(2.0, 32)
    if estimate > two_32 / 30.0 do
      ratio = 1.0 - estimate / two_32
      if ratio > 0.0, do: -two_32 * :math.log(ratio), else: estimate
    else
      estimate
    end
  end
end
