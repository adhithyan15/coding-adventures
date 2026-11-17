# frozen_string_literal: true

require_relative "test_helper"
require "json"

class TestConformance < Minitest::Test
  FIXTURE_PATH = File.expand_path("../../../../specs/fixtures/lunar-lander-v1/cases.json", __dir__)

  def self.cases_data
    @cases_data ||= JSON.parse(File.read(FIXTURE_PATH))
  end

  def test_fixtures_file_exists
    assert File.exist?(FIXTURE_PATH), "cases.json fixture must exist at #{FIXTURE_PATH}"
  end

  def test_all_39_corpus_cases
    data = self.class.cases_data
    assert_equal 1, data["version"]
    assert_equal "lunar_lander_deterministic_flight", data["suite"]

    cases = data["cases"]
    assert_equal 39, cases.length, "Corpus must have exactly 39 test cases"

    cases.each do |c|
      case_id = c["id"]
      initial = CodingAdventures::LunarLander::LanderState.from_h(c["initial_state"])

      if c["turns"] && !c["turns"].empty?
        state = initial
        c["turns"].each do |turn_info|
          turn_num = turn_info["turn"]
          cmd = CodingAdventures::LunarLander::BurnCommand.from(turn_info["command"])
          expected = turn_info["expected_state"]

          # Validate burn precedes step
          assert CodingAdventures::LunarLander.validate_burn(state, cmd),
                 "[#{case_id}] turn #{turn_num} validation failed"

          next_state = CodingAdventures::LunarLander.step_burn(state, cmd)

          assert next_state.frozen?, "[#{case_id}] turn #{turn_num} returned state must be frozen"

          assert_equal expected["time_seconds"], next_state.time_seconds,
                       "[#{case_id}] turn #{turn_num} time mismatch"
          assert_equal expected["altitude_mft"], next_state.altitude_mft,
                       "[#{case_id}] turn #{turn_num} altitude mismatch"
          assert_equal expected["velocity_mft_s"], next_state.velocity_mft_s,
                       "[#{case_id}] turn #{turn_num} velocity mismatch"
          assert_equal expected["fuel_lbs"], next_state.fuel_lbs,
                       "[#{case_id}] turn #{turn_num} fuel mismatch"
          assert_equal expected["dry_mass_lbs"], next_state.dry_mass_lbs,
                       "[#{case_id}] turn #{turn_num} dry mass mismatch"
          assert_equal expected["status"], next_state.status.to_s,
                       "[#{case_id}] turn #{turn_num} status mismatch"
          if expected["impact_velocity_mft_s"].nil?
            assert_nil next_state.impact_velocity_mft_s,
                       "[#{case_id}] turn #{turn_num} impact velocity must be nil"
          else
            assert_equal expected["impact_velocity_mft_s"], next_state.impact_velocity_mft_s,
                         "[#{case_id}] turn #{turn_num} impact velocity mismatch"
          end

          state = next_state
        end
      elsif c["expected_error"]
        cmd = CodingAdventures::LunarLander::BurnCommand.from(c["command"])
        expected_err = c["expected_error"]

        val_err = assert_raises(CodingAdventures::LunarLander::LanderError,
                                "[#{case_id}] validate_burn should raise LanderError") do
          CodingAdventures::LunarLander.validate_burn(initial, cmd)
        end
        assert_equal expected_err, val_err.code,
                     "[#{case_id}] validate_burn error code mismatch"

        step_err = assert_raises(CodingAdventures::LunarLander::LanderError,
                                 "[#{case_id}] step_burn should raise LanderError") do
          CodingAdventures::LunarLander.step_burn(initial, cmd)
        end
        assert_equal expected_err, step_err.code,
                     "[#{case_id}] step_burn error code mismatch"
      else
        flunk "Case #{case_id} has neither turns nor expected_error"
      end
    end
  end
end
