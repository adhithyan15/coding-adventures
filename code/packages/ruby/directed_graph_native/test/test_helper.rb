# frozen_string_literal: true

# --------------------------------------------------------------------------
# test_helper.rb -- Test configuration and shared setup
# --------------------------------------------------------------------------
#
# This file is required by every test file. It configures:
#
# 1. SimpleCov for code coverage measurement (must come first!)
# 2. Minitest as the test framework
# 3. The native extension under test
#
# SimpleCov must be started before any application code is loaded,
# otherwise it can't track which lines are executed.
# --------------------------------------------------------------------------

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  minimum_coverage 95
end

require "minitest/autorun"
require "coding_adventures_directed_graph_native"
