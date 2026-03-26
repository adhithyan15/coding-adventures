# frozen_string_literal: true

# --------------------------------------------------------------------------
# test_helper.rb — Test setup with SimpleCov coverage tracking
# --------------------------------------------------------------------------
#
# This file is required by all test files. It configures:
# 1. SimpleCov for code coverage measurement
# 2. Minitest as the test framework
#
# SimpleCov must be started BEFORE any application code is loaded,
# otherwise it won't track coverage for files loaded before it starts.

require "simplecov"

SimpleCov.start do
  add_filter %r{_tokens\\.rb$}
  add_filter %r{_grammar\\.rb$}
  # Track coverage for our Ruby entry point and version file
  add_filter "/test/"
  add_filter "/ext/"

  # Set minimum coverage threshold
  minimum_coverage 80
end

require "minitest/autorun"
require_relative "../lib/coding_adventures_commonmark_native"
