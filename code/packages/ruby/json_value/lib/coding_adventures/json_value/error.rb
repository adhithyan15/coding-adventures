# frozen_string_literal: true

# ================================================================
# JsonValue::Error -- Exception for JSON Value Operations
# ================================================================
#
# This error is raised when something goes wrong during:
#
# 1. AST conversion (from_ast) -- if the AST has an unexpected
#    structure, perhaps from a corrupted or non-JSON parser output.
#
# 2. Native conversion (from_native) -- if the Ruby value contains
#    types that have no JSON equivalent, like a Symbol, a Proc, or
#    a custom class instance.
#
# 3. Parsing (parse, parse_native) -- if the JSON text is malformed.
#    The underlying parser error is wrapped in a JsonValue::Error
#    so callers don't need to know about the parser layer.
#
# ================================================================

module CodingAdventures
  module JsonValue
    class Error < StandardError; end
  end
end
