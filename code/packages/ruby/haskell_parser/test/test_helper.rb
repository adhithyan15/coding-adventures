# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 80
  add_filter "_grammar"
end

require "minitest/autorun"
require "coding_adventures_haskell_parser"
