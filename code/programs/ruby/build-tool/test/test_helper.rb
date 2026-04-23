# frozen_string_literal: true

# test_helper.rb -- Shared test setup for the build tool test suite
# =================================================================
#
# This file is required by every test file. It configures SimpleCov for
# coverage measurement, loads Minitest, and provides helper methods and
# constants used across multiple test files.

# SimpleCov must be started BEFORE requiring any application code, otherwise
# the code loaded before SimpleCov starts won't be tracked. This is a common
# gotcha -- the Python equivalent (coverage.py) has the same requirement.
require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "pathname"
require "tmpdir"
require "fileutils"
require "set"
require "json"

# Load all build tool modules.
require_relative "../lib/build_tool/discovery"
require_relative "../lib/build_tool/resolver"
require_relative "../lib/build_tool/glob_match"
require_relative "../lib/build_tool/hasher"
require_relative "../lib/build_tool/cache"
require_relative "../lib/build_tool/executor"
require_relative "../lib/build_tool/reporter"
require_relative "../lib/build_tool/starlark_evaluator"
require_relative "../lib/build_tool/git_diff"
require_relative "../lib/build_tool/ci_workflow"
require_relative "../lib/build_tool/plan"
require_relative "../lib/build_tool/validator"

module TestHelper
  # FIXTURES_DIR points to the test/fixtures/ directory. We use Pathname
  # throughout (not strings) to match the application code's convention.
  FIXTURES_DIR = Pathname(__dir__) / "fixtures"

  # simple_fixture -- Path to the simple fixture (one package, no deps).
  def simple_fixture
    FIXTURES_DIR / "simple"
  end

  # diamond_fixture -- Path to the diamond fixture (4 packages, diamond deps).
  def diamond_fixture
    FIXTURES_DIR / "diamond"
  end

  # create_temp_dir -- Create a temporary directory that is cleaned up after test.
  #
  # Returns a Pathname to the temp dir. The caller should clean it up in
  # teardown, or use Ruby's Dir.mktmpdir with a block.
  def create_temp_dir
    Pathname(Dir.mktmpdir("build_tool_test"))
  end

  # write_file -- Helper to create a file with given content in a temp dir.
  #
  # Creates intermediate directories as needed.
  #
  # @param path [Pathname] The file path.
  # @param content [String] The file content.
  def write_file(path, content)
    path.dirname.mkpath
    path.write(content)
  end
end
