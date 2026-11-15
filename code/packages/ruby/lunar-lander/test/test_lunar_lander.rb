# frozen_string_literal: true

require_relative "test_helper"

class TestLunarLander < Minitest::Test
  include CodingAdventures::LunarLander

  def test_initial_flight_invariants
    flight = CodingAdventures::LunarLander.new_flight
    assert flight.frozen?, "new_flight must return a frozen LanderState"
    assert_equal 0, flight.time_seconds
    assert_equal INITIAL_ALTITUDE_MFT, flight.altitude_mft
    assert_equal INITIAL_VELOCITY_MFT_S, flight.velocity_mft_s
    assert_equal INITIAL_FUEL_LBS, flight.fuel_lbs
    assert_equal DRY_MASS_LBS, flight.dry_mass_lbs
    assert_equal "InFlight", flight.status.to_s
    assert flight.status.in_flight?
    refute flight.status.landed?
    assert_nil flight.impact_velocity_mft_s
  end

  def test_trunc_div_exactness
    assert_equal 1, CodingAdventures::LunarLander.trunc_div(1500, 1000)
    assert_equal(-1, CodingAdventures::LunarLander.trunc_div(-1500, 1000))
    assert_equal(-1, CodingAdventures::LunarLander.trunc_div(1500, -1000))
    assert_equal 1, CodingAdventures::LunarLander.trunc_div(-1500, -1000)
    assert_equal 8, CodingAdventures::LunarLander.trunc_div(-46070, -5310)
    assert_equal(-8, CodingAdventures::LunarLander.trunc_div(46070, -5310))
    assert_equal 0, CodingAdventures::LunarLander.trunc_div(500, 1000)
    assert_equal 0, CodingAdventures::LunarLander.trunc_div(-500, 1000)

    assert_raises(ZeroDivisionError) do
      CodingAdventures::LunarLander.trunc_div(100, 0)
    end
  end

  def test_div_euclid_exactness
    # Critical test vector: div_euclid(-46070, -5310) == 9
    assert_equal 9, CodingAdventures::LunarLander.div_euclid(-46070, -5310)
    assert_equal 2, CodingAdventures::LunarLander.div_euclid(7, 3)
    assert_equal(-3, CodingAdventures::LunarLander.div_euclid(-7, 3))
    assert_equal(-2, CodingAdventures::LunarLander.div_euclid(7, -3))
    assert_equal 3, CodingAdventures::LunarLander.div_euclid(-7, -3)

    # Invariant: remainder r = a - q * b satisfies 0 <= r < |b|
    [
      [-46070, -5310],
      [46070, -5310],
      [-46070, 5310],
      [46070, 5310],
      [-7, 3],
      [7, 3],
      [-7, -3],
      [7, -3]
    ].each do |a, b|
      q = CodingAdventures::LunarLander.div_euclid(a, b)
      r = a - q * b
      assert r >= 0, "Euclidean remainder must be non-negative: a=#{a}, b=#{b}, q=#{q}, r=#{r}"
      assert r < b.abs, "Euclidean remainder must be strictly less than |b|: a=#{a}, b=#{b}, q=#{q}, r=#{r}"
    end
  end

  def test_isqrt_u128_exactness
    assert_equal 0, CodingAdventures::LunarLander.isqrt_u128(0)
    assert_equal 1, CodingAdventures::LunarLander.isqrt_u128(1)
    assert_equal 1, CodingAdventures::LunarLander.isqrt_u128(2)
    assert_equal 1, CodingAdventures::LunarLander.isqrt_u128(3)
    assert_equal 2, CodingAdventures::LunarLander.isqrt_u128(4)
    assert_equal 3, CodingAdventures::LunarLander.isqrt_u128(15)
    assert_equal 4, CodingAdventures::LunarLander.isqrt_u128(16)
    assert_equal 3_205_370, CodingAdventures::LunarLander.isqrt_u128(10_274_400_000_000)

    u64_max = (1 << 64) - 1
    assert_equal u64_max, CodingAdventures::LunarLander.isqrt_u128(u64_max * u64_max)

    u128_max = (1 << 128) - 1
    assert_equal u64_max, CodingAdventures::LunarLander.isqrt_u128(u128_max)
  end

  def test_paint_vm_ascii_telemetry_hud
    flight = CodingAdventures::LunarLander.new_flight
    scene = CodingAdventures::LunarLander.to_paint_scene(flight, 0)
    assert scene.frozen?
    assert_equal 76.0, scene.width
    assert_equal 5.0, scene.height
    assert_equal "#000000", scene.background

    hud = CodingAdventures::LunarLander.render_telemetry_ascii(flight, 0)
    lines = hud.lines.map(&:chomp)
    assert_equal 5, lines.length, "Telemetry HUD must contain exactly 5 lines"

    lines.each_with_index do |line, idx|
      assert_equal 76, line.chars.count, "Line #{idx} length mismatch: #{line.inspect}"
    end

    assert_includes hud, "LUNAR LANDER TELEMETRY"
    assert_includes hud, "TIME:"
    assert_includes hud, "ALTITUDE:"
    assert_includes hud, "VELOCITY:"
    assert_includes hud, "FUEL:"
    assert_includes hud, "BURN:"
    assert_includes hud, "RATE:"
    assert_includes hud, "DESCENT"
    assert_includes hud, "█"

    # Mid-flight partial gauge test containing both █ and ░
    mid_state = LanderState.new(
      time_seconds: 50,
      altitude_mft: 60_000_000,
      velocity_mft_s: -1_500_000,
      fuel_lbs: 8_250,
      dry_mass_lbs: DRY_MASS_LBS,
      status: FlightStatus.in_flight,
      impact_velocity_mft_s: nil
    )
    hud_mid = CodingAdventures::LunarLander.render_telemetry_ascii(mid_state, 100)
    assert_includes hud_mid, "█"
    assert_includes hud_mid, "░"

    # Convenience helpers
    scene2 = CodingAdventures::LunarLander.telemetry_scene(flight)
    assert_equal 76.0, scene2.width
    hud2 = CodingAdventures::LunarLander.render_telemetry(flight)
    assert_equal hud, hud2
  end

  def test_telemetry_states_and_outcomes
    # InFlight ascent
    ascent_state = LanderState.new(
      time_seconds: 10,
      altitude_mft: 110_000_000,
      velocity_mft_s: 50_000,
      fuel_lbs: 15_000,
      dry_mass_lbs: DRY_MASS_LBS,
      status: FlightStatus.in_flight,
      impact_velocity_mft_s: nil
    )
    hud_ascent = CodingAdventures::LunarLander.render_telemetry_ascii(ascent_state, 100)
    assert_includes hud_ascent, "ASCENT"
    assert_includes hud_ascent, "▲"

    # InFlight hover
    hover_state = LanderState.new(
      time_seconds: 20,
      altitude_mft: 100_000_000,
      velocity_mft_s: 0,
      fuel_lbs: 14_000,
      dry_mass_lbs: DRY_MASS_LBS,
      status: FlightStatus.in_flight,
      impact_velocity_mft_s: nil
    )
    hud_hover = CodingAdventures::LunarLander.render_telemetry_ascii(hover_state, 50)
    assert_includes hud_hover, "HOVER"
    assert_includes hud_hover, "◆"

    # Landed outcomes
    outcomes = [
      [LandingOutcome::PERFECT, "PERFECT"],
      [LandingOutcome::HARD, "HARD"],
      [LandingOutcome::CRASH, "CRASH"],
      [LandingOutcome::FATAL, "FATAL"],
      [LandingOutcome::CRATER, "CRATER"]
    ]

    outcomes.each do |outcome, text|
      landed_state = LanderState.new(
        time_seconds: 100,
        altitude_mft: 0,
        velocity_mft_s: -1000,
        fuel_lbs: 2000,
        dry_mass_lbs: DRY_MASS_LBS,
        status: FlightStatus.landed(outcome),
        impact_velocity_mft_s: -1000
      )
      hud_landed = CodingAdventures::LunarLander.render_telemetry_ascii(landed_state, 0)
      assert_includes hud_landed, "STATUS:", "Landed HUD missing STATUS: label"
      assert_includes hud_landed, text, "Landed HUD missing outcome text #{text}"
      assert_includes hud_landed, "★", "Landed HUD missing star symbol"
    end
  end

  def test_burn_command_api
    cmd1 = BurnCommand.new(100, 5)
    assert cmd1.frozen?
    assert_equal 100, cmd1.burn_rate_lbs_s
    assert_equal 5, cmd1.duration_seconds
    assert_equal 100, cmd1[:burn_rate_lbs_s]
    assert_equal 5, cmd1["duration_seconds"]

    cmd2 = BurnCommand.new(burn_rate_lbs_s: 100, duration_seconds: 5)
    assert_equal cmd1, cmd2

    cmd3 = BurnCommand.from({ "burn_rate_lbs_s" => 100, "duration_seconds" => 5 })
    assert_equal cmd1, cmd3

    cmd4 = BurnCommand.from([100, 5])
    assert_equal cmd1, cmd4
  end

  def test_lander_state_api
    flight = CodingAdventures::LunarLander.new_flight
    assert_equal 0, flight[:time_seconds]
    assert_equal INITIAL_ALTITUDE_MFT, flight["altitude_mft"]
    assert_equal "InFlight", flight[:status].to_s

    h = flight.to_h
    flight2 = LanderState.from_h(h)
    assert_equal flight, flight2
  end
end
