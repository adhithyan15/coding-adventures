# frozen_string_literal: true

# test_helper.rb — sets up SimpleCov BEFORE requiring Minitest so that
# SimpleCov's at_exit hook is registered first.  Ruby's at_exit hooks run
# in LIFO order (last registered, first called), so registering SimpleCov
# first means Minitest finishes its own at_exit (running all tests) before
# SimpleCov's at_exit fires and writes the coverage report.
#
# Ordering matters: if you require "minitest/autorun" before SimpleCov.start,
# SimpleCov's at_exit fires before tests run and you get 0% coverage.

require "simplecov"

SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_rng"
