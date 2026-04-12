defmodule CodingAdventures.LinkedListTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LinkedList

  test "push and pop on both ends" do
    list =
      LinkedList.new()
      |> LinkedList.push_left("b")
      |> LinkedList.push_left("a")
      |> LinkedList.push_right("c")

    assert LinkedList.to_list(list) == ["a", "b", "c"]

    {list, left} = LinkedList.pop_left(list)
    assert left == "a"
    {list, right} = LinkedList.pop_right(list)
    assert right == "c"
    assert LinkedList.to_list(list) == ["b"]
  end

  test "range and index cover empty and negative paths" do
    list = LinkedList.from_list(["a", "b", "c", "d"])
    assert LinkedList.index(list, 1) == "b"
    assert LinkedList.index(list, -1) == "d"
    assert LinkedList.range(list, 1, 2) == ["b", "c"]
    assert LinkedList.range(list, -3, -2) == ["b", "c"]
    assert LinkedList.range(list, 4, 5) == []
  end

  test "metadata helpers" do
    list = LinkedList.from_list([1, 2, 3])
    assert LinkedList.len(list) == 3
    refute LinkedList.is_empty(list)
    assert LinkedList.pop_left(LinkedList.new()) == {LinkedList.new(), nil}
    assert LinkedList.pop_right(LinkedList.new()) == {LinkedList.new(), nil}
  end
end
