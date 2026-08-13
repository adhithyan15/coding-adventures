# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage line: 80, branch: 80
  add_filter "/test/"
end

require "minitest/autorun"
require "coding_adventures_zip"
