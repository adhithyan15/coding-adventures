defmodule CodingAdventures.LatticeAstToCss.LatticeV2CoverageTest do
  @moduledoc """
  Additional coverage tests for Lattice v2 features — targeting untested
  error paths and edge cases in Builtins and Values modules.
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.LatticeAstToCss.{Values, Builtins}

  # ==========================================================================
  # Builtins: error paths for map functions
  # ==========================================================================

  describe "Builtins map error paths" do
    test "map-get with non-map first arg" do
      assert {:error, _} = Builtins.call("map-get", [{:number, 1.0}, {:ident, "key"}])
    end

    test "map-get with too few args" do
      assert {:error, _} = Builtins.call("map-get", [{:map, []}])
    end

    test "map-keys with non-map arg" do
      assert {:error, _} = Builtins.call("map-keys", [{:number, 1.0}])
    end

    test "map-keys with no args" do
      assert {:error, _} = Builtins.call("map-keys", [])
    end

    test "map-values with no args" do
      assert {:error, _} = Builtins.call("map-values", [])
    end

    test "map-has-key with too few args" do
      assert {:error, _} = Builtins.call("map-has-key", [{:map, []}])
    end

    test "map-merge with non-map args" do
      assert {:error, _} = Builtins.call("map-merge", [{:number, 1.0}, {:map, []}])
    end

    test "map-merge with no args" do
      assert {:error, _} = Builtins.call("map-merge", [])
    end

    test "map-remove with no args" do
      assert {:error, _} = Builtins.call("map-remove", [])
    end

    test "map-values with non-map" do
      assert {:error, _} = Builtins.call("map-values", [{:number, 1.0}])
    end

    test "map-has-key with non-map" do
      assert {:error, _} = Builtins.call("map-has-key", [{:number, 1.0}, {:ident, "key"}])
    end
  end

  # ==========================================================================
  # Builtins: color function error paths
  # ==========================================================================

  describe "Builtins color error paths" do
    test "lighten with non-color" do
      assert {:error, _} = Builtins.call("lighten", [{:number, 1.0}, {:number, 10.0}])
    end

    test "lighten with out-of-range amount" do
      assert {:error, _} = Builtins.call("lighten", [{:color, "#fff"}, {:number, 150.0}])
    end

    test "lighten with too few args" do
      assert {:error, _} = Builtins.call("lighten", [{:color, "#fff"}])
    end

    test "darken with too few args" do
      assert {:error, _} = Builtins.call("darken", [{:color, "#000"}])
    end

    test "darken with non-color" do
      assert {:error, _} = Builtins.call("darken", [{:ident, "red"}, {:number, 10.0}])
    end

    test "saturate with too few args" do
      assert {:error, _} = Builtins.call("saturate", [{:color, "#fff"}])
    end

    test "desaturate with too few args" do
      assert {:error, _} = Builtins.call("desaturate", [{:color, "#fff"}])
    end

    test "desaturate with non-color" do
      assert {:error, _} = Builtins.call("desaturate", [{:number, 1.0}, {:number, 10.0}])
    end

    test "adjust-hue with too few args" do
      assert {:error, _} = Builtins.call("adjust-hue", [{:color, "#fff"}])
    end

    test "adjust-hue with non-numeric degrees" do
      assert {:error, _} = Builtins.call("adjust-hue", [{:color, "#fff"}, {:ident, "abc"}])
    end

    test "complement with no args" do
      assert {:error, _} = Builtins.call("complement", [])
    end

    test "complement with non-color" do
      assert {:error, _} = Builtins.call("complement", [{:number, 42.0}])
    end

    test "mix with too few args" do
      assert {:error, _} = Builtins.call("mix", [{:color, "#fff"}])
    end

    test "mix with non-color second arg" do
      assert {:error, _} = Builtins.call("mix", [{:color, "#fff"}, {:number, 1.0}])
    end

    test "red with non-color" do
      assert {:error, _} = Builtins.call("red", [{:number, 1.0}])
    end

    test "red with no args" do
      assert {:error, _} = Builtins.call("red", [])
    end

    test "green with non-color" do
      assert {:error, _} = Builtins.call("green", [{:number, 1.0}])
    end

    test "blue with non-color" do
      assert {:error, _} = Builtins.call("blue", [{:number, 1.0}])
    end

    test "hue with non-color" do
      assert {:error, _} = Builtins.call("hue", [{:number, 1.0}])
    end

    test "hue with no args" do
      assert {:error, _} = Builtins.call("hue", [])
    end

    test "saturation with non-color" do
      assert {:error, _} = Builtins.call("saturation", [{:number, 1.0}])
    end

    test "saturation with no args" do
      assert {:error, _} = Builtins.call("saturation", [])
    end

    test "lightness with non-color" do
      assert {:error, _} = Builtins.call("lightness", [{:number, 1.0}])
    end

    test "lightness with no args" do
      assert {:error, _} = Builtins.call("lightness", [])
    end

    test "rgba with unexpected args returns null" do
      assert {:ok, :null} = Builtins.call("rgba", [{:number, 1.0}])
    end

    test "rgba with 4 args" do
      {:ok, {:color, _}} = Builtins.call("rgba", [{:number, 255.0}, {:number, 0.0}, {:number, 0.0}, {:number, 0.5}])
    end

    test "mix with custom weight" do
      {:ok, {:color, _}} = Builtins.call("mix", [{:color, "#ff0000"}, {:color, "#0000ff"}, {:number, 75.0}])
    end
  end

  # ==========================================================================
  # Builtins: list function error paths
  # ==========================================================================

  describe "Builtins list error paths" do
    test "nth with too few args" do
      assert {:error, _} = Builtins.call("nth", [{:list, [{:ident, "a"}]}])
    end

    test "nth with non-numeric index" do
      assert {:error, _} = Builtins.call("nth", [{:list, [{:ident, "a"}]}, {:ident, "x"}])
    end

    test "nth with index < 1" do
      assert {:error, _} = Builtins.call("nth", [{:list, [{:ident, "a"}]}, {:number, 0.0}])
    end

    test "nth on single value" do
      assert {:ok, {:number, 42.0}} = Builtins.call("nth", [{:number, 42.0}, {:number, 1.0}])
    end

    test "nth on single value out of bounds" do
      assert {:error, _} = Builtins.call("nth", [{:number, 42.0}, {:number, 2.0}])
    end

    test "length with no args" do
      assert {:error, _} = Builtins.call("length", [])
    end

    test "length of map" do
      m = {:map, [{"a", {:number, 1.0}}, {"b", {:number, 2.0}}]}
      assert {:ok, {:number, 2.0}} = Builtins.call("length", [m])
    end

    test "join with too few args" do
      assert {:error, _} = Builtins.call("join", [{:list, []}])
    end

    test "join single values" do
      {:ok, {:list, items}} = Builtins.call("join", [{:number, 1.0}, {:number, 2.0}])
      assert length(items) == 2
    end

    test "append with too few args" do
      assert {:error, _} = Builtins.call("append", [{:list, []}])
    end

    test "append to single value" do
      {:ok, {:list, items}} = Builtins.call("append", [{:number, 1.0}, {:number, 2.0}])
      assert length(items) == 2
    end

    test "index with too few args" do
      assert {:error, _} = Builtins.call("index", [{:list, []}])
    end

    test "index on single value" do
      assert {:ok, {:number, 1.0}} = Builtins.call("index", [{:ident, "a"}, {:ident, "a"}])
    end
  end

  # ==========================================================================
  # Builtins: type function error paths
  # ==========================================================================

  describe "Builtins type error paths" do
    test "type-of with no args" do
      assert {:error, _} = Builtins.call("type-of", [])
    end

    test "unit with no args" do
      assert {:error, _} = Builtins.call("unit", [])
    end

    test "unit with non-numeric" do
      assert {:ok, {:string, ""}} = Builtins.call("unit", [{:ident, "red"}])
    end

    test "unitless with no args" do
      assert {:error, _} = Builtins.call("unitless", [])
    end

    test "comparable with too few args" do
      assert {:error, _} = Builtins.call("comparable", [{:number, 1.0}])
    end

    test "comparable number and percentage" do
      assert {:ok, {:bool, true}} = Builtins.call("comparable", [{:number, 1.0}, {:percentage, 50.0}])
    end

    test "comparable non-numeric types" do
      assert {:ok, {:bool, false}} = Builtins.call("comparable", [{:ident, "a"}, {:ident, "b"}])
    end
  end

  # ==========================================================================
  # Builtins: math function error paths
  # ==========================================================================

  describe "Builtins math error paths" do
    test "math.div with too few args" do
      assert {:error, _} = Builtins.call("math.div", [{:number, 1.0}])
    end

    test "math.div with non-numeric" do
      assert {:error, _} = Builtins.call("math.div", [{:ident, "a"}, {:number, 2.0}])
    end

    test "math.div dimension / dimension same unit" do
      assert {:ok, {:number, 2.0}} = Builtins.call("math.div", [{:dimension, 10.0, "px"}, {:dimension, 5.0, "px"}])
    end

    test "math.div percentage / number" do
      assert {:ok, {:percentage, 25.0}} = Builtins.call("math.div", [{:percentage, 50.0}, {:number, 2.0}])
    end

    test "math.floor with no args" do
      assert {:error, _} = Builtins.call("math.floor", [])
    end

    test "math.floor with non-numeric" do
      assert {:error, _} = Builtins.call("math.floor", [{:ident, "abc"}])
    end

    test "math.ceil with no args" do
      assert {:error, _} = Builtins.call("math.ceil", [])
    end

    test "math.ceil preserves unit" do
      assert {:ok, {:dimension, 4.0, "em"}} = Builtins.call("math.ceil", [{:dimension, 3.2, "em"}])
    end

    test "math.round with no args" do
      assert {:error, _} = Builtins.call("math.round", [])
    end

    test "math.round preserves percentage" do
      assert {:ok, {:percentage, 4.0}} = Builtins.call("math.round", [{:percentage, 3.5}])
    end

    test "math.abs with no args" do
      assert {:error, _} = Builtins.call("math.abs", [])
    end

    test "math.abs preserves dimension" do
      assert {:ok, {:dimension, 5.0, "px"}} = Builtins.call("math.abs", [{:dimension, -5.0, "px"}])
    end

    test "math.abs preserves percentage" do
      assert {:ok, {:percentage, 10.0}} = Builtins.call("math.abs", [{:percentage, -10.0}])
    end

    test "math.min with no args" do
      assert {:error, _} = Builtins.call("math.min", [])
    end

    test "math.min with non-numeric args" do
      assert {:error, _} = Builtins.call("math.min", [{:ident, "a"}, {:ident, "b"}])
    end

    test "math.max with no args" do
      assert {:error, _} = Builtins.call("math.max", [])
    end

    test "math.max with non-numeric args" do
      assert {:error, _} = Builtins.call("math.max", [{:ident, "a"}, {:ident, "b"}])
    end
  end

  # ==========================================================================
  # Values: additional edge cases
  # ==========================================================================

  describe "Values edge cases" do
    test "color_to_rgb with invalid hex returns black" do
      assert {0, 0, 0, 1.0} = Values.color_to_rgb("#x")
    end

    test "color_to_rgb with 8-char hex (alpha)" do
      {r, g, b, a} = Values.color_to_rgb("#ff000080")
      assert r == 255
      assert g == 0
      assert b == 0
      assert_in_delta a, 0.502, 0.01
    end

    test "color_from_hsl achromatic (gray)" do
      {:color, hex} = Values.color_from_hsl(0, 0, 50)
      assert String.starts_with?(hex, "#")
    end

    test "color_to_hsl for achromatic (gray)" do
      {h, s, _, _} = Values.color_to_hsl("#808080")
      assert h == 0.0
      assert s == 0.0
    end

    test "divide percentage / number" do
      assert {:ok, {:percentage, 25.0}} = Values.divide({:percentage, 50.0}, {:number, 2.0})
    end

    test "divide incompatible types" do
      assert {:error, _} = Values.divide({:ident, "red"}, {:number, 2.0})
    end

    test "Builtins.names returns a set" do
      names = Builtins.names()
      assert MapSet.member?(names, "map-get")
      assert MapSet.member?(names, "math.div")
    end
  end

  # ==========================================================================
  # Evaluator: eval_value_list
  # ==========================================================================

  describe "Evaluator eval_value_list" do
    alias CodingAdventures.LatticeAstToCss.{Evaluator, Scope}
    alias CodingAdventures.Parser.ASTNode
    alias CodingAdventures.Lexer.Token

    defp tok(type, value), do: %Token{type: type, value: value, line: 1, column: 1}

    test "value_list with arithmetic operators delegates to additive" do
      # Simulates: 2 + 1 -> value_list with [NUMBER(2), PLUS, NUMBER(1)]
      node = %ASTNode{
        rule_name: "value_list",
        children: [tok("NUMBER", "2"), tok("PLUS", "+"), tok("NUMBER", "1")]
      }
      scope = Scope.new()
      result = Evaluator.evaluate(node, scope)
      assert {:number, 3.0} = result
    end

    test "value_list without operators evaluates first child" do
      node = %ASTNode{
        rule_name: "value_list",
        children: [tok("IDENT", "red"), tok("COMMA", ","), tok("IDENT", "blue")]
      }
      scope = Scope.new()
      result = Evaluator.evaluate(node, scope)
      assert {:ident, "red"} = result
    end

    test "value_list with single child evaluates that child" do
      node = %ASTNode{
        rule_name: "value_list",
        children: [tok("NUMBER", "42")]
      }
      scope = Scope.new()
      result = Evaluator.evaluate(node, scope)
      assert {:number, 42.0} = result
    end

    test "value_list with empty children returns null" do
      node = %ASTNode{rule_name: "value_list", children: []}
      scope = Scope.new()
      result = Evaluator.evaluate(node, scope)
      assert :null = result
    end
  end
end
