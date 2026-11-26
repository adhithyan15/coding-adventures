# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # The production runner for the shared fuzz-v1 corpus
      # (`code/specs/fixtures/fuzz-v1/cases.json`, VER06 section 8, VER06i section 8).
      #
      # It takes the raw JSON, strictly decodes it, evaluates every case with THIS
      # gem's own public engine functions and returns human-readable problems.  An
      # empty list means "conforms".  It does no file I/O (the gem is
      # capability-free); the tests read the committed fixture.
      #
      # A runner that cannot report a problem is worse than none, so this one is
      # strict:
      #
      # * a case has exactly the fields of its op, in canonical encodings;
      # * an unknown op or a malformed value is a PROBLEM, never a silent skip;
      # * every op appears, a failure id is accepted and one rejected, and the case
      #   count equals FUZZV1_CASE_COUNT, so deleting cases is itself a problem;
      # * fixture-controlled sizes are capped (CWE-400): a hostile `steps = 10^9`
      #   is a reported problem, not a hang;
      # * every problem about a case starts with that case's id, ESCAPED (an id
      #   comes from the fixture and may hold a newline).
      module Corpus
        Values = CorpusJson::Values
        Clip = CorpusJson::Clip

        FUZZV1_CASE_COUNT = 25
        MAX_CASES = 10_000
        # The aggregate mutate_sequence work of one corpus, in bytes ACTUALLY
        # copied.  Each case is capped (10,000 steps, 10,000 bytes) but 10,000 such
        # cases would still be 10^12 byte-steps.
        MAX_SEQUENCE_WORK = 1 << 24
        TWO64 = 1 << 64

        Report = Struct.new(:evaluated, :problems)

        # Strictly decodes a fuzz-v1 document into its case list: exactly
        # {"schema_version": 1, "cases": [...]}.  Cases are returned undecoded.
        def self.parse(data)
          root = CorpusJson::StrictJson.parse(data)
          unless root.is_a?(Hash) && root.size == 2
            raise CorpusJson::StrictJson::JsonError, "fuzz corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}"
          end

          version = root["schema_version"]
          unless version.is_a?(CorpusJson::StrictJson::Number) && version.text == "1"
            raise CorpusJson::StrictJson::JsonError, "fuzz corpus: schema_version must be 1"
          end
          raise CorpusJson::StrictJson::JsonError, "fuzz corpus: \"cases\" must be an array" unless root["cases"].is_a?(Array)

          root["cases"]
        end

        # The closed field set of each op (besides id and op).
        SHAPES = {
          "mutate_bit_flip" => %w[input bit_offset count output],
          "mutate_byte_flip" => %w[input byte_offset count output],
          "mutate_arithmetic" => %w[input byte_offset delta width endian output],
          "mutate_boundary" => %w[input byte_offset width val endian output],
          "mutate_block_delete" => %w[input offset length output],
          "mutate_block_insert" => %w[input offset insert_bytes output],
          "mutate_block_duplicate" => %w[input src_offset length dst_offset output],
          "mutate_splice" => %w[target donor target_split donor_split output],
          "mutate_sequence" => %w[input seed steps output],
          "corpus_admit" => %w[existing_edges trace_edges admitted new_edges],
          "crash_dedup" => %w[crash_a_kind crash_a_frames crash_b_kind crash_b_frames is_duplicate],
          "failure_id_valid" => %w[input valid],
          "private_reporting" => %w[target failure_id raw_payload sig_hash tap_line contains_raw_payload],
          "corpus_reject" => %w[text]
        }.freeze

        Eval = Struct.new(:problems, :ran_ok, :accepted)

        def self.evaluate(cases_in, expected = FUZZV1_CASE_COUNT)
          problems = []
          evaluated = 0
          cases = cases_in
          problems << "fuzz-v1: expected #{expected} cases, got #{cases.size}" if expected && cases.size != expected
          if cases.size > MAX_CASES
            problems << "fuzz-v1: #{cases.size} cases exceeds the cap of #{MAX_CASES}"
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

          work = [MAX_SEQUENCE_WORK]
          ops_seen = {}
          accepted_seen = false
          rejected_seen = false

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
            ev = evaluate_case(cid, op, raw, work)
            problems.concat(ev.problems)
            next unless ev.ran_ok

            if op == "failure_id_valid"
              accepted_seen ||= ev.accepted
              rejected_seen ||= !ev.accepted
            end
            ops_seen[op] = true
            evaluated += 1
          end

          missing_ops = SHAPES.keys.sort - ops_seen.keys
          problems << "fuzz-v1: operations never exercised: #{missing_ops}" unless missing_ops.empty?
          problems << "fuzz-v1: failure_id_valid needs a registered-id acceptance" unless accepted_seen
          problems << "fuzz-v1: failure_id_valid needs an unregistered-id rejection" unless rejected_seen
          unregistered = FailureIds::ALL.reject { |id| Hooks.failure_id.call(id) }
          problems << "fuzz-v1: engine rejects registered ids #{unregistered}" unless unregistered.empty?
          Report.new(evaluated, problems.map { |p| Clip.clip(p, Values::MAX_PROBLEM_LEN) })
        end

        # Decodes and checks one case.  Any exception (a malformed case or an engine
        # that raised) is reported as a problem.
        def self.evaluate_case(cid, op, raw, work)
          v = {}
          raw.each { |k, val| v[k] = Values.decode(val) unless %w[id op].include?(k) }
          found = []
          accepted = false
          case op
          when "corpus_admit" then found = check_corpus_admit(cid, v)
          when "crash_dedup" then found = check_crash_dedup(cid, v)
          when "failure_id_valid"
            input = v["input"].is_a?(String) ? v["input"] : nil
            accepted = !input.nil? && Hooks.failure_id.call(input)
            found << "#{cid}: failure id membership disagrees for #{Values.show(v["input"])}" unless Values.same?(accepted, v["valid"])
          when "private_reporting" then found = check_private_reporting(cid, v)
          when "corpus_reject"
            raise Values::CaseError, "text must be a string" unless v["text"].is_a?(String)

            found << "#{cid}: corpus_reject text was accepted by the strict parser" if Hooks.tfv_valid.call(v["text"])
          else found = check_mutator(cid, op, v, work)
          end
          Eval.new(found, true, accepted)
        rescue Exception => e # rubocop:disable Lint/RescueException
          raise unless Safe.recoverable?(e)

          Eval.new(["#{cid}: #{op} could not be evaluated: #{Clip.clip(Safe.safe_string(e), Values::MAX_FRAGMENT_LEN)}"], false, false)
        end

        # -- typed field access ---------------------------------------------------

        def self.case_error(message) = Values::CaseError.new(message)

        def self.bytes_of(v, what)
          return v if v.is_a?(String) && v.encoding == Encoding::BINARY

          raise case_error("#{what} must be bytes")
        end

        def self.text_of(v, what)
          return v if v.is_a?(String) && v.encoding != Encoding::BINARY

          raise case_error("#{what} must be a string")
        end

        def self.bool_of(v, what)
          return v if v == true || v == false

          raise case_error("#{what} must be a bool")
        end

        def self.int_of(v, what)
          return v if v.is_a?(Integer)

          raise case_error("#{what} must be an int")
        end

        # An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE.
        def self.size_of(v, what)
          n = int_of(v, what)
          raise case_error("#{what} #{n} outside [0, #{Values::MAX_SIZE}]") unless n.between?(0, Values::MAX_SIZE)

          n
        end

        def self.texts_of(v, what)
          raise case_error("#{what} must be a list of strings") unless v.is_a?(Array) && v.all? { |s| s.is_a?(String) && s.encoding != Encoding::BINARY }

          v
        end

        # A list of edge ids, each within [0, 2^64).
        def self.edges_of(v, what)
          raise case_error("#{what} must be a list of ints") unless v.is_a?(Array) && v.all?(Integer)

          v.each do |e|
            raise case_error("#{what} element #{Values.show(e)} outside [0, 2^64)") unless e >= 0 && e < TWO64
          end
        end

        # -- per-op checks --------------------------------------------------------

        def self.check_mutator(cid, op, v, work)
          got =
            case op
            when "mutate_bit_flip"
              Hooks.bit_flip.call(bytes_of(v["input"], "input"), int_of(v["bit_offset"], "bit_offset"), int_of(v["count"], "count"))
            when "mutate_byte_flip"
              Hooks.byte_flip.call(bytes_of(v["input"], "input"), int_of(v["byte_offset"], "byte_offset"), int_of(v["count"], "count"))
            when "mutate_arithmetic"
              Hooks.byte_arithmetic.call(bytes_of(v["input"], "input"), int_of(v["byte_offset"], "byte_offset"),
                int_of(v["delta"], "delta"), int_of(v["width"], "width"), text_of(v["endian"], "endian"))
            when "mutate_boundary"
              Hooks.boundary_insert.call(bytes_of(v["input"], "input"), int_of(v["byte_offset"], "byte_offset"),
                int_of(v["width"], "width"), int_of(v["val"], "val"), text_of(v["endian"], "endian"))
            when "mutate_block_delete"
              Hooks.block_delete.call(bytes_of(v["input"], "input"), int_of(v["offset"], "offset"), int_of(v["length"], "length"))
            when "mutate_block_insert"
              Hooks.block_insert.call(bytes_of(v["input"], "input"), int_of(v["offset"], "offset"), bytes_of(v["insert_bytes"], "insert_bytes"))
            when "mutate_block_duplicate"
              Hooks.block_duplicate.call(bytes_of(v["input"], "input"), int_of(v["src_offset"], "src_offset"),
                int_of(v["length"], "length"), int_of(v["dst_offset"], "dst_offset"))
            when "mutate_splice"
              Hooks.splice.call(bytes_of(v["target"], "target"), bytes_of(v["donor"], "donor"),
                int_of(v["target_split"], "target_split"), int_of(v["donor_split"], "donor_split"))
            else sequence_result(v, work)
            end
          problems = compare(cid, op, got, v)
          case op
          when "mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary"
            problems << "#{cid}: #{op} changed the length of its input" if got.bytesize != v["input"].bytesize
          when "mutate_splice"
            tgt = v["target"]
            don = v["donor"]
            want = v["target_split"].clamp(0, tgt.bytesize) + don.bytesize - v["donor_split"].clamp(0, don.bytesize)
            problems << "#{cid}: splice length is not prefix + suffix" if got.bytesize != want
          end
          problems
        end

        # Charges the REAL work: the call is handed what is left of the aggregate cap
        # and what it actually copied (inserts and duplicates grow the buffer, so
        # steps x initial length would undercount) is deducted afterwards.  A call
        # that hits its cap is charged too, the aggregate is then ZEROED, and an
        # exhausted aggregate fails every later case at once without running it.
        def self.sequence_result(v, work)
          input = bytes_of(v["input"], "input")
          seed = int_of(v["seed"], "seed")
          steps = size_of(v["steps"], "steps")
          exhausted = "mutate_sequence work budget of #{MAX_SEQUENCE_WORK} byte-steps exhausted"
          raise case_error(exhausted) if work[0] <= 0

          begin
            data, done = Hooks.mutate_sequence.call(input, seed, steps, Limits::MAX_INPUT_BYTES, work[0])
          rescue Mutate::WorkExhausted
            work[0] = 0
            raise case_error(exhausted)
          end
          work[0] -= done
          data
        end

        def self.compare(cid, op, got, v)
          want = v["output"]
          return ["#{cid}: #{op} output must be bytes"] unless want.is_a?(String) && want.encoding == Encoding::BINARY
          return [] if got.bytes == want.bytes

          ["#{cid}: #{op} expected #{Clip.clip(want.unpack1("H*"), Values::MAX_FRAGMENT_LEN)}, " \
           "got #{Clip.clip(got.unpack1("H*"), Values::MAX_FRAGMENT_LEN)}"]
        end

        def self.check_corpus_admit(cid, v)
          existing = edges_of(v["existing_edges"], "existing_edges")
          trace = edges_of(v["trace_edges"], "trace_edges")
          admitted = bool_of(v["admitted"], "admitted")
          new_edges = int_of(v["new_edges"], "new_edges")
          got_admitted, got_new = Hooks.corpus_admit.call(existing, trace)
          problems = []
          problems << "#{cid}: corpus_admit admitted #{got_admitted}, corpus #{admitted}" if got_admitted != admitted
          problems << "#{cid}: corpus_admit new_edges #{got_new}, corpus #{Values.show(new_edges)}" if got_new != new_edges
          # Invariant: admission is exactly "some trace edge is unseen".
          have = existing.to_set
          problems << "#{cid}: corpus_admit disagrees with set difference" if got_admitted != trace.any? { |e| !have.include?(e) }
          problems
        end

        def self.check_crash_dedup(cid, v)
          ka = text_of(v["crash_a_kind"], "crash_a_kind")
          fa = texts_of(v["crash_a_frames"], "crash_a_frames")
          kb = text_of(v["crash_b_kind"], "crash_b_kind")
          fb = texts_of(v["crash_b_frames"], "crash_b_frames")
          want = bool_of(v["is_duplicate"], "is_duplicate")
          got = Hooks.duplicate_crash.call(ka, fa, kb, fb)
          (got == want) ? [] : ["#{cid}: crash_dedup is_duplicate #{got}, corpus #{want}"]
        end

        def self.check_private_reporting(cid, v)
          raw = bytes_of(v["raw_payload"], "raw_payload")
          target = text_of(v["target"], "target")
          fid = text_of(v["failure_id"], "failure_id")
          sig = text_of(v["sig_hash"], "sig_hash")
          tap = text_of(v["tap_line"], "tap_line")
          want_leak = bool_of(v["contains_raw_payload"], "contains_raw_payload")
          begin
            line = Hooks.format_tap.call(1, target, fid, sig)
          rescue ArgumentError, TypeError => e
            raise case_error("format_tap_failure refused: #{Clip.clip(Safe.safe_string(e), Values::MAX_FRAGMENT_LEN)}")
          end
          problems = []
          unless line == tap
            problems << "#{cid}: tap_line expected #{Clip.clip(CorpusJson::Canon.escape(tap), Values::MAX_FRAGMENT_LEN)}, " \
              "got #{Clip.clip(CorpusJson::Canon.escape(line), Values::MAX_FRAGMENT_LEN)}"
          end
          leaked = !raw.empty? && line.include?(raw.unpack1("H*"))
          problems << "#{cid}: privacy verdict wrong, payload leaked: #{leaked}" if leaked != want_leak
          problems << "#{cid}: failure line spans more than one line" if line.match?(/[\r\n]/)
          problems
        end
      end
    end
  end
end
