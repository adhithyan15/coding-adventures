# frozen_string_literal: true

# ================================================================
# JSON Serializer -- JsonValue/Native Types -> JSON Text
# ================================================================
#
# This module converts JsonValue objects (or native Ruby types) into
# JSON text strings. Two output modes are supported:
#
# 1. **Compact** -- No unnecessary whitespace. Smallest possible
#    output. Ideal for network transmission, storage, or machine
#    consumption.
#
#    serialize(JsonObject({"a": JsonNumber(1)}))
#    => '{"a":1}'
#
# 2. **Pretty** -- Human-readable with configurable indentation.
#    Ideal for config files, debugging, or display.
#
#    serialize_pretty(JsonObject({"a": JsonNumber(1)}))
#    => "{\n  \"a\": 1\n}"
#
# ================================================================
# Algorithm: serialize(value)
# ================================================================
#
# The serialization is a recursive dispatch on the JsonValue type:
#
#   Null     -> "null"
#   Boolean  -> "true" or "false"
#   Number   -> integer or float string representation
#   String   -> quoted and escaped per RFC 8259
#   Array    -> "[" + comma-separated elements + "]"
#   Object   -> "{" + comma-separated key:value pairs + "}"
#
# ================================================================
# String Escaping (RFC 8259)
# ================================================================
#
# These characters MUST be escaped in JSON strings:
#
#   Character        Escape     Why
#   ---------        ------     ---
#   " (quote)        \"         It's the string delimiter
#   \ (backslash)    \\         It's the escape character
#   Backspace        \b         Control character (U+0008)
#   Form feed        \f         Control character (U+000C)
#   Newline          \n         Control character (U+000A)
#   Carriage return  \r         Control character (U+000D)
#   Tab              \t         Control character (U+0009)
#   U+0000-U+001F    \uXXXX    All other control characters
#
# Forward slash (/) is NOT escaped -- RFC 8259 allows but does
# not require it. We follow the common convention of not escaping.
#
# ================================================================

module CodingAdventures
  module JsonSerializer
    JV = CodingAdventures::JsonValue

    # ----------------------------------------------------------------
    # serialize: JsonValue -> compact JSON text
    # ----------------------------------------------------------------
    #
    # Produces the smallest possible JSON text with no unnecessary
    # whitespace. Objects have no space after colons, arrays have
    # no space after commas.
    #
    # @param value [JsonValue type] a JsonValue instance
    # @return [::String] compact JSON text
    # @raise [Error] if value contains non-serializable values
    # ----------------------------------------------------------------
    def self.serialize(value)
      case value
      when JV::Null
        "null"
      when JV::Boolean
        value.value ? "true" : "false"
      when JV::Number
        serialize_number(value.value)
      when JV::String
        serialize_string(value.value)
      when JV::Array
        serialize_array_compact(value)
      when JV::Object
        serialize_object_compact(value)
      else
        raise Error, "Cannot serialize #{value.class}"
      end
    end

    # ----------------------------------------------------------------
    # serialize_pretty: JsonValue -> pretty-printed JSON text
    # ----------------------------------------------------------------
    #
    # Produces human-readable JSON with indentation and newlines.
    # Uses the provided config for formatting options, or defaults
    # (2-space indent, no key sorting, no trailing newline).
    #
    # @param value [JsonValue type] a JsonValue instance
    # @param config [SerializerConfig, nil] formatting options
    # @return [::String] pretty-printed JSON text
    # ----------------------------------------------------------------
    def self.serialize_pretty(value, config: nil)
      config ||= SerializerConfig.new
      result = serialize_pretty_recursive(value, config, 0)
      result += "\n" if config.trailing_newline
      result
    end

    # ----------------------------------------------------------------
    # stringify: native Ruby types -> compact JSON text
    # ----------------------------------------------------------------
    #
    # Convenience method that converts native Ruby types to compact
    # JSON text. Equivalent to: serialize(from_native(value))
    #
    # @param value [Hash, Array, ::String, Integer, Float, true, false, nil]
    # @return [::String] compact JSON text
    # ----------------------------------------------------------------
    def self.stringify(value)
      serialize(JV.from_native(value))
    end

    # ----------------------------------------------------------------
    # stringify_pretty: native Ruby types -> pretty JSON text
    # ----------------------------------------------------------------
    #
    # Convenience method that converts native Ruby types to pretty-
    # printed JSON text. Equivalent to:
    #   serialize_pretty(from_native(value), config: config)
    #
    # @param value [Hash, Array, ::String, Integer, Float, true, false, nil]
    # @param config [SerializerConfig, nil] formatting options
    # @return [::String] pretty-printed JSON text
    # ----------------------------------------------------------------
    def self.stringify_pretty(value, config: nil)
      serialize_pretty(JV.from_native(value), config: config)
    end

    # ==============================================================
    # Private Helpers
    # ==============================================================

    # Serialize a number to its JSON string representation.
    #
    # Integer values are rendered without a decimal point: 42 -> "42"
    # Float values are rendered with a decimal point: 3.14 -> "3.14"
    #
    # IEEE 754 special values (Infinity, NaN) have no JSON
    # representation and raise an error.
    def self.serialize_number(num)
      if num.is_a?(::Float)
        raise Error, "Cannot serialize Infinity" if num.infinite?
        raise Error, "Cannot serialize NaN" if num.nan?

        # Ruby's Float#to_s produces reasonable output:
        # 3.14 -> "3.14", 1.0 -> "1.0"
        num.to_s
      else
        num.to_s
      end
    end

    # Serialize a string with proper JSON escaping.
    #
    # We wrap the string in double quotes and escape all characters
    # that RFC 8259 requires to be escaped. See the truth table at
    # the top of this file for the complete list.
    def self.serialize_string(str)
      escaped = +""
      str.each_char do |ch|
        case ch
        when '"'  then escaped << '\\"'
        when '\\' then escaped << '\\\\'
        when "\b" then escaped << '\\b'
        when "\f" then escaped << '\\f'
        when "\n" then escaped << '\\n'
        when "\r" then escaped << '\\r'
        when "\t" then escaped << '\\t'
        else
          # Escape control characters (U+0000 to U+001F) that aren't
          # covered by the named escapes above.
          if ch.ord < 0x20
            escaped << format('\\u%04x', ch.ord)
          else
            escaped << ch
          end
        end
      end
      "\"#{escaped}\""
    end

    # Serialize an array in compact mode (no whitespace).
    #
    # Empty arrays: "[]"
    # Non-empty: "[1,2,3]" (no spaces after commas)
    def self.serialize_array_compact(arr)
      return "[]" if arr.elements.empty?

      parts = arr.elements.map { |elem| serialize(elem) }
      "[#{parts.join(",")}]"
    end

    # Serialize an object in compact mode (no whitespace).
    #
    # Empty objects: "{}"
    # Non-empty: '{"a":1,"b":2}' (no spaces after colons or commas)
    def self.serialize_object_compact(obj)
      return "{}" if obj.pairs.empty?

      parts = obj.pairs.map do |key, val|
        "#{serialize_string(key)}:#{serialize(val)}"
      end
      "{#{parts.join(",")}}"
    end

    # Recursive pretty-printing with indentation tracking.
    #
    # Primitive values (null, boolean, number, string) are rendered
    # the same as compact mode -- they have no internal structure
    # to indent.
    #
    # Arrays and objects get newlines between elements, with each
    # element indented one level deeper than its container.
    def self.serialize_pretty_recursive(value, config, depth)
      case value
      when JV::Null
        "null"
      when JV::Boolean
        value.value ? "true" : "false"
      when JV::Number
        serialize_number(value.value)
      when JV::String
        serialize_string(value.value)
      when JV::Array
        serialize_array_pretty(value, config, depth)
      when JV::Object
        serialize_object_pretty(value, config, depth)
      else
        raise Error, "Cannot serialize #{value.class}"
      end
    end

    # Pretty-print an array with one element per line.
    #
    # Empty arrays: "[]" (no newlines)
    # Non-empty:
    #   [
    #     1,
    #     2,
    #     3
    #   ]
    def self.serialize_array_pretty(arr, config, depth)
      return "[]" if arr.elements.empty?

      next_indent = config.indent_for(depth + 1)
      current_indent = config.indent_for(depth)

      lines = arr.elements.map do |elem|
        "#{next_indent}#{serialize_pretty_recursive(elem, config, depth + 1)}"
      end

      "[\n#{lines.join(",\n")}\n#{current_indent}]"
    end

    # Pretty-print an object with one key-value pair per line.
    #
    # Empty objects: "{}" (no newlines)
    # Non-empty:
    #   {
    #     "name": "Alice",
    #     "age": 30
    #   }
    #
    # If sort_keys is enabled, keys are sorted alphabetically.
    def self.serialize_object_pretty(obj, config, depth)
      return "{}" if obj.pairs.empty?

      next_indent = config.indent_for(depth + 1)
      current_indent = config.indent_for(depth)

      keys = config.sort_keys ? obj.pairs.keys.sort : obj.pairs.keys

      lines = keys.map do |key|
        val_str = serialize_pretty_recursive(obj.pairs[key], config, depth + 1)
        "#{next_indent}#{serialize_string(key)}: #{val_str}"
      end

      "{\n#{lines.join(",\n")}\n#{current_indent}}"
    end

    private_class_method :serialize_number, :serialize_string,
                         :serialize_array_compact, :serialize_object_compact,
                         :serialize_pretty_recursive, :serialize_array_pretty,
                         :serialize_object_pretty
  end
end
