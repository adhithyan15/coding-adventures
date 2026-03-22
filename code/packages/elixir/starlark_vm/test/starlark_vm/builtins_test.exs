defmodule CodingAdventures.StarlarkVm.BuiltinsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkVm.Builtins
  alias CodingAdventures.StarlarkVm.Handlers
  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.VirtualMachine.Errors

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

  test "type of unknown value" do
    # Atoms are not a Starlark type
    assert Builtins.builtin_type([:some_atom], nil) == "unknown"
  end

  test "type with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_type([1, 2], nil)
    end
  end

  test "type with zero args raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_type([], nil)
    end
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

  test "bool with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_bool([1, 2], nil)
    end
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

  test "int from boolean true" do
    assert Builtins.builtin_int([true], nil) == 1
  end

  test "int from boolean false" do
    assert Builtins.builtin_int([false], nil) == 0
  end

  test "int from invalid type raises" do
    assert_raise Errors.VMTypeError, ~r/must be a string or number/, fn ->
      Builtins.builtin_int([[1, 2]], nil)
    end
  end

  test "int with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes 1 or 2 arguments/, fn ->
      Builtins.builtin_int([1, 2, 3], nil)
    end
  end

  test "int with zero args raises" do
    assert_raise Errors.VMTypeError, ~r/takes 1 or 2 arguments/, fn ->
      Builtins.builtin_int([], nil)
    end
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

  test "float from string" do
    assert Builtins.builtin_float(["3.14"], nil) == 3.14
  end

  test "float from invalid type raises" do
    assert_raise Errors.VMTypeError, ~r/must be a string or number/, fn ->
      Builtins.builtin_float([[1]], nil)
    end
  end

  test "float with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_float([1, 2], nil)
    end
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

  test "str of float" do
    result = Builtins.builtin_str([3.14], nil)
    assert is_binary(result)
  end

  test "str of list" do
    result = Builtins.builtin_str([[1, 2]], nil)
    assert result == "[1, 2]"
  end

  test "str with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_str([1, 2], nil)
    end
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

  test "len of non-collection raises" do
    assert_raise Errors.VMTypeError, ~r/has no len/, fn ->
      Builtins.builtin_len([42], nil)
    end
  end

  test "len with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_len([[], []], nil)
    end
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

  test "list from map returns keys" do
    result = Builtins.builtin_list([%{"a" => 1, "b" => 2}], nil)
    assert Enum.sort(result) == ["a", "b"]
  end

  test "list from invalid type raises" do
    assert_raise Errors.VMTypeError, ~r/Cannot convert to list/, fn ->
      Builtins.builtin_list([42], nil)
    end
  end

  test "list with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes at most 1 argument/, fn ->
      Builtins.builtin_list([[1], [2]], nil)
    end
  end

  # ===========================================================================
  # Dict Function
  # ===========================================================================

  test "dict from empty" do
    assert Builtins.builtin_dict([], nil) == %{}
  end

  test "dict from key-value pairs" do
    pairs = [{"a", 1}, {"b", 2}]
    assert Builtins.builtin_dict([pairs], nil) == %{"a" => 1, "b" => 2}
  end

  test "dict with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes at most 1 argument/, fn ->
      Builtins.builtin_dict([[{"a", 1}], [{"b", 2}]], nil)
    end
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

  test "tuple from tuple" do
    assert Builtins.builtin_tuple([{1, 2}], nil) == {1, 2}
  end

  test "tuple from string" do
    assert Builtins.builtin_tuple(["abc"], nil) == {"a", "b", "c"}
  end

  test "tuple from invalid type raises" do
    assert_raise Errors.VMTypeError, ~r/Cannot convert to tuple/, fn ->
      Builtins.builtin_tuple([42], nil)
    end
  end

  test "tuple with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes at most 1 argument/, fn ->
      Builtins.builtin_tuple([[1], [2]], nil)
    end
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

  test "range with step zero raises" do
    assert_raise Errors.VMTypeError, ~r/step argument must not be zero/, fn ->
      Builtins.builtin_range([0, 10, 0], nil)
    end
  end

  test "range with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes 1 to 3 arguments/, fn ->
      Builtins.builtin_range([1, 2, 3, 4], nil)
    end
  end

  test "range with zero args raises" do
    assert_raise Errors.VMTypeError, ~r/takes 1 to 3 arguments/, fn ->
      Builtins.builtin_range([], nil)
    end
  end

  # ===========================================================================
  # Sorted and Reversed
  # ===========================================================================

  test "sorted list" do
    assert Builtins.builtin_sorted([[3, 1, 2]], nil) == [1, 2, 3]
  end

  test "sorted with reverse flag true" do
    assert Builtins.builtin_sorted([[3, 1, 2], true], nil) == [3, 2, 1]
  end

  test "sorted with reverse flag false" do
    assert Builtins.builtin_sorted([[3, 1, 2], false], nil) == [1, 2, 3]
  end

  test "sorted with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes 1 or 2 arguments/, fn ->
      Builtins.builtin_sorted([[1], true, true], nil)
    end
  end

  test "reversed list" do
    assert Builtins.builtin_reversed([[1, 2, 3]], nil) == [3, 2, 1]
  end

  test "reversed with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_reversed([[1], [2]], nil)
    end
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

  test "enumerate with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes 1 or 2 arguments/, fn ->
      Builtins.builtin_enumerate([[], 0, 1], nil)
    end
  end

  test "zip two lists" do
    result = Builtins.builtin_zip([[1, 2], ["a", "b"]], nil)
    assert result == [{1, "a"}, {2, "b"}]
  end

  test "zip with empty args" do
    result = Builtins.builtin_zip([], nil)
    assert result == []
  end

  test "zip three lists" do
    result = Builtins.builtin_zip([[1, 2], ["a", "b"], [:x, :y]], nil)
    assert result == [{1, "a", :x}, {2, "b", :y}]
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

  test "min with no args raises" do
    assert_raise Errors.VMTypeError, ~r/requires at least 1 argument/, fn ->
      Builtins.builtin_min([], nil)
    end
  end

  test "max of list" do
    assert Builtins.builtin_max([[3, 1, 2]], nil) == 3
  end

  test "max of args" do
    assert Builtins.builtin_max([3, 1, 2], nil) == 3
  end

  test "max with no args raises" do
    assert_raise Errors.VMTypeError, ~r/requires at least 1 argument/, fn ->
      Builtins.builtin_max([], nil)
    end
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

  test "abs of float" do
    assert Builtins.builtin_abs([-3.14], nil) == 3.14
  end

  test "abs with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_abs([1, 2], nil)
    end
  end

  test "all truthy" do
    assert Builtins.builtin_all([[1, 2, 3]], nil) == true
  end

  test "all with falsy" do
    assert Builtins.builtin_all([[1, 0, 3]], nil) == false
  end

  test "all with empty list" do
    assert Builtins.builtin_all([[]], nil) == true
  end

  test "all with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_all([[1], [2]], nil)
    end
  end

  test "any truthy" do
    assert Builtins.builtin_any([[0, 1, 0]], nil) == true
  end

  test "any all falsy" do
    assert Builtins.builtin_any([[0, 0, 0]], nil) == false
  end

  test "any with empty list" do
    assert Builtins.builtin_any([[]], nil) == false
  end

  test "any with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_any([[1], [2]], nil)
    end
  end

  # ===========================================================================
  # Repr and Hasattr
  # ===========================================================================

  test "repr of integer" do
    assert is_binary(Builtins.builtin_repr([42], nil))
  end

  test "repr of string" do
    assert is_binary(Builtins.builtin_repr(["hello"], nil))
  end

  test "repr with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes exactly 1 argument/, fn ->
      Builtins.builtin_repr([1, 2], nil)
    end
  end

  test "hasattr true" do
    assert Builtins.builtin_hasattr([%{"x" => 1}, "x"], nil) == true
  end

  test "hasattr false" do
    assert Builtins.builtin_hasattr([%{"x" => 1}, "y"], nil) == false
  end

  test "hasattr on non-map returns false" do
    assert Builtins.builtin_hasattr([42, "x"], nil) == false
  end

  test "hasattr with wrong args returns false" do
    assert Builtins.builtin_hasattr([], nil) == false
  end

  test "getattr existing" do
    assert Builtins.builtin_getattr([%{"x" => 42}, "x"], nil) == 42
  end

  test "getattr missing raises" do
    assert_raise Errors.VMTypeError, ~r/has no attribute/, fn ->
      Builtins.builtin_getattr([%{"x" => 42}, "y"], nil)
    end
  end

  test "getattr with default" do
    assert Builtins.builtin_getattr([%{"x" => 42}, "y", 0], nil) == 0
  end

  test "getattr with wrong arg count raises" do
    assert_raise Errors.VMTypeError, ~r/takes 2 or 3 arguments/, fn ->
      Builtins.builtin_getattr([1], nil)
    end
  end

  # ===========================================================================
  # Print
  # ===========================================================================

  test "print captures output to vm" do
    vm = GenericVM.new()
    {output, updated_vm} = Builtins.builtin_print([42], vm)
    assert output == nil
    assert updated_vm.output == ["42"]
  end

  test "print with multiple args joins with space" do
    vm = GenericVM.new()
    {_output, updated_vm} = Builtins.builtin_print([1, "hello", true], vm)
    assert updated_vm.output == ["1 hello True"]
  end

  test "print with empty args" do
    vm = GenericVM.new()
    {_output, updated_vm} = Builtins.builtin_print([], vm)
    assert updated_vm.output == [""]
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

  test "all expected builtin names are present" do
    builtins = Builtins.get_all_builtins()
    expected = ~w(type bool int float str len list dict tuple range sorted reversed enumerate zip min max abs all any repr hasattr getattr print)
    for name <- expected do
      assert Map.has_key?(builtins, name), "Missing builtin: #{name}"
    end
  end
end
