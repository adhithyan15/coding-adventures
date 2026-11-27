defmodule GensShrinkTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 60_000

  import Bitwise
  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Pbt.{Budget, ExhaustedError, Gen, Gens, Hooks, Limits, NotFalsifyingError, Shrink}

  defp draw(gen, seed, n) do
    {vals, _} =
      Enum.map_reduce(1..n, Prng.new(seed), fn _, r -> Gen.sample(gen, r) end)

    vals
  end

  test "primitives" do
    assert draw(Gens.bool(), 0, 4) == [true, false, true, false]
    assert Enum.all?(draw(Gens.u8(), 1, 50), &(&1 in 0..255))
    assert Enum.all?(draw(Gens.u64(), 1, 20), &(&1 in 0..0xFFFF_FFFF_FFFF_FFFF))
    assert Enum.all?(draw(Gens.i128(), 1, 20), &(&1 >= -(1 <<< 127) and &1 < 1 <<< 127))
    assert draw(Gens.constant(:k), 1, 2) == [:k, :k]
  end

  test "integer is inclusive, bounded, any width" do
    vals = draw(Gens.integer(-3, 3), 5, 400)
    assert Enum.sort(Enum.uniq(vals)) == Enum.to_list(-3..3)
    huge = draw(Gens.integer(0, 1 <<< 200), 5, 10)
    assert Enum.all?(huge, &(&1 in 0..(1 <<< 200)))
    assert draw(Gens.integer(4, 4), 1, 3) == [4, 4, 4]
    assert_raise ArgumentError, fn -> Gens.integer(2, 1) end
    assert_raise ArgumentError, fn -> Gens.integer(1.0, 2) end
  end

  test "bytes, lists and tuples respect their bounds" do
    for b <- draw(Gens.bytes(2, 5), 3, 100), do: assert(byte_size(b) in 2..5)
    for l <- draw(Gens.list(Gens.u8(), 0, 4), 3, 100), do: assert(length(l) in 0..4)
    assert [{a, b}] = draw(Gens.tuple([Gens.u8(), Gens.bool()]), 1, 1)
    assert a in 0..255 and is_boolean(b)
    assert_raise ArgumentError, fn -> Gens.bytes(3, 2) end
    assert_raise ArgumentError, fn -> Gens.bytes(-1, 2) end
    assert_raise ArgumentError, fn -> Gens.bytes(0, Limits.max_input_bytes() + 1) end
    assert_raise ArgumentError, fn -> Gens.bytes(0.5, 2) end
    assert_raise ArgumentError, fn -> Gens.list(Gens.u8(), 0, Limits.max_list_len() + 1) end
    assert_raise ArgumentError, fn -> Gens.tuple([]) end
    assert_raise ArgumentError, fn -> Gens.tuple(List.duplicate(Gens.u8(), 65)) end
  end

  test "map, bind and one_of" do
    assert draw(Gens.map(Gens.integer(1, 1), &(&1 + 1)), 1, 1) == [2]
    bound = Gens.bind(Gens.integer(2, 2), fn n -> Gens.list(Gens.constant(n), n, n) end)
    assert draw(bound, 1, 1) == [[2, 2]]
    assert draw(Gens.and_then(Gens.integer(1, 1), fn n -> Gens.constant(n * 7) end), 1, 1) == [7]
    vals = draw(Gens.one_of([Gens.constant(:a), Gens.constant(:b)]), 1, 200)
    assert Enum.sort(Enum.uniq(vals)) == [:a, :b]
    assert_raise ArgumentError, fn -> Gens.one_of([]) end
    # map drops shrinking
    assert Gen.shrink(Gens.map(Gens.u8(), & &1), 200, 10, fn _ -> true end) == {200, 0}
  end

  test "frequency follows its weights and never picks a zero-weight arm" do
    gen = Gens.frequency([{0, Gens.constant(:never)}, {1, Gens.constant(:a)}, {3, Gens.constant(:b)}])
    vals = draw(gen, 11, 2000)
    refute :never in vals
    a = Enum.count(vals, &(&1 == :a))
    assert a > 350 and a < 650
    assert_raise ArgumentError, fn -> Gens.frequency([]) end
    assert_raise ArgumentError, fn -> Gens.frequency([{0, Gens.u8()}]) end
    assert_raise ArgumentError, fn -> Gens.frequency([{-1, Gens.u8()}]) end
    assert_raise ArgumentError, fn -> Gens.frequency([{1 <<< 32, Gens.u8()}, {1, Gens.u8()}]) end
    assert %Gen{} = Gens.frequency([{1 <<< 32, Gens.u8()}])
  end

  test "filter exhausts after exactly max_retries samples" do
    calls = :counters.new(1, [])

    gen =
      Gens.filter(Gens.u8(), 7, fn _ ->
        :counters.add(calls, 1, 1)
        false
      end)

    assert_raise ExhaustedError, fn -> Gen.sample(gen, Prng.new(1)) end
    assert :counters.get(calls, 1) == 7
    # only an exact `true` accepts
    assert_raise ExhaustedError, fn -> Gen.sample(Gens.filter(Gens.u8(), 3, fn _ -> :yes end), Prng.new(1)) end
  end

  test "integer shrinking stays inside the bounds and reaches the threshold" do
    gen = Gens.integer(5, 10)
    assert Gen.shrink(gen, 9, 100, fn x -> x >= 7 end) == {7, 1}
    {v, _} = Gen.shrink(gen, 10, 100, fn _ -> true end)
    assert v == 5

    gen2 = Gens.integer(-50, -10)
    {v, _} = Gen.shrink(gen2, -45, 100, fn x -> x <= -20 end)
    assert v == -20
    {v, _} = Gen.shrink(Gens.i128(), 1 <<< 100, 1000, fn x -> x >= 1000 end)
    assert v == 1000
    {v, _} = Gen.shrink(Gens.u64(), 0xFFFF_FFFF_FFFF_FFFF, 1000, fn x -> x >= 77 end)
    assert v == 77
    assert Gen.shrink(Gens.u8(), 200, 0, fn _ -> true end) == {200, 0}
  end

  test "bool shrinks true to false" do
    assert Gen.shrink(Gens.bool(), true, 5, fn _ -> true end) == {false, 1}
    assert Gen.shrink(Gens.bool(), true, 5, fn v -> v end) == {true, 0}
    assert Gen.shrink(Gens.bool(), false, 5, fn _ -> true end) == {false, 0}
    assert Gen.shrink(Gens.bool(), true, 0, fn _ -> true end) == {true, 0}
  end

  test "list shrinking keeps min_len and is 1-minimal" do
    gen = Gens.list(Gens.u8(), 2, 10)
    {v, _} = Gen.shrink(gen, [9, 9, 9, 9, 9], 1000, fn l -> Enum.sum(l) >= 10 end)
    assert length(v) == 2
    assert Enum.sum(v) >= 10
    {v, _} = Gen.shrink(Gens.list(Gens.u8(), 0, 10), [1, 2, 3], 1000, fn _ -> true end)
    assert v == []
  end

  test "tuple shrinking shrinks each coordinate" do
    gen = Gens.tuple([Gens.integer(0, 1000), Gens.integer(0, 1000)])
    {v, steps} = Gen.shrink(gen, {900, 800}, 1000, fn {a, b} -> a + b >= 100 end)
    assert {a, b} = v
    assert a + b >= 100
    assert steps > 0
    assert Gen.shrink(gen, {900, 800}, 0, fn _ -> true end) == {{900, 800}, 0}
  end

  test "bytes shrinking respects min_len" do
    gen = Gens.bytes(1, 8)
    {v, _} = Gen.shrink(gen, <<5, 6, 7, 8>>, 1000, fn b -> byte_size(b) >= 2 end)
    assert byte_size(v) == 2
    assert Gen.shrink(gen, <<5>>, 0, fn _ -> true end) == {<<5>>, 0}
  end

  test "filter shrinking only visits candidates that satisfy the predicate" do
    gen = Gens.filter(Gens.integer(0, 1000), 100, fn x -> rem(x, 2) == 0 end)
    {v, _} = Gen.shrink(gen, 900, 1000, fn x -> x >= 10 end)
    # a local minimum: 450 -> 225 and 449 are odd, so the predicate refuses both
    assert v == 450
  end

  test "gen corpus entry points are bounded" do
    assert_raise ArgumentError, fn -> Gens.gen_primitive(1, "u8", Limits.max_gen_count() + 1) end
    assert_raise ArgumentError, fn -> Gens.gen_primitive(1, "u8", -1) end
    assert_raise ArgumentError, fn -> Gens.gen_bounded(1, 5, 1, 1) end
    assert_raise ArgumentError, fn -> Gens.gen_bounded(1, 1.0, 2, 1) end
    assert_raise ArgumentError, fn -> Gens.gen_bounded(1, 0, 1 <<< 100_000, 1_000_000) end
    assert_raise ArgumentError, fn -> Gens.gen_bytes(1, 0, 0, Limits.max_gen_count() + 1) end
    assert_raise ArgumentError, fn -> Gens.gen_bytes(1, 0, 1000, 100_000) end
    assert Gens.gen_bytes(1, 0, 0, 3) == ["", "", ""]
    assert length(Gens.gen_bounded(1, 0, 9, 5)) == 5
    assert Gens.gen_primitive(0, "bool", 2) == [true, false]
    assert Gens.gen_primitive(1, "i128", 1) |> hd() |> is_integer()
  end

  test "new_rng hook changes the stream" do
    a = Gens.gen_primitive(1, "u64", 2)
    b = Hooks.with([new_rng: fn s -> Prng.new(s + 1) end], fn -> Gens.gen_primitive(1, "u64", 2) end)
    refute a == b
    assert_raise FunctionClauseError, fn -> Hooks.get(:nope) end
  end

  # -- public shrinkers ----------------------------------------------------------------------------

  test "shrink_int validates and raises on a non-falsifying start" do
    assert_raise NotFalsifyingError, fn -> Shrink.shrink_int(5, 0, 0, 10, 100, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_int(5, 0, 10, 0, 100, fn _ -> false end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_int(50, 0, 0, 10, 100, fn _ -> false end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_int(5.0, 0, 0, 10, 100, fn _ -> false end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_int(5, 0, 0, 10, 10_001, fn _ -> false end) end
    assert Shrink.shrink_int(5, 0, 0, 10, 0, fn _ -> false end) == {5, 0}
    # the target itself fails: one step
    assert Shrink.shrink_int(5, 0, 0, 10, 100, fn _ -> false end) == {0, 1}
    # a property that raises propagates from the public entry point
    assert_raise RuntimeError, fn -> Shrink.shrink_int(5, 0, 0, 10, 100, fn _ -> raise "p" end) end
  end

  test "shrink_list and shrink_bytes validate and raise on non-falsifying starts" do
    assert_raise NotFalsifyingError, fn -> Shrink.shrink_list([1], 0, 10, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_list(:x, 0, 10, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_list([1.5], 0, 10, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_list([1], 0.5, 10, fn _ -> true end) end
    assert_raise NotFalsifyingError, fn -> Shrink.shrink_bytes(<<1>>, 10, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Shrink.shrink_bytes(:x, 10, fn _ -> true end) end
    assert Shrink.shrink_list([3, 4], 0, 0, fn _ -> false end) == {[3, 4], 0}
    assert Shrink.shrink_bytes(<<3, 4>>, 1000, fn _ -> false end) == {"", 1}
    assert Shrink.shrink_list([3, 4], 0, 1000, fn _ -> false end) == {[], 1}
  end

  test "min_len > 0 probes the prefix" do
    # the shortest allowed list that still fails is taken in one step
    {v, steps} =
      Budget.enter_child(100, fn _ ->
        Shrink.shrink_sequence([5, 5, 5, 5], fn l -> length(l) >= 2 end, 100, 2, nil)
      end)

    assert v == [5, 5]
    assert steps == 1
  end

  test "simulate_replay is deterministic and rejects a property that never fails" do
    assert Shrink.simulate_replay(42, fn x -> x <= 100 end) == {413, 101}
    assert_raise ArgumentError, fn -> Shrink.simulate_replay(1, fn _ -> true end) end
  end

  test "skipping the deletion phase changes the result" do
    prop = fn l -> Enum.sum(l) <= 20 end
    normal = Shrink.shrink_list([10, 5, 8, 4], 0, 1000, prop)
    skipped = Hooks.with([skip_deletion_phase: true], fn -> Shrink.shrink_list([10, 5, 8, 4], 0, 1000, prop) end)
    refute normal == skipped
  end
end
