# frozen_string_literal: true

require_relative "test_helper"
require "json"

class TestConformance < Minitest::Test
  FIXTURE_PATH = File.expand_path("../../../../specs/fixtures/hamurabi-v1/cases.json", __dir__)

  def self.cases_data
    @cases_data ||= JSON.parse(File.read(FIXTURE_PATH))
  end

  def test_fixtures_file_exists
    assert File.exist?(FIXTURE_PATH), "cases.json fixture must exist at #{FIXTURE_PATH}"
  end

  def test_all_cases_conformance
    cases = self.class.cases_data["cases"]
    refute_empty cases, "cases.json fixture must contain test cases"

    cases.each do |c|
      case_id = c["id"]
      initial = CodingAdventures::Hamurabi::HamurabiState.from_h(c["initial_state"])

      if c["turns"]
        current = initial
        c["turns"].each do |turn_info|
          turn_num = turn_info["turn"]
          order = CodingAdventures::Hamurabi::TurnOrder.from(turn_info["order"])
          expected_state = turn_info["expected_state"]
          expected_report = turn_info["expected_report"]

          next_state = CodingAdventures::Hamurabi.step_year(current, order)

          expected_state.each do |field, expected_val|
            actual_val = next_state[field]
            actual_val = actual_val.to_s if field == "status"
            assert_equal(
              expected_val,
              actual_val,
              "[#{case_id}] turn #{turn_num}: field '#{field}' mismatch"
            )
          end

          actual_report = CodingAdventures::Hamurabi.format_report(next_state)
          assert_equal(
            expected_report,
            actual_report,
            "[#{case_id}] turn #{turn_num}: report mismatch"
          )

          current = next_state
        end
      elsif c["expected_error"]
        expected_err = c["expected_error"]
        order = CodingAdventures::Hamurabi::TurnOrder.from(c["order"])

        val_err = assert_raises(CodingAdventures::Hamurabi::HamurabiError, "[#{case_id}] validate_order should fail") do
          CodingAdventures::Hamurabi.validate_order(initial, order)
        end
        assert_equal expected_err, val_err.code, "[#{case_id}] validate_order error code mismatch"

        step_err = assert_raises(CodingAdventures::Hamurabi::HamurabiError, "[#{case_id}] step_year should fail") do
          CodingAdventures::Hamurabi.step_year(initial, order)
        end
        assert_equal expected_err, step_err.code, "[#{case_id}] step_year error code mismatch"
      end
    end
  end
end
