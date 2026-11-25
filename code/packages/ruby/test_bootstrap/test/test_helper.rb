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
end

# Every test that touches the global sabotage hooks or the budget default
# restores them, so tests may run in any order.
class HookedTest < Minitest::Test
  include CodingAdventures::TestBootstrap

  def teardown
    Pbt::Hooks.reset
  end
end
