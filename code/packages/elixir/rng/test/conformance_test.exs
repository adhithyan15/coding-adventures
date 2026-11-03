defmodule CodingAdventures.Rng.ConformanceTest do
  # The shared rng corpus (spec rng.md, "Conformance corpus"), read from its
  # flat form code/specs/fixtures/rng-v1/cases.txt.
  #
  # The corpus pins exact outputs every language's rng package must reproduce;
  # its results come from the independent reference
  # code/scripts/rng_conformance.py. Each line is whitespace-separated:
  #
  #     case <id> <generator> <seed>
  #     next_u32 <result>
  #     next_u64 <result>
  #     next_float <16 hex digits of the IEEE-754 bits>
  #     next_int_in_range <min> <max> <result | error>
  #     end
  #
  # mix test runs from the package directory.
  use ExUnit.Case, async: true

  import Bitwise, only: [<<<: 2]

  alias CodingAdventures.Rng.{LCG, PCG32, Xorshift64}

  @corpus "../../../specs/fixtures/rng-v1/cases.txt"
  @i64_min -0x8000000000000000
  @i64_max 0x7FFFFFFFFFFFFFFF

  defp module_for("lcg"), do: LCG
  defp module_for("xorshift64"), do: Xorshift64
  defp module_for("pcg32"), do: PCG32

  defp float_bits(x) do
    <<bits::unsigned-big-64>> = <<x::float-big-64>>
    bits |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(16, "0")
  end

  # Fold one corpus line into {module, generator, id, cases, calls}.
  defp step(line, {mod, g, id, cases, calls}) do
    case String.split(line, " ") do
      ["case", new_id, name, seed] ->
        m = module_for(name)
        {:ok, g0} = m.new(String.to_integer(seed))
        {m, g0, new_id, cases + 1, calls}

      ["end"] ->
        {nil, nil, id, cases, calls}

      ["next_u32", want] ->
        {got, g1} = mod.next_u32(g)
        assert got == String.to_integer(want), id
        {mod, g1, id, cases, calls + 1}

      ["next_u64", want] ->
        {got, g1} = mod.next_u64(g)
        assert got == String.to_integer(want), id
        {mod, g1, id, cases, calls + 1}

      ["next_float", want] ->
        {got, g1} = mod.next_float(g)
        assert float_bits(got) == want, id
        {mod, g1, id, cases, calls + 1}

      ["next_int_in_range", lo, hi, "error"] ->
        # Raised before any draw, so the same generator carries on.
        assert_raise ArgumentError, fn ->
          mod.next_int_in_range(g, String.to_integer(lo), String.to_integer(hi))
        end

        {mod, g, id, cases, calls + 1}

      ["next_int_in_range", lo, hi, want] ->
        {got, g1} = mod.next_int_in_range(g, String.to_integer(lo), String.to_integer(hi))
        assert got == String.to_integer(want), id
        {mod, g1, id, cases, calls + 1}

      other ->
        flunk("malformed corpus line: #{inspect(other)}")
    end
  end

  test "every rng-v1 corpus case" do
    {_, _, _, cases, calls} =
      @corpus
      |> File.read!()
      |> String.split("\n", trim: true)
      |> Enum.reject(&String.starts_with?(&1, "#"))
      |> Enum.reduce({nil, nil, "", 0, 0}, &step/2)

    # Guard against a truncated or empty file passing vacuously.
    assert cases >= 27
    assert calls >= 150
  end

  describe "regressions for the v1 range bug" do
    test "wide ranges reach beyond 2^32" do
      {values, _} =
        Enum.map_reduce(1..64, elem(PCG32.new(3), 1), fn _, g ->
          PCG32.next_int_in_range(g, 0, 1 <<< 40)
        end)

      assert Enum.any?(values, &(&1 >= 1 <<< 32))
    end

    test "the whole signed 64-bit domain" do
      {values, _} =
        Enum.map_reduce(1..64, elem(LCG.new(3), 1), fn _, g ->
          LCG.next_int_in_range(g, @i64_min, @i64_max)
        end)

      assert Enum.all?(values, &(&1 >= @i64_min and &1 <= @i64_max))
      assert Enum.any?(values, &(&1 < 0)) and Enum.any?(values, &(&1 > 0))
    end

    test "invalid bounds raise" do
      {:ok, g} = Xorshift64.new(9)

      for {lo, hi} <- [{1, 0}, {@i64_min - 1, 0}, {0, @i64_max + 1}, {0.5, 2}, {"a", "b"}] do
        assert_raise ArgumentError, fn -> Xorshift64.next_int_in_range(g, lo, hi) end
      end
    end
  end
end
