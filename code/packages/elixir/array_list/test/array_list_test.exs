defmodule CodingAdventures.ArrayListTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ArrayList

  test "push and pop on both ends across chunk boundaries" do
    list = ArrayList.new(chunk_size: 2)
    list = Enum.reduce(["a", "b", "c"], list, &ArrayList.push_right(&2, &1))
    list = ArrayList.push_left(list, "z")

    assert ArrayList.to_list(list) == ["z", "a", "b", "c"]
    assert ArrayList.chunk_count(list) == 3

    {list, left} = ArrayList.pop_left(list)
    assert left == "z"
    {list, right} = ArrayList.pop_right(list)
    assert right == "c"
    assert ArrayList.to_list(list) == ["a", "b"]
  end

  test "index and range use the chunked layout" do
    list = ArrayList.from_list(["a", "b", "c", "d", "e"], chunk_size: 2)
    assert ArrayList.index(list, 0) == "a"
    assert ArrayList.index(list, -1) == "e"
    assert ArrayList.index(list, 9) == nil
    assert ArrayList.range(list, 1, 3) == ["b", "c", "d"]
    assert ArrayList.range(list, -3, -1) == ["c", "d", "e"]
    assert ArrayList.range(list, 5, 6) == []
  end

  test "metadata helpers and invalid options" do
    list = ArrayList.from_list([1, 2, 3], chunk_size: 2)
    assert ArrayList.len(list) == 3
    refute ArrayList.is_empty(list)
    assert ArrayList.chunk_size(list) == 2
    assert ArrayList.pop_left(ArrayList.new()) == {ArrayList.new(), nil}
    assert ArrayList.pop_right(ArrayList.new()) == {ArrayList.new(), nil}
    assert_raise ArgumentError, fn -> ArrayList.new(chunk_size: 0) end
  end
end
