# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # The deterministic, budgeted coverage-guided campaign (VER06 sections 5 and
      # 9.1, VER06i sections 5 and 6).
      #
      # A campaign is a pure function of (corpus, options, target): triage every
      # seed, then repeatedly pick a seed, give it an energy-scaled number of
      # mutated children, run each child and either admit it (new coverage), record
      # it (crash, timeout, OOM) or drop it (redundant).  Nothing here reads a
      # clock, a file or the environment, and nothing starts a thread.

      STALE_SENTINEL = "stale-budget-sentinel"

      # Exceptions that are the PROCESS talking: never a verdict about the target.
      # (NoMemoryError is handled separately: it is the OOM outcome.)
      PROCESS_OWNED = [SignalException, SystemExit].freeze

      # Runs one execution.  The frame named `run_target` delimits the target's
      # frames: the stack is cut there, so nothing above it is ever inspected.
      module Executor
        # Only the target runs inside the `begin`.  A real NoMemoryError is an OOM
        # outcome (nothing is rendered from it); every other exception goes to
        # `classify`.  SystemStackError from the target is a crash.  The handle is
        # retired in `ensure` and read AFTER that, so no stale thread can change what
        # the verdict reads.
        def self.run_target(target, data, handle)
          # The copy is made OUTSIDE the begin: an out-of-memory raised by the
          # engine's own copy is not a verdict about the target.
          copy = data.dup.b
          out_of_memory = false
          thrown = nil
          begin
            target.call(copy, handle)
          rescue NoMemoryError
            out_of_memory = true
          rescue *PROCESS_OWNED
            raise
          rescue Exception => e # rubocop:disable Lint/RescueException
            # Timeout::ExitException exists only once "timeout" is loaded, possibly
            # after this file: it is looked up at the moment of the rescue, as
            # Safe.recoverable? does, never required or cached.
            raise if defined?(::Timeout::ExitException) && ::Timeout::ExitException === e # rubocop:disable Style/CaseEquality

            thrown = e
          ensure
            # Runs however control leaves: a normal return, an exception, or a
            # NON-LOCAL EXIT (`break`, `return`, `throw`, Thread#kill, Thread.exit).
            # It only retires the handle and returns nothing, so a non-local exit
            # transfers control exactly as Ruby defines (see VER06i section 5).
            handle.retire
          end

          # A target that swallowed the sentinel is still a timeout/OOM, whether it
          # then returned, raised something ordinary or ran out of real memory.
          return exhausted_outcome(handle.exhausted_which) if handle.exhausted
          return Outcome.oom if out_of_memory
          return classify(thrown, handle) if thrown

          Outcome.pass
        end

        def self.exhausted_outcome(which)
          (which == :steps) ? Outcome.timeout : Outcome.oom
        end

        # Turns an exception into an outcome.  A sentinel of ANOTHER campaign is
        # re-raised to its owner; one of this campaign but another execution (or a
        # retired one) is a stale-sentinel crash.
        def self.classify(error, handle)
          # Identity, not fields: only a sentinel some campaign issued is one.  A
          # forged `BudgetExceeded.new` has no owner and falls through to an ordinary
          # crash.
          if Handle::BudgetExceeded === error && (owner = Handle::CampaignToken.owner_of(error)) # rubocop:disable Style/CaseEquality
            raise error unless owner.equal?(handle.campaign)

            epoch, which = owner.provenance(error)
            return Outcome.crash(STALE_SENTINEL, []) if epoch != handle.epoch || which == :retired

            return exhausted_outcome(which)
          end
          kind = Signature.kind_of_exception(error)
          Outcome.crash(kind, Signature.frames_of(error))
        rescue NoMemoryError
          Outcome.oom # reading the stack itself ran out of memory: a counted OOM
        end
      end

      # ---------------------------------------------------------------------------
      # Minimisation (VER06 section 5.3)
      # ---------------------------------------------------------------------------

      def self.same_crash?(outcome, sig)
        outcome.crash? && Signature.crash_signature(outcome.crash_kind, outcome.frames) == sig
      end

      # Shrinks `data` while it keeps the exact signature.  Returns [bytes,
      # executions spent, finished].  EVERY candidate execution, accepted or
      # rejected, is counted and the budget is checked before each one, so no
      # stream of rejected candidates can run past it.
      def self.minimize(data, sig, test, budget)
        cur = data.b
        spent = 0
        return [cur, 0, true] if cur.empty?

        # Pass 1: delete blocks, halving the block size.  Starting at max(1, len/2)
        # (not len/2) is what makes the empty input reachable from a 1-byte crasher.
        chunk = [1, cur.bytesize / 2].max
        while chunk.positive?
          offset = 0
          while offset < cur.bytesize
            return [cur, spent, false] if spent >= budget

            stop = offset + [chunk, cur.bytesize - offset].min
            candidate = cur.byteslice(0, offset) + cur.byteslice(stop, cur.bytesize)
            spent += 1
            if same_crash?(test.call(candidate), sig)
              cur = candidate # same offset again: the tail slid left
            else
              offset += chunk
            end
          end
          chunk /= 2
        end
        # Pass 2: zero individual non-essential bytes.
        cur = cur.dup
        cur.bytesize.times do |i|
          original = cur.getbyte(i)
          next if original.zero?
          return [cur, spent, false] if spent >= budget

          cur.setbyte(i, 0)
          spent += 1
          cur.setbyte(i, original) unless same_crash?(test.call(cur.dup), sig)
        end
        [cur, spent, true]
      end

      # Shrinks `data` while it keeps the EXACT crash signature (never trading one
      # bug for another).  `budget` caps executions of `test`.
      def self.minimize_crashing_input(data, sig, budget = Limits::DEFAULT_MINIMIZE_BUDGET, &test)
        raise ArgumentError, "a test block is required" unless test

        Limits.check_int!(budget, "budget")
        raise ArgumentError, "budget must be in [0, #{Limits::MAX_MINIMIZE_BUDGET}]" unless budget.between?(0, Limits::MAX_MINIMIZE_BUDGET)
        raise TypeError, "data must be a String" unless data.is_a?(String)
        raise ArgumentError, "data exceeds #{Limits::MAX_INPUT_BYTES} bytes" if data.bytesize > Limits::MAX_INPUT_BYTES

        minimize(data, sig, test, budget).first
      end

      # ---------------------------------------------------------------------------
      # The campaign
      # ---------------------------------------------------------------------------

      # Neither prints its fields: a seed or a crash input must not reach a log.
      SeedEntry = Struct.new(:data, :new_edges, :generation) do
        def inspect = "#<Fuzz::SeedEntry bytes=#{data.bytesize} generation=#{generation}>"

        alias_method :to_s, :inspect
      end
      CrashEntry = Struct.new(:signature, :kind, :frames, :minimal, :hits, :minimized, :input_dropped) do
        def inspect = "#<Fuzz::CrashEntry #{signature} #{kind} hits=#{hits}>"

        alias_method :to_s, :inspect
      end

      # Mutable state of one run, kept off the public surface.
      class Campaign
        attr_reader :corpus, :cumulative, :rng, :admissions_dropped, :dropped, :timeouts, :ooms

        def initialize(exec, opts, retained_cap)
          @exec = exec
          @opts = opts
          @rng = Prng::SplitMix64.new(opts.seed)
          @step = Bitmap.new
          @cumulative = Bitmap.new
          @corpus = []
          @crashes = []
          @by_sig = {}
          @timeouts = 0
          @ooms = 0
          @dropped = 0
          @admissions_dropped = 0
          @minimize_left = opts.max_iterations
          @retained = 0
          @retained_cap = retained_cap
        end

        def inspect = "#<Fuzz::Campaign seeds=#{@corpus.size} crashes=#{@crashes.size}>"

        alias_method :to_s, :inspect

        # Runs one input; the edges of the run are left in the step bitmap.
        def execute(data)
          @step.reset
          out = @exec.call(data, @step)
          raise TypeError, "exec must return a Fuzz::Outcome" unless out.is_a?(Outcome)

          out
        end

        # The minimiser's re-run: coverage is discarded and nothing is counted.
        def probe(data) = execute(data)

        # Puts one non-passing execution in its bucket.
        def file(out, data)
          case out.kind
          when :timeout then @timeouts += 1
          when :oom then @ooms += 1
          when :crash then record_crash(out, data)
          end
        end

        # Puts a passing input in the corpus iff it covered a new edge.  The first
        # passing seed is always admitted, even with no coverage, so there is
        # something to mutate in black-box mode.
        def admit(data, generation)
          has_new, count_new = @cumulative.admit(@step.covered_edges)
          first_seed = generation.zero? && @corpus.empty?
          return unless has_new || first_seed

          if @retained + data.bytesize > @retained_cap
            @admissions_dropped += 1
            return
          end
          @retained += data.bytesize
          @corpus << SeedEntry.new(data.b, count_new, generation)
        end

        def crashes
          @crashes.map do |c|
            CrashRecord.new(signature: c.signature, kind: c.kind, frames: c.frames, minimal_input: c.minimal,
              hits: c.hits, minimized: c.minimized, input_dropped: c.input_dropped)
          end
        end

        private

        def record_crash(out, data)
          sig = Signature.crash_signature(out.crash_kind, out.frames)
          if (at = @by_sig[sig])
            rec = @crashes[at]
            rec.hits += 1
            if !rec.input_dropped && data.bytesize < rec.minimal.bytesize
              @retained -= rec.minimal.bytesize - data.bytesize
              rec.minimal = data.b
            end
            return
          end
          if @crashes.size >= Limits::MAX_CRASHES
            @dropped += 1
            return
          end
          allowance = [@opts.minimize_budget, @minimize_left].min
          kept, spent, finished = Fuzz.minimize(data, sig, method(:probe), allowance)
          @minimize_left -= spent
          dropped_input = false
          if @retained + kept.bytesize > @retained_cap
            kept = "".b
            dropped_input = true
          else
            @retained += kept.bytesize
          end
          @by_sig[sig] = @crashes.size
          @crashes << CrashEntry.new(sig, out.crash_kind, out.frames, kept, 1, finished && !dropped_input, dropped_input)
        end
      end

      # Runs a bounded coverage-guided campaign over `exec`, a callable
      # `(data, bitmap) -> Outcome` that runs the code under test and reports edges
      # into the bitmap.  Contracts (VER06 section 9.1), each pinned by a test:
      #
      # * a pure function of (corpus, options, exec): same inputs, same report;
      # * with a live corpus `iterations == max_iterations` exactly, and no mutated
      #   candidate exceeds `max_input_len` (initial seeds run as given);
      # * an empty corpus is replaced by one 00 seed; seeds that crash, time out or
      #   exhaust memory are never mutated, so a corpus of crashers fuzzes nothing;
      # * every execution lands in crashes, timeouts, ooms or the corpus; `clean?`
      #   needs at least one iteration and no failure at all;
      # * one CrashRecord per signature; a novel crash is minimised.
      #
      # Per iteration the stream draws: bounded(len(corpus)) for the seed; per
      # energy slot 1 + bounded(4) steps, one next_u64 sub-seed, then, if there is
      # more than one seed, bounded(4) and on 0 a donor index and two split draws.
      #
      # `retained_cap` and `tier` are seams: the cap exists so tests can reach the
      # retained-bytes limit, `tier` lets #run label real coverage.
      def self.run_outcomes(initial_corpus, options, exec, retained_cap: Limits::MAX_RETAINED_BYTES, tier: nil)
        raise TypeError, "options must be Fuzz::Options" unless options.is_a?(Options)
        raise TypeError, "exec must respond to call" unless exec.respond_to?(:call)
        raise TypeError, "initial_corpus must be an Array of Strings" unless initial_corpus.is_a?(Array) && initial_corpus.all?(String)
        raise ArgumentError, "initial corpus exceeds #{Limits::MAX_INITIAL_CORPUS} seeds" if initial_corpus.size > Limits::MAX_INITIAL_CORPUS

        initial_corpus.each_with_index do |s, i|
          raise ArgumentError, "initial corpus seed #{i} exceeds #{Limits::MAX_INPUT_BYTES} bytes" if s.bytesize > Limits::MAX_INPUT_BYTES
        end
        seeds = initial_corpus.empty? ? ["\x00".b] : initial_corpus.map(&:b)
        camp = Campaign.new(exec, options, retained_cap)

        # 1. Triage: run every seed once.
        seeds.each do |data|
          out = camp.execute(data)
          out.pass? ? camp.admit(data, 0) : camp.file(out, data)
        end

        # 2. The mutation loop.
        iterations = 0
        rng = camp.rng
        corpus = camp.corpus
        while iterations < options.max_iterations && !corpus.empty?
          seed = corpus[rng.bounded(corpus.size)]
          energy = calculate_energy(options.base_energy, seed.data.bytesize, seed.new_edges, camp.cumulative.count)
          slot = 0
          while slot < energy && iterations < options.max_iterations
            slot += 1
            iterations += 1
            steps = 1 + rng.bounded(4)
            sub = rng.next_u64
            candidate = Mutate.mutate_sequence(seed.data, sub, steps, Limits::MAX_INPUT_BYTES)
            if corpus.size > 1 && rng.bounded(4).zero?
              donor = corpus[rng.bounded(corpus.size)].data
              t_split = rng.bounded(candidate.bytesize + 1)
              d_split = rng.bounded(donor.bytesize + 1)
              candidate = Mutate.splice(candidate, donor, t_split, d_split)
            end
            candidate = candidate.byteslice(0, options.max_input_len)
            out = camp.execute(candidate)
            out.pass? ? camp.admit(candidate, seed.generation + 1) : camp.file(out, candidate)
          end
        end

        total = camp.cumulative.count
        Report.new(
          iterations: iterations, admitted_seeds: corpus.size, total_edges: total, crashes: camp.crashes,
          timeouts: camp.timeouts, ooms: camp.ooms, crashes_dropped: camp.dropped,
          admissions_dropped: camp.admissions_dropped,
          coverage_tier: if tier
                           tier.call(total)
                         else
                           (total.positive? ? Report::TIER_REPORTED : Report::TIER_BLACK_BOX)
                         end,
          seed: options.seed, corpus: corpus.map(&:data)
        )
      end

      # Runs a campaign against `target` (a block `|data, handle|`).  Any exception
      # escaping it is a crash.  The coverage tier is `ruby-coverage-branches` if the
      # CoverageSource collected a real edge, `caller-reported` if the target called
      # `handle.edge`, and `black-box-uninstrumented` otherwise.
      def self.run(initial_corpus, options = Options.new, coverage: nil, &target)
        raise ArgumentError, "a target block is required" unless target
        raise TypeError, "options must be Fuzz::Options" unless options.is_a?(Options)
        raise TypeError, "coverage must be nil or a Fuzz::CoverageSource" unless coverage.nil? || coverage.is_a?(CoverageSource)

        campaign = Handle::CampaignToken.new # which campaign a sentinel belongs to
        epoch = 0
        caller_seen = false
        coverage&.start
        exec = lambda do |data, cov|
          epoch += 1
          # A FRESH handle per execution, retired by run_target: a stale thread or
          # fiber can never touch a later run's budgets or coverage map.
          handle = Handle.new(options.step_budget, options.mem_budget, campaign, epoch, cov)
          out = Executor.run_target(target, data, handle)
          caller_seen ||= handle.edge_calls.positive?
          coverage&.collect(cov)
          out
        end
        tier = lambda do |_total|
          # Evidence from the LATEST executions, not a sticky flag: a source that lost
          # its Coverage run, or whose reads have gone empty (the target consumed the
          # counters), is never labelled as real coverage.
          if coverage&.real_recent? then Report::TIER_BRANCHES
          elsif caller_seen then Report::TIER_REPORTED
          else
            Report::TIER_BLACK_BOX
          end
        end
        run_outcomes(initial_corpus, options, exec, tier: tier)
      ensure
        coverage.finish if coverage.is_a?(CoverageSource)
      end
    end
  end
end
