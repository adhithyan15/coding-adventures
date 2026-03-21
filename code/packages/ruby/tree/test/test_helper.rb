# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 95
end

require "minitest/autorun"
require "coding_adventures_tree"
