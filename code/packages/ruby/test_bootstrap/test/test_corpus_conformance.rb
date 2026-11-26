# frozen_string_literal: true

require_relative "test_helper"

# Replays the whole committed pbt-v1 corpus and carries its negative controls
# (VER05h section 6).  A conformance check that cannot fail proves nothing, so
# every case is corrupted in every way its op defines and the run MUST fail,
# naming exactly that case.
class TestCorpusConformance < HookedTest
  def evaluate(cases, expected = Pbt::Corpus::PBTV1_CASE_COUNT)
    Pbt::Corpus.evaluate(cases, expected)
  end

  def test_the_committed_corpus_conforms_exactly
    cases = Fx.cases
    assert_equal 25, cases.size
    report = evaluate(cases)
    assert_equal [], report.problems
    assert_equal 25, report.evaluated
  end

  def test_the_independent_reference_cases_are_the_ones_we_replay
    ids = Fx.cases.map { |c| c["id"] }
    assert_includes ids, "pbt/shrink_int/54321_le_100"
    assert_includes ids, "pbt/shrink_bytes/len_lt_5"
    assert_equal ids.uniq, ids
  end

  # -- corruption ---------------------------------------------------------------

  def corrupt_value(node)
    case node
    when true, false then !node
    when String then node + "x"
    when Array then node.empty? ? [{"kind" => "int", "value" => "0"}] : [corrupt_value(node[0])] + node[1..]
    when Hash
      if node["kind"] == "int"
        {"kind" => "int", "value" => (node["value"].to_i + 1).to_s}
      else
        hex = node["hex"]
        {"kind" => "bytes", "hex" => if hex.empty?
                                       "00"
                                     else
                                       ((hex[0] == "0") ? "1" : "0") + hex[1..]
                                     end}
      end
    end
  end

  # [label, lambda(case) that corrupts it in place] for every corruption an op defines.
  def corruptions(op)
    case op
    when "gen_primitive", "gen_bounded", "gen_bytes"
      [["outputs", ->(c) { c["outputs"] = corrupt_value(c["outputs"]) }],
        ["short outputs", ->(c) { c["outputs"] = c["outputs"][0...-1] }]]
    when "shrink_int", "shrink_refinement", "shrink_list", "shrink_bytes"
      [["minimal", ->(c) { c["expected_minimal"] = corrupt_value(c["expected_minimal"]) }],
        ["steps+1", ->(c) { c["expected_steps"] = corrupt_value(c["expected_steps"]) }]]
    when "replay_consistency"
      [["initial_fail", ->(c) { c["initial_fail"] = corrupt_value(c["initial_fail"]) }],
        ["minimal", ->(c) { c["expected_minimal"] = corrupt_value(c["expected_minimal"]) }]]
    when "filter_exhausted"
      [["failure id", ->(c) { c["failure_id"] = "property-panic" }],
        ["verdict", ->(c) { c["verdict"] = "pass" }]]
    when "failure_id_valid"
      [["valid flipped", ->(c) { c["valid"] = !c["valid"] }]]
    when "corpus_reject"
      [["a valid TFV in place of a reject text", ->(c) { c["text"] = "tfv 1\n" }]]
    end
  end

  def test_every_corruption_of_every_case_fails_and_names_only_that_case
    total = 0
    Fx.cases.each_with_index do |original, index|
      corruptions(original["op"]).each do |label, mutate|
        cases = Fx.cases
        mutate.call(cases[index])
        report = evaluate(cases)
        refute_empty report.problems, "#{original["id"]}: #{label} was not detected"
        report.problems.each do |problem|
          assert problem.start_with?("#{original["id"]}: "), "#{label}: problem names another case: #{problem}"
        end
        total += 1
      end
    end
    assert_operator total, :>=, 40
  end

  # -- structural controls ------------------------------------------------------

  def test_a_dropped_case_fails
    cases = Fx.cases
    cases.delete_at(3)
    report = evaluate(cases)
    assert(report.problems.any? { |p| p.include?("expected 25 cases, got 24") })
  end

  def test_a_truncated_corpus_fails
    report = evaluate(Fx.cases.first(5))
    refute_empty report.problems
    assert(report.problems.any? { |p| p.include?("operations never exercised") })
    assert(report.problems.any? { |p| p.include?("registered failure ids missing") })
  end

  def test_dropping_the_unregistered_id_case_fails
    cases = Fx.cases.reject { |c| c["id"] == "pbt/failure_id/unregistered" }
    assert(evaluate(cases, nil).problems.any? { |p| p.include?("unregistered-id rejection") })
  end

  def test_an_empty_corpus_fails
    refute_empty evaluate([]).problems
  end

  def test_unknown_op_extra_field_and_missing_field_are_problems
    cases = Fx.cases
    cases[0]["op"] = "mystery"
    assert(evaluate(cases).problems.any? { |p| p.include?("unknown op") })

    cases = Fx.cases
    cases[0]["extra"] = 1
    assert(evaluate(cases).problems.any? { |p| p.start_with?("pbt/gen_primitive/bool_seed0: fields") })

    cases = Fx.cases
    cases[0].delete("count")
    assert(evaluate(cases).problems.any? { |p| p.include?("fields") })
  end

  def test_malformed_ids_and_cases_are_problems_not_crashes
    cases = Fx.cases
    cases[1]["id"] = cases[0]["id"]
    assert(evaluate(cases).problems.any? { |p| p.include?("duplicate or missing case id") })

    cases = Fx.cases
    cases[0]["id"] = ["x"]
    assert(evaluate(cases).problems.any? { |p| p.include?("duplicate or missing case id") })

    cases = Fx.cases
    cases[2] = 7
    assert(evaluate(cases).problems.any? { |p| p.include?("case is not an object") })

    cases = Fx.cases
    cases[2] = {"op" => "gen_bool"}
    assert(evaluate(cases).problems.any? { |p| p.include?("case missing a string id") })
  end

  def test_hostile_sizes_are_reported_not_executed
    cases = Fx.cases
    cases[0]["count"] = {"kind" => "int", "value" => "1000000000"}
    problem = evaluate(cases).problems.first
    assert_match(/^pbt\/gen_primitive\/bool_seed0: gen_primitive could not be evaluated/, problem)
    assert_includes problem, "outside [0, 10000]"

    cases = Fx.cases
    cases[0]["count"] = {"kind" => "int", "value" => "5"} # outputs has 8
    assert(evaluate(cases).problems.any? { |p| p.include?("exactly count (5)") })

    cases = Fx.cases
    i = cases.index { |c| c["op"] == "gen_bytes" }
    cases[i]["max_len"] = {"kind" => "int", "value" => "10000"}
    cases[i]["count"] = {"kind" => "int", "value" => "10000"}
    cases[i]["outputs"] = Array.new(10_000) { {"kind" => "bytes", "hex" => ""} }
    assert(evaluate(cases).problems.any? { |p| p.include?("generation budget") })
  end

  def test_malformed_values_are_problems
    [{"kind" => "int", "value" => "007"}, {"kind" => "int", "value" => 5}, {"kind" => "int"},
      {"kind" => "bytes", "hex" => "FF"}, {"kind" => "bytes", "hex" => "abc"}, nil, {"kind" => "float"}].each do |bad|
      cases = Fx.cases
      cases[0]["seed"] = bad
      assert(evaluate(cases).problems.any? { |p| p.start_with?("pbt/gen_primitive/bool_seed0:") }, bad.inspect)
    end
    cases = Fx.cases
    cases[0]["seed"] = {"kind" => "int", "value" => "9" * 1001}
    assert(evaluate(cases).problems.any? { |p| p.include?("digits") })
  end

  def test_unknown_property_and_wrong_property_family_are_problems
    cases = Fx.cases
    i = cases.index { |c| c["op"] == "shrink_int" }
    cases[i]["property"] = "no_such_property"
    assert(evaluate(cases).problems.any? { |p| p.include?("unknown property") })
    cases[i]["property"] = "list_sum_le_20"
    assert(evaluate(cases).problems.any? { |p| p.include?("unknown property") })
  end

  # -- parse --------------------------------------------------------------------

  def test_parse_requires_the_exact_document_shape
    parse = ->(text) { Pbt::Corpus.parse(text) }
    assert_equal [], parse.call('{"schema_version": 1, "cases": []}')
    [
      '{"cases": []}', '{"schema_version": 2, "cases": []}', '{"schema_version": "1", "cases": []}',
      '{"schema_version": 1, "cases": {}}', "[]", '{"schema_version": 1, "cases": [], "x": 1}'
    ].each do |bad|
      assert_raises(CorpusJson::StrictJson::JsonError, bad) { parse.call(bad) }
    end
  end

  def test_the_whole_corpus_text_round_trips_through_the_strict_parser
    assert_equal 25, Fx.cases.size
    assert_raises(CorpusJson::StrictJson::JsonError) { Pbt::Corpus.parse(Fx.raw + "x") }
  end

  def test_a_case_list_above_the_cap_is_a_problem
    big = Array.new(Pbt::Corpus::MAX_CASES + 1) { |i| {"id" => "c#{i}", "op" => "nope"} }
    report = evaluate(big, nil)
    assert(report.problems.any? { |p| p.include?("exceeds the cap") })
  end

  # -- sabotage -----------------------------------------------------------------

  def test_a_changed_bisection_is_caught
    Pbt::Hooks.bisect_candidate = ->(current, diff) { current - (diff >> 1) - 1 }
    refute_empty evaluate(Fx.cases).problems
  end

  def test_a_changed_unit_step_is_caught
    Pbt::Hooks.unit_candidate = ->(current, diff) { current - ((diff <=> 0) * 2) }
    refute_empty evaluate(Fx.cases).problems
  end

  def test_floor_division_is_equivalent_to_the_shift
    Pbt::Hooks.bisect_candidate = ->(current, diff) { current - (diff / 2) } # floor vs the same on positives
    Pbt::Hooks.unit_candidate = ->(current, diff) { current - (diff <=> 0) }
    # Ruby's / floors like >>, so this one is equivalent: the corpus must still conform.
    assert_equal [], evaluate(Fx.cases).problems
  end

  def test_a_skipped_deletion_phase_is_caught
    Pbt::Hooks.skip_deletion_phase = true
    refute_empty evaluate(Fx.cases).problems
  end

  def test_a_shifted_rng_is_caught
    Pbt::Hooks.new_rng = ->(seed) { Prng::SplitMix64.new(seed + 1) }
    refute_empty evaluate(Fx.cases).problems
  end

  def test_an_engine_that_raises_is_a_problem_for_that_case
    Pbt::Hooks.new_rng = ->(_seed) { raise "rng boom" }
    problems = evaluate(Fx.cases).problems
    assert(problems.any? { |p| p.include?("could not be evaluated: RuntimeError: rng boom") })
  end

  def test_hooks_reset_restores_production_behaviour
    Pbt::Hooks.skip_deletion_phase = true
    Pbt::Hooks.reset
    assert_equal [], evaluate(Fx.cases).problems
  end
end
