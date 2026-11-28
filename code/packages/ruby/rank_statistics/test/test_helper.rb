# frozen_string_literal: true

require "simplecov"

SimpleCov.start do
  source_root = File.expand_path("../lib", __dir__)
  add_filter do |source|
    !source.filename.start_with?(source_root) || source.filename.end_with?("/version.rb")
  end
  track_files "lib/**/*.rb"
  minimum_coverage 95
end

require "minitest/autorun"

# Test helpers are loaded before the package under measurement, keeping the
# PBT engine outside the rank-statistics dependency graph and coverage target.
require "coding_adventures_test_bootstrap"
require "coding_adventures_rank_statistics"
