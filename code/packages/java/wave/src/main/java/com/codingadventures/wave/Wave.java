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

import com.codingadventures.trig.Trig;

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
        if (!Double.isFinite(amplitude) || amplitude < 0) {
            throw new IllegalArgumentException("Amplitude must be finite and non-negative");
        }
        if (!Double.isFinite(frequency) || frequency <= 0) {
            throw new IllegalArgumentException("Frequency must be finite and positive");
        }
        if (frequency > Double.MAX_VALUE / (2.0 * Trig.PI)) {
            throw new IllegalArgumentException("Angular frequency must remain finite");
        }
        if (!Double.isFinite(phase)) throw new IllegalArgumentException("Phase must be finite");
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
    public double angularFrequency() { return 2.0 * Trig.PI * frequency; }

    /**
     * Evaluate the wave at time t (seconds).
     *
     * @param t time in seconds
     * @return displacement y(t) = A · sin(2π·f·t + φ)
     */
    public double evaluate(double t) {
        if (!Double.isFinite(t)) throw new IllegalArgumentException("Time must be finite");
        if (amplitude == 0.0) return 0.0;
        double reducedTime = t % period();
        double reducedPhase = phase % (2.0 * Trig.PI);
        double angle = 2.0 * Trig.PI * (frequency * reducedTime) + reducedPhase;
        double unitValue = Trig.sin(angle);
        if (unitValue >= 1.0) return amplitude;
        if (unitValue <= -1.0) return -amplitude;
        return amplitude * unitValue;
    }
}
