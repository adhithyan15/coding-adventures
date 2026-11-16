# frozen_string_literal: true

module CodingAdventures
  module CanonicalCbor
    module Messages
      REDACTED_PLACEHOLDER = "<redacted>"

      ERROR_MESSAGES = {
        unexpected_eof: "canonical-cbor: unexpected end of input",
        trailing_bytes: "canonical-cbor: trailing bytes after decoded item",
        reserved: "canonical-cbor: reserved additional-info value",
        indefinite: "canonical-cbor: indefinite item rejected",
        non_minimal_integer: "canonical-cbor: argument is not in smallest form",
        invalid_utf8: "canonical-cbor: text is not valid UTF-8",
        non_canonical_map_order: "canonical-cbor: map key order is not canonical",
        unsupported_simple: "canonical-cbor: unsupported simple value",
        float_not_supported: "canonical-cbor: floats are not supported",
        too_deep: "canonical-cbor: decoded nesting is too deep",
        length_too_large: "canonical-cbor: declared length is too large",
        duplicate_map_key: "canonical-cbor: duplicate canonical map key",
        encode_too_deep: "canonical-cbor: encoded nesting is too deep",
        encode_too_large: "canonical-cbor: encoded item is too large"
      }.freeze

      def self.message_for(id)
        ERROR_MESSAGES.fetch(id)
      end
    end
  end
end
