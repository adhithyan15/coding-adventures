# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  add_filter "/test/"
  minimum_coverage line: 95
end

require "minitest/autorun"
require "coding_adventures_test_bootstrap"

# Shared helpers for the whole suite.
module Fx
  include CodingAdventures::TestBootstrap

  # Walks up from this file to the repository root and returns the COMMITTED
  # pbt-v1 fixture, never a copy.
  def self.fixture_path
    dir = File.expand_path(__dir__)
    loop do
      candidate = File.join(dir, "code/specs/fixtures/pbt-v1/cases.json")
      return candidate if File.file?(candidate)

      parent = File.dirname(dir)
      raise "could not find code/specs/fixtures/pbt-v1/cases.json above #{__dir__}" if parent == dir

      dir = parent
    end
  end

  def self.raw
    File.binread(fixture_path)
  end

  # A fresh, independent parse of the committed corpus (safe to mutate).
  def self.cases
    Pbt::Corpus.parse(raw)
  end

  # The repository root (the directory holding code/specs), found by walking up.
  def self.repo_root
    dir = File.expand_path(__dir__)
    dir = File.dirname(dir) until File.directory?(File.join(dir, "code/specs")) || dir == File.dirname(dir)
    dir
  end

  # The COMMITTED fuzz-v1 fixture, never a copy.
  def self.fuzz_raw
    File.binread(File.join(repo_root, "code/specs/fixtures/fuzz-v1/cases.json"))
  end

  def self.fuzz_cases
    Fuzz::Corpus.parse(fuzz_raw)
  end

  def self.hex(text) = [text].pack("H*")

  # The Ruby with which the tests run subprocesses (no SimpleCov in them).
  def self.lib_dir = File.expand_path("../lib", __dir__)
end

# Every test that touches the global sabotage hooks or the budget default
# restores them, so tests may run in any order.
class HookedTest < Minitest::Test
  include CodingAdventures::TestBootstrap

  def teardown
    Pbt::Hooks.reset
  end
end

# The fuzz sabotage hooks are global too.
class FuzzHookedTest < Minitest::Test
  include CodingAdventures::TestBootstrap

  def teardown
    Fuzz::Hooks.reset
  end
end
