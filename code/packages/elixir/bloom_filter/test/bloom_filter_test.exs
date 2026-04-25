defmodule CodingAdventures.BloomFilterTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BloomFilter

  test "new filter starts empty" do
    filter = BloomFilter.new()
    assert filter.bits_set == 0
    assert BloomFilter.fill_ratio(filter) == 0.0
    assert BloomFilter.estimated_false_positive_rate(filter) == 0.0
    refute BloomFilter.over_capacity?(filter)
    refute BloomFilter.contains?(filter, "anything")
  end

  test "has no false negatives for inserted values" do
    filter =
      Enum.reduce(0..249, BloomFilter.new(1_000, 0.01), fn i, filter ->
        BloomFilter.add(filter, "item-#{i}")
      end)

    Enum.each(0..249, fn i ->
      assert BloomFilter.contains?(filter, "item-#{i}")
    end)

    assert filter.bits_set > 0
  end

  test "supports explicit parameters" do
    filter = BloomFilter.from_params(10_000, 7)
    assert filter.bit_count == 10_000
    assert filter.hash_count == 7
    assert BloomFilter.size_bytes(filter) == 1_250

    filter = BloomFilter.add(filter, "hello")
    assert BloomFilter.contains?(filter, "hello")
    refute BloomFilter.over_capacity?(filter)
  end

  test "duplicate adds do not double count bits" do
    filter = BloomFilter.new() |> BloomFilter.add("dup")
    after_first = filter.bits_set
    filter = BloomFilter.add(filter, "dup")
    assert filter.bits_set == after_first
  end

  test "computes sizing helpers" do
    m = BloomFilter.optimal_m(1_000_000, 0.01)
    k = BloomFilter.optimal_k(m, 1_000_000)
    assert m > 9_000_000
    assert k == 7
    assert BloomFilter.capacity_for_memory(1_000_000, 0.01) > 0
  end

  test "detects over-capacity filters" do
    filter =
      ["a", "b", "c"]
      |> Enum.reduce(BloomFilter.new(3, 0.01), fn value, filter ->
        BloomFilter.add(filter, value)
      end)

    refute BloomFilter.over_capacity?(filter)
    filter = BloomFilter.add(filter, "d")
    assert BloomFilter.over_capacity?(filter)
    assert BloomFilter.estimated_false_positive_rate(filter) > 0
  end

  test "handles varied element types" do
    filter =
      [42, 3.14, true, :atom, "cafe\u0301"]
      |> Enum.reduce(BloomFilter.new(100, 0.01), fn value, filter ->
        filter = BloomFilter.add(filter, value)
        assert BloomFilter.contains?(filter, value)
        filter
      end)

    assert filter.bits_set > 0
  end

  test "rejects invalid parameters" do
    assert_raise ArgumentError, fn -> BloomFilter.new(0, 0.01) end
    assert_raise ArgumentError, fn -> BloomFilter.new(1, 0) end
    assert_raise ArgumentError, fn -> BloomFilter.new(1, 1) end
    assert_raise ArgumentError, fn -> BloomFilter.from_params(0, 1) end
    assert_raise ArgumentError, fn -> BloomFilter.from_params(1, 0) end
  end
end
