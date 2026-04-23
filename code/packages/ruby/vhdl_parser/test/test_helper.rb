# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 80
  # Generated compiled grammar data files are excluded from coverage.
  add_filter { |source_file| source_file.filename.match?(%r{/_grammar(?:_\d+)?\.rb$}) }
end

require "minitest/autorun"
require "coding_adventures_vhdl_parser"
