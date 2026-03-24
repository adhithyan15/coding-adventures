# frozen_string_literal: true

# ================================================================
# SerializerConfig -- Configuration for JSON Pretty-Printing
# ================================================================
#
# When serializing JSON in "pretty" mode, you often want to control
# the output format. This config struct lets you customize:
#
# 1. **indent_size** -- How many spaces (or tabs) per nesting level.
#    Default: 2. Common alternatives: 4 (Python style), 1 tab.
#
# 2. **indent_char** -- What character to use for indentation.
#    Default: " " (space). Alternative: "\t" (tab).
#
# 3. **sort_keys** -- Whether to sort object keys alphabetically.
#    Default: false (preserve insertion order). Setting to true
#    gives deterministic output regardless of insertion order.
#
# 4. **trailing_newline** -- Whether to add a newline at the very
#    end of the output. Default: false. Useful when writing to files
#    (many text editors expect a trailing newline).
#
# Example:
#
#   config = SerializerConfig.new(indent_size: 4, sort_keys: true)
#   JsonSerializer.serialize_pretty(value, config: config)
#   # => "{\n    \"age\": 30,\n    \"name\": \"Alice\"\n}"
#
# ================================================================

module CodingAdventures
  module JsonSerializer
    SerializerConfig = Data.define(:indent_size, :indent_char, :sort_keys, :trailing_newline) do
      def initialize(indent_size: 2, indent_char: " ", sort_keys: false, trailing_newline: false)
        super(
          indent_size: indent_size,
          indent_char: indent_char,
          sort_keys: sort_keys,
          trailing_newline: trailing_newline
        )
      end

      # Build the indentation string for a given nesting depth.
      #
      # For depth=0, returns "" (no indentation at root level).
      # For depth=1, returns "  " (2 spaces with default config).
      # For depth=2, returns "    " (4 spaces), etc.
      #
      # @param depth [Integer] the current nesting depth
      # @return [::String] the indentation string
      def indent_for(depth)
        (indent_char * indent_size) * depth
      end
    end
  end
end
