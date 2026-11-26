# frozen_string_literal: true

require "simplecov"

SimpleCov.start do
  source_root = File.expand_path("../lib", __dir__)
  add_filter do |source|
    !source.filename.start_with?(source_root) || source.filename.end_with?("/version.rb")
  end
  track_files "lib/**/*.rb"
  minimum_coverage 80
end

require "minitest/autorun"

# Require dependencies before package entry points, as Ruby resolves modules
# in load order. The test bootstrap stays a development-only helper.
require "coding_adventures_test_bootstrap"
require "coding_adventures_microbenchmark"
