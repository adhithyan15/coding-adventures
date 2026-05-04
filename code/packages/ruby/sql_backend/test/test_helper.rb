# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 75
end

require "minitest/autorun"
require "coding_adventures/sql_backend"
