-- ============================================================================
-- wave — Signal / waveform generation library
-- ============================================================================
--
-- This module generates digital waveforms (arrays of floating-point samples)
-- that model continuous periodic signals. Waveforms are the building blocks of
-- audio synthesis, signal processing, and digital communications.
--
-- ## What Is a Waveform?
--
-- A waveform is a mathematical description of how a signal varies over time.
-- In the digital domain we represent it as a sequence of numbers called
-- "samples", each recording the signal's amplitude at a specific moment in
-- time. The gap between consecutive samples is 1 / sample_rate seconds.
--
-- For example, with sample_rate = 44100 (CD quality), each sample represents
-- 1/44100 ≈ 22.7 microseconds.
--
-- ## The Fundamental Waveforms
--
-- Five shapes are so common in signal processing that they have names:
--
--   Sine       — the pure oscillation; no harmonics; the "smoothest" wave.
--   Cosine     — a sine wave shifted by 90°; useful for phase calculations.
--   Square     — alternates between +amplitude and -amplitude; rich in odd
--                harmonics; the "hardest" or "buzziest" sound.
--   Sawtooth   — rises linearly then jumps; contains all harmonics; sounds
--                bright and aggressive in audio synthesis.
--   Triangle   — rises and falls linearly; only odd harmonics but softer than
--                square; sounds hollow/mellow.
--
-- ## Relationship to the Trig Package
--
-- This module depends on coding_adventures.trig for sin and cos,
-- which are computed from first principles via Maclaurin series (rather than
-- using Lua's standard math.sin/cos). This preserves the educational "from
-- scratch" philosophy of the coding-adventures stack.
--
-- ## Usage Example
--
--   local wave = require("coding_adventures.wave")
--
--   -- Generate one cycle of a 440 Hz (A4 note) sine wave at 44100 Hz
--   local samples = wave.sine_wave(440, 1.0, 0, 44100, 44100)
--   -- samples[1] through samples[44100] hold one second of audio data
--
--   -- Mix two waves together
--   local a = wave.sine_wave(440, 0.5, 0, 44100, 100)
--   local b = wave.sine_wave(880, 0.5, 0, 44100, 100)
--   local mix = wave.add_waves(a, b)
--
-- ============================================================================

local wave = {}

wave.VERSION = "0.1.0"

-- ============================================================================
-- Dependencies
-- ============================================================================

-- We use the trig package for sin and cos.
-- These functions compute trigonometric values via Maclaurin series from first
-- principles, without calling Lua's standard math.sin / math.cos.
local trig = require("coding_adventures.trig")

-- ============================================================================
-- Constants
-- ============================================================================

--- TWO_PI — the angular period of one full oscillation cycle.
--
-- All periodic waveforms repeat every 2*pi radians. We expose this constant so
-- callers don't need to recompute or import it themselves.
--
-- Value: 2 * 3.141592653589793 = 6.283185307179586
wave.TWO_PI = trig.TWO_PI

-- ============================================================================
-- sine_wave — Pure sinusoidal oscillation
-- ============================================================================

--- Generate a sine wave as an array of floating-point samples.
--
-- ### The Sine Wave Formula
--
-- For sample index i (0-based), the time is:
--
--     t = i / sample_rate          (seconds)
--
-- The amplitude at that time is:
--
--     y(t) = amplitude * sin(2*pi * frequency * t + phase)
--
-- Breaking this down:
--   - `frequency * t` counts how many complete cycles have elapsed by time t.
--   - Multiplying by `2*pi` converts cycles to radians (since one cycle = 2π).
--   - Adding `phase` shifts the wave left/right in time (phase is in radians).
--   - Multiplying by `amplitude` scales the peak value.
--
-- ### Example: 1 Hz sine wave, amplitude 1, no phase, 8 samples
--
--   sample_rate = 8, frequency = 1, amplitude = 1, phase = 0
--
--   i=0: t=0.000  sin(0.000) =  0.000
--   i=1: t=0.125  sin(0.785) =  0.707
--   i=2: t=0.250  sin(1.571) =  1.000
--   i=3: t=0.375  sin(2.356) =  0.707
--   i=4: t=0.500  sin(3.142) =  0.000
--   i=5: t=0.625  sin(3.927) = -0.707
--   i=6: t=0.750  sin(4.712) = -1.000
--   i=7: t=0.875  sin(5.497) = -0.707
--
-- @param frequency   Oscillation frequency in Hz (cycles per second).
-- @param amplitude   Peak value of the wave (default 1.0 if omitted).
-- @param phase       Phase offset in radians (default 0.0 if omitted).
-- @param sample_rate Number of samples per second (e.g., 44100 for CD quality).
-- @param num_samples Total number of samples to generate.
-- @return            Array (1-indexed) of num_samples floating-point values.
function wave.sine_wave(frequency, amplitude, phase, sample_rate, num_samples)
    local samples = {}
    for i = 0, num_samples - 1 do
        -- Time of this sample in seconds (fractional)
        local t = i / sample_rate
        -- Instantaneous angle in radians: 2π * f * t + phase_offset
        local angle = wave.TWO_PI * frequency * t + phase
        samples[i + 1] = amplitude * trig.sin(angle)
    end
    return samples
end

-- ============================================================================
-- cosine_wave — Cosine variant (90° phase-shifted sine)
-- ============================================================================

--- Generate a cosine wave as an array of floating-point samples.
--
-- ### Relationship to Sine
--
-- Cosine is simply a sine wave with a 90° (pi/2 radian) phase advance:
--
--     cos(x) = sin(x + pi/2)
--
-- Cosine is useful when you need a wave that starts at its peak value (1.0)
-- at t=0 rather than starting at zero as a sine does.
--
-- ### Formula
--
--     y(t) = amplitude * cos(2*pi * frequency * t + phase)
--         where t = i / sample_rate
--
-- @param frequency   Frequency in Hz.
-- @param amplitude   Peak value.
-- @param phase       Phase offset in radians.
-- @param sample_rate Samples per second.
-- @param num_samples Number of samples to generate.
-- @return            Array of num_samples floats.
function wave.cosine_wave(frequency, amplitude, phase, sample_rate, num_samples)
    local samples = {}
    for i = 0, num_samples - 1 do
        local t     = i / sample_rate
        local angle = wave.TWO_PI * frequency * t + phase
        samples[i + 1] = amplitude * trig.cos(angle)
    end
    return samples
end

-- ============================================================================
-- square_wave — Hard-clipped binary oscillation
-- ============================================================================

--- Generate a square wave as an array of floating-point samples.
--
-- ### What Is a Square Wave?
--
-- A square wave switches instantaneously between +amplitude and -amplitude
-- based on the sign of a sine wave at that moment:
--
--     y(t) = +amplitude   if sin(2*pi * frequency * t) >= 0
--     y(t) = -amplitude   if sin(2*pi * frequency * t) <  0
--
-- ### Why Use a Sine for Comparison?
--
-- The sine function completes one full cycle over 2*pi radians. Its zero
-- crossings at 0, pi, 2*pi, … divide the cycle into exactly two equal halves.
-- This gives a 50% duty cycle (equal time at +amplitude and -amplitude).
--
-- ### Harmonic Content
--
-- A square wave contains only ODD harmonics (1f, 3f, 5f, 7f, …) with
-- amplitudes that decrease as 1/n (where n is the harmonic number). This is
-- why square waves sound "buzzy" — they are richer in overtones than a sine.
--
--     square(t) = (4/π) * [sin(f*t) + sin(3f*t)/3 + sin(5f*t)/5 + ...]
--
-- @param frequency   Frequency in Hz.
-- @param amplitude   Peak value (wave alternates between +amplitude and -amplitude).
-- @param sample_rate Samples per second.
-- @param num_samples Number of samples to generate.
-- @return            Array of num_samples floats (each ±amplitude).
function wave.square_wave(frequency, amplitude, sample_rate, num_samples)
    local samples = {}
    for i = 0, num_samples - 1 do
        local t     = i / sample_rate
        local angle = wave.TWO_PI * frequency * t
        local s     = trig.sin(angle)
        if s >= 0 then
            samples[i + 1] = amplitude
        else
            samples[i + 1] = -amplitude
        end
    end
    return samples
end

-- ============================================================================
-- sawtooth_wave — Linearly rising ramp with instantaneous reset
-- ============================================================================

--- Generate a sawtooth wave as an array of floating-point samples.
--
-- ### What Is a Sawtooth Wave?
--
-- A sawtooth wave rises linearly from -amplitude to +amplitude over one period,
-- then instantaneously resets to -amplitude. The shape resembles a saw blade.
--
-- ### Formula
--
-- At time t, the fractional phase within the current cycle is:
--
--     phase_frac = t * frequency - floor(t * frequency + 0.5)
--
-- The `floor(x + 0.5)` is a rounding operation that centers the ramp: without
-- it the wave would start at 0 rather than ramping from -amplitude. The offset
-- 0.5 shifts the zero-crossing to the start of each period.
--
-- The output value is then:
--
--     y(t) = 2 * amplitude * phase_frac
--
-- The factor of 2 ensures the wave spans [-amplitude, +amplitude]:
--   - phase_frac ranges from -0.5 to +0.5 (due to the floor(x+0.5) centering)
--   - 2 * amplitude * (-0.5) = -amplitude  (minimum)
--   - 2 * amplitude * (+0.5) = +amplitude  (maximum)
--
-- ### Harmonic Content
--
-- A sawtooth contains ALL harmonics (1f, 2f, 3f, …) with amplitudes 1/n.
-- This makes it sound the brightest / most aggressive of the five basic shapes.
--
-- @param frequency   Frequency in Hz.
-- @param amplitude   Peak value.
-- @param sample_rate Samples per second.
-- @param num_samples Number of samples to generate.
-- @return            Array of num_samples floats.
function wave.sawtooth_wave(frequency, amplitude, sample_rate, num_samples)
    local samples = {}
    for i = 0, num_samples - 1 do
        local t          = i / sample_rate
        local phase_frac = t * frequency - math.floor(t * frequency + 0.5)
        samples[i + 1]   = 2.0 * amplitude * phase_frac
    end
    return samples
end

-- ============================================================================
-- triangle_wave — Linearly rising and falling oscillation
-- ============================================================================

--- Generate a triangle wave as an array of floating-point samples.
--
-- ### What Is a Triangle Wave?
--
-- A triangle wave rises linearly from -amplitude to +amplitude over the first
-- half-cycle, then falls linearly back to -amplitude over the second half.
-- Unlike a sawtooth, it has no instantaneous jumps.
--
-- ### Relationship to Sawtooth
--
-- A triangle wave can be derived from a sawtooth by taking its absolute value
-- and re-scaling. Specifically, if saw(t) is a sawtooth in [-1, 1]:
--
--     triangle(t) = 2 * |saw(t)| - 1     (scaled to [-1, 1])
--
-- We use a double-frequency sawtooth to get the right timing:
--
--     saw_double(t) = sawtooth(2*frequency, amplitude=1, t)
--     triangle(t)   = amplitude * (2*|saw_double(t)| - 1)
--
-- Wait — let's think again. A sawtooth at 2*frequency completes two cycles
-- per period. Taking the absolute value folds the negative half up, giving
-- a shape that goes 0→1→0→1→0. Re-scaling to [-1, 1] with `2*x - 1` gives
-- -1→+1→-1→+1→-1, which IS a triangle wave.
--
-- ### Harmonic Content
--
-- Like the square wave, the triangle contains only ODD harmonics (1f, 3f, 5f,
-- …), but with amplitudes that decrease as 1/n² (much faster than 1/n for
-- square). This is why a triangle sounds mellow/hollow compared to a square.
--
-- @param frequency   Frequency in Hz.
-- @param amplitude   Peak value.
-- @param sample_rate Samples per second.
-- @param num_samples Number of samples to generate.
-- @return            Array of num_samples floats.
function wave.triangle_wave(frequency, amplitude, sample_rate, num_samples)
    local samples = {}
    for i = 0, num_samples - 1 do
        local t = i / sample_rate
        -- Compute a sawtooth at 2× frequency (1 Lua sample of the double-freq saw).
        -- phase_frac ∈ (-0.5, +0.5]
        local phase_frac = t * (2.0 * frequency) - math.floor(t * (2.0 * frequency) + 0.5)
        -- Fold negative half up (abs value), then scale:
        --   |phase_frac| ∈ [0, 0.5]
        --   2*|phase_frac| ∈ [0, 1]
        --   2*|phase_frac| - 0.5 ∈ [-0.5, 0.5]
        -- Multiply by 2*amplitude to reach [-amplitude, +amplitude].
        samples[i + 1] = amplitude * (2.0 * (2.0 * math.abs(phase_frac)) - 1.0)
    end
    return samples
end

-- ============================================================================
-- dc_offset — Constant-value "flat" signal
-- ============================================================================

--- Generate a DC offset (constant-value) signal.
--
-- ### What Is DC Offset?
--
-- In signal processing, a "DC offset" (also called "DC bias") is a constant
-- component added to a signal. "DC" stands for "Direct Current" — the term
-- comes from electronics, where a constant voltage (as opposed to alternating
-- current, which oscillates) is called a DC signal.
--
-- In audio, adding a DC offset shifts the entire waveform up or down without
-- affecting its shape. In digital signal processing, a constant sequence is
-- often used as a test signal, a bias input to a filter, or the zero-frequency
-- component of a Fourier transform.
--
-- @param value       The constant value for every sample.
-- @param num_samples Number of samples to generate.
-- @return            Array of num_samples identical floats.
function wave.dc_offset(value, num_samples)
    local samples = {}
    for i = 1, num_samples do
        samples[i] = value
    end
    return samples
end

-- ============================================================================
-- add_waves — Element-wise sum of two waves
-- ============================================================================

--- Add two waves element-by-element.
--
-- ### Superposition Principle
--
-- When two waves occupy the same medium (air, an electrical circuit, etc.),
-- their amplitudes add. This is the "superposition principle" — the combined
-- wave at each point is the sum of the individual waves' values at that point.
--
-- Mathematically: y_combined(t) = y1(t) + y2(t)
--
-- In discrete form (arrays):
--
--     result[i] = wave1[i] + wave2[i]   for each i
--
-- ### Error Condition
--
-- The two waves must have the same length — adding a 1-second wave to a
-- 2-second wave doesn't make physical sense unless you decide what to do
-- about the unmatched samples. We raise an error rather than silently
-- truncate, because silent truncation causes confusing bugs.
--
-- @param wave1  Array of floats (the first wave).
-- @param wave2  Array of floats (the second wave, must be same length as wave1).
-- @return       New array where result[i] = wave1[i] + wave2[i].
-- @error        If #wave1 ~= #wave2.
function wave.add_waves(wave1, wave2)
    if #wave1 ~= #wave2 then
        error(string.format(
            "wave.add_waves: waves must have the same length (%d vs %d)",
            #wave1, #wave2))
    end
    local result = {}
    for i = 1, #wave1 do
        result[i] = wave1[i] + wave2[i]
    end
    return result
end

-- ============================================================================
-- scale_wave — Multiply every sample by a scalar
-- ============================================================================

--- Scale a wave by multiplying every sample by a constant.
--
-- ### Why Scale a Wave?
--
-- Scaling is used to:
--   - Adjust volume/amplitude (multiply by < 1 to quiet, > 1 to amplify).
--   - Invert phase (multiply by -1 flips the wave upside down).
--   - Normalize a wave to fit within a particular range.
--
-- ### Formula
--
--     result[i] = wave[i] * scalar   for each i
--
-- @param w       Array of floats (the wave to scale).
-- @param scalar  Multiplicative factor (any real number).
-- @return        New array where result[i] = w[i] * scalar.
function wave.scale_wave(w, scalar)
    local result = {}
    for i = 1, #w do
        result[i] = w[i] * scalar
    end
    return result
end

-- ============================================================================
-- mix_waves — Element-wise sum of multiple waves
-- ============================================================================

--- Mix multiple waves together by summing them element-by-element.
--
-- ### Mixing vs. Adding
--
-- `add_waves` handles exactly two waves. `mix_waves` handles an arbitrary
-- number of waves, making it convenient for audio mixing applications where
-- you might have dozens of tracks.
--
-- Internally, `mix_waves` applies `add_waves` iteratively, but it could
-- equally be implemented as a single loop over all indices.
--
-- ### Requirement: All Waves Same Length
--
-- All waves in the input list must have identical length. If any two differ,
-- an error is raised (propagated from `add_waves`).
--
-- ### Example: Mixing Three Harmonics
--
--   local fundamental = wave.sine_wave(440, 1.0, 0, 44100, 44100)
--   local second_harm = wave.sine_wave(880, 0.5, 0, 44100, 44100)
--   local third_harm  = wave.sine_wave(1320, 0.25, 0, 44100, 44100)
--   local rich_tone   = wave.mix_waves({fundamental, second_harm, third_harm})
--
-- @param waves  List (array) of arrays of floats. All must be same length.
-- @return       New array where result[i] = sum over all waves of wave[i].
-- @error        If waves is empty, or if waves have different lengths.
function wave.mix_waves(waves)
    if #waves == 0 then
        error("wave.mix_waves: waves list must not be empty")
    end
    local result = waves[1]
    for j = 2, #waves do
        result = wave.add_waves(result, waves[j])
    end
    return result
end

return wave
