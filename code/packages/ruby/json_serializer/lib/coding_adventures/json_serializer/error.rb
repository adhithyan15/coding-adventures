# frozen_string_literal: true

# ================================================================
# JsonSerializer::Error -- Exception for Serialization Errors
# ================================================================
#
# Raised when serialization encounters a value that cannot be
# represented in JSON. The two main cases are:
#
# 1. **Infinity/NaN** -- IEEE 754 special float values have no
#    JSON representation. RFC 8259 says JSON numbers must be finite.
#
# 2. **Unknown types** -- If a value is not a recognized JsonValue
#    subclass, we can't serialize it.
#
# ================================================================

module CodingAdventures
  module JsonSerializer
    class Error < StandardError; end
  end
end
