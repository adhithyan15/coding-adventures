# frozen_string_literal: true

require_relative "test_helper"

# Regression tests for the LOW findings of the VER05h security review.
class TestSecurityLow < HookedTest
  G = Pbt::Gens
  R = Pbt::Runner

  def opts(ci) = Pbt::Options.new(ci: ci)

  def hostile_class(name_text)
    klass = Class.new(StandardError)
    klass.define_singleton_method(:to_s) { raise name_text.nil? ? "boom" : name_text }
    klass.define_singleton_method(:name) { raise "boom" }
    klass
  end

  # 1. a class whose own to_s / name raise still yields a verdict, in both modes
  def test_a_hostile_class_to_s_still_yields_a_verdict
    klass = hostile_class(nil)
    %i[off on].each do |mode|
      v = R.check(1, G.u8, opts(mode)) { |_x| raise klass, "m" }
      assert_equal Pbt::FailureIds::PANIC, v.failure_id, mode.to_s
      g = R.check(1, Pbt::Gen.new(->(_r) { raise klass, "m" }), opts(mode)) { true }
      assert_equal Pbt::FailureIds::EXHAUSTED, g.failure_id
    end
    assert_equal "Exception", Safe.class_name(klass.new)
  end

  # 1b. a class name with control characters is escaped and clipped
  def test_control_characters_in_a_class_name_are_escaped
    klass = Class.new(StandardError)
    Object.const_set(:HostileNamed, klass) unless Object.const_defined?(:HostileNamed)
    klass.define_singleton_method(:to_s) { "A\n::error::x\e[31m" } # must NOT be consulted
    assert_equal "HostileNamed", Safe.class_name(klass.new)

    # a legitimately named class containing control characters (const names cannot,
    # but a #name from Module.instance_method is what we read, so check sanitize directly)
    out = Safe.sanitize("A\n::error::x\e[31m#{"y" * 500}")
    refute_match(/[\x00-\x1f\x7f]/, out)
    assert_operator out.size, :<=, 200 + 20
    %i[off on].each do |mode|
      v = R.check(1, G.u8, opts(mode)) { |_x| raise klass, "m\n::error::z" }
      refute_includes v.message, "\n"
      refute_includes v.message, "\e"
    end
  end

  # 2. one shared visit allowance across the whole size walk
  def test_size_of_visits_are_bounded_across_elements
    visits = 0
    counting = Class.new do
      include Enumerable

      define_method(:each) do |&blk|
        100_000.times do
          visits += 1
          blk.call(1)
        end
      end
    end
    array = Array.new(10_000) { counting.new }
    R.size_of(array)
    assert_operator visits, :<=, Pbt::Runner::WALK_CAP + 1

    visits = 0
    R.size_of(Array.new(10_000) { 1..1_000_000 })
    sets = Array.new(10_000) { Set.new(1..50) }
    assert_operator R.size_of(sets), :<=, 10_000 + 10_000 + 1 # lengths are O(1); no per-element enumeration
  end

  # 3. a hostile fixture id is escaped in every problem line
  def test_hostile_case_ids_are_escaped_in_problems
    cases = Fx.cases
    cases[0]["id"] = "evil\n::error::x\e[31m"
    cases[0]["op"] = "mystery"
    problems = Pbt::Corpus.evaluate(cases).problems
    refute_empty problems
    problems.each do |p|
      refute_includes p, "\n"
      refute_includes p, "\e"
    end
    assert(problems.any? { |p| p.start_with?("evil\\n::error::x\\u001b[31m: unknown op") })

    assert_equal '"a\\nb\\u001bc"', Tfv.q("a\nb\ec")
  end

  # 4. the ambient scope is only installed inside the guarded region
  def test_an_exception_while_installing_the_ambient_scope_leaves_nothing_behind
    verbose = $VERBOSE
    $VERBOSE = nil # redefining Fiber.[]= below is deliberate
    original = Fiber.method(:[]=)
    calls = 0
    Fiber.singleton_class.send(:define_method, :[]=) do |key, value|
      calls += 1
      raise "install boom" if calls == 1 && value.is_a?(Pbt::ShrinkBudget)

      original.call(key, value)
    end
    begin
      assert_raises(RuntimeError) { Pbt::ShrinkBudget.enter_child(5) { flunk "must not run" } }
    ensure
      Fiber.singleton_class.send(:define_method, :[]=, original)
      $VERBOSE = verbose
    end
    assert_nil Pbt::ShrinkBudget.current
  end
end
