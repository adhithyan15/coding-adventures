# frozen_string_literal: true

# Tests for the JIT compiler shell gem.
#
# Since this is a shell gem with no implementation yet, these tests
# verify the package structure: that the module loads and the VERSION
# constant is defined and well-formed.

require "test_helper"

module CodingAdventures
  module JitCompiler
    class TestJitCompilerModule < Minitest::Test
      # The module should load without error.
      def test_module_is_defined
        assert defined?(CodingAdventures::JitCompiler)
      end

      # VERSION must be a semver string (e.g. "0.1.0").
      def test_version_is_valid_semver
        assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::JitCompiler::VERSION)
      end

      # VERSION should be frozen (immutable).
      def test_version_is_frozen
        assert CodingAdventures::JitCompiler::VERSION.frozen?
      end
    end
  end
end
