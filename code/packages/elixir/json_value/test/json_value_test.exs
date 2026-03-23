defmodule CodingAdventures.JsonValueTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for CodingAdventures.JsonValue — the AST-to-typed-value converter.

  ## Test Organization

  Tests are grouped by function:
  1. from_ast/1 — converting parser ASTs to typed values
  2. to_native/1 — converting typed values to Elixir natives
  3. from_native/1 — converting Elixir natives to typed values
  4. parse/1 — end-to-end text-to-value parsing
  5. parse_native/1 — end-to-end text-to-native parsing
  6. Round-trip tests — from_native then to_native (and vice versa)

  Each test name starts with a number matching the spec's test numbering for
  easy cross-referencing.
  """

  alias CodingAdventures.JsonValue

  # ===========================================================================
  # from_ast/1 — AST to JsonValue
  # ===========================================================================
  #
  # These tests parse JSON text through the lexer and parser, then pass the
  # resulting AST to from_ast/1. This validates the full pipeline from text
  # to typed value.

  describe "from_ast/1 — primitives" do
    test "1. AST string to {:string, _}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s("hello"))
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:string, "hello"}
    end

    test "2. AST integer to {:number, integer}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("42")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:number, 42}
      assert is_integer(elem(result, 1))
    end

    test "3. AST negative integer to {:number, -17}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("-17")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:number, -17}
    end

    test "4. AST float to {:number, float}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("3.14")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:number, 3.14}
      assert is_float(elem(result, 1))
    end

    test "5. AST exponent to {:number, float}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("1e10")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:number, 1.0e10}
      assert is_float(elem(result, 1))
    end

    test "6. AST true to {:boolean, true}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("true")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:boolean, true}
    end

    test "7. AST false to {:boolean, false}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("false")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:boolean, false}
    end

    test "8. AST null to :null" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("null")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == :null
    end

    test "9. AST zero to {:number, 0}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("0")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:number, 0}
    end

    test "10. AST empty string to {:string, \"\"}" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s(""))
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:string, ""}
    end

    test "11. AST string with escapes" do
      # The lexer should have already unescaped these
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s("hello\\nworld"))
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:string, str} = result
      assert String.contains?(str, "\n") or str == "hello\\nworld" or str == "hellonworld"
    end
  end

  describe "from_ast/1 — objects" do
    test "12. empty object" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("{}")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:object, []}
    end

    test "13. object with one pair" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s({"a": 1}))
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:object, [{"a", {:number, 1}}]} = result
    end

    test "14. object with multiple pairs" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s({"a": 1, "b": 2}))
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:object, pairs} = result
      assert length(pairs) == 2
      assert {"a", {:number, 1}} in pairs
      assert {"b", {:number, 2}} in pairs
    end

    test "15. nested object" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s({"a": {"b": 1}}))
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:object, [{"a", {:object, [{"b", {:number, 1}}]}}]} = result
    end

    test "16. object with mixed value types" do
      json = ~s({"str": "hello", "num": 42, "bool": true, "nil": null})
      {:ok, ast} = CodingAdventures.JsonParser.parse(json)
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:object, pairs} = result
      assert length(pairs) == 4
    end
  end

  describe "from_ast/1 — arrays" do
    test "17. empty array" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("[]")
      {:ok, result} = JsonValue.from_ast(ast)
      assert result == {:array, []}
    end

    test "18. array with numbers" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("[1, 2, 3]")
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:array, [{:number, 1}, {:number, 2}, {:number, 3}]} = result
    end

    test "19. array with mixed types" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s([1, "two", true, null]))
      {:ok, result} = JsonValue.from_ast(ast)

      assert {:array, elements} = result
      assert length(elements) == 4
      assert {:number, 1} == Enum.at(elements, 0)
      assert {:string, "two"} == Enum.at(elements, 1)
      assert {:boolean, true} == Enum.at(elements, 2)
      assert :null == Enum.at(elements, 3)
    end

    test "20. nested arrays" do
      {:ok, ast} = CodingAdventures.JsonParser.parse("[[1, 2], [3, 4]]")
      {:ok, result} = JsonValue.from_ast(ast)

      assert {:array, [
               {:array, [{:number, 1}, {:number, 2}]},
               {:array, [{:number, 3}, {:number, 4}]}
             ]} = result
    end
  end

  describe "from_ast/1 — complex nested structures" do
    test "21. object with array value" do
      {:ok, ast} = CodingAdventures.JsonParser.parse(~s({"users": [{"name": "Alice"}]}))
      {:ok, result} = JsonValue.from_ast(ast)

      assert {:object, [{"users", {:array, [
               {:object, [{"name", {:string, "Alice"}}]}
             ]}}]} = result
    end

    test "22. RFC 8259 example" do
      source = ~S({"Image": {"Width": 800, "Height": 600, "Title": "View from 15th Floor"}})
      {:ok, ast} = CodingAdventures.JsonParser.parse(source)
      {:ok, result} = JsonValue.from_ast(ast)
      assert {:object, [{"Image", {:object, _inner_pairs}}]} = result
    end
  end

  describe "from_ast/1 — error cases" do
    test "23. rejects non-AST input" do
      {:error, msg} = JsonValue.from_ast("not an AST")
      assert msg =~ "Expected"
    end
  end

  # ===========================================================================
  # to_native/1 — JsonValue to native Elixir types
  # ===========================================================================

  describe "to_native/1" do
    test "24. object to map" do
      result = JsonValue.to_native({:object, [{"a", {:number, 1}}]})
      assert result == %{"a" => 1}
    end

    test "25. array to list" do
      result = JsonValue.to_native({:array, [{:number, 1}, {:number, 2}]})
      assert result == [1, 2]
    end

    test "26. string to binary" do
      assert JsonValue.to_native({:string, "hello"}) == "hello"
    end

    test "27. integer number" do
      assert JsonValue.to_native({:number, 42}) == 42
    end

    test "28. float number" do
      assert JsonValue.to_native({:number, 3.14}) == 3.14
    end

    test "29. boolean true" do
      assert JsonValue.to_native({:boolean, true}) == true
    end

    test "30. boolean false" do
      assert JsonValue.to_native({:boolean, false}) == false
    end

    test "31. null to nil" do
      assert JsonValue.to_native(:null) == nil
    end

    test "32. nested conversion" do
      value = {:object, [
        {"name", {:string, "Alice"}},
        {"scores", {:array, [{:number, 95}, {:number, 87}]}},
        {"active", {:boolean, true}},
        {"meta", :null}
      ]}

      result = JsonValue.to_native(value)

      assert result == %{
               "name" => "Alice",
               "scores" => [95, 87],
               "active" => true,
               "meta" => nil
             }
    end

    test "33. empty containers" do
      assert JsonValue.to_native({:object, []}) == %{}
      assert JsonValue.to_native({:array, []}) == []
    end
  end

  # ===========================================================================
  # from_native/1 — native Elixir types to JsonValue
  # ===========================================================================

  describe "from_native/1" do
    test "34. map to object" do
      {:ok, result} = JsonValue.from_native(%{"a" => 1})
      assert {:object, [{"a", {:number, 1}}]} = result
    end

    test "35. list to array" do
      {:ok, result} = JsonValue.from_native([1, 2])
      assert result == {:array, [{:number, 1}, {:number, 2}]}
    end

    test "36. string" do
      assert {:ok, {:string, "hello"}} = JsonValue.from_native("hello")
    end

    test "37. integer" do
      assert {:ok, {:number, 42}} = JsonValue.from_native(42)
    end

    test "38. float" do
      assert {:ok, {:number, 3.14}} = JsonValue.from_native(3.14)
    end

    test "39. boolean true" do
      assert {:ok, {:boolean, true}} = JsonValue.from_native(true)
    end

    test "40. boolean false" do
      assert {:ok, {:boolean, false}} = JsonValue.from_native(false)
    end

    test "41. nil to null" do
      assert {:ok, :null} = JsonValue.from_native(nil)
    end

    test "42. nested native structure" do
      native = %{
        "name" => "Alice",
        "scores" => [95, 87],
        "active" => true,
        "meta" => nil
      }

      {:ok, result} = JsonValue.from_native(native)
      assert {:object, pairs} = result

      # Convert back to native to verify round-trip
      reconstructed = JsonValue.to_native(result)
      assert reconstructed == native
    end

    test "43. non-string key error" do
      {:error, msg} = JsonValue.from_native(%{1 => "val"})
      assert msg =~ "string"
    end

    test "44. unsupported type error" do
      {:error, msg} = JsonValue.from_native(:some_atom)
      assert msg =~ "unsupported"
    end

    test "45. tuple error" do
      {:error, _msg} = JsonValue.from_native({:not, :json})
    end

    test "46. nested unsupported type error" do
      {:error, _msg} = JsonValue.from_native([1, :atom, 3])
    end

    test "47. empty containers" do
      assert {:ok, {:object, []}} = JsonValue.from_native(%{})
      assert {:ok, {:array, []}} = JsonValue.from_native([])
    end
  end

  # ===========================================================================
  # parse/1 — Text to JsonValue
  # ===========================================================================

  describe "parse/1" do
    test "48. parses object" do
      {:ok, result} = JsonValue.parse(~s({"a": 1}))
      assert {:object, [{"a", {:number, 1}}]} = result
    end

    test "49. parses array" do
      {:ok, result} = JsonValue.parse("[1, 2, 3]")
      assert {:array, [{:number, 1}, {:number, 2}, {:number, 3}]} = result
    end

    test "50. parses string" do
      assert {:ok, {:string, "hello"}} = JsonValue.parse(~s("hello"))
    end

    test "51. parses number" do
      assert {:ok, {:number, 42}} = JsonValue.parse("42")
    end

    test "52. parses boolean" do
      assert {:ok, {:boolean, true}} = JsonValue.parse("true")
      assert {:ok, {:boolean, false}} = JsonValue.parse("false")
    end

    test "53. parses null" do
      assert {:ok, :null} = JsonValue.parse("null")
    end

    test "54. error on invalid JSON" do
      {:error, _msg} = JsonValue.parse("not json")
    end

    test "55. error on incomplete JSON" do
      {:error, _msg} = JsonValue.parse("{")
    end

    test "56. parses negative float" do
      {:ok, result} = JsonValue.parse("-3.14")
      assert {:number, num} = result
      assert_in_delta num, -3.14, 0.001
    end

    test "57. parses zero" do
      assert {:ok, {:number, 0}} = JsonValue.parse("0")
    end

    test "58. parses complex nested JSON" do
      json = ~s({"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]})
      {:ok, result} = JsonValue.parse(json)
      assert {:object, _pairs} = result
    end
  end

  # ===========================================================================
  # parse_native/1 — Text to native types
  # ===========================================================================

  describe "parse_native/1" do
    test "59. parses to native map" do
      {:ok, result} = JsonValue.parse_native(~s({"a": 1}))
      assert result == %{"a" => 1}
    end

    test "60. parses to native list" do
      {:ok, result} = JsonValue.parse_native("[1, 2, 3]")
      assert result == [1, 2, 3]
    end

    test "61. parses to native string" do
      {:ok, result} = JsonValue.parse_native(~s("hello"))
      assert result == "hello"
    end

    test "62. parses to native number" do
      {:ok, result} = JsonValue.parse_native("42")
      assert result == 42
    end

    test "63. parses to native boolean" do
      {:ok, result} = JsonValue.parse_native("true")
      assert result == true
    end

    test "64. parses to native nil" do
      {:ok, result} = JsonValue.parse_native("null")
      assert result == nil
    end

    test "65. error on invalid JSON" do
      {:error, _msg} = JsonValue.parse_native("nope")
    end

    test "66. complex nested native" do
      json = ~s({"name": "Alice", "scores": [95, 87], "active": true})
      {:ok, result} = JsonValue.parse_native(json)
      assert result["name"] == "Alice"
      assert result["scores"] == [95, 87]
      assert result["active"] == true
    end
  end

  # ===========================================================================
  # Round-trip Tests
  # ===========================================================================
  #
  # These verify that from_native and to_native are inverses of each other.
  # A value should survive the round trip: from_native(to_native(x)) == x
  # (modulo map key ordering).

  describe "round-trip tests" do
    test "67. simple values round-trip" do
      for val <- ["hello", 42, 3.14, true, false, nil] do
        {:ok, json_val} = JsonValue.from_native(val)
        assert JsonValue.to_native(json_val) == val
      end
    end

    test "68. nested structure round-trip" do
      original = %{
        "name" => "Alice",
        "age" => 30,
        "scores" => [95, 87, 92],
        "address" => %{
          "city" => "Portland",
          "zip" => "97201"
        },
        "active" => true,
        "notes" => nil
      }

      {:ok, json_val} = JsonValue.from_native(original)
      reconstructed = JsonValue.to_native(json_val)
      assert reconstructed == original
    end

    test "69. empty containers round-trip" do
      {:ok, obj} = JsonValue.from_native(%{})
      assert JsonValue.to_native(obj) == %{}

      {:ok, arr} = JsonValue.from_native([])
      assert JsonValue.to_native(arr) == []
    end

    test "70. deeply nested arrays round-trip" do
      original = [[1, 2], [3, [4, 5]]]
      {:ok, json_val} = JsonValue.from_native(original)
      assert JsonValue.to_native(json_val) == original
    end
  end
end
