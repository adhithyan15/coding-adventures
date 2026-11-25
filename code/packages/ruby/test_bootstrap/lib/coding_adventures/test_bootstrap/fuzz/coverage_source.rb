# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # Real, in-process edges from Ruby's stdlib `Coverage` module with branch
      # coverage (VER06 section 7, VER06i section 4.2).
      #
      #   source = Fuzz::CoverageSource.branches(load: ["/abs/path/to/target.rb"])
      #   Fuzz.run(seeds, opts, coverage: source) { |data, _h| Target.parse(data) }
      #
      # How it works.  `start` calls Coverage.setup(branches: true), `load`s the
      # target files (Coverage instruments only files compiled AFTER setup, and a
      # reload would reset their counters: see lessons.d "Ruby Coverage reload
      # resets per-file counters", so the files are loaded exactly once, here), then
      # resumes counting.  After every execution `collect` reads the counters and
      # zeroes them (`Coverage.result(stop: false, clear: true)`): what it reads is
      # exactly that execution's trace.  Each counter above zero is an edge:
      #
      #   edge = fnv1a64("<basename>:<branch>:<target>") mod 65536
      #
      # with branch and target spelled `<type>@<line>:<col>-<line>:<col>`.  The id
      # uses the basename and source positions only: never an absolute path and
      # never Ruby's internal branch counter, so it is stable across machines.
      #
      # Coexistence.  Coverage is process-global and clearing it would zero another
      # tool's counters (SimpleCov starts it first).  So the source NEVER touches a
      # run it did not start: if Coverage is already running or set up, or is not
      # available, it falls back to caller-reported edges (Handle#edge) and says so
      # in #mode.  In a fallback mode the target files are `require`d (never
      # `load`ed: reloading a file that is already loaded would reset the other
      # tool's counters and re-run its top-level side effects), so the target still
      # exists but nothing is reloaded.  Only a source that owns a fresh Coverage
      # setup `load`s.
      #
      # Every `load:` and `include:` entry is expanded to an absolute path when the
      # source is built, so a target that calls Dir.chdir, or a $LOAD_PATH search,
      # cannot change what is loaded or measured.  Files under the interpreter's
      # library directories or Gem.path are plumbing and never measured
      # (`excluded_files`, `status`).
      #
      # If the target itself stops, suspends or reads Coverage (Coverage.result,
      # Coverage.stop), reading fails: the source degrades to caller-reported edges
      # for the rest of the campaign (`degraded?`, mode :fallback_degraded) and never
      # raises out of an execution.
      class CoverageSource
        # The real `Coverage` module.  The few delegating lines are excluded from
        # the in-process coverage gate (they would clear SimpleCov's own counters if
        # a test ran them); subprocess tests execute them for real.
        # :nocov:
        module StdlibBackend
          def self.available?
            require "coverage"
            ::Coverage.respond_to?(:supported?) && ::Coverage.supported?(:branches)
          rescue LoadError
            false
          end

          def self.running? = ::Coverage.running?

          def self.setup = ::Coverage.setup(branches: true)

          def self.resume = ::Coverage.resume

          def self.read_and_clear = ::Coverage.result(stop: false, clear: true)

          def self.stop = ::Coverage.result
        end
        # :nocov:

        MODES = %i[branches fallback_active_coverage fallback_unavailable fallback_degraded].freeze
        MAX_PATH_CACHE = 10_000

        attr_reader :mode

        def self.branches(load: [], include: nil, backend: StdlibBackend)
          new(load, include, backend)
        end

        def initialize(files, include_prefixes, backend)
          raise TypeError, "load must be an Array of Strings" unless files.is_a?(Array) && files.all?(String)
          valid_include = include_prefixes.nil? || (include_prefixes.is_a?(Array) && include_prefixes.all?(String))
          raise TypeError, "include must be nil or an Array of Strings" unless valid_include

          @files = files.map { |f| File.expand_path(f).freeze }.freeze
          @include = include_prefixes&.map { |p| File.expand_path(p) }
          @backend = backend
          @mode = nil
          @started = false
          @degraded = false
          @measured = {}
        end

        # True once a mid-campaign read failed and the source fell back.
        def degraded? = @degraded

        # The `load:` files that are engine, interpreter or gem plumbing and so are
        # never measured.
        def excluded_files = @files.select { |f| Signature.plumbing?(f) }

        # A summary a caller can check: when `all_excluded` is true nothing the
        # source loaded can ever yield a real edge.
        def status
          excluded = excluded_files
          {mode: @mode, files: @files.size, excluded: excluded,
           all_excluded: !@files.empty? && excluded.size == @files.size, degraded: @degraded}
        end

        # Decides the mode, starts Coverage when it may, loads the target files and
        # begins counting.  Idempotent.  Returns the mode.
        def start
          return @mode if @mode

          @mode =
            if !@backend.available?
              :fallback_unavailable
            elsif @backend.running?
              :fallback_active_coverage
            else
              begin
                @backend.setup
                @started = true
                :branches
              rescue
                :fallback_active_coverage # "coverage measurement is already setup"
              end
            end
          begin
            @files.each { |path| @started ? Kernel.load(path) : require(path) }
            @backend.resume if @started
          rescue Exception # rubocop:disable Lint/RescueException
            finish
            @mode = nil # a later start decides afresh, never reports this one's mode
            raise
          end
          @mode
        end

        # Records this execution's real edges into `bitmap` and zeroes the counters.
        # Returns how many it recorded (0 in a fallback mode: nothing is read).
        def collect(bitmap)
          return 0 unless @started

          recorded = 0
          @backend.read_and_clear.each do |path, data|
            next unless measured?(path)

            (data[:branches] || {}).each do |base, targets|
              targets.each do |target, count|
                next unless count.positive?

                bitmap.record(Prng.fnv1a64("#{File.basename(path)}:#{spell(base)}:#{spell(target)}"))
                recorded += 1
              end
            end
          end
          recorded
        rescue
          # The target stopped, suspended or consumed Coverage (or the result is not
          # what Coverage returns): degrade, never abort the campaign.
          degrade
          0
        end

        # Stops Coverage if (and only if) this source started it.  Idempotent.
        def finish
          return nil unless @started

          @started = false
          stop_quietly
        end

        private

        def degrade
          @degraded = true
          @mode = :fallback_degraded
          return unless @started

          @started = false
          stop_quietly
        end

        # The run may already be gone (the target stopped it): that is not an error.
        def stop_quietly
          @backend.stop
          nil
        rescue
          nil
        end

        # [:if, 3, 3, 4, 7, 7] -> "if@3:4-7:7"
        def spell(key)
          type, _id, first_line, first_col, last_line, last_col = key
          "#{type}@#{first_line}:#{first_col}-#{last_line}:#{last_col}"
        end

        def measured?(path)
          return @measured[path] if @measured.key?(path)

          @measured.clear if @measured.size >= MAX_PATH_CACHE
          @measured[path] = !Signature.plumbing?(path) && (@include.nil? || @include.any? { |p| under?(File.expand_path(path), p) })
        end

        # Prefix match that respects separators: "/a/lib" holds "/a/lib/x.rb" but
        # not "/a/lib_other/x.rb".
        def under?(path, prefix)
          path == prefix || path.start_with?(prefix.end_with?("/") ? prefix : "#{prefix}/")
        end
      end
    end
  end
end
