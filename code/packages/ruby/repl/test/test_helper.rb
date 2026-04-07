# frozen_string_literal: true

# ============================================================================
# Test Helper — SimpleCov + Minitest Setup
# ============================================================================
#
# SimpleCov must be required and started BEFORE any application code is
# loaded, otherwise it cannot track which lines were executed during tests.
# The require order here is intentional and must not be changed.

require "simplecov"

SimpleCov.start do
  # Track coverage for the lib directory only (not test files themselves).
  add_filter "/test/"

  # Project standard: >80% coverage. This REPL framework should exceed that
  # comfortably since the code is small and all paths are tested.
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_repl"
