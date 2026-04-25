defmodule CodingAdventures.RngTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Rng.LCG
  alias CodingAdventures.Rng.Xorshift64
  alias CodingAdventures.Rng.PCG32

  # ── Helpers ──────────────────────────────────────────────────────────────────

  # Collect the first `n` values from a generator using `fun/1`.
  defp take(g, fun, n) do
    Enum.map_reduce(1..n, g, fn _, acc ->
      {v, acc1} = fun.(acc)
      {v, acc1}
    end)
    |> elem(0)
  end

  # ── LCG ──────────────────────────────────────────────────────────────────────

  describe "LCG" do
    test "new/1 returns {:ok, struct}" do
      assert {:ok, %LCG{}} = LCG.new(42)
    end

    test "seed=1 reference values match Go" do
      {:ok, g} = LCG.new(1)
      values = take(g, &LCG.next_u32/1, 3)
      assert values == [1_817_669_548, 2_187_888_307, 2_784_682_393]
    end

    test "next_u32 values are in [0, 2^32)" do
      {:ok, g} = LCG.new(42)
      values = take(g, &LCG.next_u32/1, 100)
      assert Enum.all?(values, fn v -> v >= 0 and v < 2 ** 32 end)
    end

    test "seed 0 produces valid output" do
      {:ok, g} = LCG.new(0)
      {v, _} = LCG.next_u32(g)
      assert v >= 0 and v < 2 ** 32
    end

    test "identical seeds produce identical streams" do
      {:ok, a} = LCG.new(999)
      {:ok, b} = LCG.new(999)
      assert take(a, &LCG.next_u32/1, 20) == take(b, &LCG.next_u32/1, 20)
    end

    test "different seeds produce different streams" do
      {:ok, a} = LCG.new(1)
      {:ok, b} = LCG.new(2)
      refute take(a, &LCG.next_u32/1, 10) == take(b, &LCG.next_u32/1, 10)
    end

    test "next_u64 returns value in [0, 2^64)" do
      {:ok, g} = LCG.new(7)
      values = take(g, &LCG.next_u64/1, 10)
      assert Enum.all?(values, fn v -> v >= 0 and v < 2 ** 64 end)
    end

    test "next_u64 equals (hi << 32) | lo from two next_u32 calls" do
      {:ok, g1} = LCG.new(5)
      {:ok, g2} = LCG.new(5)
      {u64, _}  = LCG.next_u64(g1)
      {hi, g2a} = LCG.next_u32(g2)
      {lo, _}   = LCG.next_u32(g2a)
      import Bitwise
      assert u64 == band(bor(bsl(hi, 32), lo), 0xFFFF_FFFF_FFFF_FFFF)
    end

    test "next_float returns value in [0.0, 1.0)" do
      {:ok, g} = LCG.new(13)
      values = take(g, &LCG.next_float/1, 200)
      assert Enum.all?(values, fn f -> f >= 0.0 and f < 1.0 end)
    end

    test "next_int_in_range stays within [1, 6]" do
      {:ok, g} = LCG.new(17)
      values = take(g, fn gen -> LCG.next_int_in_range(gen, 1, 6) end, 200)
      assert Enum.all?(values, fn v -> v >= 1 and v <= 6 end)
    end

    test "next_int_in_range with single value always returns min" do
      {:ok, g} = LCG.new(0)
      values = take(g, fn gen -> LCG.next_int_in_range(gen, 5, 5) end, 10)
      assert Enum.all?(values, fn v -> v == 5 end)
    end

    test "next_int_in_range covers all values in range" do
      {:ok, g} = LCG.new(99)
      values = take(g, fn gen -> LCG.next_int_in_range(gen, 1, 10) end, 1000)
      assert Enum.sort(Enum.uniq(values)) == Enum.to_list(1..10)
    end

    test "next_int_in_range works with negative range" do
      {:ok, g} = LCG.new(3)
      values = take(g, fn gen -> LCG.next_int_in_range(gen, -10, -1) end, 200)
      assert Enum.all?(values, fn v -> v >= -10 and v <= -1 end)
    end
  end

  # ── Xorshift64 ───────────────────────────────────────────────────────────────

  describe "Xorshift64" do
    test "new/1 returns {:ok, struct}" do
      assert {:ok, %Xorshift64{}} = Xorshift64.new(1)
    end

    test "seed=1 reference values match Go" do
      {:ok, g} = Xorshift64.new(1)
      values = take(g, &Xorshift64.next_u32/1, 3)
      assert values == [1_082_269_761, 201_397_313, 1_854_285_353]
    end

    test "seed 0 is replaced with 1" do
      {:ok, g_zero} = Xorshift64.new(0)
      {:ok, g_one}  = Xorshift64.new(1)
      {v0, _} = Xorshift64.next_u32(g_zero)
      {v1, _} = Xorshift64.next_u32(g_one)
      assert v0 == v1
    end

    test "next_u32 values are in [0, 2^32)" do
      {:ok, g} = Xorshift64.new(42)
      values = take(g, &Xorshift64.next_u32/1, 100)
      assert Enum.all?(values, fn v -> v >= 0 and v < 2 ** 32 end)
    end

    test "identical seeds produce identical streams" do
      {:ok, a} = Xorshift64.new(555)
      {:ok, b} = Xorshift64.new(555)
      assert take(a, &Xorshift64.next_u32/1, 20) == take(b, &Xorshift64.next_u32/1, 20)
    end

    test "different seeds produce different streams" do
      {:ok, a} = Xorshift64.new(10)
      {:ok, b} = Xorshift64.new(11)
      refute take(a, &Xorshift64.next_u32/1, 10) == take(b, &Xorshift64.next_u32/1, 10)
    end

    test "next_u64 returns value in [0, 2^64)" do
      {:ok, g} = Xorshift64.new(8)
      values = take(g, &Xorshift64.next_u64/1, 10)
      assert Enum.all?(values, fn v -> v >= 0 and v < 2 ** 64 end)
    end

    test "next_float returns value in [0.0, 1.0)" do
      {:ok, g} = Xorshift64.new(14)
      values = take(g, &Xorshift64.next_float/1, 200)
      assert Enum.all?(values, fn f -> f >= 0.0 and f < 1.0 end)
    end

    test "next_int_in_range stays within [1, 6]" do
      {:ok, g} = Xorshift64.new(18)
      values = take(g, fn gen -> Xorshift64.next_int_in_range(gen, 1, 6) end, 200)
      assert Enum.all?(values, fn v -> v >= 1 and v <= 6 end)
    end

    test "next_int_in_range with single value always returns min" do
      {:ok, g} = Xorshift64.new(0)
      values = take(g, fn gen -> Xorshift64.next_int_in_range(gen, 7, 7) end, 10)
      assert Enum.all?(values, fn v -> v == 7 end)
    end

    test "state never reaches zero across 1000 steps" do
      {:ok, g} = Xorshift64.new(1)
      values = take(g, &Xorshift64.next_u64/1, 1000)
      refute Enum.any?(values, fn v -> v == 0 end)
    end
  end

  # ── PCG32 ────────────────────────────────────────────────────────────────────

  describe "PCG32" do
    test "new/1 returns {:ok, struct}" do
      assert {:ok, %PCG32{}} = PCG32.new(1)
    end

    test "seed=1 reference values match Go" do
      {:ok, g} = PCG32.new(1)
      values = take(g, &PCG32.next_u32/1, 3)
      assert values == [1_412_771_199, 1_791_099_446, 124_312_908]
    end

    test "seed 0 produces valid output" do
      {:ok, g} = PCG32.new(0)
      {v, _} = PCG32.next_u32(g)
      assert v >= 0 and v < 2 ** 32
    end

    test "next_u32 values are in [0, 2^32)" do
      {:ok, g} = PCG32.new(123)
      values = take(g, &PCG32.next_u32/1, 100)
      assert Enum.all?(values, fn v -> v >= 0 and v < 2 ** 32 end)
    end

    test "identical seeds produce identical streams" do
      {:ok, a} = PCG32.new(777)
      {:ok, b} = PCG32.new(777)
      assert take(a, &PCG32.next_u32/1, 20) == take(b, &PCG32.next_u32/1, 20)
    end

    test "different seeds produce different streams" do
      {:ok, a} = PCG32.new(100)
      {:ok, b} = PCG32.new(200)
      refute take(a, &PCG32.next_u32/1, 10) == take(b, &PCG32.next_u32/1, 10)
    end

    test "next_u64 returns value in [0, 2^64)" do
      {:ok, g} = PCG32.new(9)
      values = take(g, &PCG32.next_u64/1, 10)
      assert Enum.all?(values, fn v -> v >= 0 and v < 2 ** 64 end)
    end

    test "next_u64 equals (hi << 32) | lo from two next_u32 calls" do
      {:ok, g1} = PCG32.new(5)
      {:ok, g2} = PCG32.new(5)
      {u64, _}  = PCG32.next_u64(g1)
      {hi, g2a} = PCG32.next_u32(g2)
      {lo, _}   = PCG32.next_u32(g2a)
      import Bitwise
      assert u64 == band(bor(bsl(hi, 32), lo), 0xFFFF_FFFF_FFFF_FFFF)
    end

    test "next_float returns value in [0.0, 1.0)" do
      {:ok, g} = PCG32.new(15)
      values = take(g, &PCG32.next_float/1, 200)
      assert Enum.all?(values, fn f -> f >= 0.0 and f < 1.0 end)
    end

    test "next_int_in_range stays within [1, 6]" do
      {:ok, g} = PCG32.new(19)
      values = take(g, fn gen -> PCG32.next_int_in_range(gen, 1, 6) end, 200)
      assert Enum.all?(values, fn v -> v >= 1 and v <= 6 end)
    end

    test "next_int_in_range with single value always returns min" do
      {:ok, g} = PCG32.new(0)
      values = take(g, fn gen -> PCG32.next_int_in_range(gen, 3, 3) end, 10)
      assert Enum.all?(values, fn v -> v == 3 end)
    end

    test "next_int_in_range works with negative range" do
      {:ok, g} = PCG32.new(5)
      values = take(g, fn gen -> PCG32.next_int_in_range(gen, -5, 0) end, 200)
      assert Enum.all?(values, fn v -> v >= -5 and v <= 0 end)
    end

    test "PCG32 output differs from LCG for same seed" do
      {:ok, pcg} = PCG32.new(1)
      {:ok, lcg} = LCG.new(1)
      pcg_vals = take(pcg, &PCG32.next_u32/1, 10)
      lcg_vals = take(lcg, &LCG.next_u32/1, 10)
      refute pcg_vals == lcg_vals
    end

    test "next_int_in_range covers all values across larger range" do
      {:ok, g} = PCG32.new(42)
      values = take(g, fn gen -> PCG32.next_int_in_range(gen, 1, 10) end, 1000)
      assert Enum.sort(Enum.uniq(values)) == Enum.to_list(1..10)
    end
  end
end
