defmodule CodingAdventures.HashSetTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HashSet

  test "add delete and membership" do
    set =
      HashSet.new(strategy: :chaining)
      |> HashSet.add(:a)
      |> HashSet.add(:b)

    assert HashSet.has?(set, :a)
    assert HashSet.size(set) == 2

    set = HashSet.delete(set, :a)
    refute HashSet.has?(set, :a)
    assert HashSet.size(set) == 1
  end

  test "set algebra works" do
    left = HashSet.from_values([1, 2, 3])
    right = HashSet.from_values([3, 4])

    assert HashSet.union(left, right) |> HashSet.to_list() |> Enum.sort() == [1, 2, 3, 4]
    assert HashSet.intersection(left, right) |> HashSet.to_list() |> Enum.sort() == [3]
    assert HashSet.difference(left, right) |> HashSet.to_list() |> Enum.sort() == [1, 2]
    assert HashSet.symmetric_difference(left, right) |> HashSet.to_list() |> Enum.sort() == [1, 2, 4]
    assert HashSet.subset?(HashSet.from_values([1, 2]), left)
    assert HashSet.superset?(left, HashSet.from_values([1, 2]))
    assert HashSet.disjoint?(HashSet.from_values([10]), left)
  end
end
