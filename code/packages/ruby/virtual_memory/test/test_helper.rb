# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_virtual_memory"
