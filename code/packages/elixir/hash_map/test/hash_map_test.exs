defmodule CodingAdventures.HashMapTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HashMap

  test "chaining put/get/delete works" do
    map =
      HashMap.new(strategy: :chaining)
      |> HashMap.put("a", 1)
      |> HashMap.put("b", 2)

    assert HashMap.get(map, "a") == 1
    assert HashMap.has_key?(map, "b")

    map = HashMap.delete(map, "a")
    refute HashMap.has_key?(map, "a")
    assert HashMap.size(map) == 1
  end

  test "open addressing preserves lookups through tombstones" do
    map =
      HashMap.new(strategy: :open_addressing, capacity: 4)
      |> HashMap.put("a", 1)
      |> HashMap.put("b", 2)
      |> HashMap.delete("a")

    assert HashMap.get(map, "b") == 2
  end

  test "entries and keys round trip" do
    map = HashMap.from_list([a: 1, b: 2], strategy: :chaining)
    assert Enum.sort(HashMap.keys(map)) == [:a, :b]
    assert Enum.sort(HashMap.values(map)) == [1, 2]
    assert Enum.sort(HashMap.entries(map)) == [a: 1, b: 2]
  end

  test "open addressing metadata and resizing" do
    map =
      HashMap.new(strategy: :open_addressing, capacity: 2, hash_fn: :djb2)
      |> HashMap.put("x", 1)
      |> HashMap.put("y", 2)

    assert HashMap.strategy(map) == :open_addressing
    assert HashMap.hash_fn(map) == :djb2
    assert HashMap.capacity(map) >= 2
    assert HashMap.load_factor(map) <= 1.0
    assert Enum.sort(HashMap.entries(map)) == [{"x", 1}, {"y", 2}]
  end

  test "delete missing is a no-op and from_list builds a map" do
    map = HashMap.from_list([{"k", "v"}], strategy: :chaining, hash_fn: :sha256)
    assert HashMap.get(map, "k") == "v"
    assert HashMap.delete(map, "missing") == map
  end

  test "open addressing delete preserves probe chains and fnv1a branch is used" do
    map =
      HashMap.new(strategy: :open_addressing, capacity: 4, hash_fn: :fnv1a)
      |> HashMap.put("cat", 1)
      |> HashMap.put("car", 2)
      |> HashMap.delete("cat")

    assert HashMap.get(map, "car") == 2
    refute HashMap.has_key?(map, "cat")
  end

  test "resizing is covered for both strategies" do
    chaining =
      HashMap.new(strategy: :chaining, capacity: 1)
      |> HashMap.put("a", 1)
      |> HashMap.put("b", 2)

    open_addressing =
      HashMap.new(strategy: :open_addressing, capacity: 1)
      |> HashMap.put("a", 1)
      |> HashMap.put("b", 2)

    assert HashMap.capacity(chaining) >= 2
    assert HashMap.capacity(open_addressing) >= 2
    assert Enum.sort(HashMap.keys(chaining)) == ["a", "b"]
    assert Enum.sort(HashMap.keys(open_addressing)) == ["a", "b"]
  end

  test "string options are normalized" do
    map = HashMap.new(strategy: "open-addressing", hash_fn: "phash2", capacity: 2)
    assert HashMap.strategy(map) == :open_addressing
    assert HashMap.hash_fn(map) == :phash2
    assert HashMap.capacity(map) == 2
  end

  test "invalid options raise" do
    assert_raise ArgumentError, fn -> HashMap.new(strategy: :bogus) end
    assert_raise ArgumentError, fn -> HashMap.new(hash_fn: :bogus) end
  end
end
