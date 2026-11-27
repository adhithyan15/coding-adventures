defmodule PrngPinsTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 60_000

  import Bitwise
  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Pbt.{Gens, Shrink}

  @mask 0xFFFF_FFFF_FFFF_FFFF
  @m1 0xBF58_476D_1CE4_E5B9
  @m2 0x94D0_49BB_1331_11EB
  @gamma 0x9E37_79B9_7F4A_7C15

  # -- inverting the SplitMix64 finalizer, to FORCE a chosen first output -----------------
  defp unxor(y, s), do: Enum.reduce(1..(div(64, s) + 1), y, fn _, acc -> bxor(y, acc >>> s) end)

  defp mul_inv(m), do: Enum.reduce(1..7, m, fn _, x -> band(x * (2 - band(m * x, @mask)), @mask) end)

  # a seed whose FIRST next_u64 output is exactly `out`
  defp seed_for(out) do
    z2 = unxor(out, 31)
    z1 = unxor(band(z2 * mul_inv(@m2), @mask), 27)
    s = unxor(band(z1 * mul_inv(@m1), @mask), 30)
    band(s - @gamma, @mask)
  end

  test "the inverter really forces the first word" do
    for out <- [0, 1, 2, 12_345, 1 <<< 63, @mask, (1 <<< 63) - 2, (1 <<< 63) - 1] do
      assert {^out, _} = Prng.next_u64(Prng.new(seed_for(out)))
    end
  end

  test "reference vector: seed 0 first output" do
    assert {0xE220A8397B1DCDAF, _} = Prng.next_u64(Prng.new(0))
  end

  test "a draw below the rejection threshold is redrawn (n = 3, threshold 1)" do
    rng = Prng.new(seed_for(0))
    {w0, r1} = Prng.next_u64(rng)
    {w1, r2} = Prng.next_u64(r1)
    assert w0 == 0
    {v, after_rng} = Prng.bounded(rng, 3)
    assert v == rem(w1, 3)
    # exactly two words were consumed
    assert after_rng == r2
  end

  test "a draw equal to the threshold is kept (n = 3, threshold 1)" do
    rng = Prng.new(seed_for(1))
    {v, after_rng} = Prng.bounded(rng, 3)
    {_, r1} = Prng.next_u64(rng)
    assert v == 1
    assert after_rng == r1
  end

  test "the 2^63 + 1 threshold boundary" do
    n = (1 <<< 63) + 1
    threshold = rem((1 <<< 64) - n, n)
    assert threshold == (1 <<< 63) - 1

    # one below the threshold: rejected, the next word decides
    below = Prng.new(seed_for(threshold - 1))
    {_, r1} = Prng.next_u64(below)
    {w1, r2} = Prng.next_u64(r1)
    {v, rng_after} = Prng.bounded(below, n)
    assert v == rem(w1, n)
    assert rng_after == r2

    # exactly the threshold: kept
    at = Prng.new(seed_for(threshold))
    {v, rng_after} = Prng.bounded(at, n)
    assert v == threshold
    assert rng_after == elem(Prng.next_u64(at), 1)

    # above n the residue wraps
    top = Prng.new(seed_for(@mask))
    assert {v, _} = Prng.bounded(top, n)
    assert v == rem(@mask, n)
  end

  test "bounded(1) consumes one word and returns 0" do
    rng = Prng.new(7)
    {0, r} = Prng.bounded(rng, 1)
    assert r == elem(Prng.next_u64(rng), 1)
  end

  test "bounded(2^64) is one raw word" do
    rng = Prng.new(9)
    assert Prng.bounded(rng, 1 <<< 64) == Prng.next_u64(rng)
  end

  test "bounded arguments are validated" do
    rng = Prng.new(1)
    assert_raise ArgumentError, fn -> Prng.bounded(rng, 0) end
    assert_raise ArgumentError, fn -> Prng.bounded(rng, -3) end
    assert_raise ArgumentError, fn -> Prng.bounded(rng, (1 <<< 64) + 1) end
    assert_raise ArgumentError, fn -> Prng.bounded(rng, 2.0) end
    assert_raise ArgumentError, fn -> Prng.bounded_big(rng, 0) end
    assert_raise ArgumentError, fn -> Prng.bounded_big(rng, :x) end
  end

  test "seeds are reduced modulo 2^64 and must be integers" do
    assert Prng.new(-1) == Prng.new(@mask)
    assert Prng.new((1 <<< 64) + 5) == Prng.new(5)
    for bad <- [true, 1.5, "1", nil], do: assert_raise(ArgumentError, fn -> Prng.new(bad) end)
  end

  test "bounded_big spans above 2^64 draw k words, high first" do
    span = (1 <<< 64) + 1
    rng = Prng.new(3)
    {w0, r1} = Prng.next_u64(rng)
    {w1, r2} = Prng.next_u64(r1)
    {v, after_rng} = Prng.bounded_big(rng, span)
    assert v == rem((w0 <<< 64) ||| w1, span)
    assert after_rng == r2

    huge = 1 <<< 200
    {v, _} = Prng.bounded_big(rng, huge)
    assert v in 0..(huge - 1)
  end

  test "bit_length" do
    assert Prng.bit_length(0) == 0
    assert Prng.bit_length(1) == 1
    assert Prng.bit_length(255) == 8
    assert Prng.bit_length(256) == 9
    assert Prng.bit_length((1 <<< 100) - 1) == 100
  end

  test "FNV-1a 64 vectors and sub_seed" do
    assert Prng.fnv1a64("") == 0xCBF29CE484222325
    assert Prng.fnv1a64("a") == 0xAF63DC4C8601EC8C
    assert Prng.fnv1a64("foobar") == 0x85944171F73967E8
    assert_raise ArgumentError, fn -> Prng.fnv1a64(:x) end

    expected = elem(Prng.next_u64(Prng.new(bxor(42, Prng.fnv1a64("prop/x")))), 0)
    assert Prng.sub_seed(42, "prop/x") == expected
    assert Prng.sub_seed(-1, "p") == Prng.sub_seed(@mask, "p")
    assert_raise ArgumentError, fn -> Prng.sub_seed("x", "y") end
  end

  test "an rng never prints its state" do
    refute inspect(Prng.new(123_456_789)) =~ "123456789"
  end

  # -- bisection: both halves, exact step counts ---------------------------------------------

  test "bisection accepts the right half and then the left half" do
    # holds iff every element <= 9; only the right half [50, 60, 70, 80] fails.
    prop = fn l -> Enum.all?(l, &(&1 <= 9)) end
    # steps: right half (1), left half [50,60] (2), left [50] (3), then 50 -> 10 in 5 moves
    assert Shrink.shrink_list([1, 2, 3, 4, 50, 60, 70, 80], 0, 1000, prop) == {[10], 8}
  end

  test "bisection steps are counted one per accepted half" do
    prop = fn l -> Enum.all?(l, &(&1 == 0)) end
    # [0,0,0,0,0,0,0,7]: only the right side fails; halves: right (1), right (2), right (3) -> [7]
    # then 7 -> 0 is not falsifying; 7 -> 4 -> ... walk toward 0 stays failing until it holds
    {minimal, steps} = Shrink.shrink_list([0, 0, 0, 0, 0, 0, 0, 7], 0, 1000, prop)
    assert minimal == [1]
    assert steps == 3 + 3
  end

  test "the left half wins when both halves fail" do
    prop = fn l -> length(l) < 2 end
    # [1,2,3,4]: the empty probe holds; the left half [1,2] fails and is taken (1);
    # then neither [1] nor [2] fails and no element can be deleted; the integer walk
    # takes 1 -> 0 (one move) and 2 -> 1 -> 0 (two moves): 1 + 1 + 2 = 4.
    assert Shrink.shrink_list([1, 2, 3, 4], 0, 1000, prop) == {[0, 0], 4}
  end

  test "gen primitives are the first words of the stream" do
    assert Gens.gen_primitive(0, "u64", 1) == [0xE220A8397B1DCDAF]
    assert_raise ArgumentError, fn -> Gens.gen_primitive(0, "nope", 1) end
  end
end
