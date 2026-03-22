defmodule CodingAdventures.StarlarkVm.BuiltinsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkVm.Builtins
  alias CodingAdventures.StarlarkVm.Handlers

  # ===========================================================================
  # Type Functions
  # ===========================================================================

  test "type of integer" do
    assert Builtins.builtin_type([42], nil) == "int"
  end

  test "type of float" do
    assert Builtins.builtin_type([3.14], nil) == "float"
  end

  test "type of string" do
    assert Builtins.builtin_type(["hello"], nil) == "string"
  end

  test "type of boolean" do
    assert Builtins.builtin_type([true], nil) == "bool"
  end

  test "type of nil" do
    assert Builtins.builtin_type([nil], nil) == "NoneType"
  end

  test "type of list" do
    assert Builtins.builtin_type([[1, 2]], nil) == "list"
  end

  test "type of dict" do
    assert Builtins.builtin_type([%{}], nil) == "dict"
  end

  test "type of tuple" do
    assert Builtins.builtin_type([{1, 2}], nil) == "tuple"
  end

  test "type of function" do
    func = %Handlers.StarlarkFunction{}
    assert Builtins.builtin_type([func], nil) == "function"
  end

  # ===========================================================================
  # Bool Function
  # ===========================================================================

  test "bool of truthy int" do
    assert Builtins.builtin_bool([1], nil) == true
  end

  test "bool of zero" do
    assert Builtins.builtin_bool([0], nil) == false
  end

  test "bool of empty string" do
    assert Builtins.builtin_bool([""], nil) == false
  end

  test "bool of non-empty string" do
    assert Builtins.builtin_bool(["hi"], nil) == true
  end

  test "bool of nil" do
    assert Builtins.builtin_bool([nil], nil) == false
  end

  test "bool of empty list" do
    assert Builtins.builtin_bool([[]], nil) == false
  end

  test "bool of non-empty list" do
    assert Builtins.builtin_bool([[1]], nil) == true
  end

  # ===========================================================================
  # Int Function
  # ===========================================================================

  test "int from int" do
    assert Builtins.builtin_int([42], nil) == 42
  end

  test "int from float" do
    assert Builtins.builtin_int([3.7], nil) == 3
  end

  test "int from string" do
    assert Builtins.builtin_int(["123"], nil) == 123
  end

  test "int from string with base" do
    assert Builtins.builtin_int(["ff", 16], nil) == 255
  end

  # ===========================================================================
  # Float Function
  # ===========================================================================

  test "float from int" do
    assert Builtins.builtin_float([42], nil) == 42.0
  end

  test "float from float" do
    assert Builtins.builtin_float([3.14], nil) == 3.14
  end

  # ===========================================================================
  # Str Function
  # ===========================================================================

  test "str of int" do
    assert Builtins.builtin_str([42], nil) == "42"
  end

  test "str of nil" do
    assert Builtins.builtin_str([nil], nil) == "None"
  end

  test "str of true" do
    assert Builtins.builtin_str([true], nil) == "True"
  end

  test "str of false" do
    assert Builtins.builtin_str([false], nil) == "False"
  end

  # ===========================================================================
  # Len Function
  # ===========================================================================

  test "len of list" do
    assert Builtins.builtin_len([[1, 2, 3]], nil) == 3
  end

  test "len of string" do
    assert Builtins.builtin_len(["hello"], nil) == 5
  end

  test "len of dict" do
    assert Builtins.builtin_len([%{"a" => 1}], nil) == 1
  end

  test "len of empty list" do
    assert Builtins.builtin_len([[]], nil) == 0
  end

  test "len of tuple" do
    assert Builtins.builtin_len([{1, 2, 3}], nil) == 3
  end

  # ===========================================================================
  # List Function
  # ===========================================================================

  test "list from empty" do
    assert Builtins.builtin_list([], nil) == []
  end

  test "list from list" do
    assert Builtins.builtin_list([[1, 2]], nil) == [1, 2]
  end

  test "list from tuple" do
    assert Builtins.builtin_list([{1, 2}], nil) == [1, 2]
  end

  test "list from string" do
    assert Builtins.builtin_list(["abc"], nil) == ["a", "b", "c"]
  end

  # ===========================================================================
  # Dict Function
  # ===========================================================================

  test "dict from empty" do
    assert Builtins.builtin_dict([], nil) == %{}
  end

  # ===========================================================================
  # Tuple Function
  # ===========================================================================

  test "tuple from empty" do
    assert Builtins.builtin_tuple([], nil) == {}
  end

  test "tuple from list" do
    assert Builtins.builtin_tuple([[1, 2]], nil) == {1, 2}
  end

  # ===========================================================================
  # Range Function
  # ===========================================================================

  test "range with one arg" do
    assert Builtins.builtin_range([5], nil) == [0, 1, 2, 3, 4]
  end

  test "range with two args" do
    assert Builtins.builtin_range([2, 5], nil) == [2, 3, 4]
  end

  test "range with step" do
    assert Builtins.builtin_range([0, 10, 3], nil) == [0, 3, 6, 9]
  end

  test "range with negative step" do
    assert Builtins.builtin_range([5, 0, -1], nil) == [5, 4, 3, 2, 1]
  end

  test "range with empty result" do
    assert Builtins.builtin_range([5, 2], nil) == []
  end

  # ===========================================================================
  # Sorted and Reversed
  # ===========================================================================

  test "sorted list" do
    assert Builtins.builtin_sorted([[3, 1, 2]], nil) == [1, 2, 3]
  end

  test "reversed list" do
    assert Builtins.builtin_reversed([[1, 2, 3]], nil) == [3, 2, 1]
  end

  # ===========================================================================
  # Enumerate and Zip
  # ===========================================================================

  test "enumerate list" do
    result = Builtins.builtin_enumerate([["a", "b", "c"]], nil)
    assert result == [{0, "a"}, {1, "b"}, {2, "c"}]
  end

  test "enumerate with start" do
    result = Builtins.builtin_enumerate([["a", "b"], 1], nil)
    assert result == [{1, "a"}, {2, "b"}]
  end

  test "zip two lists" do
    result = Builtins.builtin_zip([[1, 2], ["a", "b"]], nil)
    assert result == [{1, "a"}, {2, "b"}]
  end

  # ===========================================================================
  # Min and Max
  # ===========================================================================

  test "min of list" do
    assert Builtins.builtin_min([[3, 1, 2]], nil) == 1
  end

  test "min of args" do
    assert Builtins.builtin_min([3, 1, 2], nil) == 1
  end

  test "max of list" do
    assert Builtins.builtin_max([[3, 1, 2]], nil) == 3
  end

  test "max of args" do
    assert Builtins.builtin_max([3, 1, 2], nil) == 3
  end

  # ===========================================================================
  # Abs, All, Any
  # ===========================================================================

  test "abs positive" do
    assert Builtins.builtin_abs([5], nil) == 5
  end

  test "abs negative" do
    assert Builtins.builtin_abs([-5], nil) == 5
  end

  test "all truthy" do
    assert Builtins.builtin_all([[1, 2, 3]], nil) == true
  end

  test "all with falsy" do
    assert Builtins.builtin_all([[1, 0, 3]], nil) == false
  end

  test "any truthy" do
    assert Builtins.builtin_any([[0, 1, 0]], nil) == true
  end

  test "any all falsy" do
    assert Builtins.builtin_any([[0, 0, 0]], nil) == false
  end

  # ===========================================================================
  # Repr and Hasattr
  # ===========================================================================

  test "repr of integer" do
    assert is_binary(Builtins.builtin_repr([42], nil))
  end

  test "hasattr true" do
    assert Builtins.builtin_hasattr([%{"x" => 1}, "x"], nil) == true
  end

  test "hasattr false" do
    assert Builtins.builtin_hasattr([%{"x" => 1}, "y"], nil) == false
  end

  test "getattr existing" do
    assert Builtins.builtin_getattr([%{"x" => 42}, "x"], nil) == 42
  end

  test "getattr with default" do
    assert Builtins.builtin_getattr([%{"x" => 42}, "y", 0], nil) == 0
  end

  # ===========================================================================
  # Get All Builtins
  # ===========================================================================

  test "get_all_builtins returns 23 builtins" do
    builtins = Builtins.get_all_builtins()
    assert map_size(builtins) == 23
  end

  test "all builtin values are functions" do
    for {_name, impl} <- Builtins.get_all_builtins() do
      assert is_function(impl, 2)
    end
  end
end
