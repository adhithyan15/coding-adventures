# frozen_string_literal: true

# =============================================================================
# Wave — Sinusoidal Wave Modeling from First Principles
# =============================================================================
#
# A sinusoidal wave is the most fundamental waveform in physics. Every periodic
# signal — sound, light, radio, alternating current — can be decomposed into a
# sum of sinusoidal waves (this is the core idea behind Fourier analysis).
#
# A sinusoidal wave is described by three parameters:
#
#   y(t) = A * sin(2 * PI * f * t + phi)
#
# Where:
#   A     = amplitude   — the peak height of the wave (must be >= 0)
#   f     = frequency   — how many full cycles occur per second (in Hz, must be > 0)
#   t     = time        — the independent variable (in seconds)
#   phi   = phase       — shifts the wave left or right in time (in radians)
#
# ## What Each Parameter Means Physically
#
# **Amplitude (A):** Think of a guitar string. When you pluck it hard, it
# vibrates with a large amplitude — the string moves far from its rest
# position, producing a loud sound. A gentle pluck gives a small amplitude
# and a quiet sound. Amplitude is always non-negative: a wave with amplitude
# zero is just a flat line (no wave at all).
#
# **Frequency (f):** This is how fast the wave oscillates. A 440 Hz wave
# completes 440 full cycles every second — this is the note A above middle C.
# Double the frequency to 880 Hz and you get the A one octave higher. The
# frequency must be strictly positive: a wave with zero frequency would never
# oscillate (it would be a constant, not a wave).
#
# **Phase (phi):** Phase shifts the wave in time. A phase of 0 means the wave
# starts at zero and rises. A phase of PI/2 means the wave starts at its peak
# (like a cosine wave). Phase is measured in radians, where 2*PI corresponds
# to one full cycle.
#
# ## Derived Quantities
#
# **Period (T):** The time for one full cycle. T = 1/f. A 2 Hz wave has a
# period of 0.5 seconds.
#
# **Angular frequency (omega):** omega = 2 * PI * f. This converts frequency
# from "cycles per second" to "radians per second", which is the natural unit
# for the sin() function. The wave equation becomes: y(t) = A * sin(omega * t + phi).
#
# =============================================================================

require_relative "../../trig/lib/trig"

class Wave
  # ---------------------------------------------------------------------------
  # Accessors
  # ---------------------------------------------------------------------------
  #
  # These are the three defining parameters of a sinusoidal wave. They are
  # set once at construction and cannot be changed afterward (immutable design).

  attr_reader :amplitude, :frequency, :phase

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------
  #
  # Creates a new sinusoidal wave with the given amplitude, frequency, and
  # optional phase offset.
  #
  # @param amplitude [Numeric] peak height of the wave (must be >= 0)
  # @param frequency [Numeric] cycles per second in Hz (must be > 0)
  # @param phase [Numeric] phase offset in radians (default: 0.0)
  #
  # @raise [ArgumentError] if amplitude is negative or frequency is not positive
  #
  # ## Why These Validations?
  #
  # - Amplitude < 0 is physically meaningless. A negative amplitude would just
  #   be a phase shift of PI (flipping the wave upside down), so we require
  #   non-negative amplitude and let the caller use phase for inversion.
  #
  # - Frequency <= 0 is problematic. Zero frequency means no oscillation (the
  #   period would be infinite). Negative frequency is mathematically equivalent
  #   to positive frequency with a phase shift, so we require strictly positive.

  def initialize(amplitude, frequency, phase = 0.0)
    if amplitude < 0
      raise ArgumentError, "Amplitude must be non-negative, got #{amplitude}"
    end

    if frequency <= 0
      raise ArgumentError, "Frequency must be positive, got #{frequency}"
    end

    @amplitude = amplitude.to_f
    @frequency = frequency.to_f
    @phase = phase.to_f
  end

  # ---------------------------------------------------------------------------
  # Period
  # ---------------------------------------------------------------------------
  #
  # The period is the duration of one complete cycle, measured in seconds.
  #
  #   T = 1 / f
  #
  # A 1 Hz wave has a period of 1 second.
  # A 440 Hz wave (concert A) has a period of about 2.27 milliseconds.
  # A 1 MHz radio wave has a period of 1 microsecond.
  #
  # @return [Float] the period in seconds

  def period
    1.0 / @frequency
  end

  # ---------------------------------------------------------------------------
  # Angular Frequency
  # ---------------------------------------------------------------------------
  #
  # Angular frequency converts from cycles per second to radians per second:
  #
  #   omega = 2 * PI * f
  #
  # Why radians? The sine function's argument is in radians. One full cycle
  # is 2*PI radians. So if we want f cycles per second, the argument to sin()
  # must advance by 2*PI*f radians each second.
  #
  # @return [Float] angular frequency in radians per second

  def angular_frequency
    2.0 * Trig::PI * @frequency
  end

  # ---------------------------------------------------------------------------
  # Evaluate the Wave at Time t
  # ---------------------------------------------------------------------------
  #
  # Computes the value of the wave at a given point in time:
  #
  #   y(t) = A * sin(2 * PI * f * t + phi)
  #
  # This is the fundamental equation of a sinusoidal wave. Let's trace through
  # a concrete example:
  #
  #   Wave: amplitude=3, frequency=2 Hz, phase=0
  #
  #   At t=0:     y = 3 * sin(0)       = 3 * 0    = 0      (starts at zero)
  #   At t=0.125: y = 3 * sin(PI/2)    = 3 * 1    = 3      (reaches peak)
  #   At t=0.25:  y = 3 * sin(PI)      = 3 * 0    = 0      (back to zero)
  #   At t=0.375: y = 3 * sin(3*PI/2)  = 3 * (-1) = -3     (reaches trough)
  #   At t=0.5:   y = 3 * sin(2*PI)    = 3 * 0    = 0      (completes one cycle)
  #
  # With a phase of PI/2, the wave starts at its peak instead:
  #
  #   At t=0:     y = 3 * sin(PI/2)    = 3 * 1    = 3      (starts at peak)
  #
  # @param t [Numeric] time in seconds
  # @return [Float] the wave's value at time t

  def evaluate(t)
    @amplitude * Trig.sin(2.0 * Trig::PI * @frequency * t + @phase)
  end
end
