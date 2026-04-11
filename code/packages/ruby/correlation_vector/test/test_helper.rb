# frozen_string_literal: true

# ================================================================
# Test Helper -- Setup for all Correlation Vector tests
# ================================================================
#
# SimpleCov runs FIRST, before any application code is loaded.
# This is critical: if SimpleCov starts after the code is loaded,
# it cannot instrument those files for coverage tracking.
#
# The minimum_coverage threshold of 95% enforces our repo standard
# of "well above 80% coverage" for library packages.
# ================================================================

require "simplecov"
SimpleCov.start do
  minimum_coverage 95
  add_filter "/test/"
end

require "minitest/autorun"
require "coding_adventures_correlation_vector"
