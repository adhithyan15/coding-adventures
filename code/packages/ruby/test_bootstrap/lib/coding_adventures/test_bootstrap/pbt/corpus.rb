# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # The production runner for the shared pbt-v1 corpus
      # (`code/specs/fixtures/pbt-v1/cases.json`, VER05 section 6).
      #
      # It takes the raw JSON, strictly decodes it, evaluates every case with THIS
      # gem's own public engine functions and returns human-readable problems.  An
      # empty list means "conforms".  It does no file I/O (the gem is
      # capability-free); the test that reads the committed fixture lives in the
      # test tree.
      #
      # Why a runner that can fail matters: a conformance check that cannot report
      # a problem is worse than none.  So the runner is strict:
      #
      #   * a case must have exactly the fields of its op, with canonical encodings;
      #   * an unknown op, unknown property name or malformed value is a PROBLEM,
      #     never a silent skip;
      #   * every op and all four failure ids must appear, an unregistered id must
      #     be rejected, and the number of cases must equal PBTV1_CASE_COUNT, so
      #     deleting cases is itself a problem;
      #   * every problem about a case starts with that case's id.
      module Corpus
        Values = CorpusJson::Values
        Clip = CorpusJson::Clip

        PBTV1_CASE_COUNT = 25
        MAX_CASES = 10_000
        # Bounds the total bytes one gen case may generate and hold (CWE-400).
        MAX_GEN_BYTES = 1 << 20

        Report = Struct.new(:evaluated, :problems)

        # Strictly decodes a pbt-v1 document into its case list: exactly
        # {"schema_version": 1, "cases": [...]}.  Cases are returned undecoded.
        def self.parse(data)
          root = CorpusJson::StrictJson.parse(data)
          unless root.is_a?(Hash) && root.size == 2
            raise CorpusJson::StrictJson::JsonError, "pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}"
          end

          version = root["schema_version"]
          unless version.is_a?(CorpusJson::StrictJson::Number) && version.text == "1"
            raise CorpusJson::StrictJson::JsonError, "pbt corpus: schema_version must be 1"
          end
          unless root["cases"].is_a?(Array)
            raise CorpusJson::StrictJson::JsonError, "pbt corpus: \"cases\" must be an array"
          end

          root["cases"]
        end

        # Predicates named by the corpus.  Truthy means "holds".
        PROPERTIES = {
          "int_le_100" => ->(x) { x <= 100 },
          "int_in_range_neg10_50" => ->(x) { x.between?(-10, 50) },
          "int_is_even" => ->(x) { x.even? },
          "int_abs_le_10" => ->(x) { x.abs <= 10 },
          "refinement_x_le_50" => ->(x) { x <= 50 },
          "list_no_consecutive_duplicates" => ->(l) { l.each_cons(2).none? { |a, b| a == b } },
          "list_sum_le_20" => ->(l) { l.sum <= 20 },
          "bytes_len_lt_5" => ->(b) { b.bytesize < 5 },
          "bytes_no_zero_byte" => ->(b) { b.bytes.none?(0) }
        }.freeze
        INT_PROPS = %w[int_le_100 int_in_range_neg10_50 int_is_even int_abs_le_10 refinement_x_le_50].freeze
        LIST_PROPS = %w[list_no_consecutive_duplicates list_sum_le_20].freeze
        BYTES_PROPS = %w[bytes_len_lt_5 bytes_no_zero_byte].freeze

        # The closed field set of each op (besides id and op).
        SHAPES = {
          "gen_primitive" => %w[type seed count outputs],
          "gen_bounded" => %w[min max seed count outputs],
          "gen_bytes" => %w[min_len max_len seed count outputs],
          "shrink_int" => %w[initial target min max property expected_minimal expected_steps],
          "shrink_refinement" => %w[initial target min max property expected_minimal expected_steps],
          "shrink_list" => %w[initial elem_target property expected_minimal expected_steps],
          "shrink_bytes" => %w[initial property expected_minimal expected_steps],
          "replay_consistency" => %w[seed property initial_fail expected_minimal],
          "filter_exhausted" => %w[max_retries verdict failure_id],
          "failure_id_valid" => %w[input valid],
          "corpus_reject" => %w[text]
        }.freeze

        # Evaluates every case; returns a Report (evaluated count + problems).
        # A nil `expected` skips the case-count check (ad-hoc case lists in tests).
        def self.evaluate(cases_in, expected = PBTV1_CASE_COUNT)
          problems = []
          evaluated = 0
          cases = cases_in
          if expected && cases.size != expected
            problems << "pbt-v1: expected #{expected} cases, got #{cases.size}"
          end
          if cases.size > MAX_CASES
            problems << "pbt-v1: #{cases.size} cases exceeds the cap of #{MAX_CASES}"
            cases = cases.first(MAX_CASES)
          end

          seen_ids = {}
          cases.each do |raw|
            id = (raw.is_a?(Hash) && raw["id"].is_a?(String)) ? raw["id"] : nil
            if id.nil? || id.empty? || seen_ids.key?(id)
              problems << "duplicate or missing case id"
              break
            end
            seen_ids[id] = true
          end

          ops_seen = {}
          registered_seen = {}
          rejected_id_seen = false

          cases.each do |raw|
            unless raw.is_a?(Hash)
              problems << "case is not an object: #{Values.show(raw)}"
              next
            end
            unless raw["id"].is_a?(String) && !raw["id"].empty?
              problems << "case missing a string id: #{Values.show(raw)}"
              next
            end
            cid = CorpusJson::Canon.escape(Clip.clip(raw["id"], 100))
            op = raw["op"].is_a?(String) ? raw["op"] : ""
            unless SHAPES.key?(op)
              problems << "#{cid}: unknown op #{Values.show(raw["op"])}"
              next
            end
            want = %w[id op] + SHAPES[op]
            unless raw.keys.sort == want.sort
              problems << "#{cid}: fields #{Values.show(raw.keys.sort)} != #{want.inspect}"
              next
            end
            ev = evaluate_case(cid, op, raw)
            problems.concat(ev[:problems])
            next unless ev[:ran_ok]

            if op == "failure_id_valid"
              if ev[:registered]
                registered_seen[ev[:candidate]] = true
              else
                rejected_id_seen = true
              end
            end
            ops_seen[op] = true
            evaluated += 1
          end

          missing_ops = SHAPES.keys.sort - ops_seen.keys
          problems << "pbt-v1: operations never exercised: #{missing_ops}" unless missing_ops.empty?
          missing_ids = FailureIds::ALL - registered_seen.keys
          problems << "pbt-v1: registered failure ids missing: #{missing_ids}" unless missing_ids.empty?
          problems << "pbt-v1: failure_id_valid needs an unregistered-id rejection" unless rejected_id_seen
          Report.new(evaluated, problems.map { |p| Clip.clip(p, Values::MAX_PROBLEM_LEN) })
        end

        # Decodes and checks one case.  Any exception (a malformed case or an
        # engine that raised) is reported as a problem.
        def self.evaluate_case(cid, op, raw)
          v = {}
          raw.each { |k, val| v[k] = Values.decode(val) unless %w[id op].include?(k) }
          found = []
          candidate = ""
          registered = false
          case op
          when "gen_primitive", "gen_bounded", "gen_bytes" then found = check_gen(cid, op, v)
          when "shrink_int", "shrink_refinement" then found = check_shrink_int(cid, op, v)
          when "shrink_list" then found = check_shrink_list(cid, v)
          when "shrink_bytes" then found = check_shrink_bytes(cid, v)
          when "replay_consistency" then found = check_replay(cid, v)
          when "filter_exhausted" then found = check_filter_exhausted(cid, v)
          when "failure_id_valid"
            input = v["input"].is_a?(String) ? v["input"] : nil
            got = !input.nil? && FailureIds.pbt_failure_id?(input)
            found << "#{cid}: failure id membership disagrees for #{Values.show(v["input"])}" unless Values.same?(got, v["valid"])
            candidate = input || ""
            registered = got
          else # corpus_reject
            raise Values::CaseError, "text must be a string" unless v["text"].is_a?(String)

            found << "#{cid}: corpus_reject text was accepted by the strict parser" if Tfv.valid?(v["text"])
          end
          {problems: found, ran_ok: true, candidate: candidate, registered: registered}
        rescue Exception => e # rubocop:disable Lint/RescueException
          raise unless Safe.recoverable?(e)

          {problems: ["#{cid}: #{op} could not be evaluated: #{Clip.clip(Safe.safe_string(e), Values::MAX_FRAGMENT_LEN)}"],
           ran_ok: false, candidate: "", registered: false}
        end

        def self.case_error(message) = Values::CaseError.new(message)

        def self.int_of(value, what)
          raise case_error("#{what} must be an int") unless value.is_a?(Integer)

          value
        end

        # An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE.
        def self.size_of(value, what)
          n = int_of(value, what)
          raise case_error("#{what} #{n} outside [0, #{Values::MAX_SIZE}]") unless n.between?(0, Values::MAX_SIZE)

          n
        end

        def self.prop_of(name, allowed, op)
          raise case_error("unknown property #{Values.show(name)} for #{op}") unless name.is_a?(String) && allowed.include?(name)

          PROPERTIES.fetch(name)
        end

        def self.check_gen(cid, op, v)
          seed = int_of(v["seed"], "seed") & Prng::MASK64
          count = size_of(v["count"], "count")
          # The expected outputs are in hand already: a case whose list length is
          # not count cannot conform, so refuse it before generating anything.
          unless v["outputs"].is_a?(Array) && v["outputs"].size == count
            raise case_error("outputs must be a list of exactly count (#{count}) values")
          end

          got =
            case op
            when "gen_primitive"
              raise case_error("type must be a string") unless v["type"].is_a?(String)

              Gens.gen_primitive(seed, v["type"], count)
            when "gen_bounded"
              Gens.gen_bounded(seed, int_of(v["min"], "min"), int_of(v["max"], "max"), count)
            else
              min_len = size_of(v["min_len"], "min_len")
              max_len = size_of(v["max_len"], "max_len")
              if count * max_len > MAX_GEN_BYTES
                raise case_error("count #{count} x max_len #{max_len} exceeds the generation budget of #{MAX_GEN_BYTES} bytes")
              end

              Gens.gen_bytes(seed, min_len, max_len, count)
            end
          Values.same?(got, v["outputs"]) ? [] : ["#{cid}: #{op} outputs mismatch"]
        end

        def self.check_shrink_int(cid, op, v)
          prop = prop_of(v["property"], INT_PROPS, op)
          initial = int_of(v["initial"], "initial")
          target = int_of(v["target"], "target")
          lo = int_of(v["min"], "min")
          hi = int_of(v["max"], "max")
          minimal, steps = Shrink.shrink_int(initial, target, lo, hi, 1000, &prop)
          problems = []
          unless Values.same?(minimal, v["expected_minimal"])
            problems << "#{cid}: #{op} minimal mismatch: got #{Values.show(minimal)}, corpus #{Values.show(v["expected_minimal"])}"
          end
          unless Values.same?(steps, v["expected_steps"])
            problems << "#{cid}: #{op} step count mismatch: got #{steps}, corpus #{Values.show(v["expected_steps"])}"
          end
          # Engine invariants, independent of the corpus's expected values.
          problems << "#{cid}: #{op} minimal #{Values.show(minimal)} does not falsify the property" if prop.call(minimal)
          problems << "#{cid}: #{op} minimal #{Values.show(minimal)} left [#{lo}, #{hi}]" unless minimal.between?(lo, hi)
          problems
        end

        def self.check_shrink_list(cid, v)
          prop = prop_of(v["property"], LIST_PROPS, "shrink_list")
          raise case_error("initial must be a list of ints") unless v["initial"].is_a?(Array)

          initial = v["initial"].map { |item| int_of(item, "element") }
          elem_target = int_of(v["elem_target"], "elem_target")
          minimal, steps = Shrink.shrink_list(initial, elem_target, 1000, &prop)
          problems = []
          unless Values.same?(minimal, v["expected_minimal"])
            problems << "#{cid}: shrink_list minimal mismatch: got #{Values.show(minimal)}, corpus #{Values.show(v["expected_minimal"])}"
          end
          unless Values.same?(steps, v["expected_steps"])
            problems << "#{cid}: shrink_list steps mismatch: got #{steps}, corpus #{Values.show(v["expected_steps"])}"
          end
          problems << "#{cid}: shrink_list minimal #{Values.show(minimal)} does not falsify the property" if prop.call(minimal)
          problems
        end

        def self.check_shrink_bytes(cid, v)
          prop = prop_of(v["property"], BYTES_PROPS, "shrink_bytes")
          unless v["initial"].is_a?(String) && v["initial"].encoding == Encoding::BINARY
            raise case_error("initial must be bytes")
          end

          minimal, steps = Shrink.shrink_bytes(v["initial"], 1000, &prop)
          problems = []
          unless Values.same?(minimal, v["expected_minimal"])
            problems << "#{cid}: shrink_bytes minimal mismatch: got #{Values.show(minimal)}, corpus #{Values.show(v["expected_minimal"])}"
          end
          unless Values.same?(steps, v["expected_steps"])
            problems << "#{cid}: shrink_bytes steps mismatch: got #{steps}, corpus #{Values.show(v["expected_steps"])}"
          end
          problems << "#{cid}: shrink_bytes minimal #{Values.show(minimal)} does not falsify the property" if prop.call(minimal)
          problems
        end

        def self.check_replay(cid, v)
          seed = int_of(v["seed"], "seed") & Prng::MASK64
          prop = prop_of(v["property"], INT_PROPS, "replay_consistency")
          initial_fail, minimal = Shrink.simulate_replay(seed, &prop)
          problems = []
          unless Values.same?(initial_fail, v["initial_fail"])
            problems << "#{cid}: replay initial_fail mismatch: got #{Values.show(initial_fail)}, corpus #{Values.show(v["initial_fail"])}"
          end
          unless Values.same?(minimal, v["expected_minimal"])
            problems << "#{cid}: replay minimal mismatch: got #{Values.show(minimal)}, corpus #{Values.show(v["expected_minimal"])}"
          end
          problems
        end

        def self.check_filter_exhausted(cid, v)
          max_retries = size_of(v["max_retries"], "max_retries")
          raise case_error("max_retries must be at least 1") if max_retries < 1

          calls = 0
          gen = Gens.filter(Gens.integer(0, 10), max_retries) do |_x|
            calls += 1
            false
          end
          verdict = Runner.check(0, gen, Options.new(num_tests: 1, max_shrink_steps: 1000, ci: :on)) { true }
          problems = []
          problems << "#{cid}: an unsatisfiable filter must be a fail verdict" if v["verdict"] != "fail" || verdict.passed
          unless Values.same?(verdict.failure_id, v["failure_id"])
            problems << "#{cid}: failure id #{verdict.failure_id.inspect}, corpus #{Values.show(v["failure_id"])}"
          end
          problems << "#{cid}: filter sampled #{calls} times, expected #{max_retries}" if calls != max_retries
          problems
        end

        private_class_method :evaluate_case, :case_error, :int_of, :size_of, :prop_of, :check_gen,
          :check_shrink_int, :check_shrink_list, :check_shrink_bytes, :check_replay, :check_filter_exhausted
      end
    end
  end
end
