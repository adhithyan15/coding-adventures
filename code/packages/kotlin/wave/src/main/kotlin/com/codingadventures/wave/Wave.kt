// ============================================================================
// Wave.kt — Simple Harmonic Wave Model
// ============================================================================
//
// y(t) = A · sin(2π·f·t + φ)
//
// A sinusoidal wave is the fundamental building block of signal processing.
//
// Layer: PHY01 (physics layer 1 — leaf package, zero dependencies)
// Spec:  code/specs/PHY01-wave.md
// ============================================================================

package com.codingadventures.wave

import com.codingadventures.trig.Trig

/**
 * An immutable sinusoidal wave: y(t) = A · sin(2π·f·t + φ).
 *
 * @property amplitude peak displacement (must be >= 0)
 * @property frequency cycles per second in Hz (must be > 0)
 * @property phase starting offset in radians
 */
data class Wave(
    val amplitude: Double,
    val frequency: Double,
    val phase: Double = 0.0
) {
    init {
        require(amplitude.isFinite() && amplitude >= 0) {
            "Amplitude must be finite and non-negative (got $amplitude)"
        }
        require(frequency.isFinite() && frequency > 0) {
            "Frequency must be finite and positive (got $frequency)"
        }
        require(frequency <= Double.MAX_VALUE / (2.0 * Trig.PI)) {
            "Angular frequency must remain finite (got $frequency)"
        }
        require(phase.isFinite()) { "Phase must be finite (got $phase)" }
    }

    /** Period: time for one complete cycle. T = 1/f */
    val period: Double get() = 1.0 / frequency

    /** Angular frequency in radians per second. ω = 2π·f */
    val angularFrequency: Double get() = 2.0 * Trig.PI * frequency

    /**
     * Evaluate the wave at time [t] (seconds).
     *
     * @return displacement y(t) = A · sin(2π·f·t + φ)
     */
    fun evaluate(t: Double): Double {
        require(t.isFinite()) { "Time must be finite (got $t)" }
        if (amplitude == 0.0) return 0.0
        val reducedTime = t % period
        val reducedPhase = phase % (2.0 * Trig.PI)
        val angle = 2.0 * Trig.PI * (frequency * reducedTime) + reducedPhase
        val unitValue = Trig.sin(angle)
        if (unitValue >= 1.0) return amplitude
        if (unitValue <= -1.0) return -amplitude
        return amplitude * unitValue
    }
}
