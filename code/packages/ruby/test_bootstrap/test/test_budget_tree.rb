# frozen_string_literal: true

require_relative "test_helper"

# The hierarchical budget tree (VER05h 3.1): the bound holds whatever the
# nesting, with no classification of a call as "inside a property" or "inside a
# shrinker".  Ported from the Java engine's ShrinkBudgetTest (VER05e 4.1) and
# the C# ShrinkBudgetTreeTests.
class TestBudgetTree < HookedTest
  G = Pbt::Gens
  R = Pbt::Runner
  SB = Pbt::ShrinkBudget

  def opts(num_tests = 100, steps = 1000, ci = :off) = Pbt::Options.new(num_tests: num_tests, max_shrink_steps: steps, ci: ci)

  # Runs the block with a small root work cap; returns the root's [attempts, work].
  def root_totals(cap)
    SB.default_max_work = cap
    yield
    [SB.last_attempts, SB.last_work]
  end

  def lists(max) = G.list(G.u8, 0, max)

  def sevens(n) = Array.new(n, 7)

  def attempt_budget(n) = Pbt::Limits.attempt_budget(n)

  # Spends everything a scope's chain has left.
  def drain(budget)
    nil while budget.charge(1)
  end

  # -- 1. a property calling Runner.check, depth 1..3 ----------------------------

  def nested(depth, max_steps, evals)
    local = 0
    R.check(1, lists(40), opts(100, max_steps)) do |l|
      local += 1 if SB.current
      if depth > 1
        inner = [0]
        nested(depth - 1, max_steps, inner)
        local += inner[0]
      end
      l.size < 5
    end
    evals[0] += local
  end

  def test_a_property_calling_check_keeps_the_root_totals_within_the_root_cap
    (1..3).each do |depth|
      max_steps = 5
      evals = [0]
      attempts, work = root_totals(1_000_000) { nested(depth, max_steps, evals) }
      cap = 100 + attempt_budget(max_steps)
      assert_operator attempts, :<=, cap, "depth #{depth} attempts #{attempts}"
      assert_operator work, :<=, 1_000_000
      assert_operator evals[0], :>, 0
      assert_operator evals[0], :<=, 3 * cap * depth, "depth #{depth} evaluations #{evals[0]}"
      Pbt::Hooks.reset
    end
  end

  def branch(level, counted)
    local = 0
    R.check(1, G.integer(0, 9), opts(10, 5)) do |x|
      local += 1 if level > 1
      if level < 4
        a = [0]
        b = [0]
        branch(level + 1, a)
        branch(level + 1, b)
        local += a[0] + b[0]
      end
      x < 3
    end
    counted[0] += local
  end

  def test_a_branching_property_keeps_total_evaluations_within_the_root_cap
    counted = [0]
    attempts, = root_totals(1_000_000) { branch(1, counted) }
    cap = 10 + attempt_budget(5)
    assert_operator counted[0], :>, 0
    assert_operator attempts, :<=, cap
    assert_operator counted[0], :<=, cap * 2, "nested evaluations #{counted[0]} (2^depth would be far more)"
  end

  # -- 2. custom shrinkers and filter predicates -----------------------------------

  def test_a_recursing_custom_shrinker_stays_inside_the_root_cap
    inner = lists(50)
    start = sevens(50)
    calls = 0
    extra = 0
    max_steps = 10
    custom = lambda do |value, _falsifies, steps|
      100.times do
        inner.shrink(value.dup, steps) { |l| l.size == 50 }
        inner.shrink(value.dup, steps) { |l|
          extra += 1
          l.size == 50
        }
        Pbt::Shrink.shrink_list([1, 2, 3], 0, steps, &:empty?)
      end
      [value, 0]
    end
    outer = Pbt::Gen.new(->(_r) { start.dup }, custom)
    attempts, work = root_totals(1_000_000) do
      outer.shrink(start.dup, max_steps) {
        calls += 1
        false
      }
    end
    assert_operator attempts, :<=, attempt_budget(max_steps)
    assert_operator work, :<=, 1_000_000
    assert_operator calls + extra, :<=, attempt_budget(max_steps)
    assert_operator extra, :>, 0
  end

  def test_a_filter_predicate_that_shrinks_stays_inside_the_root_cap
    g = lists(200)
    start = Array.new(200, 3)
    inner_evals = 0
    max_steps = 5
    filtered = G.filter(G.integer(0, 1000), 100) do |_x|
      g.shrink(start.dup, 1000) { |l|
        inner_evals += 1
        l.size != 200
      }
      true
    end
    attempts, work = root_totals(5_000_000) { filtered.shrink(900, max_steps) { |x| x >= 10 } }
    assert_operator inner_evals, :>, 0, "the predicate must have shrunk"
    assert_operator attempts, :<=, attempt_budget(max_steps)
    assert_operator work, :<=, 5_000_000
    assert_operator inner_evals, :<=, attempt_budget(max_steps)
  end

  def test_a_filter_predicate_is_not_invoked_under_a_spent_budget
    calls = 0
    filtered = G.filter(G.integer(0, 1000), 100) do |_x|
      calls += 1
      true
    end
    SB.enter_child(0) do |root|
      drain(root)
      value, steps = filtered.shrink(900, 1000) { |_x| false }
      assert_equal [900, 0], [value, steps]
      assert_equal 0, calls
    end
    ok, = filtered.shrink(900, 1000) { |x| x >= 10 }
    assert_operator calls, :>, 0
    assert_equal 10, ok
  end

  def test_a_filter_sampler_rejecting_everything_stays_inside_the_root_cap_in_a_nested_check
    pred_calls = 0
    rejecting = G.filter(G.integer(0, 1000), Pbt::Limits::MAX_RETRIES) do |_x|
      pred_calls += 1
      false
    end
    max_steps = 5
    root_totals(1_000_000) do
      G.integer(0, 1000).shrink(900, max_steps) { |x|
        R.check(1, rejecting, opts(3, 10)) { true }
        x >= 10
      }
    end
    assert_operator pred_calls, :>, 0
    assert_operator pred_calls, :<=, attempt_budget(max_steps), "sampler retries #{pred_calls}"
  end

  def test_stacked_filters_cannot_multiply_retries_at_the_top_level
    SB.default_max_work = 5_000
    calls = 0
    level = ->(g, divisor) { G.filter(g, Pbt::Limits::MAX_RETRIES) { |x| (x / divisor) % 50 == 0 } }
    innermost = G.filter(G.integer(0, 10**9), Pbt::Limits::MAX_RETRIES) { |x|
      calls += 1
      x % 50 == 0
    }
    # each level accepts 1 in 50 independently, so three levels need ~125,000 draws; the root allows 5,000
    v = R.check(1, level.call(level.call(innermost, 50), 2500), opts(1, 5)) { true }
    assert_equal Pbt::FailureIds::EXHAUSTED, v.failure_id
    assert_operator calls, :<=, 5_000
  end

  def test_top_level_filter_retries_are_work_not_attempts
    # accepting 1 in 1000 would exhaust a default 100-case check if retries were attempts
    # (the attempt cap of a top-level check is only 100 + 16 * 1000 + 64)
    g = G.filter(G.integer(0, 999), 10_000) { |x| x == 0 }
    v = R.check(1, g, opts(100, 1000)) { true }
    assert v.passed, v.message
  end

  def test_max_retries_is_capped_by_a_limit
    g = G.integer(0, 9)
    refute_nil G.filter(g, Pbt::Limits::MAX_RETRIES) { true }
    assert_raises(ArgumentError) { G.filter(g, Pbt::Limits::MAX_RETRIES + 1) { true } }
  end

  # -- 3. threads -------------------------------------------------------------------

  def test_a_thread_spawned_in_a_scope_charges_the_root_even_after_the_inner_scope_closed
    root = SB.enter_child(10)
    go = Queue.new
    charged = nil
    t = nil
    SB.enter_child(10) do
      t = Thread.new do
        go.pop
        cur = SB.current
        charged = !cur.nil? && cur.charge(1)
      end
    end
    before = root.attempts
    go << 1
    t.join
    assert charged
    assert_equal before + 1, root.attempts
  ensure
    root&.close
  end

  def test_a_thread_outliving_the_root_gets_a_fresh_working_root_each_time
    go = Queue.new
    sizes = []
    t = nil
    SB.enter_child(1) do
      t = Thread.new do
        go.pop
        g = lists(50)
        3.times { sizes << g.shrink(sevens(50), 1000) { |l| l.size >= 1 }.first.size }
      end
    end
    go << 1
    t.join
    assert_equal [1, 1, 1], sizes
  end

  def test_a_thread_outliving_its_scope_shrinks_normally_afterwards
    g = lists(50)
    go = Queue.new
    output = []
    holder = nil
    parent = Pbt::Gen.new(->(_r) { 0 }, lambda { |value, _f, _s|
      holder = Thread.new do
        go.pop
        output << g.shrink_nested(sevens(50), ->(l) { l.size >= 1 }, 1000)
        output << g.shrink(sevens(50), 1000) { |l| l.size >= 1 }
      end
      [value, 0]
    })
    parent.shrink(0, 10) { true } # the scope closes here, with the thread alive
    go << 1
    holder.join
    assert_equal 2, output.size
    output.each { |r| assert_equal 1, r.first.size }
  end

  def test_threads_created_inside_a_scope_share_one_global_budget
    g = lists(50)
    start = sevens(50)
    calls = 0
    lock = Mutex.new
    max_steps = 10
    parent = Pbt::Gen.new(->(_r) { 0 }, lambda { |value, _f, _s|
      threads = Array.new(6) do
        Thread.new do
          g.shrink_nested(start.dup, lambda { |l|
            lock.synchronize { calls += 1 }
            l.size == 50
          }, max_steps)
        end
      end
      fibers = Array.new(3) do
        Fiber.new do
          g.shrink_nested(start.dup, lambda { |l|
            lock.synchronize { calls += 1 }
            l.size == 50
          }, max_steps)
        end
      end
      fibers.each(&:resume)
      threads.each(&:join)
      [value, 0]
    })
    parent.shrink(0, max_steps) { true }
    assert_operator calls, :<=, attempt_budget(max_steps), "evaluations #{calls}"
    assert_operator calls, :>, 50, "the workers must really have shrunk"
  end

  # The documented thread escape: a thread that existed before the scope has no
  # ambient budget, nor does a fiber created with storage: nil.
  def test_a_thread_that_does_not_inherit_has_no_ambient_budget
    jobs = Queue.new
    results = Queue.new
    worker = Thread.new do
      while (job = jobs.pop)
        results << job.call
      end
    end # started BEFORE any scope: it holds empty storage
    SB.enter_child(10) do |root|
      jobs << -> { SB.current }
      assert_nil results.pop # documented limitation
      refute_nil SB.current
      assert_nil Fiber.new(storage: nil) { SB.current }.resume
      assert_same root, Fiber.new { SB.current }.resume
      assert_same root, Thread.new { SB.current }.value
    end
    jobs << nil
    worker.join
  end

  # -- 5/6/8: nested checks ----------------------------------------------------------

  def test_a_passing_inner_check_with_a_huge_num_tests_is_bounded_under_an_outer_shrink
    inner_cases = 0
    max_steps = 5
    attempts, = root_totals(1_000_000) do
      G.integer(0, 1000).shrink(900, max_steps) { |x|
        R.check(1, G.integer(0, 9), opts(Pbt::Limits::MAX_CASES, 10)) {
          inner_cases += 1
          true
        }
        x >= 10
      }
    end
    assert_operator inner_cases, :<=, attempt_budget(max_steps) + 64
    assert_operator attempts, :<=, attempt_budget(max_steps)
  end

  def test_shrink_int_and_simulate_replay_inside_a_property_are_charged
    evals = 0
    max_steps = 5
    attempts, = root_totals(1_000_000) do
      G.integer(0, 1000).shrink(900, max_steps) { |x|
        Pbt::Shrink.shrink_int(500, 0, 0, 1000, 1000) { |v|
          evals += 1
          v < 100
        }
        Pbt::Shrink.simulate_replay(3) { |v|
          evals += 1
          v < 100
        }
        x >= 10
      }
    end
    assert_operator evals, :>, 0
    assert_operator attempts, :<=, attempt_budget(max_steps)
    assert_operator evals, :<=, attempt_budget(max_steps)
  end

  def test_a_starved_nested_check_is_exhausted_but_a_top_level_check_still_runs
    evaluated = 0
    SB.enter_child(0) do |root|
      drain(root)
      before = root.attempts
      nested = R.check(1, G.list(G.u8, 5, 5), opts(10, 1000)) {
        evaluated += 1
        false
      }
      assert_equal Pbt::FailureIds::EXHAUSTED, nested.failure_id
      refute nested.passed
      assert_equal 0, evaluated
      assert_equal before, root.attempts # all or nothing: unchanged

      # a size-0 sample charges no work, so ONLY the entry charge can stop this one
      zero_sized = 0
      tiny = R.check(1, G.integer(0, 9), opts(1, 10)) {
        zero_sized += 1
        true
      }
      assert_equal Pbt::FailureIds::EXHAUSTED, tiny.failure_id
      assert_equal 0, zero_sized
    end
    top = R.check(1, G.list(G.u8, 5, 5), opts(10, 1000)) { false }
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, top.failure_id
    refute_nil top.minimal
  end

  def test_a_nested_check_pays_one_attempt_at_entry
    SB.enter_child(1000) do |root|
      before = root.attempts
      v = R.check(1, G.integer(0, 9), opts(1, 10)) { true }
      assert v.passed
      assert_equal before + 1, root.attempts
    end
  end

  def test_a_nested_check_inside_the_outer_case_loop_charges_the_outer_root
    inner_cases = 0
    attempts, = root_totals(1_000_000) do
      R.check(1, G.integer(0, 9), opts(5, 5)) do |_x|
        R.check(2, G.integer(0, 9), opts(200, 10)) {
          inner_cases += 1
          true
        }
        true # the outer property never fails: no shrink at all
      end
    end
    cap = 5 + attempt_budget(5)
    assert_operator attempts, :>, 0
    assert_operator attempts, :<=, cap
    assert_operator inner_cases, :<=, cap, "inner cases #{inner_cases} (uncharged: 5 x 200)"
  end

  # -- 9/10: the primitive -------------------------------------------------------------

  def test_a_refused_take_leaves_every_counter_unchanged
    root = SB.enter_child(1000, 100)
    child = SB.enter_child(1000, 1_000_000)
    assert child.charge(60)
    refute child.charge(60) # the root has only 40 work left
    assert_equal 1, child.attempts
    assert_equal 60, child.work
    assert_equal 1, root.attempts
    assert_equal 60, root.work
    assert child.refused?
  ensure
    child&.close
    root&.close
  end

  def test_a_huge_cost_is_refused_without_wrapping
    b = SB.enter_child(1000, 100)
    assert b.charge(60)
    refute b.charge(2**63 - 1)
    refute b.charge_copy(2**63 - 1)
    refute b.charge(2**200)
    assert_equal 60, b.work
  ensure
    b&.close
  end

  def test_a_zero_or_negative_cost_is_charged_as_one
    b = SB.enter_child(1000, 100)
    assert b.charge(0)
    assert b.charge(-5)
    assert_equal 2, b.work
  ensure
    b&.close
  end

  def test_concurrent_charging_never_exceeds_the_cap
    b = SB.enter_child(1000, 5000)
    ok = 0
    lock = Mutex.new
    threads = Array.new(16) do
      Thread.new do
        1000.times { lock.synchronize { ok += 1 } if b.charge(1) }
      end
    end
    threads.each(&:join)
    assert_equal 5000, ok
    assert_equal 5000, b.work
    assert_equal 5000, b.attempts
  ensure
    b&.close
  end

  def test_the_attempt_cap_is_enforced_separately_from_work
    b = SB.enter_child(0) # 64 attempts
    64.times { assert b.charge_copy(1) } # work only: attempts unchanged
    assert_equal 0, b.attempts
    64.times { assert b.charge(1) }
    refute b.charge(1)
    refute_predicate b, :room?
  ensure
    b&.close
  end

  # -- 11: scope hygiene -----------------------------------------------------------------

  def test_the_ambient_budget_is_restored_after_a_raise_and_deep_nesting_joins
    thrower = Pbt::Gen.new(->(_r) { 0 }, ->(*) { raise "shrinker boom" })
    assert_raises(RuntimeError) { thrower.shrink(0, 5) { true } }
    assert_nil SB.current
    assert_raises(RuntimeError) { G.integer(0, 10).shrink(5, 5) { raise "property boom" } }
    assert_nil SB.current

    opened = Array.new(SB::MAX_DEPTH + 6) { SB.enter_child(10) }
    opened.each_with_index { |b, i| assert_equal i >= SB::MAX_DEPTH, b.joined?, "scope #{i}" }
    opened.reverse_each(&:close)
    assert_nil SB.current
  end

  def test_enter_check_also_joins_beyond_the_depth_cap
    opened = Array.new(SB::MAX_DEPTH + 3) { SB.enter_check(5, 5) }
    refute_predicate opened[0], :nested?
    assert_predicate opened[1], :nested?
    assert_predicate opened[SB::MAX_DEPTH + 2], :joined?
    opened.reverse_each(&:close)
    assert_nil SB.current
  end

  def test_enter_joins_the_ambient_scope_or_opens_a_root
    SB.enter(5) { |b|
      refute_predicate b, :joined?
      refute_predicate b, :nested?
      SB.enter(5) { |j|
        assert_predicate j, :joined?
        assert_same SB.current, SB.current
        assert j.charge(1)
        assert_equal 1, b.attempts # the joined handle charged its owner
        assert_equal 1, j.attempts
      }
      refute_predicate b, :closed?
    }
    assert_nil SB.current
  end

  def test_a_closed_scope_resolves_to_the_nearest_open_ancestor
    outer = SB.enter_child(10)
    inner = SB.enter_child(10)
    inner.close
    assert_same outer, SB.current
    outer.close
    assert_nil SB.current
  end

  def test_closing_is_idempotent_and_records_stats
    SB.enter_child(10) { |b| 3.times { b.charge(2) } }
    assert_equal 3, SB.last_attempts
    assert_equal 6, SB.last_work
    b = SB.enter_child(10)
    b.close
    b.close
    assert_nil SB.current
  end

  def test_charged_wraps_a_property_and_stops_at_the_cap
    SB.enter_child(0) do |b|
      calls = 0
      wrapped = b.charged(lambda { |_v|
        calls += 1
        true
      })
      100.times { wrapped.call(1) }
      assert_equal 64, calls
    end
  end

  # -- sampling work ---------------------------------------------------------------------

  def test_a_nested_check_sampling_large_lists_is_charged_as_work
    inner = []
    root_totals(10_000) do
      G.integer(0, 1000).shrink(900, 1000) { |x|
        inner << R.check(1, G.list(G.u8, 2000, 2000), opts(50, 10)) { true }
        x >= 10
      }
    end
    refute_empty inner
    assert(inner.any? { |v| v.failure_id == Pbt::FailureIds::EXHAUSTED })
  end

  def test_sampled_lists_of_lists_are_charged_by_their_contents
    inner = []
    root_totals(20_000) do
      G.integer(0, 1000).shrink(900, 1000) { |x|
        inner << R.check(1, G.list(G.list(G.u8, 100, 100), 100, 100), opts(50, 10)) { true }
        x >= 10
      }
    end
    # 100 + 100 x 100 = 10,100 per case: the second case no longer fits in 20,000
    assert_equal Pbt::FailureIds::EXHAUSTED, inner[0].failure_id
  end

  def test_a_hostile_collection_still_yields_a_verdict_top_level_and_nested
    bad = Class.new(Array) do
      def size = raise("hostile size")

      def each = raise("hostile each")

      def inspect = raise("hostile inspect")
    end.new
    v = R.check(1, G.constant(bad), opts(1, 10)) { false }
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
    assert_includes v.message, "<unrenderable>"

    nested = nil
    root_totals(1_000_000) do
      G.integer(0, 1000).shrink(900, 5) { |x|
        nested = R.check(1, G.constant(bad), opts(1, 10)) { false }
        x >= 10
      }
    end
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, nested.failure_id
  end

  def test_other_enumerables_are_counted_by_a_capped_enumeration
    lazy = Class.new do
      include Enumerable

      def each
        loop { yield 1 } # infinite
      end
    end.new
    assert_equal Pbt::Runner::WALK_CAP, R.size_of(lazy)
    assert_equal 0, R.size_of(5)
    assert_equal 3, R.size_of("abc")
    assert_equal 2, R.size_of({a: 1, b: 2}) # a Hash counts its entries; its values are scalars
    assert_equal 4, R.size_of(Struct.new(:a, :b, :c, :d).new(1, 2, 3, 4))
  end

  # -- max_steps / re-verify / starvation ------------------------------------------------

  def test_max_steps_is_validated_and_zero_steps_returns_the_input_uncopied
    g = lists(5)
    assert_raises(ArgumentError) { g.shrink([], -1) { true } }
    assert_raises(ArgumentError) { g.shrink_nested([], ->(_l) { true }, -1) }
    assert_raises(ArgumentError) { g.shrink([], Pbt::Limits::MAX_SHRINK_STEPS + 1) { true } }
    input = [1, 2, 3]
    assert_same input, Pbt::Shrink.shrink_sequence(input, ->(_l) { true }, 0, 0, nil).first
    assert_same input, g.shrink(input, 0) { true }.first
  end

  def test_a_refused_re_verification_reports_the_original_witness_with_zero_steps
    SB.enter_child(0) do
      spender = lambda do |_value, _falsifies, _steps|
        nil while SB.current.charge(1)
        [3, 1]
      end
      v = R.check(1, G.constant(100), opts(1, 5), shrink: spender) { false }
      assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
      assert_equal 100, v.minimal
      assert_equal 0, v.shrink_steps
    end
    # With room, the same shrinker's result IS adopted (3 still fails the property).
    ok = R.check(1, G.constant(100), opts(1, 5), shrink: ->(_v, _f, _s) { [3, 1] }) { false }
    assert_equal [3, 1], [ok.minimal, ok.shrink_steps]
  end

  def test_a_starved_nested_check_never_lets_the_top_level_check_pass
    v = R.check(1, G.integer(0, 9), opts(3, 0)) do
      R.check(2, G.integer(0, 9), opts(5000, 0)) { true }
      true # the property itself never fails
    end
    refute v.passed
    assert_equal Pbt::FailureIds::EXHAUSTED, v.failure_id
    assert_match(/starved/, v.message)

    ok = R.check(1, G.integer(0, 9), opts(3, 0)) do
      R.check(2, G.integer(0, 9), opts(5, 0)) { true }
      true
    end
    assert ok.passed
  end

  def test_starvation_does_not_override_a_falsified_verdict
    v = R.check(1, G.integer(0, 9), opts(3, 0)) do |x|
      R.check(2, G.integer(0, 9), opts(5000, 0)) { true }
      x > 100 # always falsifies
    end
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
  end

  def test_a_reentrant_check_matches_a_standalone_run_while_the_outer_budget_has_room
    inner_run = -> { R.check(5, G.list(G.u8, 0, 50), opts(200, 1000, :on)) { |l| l.sum <= 20 } }
    standalone = inner_run.call
    inner = []
    outer = opts(100, 10_000, :on)
    expected = R.check(9, G.integer(0, 1000), outer) { |x| x < 500 }
    actual = R.check(9, G.integer(0, 1000), outer) { |x|
      inner << inner_run.call
      x < 500
    }
    refute_empty inner
    inner.each { |v| assert_equal standalone, v }
    assert_equal expected, actual
  end

  def test_a_budget_spent_inside_the_runner_stops_every_nested_shrinker_from_copying
    touched = 0
    g = G.list(G.u8, 0, 100_000)
    SB.default_max_work = 1_000_000
    v = R.check(1, G.constant(Array.new(100_000, 0)), opts(1, 1000), shrink: ->(value, f, s) { g.shrink_nested(value, f, s) }) { |l|
      touched += l.size
      l.size != 100_000
    }
    assert_equal Pbt::FailureIds::COUNTEREXAMPLE, v.failure_id
    assert_operator touched, :<=, 1_000_000 + 300_000, "elements handed to the property #{touched}"
  end

  def test_the_whole_check_body_runs_in_one_scope
    seen = []
    R.check(1, G.integer(0, 9), opts(3, 5)) do |_x|
      seen << SB.current
      true
    end
    assert(seen.none?(&:nil?))
    assert_equal 1, seen.uniq.size
    assert_nil SB.current
  end
end
