defmodule CodingAdventures.LatticeAstToCss.LatticeV2Test do
  @moduledoc """
  Tests for Lattice v2 features ported from the Python reference implementation.

  Covers:
  - New error types (MaxIterationError, ExtendTargetNotFoundError, RangeError, ZeroDivisionInExpressionError)
  - Scope.set_global/3 for !global flag
  - Values: map type, color operations, division, type introspection
  - Built-in functions (map, color, list, type, math)
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.LatticeAstToCss.{Scope, Values, Builtins}
  alias CodingAdventures.LatticeAstToCss.Errors.{
    MaxIterationError,
    ExtendTargetNotFoundError,
    RangeError,
    ZeroDivisionInExpressionError
  }

  # ==========================================================================
  # Error Types
  # ==========================================================================

  describe "MaxIterationError" do
    test "creates error with default max iterations" do
      err = MaxIterationError.new()
      assert err.message =~ "1000"
      assert err.max_iterations == 1000
    end

    test "creates error with custom max iterations" do
      err = MaxIterationError.new(500)
      assert err.message =~ "500"
      assert err.max_iterations == 500
    end

    test "creates error with max_iterations and line (2-arg arity)" do
      err = MaxIterationError.new(200, 10)
      assert err.message =~ "200"
      assert err.max_iterations == 200
      assert err.line == 10
      assert err.column == 0
    end
  end

  describe "ExtendTargetNotFoundError" do
    test "creates error with target" do
      err = ExtendTargetNotFoundError.new("%message-shared")
      assert err.message =~ "%message-shared"
      assert err.target == "%message-shared"
    end

    test "creates error with target and line (2-arg arity)" do
      err = ExtendTargetNotFoundError.new(".btn", 42)
      assert err.message =~ ".btn"
      assert err.target == ".btn"
      assert err.line == 42
      assert err.column == 0
    end
  end

  describe "RangeError" do
    test "creates error with message" do
      err = RangeError.new("Index 5 out of bounds")
      assert err.message == "Index 5 out of bounds"
    end

    test "creates error with message and line (2-arg arity)" do
      err = RangeError.new("Index out of range", 15)
      assert err.message == "Index out of range"
      assert err.line == 15
      assert err.column == 0
    end
  end

  describe "ZeroDivisionInExpressionError" do
    test "creates error" do
      err = ZeroDivisionInExpressionError.new()
      assert err.message == "Division by zero"
    end

    test "creates error with line (1-arg arity)" do
      err = ZeroDivisionInExpressionError.new(7)
      assert err.message == "Division by zero"
      assert err.line == 7
      assert err.column == 0
    end
  end

  # ==========================================================================
  # Scope: set_global
  # ==========================================================================

  describe "Scope.set_global/3" do
    test "sets in root when called on root scope" do
      scope = Scope.new() |> Scope.set("$x", 1)
      scope = Scope.set_global(scope, "$y", 2)
      assert {:ok, 2} = Scope.get(scope, "$y")
    end

    test "sets in root from a child scope" do
      global = Scope.new() |> Scope.set("$theme", "light")
      child = Scope.child(global)
      child = Scope.set_global(child, "$theme", "dark")

      # The child's view should now see "dark" (since root was updated)
      assert {:ok, "dark"} = Scope.get(child, "$theme")
    end

    test "sets in root from a deeply nested scope" do
      global = Scope.new()
      child1 = Scope.child(global)
      child2 = Scope.child(child1)
      child3 = Scope.child(child2)

      child3 = Scope.set_global(child3, "$deep", "value")
      # Walk up to root
      root = child3.parent.parent.parent
      assert {:ok, "value"} = Scope.get(root, "$deep")
    end
  end

  describe "Scope.has_local?/2" do
    test "returns true for locally set name" do
      scope = Scope.new() |> Scope.set("$x", 1)
      assert Scope.has_local?(scope, "$x")
    end

    test "returns false for parent-only name" do
      parent = Scope.new() |> Scope.set("$x", 1)
      child = Scope.child(parent)
      refute Scope.has_local?(child, "$x")
    end
  end

  # ==========================================================================
  # Values: Map type
  # ==========================================================================

  describe "map values" do
    test "to_css formats a map" do
      m = {:map, [{"primary", {:color, "#4a90d9"}}, {"secondary", {:color, "#7b68ee"}}]}
      css = Values.to_css(m)
      assert css =~ "primary:"
      assert css =~ "#4a90d9"
    end

    test "map_get finds a key" do
      m = {:map, [{"primary", {:color, "#4a90d9"}}, {"secondary", {:color, "#7b68ee"}}]}
      assert {:ok, {:color, "#4a90d9"}} = Values.map_get(m, "primary")
    end

    test "map_get returns :error for missing key" do
      m = {:map, [{"a", {:number, 1.0}}]}
      assert :error = Values.map_get(m, "b")
    end

    test "map_keys returns all keys as idents" do
      m = {:map, [{"x", {:number, 1.0}}, {"y", {:number, 2.0}}]}
      assert {:list, [{:ident, "x"}, {:ident, "y"}]} = Values.map_keys(m)
    end

    test "map_values returns all values" do
      m = {:map, [{"x", {:number, 1.0}}, {"y", {:number, 2.0}}]}
      assert {:list, [{:number, 1.0}, {:number, 2.0}]} = Values.map_values(m)
    end

    test "map_has_key? works" do
      m = {:map, [{"x", {:number, 1.0}}]}
      assert Values.map_has_key?(m, "x")
      refute Values.map_has_key?(m, "y")
    end

    test "map_merge combines maps" do
      m1 = {:map, [{"a", {:number, 1.0}}, {"b", {:number, 2.0}}]}
      m2 = {:map, [{"b", {:number, 3.0}}, {"c", {:number, 4.0}}]}
      {:map, items} = Values.map_merge(m1, m2)
      # b should be overwritten by m2
      assert Enum.find(items, fn {k, _} -> k == "b" end) == {"b", {:number, 3.0}}
      assert length(items) == 3
    end

    test "map_remove removes keys" do
      m = {:map, [{"a", {:number, 1.0}}, {"b", {:number, 2.0}}, {"c", {:number, 3.0}}]}
      {:map, items} = Values.map_remove(m, ["b"])
      assert length(items) == 2
      refute Enum.any?(items, fn {k, _} -> k == "b" end)
    end
  end

  # ==========================================================================
  # Values: Color operations
  # ==========================================================================

  describe "color operations" do
    test "color_to_rgb parses #RGB shorthand" do
      {r, g, b, a} = Values.color_to_rgb("#f00")
      assert r == 255
      assert g == 0
      assert b == 0
      assert a == 1.0
    end

    test "color_to_rgb parses #RRGGBB" do
      {r, g, b, a} = Values.color_to_rgb("#4a90d9")
      assert r == 74
      assert g == 144
      assert b == 217
      assert a == 1.0
    end

    test "color_from_rgb creates hex color" do
      {:color, hex} = Values.color_from_rgb(255, 0, 0)
      assert hex == "#ff0000"
    end

    test "color_from_rgb with alpha creates rgba" do
      {:color, val} = Values.color_from_rgb(255, 0, 0, 0.5)
      assert val =~ "rgba"
    end

    test "color_to_hsl and back round-trips" do
      {h, s, l, _a} = Values.color_to_hsl("#ff0000")
      {:color, hex} = Values.color_from_hsl(h, s, l)
      assert hex == "#ff0000"
    end

    test "color_to_hsl for pure green" do
      {h, _, _, _} = Values.color_to_hsl("#00ff00")
      assert_in_delta h, 120.0, 0.1
    end
  end

  # ==========================================================================
  # Values: Division
  # ==========================================================================

  describe "Values.divide/2" do
    test "number / number" do
      assert {:ok, {:number, 5.0}} = Values.divide({:number, 10.0}, {:number, 2.0})
    end

    test "dimension / number" do
      assert {:ok, {:dimension, 5.0, "px"}} = Values.divide({:dimension, 10.0, "px"}, {:number, 2.0})
    end

    test "dimension / dimension (same unit) yields number" do
      assert {:ok, {:number, 2.0}} = Values.divide({:dimension, 10.0, "px"}, {:dimension, 5.0, "px"})
    end

    test "division by zero returns error" do
      assert {:error, "Division by zero"} = Values.divide({:number, 10.0}, {:number, 0.0})
    end
  end

  # ==========================================================================
  # Values: type_name_of
  # ==========================================================================

  describe "Values.type_name_of/1" do
    test "returns correct type names" do
      assert "number" = Values.type_name_of({:number, 42.0})
      assert "number" = Values.type_name_of({:dimension, 16.0, "px"})
      assert "string" = Values.type_name_of({:string, "hello"})
      assert "color" = Values.type_name_of({:color, "#fff"})
      assert "bool" = Values.type_name_of({:bool, true})
      assert "null" = Values.type_name_of(:null)
      assert "list" = Values.type_name_of({:list, []})
      assert "map" = Values.type_name_of({:map, []})
    end
  end

  # ==========================================================================
  # Built-in Functions
  # ==========================================================================

  describe "Builtins: map functions" do
    test "map-get retrieves value" do
      m = {:map, [{"primary", {:color, "#ff0000"}}]}
      assert {:ok, {:color, "#ff0000"}} = Builtins.call("map-get", [m, {:ident, "primary"}])
    end

    test "map-get returns null for missing key" do
      m = {:map, [{"a", {:number, 1.0}}]}
      assert {:ok, :null} = Builtins.call("map-get", [m, {:ident, "b"}])
    end

    test "map-keys returns key list" do
      m = {:map, [{"x", {:number, 1.0}}, {"y", {:number, 2.0}}]}
      assert {:ok, {:list, [{:ident, "x"}, {:ident, "y"}]}} = Builtins.call("map-keys", [m])
    end

    test "map-has-key returns bool" do
      m = {:map, [{"x", {:number, 1.0}}]}
      assert {:ok, {:bool, true}} = Builtins.call("map-has-key", [m, {:ident, "x"}])
      assert {:ok, {:bool, false}} = Builtins.call("map-has-key", [m, {:ident, "y"}])
    end

    test "map-merge combines maps" do
      m1 = {:map, [{"a", {:number, 1.0}}]}
      m2 = {:map, [{"b", {:number, 2.0}}]}
      {:ok, {:map, items}} = Builtins.call("map-merge", [m1, m2])
      assert length(items) == 2
    end
  end

  describe "Builtins: color functions" do
    test "lighten increases lightness" do
      {:ok, {:color, result}} = Builtins.call("lighten", [{:color, "#4a90d9"}, {:number, 10.0}])
      assert is_binary(result)
      assert String.starts_with?(result, "#")
    end

    test "darken decreases lightness" do
      {:ok, {:color, result}} = Builtins.call("darken", [{:color, "#4a90d9"}, {:number, 10.0}])
      assert is_binary(result)
    end

    test "complement rotates hue by 180" do
      {:ok, {:color, _}} = Builtins.call("complement", [{:color, "#ff0000"}])
    end

    test "mix blends two colors" do
      {:ok, {:color, _}} = Builtins.call("mix", [{:color, "#ff0000"}, {:color, "#0000ff"}])
    end

    test "red extracts red channel" do
      {:ok, {:number, r}} = Builtins.call("red", [{:color, "#ff0000"}])
      assert r == 255.0
    end

    test "green extracts green channel" do
      {:ok, {:number, g}} = Builtins.call("green", [{:color, "#00ff00"}])
      assert g == 255.0
    end

    test "blue extracts blue channel" do
      {:ok, {:number, b}} = Builtins.call("blue", [{:color, "#0000ff"}])
      assert b == 255.0
    end

    test "hue extracts hue in degrees" do
      {:ok, {:dimension, _, "deg"}} = Builtins.call("hue", [{:color, "#ff0000"}])
    end

    test "saturation extracts saturation as percentage" do
      {:ok, {:percentage, _}} = Builtins.call("saturation", [{:color, "#ff0000"}])
    end

    test "lightness extracts lightness as percentage" do
      {:ok, {:percentage, _}} = Builtins.call("lightness", [{:color, "#ff0000"}])
    end
  end

  describe "Builtins: list functions" do
    test "nth gets item at 1-based index" do
      list = {:list, [{:ident, "a"}, {:ident, "b"}, {:ident, "c"}]}
      assert {:ok, {:ident, "b"}} = Builtins.call("nth", [list, {:number, 2.0}])
    end

    test "nth out of bounds returns error" do
      list = {:list, [{:ident, "a"}]}
      assert {:error, _} = Builtins.call("nth", [list, {:number, 5.0}])
    end

    test "length returns count" do
      list = {:list, [{:ident, "a"}, {:ident, "b"}]}
      assert {:ok, {:number, 2.0}} = Builtins.call("length", [list])
    end

    test "length of single value is 1" do
      assert {:ok, {:number, 1.0}} = Builtins.call("length", [{:number, 42.0}])
    end

    test "join concatenates lists" do
      l1 = {:list, [{:ident, "a"}]}
      l2 = {:list, [{:ident, "b"}]}
      {:ok, {:list, items}} = Builtins.call("join", [l1, l2])
      assert length(items) == 2
    end

    test "append adds to end" do
      l = {:list, [{:ident, "a"}]}
      {:ok, {:list, items}} = Builtins.call("append", [l, {:ident, "b"}])
      assert length(items) == 2
    end

    test "index finds item position" do
      list = {:list, [{:ident, "a"}, {:ident, "b"}, {:ident, "c"}]}
      assert {:ok, {:number, 2.0}} = Builtins.call("index", [list, {:ident, "b"}])
    end

    test "index returns null for missing item" do
      list = {:list, [{:ident, "a"}]}
      assert {:ok, :null} = Builtins.call("index", [list, {:ident, "z"}])
    end
  end

  describe "Builtins: type functions" do
    test "type-of returns type name" do
      assert {:ok, {:string, "number"}} = Builtins.call("type-of", [{:number, 42.0}])
      assert {:ok, {:string, "string"}} = Builtins.call("type-of", [{:string, "hello"}])
      assert {:ok, {:string, "color"}} = Builtins.call("type-of", [{:color, "#fff"}])
      assert {:ok, {:string, "bool"}} = Builtins.call("type-of", [{:bool, true}])
      assert {:ok, {:string, "null"}} = Builtins.call("type-of", [:null])
      assert {:ok, {:string, "list"}} = Builtins.call("type-of", [{:list, []}])
      assert {:ok, {:string, "map"}} = Builtins.call("type-of", [{:map, []}])
    end

    test "unit returns unit string" do
      assert {:ok, {:string, "px"}} = Builtins.call("unit", [{:dimension, 16.0, "px"}])
      assert {:ok, {:string, "%"}} = Builtins.call("unit", [{:percentage, 50.0}])
      assert {:ok, {:string, ""}} = Builtins.call("unit", [{:number, 42.0}])
    end

    test "unitless returns boolean" do
      assert {:ok, {:bool, true}} = Builtins.call("unitless", [{:number, 42.0}])
      assert {:ok, {:bool, false}} = Builtins.call("unitless", [{:dimension, 16.0, "px"}])
    end

    test "comparable checks compatibility" do
      assert {:ok, {:bool, true}} = Builtins.call("comparable", [{:dimension, 1.0, "px"}, {:dimension, 2.0, "px"}])
      assert {:ok, {:bool, false}} = Builtins.call("comparable", [{:dimension, 1.0, "px"}, {:dimension, 2.0, "em"}])
    end
  end

  describe "Builtins: math functions" do
    test "math.div divides numbers" do
      assert {:ok, {:number, 5.0}} = Builtins.call("math.div", [{:number, 10.0}, {:number, 2.0}])
    end

    test "math.div divides dimension by number" do
      assert {:ok, {:dimension, 5.0, "px"}} = Builtins.call("math.div", [{:dimension, 10.0, "px"}, {:number, 2.0}])
    end

    test "math.div by zero returns error" do
      assert {:error, "Division by zero"} = Builtins.call("math.div", [{:number, 10.0}, {:number, 0.0}])
    end

    test "math.floor rounds down" do
      assert {:ok, {:number, 3.0}} = Builtins.call("math.floor", [{:number, 3.7}])
    end

    test "math.ceil rounds up" do
      assert {:ok, {:number, 4.0}} = Builtins.call("math.ceil", [{:number, 3.2}])
    end

    test "math.round rounds to nearest" do
      assert {:ok, {:number, 4.0}} = Builtins.call("math.round", [{:number, 3.5}])
    end

    test "math.abs returns absolute value" do
      assert {:ok, {:number, 5.0}} = Builtins.call("math.abs", [{:number, -5.0}])
    end

    test "math.min returns minimum" do
      assert {:ok, {:number, 1.0}} = Builtins.call("math.min", [{:number, 3.0}, {:number, 1.0}, {:number, 5.0}])
    end

    test "math.max returns maximum" do
      assert {:ok, {:number, 5.0}} = Builtins.call("math.max", [{:number, 3.0}, {:number, 1.0}, {:number, 5.0}])
    end

    test "math.floor preserves unit" do
      assert {:ok, {:dimension, 3.0, "px"}} = Builtins.call("math.floor", [{:dimension, 3.7, "px"}])
    end
  end

  describe "Builtins.builtin?/1" do
    test "returns true for known built-ins" do
      assert Builtins.builtin?("map-get")
      assert Builtins.builtin?("lighten")
      assert Builtins.builtin?("math.div")
      assert Builtins.builtin?("type-of")
      assert Builtins.builtin?("nth")
    end

    test "returns false for unknown names" do
      refute Builtins.builtin?("unknown-fn")
      refute Builtins.builtin?("calc")
    end
  end

  describe "Builtins: not_found for unknown functions" do
    test "returns :not_found for unknown function" do
      assert :not_found = Builtins.call("nonexistent", [])
    end
  end

  # ==========================================================================
  # Values: truthiness of map
  # ==========================================================================

  describe "truthiness of map" do
    test "map is truthy (even when empty)" do
      assert Values.truthy?({:map, []})
      assert Values.truthy?({:map, [{"a", {:number, 1.0}}]})
    end
  end

  # ==========================================================================
  # Values: get_numeric_value
  # ==========================================================================

  describe "Values.get_numeric_value/1" do
    test "extracts value from number types" do
      assert {:ok, 42.0} = Values.get_numeric_value({:number, 42.0})
      assert {:ok, 16.0} = Values.get_numeric_value({:dimension, 16.0, "px"})
      assert {:ok, 50.0} = Values.get_numeric_value({:percentage, 50.0})
    end

    test "returns :error for non-numeric types" do
      assert :error = Values.get_numeric_value({:string, "hello"})
      assert :error = Values.get_numeric_value({:ident, "red"})
    end
  end
end
