# frozen_string_literal: true

# =============================================================================
# Tests for the Wave Class
# =============================================================================
#
# These tests verify that our sinusoidal wave implementation correctly models
# the physics of wave motion. We test each property individually, then verify
# the wave equation against known mathematical identities.
#
# Since we're working with floating-point arithmetic and our own Trig library
# (which uses series approximations), we use assert_in_delta with a tolerance
# of 1e-10 to account for tiny rounding errors.
# =============================================================================

require "minitest/autorun"
require_relative "../lib/wave"

class TestWave < Minitest::Test
  DELTA = 1e-10

  # ---------------------------------------------------------------------------
  # Basic wave evaluation: sin(0) = 0
  # ---------------------------------------------------------------------------
  #
  # A wave with zero phase evaluated at t=0 should give exactly zero, because:
  #   y(0) = A * sin(2*PI*f*0 + 0) = A * sin(0) = A * 0 = 0

  def test_wave_at_t_zero_with_zero_phase_is_zero
    wave = Wave.new(5.0, 1.0, 0.0)
    assert_in_delta 0.0, wave.evaluate(0.0), DELTA
  end

  # ---------------------------------------------------------------------------
  # Peak value: a 1 Hz wave reaches amplitude at t=0.25
  # ---------------------------------------------------------------------------
  #
  # For a 1 Hz wave with zero phase:
  #   y(0.25) = A * sin(2*PI*1*0.25) = A * sin(PI/2) = A * 1 = A
  #
  # At a quarter of the period, the sine function reaches its maximum value of 1,
  # so the wave equals the amplitude.

  def test_1hz_wave_reaches_amplitude_at_quarter_period
    wave = Wave.new(3.0, 1.0, 0.0)
    assert_in_delta 3.0, wave.evaluate(0.25), DELTA
  end

  # ---------------------------------------------------------------------------
  # Periodicity: the wave repeats every period
  # ---------------------------------------------------------------------------
  #
  # A fundamental property of sinusoidal waves: y(t) = y(t + T) for all t,
  # where T = 1/f is the period. We test this at several time points.

  def test_wave_is_periodic
    wave = Wave.new(2.0, 3.0, 0.5)
    period = wave.period

    [0.0, 0.1, 0.2, 0.33].each do |t|
      assert_in_delta wave.evaluate(t), wave.evaluate(t + period), DELTA,
        "Wave should repeat at t=#{t} and t=#{t + period}"
    end
  end

  # ---------------------------------------------------------------------------
  # Phase shift: PI/2 phase starts at peak
  # ---------------------------------------------------------------------------
  #
  # With phase = PI/2:
  #   y(0) = A * sin(PI/2) = A * 1 = A
  #
  # This is because sin(PI/2) = 1. A phase of PI/2 turns a sine wave into
  # a cosine wave (cos(x) = sin(x + PI/2)), which starts at its maximum.

  def test_phase_pi_over_2_starts_at_peak
    wave = Wave.new(4.0, 1.0, Trig::PI / 2.0)
    assert_in_delta 4.0, wave.evaluate(0.0), DELTA
  end

  # ---------------------------------------------------------------------------
  # Period calculation
  # ---------------------------------------------------------------------------
  #
  # Period T = 1/f. A 2 Hz wave completes 2 cycles per second, so each cycle
  # takes 0.5 seconds.

  def test_period
    wave = Wave.new(1.0, 2.0)
    assert_in_delta 0.5, wave.period, DELTA
  end

  # ---------------------------------------------------------------------------
  # Angular frequency calculation
  # ---------------------------------------------------------------------------
  #
  # Angular frequency omega = 2*PI*f. For a 1 Hz wave, omega = 2*PI.

  def test_angular_frequency
    wave = Wave.new(1.0, 1.0)
    assert_in_delta 2.0 * Trig::PI, wave.angular_frequency, DELTA
  end

  # ---------------------------------------------------------------------------
  # Angular frequency for higher frequency
  # ---------------------------------------------------------------------------
  #
  # For a 5 Hz wave, omega = 2*PI*5 = 10*PI.

  def test_angular_frequency_higher
    wave = Wave.new(1.0, 5.0)
    assert_in_delta 10.0 * Trig::PI, wave.angular_frequency, DELTA
  end

  # ---------------------------------------------------------------------------
  # Validation: negative amplitude raises ArgumentError
  # ---------------------------------------------------------------------------
  #
  # Amplitude represents the peak displacement from equilibrium. A negative
  # value is physically meaningless (it's equivalent to a positive amplitude
  # with a PI phase shift), so we reject it.

  def test_negative_amplitude_raises
    assert_raises(ArgumentError) { Wave.new(-1.0, 1.0) }
  end

  # ---------------------------------------------------------------------------
  # Validation: zero frequency raises ArgumentError
  # ---------------------------------------------------------------------------
  #
  # A wave with zero frequency would never oscillate — it would be a constant
  # function, not a wave. The period would be infinite (1/0). We reject it.

  def test_zero_frequency_raises
    assert_raises(ArgumentError) { Wave.new(1.0, 0.0) }
  end

  # ---------------------------------------------------------------------------
  # Validation: negative frequency raises ArgumentError
  # ---------------------------------------------------------------------------
  #
  # Negative frequency is mathematically equivalent to positive frequency with
  # a phase shift of PI. We enforce positive frequency for clarity.

  def test_negative_frequency_raises
    assert_raises(ArgumentError) { Wave.new(1.0, -1.0) }
  end

  # ---------------------------------------------------------------------------
  # Zero amplitude produces a flat wave
  # ---------------------------------------------------------------------------
  #
  # A wave with zero amplitude is just a flat line at y=0. This is valid
  # (it's the limit as amplitude approaches zero) and should work at any time.

  def test_zero_amplitude_is_flat
    wave = Wave.new(0.0, 1.0)
    [0.0, 0.25, 0.5, 1.0].each do |t|
      assert_in_delta 0.0, wave.evaluate(t), DELTA
    end
  end

  # ---------------------------------------------------------------------------
  # Default phase is zero
  # ---------------------------------------------------------------------------
  #
  # When no phase is provided, it defaults to 0.0.

  def test_default_phase_is_zero
    wave = Wave.new(1.0, 1.0)
    assert_in_delta 0.0, wave.phase, DELTA
  end

  # ---------------------------------------------------------------------------
  # Trough value: wave reaches -amplitude at 3/4 period
  # ---------------------------------------------------------------------------
  #
  # For a 1 Hz wave with zero phase:
  #   y(0.75) = A * sin(2*PI*0.75) = A * sin(3*PI/2) = A * (-1) = -A

  def test_wave_reaches_negative_amplitude_at_three_quarter_period
    wave = Wave.new(2.0, 1.0, 0.0)
    assert_in_delta(-2.0, wave.evaluate(0.75), DELTA)
  end

  # ---------------------------------------------------------------------------
  # Attribute readers
  # ---------------------------------------------------------------------------
  #
  # Verify that the constructor stores values correctly and that they are
  # accessible via the attribute readers.

  def test_attribute_readers
    wave = Wave.new(3.5, 440.0, 1.23)
    assert_in_delta 3.5, wave.amplitude, DELTA
    assert_in_delta 440.0, wave.frequency, DELTA
    assert_in_delta 1.23, wave.phase, DELTA
  end
end
