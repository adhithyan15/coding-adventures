defmodule CodingAdventures.RadixTreeTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.RadixTree

  test "put delete and lookup work" do
    tree =
      RadixTree.new()
      |> RadixTree.put("foo", 1)
      |> RadixTree.put("foobar", 2)

    assert RadixTree.get(tree, "foo") == 1
    assert RadixTree.has?(tree, "foobar")
    assert RadixTree.size(tree) == 2
    assert RadixTree.words_with_prefix(tree, "foo") == ["foo", "foobar"]
    assert RadixTree.longest_prefix_match(tree, "foobarbaz") == "foobar"

    tree = RadixTree.delete(tree, "foo")
    refute RadixTree.has?(tree, "foo")
    assert RadixTree.keys(tree) == ["foobar"]
  end

  test "from_list and metadata helpers" do
    tree = RadixTree.from_list([{"b", 2}, {"a", 1}])
    assert RadixTree.to_map(tree) == %{"a" => 1, "b" => 2}
    assert RadixTree.values(tree) == [1, 2]
    assert RadixTree.starts_with?(tree, "a")
    refute RadixTree.is_empty(tree)
  end
end
