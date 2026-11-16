# frozen_string_literal: true

require_relative "test_helper"

class TestAdversarialCWE < Minitest::Test
  include CodingAdventures::LunarLander

  def setup
    @state = CodingAdventures::LunarLander.new_flight
    @valid_cmd = BurnCommand.new(burn_rate_lbs_s: 100, duration_seconds: 5)
  end

  # ============================================================================
  # CWE-843: Type Confusion (reject Float, String, Boolean, nil, Rational, Complex)
  # ============================================================================

  def test_reject_float_in_command
    [
      { burn_rate_lbs_s: 50.5, duration_seconds: 5 },
      { burn_rate_lbs_s: 50, duration_seconds: 5.0 },
      { burn_rate_lbs_s: 0.0, duration_seconds: 5 }
    ].each do |bad_cmd|
      assert_raises(LanderError) do
        CodingAdventures::LunarLander.validate_burn(@state, bad_cmd)
      end
      assert_raises(LanderError) do
        CodingAdventures::LunarLander.step_burn(@state, bad_cmd)
      end
    end
  end

  def test_reject_string_in_command
    [
      { burn_rate_lbs_s: "50", duration_seconds: 5 },
      { burn_rate_lbs_s: 50, duration_seconds: "5" },
      { burn_rate_lbs_s: "zero", duration_seconds: 5 }
    ].each do |bad_cmd|
      assert_raises(LanderError) do
        CodingAdventures::LunarLander.validate_burn(@state, bad_cmd)
      end
      assert_raises(LanderError) do
        CodingAdventures::LunarLander.step_burn(@state, bad_cmd)
      end
    end
  end

  def test_reject_boolean_in_command
    [
      { burn_rate_lbs_s: true, duration_seconds: 5 },
      { burn_rate_lbs_s: 50, duration_seconds: false }
    ].each do |bad_cmd|
      assert_raises(LanderError) do
        CodingAdventures::LunarLander.validate_burn(@state, bad_cmd)
      end
    end
  end

  def test_reject_nil_in_command
    [
      { burn_rate_lbs_s: nil, duration_seconds: 5 },
      { burn_rate_lbs_s: 50, duration_seconds: nil }
    ].each do |bad_cmd|
      assert_raises(LanderError) do
        CodingAdventures::LunarLander.validate_burn(@state, bad_cmd)
      end
    end
  end

  def test_reject_float_in_state_fields
    bad_fields = [
      { altitude_mft: 60_000_000.5 },
      { velocity_mft_s: -1500.0 },
      { fuel_lbs: 8250.25 },
      { dry_mass_lbs: 16500.0 },
      { time_seconds: 10.0 }
    ]

    bad_fields.each do |override|
      attrs = @state.to_h.merge(override)
      bad_state = LanderState.new(
        time_seconds: attrs[:time_seconds] || attrs["time_seconds"],
        altitude_mft: attrs[:altitude_mft] || attrs["altitude_mft"],
        velocity_mft_s: attrs[:velocity_mft_s] || attrs["velocity_mft_s"],
        fuel_lbs: attrs[:fuel_lbs] || attrs["fuel_lbs"],
        dry_mass_lbs: attrs[:dry_mass_lbs] || attrs["dry_mass_lbs"],
        status: attrs[:status] || attrs["status"],
        impact_velocity_mft_s: attrs[:impact_velocity_mft_s] || attrs["impact_velocity_mft_s"]
      )
      err = assert_raises(ArithmeticOverflowError) do
        CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
      end
      assert_equal "ArithmeticOverflow", err.code
    end
  end

  def test_reject_rational_and_complex_inputs
    bad_cmd = { burn_rate_lbs_s: Rational(100, 1), duration_seconds: 5 }
    assert_raises(LanderError) do
      CodingAdventures::LunarLander.validate_burn(@state, bad_cmd)
    end

    bad_cmd2 = { burn_rate_lbs_s: Complex(50, 0), duration_seconds: 5 }
    assert_raises(LanderError) do
      CodingAdventures::LunarLander.validate_burn(@state, bad_cmd2)
    end
  end

  # ============================================================================
  # CWE-190: Integer Overflow and Coordinate Boundaries
  # ============================================================================

  def test_altitude_boundaries
    # Altitude <= 0 in flight
    [0, -1, -120_000_000, -1 << 63].each do |bad_alt|
      bad_state = make_state(altitude_mft: bad_alt)
      assert_raises(ArithmeticOverflowError) do
        CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
      end
    end

    # Altitude > MAX_ALTITUDE_MFT
    [MAX_ALTITUDE_MFT + 1, 1 << 62, (1 << 63) - 1].each do |bad_alt|
      bad_state = make_state(altitude_mft: bad_alt)
      assert_raises(ArithmeticOverflowError) do
        CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
      end
    end
  end

  def test_velocity_boundaries
    # Velocity < MIN_VELOCITY_MFT_S
    [MIN_VELOCITY_MFT_S - 1, -1 << 63].each do |bad_vel|
      bad_state = make_state(velocity_mft_s: bad_vel)
      assert_raises(ArithmeticOverflowError) do
        CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
      end
    end

    # Velocity > MAX_VELOCITY_MFT_S
    [MAX_VELOCITY_MFT_S + 1, (1 << 63) - 1].each do |bad_vel|
      bad_state = make_state(velocity_mft_s: bad_vel)
      assert_raises(ArithmeticOverflowError) do
        CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
      end
    end
  end

  def test_fuel_boundaries
    [INITIAL_FUEL_LBS + 1, (1 << 32) - 1, -1].each do |bad_fuel|
      bad_state = make_state(fuel_lbs: bad_fuel)
      assert_raises(ArithmeticOverflowError) do
        CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
      end
    end
  end

  def test_mission_time_limits
    # Already exceeded
    bad_state = make_state(time_seconds: MAX_TIME_SECONDS + 1)
    assert_raises(ArithmeticOverflowError) do
      CodingAdventures::LunarLander.validate_burn(bad_state, @valid_cmd)
    end

    # Step pushes beyond 86,400 s
    edge_state = make_state(time_seconds: MAX_TIME_SECONDS)
    cmd = BurnCommand.new(burn_rate_lbs_s: 0, duration_seconds: 1)
    assert_raises(ArithmeticOverflowError) do
      CodingAdventures::LunarLander.validate_burn(edge_state, cmd)
    end
  end

  def test_command_burn_rate_limits
    [MAX_BURN_RATE_LBS_S + 1, 250, (1 << 32) - 1].each do |bad_burn|
      cmd = BurnCommand.new(burn_rate_lbs_s: bad_burn, duration_seconds: 5)
      assert_raises(ExcessiveBurnRateError) do
        CodingAdventures::LunarLander.validate_burn(@state, cmd)
      end
    end
  end

  def test_command_duration_limits
    [0, -1, MAX_DURATION_S + 1, 11, (1 << 32) - 1].each do |bad_dur|
      cmd = BurnCommand.new(burn_rate_lbs_s: 50, duration_seconds: bad_dur)
      assert_raises(InvalidDurationError) do
        CodingAdventures::LunarLander.validate_burn(@state, cmd)
      end
    end
  end

  # ============================================================================
  # CWE-369: Division by Zero Prevention
  # ============================================================================

  def test_cwe369_corrupted_dry_mass_and_fuel_zero
    hostile_state = LanderState.new(
      time_seconds: 0,
      altitude_mft: 100_000_000,
      velocity_mft_s: -2_000_000,
      fuel_lbs: 0,
      dry_mass_lbs: 0,
      status: FlightStatus.in_flight,
      impact_velocity_mft_s: nil
    )
    assert_raises(ArithmeticOverflowError) do
      CodingAdventures::LunarLander.validate_burn(hostile_state, @valid_cmd)
    end
    assert_raises(ArithmeticOverflowError) do
      CodingAdventures::LunarLander.step_burn(hostile_state, @valid_cmd)
    end
  end

  # ============================================================================
  # CWE-502 & VER04m: Immutability & Safe Serialization
  # ============================================================================

  def test_freeze_fresh_value_semantics
    next_state = CodingAdventures::LunarLander.step_burn(@state, @valid_cmd)
    assert next_state.frozen?, "Returned state must be frozen (VER04m fresh)"
    assert_raises(FrozenError) do
      next_state.instance_variable_set(:@altitude_mft, 999)
    end

    scene = CodingAdventures::LunarLander.to_paint_scene(@state, 0)
    assert scene.frozen?, "Returned PaintScene must be frozen"
    assert scene.instructions.frozen?, "PaintScene instructions must be frozen"
    assert_raises(FrozenError) do
      scene.instructions << PaintRect.new(x: 0, y: 0, width: 1, height: 1)
    end
  end

  def test_landed_state_refuses_further_burns
    landed_state = LanderState.new(
      time_seconds: 50,
      altitude_mft: 0,
      velocity_mft_s: -3000,
      fuel_lbs: 10_000,
      dry_mass_lbs: DRY_MASS_LBS,
      status: FlightStatus.landed(LandingOutcome::PERFECT),
      impact_velocity_mft_s: -3000
    )
    err = assert_raises(FlightAlreadyEndedError) do
      CodingAdventures::LunarLander.validate_burn(landed_state, @valid_cmd)
    end
    assert_equal "FlightAlreadyEnded", err.code

    err2 = assert_raises(FlightAlreadyEndedError) do
      CodingAdventures::LunarLander.step_burn(landed_state, @valid_cmd)
    end
    assert_equal "FlightAlreadyEnded", err2.code
  end

  private

  def make_state(**overrides)
    h = @state.to_h.merge(overrides)
    LanderState.new(
      time_seconds: h[:time_seconds] || h["time_seconds"],
      altitude_mft: h[:altitude_mft] || h["altitude_mft"],
      velocity_mft_s: h[:velocity_mft_s] || h["velocity_mft_s"],
      fuel_lbs: h[:fuel_lbs] || h["fuel_lbs"],
      dry_mass_lbs: h[:dry_mass_lbs] || h["dry_mass_lbs"],
      status: h[:status] || h["status"],
      impact_velocity_mft_s: h[:impact_velocity_mft_s] || h["impact_velocity_mft_s"]
    )
  end
end
