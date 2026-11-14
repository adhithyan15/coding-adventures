# frozen_string_literal: true

require_relative "coding_adventures/canonical_cbor/version"
require_relative "coding_adventures/canonical_cbor/constants"
require_relative "coding_adventures/canonical_cbor/messages"
require_relative "coding_adventures/canonical_cbor/types"
require_relative "coding_adventures/canonical_cbor/codec"

module CodingAdventures
  module CanonicalCbor
    def self.encode_checked(value)
      Codec.encode_checked(value)
    end

    def self.encode_into_checked(value, dst)
      Codec.encode_into_checked(value, dst)
    end

    def self.encode(value)
      Codec.encode(value)
    end

    def self.encode_into(value, dst)
      Codec.encode_into(value, dst)
    end

    def self.decode(data)
      Codec.decode(data)
    end

    def self.decode_map_spans(data)
      Codec.decode_map_spans(data)
    end
  end
end
