defmodule CodingAdventures.LunarLander.ConformanceTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.LunarLander
  alias CodingAdventures.LunarLander.{BurnCommand, LanderState}

  @fixture Path.expand("../../../../specs/fixtures/lunar-lander-v1/cases.json", __DIR__)

  defp json_to_state(map) do
    {:ok, status} = LunarLander.status_from_string(map["status"])
    impact = map["impact_velocity_mft_s"]
    norm_impact = if impact == :null, do: nil, else: impact

    %LanderState{
      time_seconds: map["time_seconds"],
      altitude_mft: map["altitude_mft"],
      velocity_mft_s: map["velocity_mft_s"],
      fuel_lbs: map["fuel_lbs"],
      dry_mass_lbs: map["dry_mass_lbs"],
      status: status,
      impact_velocity_mft_s: norm_impact
    }
  end

  defp json_to_command(map) do
    %BurnCommand{
      burn_rate_lbs_s: map["burn_rate_lbs_s"],
      duration_seconds: map["duration_seconds"]
    }
  end

  test "verifies all 39 cases in lunar-lander-v1 cases.json corpus" do
    assert File.exists?(@fixture), "Missing cases.json fixture at #{@fixture}"

    json_text = File.read!(@fixture)
    data = :json.decode(json_text)
    assert data["version"] == 1
    assert data["suite"] == "lunar_lander_deterministic_flight"

    cases = data["cases"]
    assert length(cases) == 39

    for tc <- cases do
      id = tc["id"]
      init_state = json_to_state(tc["initial_state"])
      turns = tc["turns"]

      if turns != nil and length(turns) > 0 do
        Enum.reduce(turns, init_state, fn turn, state ->
          turn_num = turn["turn"]
          cmd = json_to_command(turn["command"])
          expected = json_to_state(turn["expected_state"])

          assert :ok == LunarLander.validate_burn(state, cmd),
                 "#{id} turn #{turn_num}: validate_burn failed"

          assert {:ok, next_state} = LunarLander.step_burn(state, cmd),
                 "#{id} turn #{turn_num}: step_burn failed"

          assert next_state.time_seconds == expected.time_seconds,
                 "#{id} turn #{turn_num}: time_seconds mismatch"

          assert next_state.altitude_mft == expected.altitude_mft,
                 "#{id} turn #{turn_num}: altitude_mft mismatch"

          assert next_state.velocity_mft_s == expected.velocity_mft_s,
                 "#{id} turn #{turn_num}: velocity_mft_s mismatch"

          assert next_state.fuel_lbs == expected.fuel_lbs,
                 "#{id} turn #{turn_num}: fuel_lbs mismatch"

          assert next_state.dry_mass_lbs == expected.dry_mass_lbs,
                 "#{id} turn #{turn_num}: dry_mass_lbs mismatch"

          assert next_state.status == expected.status,
                 "#{id} turn #{turn_num}: status mismatch"

          assert next_state.impact_velocity_mft_s == expected.impact_velocity_mft_s,
                 "#{id} turn #{turn_num}: impact_velocity_mft_s mismatch"

          next_state
        end)
      else
        cmd = json_to_command(tc["command"])
        expected_err_str = tc["expected_error"]
        {:ok, expected_err} = LunarLander.error_from_string(expected_err_str)

        assert {:error, val_err} = LunarLander.validate_burn(init_state, cmd),
               "#{id}: validate_burn should fail"

        assert val_err == expected_err,
               "#{id}: validate_burn error #{inspect(val_err)} != expected #{inspect(expected_err)}"

        assert LunarLander.error_to_string(val_err) == expected_err_str

        assert {:error, step_err} = LunarLander.step_burn(init_state, cmd),
               "#{id}: step_burn should fail"

        assert step_err == expected_err,
               "#{id}: step_burn error #{inspect(step_err)} != expected #{inspect(expected_err)}"

        assert LunarLander.error_to_string(step_err) == expected_err_str
      end
    end
  end
end
