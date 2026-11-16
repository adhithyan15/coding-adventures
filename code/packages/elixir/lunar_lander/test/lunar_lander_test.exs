defmodule CodingAdventures.LunarLanderTest do
  use ExUnit.Case, async: true

  import Bitwise

  alias CodingAdventures.LunarLander

  alias CodingAdventures.LunarLander.{
    BurnCommand,
    Error,
    LanderState,
    PaintRect,
    PaintText
  }

  test "initial flight invariants" do
    flight = LunarLander.new_flight()

    assert flight.time_seconds == 0
    assert flight.altitude_mft == 120_000_000
    assert flight.velocity_mft_s == -3_000_000
    assert flight.fuel_lbs == 16_500
    assert flight.dry_mass_lbs == 16_500
    assert flight.status == :in_flight
    assert flight.impact_velocity_mft_s == nil

    assert LunarLander.g_lunar_mft_s2() == 5_310
    assert LunarLander.dry_mass_lbs() == 16_500
    assert LunarLander.initial_fuel_lbs() == 16_500
    assert LunarLander.total_initial_mass_lbs() == 33_000
    assert LunarLander.initial_altitude_mft() == 120_000_000
    assert LunarLander.initial_velocity_mft_s() == -3_000_000
    assert LunarLander.exhaust_velocity_mft_s() == 8_000_000
    assert LunarLander.max_burn_rate_lbs_s() == 200
    assert LunarLander.min_duration_s() == 1
    assert LunarLander.max_duration_s() == 10
    assert LunarLander.max_time_seconds() == 86_400
    assert LunarLander.max_altitude_mft() == 200_000_000
    assert LunarLander.min_velocity_mft_s() == -10_000_000
    assert LunarLander.max_velocity_mft_s() == 10_000_000
  end

  test "div_euclid exactness across dividend/divisor sign combinations" do
    # Canonical verification vector from advisor #88
    assert LunarLander.div_euclid(-46_070, -5_310) == 9

    assert LunarLander.div_euclid(-7, 3) == -3
    assert LunarLander.div_euclid(7, 3) == 2
    assert LunarLander.div_euclid(7, -3) == -2
    assert LunarLander.div_euclid(0, 5) == 0
    assert LunarLander.div_euclid(10, 2) == 5
    assert LunarLander.div_euclid(-10, 2) == -5
    assert LunarLander.div_euclid(-10, -2) == 5
    assert LunarLander.div_euclid(10, -2) == -5
  end

  test "isqrt_u128 exactness" do
    assert LunarLander.isqrt_u128(0) == 0
    assert LunarLander.isqrt_u128(1) == 1
    assert LunarLander.isqrt_u128(2) == 1
    assert LunarLander.isqrt_u128(3) == 1
    assert LunarLander.isqrt_u128(4) == 2
    assert LunarLander.isqrt_u128(15) == 3
    assert LunarLander.isqrt_u128(16) == 4
    assert LunarLander.isqrt_u128(10_274_400_000_000) == 3_205_370

    u64_max = (1 <<< 64) - 1
    assert LunarLander.isqrt_u128(u64_max * u64_max) == u64_max

    u128_max = (1 <<< 128) - 1
    assert LunarLander.isqrt_u128(u128_max) == u64_max
  end

  test "format_with_commas exactness" do
    assert LunarLander.format_with_commas(0) == "0"
    assert LunarLander.format_with_commas(7) == "7"
    assert LunarLander.format_with_commas(99) == "99"
    assert LunarLander.format_with_commas(123) == "123"
    assert LunarLander.format_with_commas(1_234) == "1,234"
    assert LunarLander.format_with_commas(12_345) == "12,345"
    assert LunarLander.format_with_commas(120_000) == "120,000"
    assert LunarLander.format_with_commas(-3_000) == "-3,000"
    assert LunarLander.format_with_commas(16_500) == "16,500"
    assert LunarLander.format_with_commas(-1_234_567) == "-1,234,567"
  end

  test "paint scene telemetry structure and dimensions" do
    state = LunarLander.new_flight()
    scene = LunarLander.to_paint_scene(state, 0)

    assert scene.width == 76.0
    assert scene.height == 5.0
    assert scene.background == "#000000"
    assert is_list(scene.instructions)
    assert length(scene.instructions) >= 1

    [text_inst | rect_insts] = scene.instructions
    assert %PaintText{} = text_inst
    assert text_inst.kind == :text
    assert text_inst.x == 0.0
    assert text_inst.y == 0.0

    for rect <- rect_insts do
      assert %PaintRect{} = rect
      assert rect.kind == :rect
      assert rect.fill == "#00ff00"
    end

    scene2 = LunarLander.telemetry_scene(state)
    assert scene2.width == 76.0
    assert scene2.height == 5.0

    # Test scene when altitude and fuel gauges are 0
    zero_state = %{state | altitude_mft: 0, fuel_lbs: 0, status: {:landed, :perfect}}
    zero_scene = LunarLander.to_paint_scene(zero_state, 0)
    assert length(zero_scene.instructions) == 1

    # Test scene with string status
    str_scene = LunarLander.to_paint_scene(%{state | status: "InFlight"}, 50)
    assert str_scene.width == 76.0

    unknown_scene = LunarLander.to_paint_scene(%{state | status: "NonExistentStatus"}, 0)
    assert unknown_scene.width == 76.0
  end

  test "render_telemetry_ascii and render_telemetry 5x76 HUD layout" do
    state = LunarLander.new_flight()
    hud = LunarLander.render_telemetry_ascii(state, 0)

    lines = String.split(hud, "\n")
    assert length(lines) == 5, "Telemetry HUD must contain exactly 5 lines"

    for {line, idx} <- Enum.with_index(lines) do
      grapheme_count = length(String.graphemes(line))

      assert grapheme_count == 76,
             "Line #{idx} grapheme count #{grapheme_count} != 76: #{inspect(line)}"
    end

    assert String.contains?(hud, "LUNAR LANDER TELEMETRY")
    assert String.contains?(hud, "TIME:")
    assert String.contains?(hud, "ALTITUDE:")
    assert String.contains?(hud, "VELOCITY:")
    assert String.contains?(hud, "FUEL:")
    assert String.contains?(hud, "BURN:")
    assert String.contains?(hud, "RATE:")
    assert String.contains?(hud, "DESCENT")
    assert String.contains?(hud, "█")

    # Mid-flight state with partial gauge fills containing both █ and ░
    mid_state = %{state | fuel_lbs: 8_250, altitude_mft: 60_000_000}
    hud_mid = LunarLander.render_telemetry_ascii(mid_state, 100)
    assert String.contains?(hud_mid, "█")
    assert String.contains?(hud_mid, "░")

    # render_telemetry equivalence
    hud2 = LunarLander.render_telemetry(state)
    assert hud2 == hud
  end

  test "telemetry states and flight outcome symbols" do
    state = LunarLander.new_flight()

    # Ascent
    hud_ascent = LunarLander.render_telemetry_ascii(%{state | velocity_mft_s: 50_000}, 100)
    assert String.contains?(hud_ascent, "ASCENT")
    assert String.contains?(hud_ascent, "▲")

    # Hover
    hud_hover = LunarLander.render_telemetry_ascii(%{state | velocity_mft_s: 0}, 50)
    assert String.contains?(hud_hover, "HOVER")
    assert String.contains?(hud_hover, "◆")

    # Landed outcomes
    outcomes = [
      {:perfect, "PERFECT"},
      {:hard, "HARD"},
      {:crash, "CRASH"},
      {:fatal, "FATAL"},
      {:crater, "CRATER"}
    ]

    for {outcome, text} <- outcomes do
      landed_state = %{state | status: {:landed, outcome}, impact_velocity_mft_s: -1_000}
      hud_landed = LunarLander.render_telemetry_ascii(landed_state, 0)
      assert String.contains?(hud_landed, "STATUS:"), "Missing STATUS: for #{text}"
      assert String.contains?(hud_landed, text), "Missing outcome text #{text}"
      assert String.contains?(hud_landed, "★"), "Missing ★ for #{text}"
    end
  end

  test "raising variants validate_burn! and step_burn!" do
    state = LunarLander.new_flight()
    cmd = %BurnCommand{burn_rate_lbs_s: 100, duration_seconds: 5}

    assert :ok == LunarLander.validate_burn!(state, cmd)
    next_state = LunarLander.step_burn!(state, cmd)
    assert next_state.time_seconds == 5

    invalid_cmd = %BurnCommand{burn_rate_lbs_s: 300, duration_seconds: 5}

    assert_raise Error, fn ->
      LunarLander.validate_burn!(state, invalid_cmd)
    end

    assert_raise Error, fn ->
      LunarLander.step_burn!(state, invalid_cmd)
    end
  end

  test "touchdown solver a_half == 0 branch" do
    # When a_net == 1, div_euclid(1, 2) == 0 -> a_half == 0
    state = %LanderState{
      time_seconds: 10,
      altitude_mft: 500,
      velocity_mft_s: -1_000,
      fuel_lbs: 72,
      dry_mass_lbs: 16_500,
      status: :in_flight,
      impact_velocity_mft_s: nil
    }

    cmd = %BurnCommand{burn_rate_lbs_s: 11, duration_seconds: 1}
    assert {:ok, landed} = LunarLander.step_burn(state, cmd)
    assert landed.altitude_mft == 0
    assert landed.status == {:landed, :perfect}
  end

  test "plain maps passed directly to validate_burn and step_burn" do
    state_map = %{
      time_seconds: 0,
      altitude_mft: 120_000_000,
      velocity_mft_s: -3_000_000,
      fuel_lbs: 16_500,
      dry_mass_lbs: 16_500,
      status: :in_flight,
      impact_velocity_mft_s: nil
    }

    cmd_map = %{burn_rate_lbs_s: 100, duration_seconds: 5}

    assert :ok == LunarLander.validate_burn(state_map, cmd_map)
    assert {:ok, %LanderState{}} = LunarLander.step_burn(state_map, cmd_map)

    # String keys
    str_state_map = %{
      "time_seconds" => 0,
      "altitude_mft" => 120_000_000,
      "velocity_mft_s" => -3_000_000,
      "fuel_lbs" => 16_500,
      "dry_mass_lbs" => 16_500,
      "status" => "InFlight",
      "impact_velocity_mft_s" => nil
    }

    str_cmd_map = %{"burn_rate_lbs_s" => 100, "duration_seconds" => 5}

    assert :ok == LunarLander.validate_burn(str_state_map, str_cmd_map)
    assert {:ok, %LanderState{}} = LunarLander.step_burn(str_state_map, str_cmd_map)

    # Coerce failure paths
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn("not_a_map", cmd_map)
    assert {:error, :arithmetic_overflow} = LunarLander.step_burn("not_a_map", cmd_map)
    assert {:error, :invalid_duration} = LunarLander.validate_burn(state_map, "not_a_cmd")
    assert {:error, :invalid_duration} = LunarLander.step_burn(state_map, "not_a_cmd")
  end

  test "adversarial CWE-843 type confusion rejection" do
    valid_state = LunarLander.new_flight()
    valid_cmd = %BurnCommand{burn_rate_lbs_s: 50, duration_seconds: 5}

    # Reject float in command
    float_burn = %{burn_rate_lbs_s: 50.0, duration_seconds: 5}
    assert {:error, :invalid_duration} = LunarLander.validate_burn(valid_state, float_burn)
    assert {:error, :invalid_duration} = LunarLander.step_burn(valid_state, float_burn)

    float_dur = %{burn_rate_lbs_s: 50, duration_seconds: 5.5}
    assert {:error, :invalid_duration} = LunarLander.validate_burn(valid_state, float_dur)

    # Reject strings, booleans, atoms, lists in command
    assert {:error, :invalid_duration} =
             LunarLander.validate_burn(valid_state, %{burn_rate_lbs_s: "50", duration_seconds: 5})

    assert {:error, :invalid_duration} =
             LunarLander.validate_burn(valid_state, %{burn_rate_lbs_s: true, duration_seconds: 5})

    # Reject non-integers in state
    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(%{valid_state | fuel_lbs: 100.5}, valid_cmd)

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(%{valid_state | altitude_mft: "120000000"}, valid_cmd)

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(%{valid_state | velocity_mft_s: nil}, valid_cmd)

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(%{valid_state | dry_mass_lbs: [16_500]}, valid_cmd)
  end

  test "adversarial CWE-190 and CWE-369 boundary rejection" do
    valid_cmd = %BurnCommand{burn_rate_lbs_s: 50, duration_seconds: 5}

    # Corrupted dry mass (CWE-369 division by zero vector)
    zero_mass_state = %{LunarLander.new_flight() | dry_mass_lbs: 0}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(zero_mass_state, valid_cmd)
    assert {:error, :arithmetic_overflow} = LunarLander.step_burn(zero_mass_state, valid_cmd)

    wrong_mass_state = %{LunarLander.new_flight() | dry_mass_lbs: 16_501}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(wrong_mass_state, valid_cmd)

    # Fuel boundaries
    neg_fuel_state = %{LunarLander.new_flight() | fuel_lbs: -1}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(neg_fuel_state, valid_cmd)

    excess_fuel_state = %{LunarLander.new_flight() | fuel_lbs: 16_501}

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(excess_fuel_state, valid_cmd)

    # Altitude boundaries
    zero_alt_state = %{LunarLander.new_flight() | altitude_mft: 0}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(zero_alt_state, valid_cmd)

    neg_alt_state = %{LunarLander.new_flight() | altitude_mft: -1}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(neg_alt_state, valid_cmd)

    excess_alt_state = %{LunarLander.new_flight() | altitude_mft: 200_000_001}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(excess_alt_state, valid_cmd)

    # Velocity boundaries
    min_vel_state = %{LunarLander.new_flight() | velocity_mft_s: -10_000_001}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(min_vel_state, valid_cmd)

    max_vel_state = %{LunarLander.new_flight() | velocity_mft_s: 10_000_001}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(max_vel_state, valid_cmd)

    # Time boundaries
    max_time_state = %{LunarLander.new_flight() | time_seconds: 86_400}
    add_one = %BurnCommand{burn_rate_lbs_s: 0, duration_seconds: 1}
    assert {:error, :arithmetic_overflow} = LunarLander.validate_burn(max_time_state, add_one)

    # Command burn rate boundaries
    excessive_burn = %BurnCommand{burn_rate_lbs_s: 201, duration_seconds: 5}

    assert {:error, :excessive_burn_rate} =
             LunarLander.validate_burn(LunarLander.new_flight(), excessive_burn)

    neg_burn = %{burn_rate_lbs_s: -1, duration_seconds: 5}

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(LunarLander.new_flight(), neg_burn)

    u32_overflow_burn = %{burn_rate_lbs_s: 0x1_0000_0000, duration_seconds: 5}

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(LunarLander.new_flight(), u32_overflow_burn)

    # Command duration boundaries
    zero_dur = %BurnCommand{burn_rate_lbs_s: 50, duration_seconds: 0}

    assert {:error, :invalid_duration} =
             LunarLander.validate_burn(LunarLander.new_flight(), zero_dur)

    excessive_dur = %BurnCommand{burn_rate_lbs_s: 50, duration_seconds: 11}

    assert {:error, :invalid_duration} =
             LunarLander.validate_burn(LunarLander.new_flight(), excessive_dur)

    # InFlight with impact velocity
    corrupted_impact_state = %{LunarLander.new_flight() | impact_velocity_mft_s: -5000}

    assert {:error, :arithmetic_overflow} =
             LunarLander.validate_burn(corrupted_impact_state, valid_cmd)

    # Flight already ended
    for outcome <- [:perfect, :hard, :crash, :fatal, :crater] do
      landed_state = %{LunarLander.new_flight() | status: {:landed, outcome}}
      assert {:error, :flight_already_ended} = LunarLander.validate_burn(landed_state, valid_cmd)
      assert {:error, :flight_already_ended} = LunarLander.step_burn(landed_state, valid_cmd)
    end
  end

  test "serialization and string conversions" do
    assert LunarLander.status_to_string(:in_flight) == "InFlight"
    assert LunarLander.status_to_string({:landed, :perfect}) == "Landed(Perfect)"
    assert LunarLander.status_to_string({:landed, :hard}) == "Landed(Hard)"
    assert LunarLander.status_to_string({:landed, :crash}) == "Landed(Crash)"
    assert LunarLander.status_to_string({:landed, :fatal}) == "Landed(Fatal)"
    assert LunarLander.status_to_string({:landed, :crater}) == "Landed(Crater)"
    assert LunarLander.status_to_string("InFlight") == "InFlight"

    assert LunarLander.status_from_string("InFlight") == {:ok, :in_flight}
    assert LunarLander.status_from_string("Landed(Perfect)") == {:ok, {:landed, :perfect}}
    assert LunarLander.status_from_string("Landed(Hard)") == {:ok, {:landed, :hard}}
    assert LunarLander.status_from_string("Landed(Crash)") == {:ok, {:landed, :crash}}
    assert LunarLander.status_from_string("Landed(Fatal)") == {:ok, {:landed, :fatal}}
    assert LunarLander.status_from_string("Landed(Crater)") == {:ok, {:landed, :crater}}
    assert LunarLander.status_from_string("Invalid") == {:error, :invalid_status}

    assert LunarLander.error_to_string(:flight_already_ended) == "FlightAlreadyEnded"
    assert LunarLander.error_to_string(:excessive_burn_rate) == "ExcessiveBurnRate"
    assert LunarLander.error_to_string(:invalid_duration) == "InvalidDuration"
    assert LunarLander.error_to_string(:arithmetic_overflow) == "ArithmeticOverflow"
    assert LunarLander.error_to_string("Custom") == "Custom"

    assert LunarLander.error_from_string("FlightAlreadyEnded") == {:ok, :flight_already_ended}
    assert LunarLander.error_from_string("ExcessiveBurnRate") == {:ok, :excessive_burn_rate}
    assert LunarLander.error_from_string("InvalidDuration") == {:ok, :invalid_duration}
    assert LunarLander.error_from_string("ArithmeticOverflow") == {:ok, :arithmetic_overflow}
    assert LunarLander.error_from_string("Unknown") == {:error, :unknown_error}
  end

  test "Error exception constructors and messages" do
    e1 = Error.exception(:flight_already_ended)
    assert e1.reason == :flight_already_ended
    assert e1.message == "Flight has already ended"

    e2 = Error.exception(:excessive_burn_rate)
    assert e2.message == "Burn rate exceeds maximum engine capacity"

    e3 = Error.exception(:invalid_duration)
    assert e3.message == "Duration is invalid (must be between 1 and 10 seconds)"

    e4 = Error.exception(:arithmetic_overflow)
    assert e4.message == "Arithmetic overflow or coordinate bounds violation"

    e5 = Error.exception(:other_unknown)
    assert String.contains?(e5.message, "other_unknown")

    e6 = Error.exception(reason: :invalid_duration)
    assert e6.reason == :invalid_duration

    e7 = Error.exception(reason: :invalid_duration, message: "Custom message")
    assert e7.message == "Custom message"
  end

  test "struct new/1 constructors" do
    assert {:ok, %LanderState{}} = LanderState.new(LunarLander.new_flight())

    state_map = %{
      time_seconds: 10,
      altitude_mft: 80_000_000,
      velocity_mft_s: -2_000_000,
      fuel_lbs: 10_000,
      dry_mass_lbs: 16_500,
      status: "InFlight",
      impact_velocity_mft_s: nil
    }

    assert {:ok, %LanderState{} = s} = LanderState.new(state_map)
    assert s.time_seconds == 10

    # All status variants in LanderState.new
    for status_str <- [
          "InFlight",
          "Landed(Perfect)",
          "Landed(Hard)",
          "Landed(Crash)",
          "Landed(Fatal)",
          "Landed(Crater)"
        ] do
      assert {:ok, %LanderState{}} = LanderState.new(%{state_map | status: status_str})
    end

    assert {:ok, %LanderState{}} = LanderState.new(%{state_map | status: {:landed, :crash}})

    string_map = %{
      "time_seconds" => 10,
      "altitude_mft" => 80_000_000,
      "velocity_mft_s" => -2_000_000,
      "fuel_lbs" => 10_000,
      "dry_mass_lbs" => 16_500,
      "status" => "InFlight",
      "impact_velocity_mft_s" => :null
    }

    assert {:ok, %LanderState{} = s2} = LanderState.new(string_map)
    assert s2.impact_velocity_mft_s == nil

    assert {:error, :arithmetic_overflow} = LanderState.new(%{state_map | status: "BadStatus"})

    assert {:error, :arithmetic_overflow} =
             LanderState.new(%{state_map | impact_velocity_mft_s: "bad"})

    assert {:error, :arithmetic_overflow} = LanderState.new(%{invalid: true})
    assert {:error, :arithmetic_overflow} = LanderState.new("string_not_map")

    cmd = %BurnCommand{burn_rate_lbs_s: 50, duration_seconds: 5}
    assert {:ok, ^cmd} = BurnCommand.new(cmd)

    assert {:ok, %BurnCommand{burn_rate_lbs_s: 50}} =
             BurnCommand.new(%{burn_rate_lbs_s: 50, duration_seconds: 5})

    assert {:ok, %BurnCommand{burn_rate_lbs_s: 50}} =
             BurnCommand.new(%{"burn_rate_lbs_s" => 50, "duration_seconds" => 5})

    assert {:error, :invalid_duration} =
             BurnCommand.new(%{burn_rate_lbs_s: "50", duration_seconds: 5})

    assert {:error, :invalid_duration} = BurnCommand.new("not_a_map")
  end
end
