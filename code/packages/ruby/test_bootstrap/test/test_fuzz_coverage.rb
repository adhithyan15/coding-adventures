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
    attr_accessor :available, :running, :setup_raises, :on_read
    attr_reader :calls

    def initialize
      @available = true
      @running = false
      @setup_raises = false
      @calls = []
      @on_read = -> { {} }
    end

    def available? = @available

    def running? = @running

    def setup
      @calls << :setup
      raise "coverage measurement is already setup" if @setup_raises
    end

    def resume = @calls << :resume

    def read_and_clear = (@calls << :read) && @on_read.call

    def stop = (@calls << :stop) && {}
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
