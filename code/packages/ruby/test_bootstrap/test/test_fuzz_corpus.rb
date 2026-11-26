# frozen_string_literal: true

require_relative "test_helper"

# Replays the whole committed fuzz-v1 corpus and carries its negative controls
# (VER06i section 10).  Every case is corrupted in every way its op defines and
# the run MUST fail, naming exactly that case.
class TestFuzzCorpus < FuzzHookedTest
  def evaluate(cases, expected = Fuzz::Corpus::FUZZV1_CASE_COUNT)
    Fuzz::Corpus.evaluate(cases, expected)
  end

  def test_the_committed_corpus_conforms_exactly
    cases = Fx.fuzz_cases
    assert_equal 25, cases.size
    report = evaluate(cases)
    assert_equal [], report.problems
    assert_equal 25, report.evaluated
  end

  def flip_bytes(node)
    hex = node["hex"]
    {"kind" => "bytes", "hex" => if hex.empty?
                                   "00"
                                 else
                                   (((hex[0] == "0") ? "1" : "0") + hex[1..])
                                 end}
  end

  def bump(node) = {"kind" => "int", "value" => (node["value"].to_i + 1).to_s}

  def corruptions(op)
    case op
    when /\Amutate_/
      [["output", ->(c) { c["output"] = flip_bytes(c["output"]) }],
        ["output extended", ->(c) { c["output"] = {"kind" => "bytes", "hex" => c["output"]["hex"] + "00"} }]]
    when "corpus_admit"
      [["admitted", ->(c) { c["admitted"] = !c["admitted"] }], ["new_edges", ->(c) { c["new_edges"] = bump(c["new_edges"]) }]]
    when "crash_dedup"
      [["is_duplicate", ->(c) { c["is_duplicate"] = !c["is_duplicate"] }]]
    when "failure_id_valid"
      [["valid", ->(c) { c["valid"] = !c["valid"] }]]
    when "private_reporting"
      [["tap_line", ->(c) { c["tap_line"] += "x" }],
        ["leak verdict", ->(c) { c["contains_raw_payload"] = !c["contains_raw_payload"] }],
        ["sig_hash", ->(c) { c["sig_hash"] = "3a8f19b3" }]]
    when "corpus_reject"
      [["valid TFV in place of a reject text", ->(c) { c["text"] = "tfv 1\n" }]]
    end
  end

  def test_every_corruption_of_every_case_fails_and_names_only_that_case
    total = 0
    Fx.fuzz_cases.each_with_index do |original, index|
      corruptions(original["op"]).each do |label, mutate|
        cases = Fx.fuzz_cases
        mutate.call(cases[index])
        problems = evaluate(cases).problems
        refute_empty problems, "#{original["id"]}: #{label} was not detected"
        problems.each do |problem|
          assert problem.start_with?("#{original["id"]}: "), "#{label}: problem names another case: #{problem}"
        end
        total += 1
      end
    end
    assert_operator total, :>=, 45
  end

  def test_every_op_is_in_the_corpus_and_shapes_cover_exactly_those
    assert_equal Fuzz::Corpus::SHAPES.keys.sort, Fx.fuzz_cases.map { |c| c["op"] }.uniq.sort
  end

  # -- structural controls ------------------------------------------------------

  def test_a_dropped_case_fails
    Fx.fuzz_cases.each_index do |i|
      cases = Fx.fuzz_cases
      cases.delete_at(i)
      assert(evaluate(cases).problems.any? { |p| p.include?("expected 25 cases, got 24") }, "case #{i}")
    end
  end

  def test_a_truncated_corpus_fails
    problems = evaluate(Fx.fuzz_cases.first(5)).problems
    assert(problems.any? { |p| p.include?("operations never exercised") })
    assert(problems.any? { |p| p.include?("registered-id acceptance") })
  end

  def test_dropping_either_failure_id_case_fails
    cases = Fx.fuzz_cases.reject { |c| c["id"] == "fuzz/failure_id/unregistered" }
    assert(evaluate(cases, nil).problems.any? { |p| p.include?("unregistered-id rejection") })
    cases = Fx.fuzz_cases.reject { |c| c["id"] == "fuzz/failure_id/crash_valid" }
    assert(evaluate(cases, nil).problems.any? { |p| p.include?("registered-id acceptance") })
  end

  def test_dropping_every_case_of_one_op_fails
    Fuzz::Corpus::SHAPES.each_key do |op|
      cases = Fx.fuzz_cases.reject { |c| c["op"] == op }
      assert(evaluate(cases, nil).problems.any? { |p| p.include?("operations never exercised") && p.include?(op) }, op)
    end
  end

  def test_an_empty_corpus_fails
    refute_empty evaluate([]).problems
  end

  def test_unknown_op_extra_field_and_missing_field_are_problems
    cases = Fx.fuzz_cases
    cases[0]["op"] = "mystery"
    assert(evaluate(cases).problems.any? { |p| p.include?("unknown op") })
    cases = Fx.fuzz_cases
    cases[0]["extra"] = 1
    assert(evaluate(cases).problems.any? { |p| p.start_with?("fuzz/mutate_bit_flip/bit0: fields") })
    cases = Fx.fuzz_cases
    cases[0].delete("count")
    assert(evaluate(cases).problems.any? { |p| p.include?("fields") })
  end

  def test_malformed_ids_and_cases_are_problems_not_crashes
    cases = Fx.fuzz_cases
    cases[1]["id"] = cases[0]["id"]
    assert(evaluate(cases).problems.any? { |p| p.include?("duplicate or missing case id") })
    cases = Fx.fuzz_cases
    cases[0]["id"] = ["x"]
    assert(evaluate(cases).problems.any? { |p| p.include?("duplicate or missing case id") })
    cases = Fx.fuzz_cases
    cases[2] = 7
    assert(evaluate(cases).problems.any? { |p| p.include?("case is not an object") })
    cases = Fx.fuzz_cases
    cases[2] = {"op" => "mutate_bit_flip"}
    assert(evaluate(cases).problems.any? { |p| p.include?("case missing a string id") })
  end

  def test_a_fixture_id_with_a_newline_is_escaped_in_problem_lines
    cases = Fx.fuzz_cases
    cases[0]["id"] = "evil\nnot ok 99 - forged"
    cases[0]["output"] = flip_bytes(cases[0]["output"])
    problems = evaluate(cases).problems
    refute_empty problems
    problems.each { |p| refute_match(/[\r\n]/, p) }
    assert(problems.any? { |p| p.start_with?("evil\\nnot ok 99") })
  end

  def test_hostile_sizes_are_reported_not_executed
    cases = Fx.fuzz_cases
    i = cases.index { |c| c["op"] == "mutate_sequence" }
    cases[i]["steps"] = {"kind" => "int", "value" => "1000000000"}
    problem = evaluate(cases).problems.first
    assert_includes problem, "mutate_sequence could not be evaluated"
    assert_includes problem, "outside [0, 10000]"
  end

  def test_malformed_values_are_problems
    [{"kind" => "int", "value" => "007"}, {"kind" => "int", "value" => 5}, {"kind" => "int"},
      {"kind" => "bytes", "hex" => "FF"}, {"kind" => "bytes", "hex" => "abc"}, nil, {"kind" => "float"},
      {"kind" => "bytes", "hex" => "00"}].each do |bad|
      cases = Fx.fuzz_cases
      cases[0]["bit_offset"] = bad
      assert(evaluate(cases).problems.any? { |p| p.start_with?("fuzz/mutate_bit_flip/bit0:") }, bad.inspect)
    end
    cases = Fx.fuzz_cases
    cases[0]["bit_offset"] = {"kind" => "int", "value" => "9" * 1001}
    assert(evaluate(cases).problems.any? { |p| p.include?("digits") })
  end

  def test_type_confusions_in_every_op_are_problems
    set = lambda do |id, field, value|
      cases = Fx.fuzz_cases
      cases.find { |c| c["id"] == id }[field] = value
      evaluate(cases).problems.select { |p| p.start_with?("#{id}:") }
    end
    int = {"kind" => "int", "value" => "1"}
    refute_empty set.call("fuzz/mutate_arithmetic/u8_add1", "endian", "middle")
    refute_empty set.call("fuzz/mutate_arithmetic/u8_add1", "endian", int)
    refute_empty set.call("fuzz/mutate_bit_flip/bit0", "input", "text")
    refute_empty set.call("fuzz/mutate_block_insert/middle", "insert_bytes", int)
    refute_empty set.call("fuzz/mutate_bit_flip/bit0", "output", "text")
    refute_empty set.call("fuzz/corpus_admit/novel_edge", "existing_edges", [{"kind" => "int", "value" => "-1"}])
    refute_empty set.call("fuzz/corpus_admit/novel_edge", "existing_edges", [{"kind" => "int", "value" => (2**64).to_s}])
    refute_empty set.call("fuzz/corpus_admit/novel_edge", "trace_edges", [true])
    refute_empty set.call("fuzz/corpus_admit/novel_edge", "admitted", int)
    refute_empty set.call("fuzz/crash_dedup/same_signature", "crash_a_frames", [int])
    refute_empty set.call("fuzz/crash_dedup/same_signature", "crash_a_kind", int)
    refute_empty set.call("fuzz/crash_dedup/same_signature", "is_duplicate", "yes")
    refute_empty set.call("fuzz/corpus_reject/bad_header", "text", int)
    refute_empty set.call("fuzz/failure_id/crash_valid", "valid", "true")
    refute_empty set.call("fuzz/private_reporting/opaque_token", "failure_id", "fuzz-bogus")
    refute_empty set.call("fuzz/private_reporting/opaque_token", "sig_hash", "xyz")
    refute_empty set.call("fuzz/private_reporting/opaque_token", "raw_payload", "text")
    refute_empty set.call("fuzz/mutate_sequence/seed42_steps4", "seed", "42")
  end

  # The privacy verdict is recomputed from the bytes: a payload whose hex IS in
  # the line must be reported as a leak.
  def test_the_privacy_verdict_is_recomputed_from_the_bytes
    cases = Fx.fuzz_cases
    c = cases.find { |x| x["op"] == "private_reporting" }
    c["raw_payload"] = {"kind" => "bytes", "hex" => "3361386631396232"} # not hex of the sig itself
    assert_equal [], evaluate(cases).problems.select { |p| p.start_with?(c["id"]) }
    c["target"] = "ct_compare"
    c["sig_hash"] = "3a8f19b2"
    c["raw_payload"] = {"kind" => "bytes", "hex" => "3a8f19b2"} # the hex text appears in the line
    problems = evaluate(cases).problems
    assert(problems.any? { |p| p.include?("payload leaked: true") })
  end

  # -- parse --------------------------------------------------------------------

  def test_parse_requires_the_exact_document_shape
    parse = ->(text) { Fuzz::Corpus.parse(text) }
    assert_equal [], parse.call('{"schema_version": 1, "cases": []}')
    [
      '{"cases": []}', '{"schema_version": 2, "cases": []}', '{"schema_version": "1", "cases": []}',
      '{"schema_version": 1, "cases": {}}', "[]", '{"schema_version": 1, "cases": [], "x": 1}'
    ].each do |bad|
      assert_raises(CorpusJson::StrictJson::JsonError, bad) { parse.call(bad) }
    end
    assert_raises(CorpusJson::StrictJson::JsonError) { Fuzz::Corpus.parse(Fx.fuzz_raw + "x") }
  end

  def test_a_case_list_above_the_cap_is_a_problem
    big = Array.new(Fuzz::Corpus::MAX_CASES + 1) { |i| {"id" => "c#{i}", "op" => "nope"} }
    assert(evaluate(big, nil).problems.any? { |p| p.include?("exceeds the cap") })
  end

  # -- work cap -----------------------------------------------------------------

  def test_the_aggregate_sequence_work_is_capped_and_a_breach_zeroes_the_budget
    cases = Fx.fuzz_cases
    template = cases.find { |c| c["op"] == "mutate_sequence" }
    # Each case copies ~10,000 bytes x 10,000 steps = 10^8 byte-steps: well past 2^24.
    big = lambda do |n|
      c = Marshal.load(Marshal.dump(template))
      c["id"] = "fuzz/mutate_sequence/big#{n}"
      c["input"] = {"kind" => "bytes", "hex" => "00" * 10_000}
      c["steps"] = {"kind" => "int", "value" => "10000"}
      c
    end
    cases = [template, big.call(1), big.call(2), Marshal.load(Marshal.dump(template)).merge("id" => "fuzz/mutate_sequence/after")]
    problems = evaluate(cases, nil).problems
    exhausted = problems.grep(/work budget of #{Fuzz::Corpus::MAX_SEQUENCE_WORK} byte-steps exhausted/o)
    assert_operator exhausted.size, :>=, 2
    # The good case AFTER the breach fails at once without running (aggregate zeroed).
    assert(exhausted.any? { |p| p.start_with?("fuzz/mutate_sequence/after:") })
    refute(problems.any? { |p| p.start_with?("fuzz/mutate_sequence/seed42_steps4:") })
  end

  def test_the_work_charged_includes_the_initial_copy
    called = []
    Fuzz::Hooks.mutate_sequence = lambda do |data, seed, steps, max_len, cap|
      called << cap
      Fuzz::Mutate.mutate_sequence_counted(data, seed, steps, max_len, cap)
    end
    assert_equal [], evaluate(Fx.fuzz_cases).problems
    assert_equal 1, called.size
    assert_equal Fuzz::Corpus::MAX_SEQUENCE_WORK, called.first
  end

  # -- sabotage -----------------------------------------------------------------

  {
    bit_flip: ->(data, off, count) { Fuzz::Mutate.bit_flip(data, off + 1, count) },
    byte_flip: ->(data, off, count) { Fuzz::Mutate.byte_flip(data, off, count + 2) },
    byte_arithmetic: ->(data, off, delta, w, e) { Fuzz::Mutate.byte_arithmetic(data, off, delta + 1, w, e) },
    boundary_insert: ->(data, off, w, val, e) { Fuzz::Mutate.boundary_insert(data, off, w, val, (e == "little") ? "big" : "little") },
    block_delete: ->(data, off, len) { Fuzz::Mutate.block_delete(data, off, len + 1) },
    block_insert: ->(data, off, ins) { Fuzz::Mutate.block_insert(data, off + 1, ins) },
    block_duplicate: ->(data, src, len, dst) { Fuzz::Mutate.block_duplicate(data, src, len, dst + 1) },
    splice: ->(t, d, ts, ds) { Fuzz::Mutate.splice(t, d, ts + 1, ds) },
    mutate_sequence: ->(d, s, steps, max_len, cap) { Fuzz::Mutate.mutate_sequence_counted(d, s + 1, steps, max_len, cap) },
    corpus_admit: ->(existing, trace) { Fuzz.corpus_admit(existing, trace).then { |a, n| [a, n + 1] } },
    duplicate_crash: ->(*) { true },
    failure_id: ->(id) { id.start_with?("fuzz-") },
    format_tap: ->(n, t, f, s) { Fuzz::Signature.format_tap_failure(n, t, f, s).sub("not ok", "ok") },
    tfv_valid: ->(_text) { true }
  }.each do |hook, broken|
    define_method(:"test_a_broken_#{hook}_hook_is_caught") do
      Fuzz::Hooks.public_send(:"#{hook}=", broken)
      refute_empty evaluate(Fx.fuzz_cases).problems
    end
  end

  def test_an_engine_that_raises_is_a_problem_for_that_case
    Fuzz::Hooks.bit_flip = ->(*) { raise "boom" }
    problems = evaluate(Fx.fuzz_cases).problems
    assert(problems.any? { |p| p.include?("could not be evaluated: RuntimeError: boom") })
  end

  def test_an_engine_that_rejects_a_registered_id_is_caught
    Fuzz::Hooks.failure_id = ->(id) { id != "fuzz-leak" }
    assert(evaluate(Fx.fuzz_cases).problems.any? { |p| p.include?("engine rejects registered ids") })
  end

  def test_hooks_reset_restores_production_behaviour
    Fuzz::Hooks.bit_flip = ->(*) { "".b }
    Fuzz::Hooks.reset
    assert_equal [], evaluate(Fx.fuzz_cases).problems
  end
end
