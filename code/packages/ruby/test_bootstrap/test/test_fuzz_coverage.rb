# frozen_string_literal: true

require_relative "test_helper"
require "open3"
require "tmpdir"

# The coverage source (VER06i section 4.2).  Under SimpleCov the stdlib Coverage
# module is already running, so (a) the source's logic is tested in-process
# against an injected fake backend, and (b) the REAL Coverage behaviour is tested
# in subprocesses that load no SimpleCov.
class TestFuzzCoverage < Minitest::Test
  include CodingAdventures::TestBootstrap

  # A scripted stand-in for the stdlib Coverage module.
  class FakeBackend
    attr_accessor :available, :running, :setup_raises, :on_read, :instrumented
    attr_reader :calls

    def initialize
      @available = true
      @running = false
      @setup_raises = false
      @calls = []
      @instrumented = []
      @on_read = -> { {} }
    end

    def available? = @available

    def running? = @running

    def setup
      @calls << :setup
      raise "coverage measurement is already setup" if @setup_raises
    end

    def resume = (@calls << :resume) && (@running = true)

    def instrumented_files = @instrumented

    def read_and_clear = (@calls << :read) && @on_read.call

    def stop = (@calls << :stop) && (@running = false) || {}
  end

  BR = [:if, 0, 3, 4, 7, 7].freeze
  THEN = [:then, 1, 4, 6, 4, 11].freeze
  ELSE = [:else, 2, 6, 6, 6, 10].freeze

  def source(backend, **kw) = Fuzz::CoverageSource.branches(backend: backend, **kw)

  def with_target
    Dir.mktmpdir("fzcov") do |dir|
      path = File.join(dir, "fz_loaded_target.rb")
      File.write(path, "module FzLoadedTarget; MARK = :loaded; end\n")
      yield dir, path
    end
  ensure
    Object.send(:remove_const, :FzLoadedTarget) if Object.const_defined?(:FzLoadedTarget)
  end

  def edge_of(base, branch, target) = Prng.fnv1a64("#{base}:#{branch}:#{target}") % 65_536

  # -- mode decision -----------------------------------------------------------

  def test_mode_is_branches_when_coverage_is_free
    b = FakeBackend.new
    s = source(b)
    assert_nil s.mode
    assert_equal :branches, s.start
    assert_equal [:setup, :resume], b.calls
    assert_equal :branches, s.start # idempotent
    assert_equal [:setup, :resume], b.calls
  end

  def test_an_already_running_coverage_is_never_touched
    b = FakeBackend.new
    b.running = true
    s = source(b)
    assert_equal :fallback_active_coverage, s.start
    bitmap = Fuzz::Bitmap.new
    assert_equal 0, s.collect(bitmap)
    s.finish
    assert_equal [], b.calls, "setup/read/stop must never be called on someone else's run"
  end

  def test_an_already_setup_but_not_running_coverage_is_a_fallback_too
    b = FakeBackend.new
    b.setup_raises = true
    s = source(b)
    assert_equal :fallback_active_coverage, s.start
    assert_equal [:setup], b.calls
    s.finish
    assert_equal [:setup], b.calls # finish never stops what it did not start
  end

  def test_an_unavailable_coverage_is_a_fallback
    b = FakeBackend.new
    b.available = false
    s = source(b)
    assert_equal :fallback_unavailable, s.start
    assert_equal 0, s.collect(Fuzz::Bitmap.new)
    assert_equal [], b.calls
  end

  def test_target_files_are_loaded_after_setup_and_before_resume_in_every_mode
    [[:branches, {}], [:fallback_active_coverage, {running: true}], [:fallback_unavailable, {available: false}]].each do |mode, cfg|
      with_target do |_dir, path|
        b = FakeBackend.new
        cfg.each { |k, v| b.public_send(:"#{k}=", v) }
        order = []
        b.define_singleton_method(:setup) { order << [:setup, Object.const_defined?(:FzLoadedTarget)] }
        b.define_singleton_method(:resume) { order << [:resume, Object.const_defined?(:FzLoadedTarget)] }
        assert_equal mode, source(b, load: [path]).start
        assert Object.const_defined?(:FzLoadedTarget), mode.to_s
        next unless mode == :branches

        assert_equal [[:setup, false], [:resume, true]], order
      end
    end
  end

  def test_a_failing_load_stops_what_it_started_and_reraises
    b = FakeBackend.new
    s = source(b, load: ["/no/such/fz_target.rb"])
    assert_raises(LoadError) { s.start }
    assert_equal [:setup, :stop], b.calls
    b2 = FakeBackend.new
    b2.running = true
    assert_raises(LoadError) { source(b2, load: ["/no/such/fz_target.rb"]).start }
    assert_equal [], b2.calls
  end

  def test_finish_is_idempotent_and_only_stops_a_run_it_started
    b = FakeBackend.new
    s = source(b)
    s.finish # before start
    s.start
    s.finish
    s.finish
    assert_equal [:setup, :resume, :stop], b.calls
  end

  def test_argument_validation
    assert_raises(TypeError) { Fuzz::CoverageSource.branches(load: "a.rb") }
    assert_raises(TypeError) { Fuzz::CoverageSource.branches(load: [1]) }
    assert_raises(TypeError) { Fuzz::CoverageSource.branches(include: "a") }
    assert_raises(TypeError) { Fuzz::CoverageSource.branches(include: [1]) }
  end

  # -- security review regressions (M1, M2, M3, L2, L3) -------------------------------

  def counting_file(dir, name = "fz_counting.rb")
    path = File.join(dir, name)
    File.write(path, "Thread.current[:fz_loads] = (Thread.current[:fz_loads] || 0) + 1\n")
    path
  end

  def test_a_fallback_never_reloads_a_file_that_is_already_loaded
    Thread.current[:fz_loads] = 0
    Dir.mktmpdir("fzcov") do |dir|
      path = counting_file(dir)
      require path
      assert_equal 1, Thread.current[:fz_loads]
      b = FakeBackend.new
      b.running = true # another tool's Coverage is active
      source(b, load: [path]).start
      assert_equal 1, Thread.current[:fz_loads], "reloading would reset the other tool's counters and re-run side effects"
      b2 = FakeBackend.new
      b2.available = false
      source(b2, load: [path]).start
      assert_equal 1, Thread.current[:fz_loads]
      # a file not yet loaded IS loaded once, by require
      fresh = counting_file(dir, "fz_fresh.rb")
      source(b, load: [fresh]).start
      assert_equal 2, Thread.current[:fz_loads]
      source(b, load: [fresh]).start
      assert_equal 2, Thread.current[:fz_loads]
      # a source that owns a fresh setup loads (the file must be compiled under Coverage)
      source(FakeBackend.new, load: [path]).start
      assert_equal 3, Thread.current[:fz_loads]
    end
  ensure
    Thread.current[:fz_loads] = nil
  end

  def test_load_and_include_entries_are_expanded_when_the_source_is_built
    Dir.mktmpdir("fzcov") do |dir|
      counting_file(dir)
      src = Dir.chdir(dir) { source(FakeBackend.new, load: ["fz_counting.rb"], include: ["."]) }
      Thread.current[:fz_loads] = 0
      src.start # the cwd is no longer dir, and $LOAD_PATH has no dir: only the expanded path works
      assert_equal 1, Thread.current[:fz_loads]
      assert_equal [], src.excluded_files
      b = FakeBackend.new
      b.on_read = -> { {File.join(File.realpath(dir), "t.rb") => {branches: {BR => {THEN => 1}}}} }
      s2 = Dir.chdir(dir) { source(b, include: ["."]) }
      s2.start
      bitmap = Fuzz::Bitmap.new
      assert_equal 1, s2.collect(bitmap) if File.realpath(dir) == File.expand_path(dir)
    end
  ensure
    Thread.current[:fz_loads] = nil
  end

  def test_the_include_filter_respects_separators
    keep = File.join(Dir.tmpdir, "fz_a", "lib", "x.rb")
    drop = File.join(Dir.tmpdir, "fz_a", "lib_other", "x.rb")
    exact = File.join(Dir.tmpdir, "fz_a", "lib")
    one = {branches: {BR => {THEN => 1}}}
    n, = collect({keep => one, drop => one, exact => one}, include: [File.join(Dir.tmpdir, "fz_a", "lib")])
    assert_equal 2, n # the directory's own file and the path itself, never lib_other
    n, = collect({keep => one, drop => one}, include: [File.join(Dir.tmpdir, "fz_a", "lib") + "/"])
    assert_equal 1, n
  end

  def test_a_failed_start_does_not_leave_a_stale_mode
    Dir.mktmpdir("fzcov") do |dir|
      path = File.join(dir, "fz_late.rb")
      b = FakeBackend.new
      s = source(b, load: [path])
      assert_raises(LoadError) { s.start }
      assert_nil s.mode
      File.write(path, "FZ_LATE = 1\n")
      assert_equal :branches, s.start
      Object.send(:remove_const, :FZ_LATE)
    end
  end

  def test_status_reports_files_that_are_plumbing_and_never_measured
    lib = File.join(RbConfig::CONFIG["rubylibdir"], "set.rb")
    s = source(FakeBackend.new, load: [lib])
    assert_equal [lib], s.excluded_files
    assert s.status[:all_excluded]
    assert_equal 1, s.status[:files]
    with_target do |_dir, path|
      mixed = source(FakeBackend.new, load: [lib, path])
      refute mixed.status[:all_excluded]
      assert_equal [lib], mixed.status[:excluded]
    end
    refute source(FakeBackend.new).status[:all_excluded]
  end

  def test_a_target_that_consumes_coverage_degrades_the_source_instead_of_aborting
    b = FakeBackend.new
    reads = 0
    b.on_read = lambda do
      reads += 1
      raise "coverage measurement is not enabled" if reads > 3 # the target stopped it

      {File.join(Dir.tmpdir, "c.rb") => {branches: {[:if, reads, 9, 4, 12, 7] => {[:then, 4, 10, 6, 10, 11] => 1}}}}
    end
    b.define_singleton_method(:stop) { raise "coverage measurement is not enabled" }
    s = source(b)
    report = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 30, seed: 3), coverage: s) { |d, h| h.edge(d.getbyte(0) & 1) }
    assert_instance_of Fuzz::Report, report
    assert_predicate s, :degraded?
    assert_equal :fallback_degraded, s.mode
    assert_equal "caller-reported", report.coverage_tier, "a degraded source is never labelled as real coverage"
    assert_equal 30, report.iterations
    assert_equal 4, reads # three good reads, one failure, none after
    assert_nil s.finish # finish never raises, never masks
    black = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 5), coverage: source(FakeBackend.new.tap { |x| x.on_read = -> { raise "gone" } })) { |_, _| }
    assert_equal "black-box-uninstrumented", black.coverage_tier
  end

  def test_a_malformed_result_degrades_too
    b = FakeBackend.new
    b.on_read = -> {}
    s = source(b)
    s.start
    assert_equal 0, s.collect(Fuzz::Bitmap.new)
    assert_predicate s, :degraded?
    assert_equal [:setup, :resume, :read, :stop], b.calls
  end

  # -- second review (F4-F10) ----------------------------------------------------------

  def test_a_result_that_lacks_a_loaded_file_means_the_run_was_restarted_and_is_not_stopped
    with_target do |_dir, path|
      b = FakeBackend.new
      b.on_read = -> { {"/somewhere/else.rb" => {lines: [1]}} } # restarted without branches: our file is gone
      s = source(b, load: [path])
      s.start
      assert_equal 0, s.collect(Fuzz::Bitmap.new)
      assert_predicate s, :degraded?
      refute_includes b.calls, :stop, "a run that is no longer ours must not be stopped"
      s.finish
      refute_includes b.calls, :stop
    end
  end

  def test_a_result_with_no_branch_data_for_a_loaded_file_degrades_too
    with_target do |_dir, path|
      b = FakeBackend.new
      b.on_read = -> { {path => {lines: [1]}} }
      s = source(b, load: [path])
      s.start
      s.collect(Fuzz::Bitmap.new)
      assert_predicate s, :degraded?
    end
  end

  def test_the_tier_is_judged_by_the_latest_executions_not_a_sticky_flag
    with_target do |_dir, path|
      b = FakeBackend.new
      reads = 0
      # real edges for the first 3 reads, then the target clears the counters itself:
      # every read succeeds, with the file present and every counter zero.
      b.on_read = lambda do
        reads += 1
        {path => {branches: {BR => {THEN => (reads <= 3) ? 1 : 0}}}}
      end
      s = source(b, load: [path])
      report = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 60, seed: 1), coverage: s) { |_, _| }
      assert_equal "black-box-uninstrumented", report.coverage_tier
      refute_predicate s, :degraded?
      assert_equal 0, report.admitted_seeds - 1 # only the first seed ever found an edge
    end
  end

  def test_the_tier_is_real_while_edges_keep_arriving
    with_target do |_dir, path|
      b = FakeBackend.new
      n = 0
      b.on_read = -> { {path => {branches: {BR => {THEN => 1, ELSE => ((n += 1) % 5).zero? ? 1 : 0}}}} }
      report = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 40, seed: 1), coverage: source(b, load: [path])) { |_, _| }
      assert_equal "ruby-coverage-branches", report.coverage_tier
    end
  end

  def test_a_degrade_stops_the_run_only_if_it_is_still_running
    b = FakeBackend.new
    b.on_read = -> { raise "boom" }
    s = source(b)
    s.start
    b.running = false # the target stopped it
    s.collect(Fuzz::Bitmap.new)
    refute_includes b.calls, :stop
    b2 = FakeBackend.new
    b2.on_read = -> { raise "boom" }
    s2 = source(b2)
    s2.start
    s2.collect(Fuzz::Bitmap.new) # still running: ours, so it is stopped
    assert_equal :stop, b2.calls.last
  end

  def test_a_fallback_skips_files_that_a_running_coverage_already_instrumented
    Dir.mktmpdir("fzcov") do |dir|
      path = counting_file(dir, "fz_instr.rb")
      b = FakeBackend.new
      b.running = true
      b.instrumented = [path]
      Thread.current[:fz_loads] = 0
      source(b, load: [path]).start
      assert_equal 0, Thread.current[:fz_loads], "an already-instrumented file is never executed again"
    end
  ensure
    Thread.current[:fz_loads] = nil
  end

  def test_a_symlinked_path_of_a_loaded_feature_is_not_executed_again
    Dir.mktmpdir("fzcov") do |dir|
      real = counting_file(dir, "fz_real.rb")
      link = File.join(dir, "fz_link.rb")
      File.symlink(real, link)
      Thread.current[:fz_loads] = 0
      require File.realpath(real)
      assert_equal 1, Thread.current[:fz_loads]
      b = FakeBackend.new
      b.running = true
      source(b, load: [link]).start
      assert_equal 1, Thread.current[:fz_loads]
    end
  ensure
    Thread.current[:fz_loads] = nil
  end

  def test_a_fallback_loads_a_non_rb_file_instead_of_failing_on_require
    Dir.mktmpdir("fzcov") do |dir|
      path = File.join(dir, "fz_script.txt")
      File.write(path, "Thread.current[:fz_loads] = (Thread.current[:fz_loads] || 0) + 1\n")
      Thread.current[:fz_loads] = 0
      b = FakeBackend.new
      b.running = true
      source(b, load: [path]).start # require would raise LoadError for a .txt name
      assert_equal 1, Thread.current[:fz_loads]
    end
  ensure
    Thread.current[:fz_loads] = nil
  end

  # -- edge derivation -----------------------------------------------------------

  def collect(result, **kw)
    b = FakeBackend.new
    b.on_read = -> { result }
    s = source(b, **kw)
    s.start
    bitmap = Fuzz::Bitmap.new
    n = s.collect(bitmap)
    [n, bitmap.covered_edges]
  end

  def test_edges_are_fnv_of_basename_branch_and_target_mod_65536
    path = File.join(Dir.tmpdir, "fz_a", "target.rb")
    n, edges = collect({path => {branches: {BR => {THEN => 3, ELSE => 0}}}})
    assert_equal 1, n
    assert_equal [edge_of("target.rb", "if@3:4-7:7", "then@4:6-4:11")], edges
    # the pinned value, so a change to the derivation is a visible break
    assert_equal Prng.fnv1a64("target.rb:if@3:4-7:7:then@4:6-4:11") % 65_536, edges.first
  end

  def test_ids_use_the_basename_only_and_ignore_the_internal_counter
    a = collect({File.join(Dir.tmpdir, "one", "t.rb") => {branches: {[:if, 0, 3, 4, 7, 7] => {[:then, 1, 4, 6, 4, 11] => 1}}}})
    b = collect({File.join(Dir.tmpdir, "two", "deep", "t.rb") => {branches: {[:if, 9, 3, 4, 7, 7] => {[:then, 12, 4, 6, 4, 11] => 5}}}})
    assert_equal a, b
    c = collect({File.join(Dir.tmpdir, "one", "u.rb") => {branches: {[:if, 0, 3, 4, 7, 7] => {[:then, 1, 4, 6, 4, 11] => 1}}}})
    refute_equal a.last, c.last
    d = collect({File.join(Dir.tmpdir, "one", "t.rb") => {branches: {[:if, 0, 3, 4, 7, 7] => {[:else, 1, 4, 6, 4, 11] => 1}}}})
    refute_equal a.last, d.last
  end

  def test_zero_counters_and_missing_branch_data_record_nothing
    path = File.join(Dir.tmpdir, "t.rb")
    assert_equal [0, []], collect({path => {branches: {BR => {THEN => 0, ELSE => 0}}}})
    assert_equal [0, []], collect({path => {lines: [1, 2]}})
    assert_equal [0, []], collect({})
  end

  def test_only_target_code_is_measured
    good = File.join(Dir.tmpdir, "good.rb")
    one = {BR => {THEN => 1}}
    result = {
      good => {branches: one},
      File.join(Fuzz::Signature::ENGINE_DIR, "fuzz/engine.rb") => {branches: one},
      File.join(RbConfig::CONFIG["rubylibdir"], "set.rb") => {branches: one},
      "<internal:kernel>" => {branches: one},
      "(eval at x.rb:1)" => {branches: one}
    }
    n, edges = collect(result)
    assert_equal 1, n
    assert_equal [edge_of("good.rb", "if@3:4-7:7", "then@4:6-4:11")], edges
  end

  def test_the_include_filter_keeps_only_matching_prefixes
    a = File.join(Dir.tmpdir, "fz_inc", "a.rb")
    b = File.join(Dir.tmpdir, "fz_exc", "b.rb")
    result = {a => {branches: {BR => {THEN => 1}}}, b => {branches: {BR => {THEN => 1}}}}
    n, = collect(result, include: [File.join(Dir.tmpdir, "fz_inc")])
    assert_equal 1, n
    assert_equal 2, collect(result)[0]
  end

  def test_the_path_cache_is_bounded
    result = (0..10_005).to_h { |i| ["/fz/cache/f#{i}.rb", {branches: {}}] }
    assert_equal 0, collect(result)[0]
  end

  # -- inside a campaign ------------------------------------------------------------

  def test_a_campaign_with_real_edges_is_labelled_ruby_coverage_branches_and_stops_coverage
    b = FakeBackend.new
    n = 0
    b.on_read = lambda do
      n += 1
      {File.join(Dir.tmpdir, "c.rb") => {branches: {BR => {THEN => 1}, [:if, 3, 9, 4, 12, 7] => {[:then, 4, 10, 6, 10, 11] => (n % 3).zero? ? 1 : 0}}}}
    end
    report = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 30, seed: 3), coverage: source(b)) { |_, _| }
    assert_equal "ruby-coverage-branches", report.coverage_tier
    assert_equal 2, report.total_edges
    assert_equal 2, report.admitted_seeds
    assert_equal :stop, b.calls.last
    assert_equal 1, b.calls.count(:stop)
    assert_equal 31, b.calls.count(:read) # every execution is read (and cleared) once
  end

  def test_a_fallback_campaign_uses_caller_reported_edges
    b = FakeBackend.new
    b.running = true
    report = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 20), coverage: source(b)) { |d, h| h.edge(d.getbyte(0) & 1) }
    assert_equal "caller-reported", report.coverage_tier
    assert_equal [], b.calls
  end

  def test_a_source_without_any_edge_is_black_box
    b = FakeBackend.new
    report = Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 5), coverage: source(b)) { |_, _| }
    assert_equal "black-box-uninstrumented", report.coverage_tier
  end

  def test_coverage_is_stopped_even_when_the_campaign_raises
    b = FakeBackend.new
    assert_raises(Interrupt) { Fuzz.run(["a"], Fuzz::Options.new(max_iterations: 5), coverage: source(b)) { |_, _| raise Interrupt } }
    assert_equal :stop, b.calls.last
  end

  def test_the_real_backend_reports_itself_unavailable_or_available_without_touching_a_running_run
    # SimpleCov is running here: the real backend must say so (and the source fall back).
    assert Coverage.running?
    s = Fuzz::CoverageSource.branches
    assert_equal :fallback_active_coverage, s.start
    assert Coverage.running?, "the source must leave SimpleCov's run alone"
    assert_equal 0, s.collect(Fuzz::Bitmap.new)
    s.finish
    assert Coverage.running?
  end

  # -- the real Coverage module, in subprocesses ---------------------------------------

  TARGET = <<~RUBY
    module Target
      def self.check(s)
        if s.getbyte(0) == 35
          if s.getbyte(1) == 128
            raise ArgumentError, "deep" if s.getbyte(2) == 255
            :two
          else
            :one
          end
        else
          :zero
        end
      end
    end
  RUBY

  def ruby(dir, script)
    File.write(File.join(dir, "target.rb"), TARGET)
    File.write(File.join(dir, "drv.rb"), "require \"coding_adventures_test_bootstrap\"\ninclude CodingAdventures::TestBootstrap\n#{script}")
    env = {"RUBYOPT" => nil, "RUBYLIB" => nil, "BUNDLE_GEMFILE" => nil}
    out, status = Open3.capture2e(env, RbConfig.ruby, "-I", Fx.lib_dir, File.join(dir, "drv.rb"))
    assert status.success?, out
    out.lines.map(&:chomp)
  end

  def test_real_branch_coverage_discovers_edges_admits_seeds_and_cracks_a_staircase_with_no_caller_edges
    Dir.mktmpdir("fzreal") do |dir|
      out = ruby(dir, <<~RUBY)
        src = Fuzz::CoverageSource.branches(load: [File.join(__dir__, "target.rb")])
        r = Fuzz.run(["\\x00\\x00\\x00".b], Fuzz::Options.new(seed: 1, max_iterations: 6000), coverage: src) { |d, _h| Target.check(d) }
        puts [src.mode, r.coverage_tier, r.total_edges, r.admitted_seeds, r.crashes.size, r.crashes.first&.kind, Coverage.running?].join(",")
        blind = Fuzz.run(["\\x00\\x00\\x00".b], Fuzz::Options.new(seed: 1, max_iterations: 6000)) { |d, _h| Target.check(d) }
        puts [blind.coverage_tier, blind.total_edges, blind.crashes.size].join(",")
      RUBY
      mode, tier, edges, admitted, crashes, kind, running = out[0].split(",")
      assert_equal "branches", mode
      assert_equal "ruby-coverage-branches", tier
      assert_operator edges.to_i, :>=, 5
      assert_operator admitted.to_i, :>, 1, "distinct branches must admit new seeds"
      assert_equal "1", crashes
      assert_equal "ArgumentError", kind
      assert_equal "false", running, "the engine must stop the Coverage run it started"
      # the same target, loaded without a source, has no edges: blind mutation does not crack it
      assert_equal "black-box-uninstrumented,0,0", out[1]
    end
  end

  def test_real_edges_are_exactly_the_branches_of_the_target_and_exclude_stdlib_and_the_engine
    Dir.mktmpdir("fzreal") do |dir|
      out = ruby(dir, <<~RUBY)
        src = Fuzz::CoverageSource.branches(load: [File.join(__dir__, "target.rb")])
        src.start
        Target.check("#\\x80\\x00".b)
        require "securerandom"
        SecureRandom.hex(4)
        Fuzz::Mutate.bit_flip("ab", 3, 1)
        bm = Fuzz::Bitmap.new
        first = src.collect(bm)
        second = src.collect(Fuzz::Bitmap.new)
        puts [first, second, bm.covered_edges.join(" ")].join(",")
        src.finish
      RUBY
      first, second, edges = out.last.split(",")
      assert_equal "3", first, "outer-then, mid-then and modifier-else only; nothing from SecureRandom or the engine"
      assert_equal "0", second, "the counters are zeroed after every read"
      assert_equal 3, edges.split.size
      assert(edges.split.all? { |e| e.to_i.between?(0, 65_535) })
    end
  end

  def test_edge_ids_do_not_depend_on_the_directory
    ids = %w[one two].map do |name|
      Dir.mktmpdir(name) do |dir|
        ruby(dir, <<~RUBY).last
          src = Fuzz::CoverageSource.branches(load: [File.join(__dir__, "target.rb")])
          src.start
          Target.check("#\\x80\\x00".b)
          bm = Fuzz::Bitmap.new
          src.collect(bm)
          puts bm.covered_edges.join(" ")
          src.finish
        RUBY
      end
    end
    assert_equal ids[0], ids[1]
    refute_empty ids[0]
  end

  def test_a_target_that_stops_coverage_or_changes_directory_still_yields_a_report_or_edges
    Dir.mktmpdir("fzreal") do |dir|
      out = ruby(dir, <<~RUBY)
        stopper = File.join(__dir__, "stopper.rb")
        File.write(stopper, "module Stopper; def self.go(s, kill); Dir.chdir('/') if kill == :cd; Coverage.result if kill == :stop; s.getbyte(0) == 1 ? :a : :b; end; end\n")
        res = {}
        [:cd, :stop].each do |kill|
          src = Fuzz::CoverageSource.branches(load: [stopper])
          n = 0
          r = Fuzz.run(["\x00".b], Fuzz::Options.new(seed: 1, max_iterations: 40), coverage: src) { |d, h| n += 1; h.edge(1) if n > 1; Stopper.go(d, (n == 2) ? kill : :none) }
          res[kill] = [r.coverage_tier, r.iterations, src.mode, src.degraded?, Coverage.running?]
          Dir.chdir(__dir__)
        end
        puts res[:cd].join(","), res[:stop].join(",")
      RUBY
      cd, stop = out.last(2).map { |l| l.split(",") }
      assert_equal ["ruby-coverage-branches", "40", "branches", "false", "false"], cd, "a chdir in the target must not hide its edges"
      assert_equal ["caller-reported", "40", "fallback_degraded", "true", "false"], stop
    end
  end

  def test_a_target_that_clears_or_restarts_coverage_is_not_labelled_as_real_coverage
    Dir.mktmpdir("fzreal") do |dir|
      out = ruby(dir, <<~RUBY)
        file = File.join(__dir__, "tricky.rb")
        File.write(file, "module Tricky; def self.go(s, mode); r = (s.getbyte(0) == 1 ? :a : :b); if mode == :clear then Coverage.result(stop: false, clear: true) elsif mode == :restart then Coverage.result; Coverage.start(lines: true) end; r; end; end\n")
        [:clear, :restart].each do |mode|
          src = Fuzz::CoverageSource.branches(load: [file])
          n = 0
          r = Fuzz.run(["\x00".b], Fuzz::Options.new(seed: 1, max_iterations: 100), coverage: src) { |d, _h| n += 1; Tricky.go(d, (n > 1) ? mode : :none) }
          puts [mode, r.coverage_tier, src.degraded?, r.iterations].join(",")
          Coverage.result if Coverage.running?
        end
      RUBY
      clear, restart = out.last(2).map { |l| l.split(",") }
      assert_equal ["clear", "black-box-uninstrumented"], clear.first(2)
      assert_equal ["restart", "black-box-uninstrumented", "true", "100"], restart
    end
  end

  def test_a_coverage_run_started_by_someone_else_is_left_intact
    Dir.mktmpdir("fzreal") do |dir|
      out = ruby(dir, <<~RUBY)
        require "coverage"
        Coverage.start(branches: true)
        src = Fuzz::CoverageSource.branches(load: [File.join(__dir__, "target.rb")])
        r = Fuzz.run(["\\x00".b], Fuzz::Options.new(max_iterations: 50), coverage: src) { |d, h| Target.check(d); h.edge(1) }
        counted = Coverage.peek_result.values.sum { |v| v[:branches].values.sum { |t| t.values.sum } }
        puts [src.mode, r.coverage_tier, Coverage.running?, counted.positive?].join(",")
      RUBY
      assert_equal "fallback_active_coverage,caller-reported,true,true", out.last
    end
  end
end
