defmodule CodingAdventures.FenwickTreeTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FenwickTree

  test "new and from_list build prefix sums" do
    tree = FenwickTree.from_list([3, 2, 1, 7, 4])
    assert tree.length == 5
    refute FenwickTree.empty?(tree)

    Enum.each(Enum.with_index([3, 5, 6, 13, 17], 1), fn {expected, index} ->
      assert FenwickTree.prefix_sum(tree, index) == expected
    end)

    assert FenwickTree.empty?(FenwickTree.new(0))
    assert_raise ArgumentError, fn -> FenwickTree.new(-1) end
  end

  test "prefix range point and update operations" do
    tree = FenwickTree.from_list([3, 2, 1, 7, 4])
    assert FenwickTree.prefix_sum(tree, 0) == 0.0
    assert FenwickTree.range_sum(tree, 2, 4) == 10
    assert FenwickTree.point_query(tree, 4) == 7

    tree = FenwickTree.update(tree, 3, 5)
    assert FenwickTree.point_query(tree, 3) == 6
    assert FenwickTree.prefix_sum(tree, 3) == 11
  end

  test "find_kth examples" do
    tree = FenwickTree.from_list([1, 2, 3, 4, 5])
    assert FenwickTree.find_kth(tree, 1) == 1
    assert FenwickTree.find_kth(tree, 2) == 2
    assert FenwickTree.find_kth(tree, 3) == 2
    assert FenwickTree.find_kth(tree, 4) == 3
    assert FenwickTree.find_kth(tree, 10) == 4
  end

  test "find_kth errors" do
    assert_raise ArgumentError, fn -> FenwickTree.find_kth(FenwickTree.new(0), 1) end

    tree = FenwickTree.from_list([1, 2, 3])
    assert_raise ArgumentError, fn -> FenwickTree.find_kth(tree, 0) end
    assert_raise ArgumentError, fn -> FenwickTree.find_kth(tree, 100) end
  end

  test "invalid indices and ranges" do
    tree = FenwickTree.from_list([1, 2, 3])
    assert_raise ArgumentError, fn -> FenwickTree.prefix_sum(tree, 4) end
    assert_raise ArgumentError, fn -> FenwickTree.update(tree, 0, 1) end
    assert_raise ArgumentError, fn -> FenwickTree.range_sum(tree, 3, 1) end
    assert_raise ArgumentError, fn -> FenwickTree.range_sum(tree, 0, 3) end
  end

  test "brute force prefix sums and bit array" do
    values = [5, -2, 7, 1.5, 4.5]
    tree = FenwickTree.from_list(values)

    values
    |> Enum.with_index(1)
    |> Enum.each(fn {_value, index} ->
      assert FenwickTree.prefix_sum(tree, index) == values |> Enum.take(index) |> Enum.sum()
    end)

    assert length(FenwickTree.bit_array(tree)) == length(values)
  end
end
