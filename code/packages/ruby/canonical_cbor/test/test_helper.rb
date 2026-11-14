# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  add_filter "/test/"
  minimum_coverage line: 95, branch: 95
end

require "minitest/autorun"
require "coding_adventures_canonical_cbor"

CanonicalCbor = CodingAdventures::CanonicalCbor
