# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  add_filter "/test/"
  minimum_coverage line: 95
end

require "minitest/autorun"
require "coding_adventures_test_bootstrap"
