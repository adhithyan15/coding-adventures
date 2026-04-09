# frozen_string_literal: true

# ============================================================================
# test_helper.rb — shared setup for all Parrot REPL tests
# ============================================================================
#
# Every test file in test/ begins with `require "test_helper"`. This file is
# the single place to configure the test environment.
#
# ## What we do here
#
# 1. Require minitest/autorun — this wires Minitest into Ruby's at_exit hook
#    so tests run automatically when the file is loaded. Without this, you'd
#    have to call Minitest.run manually.
#
# 2. Prepend `lib/` to $LOAD_PATH — so `require "parrot"` in test files finds
#    this project's lib rather than any installed gem.
#
# ## Why not use `bundle exec`?
#
# `bundle exec` ensures the Gemfile's gem versions are active. When running
# via `rake test` (the BUILD script), bundler manages the load path. This
# $LOAD_PATH manipulation is a belt-and-suspenders fallback that keeps tests
# runnable even with `ruby -Itest test/test_parrot.rb`.

require "minitest/autorun"

$LOAD_PATH.unshift File.join(__dir__, "..", "lib")
