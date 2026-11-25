# frozen_string_literal: true

require_relative "test_helper"

class TestSafe < HookedTest
  S = CodingAdventures::TestBootstrap::Safe

  def test_recoverable_classification
    custom = Class.new(Exception) # rubocop:disable Lint/InheritException
    [StandardError.new, SystemStackError.new, NotImplementedError.new, ScriptError.new, custom.new].each do |e|
      assert S.recoverable?(e), e.class.to_s
    end
    [NoMemoryError.new, Interrupt.new, SignalException.new("TERM"), SystemExit.new].each do |e|
      refute S.recoverable?(e), e.class.to_s
    end
  end

  def test_safe_string_is_total_and_bounded
    assert_equal "RuntimeError: boom", S.safe_string(RuntimeError.new("boom"))
    assert_operator S.safe_string(RuntimeError.new("x" * 1000)).size, :<=, 200
    refute_includes S.safe_string(RuntimeError.new("a\nb\e")), "\n"
    hostile = StandardError.new
    def hostile.message = raise("no message for you")
    assert_equal "StandardError: ", S.safe_string(hostile)
    interrupting = StandardError.new
    def interrupting.message = raise(Interrupt)
    assert_raises(Interrupt) { S.safe_string(interrupting) }
  end

  def test_class_name_is_total
    assert_equal "ArgumentError", S.class_name(ArgumentError.new)
    klass = Class.new(StandardError) { def self.to_s = raise("no name") }
    assert_equal "Exception", S.class_name(klass.new)
  end

  def test_version_and_entry_point
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::TestBootstrap::VERSION)
  end
end
