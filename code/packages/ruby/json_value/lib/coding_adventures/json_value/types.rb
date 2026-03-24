# frozen_string_literal: true

# ================================================================
# JsonValue Type Classes -- Typed Representations of JSON Values
# ================================================================
#
# JSON defines exactly six value types: object, array, string,
# number, boolean, and null. This module provides a Ruby class
# for each one.
#
# Why not just use native Ruby types directly?
#
# Two reasons:
#
# 1. **Type safety.** A JsonValue::String is unambiguously a JSON
#    string. A plain Ruby String could be anything -- a file path,
#    a SQL query, a person's name. The wrapper makes the provenance
#    explicit.
#
# 2. **Round-trip fidelity.** By preserving the original JSON type
#    (e.g., integer vs. float for numbers), we can serialize back
#    to JSON and get the same output. If we collapsed everything to
#    native types immediately, we'd lose information like "was this
#    number an integer or a float in the original JSON?"
#
# Each class uses Data.define for immutability. Once created, a
# JsonValue never changes -- just like the JSON text it came from.
#
# ================================================================
# Type Hierarchy
# ================================================================
#
#   JsonValue (conceptual base -- not a Ruby class, just a namespace)
#     |-- Object   -- {"key": value, ...}  ordered key-value pairs
#     |-- Array    -- [value, ...]         ordered sequence of values
#     |-- String   -- "hello"              text value
#     |-- Number   -- 42 or 3.14           numeric value (int or float)
#     |-- Boolean  -- true or false        logical value
#     |-- Null     -- null                 the absence of a value
#
# ================================================================

module CodingAdventures
  module JsonValue
    # ============================================================
    # JsonValue::Object -- An ordered collection of key-value pairs
    # ============================================================
    #
    # JSON objects look like: {"name": "Alice", "age": 30}
    #
    # We store pairs in a Ruby Hash, which preserves insertion order
    # since Ruby 1.9. Keys are always Ruby strings. Values are
    # JsonValue instances.
    #
    # Why "ordered"? RFC 8259 says JSON objects are "unordered
    # collections," but in practice, insertion order matters for:
    # - Human readability (config files, API responses)
    # - Round-trip fidelity (parse then serialize = same output)
    # - Deterministic test output
    #
    # @param pairs [Hash<::String, JsonValue>] the key-value pairs
    # ============================================================
    Object = Data.define(:pairs) do
      def initialize(pairs: {})
        super(pairs: pairs)
      end
    end

    # ============================================================
    # JsonValue::Array -- An ordered sequence of values
    # ============================================================
    #
    # JSON arrays look like: [1, "two", true, null]
    #
    # Elements can be any JsonValue type, including nested objects
    # and arrays. This is what makes JSON a recursive data format.
    #
    # @param elements [::Array<JsonValue>] the array elements
    # ============================================================
    Array = Data.define(:elements) do
      def initialize(elements: [])
        super(elements: elements)
      end
    end

    # ============================================================
    # JsonValue::String -- A text value
    # ============================================================
    #
    # JSON strings look like: "hello world"
    #
    # The value stored here is the *unescaped* content. That is,
    # if the JSON source had "hello\nworld", the value here is
    # the two-line string with an actual newline character. The
    # lexer already handled the unescaping.
    #
    # IMPORTANT: This class is namespaced under CodingAdventures::
    # JsonValue::String. It does NOT shadow Ruby's built-in String
    # class. Within this module, use ::String to refer to Ruby's
    # String if needed.
    #
    # @param value [::String] the string content
    # ============================================================
    String = Data.define(:value)

    # ============================================================
    # JsonValue::Number -- A numeric value (integer or float)
    # ============================================================
    #
    # JSON numbers can be integers (42, -17, 0) or floating-point
    # (3.14, 1e10, -2.5e-3). JSON itself doesn't distinguish, but
    # we do, because:
    #
    # - Users expect 42 to be an Integer, not 42.0
    # - Round-tripping: parse("42") should serialize back to "42",
    #   not "42.0"
    #
    # The integer? predicate tells you which representation is used.
    #
    # @param value [::Integer, ::Float] the numeric value
    # ============================================================
    Number = Data.define(:value) do
      # Is this number stored as an integer?
      #
      # Rule: if the original JSON had no decimal point and no
      # exponent, it's an integer. Otherwise it's a float.
      #
      # Examples:
      #   Number.new(value: 42).integer?     #=> true
      #   Number.new(value: 3.14).integer?   #=> false
      #   Number.new(value: 1e10).integer?   #=> false
      def integer?
        value.is_a?(::Integer)
      end
    end

    # ============================================================
    # JsonValue::Boolean -- A logical true/false value
    # ============================================================
    #
    # JSON booleans are either true or false. Simple.
    #
    # @param value [TrueClass, FalseClass] the boolean value
    # ============================================================
    Boolean = Data.define(:value)

    # ============================================================
    # JsonValue::Null -- The absence of a value
    # ============================================================
    #
    # JSON null represents "nothing" or "no value." It maps to
    # Ruby's nil.
    #
    # Unlike the other types, Null has no fields. It's a singleton
    # concept, though we don't enforce singleton-ness -- two Null
    # instances are equal because Data.define gives us structural
    # equality.
    # ============================================================
    Null = Data.define do
      def initialize
        super()
      end
    end
  end
end
