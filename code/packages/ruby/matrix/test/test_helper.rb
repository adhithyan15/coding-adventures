# frozen_string_literal: true

# Coverage starts before loading the library: declarations as well as method
# bodies count. Only library files enter the 95% floor, never the tests.
require "simplecov"
SimpleCov.start do
  root File.expand_path("..", __dir__)
  track_files "lib/**/*.rb"
  add_filter "/test/"
  add_filter "/scripts/"
  minimum_coverage 95
end

require "minitest/autorun"
require "matrix_ml"
