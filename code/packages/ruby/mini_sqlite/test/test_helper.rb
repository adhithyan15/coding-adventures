# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 60
end

require "minitest/autorun"
require "coding_adventures_mini_sqlite"
