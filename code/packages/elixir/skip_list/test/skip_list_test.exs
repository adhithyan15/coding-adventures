defmodule CodingAdventures.SkipListTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.SkipList

  test "insert search delete and ordering" do
    list =
      SkipList.new()
      |> SkipList.insert(3, "three")
      |> SkipList.insert(1, "one")
      |> SkipList.insert(2, "two")

    assert SkipList.keys(list) == [1, 2, 3]
    assert SkipList.search(list, 2) == "two"
    assert SkipList.rank(list, 3) == 2
    assert SkipList.by_rank(list, 0) == 1
    assert SkipList.min(list) == 1
    assert SkipList.max(list) == 3

    list = SkipList.delete(list, 2)
    refute SkipList.contains?(list, 2)
  end

  test "range returns sorted pairs" do
    list = SkipList.from_list([{3, "three"}, {5, "five"}, {7, "seven"}])
    assert SkipList.range(list, 3, 5, true) == [{3, "three"}, {5, "five"}]
    assert SkipList.range(list, 3, 5, false) == []
  end

  test "empty skip list reports emptiness and missing lookups" do
    list = SkipList.new()
    assert SkipList.size(list) == 0
    assert SkipList.is_empty(list)
    assert SkipList.search(list, 99) == nil
    assert SkipList.rank(list, 99) == nil
    assert SkipList.by_rank(list, 0) == nil
    assert SkipList.min(list) == nil
    assert SkipList.max(list) == nil
  end
end
