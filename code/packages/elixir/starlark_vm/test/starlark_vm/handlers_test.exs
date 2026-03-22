defmodule CodingAdventures.StarlarkVm.HandlersTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StarlarkVm.Handlers
  alias CodingAdventures.StarlarkVm.Handlers.{StarlarkFunction, StarlarkIterator, StarlarkResult}

  # ===========================================================================
  # Struct Tests
  # ===========================================================================

  test "StarlarkFunction has correct default fields" do
    func = %StarlarkFunction{}
    assert func.name == "<lambda>"
    assert func.code == nil
    assert func.params == []
    assert func.defaults == []
    assert func.closure_cells == []
  end

  test "StarlarkIterator has correct default fields" do
    iter = %StarlarkIterator{}
    assert iter.items == []
    assert iter.index == 0
  end

  test "StarlarkResult has correct default fields" do
    result = %StarlarkResult{}
    assert result.variables == %{}
    assert result.output == []
    assert result.traces == []
  end

  # ===========================================================================
  # Truthiness
  # ===========================================================================

  test "nil is falsy" do
    assert Handlers.truthy?(nil) == false
  end

  test "false is falsy" do
    assert Handlers.truthy?(false) == false
  end

  test "0 is falsy" do
    assert Handlers.truthy?(0) == false
  end

  test "0.0 is falsy" do
    assert Handlers.truthy?(0.0) == false
  end

  test "empty string is falsy" do
    assert Handlers.truthy?("") == false
  end

  test "empty list is falsy" do
    assert Handlers.truthy?([]) == false
  end

  test "empty map is falsy" do
    assert Handlers.truthy?(%{}) == false
  end

  test "empty tuple is falsy" do
    assert Handlers.truthy?({}) == false
  end

  test "1 is truthy" do
    assert Handlers.truthy?(1) == true
  end

  test "non-empty string is truthy" do
    assert Handlers.truthy?("hello") == true
  end

  test "non-empty list is truthy" do
    assert Handlers.truthy?([1]) == true
  end

  test "non-empty map is truthy" do
    assert Handlers.truthy?(%{a: 1}) == true
  end

  test "true is truthy" do
    assert Handlers.truthy?(true) == true
  end

  # ===========================================================================
  # starlark_repr
  # ===========================================================================

  test "repr of nil" do
    assert Handlers.starlark_repr(nil) == "None"
  end

  test "repr of true" do
    assert Handlers.starlark_repr(true) == "True"
  end

  test "repr of false" do
    assert Handlers.starlark_repr(false) == "False"
  end

  test "repr of integer" do
    assert Handlers.starlark_repr(42) == "42"
  end

  test "repr of string" do
    assert Handlers.starlark_repr("hello") == "hello"
  end

  test "repr of list" do
    result = Handlers.starlark_repr([1, 2, 3])
    assert result == "[1, 2, 3]"
  end

  test "repr of empty list" do
    assert Handlers.starlark_repr([]) == "[]"
  end
end
