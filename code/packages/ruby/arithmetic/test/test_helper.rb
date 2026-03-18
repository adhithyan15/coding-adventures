# frozen_string_literal: true

# ============================================================================
# Test Helper — SimpleCov + Minitest Setup
# ============================================================================
#
# SimpleCov must be required and started BEFORE any application code is
# loaded, otherwise it cannot track which lines were executed during tests.

require "simplecov"

SimpleCov.start do
  # Track coverage for the lib directory only (not test files themselves).
  add_filter "/test/"

  # Set a minimum coverage threshold — fail the build if coverage drops
  # below 80% (our project standard is >80%).
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_arithmetic"
