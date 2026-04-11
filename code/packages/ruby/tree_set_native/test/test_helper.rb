# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 85
end

require "minitest/autorun"
require_relative "../lib/coding_adventures_tree_set_native"
