defmodule CodingAdventures.JsonSerializerTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for CodingAdventures.JsonSerializer — JSON value to text conversion.

  ## Test Organization

  Tests are grouped by function:
  1. serialize/1 — compact JSON output
  2. serialize_pretty/2 — pretty-printed JSON output
  3. stringify/1 — compact JSON from native types
  4. stringify_pretty/2 — pretty-printed JSON from native types
  5. Full round-trip tests — parse + serialize
  6. String escaping edge cases
  """

  alias CodingAdventures.JsonSerializer
  alias CodingAdventures.JsonValue

  # ===========================================================================
  # serialize/1 — Compact JSON output
  # ===========================================================================
  #
  # Compact mode produces the smallest possible valid JSON text. No whitespace
  # is added between tokens — colons have no trailing space, commas have no
  # trailing space, and there are no newlines.

  describe "serialize/1 — primitives" do
    test "1. null" do
      assert {:ok, "null"} = JsonSerializer.serialize(:null)
    end

    test "2. true" do
      assert {:ok, "true"} = JsonSerializer.serialize({:boolean, true})
    end

    test "3. false" do
      assert {:ok, "false"} = JsonSerializer.serialize({:boolean, false})
    end

    test "4. positive integer" do
      assert {:ok, "42"} = JsonSerializer.serialize({:number, 42})
    end

    test "5. negative integer" do
      assert {:ok, "-5"} = JsonSerializer.serialize({:number, -5})
    end

    test "6. zero" do
      assert {:ok, "0"} = JsonSerializer.serialize({:number, 0})
    end

    test "7. float" do
      {:ok, text} = JsonSerializer.serialize({:number, 3.14})
      assert String.to_float(text) == 3.14
    end

    test "8. negative float" do
      {:ok, text} = JsonSerializer.serialize({:number, -2.5})
      assert String.to_float(text) == -2.5
    end

    test "9. simple string" do
      assert {:ok, ~s("hello")} = JsonSerializer.serialize({:string, "hello"})
    end

    test "10. empty string" do
      assert {:ok, ~s("")} = JsonSerializer.serialize({:string, ""})
    end
  end

  describe "serialize/1 — string escaping" do
    test "11. escapes newline" do
      {:ok, text} = JsonSerializer.serialize({:string, "a\nb"})
      assert text == ~s("a\\nb")
    end

    test "12. escapes quote" do
      {:ok, text} = JsonSerializer.serialize({:string, ~s(say "hi")})
      assert text == ~s("say \\"hi\\"")
    end

    test "13. escapes backslash" do
      {:ok, text} = JsonSerializer.serialize({:string, "a\\b"})
      assert text == ~s("a\\\\b")
    end

    test "14. escapes tab" do
      {:ok, text} = JsonSerializer.serialize({:string, "\t"})
      assert text == ~s("\\t")
    end

    test "15. escapes carriage return" do
      {:ok, text} = JsonSerializer.serialize({:string, "\r"})
      assert text == ~s("\\r")
    end

    test "16. escapes backspace" do
      {:ok, text} = JsonSerializer.serialize({:string, "\b"})
      assert text == ~s("\\b")
    end

    test "17. escapes form feed" do
      {:ok, text} = JsonSerializer.serialize({:string, "\f"})
      assert text == ~s("\\f")
    end

    test "18. escapes null byte (control char)" do
      {:ok, text} = JsonSerializer.serialize({:string, <<0>>})
      assert text == ~s("\\u0000")
    end

    test "19. escapes other control chars" do
      # U+0001 (SOH)
      {:ok, text} = JsonSerializer.serialize({:string, <<1>>})
      assert text == ~s("\\u0001")
    end

    test "20. does NOT escape forward slash" do
      {:ok, text} = JsonSerializer.serialize({:string, "a/b"})
      assert text == ~s("a/b")
    end

    test "21. preserves Unicode characters" do
      {:ok, text} = JsonSerializer.serialize({:string, "hello"})
      assert text == ~s("hello")
    end

    test "22. multiple escapes in one string" do
      {:ok, text} = JsonSerializer.serialize({:string, "line1\nline2\ttab"})
      assert text == ~s("line1\\nline2\\ttab")
    end
  end

  describe "serialize/1 — arrays" do
    test "23. empty array" do
      assert {:ok, "[]"} = JsonSerializer.serialize({:array, []})
    end

    test "24. single element" do
      assert {:ok, "[1]"} = JsonSerializer.serialize({:array, [{:number, 1}]})
    end

    test "25. multiple elements" do
      arr = {:array, [{:number, 1}, {:number, 2}, {:number, 3}]}
      assert {:ok, "[1,2,3]"} = JsonSerializer.serialize(arr)
    end

    test "26. mixed types" do
      arr = {:array, [{:number, 1}, {:string, "two"}, {:boolean, true}, :null]}
      assert {:ok, ~s([1,"two",true,null])} = JsonSerializer.serialize(arr)
    end

    test "27. nested arrays" do
      arr = {:array, [{:array, [{:number, 1}]}, {:array, [{:number, 2}]}]}
      assert {:ok, "[[1],[2]]"} = JsonSerializer.serialize(arr)
    end
  end

  describe "serialize/1 — objects" do
    test "28. empty object" do
      assert {:ok, "{}"} = JsonSerializer.serialize({:object, []})
    end

    test "29. single pair" do
      obj = {:object, [{"a", {:number, 1}}]}
      assert {:ok, ~s({"a":1})} = JsonSerializer.serialize(obj)
    end

    test "30. multiple pairs" do
      obj = {:object, [{"a", {:number, 1}}, {"b", {:number, 2}}]}
      assert {:ok, ~s({"a":1,"b":2})} = JsonSerializer.serialize(obj)
    end

    test "31. nested object" do
      obj = {:object, [{"outer", {:object, [{"inner", {:number, 1}}]}}]}
      assert {:ok, ~s({"outer":{"inner":1}})} = JsonSerializer.serialize(obj)
    end

    test "32. object with array value" do
      obj = {:object, [{"nums", {:array, [{:number, 1}, {:number, 2}]}}]}
      assert {:ok, ~s({"nums":[1,2]})} = JsonSerializer.serialize(obj)
    end

    test "33. key with special characters" do
      obj = {:object, [{"key with \"quotes\"", {:number, 1}}]}
      {:ok, text} = JsonSerializer.serialize(obj)
      assert text == ~s({"key with \\"quotes\\"":1})
    end
  end

  # ===========================================================================
  # serialize_pretty/2 — Pretty-printed output
  # ===========================================================================
  #
  # Pretty mode adds newlines and indentation for readability. The default
  # configuration is 2-space indentation with no key sorting.

  describe "serialize_pretty/2 — defaults" do
    test "34. empty object stays compact" do
      assert {:ok, "{}"} = JsonSerializer.serialize_pretty({:object, []})
    end

    test "35. simple object with 2-space indent" do
      obj = {:object, [{"a", {:number, 1}}]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj)
      assert text == "{\n  \"a\": 1\n}"
    end

    test "36. nested object indentation" do
      obj = {:object, [
        {"outer", {:object, [{"inner", {:number, 1}}]}}
      ]}

      {:ok, text} = JsonSerializer.serialize_pretty(obj)

      expected = """
      {
        "outer": {
          "inner": 1
        }
      }\
      """

      assert text == expected
    end

    test "37. empty array stays compact" do
      assert {:ok, "[]"} = JsonSerializer.serialize_pretty({:array, []})
    end

    test "38. array with elements" do
      arr = {:array, [{:number, 1}, {:number, 2}]}
      {:ok, text} = JsonSerializer.serialize_pretty(arr)
      assert text == "[\n  1,\n  2\n]"
    end

    test "39. nested array indentation" do
      arr = {:array, [
        {:array, [{:number, 1}, {:number, 2}]},
        {:array, [{:number, 3}]}
      ]}

      {:ok, text} = JsonSerializer.serialize_pretty(arr)

      expected = """
      [
        [
          1,
          2
        ],
        [
          3
        ]
      ]\
      """

      assert text == expected
    end

    test "40. mixed object with array" do
      obj = {:object, [
        {"name", {:string, "Alice"}},
        {"scores", {:array, [{:number, 95}, {:number, 87}]}},
        {"active", {:boolean, true}}
      ]}

      {:ok, text} = JsonSerializer.serialize_pretty(obj)

      expected = """
      {
        "name": "Alice",
        "scores": [
          95,
          87
        ],
        "active": true
      }\
      """

      assert text == expected
    end
  end

  describe "serialize_pretty/2 — custom options" do
    test "41. 4-space indent" do
      obj = {:object, [{"a", {:number, 1}}]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj, indent_size: 4)
      assert text == "{\n    \"a\": 1\n}"
    end

    test "42. tab indent" do
      obj = {:object, [{"a", {:number, 1}}]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj, indent_char: "\t", indent_size: 1)
      assert text == "{\n\t\"a\": 1\n}"
    end

    test "43. sort keys" do
      obj = {:object, [{"c", {:number, 3}}, {"a", {:number, 1}}, {"b", {:number, 2}}]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj, sort_keys: true)

      expected = """
      {
        "a": 1,
        "b": 2,
        "c": 3
      }\
      """

      assert text == expected
    end

    test "44. trailing newline" do
      obj = {:object, [{"a", {:number, 1}}]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj, trailing_newline: true)
      assert String.ends_with?(text, "\n")
      assert text == "{\n  \"a\": 1\n}\n"
    end

    test "45. combined options" do
      obj = {:object, [{"b", {:number, 2}}, {"a", {:number, 1}}]}

      {:ok, text} =
        JsonSerializer.serialize_pretty(obj,
          indent_size: 4,
          sort_keys: true,
          trailing_newline: true
        )

      expected = "{\n    \"a\": 1,\n    \"b\": 2\n}\n"
      assert text == expected
    end

    test "46. primitives unchanged in pretty mode" do
      assert {:ok, "null"} = JsonSerializer.serialize_pretty(:null)
      assert {:ok, "true"} = JsonSerializer.serialize_pretty({:boolean, true})
      assert {:ok, "42"} = JsonSerializer.serialize_pretty({:number, 42})
      assert {:ok, ~s("hi")} = JsonSerializer.serialize_pretty({:string, "hi"})
    end

    test "47. trailing newline on primitive" do
      {:ok, text} = JsonSerializer.serialize_pretty(:null, trailing_newline: true)
      assert text == "null\n"
    end
  end

  # ===========================================================================
  # stringify/1 — Compact JSON from native types
  # ===========================================================================

  describe "stringify/1" do
    test "48. map" do
      {:ok, text} = JsonSerializer.stringify(%{"a" => 1})
      assert text == ~s({"a":1})
    end

    test "49. list" do
      {:ok, text} = JsonSerializer.stringify([1, 2])
      assert text == "[1,2]"
    end

    test "50. string" do
      assert {:ok, ~s("hello")} = JsonSerializer.stringify("hello")
    end

    test "51. integer" do
      assert {:ok, "42"} = JsonSerializer.stringify(42)
    end

    test "52. float" do
      {:ok, text} = JsonSerializer.stringify(3.14)
      assert String.to_float(text) == 3.14
    end

    test "53. boolean" do
      assert {:ok, "true"} = JsonSerializer.stringify(true)
      assert {:ok, "false"} = JsonSerializer.stringify(false)
    end

    test "54. nil" do
      assert {:ok, "null"} = JsonSerializer.stringify(nil)
    end

    test "55. nested native structure" do
      native = %{"name" => "Alice", "scores" => [95, 87]}
      {:ok, text} = JsonSerializer.stringify(native)
      # Should be valid compact JSON
      assert String.contains?(text, "\"name\"")
      assert String.contains?(text, "\"Alice\"")
    end

    test "56. error on unsupported type" do
      {:error, _msg} = JsonSerializer.stringify(:atom)
    end
  end

  # ===========================================================================
  # stringify_pretty/2 — Pretty JSON from native types
  # ===========================================================================

  describe "stringify_pretty/2" do
    test "57. pretty map" do
      {:ok, text} = JsonSerializer.stringify_pretty(%{"a" => 1})
      assert text == "{\n  \"a\": 1\n}"
    end

    test "58. pretty list" do
      {:ok, text} = JsonSerializer.stringify_pretty([1, 2])
      assert text == "[\n  1,\n  2\n]"
    end

    test "59. pretty with options" do
      {:ok, text} = JsonSerializer.stringify_pretty(%{"b" => 2, "a" => 1}, sort_keys: true)
      # Keys should be sorted
      a_pos = :binary.match(text, "\"a\"") |> elem(0)
      b_pos = :binary.match(text, "\"b\"") |> elem(0)
      assert a_pos < b_pos
    end

    test "60. error on unsupported type" do
      {:error, _msg} = JsonSerializer.stringify_pretty(:atom)
    end
  end

  # ===========================================================================
  # Full Round-trip Tests — parse then serialize
  # ===========================================================================
  #
  # These tests verify that parsing JSON text and then serializing it back
  # produces equivalent output. Note that compact serialization may differ
  # from the original input (whitespace is stripped), but the semantic content
  # should be identical.

  describe "round-trip: parse then serialize" do
    test "61. simple object" do
      original = ~s({"a":1})
      {:ok, value} = JsonValue.parse(original)
      {:ok, text} = JsonSerializer.serialize(value)
      assert text == original
    end

    test "62. simple array" do
      original = "[1,2,3]"
      {:ok, value} = JsonValue.parse(original)
      {:ok, text} = JsonSerializer.serialize(value)
      assert text == original
    end

    test "63. string value" do
      original = ~s("hello world")
      {:ok, value} = JsonValue.parse(original)
      {:ok, text} = JsonSerializer.serialize(value)
      assert text == original
    end

    test "64. number values" do
      for num_str <- ["42", "0", "-5"] do
        {:ok, value} = JsonValue.parse(num_str)
        {:ok, text} = JsonSerializer.serialize(value)
        assert text == num_str
      end
    end

    test "65. boolean and null" do
      for literal <- ["true", "false", "null"] do
        {:ok, value} = JsonValue.parse(literal)
        {:ok, text} = JsonSerializer.serialize(value)
        assert text == literal
      end
    end

    test "66. empty containers" do
      for container <- ["[]", "{}"] do
        {:ok, value} = JsonValue.parse(container)
        {:ok, text} = JsonSerializer.serialize(value)
        assert text == container
      end
    end

    test "67. complex nested structure" do
      # Parse with whitespace, serialize to compact
      source = ~s({"users": [{"name": "Alice", "age": 30}], "count": 1})
      {:ok, value} = JsonValue.parse(source)
      {:ok, text} = JsonSerializer.serialize(value)

      # Re-parse the compact output — should produce identical value
      {:ok, reparsed} = JsonValue.parse(text)
      assert JsonValue.to_native(value) == JsonValue.to_native(reparsed)
    end

    test "68. native round-trip via stringify" do
      original = %{"name" => "Alice", "age" => 30}
      {:ok, text} = JsonSerializer.stringify(original)
      {:ok, reparsed} = JsonValue.parse_native(text)
      assert reparsed == original
    end
  end

  # ===========================================================================
  # Edge Cases
  # ===========================================================================

  describe "edge cases" do
    test "69. large integer" do
      assert {:ok, "1000000"} = JsonSerializer.serialize({:number, 1_000_000})
    end

    test "70. very small float" do
      {:ok, text} = JsonSerializer.serialize({:number, 0.001})
      assert String.to_float(text) == 0.001
    end

    test "71. string with only special chars" do
      {:ok, text} = JsonSerializer.serialize({:string, "\"\\\n\r\t"})
      assert text == ~s("\\\"\\\\\\n\\r\\t")
    end

    test "72. deeply nested structure" do
      # 5 levels of nesting
      deep = {:object, [
        {"level1", {:object, [
          {"level2", {:object, [
            {"level3", {:array, [
              {:object, [{"level4", {:string, "deep"}}]}
            ]}}
          ]}}
        ]}}
      ]}

      {:ok, compact} = JsonSerializer.serialize(deep)
      {:ok, pretty} = JsonSerializer.serialize_pretty(deep)

      # Both should be valid — re-parse and compare
      {:ok, from_compact} = JsonValue.parse(compact)
      {:ok, from_pretty} = JsonValue.parse(pretty)
      assert JsonValue.to_native(from_compact) == JsonValue.to_native(from_pretty)
    end

    test "73. object with empty string key" do
      obj = {:object, [{"", {:number, 1}}]}
      assert {:ok, ~s({"":1})} = JsonSerializer.serialize(obj)
    end

    test "74. array of arrays of arrays" do
      arr = {:array, [{:array, [{:array, [{:number, 1}]}]}]}
      assert {:ok, "[[[1]]]"} = JsonSerializer.serialize(arr)
    end

    test "75. pretty-print preserves key order without sort_keys" do
      obj = {:object, [{"z", {:number, 1}}, {"a", {:number, 2}}]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj)
      z_pos = :binary.match(text, "\"z\"") |> elem(0)
      a_pos = :binary.match(text, "\"a\"") |> elem(0)
      assert z_pos < a_pos
    end

    test "76. pretty-print single-element array" do
      arr = {:array, [{:number, 42}]}
      {:ok, text} = JsonSerializer.serialize_pretty(arr)
      assert text == "[\n  42\n]"
    end

    test "77. pretty-print nested arrays with sort_keys" do
      obj = {:object, [
        {"b", {:array, [{:number, 2}]}},
        {"a", {:array, [{:number, 1}]}}
      ]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj, sort_keys: true)
      a_pos = :binary.match(text, "\"a\"") |> elem(0)
      b_pos = :binary.match(text, "\"b\"") |> elem(0)
      assert a_pos < b_pos
    end

    test "78. compact serialize string with all escape types" do
      # String with every escape char type
      input_str = "\"\\\b\f\n\r\t" <> <<0x01>> <> <<0x1F>>
      {:ok, text} = JsonSerializer.serialize({:string, input_str})
      assert String.contains?(text, "\\\"")
      assert String.contains?(text, "\\\\")
      assert String.contains?(text, "\\b")
      assert String.contains?(text, "\\f")
      assert String.contains?(text, "\\n")
      assert String.contains?(text, "\\r")
      assert String.contains?(text, "\\t")
      assert String.contains?(text, "\\u0001")
      assert String.contains?(text, "\\u001f")
    end

    test "79. compact serialize large negative integer" do
      assert {:ok, "-999999"} = JsonSerializer.serialize({:number, -999_999})
    end

    test "80. pretty-print with trailing newline on array" do
      arr = {:array, [{:number, 1}]}
      {:ok, text} = JsonSerializer.serialize_pretty(arr, trailing_newline: true)
      assert String.ends_with?(text, "\n")
    end

    test "81. pretty-print boolean value" do
      {:ok, text} = JsonSerializer.serialize_pretty({:boolean, false})
      assert text == "false"
    end

    test "82. pretty-print with trailing newline on string" do
      {:ok, text} = JsonSerializer.serialize_pretty({:string, "hi"}, trailing_newline: true)
      assert text == "\"hi\"\n"
    end

    test "83. stringify_pretty with trailing_newline" do
      {:ok, text} = JsonSerializer.stringify_pretty(%{"a" => 1}, trailing_newline: true)
      assert String.ends_with?(text, "\n")
    end

    test "84. stringify empty list" do
      assert {:ok, "[]"} = JsonSerializer.stringify([])
    end

    test "85. stringify empty map" do
      assert {:ok, "{}"} = JsonSerializer.stringify(%{})
    end

    test "86. pretty multiple pairs object" do
      obj = {:object, [
        {"a", {:number, 1}},
        {"b", {:string, "two"}},
        {"c", {:boolean, true}},
        {"d", :null}
      ]}
      {:ok, text} = JsonSerializer.serialize_pretty(obj)
      expected = "{\n  \"a\": 1,\n  \"b\": \"two\",\n  \"c\": true,\n  \"d\": null\n}"
      assert text == expected
    end

    test "87. compact array with nested objects" do
      arr = {:array, [{:object, [{"x", {:number, 1}}]}, {:object, [{"y", {:number, 2}}]}]}
      {:ok, text} = JsonSerializer.serialize(arr)
      assert text == ~s([{"x":1},{"y":2}])
    end

    test "88. compact object with null value" do
      obj = {:object, [{"key", :null}]}
      assert {:ok, ~s({"key":null})} = JsonSerializer.serialize(obj)
    end

    test "89. compact object with boolean values" do
      obj = {:object, [{"t", {:boolean, true}}, {"f", {:boolean, false}}]}
      assert {:ok, ~s({"t":true,"f":false})} = JsonSerializer.serialize(obj)
    end

    test "90. stringify nested list" do
      {:ok, text} = JsonSerializer.stringify([[1, 2], [3]])
      assert text == "[[1,2],[3]]"
    end
  end
end
