# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  minimum_coverage 80
  add_filter "/test/"
  add_filter "/vendor/"
end

require "minitest/autorun"
require "coding_adventures_register_vm"

# Pull all types and helpers into the test namespace so tests can write
# CodeObject.new(...) directly without the full module path.
include CodingAdventures::RegisterVM
