defmodule CodingAdventures.TreeSetTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.TreeSet

  test "ordered operations and set algebra" do
    set = TreeSet.from_values([3, 1, 2])
    assert TreeSet.to_list(set) == [1, 2, 3]
    assert TreeSet.min(set) == 1
    assert TreeSet.max(set) == 3
    assert TreeSet.rank(set, 2) == 1
    assert TreeSet.by_rank(set, 0) == 1
    assert TreeSet.predecessor(set, 2) == 1
    assert TreeSet.successor(set, 2) == 3

    other = TreeSet.from_values([3, 4])
    assert TreeSet.union(set, other) |> TreeSet.to_list() |> Enum.sort() == [1, 2, 3, 4]
    assert TreeSet.intersection(set, other) |> TreeSet.to_list() == [3]
    assert TreeSet.difference(set, other) |> TreeSet.to_list() == [1, 2]
    assert TreeSet.symmetric_difference(set, other) |> TreeSet.to_list() |> Enum.sort() == [1, 2, 4]
    assert TreeSet.subset?(TreeSet.from_values([1, 2]), set)
    assert TreeSet.superset?(set, TreeSet.from_values([1, 2]))
    assert TreeSet.disjoint?(TreeSet.from_values([10]), set)
  end

  test "empty set helpers and delete/has work" do
    set = TreeSet.new()
    assert TreeSet.is_empty(set)
    assert TreeSet.size(set) == 0
    assert TreeSet.has?(TreeSet.add(set, 10), 10)
    assert TreeSet.delete(TreeSet.add(set, 10), 10) |> TreeSet.is_empty()
    assert TreeSet.range(TreeSet.from_values([1, 2, 3]), 2, 3, true) == [2, 3]
  end
end
