defmodule Wave do
  @moduledoc """
  # Wave --- Sinusoidal Wave Generator

  This module models a **sinusoidal wave**, the most fundamental periodic
  waveform in physics and signal processing. A sinusoidal wave is described
  by three parameters:

  ## The Three Parameters

  1. **Amplitude** (`A`) --- the peak displacement from zero. Think of it
     as "how tall" the wave is. A speaker cone moves further for louder
     sounds (higher amplitude). Must be non-negative (>= 0).

  2. **Frequency** (`f`) --- how many complete cycles occur per second,
     measured in Hertz (Hz). A 440 Hz wave completes 440 full oscillations
     every second --- that's the note A above middle C. Must be positive (> 0).

  3. **Phase** (`phi`) --- the starting offset of the wave, in radians.
     A phase of 0 means the wave starts at zero and rises. A phase of
     pi/2 means the wave starts at its peak. Default is 0.

  ## The Wave Equation

  The displacement at time `t` (in seconds) is:

      y(t) = A * sin(2 * pi * f * t + phi)

  Breaking this down:
  - `2 * pi * f` converts frequency (cycles/second) to **angular frequency**
    (radians/second), often written as omega (w).
  - `2 * pi * f * t` gives the angle (in radians) at time `t`.
  - Adding `phi` shifts the starting point of the wave.
  - Multiplying by `A` scales the result from [-1, 1] to [-A, A].

  ## Derived Quantities

  - **Period** (`T = 1/f`) --- the time for one complete cycle, in seconds.
    A 2 Hz wave has a period of 0.5 seconds.
  - **Angular frequency** (`w = 2 * pi * f`) --- how fast the angle changes,
    in radians per second.

  ## Truth Table (1 Hz, Amplitude 1, Phase 0)

      t (seconds) | angle (radians) | sin(angle) | y(t)
      ------------|-----------------|------------|-----
      0.00        | 0               | 0.0        | 0.0
      0.25        | pi/2            | 1.0        | 1.0   (peak)
      0.50        | pi              | 0.0        | 0.0   (zero crossing)
      0.75        | 3*pi/2          | -1.0       | -1.0  (trough)
      1.00        | 2*pi            | 0.0        | 0.0   (back to start)

  ## Dependencies

  This package depends on `Trig` (a from-scratch trigonometry library)
  for `sin`, `cos`, and `pi`. No host trigonometric or square-root routine is used;
  `:math.fmod/2` provides only scalar signed-remainder reduction.

  ## Layer

  This is a PHY01 (physics layer 1) package. It depends on:
  - `trig` (PHY00) --- sine function and pi constant
  """

  # ---------------------------------------------------------------------------
  # Struct Definition
  # ---------------------------------------------------------------------------

  # A Wave struct holds the three defining parameters. Using a struct
  # (rather than a plain map) gives us:
  #   1. Compile-time field validation --- misspelled keys cause errors
  #   2. Pattern matching on the struct type in function heads
  #   3. Clear documentation of what a "wave" is

  defstruct [:amplitude, :frequency, :phase]

  @max_float Float.max_finite()
  @two_pi 2.0 * Trig.pi()
  @max_frequency @max_float / @two_pi
  @period_overflow_frequency 1.0 / @max_float

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------

  @doc """
  Creates a new wave with the given amplitude, frequency, and optional phase.

  Returns `{:ok, %Wave{}}` on success, or `{:error, reason}` on failure.

  ## Validation Rules

  - `amplitude` must be >= 0 (negative amplitude is physically meaningless;
    to invert a wave, use a phase shift of pi instead)
  - `frequency` must be > 0 (zero frequency means no oscillation --- that's
    just a constant, not a wave)
  - `phase` can be any real number (it wraps around every 2*pi anyway)

  ## Examples

      iex> Wave.new(1.0, 440.0)
      {:ok, %Wave{amplitude: 1.0, frequency: 440.0, phase: 0.0}}

      iex> Wave.new(5.0, 2.0, 1.5708)
      {:ok, %Wave{amplitude: 5.0, frequency: 2.0, phase: 1.5708}}

      iex> Wave.new(-1.0, 440.0)
      {:error, "amplitude must be non-negative"}

      iex> Wave.new(1.0, 0.0)
      {:error, "frequency must be positive"}
  """
  def new(amplitude, freq, phi \\ 0.0)
      when is_number(amplitude) and is_number(freq) and is_number(phi) do
    cond do
      amplitude < 0 ->
        {:error, "amplitude must be non-negative"}

      amplitude > @max_float or abs(phi) > @max_float ->
        {:error, "wave parameters must be finite"}

      freq <= 0 ->
        {:error, "frequency must be positive"}

      freq > @max_frequency ->
        {:error, "angular frequency must be finite"}

      true ->
        {:ok, %Wave{amplitude: amplitude / 1.0, frequency: freq / 1.0, phase: phi / 1.0}}
    end
  end

  # ---------------------------------------------------------------------------
  # Derived Quantities
  # ---------------------------------------------------------------------------

  @doc """
  Returns the period of the wave in seconds.

  The period is the reciprocal of frequency:

      T = 1 / f

  A 2 Hz wave repeats every 0.5 seconds. A 1000 Hz wave repeats every
  0.001 seconds (1 millisecond).

  ## Examples

      iex> {:ok, w} = Wave.new(1.0, 4.0)
      iex> Wave.period(w)
      0.25
  """
  def period(%Wave{} = wave) do
    validate_wave!(wave)

    if wave.frequency < @period_overflow_frequency do
      :positive_infinity
    else
      1.0 / wave.frequency
    end
  end

  @doc """
  Returns the angular frequency (omega) in radians per second.

  Angular frequency converts from "cycles per second" to "radians per
  second" by multiplying by 2*pi (since one full cycle = 2*pi radians):

      omega = 2 * pi * f

  This is the rate at which the angle argument to sin() increases.
  A 1 Hz wave has omega = 2*pi ~= 6.283 rad/s.
  A 440 Hz wave has omega ~= 2764.6 rad/s.

  ## Examples

      iex> {:ok, w} = Wave.new(1.0, 1.0)
      iex> Wave.angular_frequency(w)
      6.283185307179586
  """
  def angular_frequency(%Wave{} = wave) do
    validate_wave!(wave)
    @two_pi * wave.frequency
  end

  # ---------------------------------------------------------------------------
  # Evaluation
  # ---------------------------------------------------------------------------

  @doc """
  Evaluates the wave at time `t` (in seconds).

  Applies the sinusoidal wave equation:

      y(t) = A * sin(2 * pi * f * t + phi)

  The steps are:
  1. Compute the angle: `theta = 2 * pi * f * t + phase`
  2. Compute `sin(theta)` using the Trig library (Maclaurin series)
  3. Scale by amplitude: `A * sin(theta)`

  ## Examples

      # A 1 Hz wave at t=0 with no phase offset starts at zero
      iex> {:ok, w} = Wave.new(1.0, 1.0)
      iex> Wave.evaluate(w, 0.0)
      0.0

      # At t=0.25 (quarter period), the wave reaches its peak
      iex> {:ok, w} = Wave.new(1.0, 1.0)
      iex> abs(Wave.evaluate(w, 0.25) - 1.0) < 1.0e-10
      true
  """
  def evaluate(%Wave{} = wave, t) when is_number(t) do
    validate_wave!(wave)

    if abs(t) > @max_float do
      raise ArgumentError, "time must be finite"
    end

    time = t / 1.0

    if wave.amplitude == 0.0 do
      0.0
    else
      reduced_time =
        if wave.frequency < @period_overflow_frequency do
          time
        else
          :math.fmod(time, period(wave))
        end

      reduced_phase = :math.fmod(wave.phase, @two_pi)
      theta = @two_pi * (wave.frequency * reduced_time) + reduced_phase
      unit = Trig.sin(theta) |> max(-1.0) |> min(1.0)
      wave.amplitude * unit
    end
  end

  defp validate_wave!(%Wave{} = wave) do
    case new(wave.amplitude, wave.frequency, wave.phase) do
      {:ok, _validated} -> :ok
      {:error, reason} -> raise ArgumentError, reason
    end
  end
end
