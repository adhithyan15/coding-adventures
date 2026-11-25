# frozen_string_literal: true

require_relative "test_helper"
require "minitest/mock"

# Campaign, executor and handle contracts (VER06 section 9.1, VER06i sections 5-6).
class TestFuzzEngine < Minitest::Test
  include CodingAdventures::TestBootstrap

  # An exception whose every observable method is hostile.  The engine must still
  # classify it by its real class, read no message and keep the campaign alive.
  class Hostile < StandardError
    TOUCHED = []

    def self.name = raise("hostile name")

    def self.to_s = raise("hostile class to_s")

    def self.inspect = raise("hostile class inspect")

    def class = raise("hostile class")

    def message = (TOUCHED << :message) && raise("hostile message")

    def to_s = (TOUCHED << :to_s) && raise("hostile to_s")

    def inspect = (TOUCHED << :inspect) && raise("hostile inspect")

    def full_message(*) = (TOUCHED << :full_message) && raise("hostile full_message")

    def detailed_message(*) = (TOUCHED << :detailed_message) && raise("hostile detailed_message")

    def is_a?(*) = raise("hostile is_a?")

    def kind_of?(*) = raise("hostile kind_of?")

    def backtrace_locations = raise("hostile backtrace_locations")

    def backtrace = raise("hostile backtrace")
  end

  module Boom
    def self.go(depth) = (depth <= 0) ? raise(ArgumentError, "boom") : go(depth - 1)

    def self.outer = go(0)
  end

  class ErrA < StandardError; end

  class ErrB < StandardError; end

  OPTS = Fuzz::Options.new(max_iterations: 200, seed: 1)

  def fz(corpus = [], opts = OPTS, **kw, &target) = Fuzz.run(corpus, opts, **kw, &target)

  def opts(**kw) = Fuzz::Options.new(max_iterations: 200, seed: 1, **kw)

  def one_exec(target, step_budget: 5, mem_budget: 100)
    handle = Fuzz::Handle.new(step_budget, mem_budget, Fuzz::Handle::CampaignToken.new, 1, Fuzz::Bitmap.new)
    [Fuzz::Executor.run_target(target, "ab".b, handle), handle]
  end

  # -- contracts ----------------------------------------------------------------

  def test_a_campaign_is_a_pure_function_of_corpus_options_and_target
    target = lambda do |d, h|
      h.edge(d.getbyte(0).to_i)
      raise ErrA if d.bytesize > 6 && d.getbyte(1) == 255
    end
    a = fz(["abc".b], opts(max_iterations: 400), &target)
    b = fz(["abc".b], opts(max_iterations: 400), &target)
    assert_equal a.corpus, b.corpus
    assert_equal a.inspect, b.inspect
    assert_equal a.crashes.map(&:signature), b.crashes.map(&:signature)
    assert_equal a.crashes.map(&:minimal_input), b.crashes.map(&:minimal_input)
    c = fz(["abc".b], opts(max_iterations: 400, seed: 2), &target)
    refute_equal a.corpus, c.corpus
  end

  def test_the_iteration_budget_is_exact_and_candidates_respect_the_length_cap
    seen = []
    report = fz(["abcdef".b], opts(max_iterations: 333, max_input_len: 9)) { |d, _| seen << d.bytesize }
    assert_equal 333, report.iterations
    assert_equal 334, seen.size # one triage run plus every iteration
    assert_equal 6, seen.first
    assert_operator seen.drop(1).max, :<=, 9
    assert_equal 0, fz(["a"], opts(max_iterations: 0)) { |_, _| }.iterations
  end

  def test_initial_seeds_run_as_given_even_above_the_candidate_cap
    seen = []
    fz(["a" * 50], opts(max_iterations: 1, max_input_len: 4)) { |d, _| seen << d.bytesize }
    assert_equal 50, seen.first
    assert_operator seen.last, :<=, 4
  end

  def test_an_empty_corpus_is_replaced_by_one_zero_seed
    seen = []
    report = fz([], opts(max_iterations: 3)) { |d, _| seen << d }
    assert_equal "\x00".b, seen.first
    assert_equal 3, report.iterations
    assert_equal ["\x00".b], report.corpus.first(1)
  end

  def test_a_corpus_of_crashers_fuzzes_nothing_and_is_never_clean
    report = fz(%w[a b c]) { |_, _| raise ErrA }
    assert_equal 0, report.iterations
    assert_equal 1, report.crashes.size
    assert_equal 3, report.crashes.first.hits
    refute_predicate report, :clean?
    t = fz(%w[a b], opts(step_budget: 10)) { |_, h| 1000.times { h.tick } }
    assert_equal [0, 2], [t.iterations, t.timeouts]
    o = fz(%w[a b]) { |_, h| h.reserve(1 << 40) }
    assert_equal [0, 2], [o.iterations, o.ooms]
  end

  def test_clean_needs_iterations_and_no_failure
    assert_predicate fz(["a"], opts(max_iterations: 5)) { |_, _| }, :clean?
    refute_predicate fz(["a"], opts(max_iterations: 0)) { |_, _| }, :clean?
    refute_predicate fz(["a"], opts(max_iterations: 50)) { |d, _| raise ErrA if d.bytesize != 1 }, :clean?
    refute_predicate fz(["a"], opts(max_iterations: 50)) { |d, h| h.tick while d.bytesize > 1 }, :clean?
  end

  def test_the_first_passing_seed_is_admitted_even_with_no_coverage
    report = fz(%w[a b], opts(max_iterations: 10)) { |_, _| }
    assert_equal ["a"], report.corpus.first(1)
    assert_equal "black-box-uninstrumented", report.coverage_tier
    assert_equal 0, report.total_edges
  end

  def test_caller_reported_tier_and_the_admission_rule
    report = fz(["a"], opts(max_iterations: 100)) { |d, h| h.edge(d.getbyte(0) & 3) }
    assert_equal "caller-reported", report.coverage_tier
    assert_operator report.total_edges, :<=, 4
    assert_operator report.total_edges, :>, 1
    assert_operator report.admitted_seeds, :<=, report.total_edges
  end

  def staircase(guided)
    lambda do |d, h|
      next unless d.getbyte(0) == 35

      h.edge(1) if guided
      next unless d.getbyte(1) == 128

      h.edge(2) if guided
      raise ArgumentError, "x" if d.getbyte(2) == 255 && (!guided || h.edge(3).nil?)
    end
  end

  def test_coverage_guidance_cracks_a_staircase_that_blind_mutation_cannot
    cracked = ->(guided, seeds) { seeds.count { |s| !fz(["\x00\x00\x00".b], opts(seed: s, max_iterations: 6000), &staircase(guided)).crashes.empty? } }
    assert_equal 3, cracked.call(true, 1..3)
    assert_equal 0, cracked.call(false, 1..2)
  end

  # -- dedup and minimisation ---------------------------------------------------

  def test_crashes_deduplicate_by_signature
    report = fz(["aa"], opts(max_iterations: 100)) do |d, _|
      raise ErrA if d.bytesize.odd?
      raise ErrB if d.bytesize.even? && d.bytesize > 2
    end
    kinds = report.crashes.map(&:kind).sort
    assert_equal ["TestFuzzEngine.ErrA", "TestFuzzEngine.ErrB"], kinds
    assert_equal report.crashes.map(&:signature).uniq.size, report.crashes.size
    assert_operator report.crashes.sum(&:hits), :>, 2
    # one record per signature even though the SAME class raised from two lines
    same = fz(["aa"], opts(max_iterations: 50)) { |d, _|
      if d.bytesize.odd?
        raise(ErrA)
      else
        ((d.bytesize == 2) ? nil : raise(ErrA))
      end
    }
    assert_equal 1, same.crashes.size
  end

  def test_different_frames_are_different_signatures
    report = fz(%w[a bb]) { |d, _| (d.bytesize == 1) ? Boom.go(0) : Boom.outer }
    assert_equal 2, report.crashes.size
  end

  def test_a_novel_crash_is_minimised_to_its_essence
    report = fz(["xxxBxxxxxx".b]) { |d, _| raise ErrA if d.include?("B") }
    crash = report.crashes.first
    assert_equal "B", crash.minimal_input
    assert_predicate crash, :minimized?
    assert_equal 1, report.crashes.size
  end

  def test_minimisation_can_reach_the_empty_input_and_zeroes_nonessential_bytes
    assert_equal "", fz(["z".b]) { |_, _| raise ErrA }.crashes.first.minimal_input
    r = fz(["\x07\x09\x03".b]) { |d, _| raise ErrA if d.bytesize == 3 && d.getbyte(1) == 9 }
    assert_equal "\x00\x09\x00".b, r.crashes.first.minimal_input
  end

  def test_minimisation_keeps_the_exact_signature_and_never_trades_one_bug_for_another
    report = fz(["aaaaaaaaaa"]) { |d, _| raise((d.bytesize >= 5) ? ErrA : ErrB) }
    crash = report.crashes.first
    assert_equal "TestFuzzEngine.ErrA", crash.kind
    assert_equal 5, crash.minimal_input.bytesize
  end

  def test_the_per_crash_minimisation_budget_counts_rejected_candidates
    calls = 0
    report = fz(["a" * 64], opts(minimize_budget: 10)) do |d, _|
      calls += 1
      raise ErrA if d.bytesize == 64 # every shorter candidate is rejected
    end
    assert_equal 1 + 10, calls
    refute_predicate report.crashes.first, :minimized?
    zero = fz(["a" * 64], opts(minimize_budget: 0)) { |_, _| raise ErrA }
    refute_predicate zero.crashes.first, :minimized?
    assert_equal 64, zero.crashes.first.minimal_input.bytesize
  end

  def test_the_campaign_wide_minimisation_budget_is_max_iterations
    calls = 0
    exec = lambda do |data, _cov|
      calls += 1
      Fuzz::Outcome.crash("k#{data.bytes.first}", [])
    end
    seeds = (1..20).map { |i| ([i] + [9] * 40).pack("C*") }
    report = Fuzz.run_outcomes(seeds, opts(max_iterations: 7, minimize_budget: 1000), exec)
    assert_equal 20, report.crashes.size
    # 20 triage runs + at most max_iterations (7) minimisation executions in total
    assert_operator calls, :<=, 20 + 7
    assert_operator report.crashes.count { |c| !c.minimized? }, :>=, 13
  end

  def test_minimize_crashing_input_is_validated_and_bounded
    sig = Fuzz::Signature.crash_signature("K", [])
    test = ->(d) { d.empty? ? Fuzz::Outcome.pass : Fuzz::Outcome.crash("K", []) }
    assert_equal "\x00".b, Fuzz.minimize_crashing_input("abcd", sig, 100, &test) # empty passes, so one byte stays, zeroed
    assert_raises(ArgumentError) { Fuzz.minimize_crashing_input("a", sig, 100) }
    assert_raises(ArgumentError) { Fuzz.minimize_crashing_input("a", sig, -1, &test) }
    assert_raises(ArgumentError) { Fuzz.minimize_crashing_input("a", sig, 100_001, &test) }
    assert_raises(TypeError) { Fuzz.minimize_crashing_input("a", sig, 1.5, &test) }
    assert_raises(TypeError) { Fuzz.minimize_crashing_input(5, sig, 10, &test) }
    assert_raises(ArgumentError) { Fuzz.minimize_crashing_input("a" * ((1 << 20) + 1), sig, 10, &test) }
    assert_equal "", Fuzz.minimize("".b, sig, test, 10).first
    assert_equal "", Fuzz.minimize_crashing_input("", sig, 10, &test)
  end

  # -- sticky exhaustion --------------------------------------------------------

  def swallow = ->(&blk) { blk.call } && nil

  def test_swallow_then_call_is_a_timeout_not_a_pass_or_crash
    out, = one_exec(lambda do |_d, h|
      begin
        10.times { h.tick }
      rescue Exception # rubocop:disable Lint/RescueException
        nil
      end
      3.times do
        h.tick
      rescue Exception # rubocop:disable Lint/RescueException
        nil
      end
    end)
    assert_equal :timeout, out.kind
  end

  def test_swallow_then_return_is_a_timeout
    out, = one_exec(lambda do |_d, h|
      loop { h.tick }
    rescue Exception # rubocop:disable Lint/RescueException
      :swallowed
    end)
    assert_equal :timeout, out.kind
  end

  def test_swallow_then_raise_is_a_timeout_not_a_crash
    out, = one_exec(lambda do |_d, h|
      loop { h.tick }
    rescue Exception # rubocop:disable Lint/RescueException
      raise ErrA, "after swallowing"
    end)
    assert_equal :timeout, out.kind
  end

  def test_a_plain_rescue_does_not_swallow_the_sentinel
    out, = one_exec(lambda do |_d, h|
      loop { h.tick }
    rescue => e
      raise "caught #{e.class}"
    end)
    assert_equal :timeout, out.kind
  end

  def test_exhaustion_is_shared_the_first_breach_wins
    # a memory breach swallowed, then tick: still the memory outcome
    out, = one_exec(lambda do |_d, h|
      begin
        h.reserve(1000)
      rescue Exception # rubocop:disable Lint/RescueException
        nil
      end
      h.tick
    end)
    assert_equal :oom, out.kind
    # a step breach swallowed, then reserve: still the timeout
    out, = one_exec(lambda do |_d, h|
      begin
        loop { h.tick }
      rescue Exception # rubocop:disable Lint/RescueException
        nil
      end
      h.reserve(1)
    end)
    assert_equal :timeout, out.kind
    # memory swallowed then returned
    out, = one_exec(lambda do |_d, h|
      h.reserve(101)
    rescue Exception # rubocop:disable Lint/RescueException
      nil
    end)
    assert_equal :oom, out.kind
  end

  def test_exhaustion_is_sticky_and_shared_on_the_handle_itself
    h = Fuzz::Handle.new(2, 10, Fuzz::Handle::CampaignToken.new, 1, nil)
    assert_raises(Fuzz::Handle::BudgetExceeded) { h.reserve(11) }
    first = h.exhausted
    assert_equal :memory, h.exhausted_which
    # a step well within budget still raises the memory sentinel: stickiness, not the counter
    assert_same first, assert_raises(Fuzz::Handle::BudgetExceeded) { h.tick }
    assert_same first, assert_raises(Fuzz::Handle::BudgetExceeded) { h.reserve(1) }
    assert_same first, assert_raises(Fuzz::Handle::BudgetExceeded) { h.reserve(0) }
    g = Fuzz::Handle.new(1, 10, Fuzz::Handle::CampaignToken.new, 1, nil)
    g.tick
    assert_raises(Fuzz::Handle::BudgetExceeded) { g.tick }
    steps = g.exhausted
    assert_equal :steps, g.exhausted_which
    assert_same steps, assert_raises(Fuzz::Handle::BudgetExceeded) { g.reserve(1) }
    assert_same steps, assert_raises(Fuzz::Handle::BudgetExceeded) { g.tick }
  end

  def test_every_call_after_a_swallowed_breach_keeps_raising
    raised = 0
    one_exec(lambda do |_d, h|
      begin
        h.reserve(1000)
      rescue Exception # rubocop:disable Lint/RescueException
        nil
      end
      5.times do
        h.tick
      rescue Exception # rubocop:disable Lint/RescueException
        raised += 1
      end
    end)
    assert_equal 5, raised
  end

  def test_the_budgets_are_exact
    out, h = one_exec(->(_d, hd) { 5.times { hd.tick } })
    assert_equal :pass, out.kind
    assert_nil h.exhausted
    out, = one_exec(->(_d, hd) { 6.times { hd.tick } })
    assert_equal :timeout, out.kind
    out, = one_exec(->(_d, hd) {
      hd.reserve(60)
      hd.reserve(40)
    })
    assert_equal :pass, out.kind
    out, = one_exec(->(_d, hd) {
      hd.reserve(60)
      hd.reserve(41)
    })
    assert_equal :oom, out.kind
    out, = one_exec(->(_d, hd) { [0, -5, -2**70].each { |n| hd.reserve(n) } })
    assert_equal :pass, out.kind
    out, = one_exec(->(_d, hd) { hd.reserve(2**70) })
    assert_equal :oom, out.kind
    out, = one_exec(->(_d, hd) { hd.reserve("1") })
    assert_equal ["crash", "TypeError"], [out.kind.to_s, out.crash_kind]
  end

  def test_step_budget_zero_times_out_on_the_first_tick
    report = fz(["a"], opts(step_budget: 0, max_iterations: 0)) { |_, h| h.tick }
    assert_equal 1, report.timeouts
  end

  def test_real_nomemoryerror_is_an_oom_and_only_around_the_target
    out, = one_exec(->(_d, _h) { raise NoMemoryError, "failed to allocate" })
    assert_equal :oom, out.kind
    report = fz(["a"], opts(max_iterations: 0)) { |_, _| raise NoMemoryError }
    assert_equal [0, 1, 0], [report.crashes.size, report.ooms, report.iterations]
    # a NoMemoryError while rendering a crash is an OOM too, never a crash
    Fuzz::Signature.stub(:frames_of, ->(_e) { raise NoMemoryError }) do
      out, = one_exec(->(_d, _h) { raise ErrA })
      assert_equal :oom, out.kind
    end
  end

  def test_system_stack_error_is_a_crash_with_collapsed_frames
    deep = ->(n) { deep.call(n + 1) }
    out, = one_exec(->(_d, _h) { deep.call(0) })
    assert_equal "crash", out.kind.to_s
    assert_equal "SystemStackError", out.crash_kind
    assert_operator out.frames.size, :<=, 3 # one lambda frame, collapsed
  end

  def test_process_owned_exceptions_propagate_out_of_the_campaign
    assert_raises(Interrupt) { fz(["a"]) { |_, _| raise Interrupt } }
    assert_raises(SystemExit) { fz(["a"]) { |_, _| exit(3) } }
    assert_raises(Timeout::ExitException) { fz(["a"]) { |_, _| raise Timeout::ExitException } }
    assert_raises(SignalException) { fz(["a"]) { |_, _| raise SignalException, "TERM" } }
  end

  # -- handle lifetime ------------------------------------------------------------

  def test_a_retired_handle_is_inert
    cov = Fuzz::Bitmap.new
    token = Fuzz::Handle::CampaignToken.new
    h = Fuzz::Handle.new(10, 10, token, 4, cov)
    h.edge(5)
    assert_equal 1, h.edge_calls
    h.retire
    assert_predicate h, :retired?
    h.edge(6)
    assert_equal [5], cov.covered_edges
    assert_equal 1, h.edge_calls
    sentinel = assert_raises(Fuzz::Handle::BudgetExceeded) { h.tick }
    assert_equal [4, :retired], token.provenance(sentinel)
    assert_raises(Fuzz::Handle::BudgetExceeded) { h.reserve(1) }
    assert_raises(Fuzz::Handle::BudgetExceeded) { h.reserve(0) }
    refute_kind_of StandardError, sentinel
    assert_kind_of Exception, sentinel
  end

  def test_each_execution_gets_a_fresh_handle_and_a_stale_thread_cannot_touch_later_runs
    stale = nil
    gate = Queue.new
    thread = nil
    seen_errors = Queue.new
    first = true
    report = fz(%w[a b], opts(max_iterations: 20)) do |_d, h|
      if first
        first = false
        stale = h
        thread = Thread.new do
          gate.pop
          stale.edge(777)
          [:tick, :reserve].each do |m|
            stale.public_send(m, *((m == :reserve) ? [1] : []))
          rescue Exception => e # rubocop:disable Lint/RescueException
            seen_errors << e
          end
          stale.edge(778)
        end
        thread.report_on_exception = false
      else
        refute_same stale, h
        gate << :go if gate.empty? && !thread.nil? && thread.alive?
        thread.join(1)
      end
    end
    thread.join(2)
    assert_equal 0, report.total_edges, "a stale handle must not record into later runs"
    assert_equal "black-box-uninstrumented", report.coverage_tier
    assert_equal [:retired, :retired], [seen_errors.pop, seen_errors.pop].map { |e| stale.campaign.provenance(e).last }
    assert_equal 0, report.timeouts + report.ooms + report.crashes.size
    assert_equal 20, report.iterations
  end

  def test_a_sentinel_stored_in_one_run_and_reraised_in_another_is_stale
    stash = nil
    report = fz(%w[a b], opts(step_budget: 0, max_iterations: 0)) do |d, h|
      if d == "a"
        begin
          h.tick
        rescue Exception => e # rubocop:disable Lint/RescueException
          stash = e
        end
      else
        raise stash
      end
    end
    assert_equal 1, report.timeouts # "a": swallowed the sentinel, returned
    assert_equal ["stale-budget-sentinel"], report.crashes.map(&:kind)
    assert_equal [], report.crashes.first.frames
  end

  def test_a_retired_sentinel_reraised_in_the_same_epoch_is_stale_too
    out, = one_exec(lambda do |_d, h|
      h.retire
      h.tick
    end)
    assert_equal "stale-budget-sentinel", out.crash_kind
  end

  def test_an_outer_campaigns_sentinel_is_not_claimed_by_an_inner_campaign
    outer = fz(["a"], opts(step_budget: 0, max_iterations: 0)) do |_d, h|
      sentinel = begin
        h.tick
      rescue Exception => e # rubocop:disable Lint/RescueException
        e
      end
      fz(["x"]) { |_, _| raise sentinel }
    end
    assert_equal [1, 0], [outer.timeouts, outer.crashes.size]
  end

  # -- security review regressions (L5a, L5b, L6) --------------------------------

  def test_a_forged_sentinel_is_a_crash_never_a_timeout_and_never_aborts_the_campaign
    [:steps, :memory, :retired].each do |which|
      report = fz(%w[a b], opts(max_iterations: 5)) do |_d, h|
        forged = Fuzz::Handle::BudgetExceeded.new
        forged.instance_variable_set(:@which, which)
        raise forged if h
      end
      assert_equal [0, 0], [report.timeouts, report.ooms], which.to_s
      assert_equal 1, report.crashes.size
      assert_match(/BudgetExceeded/, report.crashes.first.kind)
    end
    assert_raises(ArgumentError) { Fuzz::Handle::BudgetExceeded.new(Object.new, 1, :steps) }
  end

  def test_a_genuine_sentinel_is_recognised_by_identity_and_a_copy_is_not
    out, = one_exec(lambda do |_d, h|
      loop { h.tick }
    rescue Exception => e # rubocop:disable Lint/RescueException
      raise e.dup # a copy is a different object no campaign issued
    end)
    # the handle IS exhausted, so the post-run check still judges it a timeout
    assert_equal :timeout, out.kind
    token = Fuzz::Handle::CampaignToken.new
    other = Fuzz::Handle.new(0, 1, token, 1, nil)
    sentinel = assert_raises(Fuzz::Handle::BudgetExceeded) { other.tick }
    assert_same token, Fuzz::Handle::CampaignToken.owner_of(sentinel)
    assert_nil Fuzz::Handle::CampaignToken.owner_of(sentinel.dup)
    assert_nil token.provenance(sentinel.dup)
  end

  # A non-local exit is the caller's own code transferring control: it propagates
  # to the caller exactly as Ruby defines (VER06i section 5); the ensure only
  # retires the handle.

  def test_a_throw_from_the_target_reaches_the_callers_catch_and_the_handle_is_retired
    stash = nil
    value = catch(:x) do
      fz(["a"]) do |_d, h|
        stash = h
        throw :x, :thrown
      end
      :not_reached
    end
    assert_equal :thrown, value
    assert_predicate stash, :retired?
  end

  def test_break_and_return_out_of_the_target_transfer_control_and_retire_the_handle
    stash = nil
    assert_equal :out, fz(%w[a b]) { |_d, h|
      stash = h
      break :out
    }
    assert_predicate stash, :retired?
    helper = -> { fz(["a"]) { |_d, _h| return :returned } } # rubocop:disable Lint/NonLocalExitFromIterator
    assert_equal :returned, helper.call
  end

  def orphan_break_proc = proc { |_d, _h| break :orphan }

  def test_a_break_from_a_proc_whose_defining_method_returned_is_an_ordinary_crash
    report = Fuzz.run(["a"], OPTS, &orphan_break_proc)
    assert_equal ["LocalJumpError"], report.crashes.map(&:kind)
  end

  # The worker campaigns below are FINITE (300 iterations of ~10 ms), so a regression
  # that ignores the kill or the exit fails these assertions after a few seconds
  # instead of hanging CI.

  def test_thread_kill_of_a_worker_running_a_campaign_stops_it_promptly
    executions = 0
    started = Queue.new
    worker = Thread.new do
      fz(["a"], opts(max_iterations: 300)) do |_d, _h|
        executions += 1
        started << :go
        sleep 0.01
      end
    end
    started.pop
    at_kill = executions
    worker.kill
    assert worker.join(2), "the killed worker must end within 2 s"
    refute_predicate worker, :alive?
    assert_operator executions - at_kill, :<=, 1, "at most the execution in flight may finish after the kill"
  end

  def test_thread_exit_in_a_target_ends_that_worker_thread
    executions = 0
    worker = Thread.new do
      fz(["a"], opts(max_iterations: 300)) do |_d, _h|
        executions += 1
        sleep 0.01
        Thread.exit
      end
      :campaign_returned
    end
    assert worker.join(5), "the worker must end promptly"
    refute_predicate worker, :alive?
    assert_nil worker.value, "Thread.exit must end the thread, not be turned into a verdict"
    assert_operator executions, :<=, 1
  end

  def test_sentinel_provenance_is_weak_so_a_timeout_on_every_execution_stays_bounded
    token = Fuzz::Handle::CampaignToken.new
    4000.times do |i|
      h = Fuzz::Handle.new(0, 1, token, i, nil)
      begin
        h.tick
      rescue Exception # rubocop:disable Lint/RescueException
        nil
      end
      h.retire
    end
    3.times { GC.start }
    assert_operator token.issued_count, :<, 400, "provenance must not keep every sentinel (and its backtrace) alive"
    kept = nil
    h = Fuzz::Handle.new(0, 1, token, 9, nil)
    begin
      h.tick
    rescue Exception => e # rubocop:disable Lint/RescueException
      kept = e
    end
    3.times { GC.start }
    assert_equal [9, :steps], token.provenance(kept), "a sentinel a target holds is still recognised (stale detection)"
  end

  def test_timeout_exit_exception_is_process_owned_even_if_timeout_loads_late
    assert_includes $LOADED_FEATURES.grep(/timeout/).join, "timeout"
    assert_raises(Timeout::ExitException) { fz(["a"]) { |_, _| raise Timeout::ExitException } }
    refute_includes Fuzz::PROCESS_OWNED.map(&:name), "Timeout::ExitException"
  end

  def test_the_target_gets_its_own_copy_of_the_input
    seed = "abc"
    report = fz([seed], opts(max_iterations: 20)) do |d, _|
      d << "zzz"
      d.setbyte(0, 0)
      d.force_encoding("UTF-8")
    end
    assert_equal "abc", seed
    assert_equal "abc", report.corpus.first
  end

  def test_the_target_input_is_binary
    encodings = []
    fz(["é"], opts(max_iterations: 0)) { |d, _| encodings << d.encoding }
    assert_equal [Encoding::BINARY], encodings
  end

  # -- crash classification ---------------------------------------------------------

  def test_a_hostile_exception_class_is_classified_without_calling_its_methods
    Hostile::TOUCHED.clear
    out, = one_exec(->(_d, _h) { raise Hostile })
    assert_equal "crash", out.kind.to_s
    assert_equal "TestFuzzEngine.Hostile", out.crash_kind
    assert_empty Hostile::TOUCHED
    assert(out.frames.all? { |f| f.start_with?("test_fuzz_engine.rb#") }, out.frames.inspect)
    assert_equal 1, fz(["a"], opts(max_iterations: 5)) { |_, _| raise Hostile }.crashes.size
    assert_empty Hostile::TOUCHED
  end

  def test_the_message_is_never_read_and_never_changes_the_signature
    a = one_exec(->(_d, _h) { raise ErrA, "secret-one" }).first
    b = one_exec(->(_d, _h) { raise ErrA, "secret-two" }).first
    # the same class from the same frame has the same signature whatever the message
    assert_equal a.crash_kind, b.crash_kind
    sigs = [a, b].map { |o| Fuzz::Signature.crash_signature(o.crash_kind, o.frames) }
    refute_includes sigs.join, "secret"
    refute_includes (a.frames + [a.crash_kind]).join, "secret"
  end

  def test_anonymous_and_unreadable_exceptions
    anon = Class.new(StandardError)
    out, = one_exec(->(_d, _h) { raise anon })
    assert_equal "<anonymous>", out.crash_kind
    Fuzz::Signature.stub(:normalise_kind, ->(_n) { raise "unreadable" }) do
      assert_equal "<unreadable>", Fuzz::Signature.kind_of_exception(ErrA.new)
    end
    assert_equal [], Fuzz::Signature.frames_of(StandardError.new) # never raised: no backtrace
    assert_equal [], Fuzz::Signature.frames_of(Object.new)        # not an exception: guarded
  end

  def test_frames_are_basename_label_colon_free_and_skip_the_engine
    out, = one_exec(->(_d, _h) { Boom.outer })
    assert_operator out.frames.size, :>=, 2
    out.frames.each do |f|
      refute_includes f, ":"
      refute_match(/[\/\\]/, f.split("#", 2).first)
      refute_includes f, "fuzz/"
    end
    assert_match(/\Atest_fuzz_engine\.rb#/, out.frames.first)
    refute(out.frames.any? { |f| f.include?("run_target") || f.start_with?("engine.rb", "handle.rb") })
  end

  def test_recursion_collapses_so_depth_does_not_change_the_signature
    a = one_exec(->(_d, _h) { Boom.go(3) }).first
    b = one_exec(->(_d, _h) { Boom.go(40) }).first
    assert_equal Fuzz::Signature.crash_signature(a.crash_kind, a.frames), Fuzz::Signature.crash_signature(b.crash_kind, b.frames)
  end

  def test_frame_walk_is_capped_and_frames_are_bounded
    chain = ->(n) { n.zero? ? raise(ArgumentError) : chain.call(n - 1) }
    out, = one_exec(->(_d, _h) { chain.call(300) })
    assert_operator out.frames.size, :<=, Fuzz::Limits::MAX_FRAMES
  end

  def test_an_unrescued_throw_is_a_crash
    out, = one_exec(->(_d, _h) { throw :nobody })
    assert_equal "UncaughtThrowError", out.crash_kind
  end

  def test_engine_and_library_paths_are_plumbing
    assert Fuzz::Signature.plumbing?(nil)
    assert Fuzz::Signature.plumbing?("<internal:kernel>")
    assert Fuzz::Signature.plumbing?("(eval at x:1)")
    assert Fuzz::Signature.plumbing?(File.join(Fuzz::Signature::ENGINE_DIR, "fuzz/engine.rb"))
    assert Fuzz::Signature.plumbing?(File.join(RbConfig::CONFIG["rubylibdir"], "set.rb"))
    refute Fuzz::Signature.plumbing?(__FILE__)
  end

  # -- caps ----------------------------------------------------------------------------

  def test_input_validation
    assert_raises(ArgumentError) { Fuzz.run(["a"], OPTS) }
    assert_raises(TypeError) { Fuzz.run(["a"], 5) { |_, _| } }
    assert_raises(TypeError) { Fuzz.run("a", OPTS) { |_, _| } }
    assert_raises(TypeError) { Fuzz.run([1], OPTS) { |_, _| } }
    assert_raises(TypeError) { Fuzz.run(["a"], OPTS, coverage: :branches) { |_, _| } }
    assert_raises(TypeError) { Fuzz.run_outcomes(["a"], OPTS, 5) }
    assert_raises(ArgumentError) { Fuzz.run(Array.new(10_001, "a"), OPTS) { |_, _| } }
    assert_raises(ArgumentError) { Fuzz.run(["a" * ((1 << 20) + 1)], OPTS) { |_, _| } }
    assert_raises(TypeError) { Fuzz.run_outcomes(["a"], OPTS, ->(_d, _c) { :not_an_outcome }) }
    assert_equal 0, Fuzz.run_outcomes(["a"], opts(max_iterations: 0), ->(_d, _c) { Fuzz::Outcome.pass }).crashes.size
  end

  def test_distinct_crashes_are_capped
    exec = ->(d, _c) { Fuzz::Outcome.crash("k#{d.unpack1("H*")}", []) }
    seeds = (0...1002).map { |i| [i].pack("n") }
    report = Fuzz.run_outcomes(seeds, opts(max_iterations: 0), exec)
    assert_equal 1000, report.crashes.size
    assert_equal 2, report.crashes_dropped
    assert_equal 1002, report.unique_crashes
    refute_predicate report, :clean?
  end

  def test_the_retained_bytes_cap_drops_admissions_and_crash_inputs_visibly
    seeds = (1..10).map { |i| i.to_s * 10 }
    report = Fuzz.run_outcomes(seeds, opts(max_iterations: 0), ->(_d, c) {
      c.record(0)
      Fuzz::Outcome.pass
    }, retained_cap: 25)
    assert_equal 1, report.admitted_seeds # only the first seed adds an edge (the rest are redundant)
    exec = lambda do |d, c|
      c.record(d.getbyte(0))
      Fuzz::Outcome.pass
    end
    capped = Fuzz.run_outcomes(seeds.map { |s| s.b }, opts(max_iterations: 0), exec, retained_cap: 25)
    assert_equal 2, capped.admitted_seeds # 10 + 10 bytes fit; the third would pass 25
    assert_equal 7, capped.admissions_dropped
    crash_exec = ->(_d, _c) { Fuzz::Outcome.crash("k", []) }
    r = Fuzz.run_outcomes(["abcdef"], opts(minimize_budget: 0), crash_exec, retained_cap: 3)
    rec = r.crashes.first
    assert_predicate rec, :input_dropped?
    refute_predicate rec, :minimized?
    assert_equal "", rec.minimal_input
    assert_equal 1, rec.hits
  end

  def test_a_shorter_crashing_input_replaces_the_retained_one_and_adjusts_the_account
    sizes = [5, 3, 4]
    exec = ->(_d, _c) { Fuzz::Outcome.crash("k", []) }
    report = Fuzz.run_outcomes(sizes.map { |n| "a" * n }, opts(max_iterations: 0, minimize_budget: 0), exec)
    assert_equal 3, report.crashes.first.hits
    assert_equal 3, report.crashes.first.minimal_input.bytesize
  end

  # -- TAP from a real campaign -----------------------------------------------------------

  def test_a_real_campaigns_tap_lines_carry_no_input
    report = fz(["SECRETSEED"], opts(max_iterations: 0)) { |d, _| raise ErrA, d }
    lines = Fuzz::Signature.tap_failures(report, "my_target")
    assert_equal 1, lines.size
    assert_match(/\Anot ok 1 - my_target \[fuzz-crash:[0-9a-f]{8}\]\z/, lines.first)
    refute_includes lines.first, "SECRET"
    refute_includes report.inspect, "SECRET"
    refute_includes report.crashes.first.inspect, "SECRET"
    t = fz(["a"], opts(max_iterations: 0, step_budget: 0)) { |_, h| h.tick }
    assert_equal ["not ok 1 - x [fuzz-timeout:4e61ea76]"], Fuzz::Signature.tap_failures(t, "x")
  end

  def test_edge_ids_are_stable_fnv1a
    assert_equal Prng.fnv1a64("parse:ok"), Fuzz::Handle.edge_id("parse:ok")
    assert_equal 16_654_208_175_385_433_931, Fuzz::Handle.edge_id("abc")
  end
end
