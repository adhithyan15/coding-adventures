# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for CodingAdventures::JsonValue
# ================================================================
#
# These tests verify the complete json-value pipeline:
#
#   1. from_ast   -- AST nodes -> typed JsonValue objects
#   2. to_native  -- JsonValue -> Ruby Hash/Array/String/etc.
#   3. from_native -- Ruby native types -> JsonValue
#   4. parse       -- JSON text -> JsonValue (convenience)
#   5. parse_native -- JSON text -> native Ruby types (convenience)
#   6. Round-trips -- from_native -> to_native preserves values
#
# The test strategy follows the spec (D20-json.md) test matrix,
# covering every JSON type, edge case, and error path.
# ================================================================

class TestJsonValueFromAst < Minitest::Test
  JV = CodingAdventures::JsonValue

  # Helper: parse JSON text to AST, then convert to JsonValue.
  # This exercises both the parser and from_ast in combination.
  def from_json(text)
    ast = CodingAdventures::JsonParser.parse(text)
    JV.from_ast(ast)
  end

  # ==============================================================
  # from_ast: Primitive Values
  # ==============================================================

  # Spec test #3: AST string to JsonString
  # The simplest case: a standalone JSON string.
  def test_from_ast_string
    result = from_json('"hello"')
    assert_instance_of JV::String, result
    assert_equal "hello", result.value
  end

  # Spec test #19: AST empty string
  # Edge case: the empty string is a valid JSON value.
  def test_from_ast_empty_string
    result = from_json('""')
    assert_instance_of JV::String, result
    assert_equal "", result.value
  end

  # Spec test #18: AST string with escape sequences
  # The lexer unescapes \n, \t, etc., so the JsonValue should
  # contain actual newline and tab characters.
  def test_from_ast_string_with_escapes
    result = from_json('"hello\\nworld"')
    assert_instance_of JV::String, result
    assert_equal "hello\nworld", result.value
  end

  # Spec test #4: AST integer to JsonNumber
  # Numbers without decimal points or exponents are integers.
  def test_from_ast_integer
    result = from_json("42")
    assert_instance_of JV::Number, result
    assert_equal 42, result.value
    assert result.integer?, "42 should be stored as integer"
  end

  # Spec test #20: AST zero
  # Zero is a valid integer.
  def test_from_ast_zero
    result = from_json("0")
    assert_instance_of JV::Number, result
    assert_equal 0, result.value
    assert result.integer?
  end

  # Spec test #5: AST negative integer
  def test_from_ast_negative_integer
    result = from_json("-17")
    assert_instance_of JV::Number, result
    assert_equal(-17, result.value)
    assert result.integer?
  end

  # Spec test #6: AST float to JsonNumber
  # Numbers with decimal points are floats.
  def test_from_ast_float
    result = from_json("3.14")
    assert_instance_of JV::Number, result
    assert_in_delta 3.14, result.value, 0.001
    refute result.integer?, "3.14 should be stored as float"
  end

  # Spec test #7: AST exponent to JsonNumber
  # Numbers with exponents are always stored as floats.
  def test_from_ast_exponent
    result = from_json("1e10")
    assert_instance_of JV::Number, result
    assert_in_delta 1e10, result.value, 1
    refute result.integer?, "1e10 should be stored as float"
  end

  # Spec test #8: AST true to JsonBool
  def test_from_ast_true
    result = from_json("true")
    assert_instance_of JV::Boolean, result
    assert_equal true, result.value
  end

  # Spec test #9: AST false to JsonBool
  def test_from_ast_false
    result = from_json("false")
    assert_instance_of JV::Boolean, result
    assert_equal false, result.value
  end

  # Spec test #10: AST null to JsonNull
  def test_from_ast_null
    result = from_json("null")
    assert_instance_of JV::Null, result
  end

  # ==============================================================
  # from_ast: Objects
  # ==============================================================

  # Spec test #1: Empty object
  def test_from_ast_empty_object
    result = from_json("{}")
    assert_instance_of JV::Object, result
    assert_equal({}, result.pairs)
  end

  # Spec test #11: Simple object with one pair
  def test_from_ast_simple_object
    result = from_json('{"a": 1}')
    assert_instance_of JV::Object, result
    assert_equal 1, result.pairs.length
    assert_instance_of JV::Number, result.pairs["a"]
    assert_equal 1, result.pairs["a"].value
  end

  # Spec test #12: Object with multiple pairs
  def test_from_ast_multi_key_object
    result = from_json('{"a": 1, "b": 2}')
    assert_instance_of JV::Object, result
    assert_equal 2, result.pairs.length
    assert_equal 1, result.pairs["a"].value
    assert_equal 2, result.pairs["b"].value
  end

  # Spec test #15: Nested object
  def test_from_ast_nested_object
    result = from_json('{"a": {"b": 1}}')
    assert_instance_of JV::Object, result
    inner = result.pairs["a"]
    assert_instance_of JV::Object, inner
    assert_equal 1, inner.pairs["b"].value
  end

  # ==============================================================
  # from_ast: Arrays
  # ==============================================================

  # Spec test #2: Empty array
  def test_from_ast_empty_array
    result = from_json("[]")
    assert_instance_of JV::Array, result
    assert_equal [], result.elements
  end

  # Spec test #13: Simple array with numbers
  def test_from_ast_simple_array
    result = from_json("[1, 2, 3]")
    assert_instance_of JV::Array, result
    assert_equal 3, result.elements.length
    assert_equal [1, 2, 3], result.elements.map(&:value)
  end

  # Spec test #14: Mixed-type array
  # JSON arrays can contain any mix of value types.
  def test_from_ast_mixed_array
    result = from_json('[1, "two", true, null]')
    assert_instance_of JV::Array, result
    assert_equal 4, result.elements.length
    assert_instance_of JV::Number, result.elements[0]
    assert_instance_of JV::String, result.elements[1]
    assert_instance_of JV::Boolean, result.elements[2]
    assert_instance_of JV::Null, result.elements[3]
  end

  # Spec test #16: Nested arrays
  def test_from_ast_nested_arrays
    result = from_json("[[1, 2], [3, 4]]")
    assert_instance_of JV::Array, result
    assert_equal 2, result.elements.length
    assert_instance_of JV::Array, result.elements[0]
    assert_instance_of JV::Array, result.elements[1]
    assert_equal [1, 2], result.elements[0].elements.map(&:value)
    assert_equal [3, 4], result.elements[1].elements.map(&:value)
  end

  # ==============================================================
  # from_ast: Complex Nested Structures
  # ==============================================================

  # Spec test #17: Complex nested structure
  # This exercises the full recursive walk.
  def test_from_ast_complex_nested
    json = '{"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]}'
    result = from_json(json)

    assert_instance_of JV::Object, result
    users = result.pairs["users"]
    assert_instance_of JV::Array, users
    assert_equal 2, users.elements.length

    alice = users.elements[0]
    assert_instance_of JV::Object, alice
    assert_equal "Alice", alice.pairs["name"].value
    assert_equal 30, alice.pairs["age"].value

    bob = users.elements[1]
    assert_equal "Bob", bob.pairs["name"].value
    assert_equal 25, bob.pairs["age"].value
  end

  # Object with all value types as values
  def test_from_ast_object_with_all_types
    json = '{"str": "hi", "num": 42, "flt": 1.5, "t": true, "f": false, "n": null, "arr": [], "obj": {}}'
    result = from_json(json)

    assert_instance_of JV::String, result.pairs["str"]
    assert_instance_of JV::Number, result.pairs["num"]
    assert_instance_of JV::Number, result.pairs["flt"]
    assert_instance_of JV::Boolean, result.pairs["t"]
    assert_instance_of JV::Boolean, result.pairs["f"]
    assert_instance_of JV::Null, result.pairs["n"]
    assert_instance_of JV::Array, result.pairs["arr"]
    assert_instance_of JV::Object, result.pairs["obj"]
  end
end

# ================================================================
# Tests for to_native
# ================================================================

class TestJsonValueToNative < Minitest::Test
  JV = CodingAdventures::JsonValue

  # Spec test #21: JsonObject to Hash
  def test_to_native_object
    obj = JV::Object.new(pairs: { "a" => JV::Number.new(value: 1) })
    result = JV.to_native(obj)
    assert_equal({ "a" => 1 }, result)
  end

  # Spec test #22: JsonArray to Array
  def test_to_native_array
    arr = JV::Array.new(elements: [JV::Number.new(value: 1), JV::Number.new(value: 2)])
    result = JV.to_native(arr)
    assert_equal [1, 2], result
  end

  # Spec test #23: JsonString to String
  def test_to_native_string
    str = JV::String.new(value: "hello")
    assert_equal "hello", JV.to_native(str)
  end

  # Spec test #24: JsonNumber int to Integer
  def test_to_native_integer
    num = JV::Number.new(value: 42)
    result = JV.to_native(num)
    assert_equal 42, result
    assert_instance_of Integer, result
  end

  # Spec test #25: JsonNumber float to Float
  def test_to_native_float
    num = JV::Number.new(value: 3.14)
    result = JV.to_native(num)
    assert_in_delta 3.14, result, 0.001
    assert_instance_of Float, result
  end

  # Spec test #26: JsonBool to boolean
  def test_to_native_boolean_true
    assert_equal true, JV.to_native(JV::Boolean.new(value: true))
  end

  def test_to_native_boolean_false
    assert_equal false, JV.to_native(JV::Boolean.new(value: false))
  end

  # Spec test #27: JsonNull to nil
  def test_to_native_null
    assert_nil JV.to_native(JV::Null.new)
  end

  # Spec test #28: Nested to_native
  # A deeply nested structure should be fully converted.
  def test_to_native_nested
    obj = JV::Object.new(pairs: {
      "users" => JV::Array.new(elements: [
        JV::Object.new(pairs: {
          "name" => JV::String.new(value: "Alice"),
          "scores" => JV::Array.new(elements: [
            JV::Number.new(value: 95),
            JV::Number.new(value: 87)
          ])
        })
      ]),
      "count" => JV::Number.new(value: 1),
      "active" => JV::Boolean.new(value: true)
    })

    expected = {
      "users" => [
        { "name" => "Alice", "scores" => [95, 87] }
      ],
      "count" => 1,
      "active" => true
    }

    assert_equal expected, JV.to_native(obj)
  end

  # Empty containers
  def test_to_native_empty_object
    assert_equal({}, JV.to_native(JV::Object.new))
  end

  def test_to_native_empty_array
    assert_equal [], JV.to_native(JV::Array.new)
  end
end

# ================================================================
# Tests for from_native
# ================================================================

class TestJsonValueFromNative < Minitest::Test
  JV = CodingAdventures::JsonValue

  # Spec test #29: Hash to JsonObject
  def test_from_native_hash
    result = JV.from_native({ "a" => 1 })
    assert_instance_of JV::Object, result
    assert_equal 1, result.pairs["a"].value
  end

  # Spec test #30: Array to JsonArray
  def test_from_native_array
    result = JV.from_native([1, 2])
    assert_instance_of JV::Array, result
    assert_equal [1, 2], result.elements.map(&:value)
  end

  # Spec test #31: String to JsonString
  def test_from_native_string
    result = JV.from_native("hello")
    assert_instance_of JV::String, result
    assert_equal "hello", result.value
  end

  # Spec test #32: Integer to JsonNumber
  def test_from_native_integer
    result = JV.from_native(42)
    assert_instance_of JV::Number, result
    assert_equal 42, result.value
    assert result.integer?
  end

  # Spec test #33: Float to JsonNumber
  def test_from_native_float
    result = JV.from_native(3.14)
    assert_instance_of JV::Number, result
    assert_in_delta 3.14, result.value, 0.001
    refute result.integer?
  end

  # Spec test #34: Boolean to JsonBool
  def test_from_native_true
    result = JV.from_native(true)
    assert_instance_of JV::Boolean, result
    assert_equal true, result.value
  end

  def test_from_native_false
    result = JV.from_native(false)
    assert_instance_of JV::Boolean, result
    assert_equal false, result.value
  end

  # Spec test #35: nil to JsonNull
  def test_from_native_nil
    result = JV.from_native(nil)
    assert_instance_of JV::Null, result
  end

  # Spec test #36: Nested from_native
  def test_from_native_nested
    native = {
      "users" => [
        { "name" => "Alice", "age" => 30 }
      ],
      "active" => true,
      "count" => nil
    }

    result = JV.from_native(native)
    assert_instance_of JV::Object, result
    users = result.pairs["users"]
    assert_instance_of JV::Array, users
    alice = users.elements[0]
    assert_instance_of JV::Object, alice
    assert_equal "Alice", alice.pairs["name"].value
    assert_equal 30, alice.pairs["age"].value
    assert_instance_of JV::Boolean, result.pairs["active"]
    assert_instance_of JV::Null, result.pairs["count"]
  end

  # Spec test #37: Non-string key error
  def test_from_native_non_string_key_raises
    error = assert_raises(JV::Error) { JV.from_native({ 1 => "val" }) }
    assert_match(/keys must be strings/, error.message)
  end

  # Spec test #38: Non-JSON type error
  def test_from_native_symbol_raises
    assert_raises(JV::Error) { JV.from_native(:symbol) }
  end

  def test_from_native_proc_raises
    assert_raises(JV::Error) { JV.from_native(-> { 42 }) }
  end

  def test_from_native_object_raises
    assert_raises(JV::Error) { JV.from_native(Object.new) }
  end

  # Empty containers
  def test_from_native_empty_hash
    result = JV.from_native({})
    assert_instance_of JV::Object, result
    assert_equal({}, result.pairs)
  end

  def test_from_native_empty_array
    result = JV.from_native([])
    assert_instance_of JV::Array, result
    assert_equal [], result.elements
  end
end

# ================================================================
# Tests for parse and parse_native (convenience methods)
# ================================================================

class TestJsonValueParse < Minitest::Test
  JV = CodingAdventures::JsonValue

  # Spec test #39: parse returns JsonValue
  def test_parse_returns_json_value
    result = JV.parse('{"a": 1}')
    assert_instance_of JV::Object, result
    assert_equal 1, result.pairs["a"].value
  end

  # Spec test #40: parse_native returns native types
  def test_parse_native_returns_native
    result = JV.parse_native('{"a": 1}')
    assert_equal({ "a" => 1 }, result)
  end

  # Spec test #41: parse invalid JSON raises Error
  def test_parse_invalid_json_raises
    assert_raises(JV::Error) { JV.parse("not json") }
  end

  # Spec test #42: parse_native invalid JSON raises Error
  def test_parse_native_invalid_json_raises
    assert_raises(JV::Error) { JV.parse_native("{") }
  end

  # Parse various standalone values
  def test_parse_string
    result = JV.parse('"hello"')
    assert_instance_of JV::String, result
    assert_equal "hello", result.value
  end

  def test_parse_number
    assert_equal 42, JV.parse_native("42")
  end

  def test_parse_true
    assert_equal true, JV.parse_native("true")
  end

  def test_parse_false
    assert_equal false, JV.parse_native("false")
  end

  def test_parse_null
    assert_nil JV.parse_native("null")
  end

  def test_parse_array
    assert_equal [1, 2, 3], JV.parse_native("[1, 2, 3]")
  end

  # Parse complex JSON
  def test_parse_complex_json
    json = '{"name": "Alice", "scores": [95, 87, 92], "active": true, "address": null}'
    expected = {
      "name" => "Alice",
      "scores" => [95, 87, 92],
      "active" => true,
      "address" => nil
    }
    assert_equal expected, JV.parse_native(json)
  end
end

# ================================================================
# Round-trip Tests
# ================================================================

class TestJsonValueRoundTrip < Minitest::Test
  JV = CodingAdventures::JsonValue

  # Spec test #43: from_native -> to_native preserves value
  def test_round_trip_simple
    original = { "name" => "Alice", "age" => 30 }
    round_tripped = JV.to_native(JV.from_native(original))
    assert_equal original, round_tripped
  end

  # Spec test #44: Complex nested round-trip
  def test_round_trip_nested
    original = {
      "users" => [
        { "name" => "Alice", "scores" => [95, 87.5] },
        { "name" => "Bob", "active" => false }
      ],
      "meta" => { "version" => 1, "debug" => nil }
    }
    round_tripped = JV.to_native(JV.from_native(original))
    assert_equal original, round_tripped
  end

  # Round-trip every primitive type
  def test_round_trip_string
    assert_equal "hello", JV.to_native(JV.from_native("hello"))
  end

  def test_round_trip_integer
    assert_equal 42, JV.to_native(JV.from_native(42))
  end

  def test_round_trip_float
    assert_in_delta 3.14, JV.to_native(JV.from_native(3.14)), 0.001
  end

  def test_round_trip_true
    assert_equal true, JV.to_native(JV.from_native(true))
  end

  def test_round_trip_false
    assert_equal false, JV.to_native(JV.from_native(false))
  end

  def test_round_trip_nil
    assert_nil JV.to_native(JV.from_native(nil))
  end

  def test_round_trip_empty_hash
    assert_equal({}, JV.to_native(JV.from_native({})))
  end

  def test_round_trip_empty_array
    assert_equal [], JV.to_native(JV.from_native([]))
  end

  # Full pipeline round-trip: text -> parse_native
  def test_full_pipeline_object
    json = '{"a": 1, "b": [2, 3], "c": true}'
    result = JV.parse_native(json)
    assert_equal({ "a" => 1, "b" => [2, 3], "c" => true }, result)
  end
end

# ================================================================
# Tests for JsonValue Type Classes
# ================================================================

class TestJsonValueTypes < Minitest::Test
  JV = CodingAdventures::JsonValue

  # Number.integer? predicate
  def test_number_integer_predicate
    assert JV::Number.new(value: 0).integer?
    assert JV::Number.new(value: 42).integer?
    assert JV::Number.new(value: -17).integer?
    refute JV::Number.new(value: 3.14).integer?
    refute JV::Number.new(value: 0.0).integer?
  end

  # Data.define gives us structural equality
  def test_null_equality
    assert_equal JV::Null.new, JV::Null.new
  end

  def test_string_equality
    assert_equal JV::String.new(value: "hello"), JV::String.new(value: "hello")
    refute_equal JV::String.new(value: "hello"), JV::String.new(value: "world")
  end

  def test_number_equality
    assert_equal JV::Number.new(value: 42), JV::Number.new(value: 42)
    refute_equal JV::Number.new(value: 42), JV::Number.new(value: 43)
  end

  def test_boolean_equality
    assert_equal JV::Boolean.new(value: true), JV::Boolean.new(value: true)
    refute_equal JV::Boolean.new(value: true), JV::Boolean.new(value: false)
  end

  # Default values for Object and Array
  def test_object_default_pairs
    obj = JV::Object.new
    assert_equal({}, obj.pairs)
  end

  def test_array_default_elements
    arr = JV::Array.new
    assert_equal [], arr.elements
  end
end

# ================================================================
# Error Handling Edge Cases
# ================================================================

class TestJsonValueErrors < Minitest::Test
  JV = CodingAdventures::JsonValue

  # to_native with unknown type
  def test_to_native_unknown_type_raises
    assert_raises(JV::Error) { JV.to_native("not a json value") }
  end

  # Deeply nested non-JSON type inside a hash
  def test_from_native_nested_invalid_type
    assert_raises(JV::Error) do
      JV.from_native({ "a" => { "b" => :symbol } })
    end
  end

  # Non-JSON type inside an array
  def test_from_native_array_with_invalid_type
    assert_raises(JV::Error) do
      JV.from_native([1, 2, :symbol])
    end
  end
end
