# frozen_string_literal: true

require "timeout"
require_relative "test_helper"

class TestRunner < HookedTest
  G = Pbt::Gens
  R = Pbt::Runner

  def opts(**kw) = Pbt::Options.new(ci: :off, **kw)

  def test_a_holding_property_passes
    v = R.check(1, G.integer(0, 100), opts(num_tests: 50)) { |x| x <= 100 }
    assert v.passed
    refute_predicate v, :failed?
    assert_equal 50, v.cases_run
    assert_nil v.failure_id
  end

  def test_a_falsified_property_shrinks_to_the_minimal_witness
    v = R.check(42, G.integer(0, 1000), opts) { |x| x <= 100 }
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
    assert_equal 101, v.minimal
    assert_operator v.counterexample, :>, 100
    assert_equal 42, v.seed
    assert_predicate v, :failed?
    assert_match(/seed=42/, v.message)
    assert_match(/minimal=101/, v.message)
  end

  def test_replay_from_the_seed_gives_the_identical_verdict
    a = R.check(7, G.list(G.u8, 0, 20), opts) { |l| l.sum < 200 }
    b = R.check(7, G.list(G.u8, 0, 20), opts) { |l| l.sum < 200 }
    assert_equal a, b
    refute_equal a.counterexample, R.check(8, G.list(G.u8, 0, 20), opts) { |l| l.sum < 200 }.counterexample
  end

  def test_nil_and_false_falsify_truthy_holds
    assert R.check(1, G.u8, opts) { |_x| 1 }.passed
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, R.check(1, G.u8, opts) { |_x| nil }.failure_id
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, R.check(1, G.u8, opts) { |_x| false }.failure_id
  end

  def test_zero_cases_is_never_a_pass
    v = R.check(1, G.u8, opts(num_tests: 0)) { |_x| true }
    refute v.passed
    assert_equal Pbt::FailureIds::EXHAUSTED, v.failure_id
    assert_match(/never ran/, v.message)
  end

  def test_a_raising_property_is_a_panic
    v = R.check(1, G.u8, opts) { |_x| raise ArgumentError, "boom" }
    assert_equal Pbt::FailureIds::PANIC, v.failure_id
    assert_includes v.message, "ArgumentError: boom"
    assert_equal 1, v.seed
    red = R.check(1, G.u8, opts(ci: :on)) { |_x| raise ArgumentError, "secret" }
    refute_includes red.message, "secret"
    assert_includes red.message, "ArgumentError"
    assert_nil red.seed
  end

  def test_assertion_style_exceptions_that_are_not_standard_errors_are_panics
    klass = Class.new(Exception) # rubocop:disable Lint/InheritException
    v = R.check(1, G.u8, opts) { |_x| raise klass, "assertion failed" }
    assert_equal Pbt::FailureIds::PANIC, v.failure_id
  end

  def test_process_level_exceptions_are_never_swallowed
    assert_raises(Interrupt) { R.check(1, G.u8, opts) { |_x| raise Interrupt } }
    assert_raises(SystemExit) { R.check(1, G.u8, opts) { |_x| exit } }
    assert_raises(NoMemoryError) { R.check(1, G.u8, opts) { |_x| raise NoMemoryError } }
    assert_raises(NoMemoryError) { R.check(1, Pbt::Gen.new(->(_r) { raise NoMemoryError }), opts) { |_x| true } }
    assert_raises(Timeout::ExitException) { R.check(1, G.u8, opts) { |_x| raise Timeout::ExitException } }
  end

  def test_a_raise_during_shrinking_is_not_a_smaller_counterexample
    # x == 5 raises; the property falsifies for x > 5.  Shrinking must stop at 6.
    v = R.check(3, G.integer(0, 1000), opts) do |x|
      raise "five" if x == 5
      x <= 5
    end
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
    assert_equal 6, v.minimal
  end

  def test_a_generator_error_is_exhausted
    bad = Pbt::Gen.new(->(_r) { raise "gen boom" })
    v = R.check(1, bad, opts) { |_x| true }
    assert_equal Pbt::FailureIds::EXHAUSTED, v.failure_id
    assert_includes v.message, "RuntimeError: gen boom"
    red = R.check(1, bad, opts(ci: :on)) { |_x| true }
    refute_includes red.message, "gen boom"
    assert_includes red.message, "RuntimeError"
  end

  def test_an_unsatisfiable_filter_is_exhausted
    g = G.filter(G.u8, 5) { |_x| false }
    v = R.check(1, g, opts) { |_x| true }
    assert_equal Pbt::FailureIds::EXHAUSTED, v.failure_id
    assert_match(/rejected all candidates at iteration 1/, v.message)
  end

  def test_a_lying_shrinker_cannot_change_the_witness
    # returns a value that PASSES: must fall back to the original, 0 steps
    lying = ->(_value, _falsifies, _steps) { [0, 3] }
    v = R.check(5, G.integer(10, 100), opts, shrink: lying) { |x| x < 10 }
    assert_equal v.counterexample, v.minimal
    assert_equal 0, v.shrink_steps
  end

  def test_shrinker_failures_fall_back_to_the_original
    [->(*) { raise "boom" }, ->(*) {}, ->(v, *) { [v, -1] }, ->(v, *) { [v, 10_001] }, ->(v, *) { [v, "x"] },
      ->(*) { 5 }].each do |bad|
      v = R.check(5, G.integer(10, 100), opts, shrink: bad) { |x| x < 10 }
      assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
      assert_equal v.counterexample, v.minimal
      assert_equal 0, v.shrink_steps
    end
  end

  def test_a_shrinker_returning_a_value_the_property_raises_on_is_not_adopted
    sneaky = ->(_v, _f, _s) { [-1, 1] }
    v = R.check(5, G.integer(10, 100), opts, shrink: sneaky) do |x|
      raise "negative" if x < 0
      x < 10
    end
    assert_equal v.counterexample, v.minimal
  end

  def test_a_good_custom_shrinker_is_used
    v = R.check(5, G.integer(10, 100), opts, shrink: ->(_v, _f, _s) { [10, 1] }) { |x| x < 10 }
    assert_equal [10, 1], [v.minimal, v.shrink_steps]
  end

  # -- CI redaction ----------------------------------------------------------------

  def test_unset_and_on_redact_everything
    %i[unset on].each do |mode|
      v = R.check(99_999, G.integer(0, 1000), Pbt::Options.new(ci: mode)) { |x| x <= 100 }
      assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
      assert_nil v.seed
      assert_nil v.counterexample
      assert_nil v.minimal
      refute_includes v.message, "99999"
      refute_includes v.message, "101"
      assert_match(/repro-fnv: [0-9a-f]{16}/, v.message)
      assert_equal 16, v.digest.size
    end
  end

  def test_the_default_options_redact
    v = R.check(5, G.integer(0, 1000)) { |x| x <= 100 }
    assert_nil v.minimal
  end

  def test_the_digest_is_stable_and_witness_sensitive
    a = R.check(1, G.integer(0, 1000), opts(ci: :on)) { |x| x <= 100 }
    b = R.check(1, G.integer(0, 1000), opts(ci: :on)) { |x| x <= 100 }
    c = R.check(1, G.integer(0, 1000), opts(ci: :on)) { |x| x <= 200 }
    assert_equal a.digest, b.digest
    refute_equal a.digest, c.digest
    expected = format("%016x", Prng.fnv1a64("iter:#{a.iteration};steps:#{a.shrink_steps};shrunk:101"))
    assert_equal expected, a.digest
  end

  def test_ci_from_lookup
    assert_equal :off, Pbt::Options.ci_from_lookup({})
    ["", " ", "0", "false", "FALSE", " False "].each { |v| assert_equal :off, Pbt::Options.ci_from_lookup({"CI" => v}), v.inspect }
    %w[1 true yes anything].each { |v| assert_equal :on, Pbt::Options.ci_from_lookup({"CI" => v}), v }
  end

  # -- validation ------------------------------------------------------------------

  def test_invalid_options_raise_instead_of_producing_a_verdict
    assert_raises(ArgumentError) { Pbt::Options.new(num_tests: -1) }
    assert_raises(ArgumentError) { Pbt::Options.new(num_tests: Pbt::Limits::MAX_CASES + 1) }
    assert_raises(ArgumentError) { Pbt::Options.new(max_shrink_steps: -1) }
    assert_raises(ArgumentError) { Pbt::Options.new(max_shrink_steps: Pbt::Limits::MAX_SHRINK_STEPS + 1) }
    assert_raises(ArgumentError) { Pbt::Options.new(ci: true) }
    assert_raises(TypeError) { Pbt::Options.new(num_tests: 1.5) }
    assert_raises(TypeError) { R.check("1", G.u8) { true } }
    assert_raises(TypeError) { R.check(1, :nope) { true } }
    assert_raises(TypeError) { R.check(1, G.u8, 5) { true } }
    assert_raises(ArgumentError) { R.check(1, G.u8) }
    assert_raises(ArgumentError) { R.check(1, G.u8, nil, shrink: 3) { true } }
  end

  def test_negative_and_huge_seeds_wrap
    a = R.check(-1, G.integer(0, 1000), opts) { |x| x <= 100 }
    b = R.check(2**64 - 1, G.integer(0, 1000), opts) { |x| x <= 100 }
    assert_equal a, b
    assert_equal 2**64 - 1, a.seed
  end

  def test_failure_ids_are_closed
    assert_equal 4, Pbt::FailureIds::ALL.size
    Pbt::FailureIds::ALL.each { |id| assert Pbt::FailureIds.pbt_failure_id?(id) }
    refute Pbt::FailureIds.pbt_failure_id?("unregistered-id")
    refute Pbt::FailureIds.pbt_failure_id?(nil)
    refute Pbt::FailureIds.pbt_failure_id?(:"property-panic")
  end

  def test_rendering_a_hostile_witness_still_yields_a_verdict
    bomb = Object.new
    def bomb.inspect = raise("inspect boom")
    v = R.check(1, G.constant(bomb), opts) { |_x| false }
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
    assert_includes v.message, "<unrenderable>"
    red = R.check(1, G.constant(bomb), opts(ci: :on)) { |_x| false }
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, red.failure_id

    deep = Object.new
    def deep.inspect = inspect
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, R.check(1, G.constant(deep), opts) { |_x| false }.failure_id
  end

  def test_message_escapes_forged_log_lines
    v = R.check(1, G.constant("a\nFAKE: ok"), opts) { |_x| false }
    refute_includes v.message, "\n"
  end

  def test_each_case_draws_from_the_stream_in_order
    seen = []
    R.check(3, G.integer(0, 1000), opts(num_tests: 5)) { |x| seen << x }
    assert_equal G.gen_bounded(3, 0, 1000, 5), seen
  end
end
