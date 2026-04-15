// ============================================================================
// Wave.java — Simple Harmonic Wave Model
// ============================================================================
//
// y(t) = A · sin(2π·f·t + φ)
//
// A sinusoidal wave is the fundamental building block of signal processing.
// Every signal — from AM radio to 5G — is a combination of sine waves.
//
// Layer: PHY01 (physics layer 1 — leaf package, zero dependencies)
// Spec:  code/specs/PHY01-wave.md
// ============================================================================

package com.codingadventures.wave;

/**
 * An immutable sinusoidal wave: y(t) = A · sin(2π·f·t + φ).
 */
public final class Wave {

    private final double amplitude;
    private final double frequency;
    private final double phase;

    /**
     * Create a new wave.
     *
     * @param amplitude peak displacement (must be >= 0)
     * @param frequency cycles per second in Hz (must be > 0)
     * @param phase starting offset in radians
     */
    public Wave(double amplitude, double frequency, double phase) {
        if (amplitude < 0) throw new IllegalArgumentException("Amplitude must be non-negative");
        if (frequency <= 0) throw new IllegalArgumentException("Frequency must be positive");
        this.amplitude = amplitude;
        this.frequency = frequency;
        this.phase = phase;
    }

    /** Create a wave with zero phase. */
    public Wave(double amplitude, double frequency) {
        this(amplitude, frequency, 0.0);
    }

    public double getAmplitude() { return amplitude; }
    public double getFrequency() { return frequency; }
    public double getPhase() { return phase; }

    /** Period: time for one complete cycle. T = 1/f */
    public double period() { return 1.0 / frequency; }

    /** Angular frequency in radians per second. ω = 2π·f */
    public double angularFrequency() { return 2.0 * Math.PI * frequency; }

    /**
     * Evaluate the wave at time t (seconds).
     *
     * @param t time in seconds
     * @return displacement y(t) = A · sin(2π·f·t + φ)
     */
    public double evaluate(double t) {
        return amplitude * Math.sin(2.0 * Math.PI * frequency * t + phase);
    }
}
