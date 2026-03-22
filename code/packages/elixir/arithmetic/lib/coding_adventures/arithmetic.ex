# === Arithmetic — Layer 2 of the computing stack ===
#
# Half adder, full adder, ripple carry adder, and ALU.
# Built entirely from logic gates (Layer 1).
#
# This is how real CPUs do math: chains of simple gate circuits
# that propagate carry bits from the least significant bit to
# the most significant. No magic, just careful wiring.

defmodule CodingAdventures.Arithmetic do
  @moduledoc """
  Arithmetic circuits built from logic gates.

  Provides half adder, full adder, ripple carry adder, and a
  configurable N-bit ALU with ADD, SUB, AND, OR, XOR, NOT operations.
  """

  alias CodingAdventures.LogicGates.Gates

  # ---------------------------------------------------------------------------
  # Adders
  # ---------------------------------------------------------------------------

  @doc """
  Half adder — adds two single bits.

  Returns `{sum, carry}`.

  Truth table:
      0 + 0 = 0, carry 0
      0 + 1 = 1, carry 0
      1 + 0 = 1, carry 0
      1 + 1 = 0, carry 1  (1+1 = 10 in binary)
  """
  def half_adder(a, b) do
    sum = Gates.xor_gate(a, b)
    carry = Gates.and_gate(a, b)
    {sum, carry}
  end

  @doc """
  Full adder — adds two bits plus a carry-in.

  Returns `{sum, carry_out}`.

  Built from two half adders and an OR gate:
    1. Half-add a and b → partial_sum, partial_carry
    2. Half-add partial_sum and carry_in → sum, carry2
    3. carry_out = OR(partial_carry, carry2)
  """
  def full_adder(a, b, carry_in) do
    {partial_sum, partial_carry} = half_adder(a, b)
    {sum, carry2} = half_adder(partial_sum, carry_in)
    carry_out = Gates.or_gate(partial_carry, carry2)
    {sum, carry_out}
  end

  @doc """
  Ripple carry adder — adds two N-bit numbers using chained full adders.

  Both `a` and `b` are lists of bits in LSB-first order.
  Returns `{sum_bits, carry_out}`.
  """
  def ripple_carry_adder(a, b, carry_in \\ 0) do
    {sum_bits, carry_out} =
      Enum.zip(a, b)
      |> Enum.reduce({[], carry_in}, fn {a_bit, b_bit}, {acc, carry} ->
        {sum, new_carry} = full_adder(a_bit, b_bit, carry)
        {[sum | acc], new_carry}
      end)

    {Enum.reverse(sum_bits), carry_out}
  end

  # ---------------------------------------------------------------------------
  # ALU
  # ---------------------------------------------------------------------------

  @type alu_op :: :add | :sub | :and_op | :or_op | :xor_op | :not_op

  defmodule ALUResult do
    @moduledoc "Result of an ALU operation."
    defstruct [:value, :zero, :carry, :negative, :overflow]
  end

  @doc """
  Execute an ALU operation on two N-bit inputs.

  Operations: :add, :sub, :and_op, :or_op, :xor_op, :not_op

  Returns `%ALUResult{}` with value (bit list), zero, carry, negative, overflow.
  """
  def alu_execute(op, a, b) do
    {value, carry} = execute_op(op, a, b)

    zero = Enum.all?(value, &(&1 == 0))
    negative = List.last(value) == 1
    overflow = compute_overflow(op, a, b, value)

    %ALUResult{
      value: value,
      zero: zero,
      carry: carry,
      negative: negative,
      overflow: overflow
    }
  end

  defp execute_op(:add, a, b) do
    {sum, carry_bit} = ripple_carry_adder(a, b)
    {sum, carry_bit == 1}
  end

  defp execute_op(:sub, a, b) do
    # A - B = A + NOT(B) + 1 (two's complement)
    inverted = Enum.map(b, &Gates.not_gate/1)
    one = [1 | List.duplicate(0, length(b) - 1)]
    {neg_b, _} = ripple_carry_adder(inverted, one)
    {sum, carry_bit} = ripple_carry_adder(a, neg_b)
    {sum, carry_bit == 1}
  end

  defp execute_op(:and_op, a, b) do
    {Enum.zip_with(a, b, &Gates.and_gate/2), false}
  end

  defp execute_op(:or_op, a, b) do
    {Enum.zip_with(a, b, &Gates.or_gate/2), false}
  end

  defp execute_op(:xor_op, a, b) do
    {Enum.zip_with(a, b, &Gates.xor_gate/2), false}
  end

  defp execute_op(:not_op, a, _b) do
    {Enum.map(a, &Gates.not_gate/1), false}
  end

  defp compute_overflow(op, a, b, result) when op in [:add, :sub] do
    a_sign = List.last(a)
    b_sign = if op == :add, do: List.last(b), else: Gates.not_gate(List.last(b))
    result_sign = List.last(result)
    a_sign == b_sign and result_sign != a_sign
  end

  defp compute_overflow(_, _, _, _), do: false
end
